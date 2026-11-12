//! [`Accumulate`] pull combinator — a future that eagerly drains a pull into an
//! [`AccumState`], then resolves to a pull over the accumulated output.

use core::pin::Pin;
use core::task::Poll;

use pin_project_lite::pin_project;

use crate::Context;
use crate::accum::AccumState;
use crate::pull::{Iter, Pull, PullStep};

pin_project! {
    /// Future that eagerly drains a pull into an [`AccumState`], resolving to an [`Iter`] pull
    /// over the accumulated output items.
    ///
    /// This is the pull-side counterpart of [`crate::push::Accumulate`]. Unlike a lazy pull
    /// adapter, this is a future: the input is fully consumed when the future is awaited,
    /// ensuring the accumulator state is fully updated even if the resulting output pull is
    /// never (fully) consumed. This matters for borrowed-mode accumulators
    /// ([`crate::accum::FoldState`], [`crate::accum::ReduceState`]) whose state persists in
    /// external storage across ticks.
    ///
    /// Created by [`Pull::accumulate`], [`Pull::fold`], [`Pull::fold_ref`], [`Pull::reduce`],
    /// or [`Pull::reduce_ref`].
    #[must_use = "futures do nothing unless polled"]
    pub struct Accumulate<Prev, State> {
        #[pin]
        prev: Prev,
        state: Option<State>,
    }
}

impl<Prev, State> Accumulate<Prev, State>
where
    Self: Future,
{
    pub(crate) const fn new(prev: Prev, state: State) -> Self {
        Self {
            prev,
            state: Some(state),
        }
    }
}

impl<Prev, State> Future for Accumulate<Prev, State>
where
    Prev: Pull,
    State: AccumState<Input = Prev::Item>,
    for<'ctx> Prev::Ctx<'ctx>: Context<'ctx>,
{
    type Output = Iter<State::Iter>;

    fn poll(self: Pin<&mut Self>, cx: &mut core::task::Context<'_>) -> Poll<Self::Output> {
        let mut this = self.project();
        let ctx = <Prev::Ctx<'_> as Context<'_>>::from_task(cx);
        loop {
            return match this.prev.as_mut().pull(ctx) {
                PullStep::Ready(item, _meta) => {
                    this.state
                        .as_mut()
                        .expect("`Accumulate` polled after completion.")
                        .accumulate(item);
                    continue;
                }
                PullStep::Pending(_) => Poll::Pending,
                PullStep::Ended(_) => Poll::Ready(crate::pull::iter(
                    this.state
                        .take()
                        .expect("`Accumulate` polled after completion.")
                        .into_iter(),
                )),
            };
        }
    }
}

#[cfg(test)]
mod tests {
    use core::pin::pin;
    use core::task::{Poll, Waker};

    use crate::pull::test_utils::TestPull;
    use crate::pull::{Pull, PullStep};
    use crate::{No, Yes};

    /// Polls the given future to completion, expecting it to be ready immediately.
    macro_rules! assert_ready {
        ($fut:expr) => {{
            let waker = Waker::noop();
            let mut cx = core::task::Context::from_waker(waker);
            let Poll::Ready(out) = pin!($fut).as_mut().poll(&mut cx) else {
                panic!("expected future to be ready");
            };
            out
        }};
    }

    // ========================================================================
    // Fold owned mode
    // ========================================================================

    #[test]
    fn fold_owned_emits_single_value() {
        let pull = TestPull::items(1..=3);
        let out = assert_ready!(pull.fold(0i32, |acc, x| *acc += x));
        let mut out = pin!(out);
        assert!(matches!(out.as_mut().pull(&mut ()), PullStep::Ready(6, ())));
        assert!(matches!(out.as_mut().pull(&mut ()), PullStep::Ended(_)));
    }

    #[test]
    fn fold_owned_emits_initial_when_no_items() {
        let pull = TestPull::items(core::iter::empty::<i32>());
        let out = assert_ready!(pull.fold(42i32, |acc, x| *acc += x));
        let mut out = pin!(out);
        assert!(matches!(
            out.as_mut().pull(&mut ()),
            PullStep::Ready(42, ())
        ));
        assert!(matches!(out.as_mut().pull(&mut ()), PullStep::Ended(_)));
    }

    /// The accumulator is emitted by move — no `Clone` bound required.
    #[test]
    fn fold_owned_no_clone() {
        #[derive(Debug, PartialEq, Eq)]
        struct NoClone(i32);

        let pull = TestPull::items([NoClone(1), NoClone(2)]);
        let out = assert_ready!(pull.fold(NoClone(0), |acc, x| acc.0 += x.0));
        let mut out = pin!(out);
        assert!(matches!(
            out.as_mut().pull(&mut ()),
            PullStep::Ready(NoClone(3), ())
        ));
        assert!(matches!(out.as_mut().pull(&mut ()), PullStep::Ended(_)));
    }

    // ========================================================================
    // Fold borrowed mode
    // ========================================================================

