//! [`SinkPush`] adapter wrapping a [`futures_sink::Sink`] as a [`Push`].
use core::pin::Pin;
use core::task::Poll;

use futures_sink::Sink;
use pin_project_lite::pin_project;

use crate::Yes;
use crate::push::{Push, PushStep};

pin_project! {
    /// Adapter that wraps a [`Sink`] to implement the [`Push`] trait.
    ///
    /// Since `Sink` is asynchronous, this push requires `core::task::Context`
    /// and can return `Pending`.
    #[must_use = "`Push`es do nothing unless items are pushed into them"]
    pub struct SinkPush<Si> {
        #[pin]
        sink: Si,
    }
}

impl<Si> SinkPush<Si> {
    /// Creates a new [`SinkPush`] wrapping the given [`Sink`].
    pub(crate) const fn new(sink: Si) -> Self {
        Self { sink }
    }
}

impl<Si, Item, Meta> Push<Item, Meta> for SinkPush<Si>
where
    Si: Sink<Item>,
    Si::Error: core::fmt::Debug,
    Meta: Copy,
{
    type Ctx<'ctx> = core::task::Context<'ctx>;

    type CanPend = Yes;

    fn poll_ready(self: Pin<&mut Self>, ctx: &mut Self::Ctx<'_>) -> PushStep<Self::CanPend> {
        match self.project().sink.poll_ready(ctx) {
            Poll::Ready(Ok(())) => PushStep::Done,
            Poll::Ready(Err(err)) => panic!("Sink error during poll_ready: {err:?}"),
            Poll::Pending => PushStep::Pending(Yes),
        }
    }

    fn start_send(self: Pin<&mut Self>, item: Item, _meta: Meta) {
        // Discard `_meta`.
        match self.project().sink.start_send(item) {
            Ok(()) => {}
            Err(err) => panic!("Sink error during start_send: {err:?}"),
        }
    }

    fn poll_flush(self: Pin<&mut Self>, ctx: &mut Self::Ctx<'_>) -> PushStep<Self::CanPend> {
        match self.project().sink.poll_flush(ctx) {
            Poll::Ready(Ok(())) => PushStep::Done,
            Poll::Ready(Err(err)) => panic!("Sink error during poll_flush: {err:?}"),
            Poll::Pending => PushStep::Pending(Yes),
        }
    }
}
