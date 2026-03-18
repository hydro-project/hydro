//! [`ResolveFutures`] push operator for resolving futures and pushing their outputs downstream.

use core::borrow::BorrowMut;
use core::marker::PhantomData;
use core::pin::Pin;
use core::task::{Context, Poll, Waker};

use futures_core::stream::{FusedStream, Stream};
use pin_project_lite::pin_project;

use crate::Yes;
use crate::push::{Push, PushStep, ready};

pin_project! {
    /// Push operator that receives futures, queues them, and pushes their resolved outputs downstream.
    ///
    /// `Queue` is expected to be either [`futures_util::stream::FuturesOrdered`] or [`futures_util::stream::FuturesUnordered`]
    /// (or a mutable reference thereof).
    #[must_use = "`Push`es do nothing unless items are pushed into them"]
pub struct ResolveFutures<Psh, Queue, QueueInner> {
        #[pin]
        push: Psh,
        queue: Queue,
        // If `Some`, this waker will schedule future ticks, so all futures should be driven
        // by it. If `None`, the subgraph execution should block until all futures are resolved.
        subgraph_waker: Option<Waker>,

        _phantom: PhantomData<QueueInner>,
    }
}

impl<Psh, Queue, QueueInner> ResolveFutures<Psh, Queue, QueueInner> {
    /// Create with the given queue and following push.
    ///
    /// If `subgraph_waker` is `Some`, the queue will be polled with this waker.
    pub(crate) const fn new<Fut>(queue: Queue, subgraph_waker: Option<Waker>, push: Psh) -> Self
    where
        Psh: Push<Fut::Output, ()>,
        Queue: BorrowMut<QueueInner>,
        QueueInner: Extend<Fut> + FusedStream<Item = Fut::Output> + Unpin,
        Fut: Future,
        // for<'ctx> Psh::Ctx<'ctx>: crate::Context<'ctx>,
    {
        Self {
            push,
            queue,
            subgraph_waker,
            _phantom: PhantomData,
        }
    }

    /// Empties any ready items from the queue into the following push, and readies for the next send.
    fn empty_ready<Fut>(self: Pin<&mut Self>, ctx: &mut Context<'_>) -> PushStep<Yes>
    where
        Psh: Push<Fut::Output, ()>,
        Queue: BorrowMut<QueueInner>,
        QueueInner: Extend<Fut> + FusedStream<Item = Fut::Output> + Unpin,
        Fut: Future,
    {
        let mut this = self.project();

        loop {
            // Ensure the following push is ready.
            ready!(
                this.push
                    .as_mut()
                    .poll_ready(crate::Context::from_task(ctx))
            );

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
                        return PushStep::Done; // We will be re-woken on a future tick
                    } else {
                        // We will pend until the queue is emptied.
                        // TODO(mingwei): Does this mean only one item may be sent at a time?
                        return PushStep::Pending(Yes);
                    }
                }
            }
        }
    }
}

// TODO(mingwei): support arbitrary metadata
impl<Psh, Queue, QueueInner, Fut> Push<Fut, ()> for ResolveFutures<Psh, Queue, QueueInner>
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
        self.as_mut().empty_ready(ctx) // TODO(mingwei): see above
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
        ready!(self.as_mut().empty_ready(ctx));
        // Then flush the downstream push.
        let this = self.project();
        this.push
            .poll_flush(crate::Context::from_task(ctx))
            .convert_into()
    }

    fn size_hint(self: Pin<&mut Self>, hint: (usize, Option<usize>)) {
        // Each future input produces one value output.
        self.project().push.size_hint(hint);
    }
}

#[cfg(test)]
mod tests {
    use core::pin::Pin;
    use core::task::{Context, Waker};

    use futures_util::stream::FuturesUnordered;

    use super::*;
    use crate::push::Push;
    use crate::push::test_utils::{PushCall, TestPush};

    type Queue = FuturesUnordered<core::future::Ready<i32>>;

    #[test]
    fn test_poll_ready_readies_downstream() {
        let waker = Waker::noop();
        let mut cx = Context::from_waker(waker);

        let mut mock = TestPush::no_pend();
        let mut queue: Queue = [1, 2, 3].into_iter().map(core::future::ready).collect();
        let mut rf = ResolveFutures::<_, _, Queue>::new(&mut queue, None, &mut mock);

        let result = Push::<core::future::Ready<i32>, ()>::poll_ready(Pin::new(&mut rf), &mut cx);
        assert!(result.is_done());

        drop(rf);
        assert!(
            mock.history
                .iter()
                .any(|c| matches!(c, PushCall::PollReady)),
            "downstream poll_ready was not called"
        );
        assert!(
            mock.history
                .iter()
                .any(|c| matches!(c, PushCall::SendItem(_))),
            "downstream should have received items"
        );
    }

    #[test]
    fn test_poll_flush_calls_downstream_flush() {
        let waker = Waker::noop();
        let mut cx = Context::from_waker(waker);

        let mut mock = TestPush::no_pend();
        let mut queue: Queue = FuturesUnordered::new();
        let mut rf = ResolveFutures::<_, _, Queue>::new(&mut queue, None, &mut mock);

        let result = Push::<core::future::Ready<i32>, ()>::poll_flush(Pin::new(&mut rf), &mut cx);
        assert!(result.is_done());

        drop(rf);
        assert!(
            mock.history
                .iter()
                .any(|c| matches!(c, PushCall::PollFlush)),
            "downstream poll_flush was not called"
        );
    }

    #[test]
    fn resolve_futures_readies_downstream_before_each_send() {
        let waker = Waker::noop();
        let mut cx = Context::from_waker(waker);

        let mut guard = TestPush::no_pend();
        let mut queue: Queue = [1, 2, 3].into_iter().map(core::future::ready).collect();
        let mut rf = ResolveFutures::<_, _, Queue>::new(&mut queue, None, &mut guard);

        // poll_ready drains resolved futures into downstream, each with poll_ready before start_send.
        let result = Push::<core::future::Ready<i32>, ()>::poll_ready(Pin::new(&mut rf), &mut cx);
        assert!(result.is_done());

        // Send a new immediately-resolving future.
        Push::<core::future::Ready<i32>, ()>::start_send(
            Pin::new(&mut rf),
            core::future::ready(4),
            (),
        );

        // Flush should drain and flush without violating the ready guard.
        let result = Push::<core::future::Ready<i32>, ()>::poll_flush(Pin::new(&mut rf), &mut cx);
        assert!(result.is_done());
    }
}
