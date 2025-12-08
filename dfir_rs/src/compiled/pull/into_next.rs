use std::pin::Pin;
use std::task::{Context, Poll};

use futures::stream::Stream;
use pin_project_lite::pin_project;

pin_project! {
    /// A future which resolves with the next item in the stream.
    pub struct IntoNext<St> {
        #[pin]
        pub(crate) stream: St,
    }
}

impl<St> IntoNext<St>
where
    St: Stream,
{
    /// Create a new IntoNext future.
    pub fn new(stream: St) -> Self {
        Self { stream }
    }
}

impl<St> Future for IntoNext<St>
where
    St: Stream,
{
    type Output = Option<St::Item>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.project();
        this.stream.poll_next(cx)
    }
}
