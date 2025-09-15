use std::cell::RefMut;
use std::pin::Pin;
use std::task::{Context, Poll, Waker};

use futures::Stream;
use futures::sink::Sink;
use pin_project_lite::pin_project;

pin_project! {
    /// Special sink for the `resolve_futures` and `resolve_futures_ordered` operators.
    #[must_use = "sinks do nothing unless polled"]
    pub struct ResolveFutures<'ctx, Si, Queue, Out> {
        #[pin]
        sink: Si,
        queue: RefMut<'ctx, Queue>,
        subgraph_waker: Waker,
        out: Option<Out>,
    }
}

impl<'ctx, Si, Queue, Out> ResolveFutures<'ctx, Si, Queue, Out> {
    /// Create with the given queue and following sink.
    pub fn new(queue: RefMut<'ctx, Queue>, subgraph_waker: Waker, sink: Si) -> Self {
        Self {
            sink,
            queue,
            subgraph_waker,
            out: None,
        }
    }

    /// Empties any ready items from the queue into the following sink.
    fn empty_ready(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Result<(), Si::Error>
    where
        Si: Sink<Out>,
        Queue: Stream<Item = Out> + Unpin,
    {
        let mut this = self.project();

        while this.out.is_some()
            && let Poll::Ready(ready_result) = this.sink.as_mut().poll_ready(cx)
        {
            // Propegate any downstream errors.
            let () = ready_result?;

            // Send the item.
            let out = this.out.take().unwrap();
            this.sink.as_mut().start_send(out)?;

            // Replace the next item (if any).
            // TODO(mingwei) comment on this.subgraph_waker vs cx.
            if let Poll::Ready(Some(out)) = Stream::poll_next(Pin::new(&mut **this.queue), &mut Context::from_waker(&this.subgraph_waker)) {
                *this.out = Some(out);
            }
        }
        Ok(())
    }
}

impl<'ctx, Si, Queue, Fut> Sink<Fut> for ResolveFutures<'ctx, Si, Queue, Fut::Output>
where
    Si: Sink<Fut::Output>,
    Queue: Extend<Fut> + Stream<Item = Fut::Output> + Unpin,
    Fut: Future,
{
    type Error = Si::Error;

    fn poll_ready(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        // Queue is always ready to receive more.
        Poll::Ready(self.empty_ready(cx))
    }
    fn start_send(self: Pin<&mut Self>, item: Fut) -> Result<(), Self::Error> {
        self.project().queue.extend(std::iter::once(item));
        Ok(())
    }
    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.as_mut().empty_ready(cx)?;
        self.project().sink.poll_flush(cx)
    }
    fn poll_close(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.as_mut().empty_ready(cx)?;
        self.project().sink.poll_close(cx)
    }
}