    #[test]
    fn fold_borrowed_updates_external_state() {
        let mut acc = 0i32;
        {
            let pull = TestPull::items(1..=3);
            let out = assert_ready!(pull.fold_ref(&mut acc, |acc, x| *acc += x));
            let mut out = pin!(out);
            let PullStep::Ready(acc_ref, ()) = out.as_mut().pull(&mut ()) else {
                panic!("expected `Ready`");
            };
            assert_eq!(6, *acc_ref);
            assert!(matches!(out.as_mut().pull(&mut ()), PullStep::Ended(_)));
        }
        assert_eq!(6, acc);
    }

    /// External state is updated even if the output pull is never consumed.
    #[test]
    fn fold_borrowed_updates_state_without_output_consumption() {
        let mut acc = 0i32;
        {
            let pull = TestPull::items(1..=3);
            let _out = assert_ready!(pull.fold_ref(&mut acc, |acc, x| *acc += x));
        }
        assert_eq!(6, acc);
    }

    // ========================================================================
    // Reduce owned mode
    // ========================================================================

    #[test]
    fn reduce_owned_emits_single_value() {
        let pull = TestPull::items(1..=3);
        let out = assert_ready!(pull.reduce(|acc, x| *acc += x));
        let mut out = pin!(out);
        assert!(matches!(out.as_mut().pull(&mut ()), PullStep::Ready(6, ())));
        assert!(matches!(out.as_mut().pull(&mut ()), PullStep::Ended(_)));
    }

    #[test]
    fn reduce_owned_no_items_emits_nothing() {
        let pull = TestPull::items(core::iter::empty::<i32>());
        let out = assert_ready!(pull.reduce(|acc, x| *acc += x));
        let mut out = pin!(out);
        assert!(matches!(out.as_mut().pull(&mut ()), PullStep::Ended(_)));
    }

    // ========================================================================
    // Reduce borrowed mode
    // ========================================================================

    #[test]
    fn reduce_borrowed_persists_across_runs() {
        let mut acc: Option<i32> = None;
        {
            let pull = TestPull::items([10]);
            let _out = assert_ready!(pull.reduce_ref(&mut acc, |acc, x| *acc += x));
        }
        assert_eq!(Some(10), acc);
        {
            let pull = TestPull::items([5]);
            let out = assert_ready!(pull.reduce_ref(&mut acc, |acc, x| *acc += x));
            let mut out = pin!(out);
            let PullStep::Ready(acc_ref, ()) = out.as_mut().pull(&mut ()) else {
                panic!("expected `Ready`");
            };
            assert_eq!(15, *acc_ref);
            assert!(matches!(out.as_mut().pull(&mut ()), PullStep::Ended(_)));
        }
        assert_eq!(Some(15), acc);
    }

    /// Occupied external state is emitted even when no new items arrive.
    #[test]
    fn reduce_borrowed_replays_prior_state() {
        let mut acc: Option<i32> = Some(10);
        let pull = TestPull::items(core::iter::empty::<i32>());
        let out = assert_ready!(pull.reduce_ref(&mut acc, |acc, x| *acc += x));
        let mut out = pin!(out);
        let PullStep::Ready(acc_ref, ()) = out.as_mut().pull(&mut ()) else {
            panic!("expected `Ready`");
        };
        assert_eq!(10, *acc_ref);
        assert!(matches!(out.as_mut().pull(&mut ()), PullStep::Ended(_)));
    }

    // ========================================================================
    // Pending handling
    // ========================================================================

    /// `Pending` from the input propagates, and accumulation resumes on re-poll.
    #[test]
    fn accumulate_resumes_after_pending() {
        let pull: TestPull<i32, (), Yes, Yes, false> = TestPull::new([
            PullStep::Ready(1, ()),
            PullStep::Pending(Yes),
            PullStep::Ready(2, ()),
            PullStep::Ended(Yes),
        ]);
        let fut = pull.fold(0i32, |acc, x| *acc += x);
        let mut fut = pin!(fut);
        let waker = Waker::noop();
        let mut cx = core::task::Context::from_waker(waker);

        assert!(fut.as_mut().poll(&mut cx).is_pending());
        let Poll::Ready(out) = fut.as_mut().poll(&mut cx) else {
            panic!("expected future to be ready");
        };
        let mut out = pin!(out);
        assert!(matches!(out.as_mut().pull(&mut ()), PullStep::Ready(3, ())));
        assert!(matches!(out.as_mut().pull(&mut ()), PullStep::Ended(_)));
    }

    /// Type check: `No`-pend inputs produce a never-pending future path (compiles with `No`).
    #[test]
    fn accumulate_no_pend_input() {
        let pull: TestPull<i32, (), No, Yes, false> = TestPull::items(1..=4);
        let out = assert_ready!(pull.reduce(|acc, x| *acc *= x));
        let mut out = pin!(out);
        assert!(matches!(
            out.as_mut().pull(&mut ()),
            PullStep::Ready(24, ())
        ));
    }
}
