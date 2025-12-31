// TODO(mingwei): Move this & derive macro to separate crate ([`sinktools`])
use std::pin::Pin;
use std::task::{Context, Poll};

use futures::sink::Sink;
use pin_project_lite::pin_project;

use crate::util::demux_enum::DemuxEnumSink;

pin_project! {
    /// Special sink for the `demux_enum` operator.
    #[must_use = "sinks do nothing unless polled"]
    pub struct DemuxEnum<Outputs> {
        outputs: Outputs,
    }
}

impl<Outputs> DemuxEnum<Outputs> {
    /// Creates with the given `Outputs`.
    pub fn new<Item>(outputs: Outputs) -> Self
    where
        Self: Sink<Item>,
    {
        Self { outputs }
    }
}

impl<Outputs, Item> Sink<Item> for DemuxEnum<Outputs>
where
    Item: DemuxEnumSink<Outputs>,
{
    type Error = Item::Error;

    fn poll_ready(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Item::poll_ready(self.project().outputs, cx)
    }

    fn start_send(self: Pin<&mut Self>, item: Item) -> Result<(), Self::Error> {
        Item::start_send(item, self.project().outputs)
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Item::poll_flush(self.project().outputs, cx)
    }

    fn poll_close(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Item::poll_close(self.project().outputs, cx)
    }
}
