use std::pin::Pin;
use std::task::{Context, Poll, ready};

use futures::future::FusedFuture;
use futures::stream::{FusedStream, Stream};
use pin_project_lite::pin_project;

pin_project! {
    /// A future which consumes a stream by feeding a sync function `Func` with all items.
    pub struct ForEach<St, Func> {
        #[pin]
        pub(crate) stream: St,
        pub(crate) f: Func,
    }
}

impl<St, Func> ForEach<St, Func>
where
    St: Stream,
    Func: FnMut(St::Item),
{
    /// Create a new ForEach future.
    pub fn new(stream: St, f: Func) -> Self {
        Self { stream, f }
    }
}

impl<St, Func> Future for ForEach<St, Func>
where
    St: Stream,
    Func: FnMut(St::Item),
{
    type Output = ();

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let mut this = self.project();
        while let Some(item) = ready!(this.stream.as_mut().poll_next(cx)) {
            let () = (this.f)(item);
        }
        Poll::Ready(())
    }
}

impl<St, Func> FusedFuture for ForEach<St, Func>
where
    St: FusedStream,
    Func: FnMut(St::Item),
{
    fn is_terminated(&self) -> bool {
        self.stream.is_terminated()
    }
}
