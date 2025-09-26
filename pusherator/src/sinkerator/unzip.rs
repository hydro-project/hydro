use std::pin::Pin;
use std::task::{Context, Poll, ready};

use pin_project_lite::pin_project;

use super::Sinkerator;

pin_project! {
    /// An [`Sinkerator`] which splits each item to push to both downstream sinks.
    pub struct Unzip<Si0, Si1> {
        #[pin]
        si0: Si0,
        #[pin]
        si1: Si1,
    }
}

impl<Si0, Si1> Unzip<Si0, Si1> {
    /// Creates a new [`Unzip`], which splits each item to push to both `si0` and `si1`.
    pub fn new<Item>(si0: Si0, si1: Si1) -> Self
    where
        Self: Sinkerator<Item>,
    {
        Self { si0, si1 }
    }
}

impl<Si0, Si1, Item0, Item1> Sinkerator<(Item0, Item1)> for Unzip<Si0, Si1>
where
    Si0: Sinkerator<Item0>,
    Si1: Sinkerator<Item1, Error = Si0::Error>,
{
    type Error = Si0::Error;

    fn poll_send(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        item: Option<(Item0, Item1)>,
    ) -> Poll<Result<(), Self::Error>> {
        let this = self.project();
        let (item0, item1) = item.map_or((None, None), |(item0, item1)| (Some(item0), Some(item1)));
        let poll0 = this.si0.poll_send(cx, item0)?;
        let poll1 = this.si1.poll_send(cx, item1)?;
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
