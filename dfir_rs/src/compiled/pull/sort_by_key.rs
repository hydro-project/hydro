use std::task::{Poll, ready};

use futures::Stream;
use futures::stream::FusedStream;
use pin_project_lite::pin_project;

pin_project! {
    #[project = SortByKeyProj]
    enum SortByKeyState<St, Item, Func> {
        Accumulating {
            #[pin]
            stream: St,
            accum: Vec<Item>,
            sort_func: Func,
        },
        Emitting {
            into_iter: std::vec::IntoIter<Item>,
        }
    }
}

pin_project! {
    /// Stream combinator which waits for all upstream items, sorts them using `Func`, then emits them.
    #[must_use = "streams do nothing unless polled"]
    pub struct SortByKey<St, Func>
    where
        St: Stream,
    {
        #[pin]
        state: SortByKeyState<St, St::Item, Func>,
    }
}

impl<St, Func, K> SortByKey<St, Func>
where
    St: Stream,
    Func: for<'a> FnMut(&'a St::Item) -> &'a K,
    K: Ord,
{
    /// Creates a new `SortByKey` stream combinator.
    pub fn new(stream: St, sort_func: Func) -> Self {
        let size_hint = stream.size_hint().0;
        Self {
            state: SortByKeyState::Accumulating {
                stream,
                accum: Vec::with_capacity(size_hint),
                sort_func,
            },
        }
    }
}

impl<St, Func, K> Stream for SortByKey<St, Func>
where
    St: Stream,
    Func: for<'a> FnMut(&'a St::Item) -> &'a K,
    K: Ord,
{
    type Item = St::Item;

    fn poll_next(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> Poll<Option<Self::Item>> {
        let mut this = self.project();

        if let SortByKeyProj::Accumulating {
            mut stream,
            accum,
            sort_func,
        } = this.state.as_mut().project()
        {
            while let Some(item) = ready!(stream.as_mut().poll_next(cx)) {
                accum.push(item);
            }
            // Stream exhausted. Sort and transition to emitting.
            accum.sort_unstable_by(|a, b| (sort_func)(a).cmp((sort_func)(b)));
            let into_iter = std::mem::take(accum).into_iter();
            this.state
                .as_mut()
                .set(SortByKeyState::Emitting { into_iter });
        }

        if let SortByKeyProj::Emitting { into_iter } = this.state.as_mut().project() {
            return Poll::Ready(into_iter.next());
        }

        unreachable!();
    }
}

impl<St, Func, K> FusedStream for SortByKey<St, Func>
where
    St: Stream,
    Func: for<'a> FnMut(&'a St::Item) -> &'a K,
    K: Ord,
{
    fn is_terminated(&self) -> bool {
        if let SortByKeyState::Emitting { into_iter } = &self.state
            && 0 == into_iter.len()
        {
            true
        } else {
            false
        }
    }
}
