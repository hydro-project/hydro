use std::pin::Pin;
use std::task::{Context, Poll, ready};

use futures::stream::{FusedStream, Stream};
use pin_project_lite::pin_project;

pin_project! {
    #[project = ReduceStateProj]
    enum ReduceState<'a, St, Accum, Func> {
        Reducing {
            #[pin]
            stream: St,
            accumulator: &'a mut Option<Accum>,
            func: Func,
        },
        Done,
    }
}

pin_project! {
    pub struct Reduce<'a, St, Func>
    where
        St: Stream,
    {
        #[pin]
        state: ReduceState<'a, St, St::Item, Func>,
    }
}

impl<'a, St, Func> Reduce<'a, St, Func>
where
    St: Stream,
    St::Item: Clone,
    Func: FnMut(&mut St::Item, St::Item),
{
    pub fn new(stream: St, accumulator: &'a mut Option<St::Item>, func: Func) -> Self {
        Self {
            state: ReduceState::Reducing {
                stream,
                accumulator,
                func,
            },
        }
    }
}

impl<St, Func> Stream for Reduce<'_, St, Func>
where
    St: Stream,
    St::Item: Clone,
    Func: FnMut(&mut St::Item, St::Item),
{
    type Item = St::Item;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let mut this = self.project();

        match this.state.as_mut().project() {
            ReduceStateProj::Reducing {
                mut stream,
                accumulator,
                func,
            } => {
                while let Some(item) = ready!(stream.as_mut().poll_next(cx)) {
                    if let Some(accumulator) = accumulator {
                        (func)(accumulator, item);
                    } else {
                        **accumulator = Some(item);
                    }
                }

                // Release once, after the stream is exhausted.
                let item = accumulator.clone();
                this.state.set(ReduceState::Done);
                Poll::Ready(item)
            }
            ReduceStateProj::Done => Poll::Ready(None),
        }
    }
}

impl<St, Func> FusedStream for Reduce<'_, St, Func>
where
    St: Stream,
    St::Item: Clone,
    Func: FnMut(&mut St::Item, St::Item),
{
    fn is_terminated(&self) -> bool {
        matches!(self.state, ReduceState::Done)
    }
}
