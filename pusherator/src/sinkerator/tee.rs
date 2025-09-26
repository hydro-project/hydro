use std::pin::Pin;
use std::task::{Context, Poll, ready};

use pin_project_lite::pin_project;

use super::Sinkerator;

pin_project! {
    /// An [`Sinkerator`] which pushes each item to two downstream sinks by [`Clone`]ing each item once.
    pub struct Tee<Si0, Si1> {
        #[pin]
        si0: Si0,
        #[pin]
        si1: Si1,
    }
}

impl<Si0, Si1> Tee<Si0, Si1> {
    /// Creates a new [`Tee`], which pushes each item to both `si0` and `si1` by [`Clone`]ing each item once.
    pub fn new<Item>(si0: Si0, si1: Si1) -> Self
    where
        Self: Sinkerator<Item>,
    {
        Self { si0, si1 }
    }
}

impl<Si0, Si1, Item> Sinkerator<Item> for Tee<Si0, Si1>
where
    Si0: Sinkerator<Item>,
    Si1: Sinkerator<Item, Error = Si0::Error>,
    Item: Clone,
{
    type Error = Si0::Error;

    fn poll_send(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        item: Option<Item>,
    ) -> Poll<Result<(), Self::Error>> {
        let this = self.project();
        let poll0 = this.si0.poll_send(cx, item.clone())?;
        let poll1 = this.si1.poll_send(cx, item)?;
        // TODO(mingwei): Determine whether it is worth keeping track of the poll state of each of
        // the two downstream sinks.
        ready!(poll0);
        ready!(poll1);
        Poll::Ready(Ok(()))
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        let this = self.project();
        let poll0 = this.si0.poll_flush(cx)?;
        let poll1 = this.si1.poll_flush(cx)?;
        ready!(poll0);
        ready!(poll1);
        Poll::Ready(Ok(()))
    }

    fn poll_close(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        let this = self.project();
        let poll0 = this.si0.poll_close(cx)?;
        let poll1 = this.si1.poll_close(cx)?;
        ready!(poll0);
        ready!(poll1);
        Poll::Ready(Ok(()))
    }
}
