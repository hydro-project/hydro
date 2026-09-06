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
                        // Pend until the queue is emptied. This is used by
                        // poll_flush to block until all futures resolve.
                        // poll_ready discards this result so callers can keep
                        // adding futures (see #2662).
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
        // Opportunistically drain any ready futures from the queue, but don't
        // block on pending ones. The queue can always accept new futures.
        let _ = self.as_mut().empty_ready(ctx);
        PushStep::Done
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

    fn poll_finalize(mut self: Pin<&mut Self>, ctx: &mut Self::Ctx<'_>) -> PushStep<Self::CanPend> {
        // First drain any ready items from the queue.
        ready!(self.as_mut().empty_ready(ctx));
        // Then flush the downstream push.
        let this = self.project();
        this.push
            .poll_finalize(crate::Context::from_task(ctx))
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

    /// Regression test for https://github.com/hydro-project/hydro/issues/2662
    ///
    /// `poll_ready` in blocking mode (no subgraph_waker) must return `Done` even
    /// when the queue contains pending futures, so that the caller can keep adding
    /// new futures via `start_send`. Otherwise the queue effectively serialises to
    /// one future at a time.
    #[test]
    fn poll_ready_allows_send_while_futures_pending() {
        use core::task::Poll;

        /// A future that is pending on the first poll and ready on the second.
        struct TwoPollFuture(bool);
        impl Future for TwoPollFuture {
            type Output = i32;
            fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
                if self.0 {
                    Poll::Ready(42)
                } else {
                    self.0 = true;
                    cx.waker().wake_by_ref();
                    Poll::Pending
                }
            }
        }

        type PendQueue = FuturesUnordered<TwoPollFuture>;

        let waker = Waker::noop();
        let mut cx = Context::from_waker(waker);

        let mock = AsyncMockPush::default();
        let mut queue: PendQueue = FuturesUnordered::new();
        queue.extend(core::iter::once(TwoPollFuture(false)));

        let mut rf = ResolveFutures::<_, _, PendQueue>::new(&mut queue, None, mock);

        // poll_ready should return Done so we can send more futures, even though
        // the queue has a pending future.
        let result = Push::<TwoPollFuture, ()>::poll_ready(Pin::new(&mut rf), &mut cx);
        assert!(
            result.is_done(),
            "poll_ready must not block on pending futures in the queue"
        );

        // We should be able to add another future.
        Push::<TwoPollFuture, ()>::start_send(Pin::new(&mut rf), TwoPollFuture(false), ());

        // Now flush should eventually resolve everything.
        // (May need multiple calls since futures need two polls each.)
        let mut saw_done = false;
        for _ in 0..4 {
            let r = Push::<TwoPollFuture, ()>::poll_flush(Pin::new(&mut rf), &mut cx);
            if r.is_done() {
                saw_done = true;
                break;
            }
        }
        assert!(
            saw_done,
            "poll_flush did not complete within the expected number of polls"
        );

        assert_eq!(rf.push.items.len(), 2, "both futures should have resolved");
    }

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
    fn test_poll_finalize_calls_downstream_finalize() {
        let waker = Waker::noop();
        let mut cx = Context::from_waker(waker);

        let mut mock = TestPush::no_pend();
        let mut queue: Queue = FuturesUnordered::new();
        let mut rf = ResolveFutures::<_, _, Queue>::new(&mut queue, None, &mut mock);

        let result =
            Push::<core::future::Ready<i32>, ()>::poll_finalize(Pin::new(&mut rf), &mut cx);
        assert!(result.is_done());

        drop(rf);
        assert!(
            mock.history
                .iter()
                .any(|c| matches!(c, PushCall::PollFinalize)),
            "downstream poll_finalize was not called"
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

        // Finalize should drain and finalize without violating the ready guard.
        let result =
            Push::<core::future::Ready<i32>, ()>::poll_finalize(Pin::new(&mut rf), &mut cx);
        assert!(result.is_done());
    }
}
