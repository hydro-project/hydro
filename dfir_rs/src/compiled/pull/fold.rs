use std::pin::Pin;
use std::task::{Context, Poll, ready};

use futures::stream::Stream;
use pin_project_lite::pin_project;

pin_project! {
    pub struct Fold<'a, St, Accum, Func> {
        #[pin]
        stream: St,
        accumulator: &'a mut Accum,
        func: Func,
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

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let mut this = self.project();

        while let Some(item) = ready!(this.stream.as_mut().poll_next(cx)) {
            let () = (this.func)(this.accumulator, item);
        }

        // Release once, after the stream is exhausted.
        let item = this.accumulator.clone();
        Poll::Ready(Some(item))
    }
}
