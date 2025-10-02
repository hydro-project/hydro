use std::cell::RefMut;
use std::pin::Pin;
use std::task::{Context, Poll, Waker, ready};

use futures::Stream;
use futures::sink::Sink;
use pin_project_lite::pin_project;

pin_project! {
    /// Special sink for the `resolve_futures` and `resolve_futures_ordered` operators.
    #[must_use = "sinks do nothing unless polled"]
    pub struct ResolveFutures<'ctx, Si, Queue> {
        #[pin]
        sink: Si,
        queue: RefMut<'ctx, Queue>,
        subgraph_waker: Waker,
    }
}

impl<'ctx, Si, Queue> ResolveFutures<'ctx, Si, Queue> {
    /// Create with the given queue and following sink.
    pub fn new(queue: RefMut<'ctx, Queue>, subgraph_waker: Waker, sink: Si) -> Self {
        Self {
            sink,
            queue,
            subgraph_waker,
        }
    }

    /// Empties any ready items from the queue into the following sink.
    fn empty_ready(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Si::Error>>
    where
        Si: Sink<Queue::Item>,
        Queue: Stream + Unpin,
    {
        let mut this = self.project();

        loop {
            // Ensure the following sink is ready.
            ready!(this.sink.as_mut().poll_ready(cx))?;

            if let Poll::Ready(Some(out)) = Stream::poll_next(
                Pin::new(&mut **this.queue),
                &mut Context::from_waker(this.subgraph_waker),
            ) {
                this.sink.as_mut().start_send(out)?;
            } else {
                return Poll::Ready(Ok(()));
            }
        }
    }
}

impl<'ctx, Si, Queue, Fut> Sink<Fut> for ResolveFutures<'ctx, Si, Queue>
where
    Si: Sink<Fut::Output>,
    Queue: Extend<Fut> + Stream<Item = Fut::Output> + Unpin,
    Fut: Future,
{
    type Error = Si::Error;

    fn poll_ready(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.empty_ready(cx)
    }
    fn start_send(self: Pin<&mut Self>, item: Fut) -> Result<(), Self::Error> {
        let mut this = self.project();

        this.queue.extend(std::iter::once(item));
        // We MUST poll the queue stream to ensure that the futures begin.
        // We use `this.subgraph_waker` to poll the queue stream, which means the futures are driven
        // by the subgraph's own waker. This allows the subgraph execution to continue without waiting
        // for the queued futures to complete; the subgraph does not block ("yield") on their readiness.
        // If we instead used `cx.waker()`, the subgraph execution would yield ("block") until all queued
        // futures are ready, effectively pausing subgraph progress until completion of those futures.
        // Choose the waker based on whether you want subgraph execution to proceed independently of
        // the queued futures, or to wait for them to complete before continuing.
        if let Poll::Ready(Some(out)) = Stream::poll_next(
            Pin::new(&mut **this.queue),
            &mut Context::from_waker(this.subgraph_waker),
        ) {
            this.sink.as_mut().start_send(out)?;
        }
        Ok(())
    }
    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        ready!(self.as_mut().empty_ready(cx))?;
        self.project().sink.poll_flush(cx)
    }
    fn poll_close(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        ready!(self.as_mut().empty_ready(cx))?;
        self.project().sink.poll_close(cx)
    }
}
