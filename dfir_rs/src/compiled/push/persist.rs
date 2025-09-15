use std::pin::Pin;
use std::task::{Context, Poll, ready};

use futures::sink::Sink;
use pin_project_lite::pin_project;

pin_project! {
    /// Special sink for the `persist` operator.
    #[must_use = "sinks do nothing unless polled"]
    pub struct Persist<'ctx, Si, Item> {
        #[pin]
        sink: Si,
        replay: std::slice::Iter<'ctx, Item>,
    }
}

impl<'ctx, Si, Item> Persist<'ctx, Si, Item> {
    /// Create with the given replay and following sink.
    pub fn new(replay: std::slice::Iter<'ctx, Item>, sink: Si) -> Self {
        Self { sink, replay }
    }

    fn empty_replay(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Si::Error>>
    where
        Si: Sink<Item>,
        Item: Clone,
    {
        let mut this = self.project();
        while let Some(item) = this.replay.next() {
            ready!(this.sink.as_mut().poll_ready(cx))?;
            this.sink.as_mut().start_send(item.clone())?;
        }
        Poll::Ready(Ok(()))
    }
}

impl<'ctx, Si, Item> Sink<Item> for Persist<'ctx, Si, Item>
where
    Si: Sink<Item>,
    Item: Clone,
{
    type Error = Si::Error;

    fn poll_ready(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        ready!(self.as_mut().empty_replay(cx))?;
        self.project().sink.poll_ready(cx)
    }
    fn start_send(self: Pin<&mut Self>, item: Item) -> Result<(), Self::Error> {
        self.project().sink.start_send(item)
    }
    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        ready!(self.as_mut().empty_replay(cx))?; // TODO(mingwei): needed?
        self.project().sink.poll_flush(cx)
    }
    fn poll_close(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        ready!(self.as_mut().empty_replay(cx))?; // TODO(mingwei): needed?
        self.project().sink.poll_close(cx)
    }
}
