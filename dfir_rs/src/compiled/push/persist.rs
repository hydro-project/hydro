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
        vec: &'ctx mut Vec<Item>,
        replay_idx: usize,
    }
}

impl<'ctx, Si, Item> Persist<'ctx, Si, Item> {
    /// Create with the given replay and following sink.
    pub fn new(vec: &'ctx mut Vec<Item>, replay_idx: usize, sink: Si) -> Self
    where
        Self: Sink<Item>,
    {
        debug_assert!(replay_idx <= vec.len());

        Self {
            sink,
            vec,
            replay_idx,
        }
    }

    fn empty_replay(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Si::Error>>
    where
        Si: Sink<Item>,
        Item: Clone,
    {
        let mut this = self.project();
        while let Some(item) = this.vec.get(*this.replay_idx) {
            ready!(this.sink.as_mut().poll_ready(cx))?;
            this.sink.as_mut().start_send(item.clone())?;
            *this.replay_idx += 1;
        }
        debug_assert_eq!(this.vec.len(), *this.replay_idx);
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
        let this = self.project();
        debug_assert_eq!(this.vec.len(), *this.replay_idx);

        // Persist
        this.vec.push(item.clone());
        *this.replay_idx += 1;

        this.sink.start_send(item)
    }
    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        // Ensure all replayed items are sent before flushing the underlying sink.
        ready!(self.as_mut().empty_replay(cx))?;
        self.project().sink.poll_flush(cx)
    }
    fn poll_close(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        // Ensure all replayed items are sent before closing the underlying sink.
        ready!(self.as_mut().empty_replay(cx))?;
        self.project().sink.poll_close(cx)
    }
}
