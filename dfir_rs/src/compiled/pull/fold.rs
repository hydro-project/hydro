use std::pin::Pin;
use std::task::{Context, Poll, ready};

use futures::stream::{FusedStream, Stream};
use pin_project_lite::pin_project;

pin_project! {
    #[project = FoldProj]
    pub enum Fold<'a, St, Accum, Func> {
        Folding {
            #[pin]
            stream: St,
            accumulator: &'a mut Accum,
            func: Func,
        },
        Done,
    }
}

impl<'a, St, Accum, Func> Fold<'a, St, Accum, Func> {
    pub fn new(stream: St, accumulator: &'a mut Accum, func: Func) -> Self {
        Self::Folding {
            stream,
            accumulator,
            func,
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

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        match self.as_mut().project() {
            FoldProj::Folding {
                stream,
                accumulator,
                func,
            } => {
                if let Some(item) = ready!(stream.poll_next(cx)) {
                    let () = (func)(accumulator, item);
                    Poll::Pending
                } else {
                    // Release once, after the stream is exhausted.
                    let item = accumulator.clone();
                    self.set(Fold::Done);
                    Poll::Ready(Some(item))
                }
            }
            FoldProj::Done => Poll::Ready(None),
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
        matches!(self, Self::Done)
    }
}
