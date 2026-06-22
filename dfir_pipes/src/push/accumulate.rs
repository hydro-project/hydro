//! [`Accumulate`] push combinator — a unified accumulator that covers fold, reduce, sort, etc.
use core::pin::Pin;

use pin_project_lite::pin_project;

use crate::push::{Push, PushStep, ready};

/// Trait for accumulator state used by the [`Accumulate`] push combinator.
///
/// Implementors define how items are accumulated during `start_send` and how
/// the accumulated state is drained into an iterator during `poll_finalize`.
///
/// The key insight is that [`AccumState::into_iter`] consumes `self` by value,
/// which allows borrowed-mode implementations to return references with the
/// original lifetime (e.g., `&'a mut T` from `&'a mut Option<T>`).
///
/// # Two modes
///
/// - **Owned / `'tick` mode**: The state is created fresh each tick and consumed
///   on finalize. Example: `FoldState<i32, F, i32, Item>` yields `Once<i32>`.
/// - **Borrowed / `'static` mode**: The state borrows externally-owned storage
///   that persists across ticks. Example: `FoldState<&'a mut i32, F, i32, Item>`
///   yields `Once<&'a mut i32>`.
pub trait AccumState: Sized {
    /// The type of items being accumulated (input to the combinator).
    type Input;
    /// The type of items emitted downstream after finalization.
    type Output;
    /// The iterator type returned by [`AccumState::into_iter`].
    type Iter: Iterator<Item = Self::Output>;

    /// Fold an incoming item into the accumulator.
    fn accumulate(&mut self, item: Self::Input);

    /// Consume the accumulator and return an iterator over the output items.
    ///
    /// This is called once during `poll_finalize`. Because it takes `self` by
    /// value, borrowed-mode implementations can release their full lifetime.
    fn into_iter(self) -> Self::Iter;

    /// Returns the size hint for the output iterator, given the input's size hint.
    ///
    /// This is called during `size_hint` while still in the `Accumulating` phase,
    /// allowing the combinator to inform downstream of expected output cardinality
    /// before finalization occurs.
    ///
    /// The default returns `(0, None)` (unknown).
    fn size_hint(&self, _input_hint: (usize, Option<usize>)) -> (usize, Option<usize>) {
        (0, None)
    }
}

/// Internal enum representing the phase of the accumulator.
enum AccumPhase<State, Iter> {
    /// Actively accumulating items via `start_send`.
    Accumulating(State),
    /// Draining accumulated output into downstream via `poll_finalize`.
    Draining(Iter),
    /// Finalization complete — iterator exhausted, awaiting downstream finalize.
    Done,
}

pin_project! {
    /// A unified push combinator that accumulates items during `start_send` and
    /// drains them downstream during `poll_finalize`.
    ///
    /// This single struct can express fold, reduce, sort, and other
    /// accumulate-then-emit patterns by varying the [`AccumState`] implementation.
    ///
    /// # Finalization protocol
    ///
    /// On `poll_finalize`, the combinator:
    /// 1. Transitions from `Accumulating` to `Draining` by calling
    ///    [`AccumState::into_iter`].
    /// 2. Drains the iterator into the downstream `Next` push, respecting
    ///    backpressure (re-polling on `Pending`).
    /// 3. Transitions to `Done` and finalizes the downstream push.
    #[must_use = "`Push`es do nothing unless items are pushed into them"]
    pub struct Accumulate<State, Next>
    where
        State: AccumState,
    {
        #[pin]
        next: Next,
        phase: AccumPhase<State, State::Iter>,
    }
}

impl<State, Next> Accumulate<State, Next>
where
    State: AccumState,
{
    /// Creates a new `Accumulate` push combinator with the given initial state.
    pub const fn new(state: State, next: Next) -> Self {
        Self {
            next,
            phase: AccumPhase::Accumulating(state),
        }
    }
}

