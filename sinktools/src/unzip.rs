use core::pin::Pin;
use core::task::{Context, Poll};

use either::Either;
use pin_project_lite::pin_project;

use crate::{Sink, ready_both};

pin_project! {
    /// Same as [`core::iterator::Unzip`] but as a [`Sink`].
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
    /// Creates with next sinks `sink0` and `sink1`.
    pub fn new(sink0: Si0, sink1: Si1) -> Self {
        Self { sink0, sink1 }
    }

    /// Creates with next sinks `sink0` and `sink1`, ensuring this implements `Sink<(Item0, Item1)>`.
    pub fn new_sink<Item0, Item1>(sink0: Si0, sink1: Si1) -> Self
    where
        Self: Sink<(Item0, Item1)>,
    {
        Self::new(sink0, sink1)
    }
}

impl<Si0, Si1, Item0, Item1> Sink<(Item0, Item1)> for Unzip<Si0, Si1>
where
    Si0: Sink<Item0>,
    Si1: Sink<Item1>,
{
    type Error = Either<Si0::Error, Si1::Error>;

    fn poll_ready(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        let this = self.project();
        ready_both!(
            this.sink0.poll_ready(cx).map_err(Either::Left)?,
            this.sink1.poll_ready(cx).map_err(Either::Right)?,
        );
        Poll::Ready(Ok(()))
    }
    fn start_send(self: Pin<&mut Self>, item: (Item0, Item1)) -> Result<(), Self::Error> {
        let this = self.project();
        this.sink0.start_send(item.0).map_err(Either::Left)?;
        this.sink1.start_send(item.1).map_err(Either::Right)?;
        Ok(())
    }
    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        let this = self.project();
        ready_both!(
            this.sink0.poll_flush(cx).map_err(Either::Left)?,
            this.sink1.poll_flush(cx).map_err(Either::Right)?,
        );
        Poll::Ready(Ok(()))
    }
    fn poll_close(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        let this = self.project();
        ready_both!(
            this.sink0.poll_close(cx).map_err(Either::Left)?,
            this.sink1.poll_close(cx).map_err(Either::Right)?,
        );
        Poll::Ready(Ok(()))
    }
}
