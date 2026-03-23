//! [`FlatMapStream`] pull combinator.
use core::pin::Pin;
use core::task::Context;

use futures_core::Stream;
use pin_project_lite::pin_project;

use crate::Yes;
use crate::pull::{FusedPull, Pull, PullStep};

pin_project! {
    /// Pull combinator that maps each item to a stream via a closure and flattens the results.
    ///
    /// When the inner stream yields `Poll::Pending`, this operator yields `Pending` as well.
    #[must_use = "`Pull`s do nothing unless polled"]
    pub struct FlatMapStream<Prev, Func, St, Meta> where St: Stream {
        #[pin]
        prev: Prev,
        func: Func,
        #[pin]
        current: Option<FlatMapStreamCurrent<St, Meta>>,
    }
}

pin_project! {
    struct FlatMapStreamCurrent<St, Meta> where St: Stream {
        #[pin]
        stream: St,
        meta: Meta,
    }
}

impl<Prev, Func, St, Meta> FlatMapStream<Prev, Func, St, Meta>
where
    Self: Pull,
    St: Stream,
{
    pub(crate) const fn new(prev: Prev, func: Func) -> Self {
        Self {
            prev,
            func,
            current: None,
        }
    }
}

impl<Prev, Func, St> Pull for FlatMapStream<Prev, Func, St, Prev::Meta>
where
    Prev: Pull,
    Func: FnMut(Prev::Item) -> St,
    St: Stream,
{
    type Ctx<'ctx> = Context<'ctx>;

    type Item = St::Item;
    type Meta = Prev::Meta;
    type CanPend = Yes;
    type CanEnd = Prev::CanEnd;

    fn size_hint(&self) -> (usize, Option<usize>) {
        let current_lower = self
            .current
            .as_ref()
            .map(|c| c.stream.size_hint().0)
            .unwrap_or_default();
        (current_lower, None)
    }

    fn pull(
        self: Pin<&mut Self>,
        ctx: &mut Self::Ctx<'_>,
    ) -> PullStep<Self::Item, Self::Meta, Self::CanPend, Self::CanEnd> {
        let mut this = self.project();
        loop {
            if let Some(cur) = this.current.as_mut().as_pin_mut().map(|c| c.project()) {
                match Stream::poll_next(cur.stream, ctx) {
                    core::task::Poll::Ready(Some(item)) => {
                        return PullStep::Ready(item, *cur.meta);
                    }
                    core::task::Poll::Ready(None) => {
                        this.current.as_mut().set(None);
                    }
                    core::task::Poll::Pending => {
                        return PullStep::Pending(Yes);
                    }
                }
            }
            debug_assert!(this.current.is_none());

            match this.prev.as_mut().pull(crate::Context::from_task(ctx)) {
                PullStep::Ready(item, meta) => {
                    let stream = (this.func)(item);
                    this.current
                        .as_mut()
                        .set(Some(FlatMapStreamCurrent { stream, meta }));
                }
                PullStep::Pending(_) => {
                    return PullStep::Pending(Yes);
                }
                PullStep::Ended(can_end) => {
                    return PullStep::Ended(can_end);
                }
            }
        }
    }
}

impl<Prev, Func, St> FusedPull for FlatMapStream<Prev, Func, St, Prev::Meta>
where
    Prev: FusedPull,
    Func: FnMut(Prev::Item) -> St,
    St: Stream,
{
}

#[cfg(test)]
mod tests {
    use core::pin::Pin;
    use core::task::{Context, Waker};

    extern crate alloc;
    use alloc::vec;

    use futures_util::stream;

    use crate::Yes;
    use crate::pull::{Pull, PullStep};

    #[test]
    fn flat_map_stream_basic() {
        let waker = Waker::noop();
        let mut cx = Context::from_waker(waker);

        let mut p =
            crate::pull::iter(vec![1, 2, 3]).flat_map_stream(|x| stream::iter(vec![x, x * 10]));
        let mut p = Pin::new(&mut p);

        assert_eq!(PullStep::Ready(1, ()), p.as_mut().pull(&mut cx));
        assert_eq!(PullStep::Ready(10, ()), p.as_mut().pull(&mut cx));
        assert_eq!(PullStep::Ready(2, ()), p.as_mut().pull(&mut cx));
        assert_eq!(PullStep::Ready(20, ()), p.as_mut().pull(&mut cx));
        assert_eq!(PullStep::Ready(3, ()), p.as_mut().pull(&mut cx));
        assert_eq!(PullStep::Ready(30, ()), p.as_mut().pull(&mut cx));

        let step: PullStep<i32, (), Yes, Yes> = p.as_mut().pull(&mut cx);
        assert!(step.is_ended());
    }

    #[test]
    fn flat_map_stream_pending_propagates() {
        let waker = Waker::noop();
        let mut cx = Context::from_waker(waker);

        let mut p = crate::pull::iter(vec![1]).flat_map_stream(|_| stream::pending::<i32>());
        let mut p = Pin::new(&mut p);

        for _ in 0..10 {
            let step: PullStep<i32, (), Yes, Yes> = p.as_mut().pull(&mut cx);
            assert!(step.is_pending());
        }
    }

    #[test]
    fn flat_map_stream_fused_runtime() {
        let waker = Waker::noop();
        let mut cx = Context::from_waker(waker);

        // Use a simple, finite upstream and inner stream so we can reach `Ended`.
        let mut p =
            crate::pull::iter(vec![1, 2, 3]).flat_map_stream(|x| stream::iter(vec![x, x * 10]));
        let mut p = Pin::new(&mut p);

        // Drive the pull until we observe `Ended` once.
        loop {
            let step: PullStep<i32, (), Yes, Yes> = p.as_mut().pull(&mut cx);
            if step.is_ended() {
                break;
            }
        }

        // Once ended, further pulls must remain ended (fused behavior).
        for _ in 0..10 {
            let step: PullStep<i32, (), Yes, Yes> = p.as_mut().pull(&mut cx);
            assert!(step.is_ended());
        }
    }
}
