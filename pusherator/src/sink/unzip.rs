use std::pin::Pin;
use std::task::{Context, Poll, ready};

use futures::sink::Sink;
use pin_project_lite::pin_project;

pin_project! {
    /// Same as [`std::iterator::Unzip`] but as a [`Sink`].
    ///
    /// Synchronously maps items and sends the output to the following sink.
    #[must_use = "sinks do nothing unless polled"]
    pub struct Unzip<Si0, Si1> {
        #[pin]
        sink0: Si0,
        #[pin]
        sink1: Si1,
    }
}

impl<Si0, Si1> Unzip<Si0, Si1> {
    /// Creates with mapping `func`, following `sink`.
    pub fn new(sink0: Si0, sink1: Si1) -> Self {
        Self { sink0, sink1 }
    }
}

impl<Si0, Si1, Item0, Item1> Sink<(Item0, Item1)> for Unzip<Si0, Si1>
where
    Si0: Sink<Item0>,
    Si1: Sink<Item1, Error = Si0::Error>, // TODO(mingwei): Convert to `Either` error?
{
    type Error = Si0::Error;

    fn poll_ready(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        let this = self.project();
        let poll0 = this.sink0.poll_ready(cx)?;
        let poll1 = this.sink1.poll_ready(cx)?;
        ready!(poll0);
        ready!(poll1);
        Poll::Ready(Ok(()))
    }
    fn start_send(self: Pin<&mut Self>, item: (Item0, Item1)) -> Result<(), Self::Error> {
        let this = self.project();
        this.sink0.start_send(item.0)?;
        this.sink1.start_send(item.1)?;
        Ok(())
    }
    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        let this = self.project();
        let poll0 = this.sink0.poll_flush(cx)?;
        let poll1 = this.sink1.poll_flush(cx)?;
        ready!(poll0);
        ready!(poll1);
        Poll::Ready(Ok(()))
    }
    fn poll_close(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        let this = self.project();
        let poll0 = this.sink0.poll_close(cx)?;
        let poll1 = this.sink1.poll_close(cx)?;
        ready!(poll0);
        ready!(poll1);
        Poll::Ready(Ok(()))
    }
}
