//! [`Unzip`].
use core::pin::Pin;
use core::task::{Context, Poll};

use pin_project_lite::pin_project;

use crate::{Sink, ready_both};

pin_project! {
    /// Same as [`core::iterator::Unzip`] but as a [`Sink`].
    ///
    /// Synchronously maps items and sends the output to the following sink.
    #[must_use = "sinks do nothing unless polled"]
    pub struct Unzip<Si0, Si1> {
        #[pin]
        sink_0: Si0,
        #[pin]
        sink_1: Si1,
    }
}

impl<Si0, Si1> Unzip<Si0, Si1> {
    /// Creates with next sinks `sink_0` and `sink_1`.
    pub fn new<Item>(sink_0: Si0, sink_1: Si1) -> Self
    where
        Self: Sink<Item>,
    {
        Self { sink_0, sink_1 }
    }
}

impl<Si0, Si1, Item0, Item1> Sink<(Item0, Item1)> for Unzip<Si0, Si1>
where
    Si0: Sink<Item0>,
    Si1: Sink<Item1>,
    Si0::Error: From<Si1::Error>,
{
    type Error = Si0::Error;

    fn poll_ready(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        let this = self.project();
        ready_both!(this.sink_0.poll_ready(cx)?, this.sink_1.poll_ready(cx)?,);
        Poll::Ready(Ok(()))
    }
    fn start_send(self: Pin<&mut Self>, item: (Item0, Item1)) -> Result<(), Self::Error> {
        let this = self.project();
        this.sink_0.start_send(item.0)?;
        this.sink_1.start_send(item.1)?;
        Ok(())
    }
    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        let this = self.project();
        ready_both!(this.sink_0.poll_flush(cx)?, this.sink_1.poll_flush(cx)?,);
        Poll::Ready(Ok(()))
    }
    fn poll_close(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        let this = self.project();
        ready_both!(this.sink_0.poll_close(cx)?, this.sink_1.poll_close(cx)?,);
        Poll::Ready(Ok(()))
    }
}
