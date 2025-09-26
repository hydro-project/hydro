use std::pin::Pin;
use std::task::{Context, Poll};

use futures::sink::Sink;
use pin_project_lite::pin_project;

pin_project! {
    /// [`Future`] for pulling from an [`Iterator`] and pushing to a [`Sink`].
    pub struct Pivot<Pull, Push, Item> {
        pull: Pull,
        #[pin]
        push: Push,
        buf: Option<Item>
    }
}
impl<Pull, Push, Item> Pivot<Pull, Push, Item> {
    /// Create a new [`Pivot`] from the given `pull` and `push` sides.
    pub fn new(pull: Pull, push: Push) -> Self
    where
        Self: Future,
    {
        Self {
            pull,
            push,
            buf: None,
        }
    }
}
impl<Pull, Push, Item> Future for Pivot<Pull, Push, Item>
where
    Pull: Iterator<Item = Item>,
    Push: Sink<Item>,
{
    type Output = Result<(), Push::Error>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let mut this = self.project();

        while let Some(item) = this.buf.take().or_else(|| this.pull.next()) {
            if let Poll::Ready(()) = this.push.as_mut().poll_ready(cx)? {
                this.push.as_mut().start_send(item)?;
            } else {
                *this.buf = Some(item);
                return Poll::Pending;
            }
        }
        this.push.as_mut().poll_flush(cx)
    }
}