// TODO(mingwei): support arbitrary metadata.
impl<State, Next> Push<State::Input, ()> for Accumulate<State, Next>
where
    State: AccumState,
    Next: Push<State::Output, ()>,
{
    type Ctx<'ctx> = Next::Ctx<'ctx>;

    type CanPend = Next::CanPend;

    fn poll_ready(self: Pin<&mut Self>, _ctx: &mut Self::Ctx<'_>) -> PushStep<Self::CanPend> {
        PushStep::Done
    }

    fn start_send(self: Pin<&mut Self>, item: State::Input, _meta: ()) {
        let this = self.project();
        let AccumPhase::Accumulating(state) = this.phase else {
            panic!("start_send called after finalize");
        };
        state.accumulate(item);
    }

    fn poll_finalize(self: Pin<&mut Self>, ctx: &mut Self::Ctx<'_>) -> PushStep<Self::CanPend> {
        let mut this = self.project();

        // Transition from Accumulating -> Draining on first poll_finalize call.
        if matches!(this.phase, AccumPhase::Accumulating(..)) {
            let old_phase = core::mem::replace(this.phase, AccumPhase::Done);
            let AccumPhase::Accumulating(state) = old_phase else {
                unreachable!()
            };
            *this.phase = AccumPhase::Draining(state.into_iter());
        }

        // Drain the iterator into downstream, respecting backpressure.
        if let AccumPhase::Draining(iter) = this.phase {
            loop {
                ready!(this.next.as_mut().poll_ready(ctx));
                let Some(item) = iter.next() else {
                    break;
                };
                this.next.as_mut().start_send(item, ());
            }
            *this.phase = AccumPhase::Done;
        }

        debug_assert!(matches!(this.phase, AccumPhase::Done));
        this.next.poll_finalize(ctx)
    }

    fn size_hint(self: Pin<&mut Self>, hint: (usize, Option<usize>)) {
        let this = self.project();
        let output_hint = match this.phase {
            AccumPhase::Accumulating(state) => state.size_hint(hint),
            AccumPhase::Draining(iter) => iter.size_hint(),
            AccumPhase::Done => (0, Some(0)),
        };
        this.next.size_hint(output_hint);
    }
}

#[cfg(test)]
mod tests {
    use alloc::vec;
    use alloc::vec::Vec;
    use core::pin::Pin;

    use super::super::accum_state::{FoldState, ReduceState};
    use super::Accumulate;
    use crate::Yes;
    use crate::push::test_utils::TestPush;
    use crate::push::{Push, PushStep};

    // ========================================================================
    // Fold owned mode
    // ========================================================================

    #[test]
    fn fold_owned_emits_on_finalize() {
        let mut tp = TestPush::no_pend();
        let state = FoldState::new(0i32, |acc: &mut i32, x: i32| *acc += x);
        let mut a = Accumulate::new(state, &mut tp);
        let mut a = Pin::new(&mut a);
        a.as_mut().poll_ready(&mut ());
        a.as_mut().start_send(1, ());
        a.as_mut().poll_ready(&mut ());
        a.as_mut().start_send(2, ());
        a.as_mut().poll_ready(&mut ());
        a.as_mut().start_send(3, ());
        a.as_mut().poll_finalize(&mut ());
        assert_eq!(tp.items(), vec![6]);
    }

    #[test]
    fn fold_owned_emits_initial_when_no_items() {
        let mut tp = TestPush::no_pend();
        let state = FoldState::new(0i32, |acc: &mut i32, x: i32| *acc += x);
        let mut a = Accumulate::new(state, &mut tp);
        let mut a = Pin::new(&mut a);
        a.as_mut().poll_finalize(&mut ());
        assert_eq!(tp.items(), vec![0]);
    }

    // ========================================================================
    // Fold borrowed mode
    // ========================================================================

    #[test]
    fn fold_borrowed_emits_ref_on_finalize() {
        let mut val = 0i32;
        let mut tp = TestPush::no_pend();
        let state = FoldState::new(&mut val, |acc: &mut i32, x: i32| *acc += x);
        let mut a = Accumulate::new(state, &mut tp);
        let mut a = Pin::new(&mut a);
        a.as_mut().poll_ready(&mut ());
        a.as_mut().start_send(1, ());
        a.as_mut().poll_ready(&mut ());
        a.as_mut().start_send(2, ());
        a.as_mut().poll_ready(&mut ());
        a.as_mut().start_send(3, ());
        a.as_mut().poll_finalize(&mut ());
        // After finalize, val should be 6.
        assert_eq!(val, 6);
    }

    #[test]
    fn fold_borrowed_persists_across_ticks() {
        let mut val = 0i32;
        // First tick.
        {
            let mut tp = TestPush::no_pend();
            let state = FoldState::new(&mut val, |acc: &mut i32, x: i32| *acc += x);
            let mut a = Accumulate::new(state, &mut tp);
            let mut a = Pin::new(&mut a);
            a.as_mut().start_send(10, ());
            a.as_mut().poll_finalize(&mut ());
        }
        assert_eq!(val, 10);
        // Second tick — val persists.
        {
            let mut tp = TestPush::no_pend();
            let state = FoldState::new(&mut val, |acc: &mut i32, x: i32| *acc += x);
            let mut a = Accumulate::new(state, &mut tp);
            let mut a = Pin::new(&mut a);
            a.as_mut().start_send(5, ());
            a.as_mut().poll_finalize(&mut ());
        }
        assert_eq!(val, 15);
    }

    // ========================================================================
    // Reduce owned mode
    // ========================================================================

    #[test]
    fn reduce_owned_emits_on_finalize() {
        let mut tp = TestPush::no_pend();
        let state = ReduceState::new(None, |acc: &mut i32, x| *acc += x);
        let mut a = Accumulate::new(state, &mut tp);
        let mut a = Pin::new(&mut a);
        a.as_mut().start_send(1, ());
        a.as_mut().start_send(2, ());
        a.as_mut().start_send(3, ());
        a.as_mut().poll_finalize(&mut ());
        assert_eq!(tp.items(), vec![6]);
    }

