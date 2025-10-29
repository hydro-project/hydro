use std::{
    pin::Pin,
    task::{Context, Poll},
};

use pin_project_lite::pin_project;

pin_project! {
    struct Instrument<Fut> {
        #[pin]
        future: Fut,
    }
}

impl<Fut> Future for Instrument<Fut>
where
    Fut: Future,
{
    type Output = Fut::Output;
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.project();
        this.future.poll(cx)
    }
}
