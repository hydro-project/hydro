use std::pin::Pin;
use std::task::{Context, Poll, Waker};

use futures::stream::{FusedStream, Stream};
use pin_project_lite::pin_project;

pin_project! {
    /// Special stream for the `resolve_futures[_blocking][_ordered]` operators.
    ///
    /// `Queue` may be either [`futures::stream::FuturesOrdered`] or [`futures::stream::FuturesUnordered`].
    #[must_use = "streams do nothing unless polled"]
    pub struct ResolveFutures<'ctx, St, Queue> {
        #[pin]
        stream: St,
        #[pin]
        queue: &'ctx mut Queue,
        // If `Some`, this waker will schedule future ticks, so all futures should be driven
        // by it. If `None`, the subgraph execution should block until all futures are resolved.
        subgraph_waker: Option<Waker>,
    }
}

impl<'ctx, St, Queue, Fut> ResolveFutures<'ctx, St, Queue>
where
    St: FusedStream<Item = Fut>,
    Queue: Extend<Fut> + Stream<Item = Fut::Output> + Unpin,
    Fut: Future,
{
    /// Creates a new `ResolveFutures` stream.
    pub fn new(stream: St, queue: &'ctx mut Queue, subgraph_waker: Option<Waker>) -> Self {
        Self {
            stream,
            queue,
            subgraph_waker,
        }
    }
}

impl<'ctx, St, Queue, Fut> Stream for ResolveFutures<'ctx, St, Queue>
where
    St: FusedStream<Item = Fut>,
    Queue: Extend<Fut> + Stream<Item = Fut::Output> + Unpin,
    Fut: Future,
{
    type Item = Fut::Output;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let mut this = self.project();

        // Consume upstream and extend the queue.
        let stream_poll = loop {
            match this.stream.as_mut().poll_next(cx) {
                Poll::Ready(Some(fut)) => this.queue.extend(std::iter::once(fut)),
                Poll::Ready(None) => break Poll::Ready(None),
                Poll::Pending => break Poll::Pending,
            }
        };

        let poll_item = if let Some(w) = this.subgraph_waker.as_ref() {
            Stream::poll_next(
                Pin::new(&mut **this.queue),
                &mut Context::<'_>::from_waker(w),
            )
        } else {
            Stream::poll_next(Pin::new(&mut **this.queue), cx)
        };

        match poll_item {
            Poll::Ready(Some(out)) => Poll::Ready(Some(out)),
            Poll::Ready(None) => {
                // Ensure upstream is exhausted before EOS.
                stream_poll
            }
            Poll::Pending => {
                if this.subgraph_waker.is_some() {
                    // Only exhaust upstream, queue may have more futures in it.
                    stream_poll
                } else {
                    Poll::Pending
                }
            }
        }
    }
}
