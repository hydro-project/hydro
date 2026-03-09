//! [`ResolveFutures`] push operator for resolving futures and pushing their outputs downstream.

use core::borrow::BorrowMut;
use core::marker::PhantomData;
use core::pin::Pin;
use core::task::{Context, Poll, Waker};

use futures_core::stream::{FusedStream, Stream};
use pin_project_lite::pin_project;

use crate::Yes;
use crate::push::{Push, PushStep};

pin_project! {
    /// Push operator that receives futures, queues them, and pushes their resolved outputs downstream.
    ///
    /// `Queue` is expected to be either [`futures_util::stream::FuturesOrdered`] or [`futures_util::stream::FuturesUnordered`].
    #[must_use = "pushes do nothing unless items are pushed into them"]
pub struct ResolveFutures<Psh, Queue, QueueInner, Fut> {
        #[pin]
        push: Psh,
        queue: Queue,
        // If `Some`, this waker will schedule future ticks, so all futures should be driven
        // by it. If `None`, the subgraph execution should block until all futures are resolved.
        subgraph_waker: Option<Waker>,

        _phantom: PhantomData<(QueueInner, fn(Fut))>,
    }
}

impl<Psh, Queue, QueueInner, Fut> ResolveFutures<Psh, Queue, QueueInner, Fut>
where
    Psh: Push<Fut::Output, ()>,
    Queue: BorrowMut<QueueInner>,
    QueueInner: Extend<Fut> + FusedStream<Item = Fut::Output> + Unpin,
    Fut: Future,
    for<'ctx> Psh::Ctx<'ctx>: crate::Context<'ctx>,
{
    /// Create with the given queue and following push.
    ///
    /// If `subgraph_waker` is `Some`, the queue will be polled with this waker.
    pub(crate) const fn new(queue: Queue, subgraph_waker: Option<Waker>, push: Psh) -> Self {
        Self {
            push,
            queue,
            subgraph_waker,
            _phantom: PhantomData,
        }
    }

    /// Empties any ready items from the queue into the following push, and readies for the next send.
    fn empty_ready(self: Pin<&mut Self>, ctx: &mut Context<'_>) -> PushStep<Yes> {
        let mut this = self.project();

        loop {
            // Ensure the following push is ready.
            match this
                .push
                .as_mut()
                .poll_ready(crate::Context::from_task(ctx))
            {
                PushStep::Done => {}
                PushStep::Pending(_) => return PushStep::Pending(Yes),
            }

            let poll_result = if let Some(w) = this.subgraph_waker.as_ref() {
                Stream::poll_next(
                    Pin::new(this.queue.borrow_mut()),
                    &mut Context::<'_>::from_waker(w),
                )
            } else {
                Stream::poll_next(Pin::new(this.queue.borrow_mut()), ctx)
            };

            match poll_result {
                Poll::Ready(Some(out)) => {
                    this.push.as_mut().start_send(out, ());
                }
                Poll::Ready(None) => {
                    return PushStep::Done;
                }
                Poll::Pending => {
                    if this.subgraph_waker.is_some() {
                        return PushStep::Done; // we will be re-woken on a future tick
                    } else {
                        return PushStep::Pending(Yes);
                    }
                }
            }
        }
    }
}

impl<Psh, Queue, QueueInner, Fut> Push<Fut, ()> for ResolveFutures<Psh, Queue, QueueInner, Fut>
where
    Psh: Push<Fut::Output, ()>,
    Queue: BorrowMut<QueueInner>,
    QueueInner: Extend<Fut> + FusedStream<Item = Fut::Output> + Unpin,
    Fut: Future,
    for<'ctx> Psh::Ctx<'ctx>: crate::Context<'ctx>,
{
    type Ctx<'ctx> = Context<'ctx>;
    type CanPend = Yes;

    fn poll_ready(mut self: Pin<&mut Self>, ctx: &mut Self::Ctx<'_>) -> PushStep<Self::CanPend> {
        self.as_mut().empty_ready(ctx) // This includes readying `this.push`.
    }

    fn start_send(self: Pin<&mut Self>, item: Fut, _meta: ()) {
        let this = self.project();
        this.queue.borrow_mut().extend(core::iter::once(item));

        if let Some(waker) = this.subgraph_waker.as_ref() {
            // If `subgraph_waker` is `Some`:
            // We MUST poll the queue stream to ensure that the futures begin.
            // We use `this.subgraph_waker` to poll the queue stream, which means the futures are driven
            // by the subgraph's own waker. This allows the subgraph execution to continue without waiting
            // for the queued futures to complete; the subgraph does not block ("yield") on their readiness.
            // If we instead used `cx.waker()`, the subgraph execution would yield ("block") until all queued
            // futures are ready, effectively pausing subgraph progress until completion of those futures.
            // Choose the waker based on whether you want subgraph execution to proceed independently of
            // the queued futures, or to wait for them to complete before continuing.
            if let Poll::Ready(Some(out)) = Stream::poll_next(
                Pin::new(this.queue.borrow_mut()),
                &mut Context::from_waker(waker),
            ) {
                this.push.start_send(out, ());
            }
        }
    }

    fn poll_flush(mut self: Pin<&mut Self>, ctx: &mut Self::Ctx<'_>) -> PushStep<Self::CanPend> {
        // First drain any ready items from the queue.
        match self.as_mut().empty_ready(ctx) {
            PushStep::Done => {}
            PushStep::Pending(_) => return PushStep::pending(),
        }
        // Then flush the downstream push.
        let this = self.project();
        this.push
            .poll_flush(crate::Context::from_task(ctx))
            .convert_into()
    }
}

#[cfg(test)]
mod tests {
    use core::pin::Pin;
    use core::task::{Context, Waker};

    use futures_util::stream::FuturesUnordered;

    use super::*;
    use crate::push::Push;
    use crate::push::test_utils::AsyncMockPush;

    type Queue = FuturesUnordered<core::future::Ready<i32>>;

    #[test]
    fn test_poll_ready_readies_downstream() {
        let waker = Waker::noop();
        let mut cx = Context::from_waker(waker);

        let mock = AsyncMockPush::default();
        let mut queue: Queue = [1, 2, 3].into_iter().map(core::future::ready).collect();
        let mut rf = ResolveFutures::<_, _, Queue, core::future::Ready<i32>>::new(
            &mut queue, None, mock,
        );

        let result = Push::<core::future::Ready<i32>, ()>::poll_ready(Pin::new(&mut rf), &mut cx);
        assert!(result.is_done());

        // poll_ready on downstream should have been called (at least once per item + once final).
        assert!(
            rf.push.poll_ready_count > 0,
            "downstream poll_ready was not called"
        );
        assert!(
            !rf.push.items.is_empty(),
            "downstream should have received items"
        );
    }

    #[test]
    fn test_poll_flush_calls_downstream_flush() {
        let waker = Waker::noop();
        let mut cx = Context::from_waker(waker);

        let mock = AsyncMockPush::default();
        let mut queue: Queue = FuturesUnordered::new();
        let mut rf = ResolveFutures::<_, _, Queue, core::future::Ready<i32>>::new(
            &mut queue, None, mock,
        );

        let result = Push::<core::future::Ready<i32>, ()>::poll_flush(Pin::new(&mut rf), &mut cx);
        assert!(result.is_done());

        // Verify downstream poll_flush was called.
        assert!(
            rf.push.poll_flush_count > 0,
            "downstream poll_flush was not called"
        );
    }
}
