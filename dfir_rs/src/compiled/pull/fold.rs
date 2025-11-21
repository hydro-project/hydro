use std::pin::Pin;
use std::task::{Context, Poll, ready};

use futures::stream::{FusedStream, Stream};
use pin_project_lite::pin_project;

pin_project! {
    #[project = FoldStateProj]
    enum FoldState<'a, St, Accum, Func> {
        Folding {
            #[pin]
            stream: St,
            accumulator: &'a mut Accum,
            func: Func,
        },
        Done,
    }
}

pin_project! {
    pub struct Fold<'a, St, Accum, Func> {
        #[pin]
        state: FoldState<'a, St, Accum, Func>,
    }
}

impl<'a, St, Accum, Func> Fold<'a, St, Accum, Func>
where
    St: Stream,
    Accum: Clone,
    Func: FnMut(&mut Accum, St::Item),
{
    pub fn new(stream: St, accumulator: &'a mut Accum, func: Func) -> Self {
        Self {
            state: FoldState::Folding {
                stream,
                accumulator,
                func,
            },
        }
    }
}

impl<St, Accum, Func> Stream for Fold<'_, St, Accum, Func>
where
    St: Stream,
    Accum: Clone,
    Func: FnMut(&mut Accum, St::Item),
{
    type Item = Accum;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let mut this = self.project();

        match this.state.as_mut().project() {
            FoldStateProj::Folding {
                mut stream,
                accumulator,
                func,
            } => {
                while let Some(item) = ready!(stream.as_mut().poll_next(cx)) {
                    let () = (func)(accumulator, item);
                }

                // Release once, after the stream is exhausted.
                let item = accumulator.clone();
                this.state.set(FoldState::Done);
                Poll::Ready(Some(item))
            }
            FoldStateProj::Done => Poll::Ready(None),
        }
    }
}

impl<St, Accum, Func> FusedStream for Fold<'_, St, Accum, Func>
where
    St: Stream,
    Accum: Clone,
    Func: FnMut(&mut Accum, St::Item),
{
    fn is_terminated(&self) -> bool {
        matches!(self.state, FoldState::Done)
    }
}
