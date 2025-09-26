// TODO(mingwei): Move this to separate crate (pusherator)
use std::pin::Pin;
use std::task::{Context, Poll, ready};

use futures::sink::Sink;
use pin_project_lite::pin_project;
use pusherator::sinkerator::Sinkerator;

use crate::util::demux_enum::{DemuxEnumSink, DemuxEnumSinkerator};

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

    fn poll_ready_impl(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Item::Error>>
    where
        Item: DemuxEnumSink<Outputs>,
    {
        let this = self.project();
        if let Some(item) = &this.item {
            ready!(Item::poll_ready(item, this.outputs, cx))?;
            let item = this.item.take().unwrap();
            Item::start_send(item, this.outputs)?;
        }
        debug_assert!(
            this.item.is_none(),
            "Sink not ready: `poll_ready` must be called and return `Ready` before `start_send` is called."
        );
        Poll::Ready(Ok(()))
    }
}

impl<Outputs, Item> Sink<Item> for DemuxEnum<Outputs, Item>
where
    Item: DemuxEnumSink<Outputs>,
{
    type Error = Item::Error;

    fn poll_ready(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.poll_ready_impl(cx)
    }

    fn start_send(self: Pin<&mut Self>, item: Item) -> Result<(), Self::Error> {
        let this = self.project();
        debug_assert!(
            this.item.is_none(),
            "Sink not ready: `poll_ready` must be called and return `Ready` before `start_send` is called."
        );
        *this.item = Some(item);
        Ok(())
    }

    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        ready!(self.as_mut().poll_ready_impl(cx))?;
        Item::poll_flush(self.project().outputs, cx)
    }

    fn poll_close(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        ready!(self.as_mut().poll_ready_impl(cx))?;
        Item::poll_close(self.project().outputs, cx)
    }
}

impl<Outputs, Item> Sinkerator<Item> for DemuxEnum<Outputs, crate::Never>
where
    Item: DemuxEnumSinkerator<Outputs>,
{
    type Error = Item::Error;

    fn poll_send(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        item: Option<Item>,
    ) -> Poll<Result<(), Self::Error>> {
        Item::poll_send(self.project().outputs, cx, item)
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Item::poll_flush(self.project().outputs, cx)
    }

    fn poll_close(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Item::poll_close(self.project().outputs, cx)
    }
}
