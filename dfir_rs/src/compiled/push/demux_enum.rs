use std::pin::Pin;
use std::task::{Context, Poll, ready};

use futures::sink::Sink;
use pin_project_lite::pin_project;

use crate::util::demux_enum::DemuxEnumSink;

pin_project! {
    /// Special sink for the `demux_enum` operator.
    #[must_use = "sinks do nothing unless polled"]
    pub struct DemuxEnum<Outputs, Item> {
        outputs: Outputs,
        item: Option<Item>,
    }
}

impl<Outputs, Item> DemuxEnum<Outputs, Item> {
    /// Creates with the given `Outputs`.
    pub fn new(outputs: Outputs) -> Self
    where
        Self: Sink<Item>,
    {
        Self {
            outputs,
            item: None,
        }
    }
}

impl<Outputs, Item> Sink<Item> for DemuxEnum<Outputs, Item>
where
    Item: DemuxEnumSink<Outputs>,
{
    type Error = Item::Error;

    fn poll_ready(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        let this = self.project();
        if let Some(item) = &this.item {
            ready!(Item::poll_ready(item, this.outputs, cx))?;
            let item = this.item.take().unwrap();
            Item::start_send(item, this.outputs)?;
        }
        debug_assert!(this.item.is_none(), "Sink not ready.");
        Poll::Ready(Ok(()))
    }

    fn start_send(self: Pin<&mut Self>, item: Item) -> Result<(), Self::Error> {
        let this = self.project();
        debug_assert!(this.item.is_none(), "Sink not ready.");
        *this.item = Some(item);
        Ok(())
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Item::poll_flush(self.project().outputs, cx)
    }

    fn poll_close(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Item::poll_close(self.project().outputs, cx)
    }
}