    #[test]
    fn reduce_owned_no_items_emits_nothing() {
        let mut tp = TestPush::no_pend();
        let state: ReduceState<Option<i32>, _, i32> =
            ReduceState::new(None, |acc: &mut i32, x| *acc += x);
        let mut a = Accumulate::new(state, &mut tp);
        let mut a = Pin::new(&mut a);
        a.as_mut().poll_finalize(&mut ());
        assert_eq!(tp.items(), Vec::<i32>::new());
    }

    // ========================================================================
    // Reduce borrowed mode
    // ========================================================================

    #[test]
    fn reduce_borrowed_emits_ref_on_finalize() {
        let mut val: Option<i32> = None;
        let mut tp = TestPush::no_pend();
        let state = ReduceState::new(&mut val, |acc: &mut i32, x| *acc += x);
        let mut a = Accumulate::new(state, crate::push::map(|v: &mut i32| *v, &mut tp));
        let mut a = Pin::new(&mut a);
        a.as_mut().start_send(1, ());
        a.as_mut().start_send(2, ());
        a.as_mut().start_send(3, ());
        a.as_mut().poll_finalize(&mut ());
        // Value persists in external storage.
        assert_eq!(tp.items(), vec![6]);
        assert_eq!(val, Some(6));
    }

    #[test]
    fn reduce_borrowed_persists_across_ticks() {
        let mut val: Option<i32> = None;
        // First tick.
        {
            let mut tp = TestPush::no_pend();
            let state = ReduceState::new(&mut val, |acc: &mut i32, x| *acc += x);
            let mut a = Accumulate::new(state, crate::push::map(|v: &mut i32| *v, &mut tp));
            let mut a = Pin::new(&mut a);
            a.as_mut().start_send(10, ());
            a.as_mut().poll_finalize(&mut ());
            assert_eq!(tp.items(), vec![10]);
        }
        assert_eq!(val, Some(10));
        // Second tick — val persists, reduce merges into existing.
        {
            let mut tp = TestPush::no_pend();
            let state = ReduceState::new(&mut val, |acc: &mut i32, x| *acc += x);
            let mut a = Accumulate::new(state, crate::push::map(|v: &mut i32| *v, &mut tp));
            let mut a = Pin::new(&mut a);
            a.as_mut().start_send(5, ());
            a.as_mut().poll_finalize(&mut ());
            assert_eq!(tp.items(), vec![15]);
        }
        assert_eq!(val, Some(15));
    }

    #[test]
    fn reduce_borrowed_no_items_no_output() {
        let mut val: Option<i32> = None;
        let mut tp = TestPush::no_pend();
        let state = ReduceState::new(&mut val, |acc: &mut i32, x| *acc += x);
        let mut a = Accumulate::new(state, crate::push::map(|v: &mut i32| *v, &mut tp));
        let mut a = Pin::new(&mut a);
        a.as_mut().poll_finalize(&mut ());
        assert!(tp.items().is_empty());
        assert_eq!(val, None);
    }

    // ========================================================================
    // Sort
    // ========================================================================

    #[test]
    fn sort_emits_sorted_on_finalize() {
        use super::super::accum_state::SortState;

        let mut tp = TestPush::no_pend();
        let state = SortState { buf: Vec::new() };
        let mut a = Accumulate::new(state, &mut tp);
        let mut a = Pin::new(&mut a);
        a.as_mut().start_send(3, ());
        a.as_mut().start_send(1, ());
        a.as_mut().start_send(2, ());
        a.as_mut().poll_finalize(&mut ());
        assert_eq!(tp.items(), vec![1, 2, 3]);
    }

    // ========================================================================
    // Backpressure handling
    // ========================================================================

    #[test]
    fn accumulate_resumes_after_pending() {
        use super::super::accum_state::SortState;

        // Downstream returns Pending on the second poll_ready.
        let mut tp: TestPush<i32, Yes, true> = TestPush::new_fused(
            [
                PushStep::Done,
                PushStep::pending(),
                PushStep::Done,
                PushStep::Done,
            ],
            [],
        );
        let state = SortState { buf: Vec::new() };
        let mut a = Accumulate::new(state, &mut tp);
        let mut a = Pin::new(&mut a);
        a.as_mut().start_send(3, ());
        a.as_mut().start_send(1, ());
        a.as_mut().start_send(2, ());
        // First finalize: sends item 1, then poll_ready returns Pending.
        let step = a.as_mut().poll_finalize(&mut ());
        assert!(step.is_pending());
        // Second finalize: resumes, sends remaining items.
        let step = a.as_mut().poll_finalize(&mut ());
        assert!(step.is_done());
        assert_eq!(tp.items(), vec![1, 2, 3]);
    }
}
