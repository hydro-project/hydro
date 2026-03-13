//! Symmetric hash join combinator for Pull-based streams.

use std::borrow::{BorrowMut, Cow};
use std::marker::PhantomData;
use std::pin::Pin;

use itertools::Either;
use pin_project_lite::pin_project;
use smallvec::SmallVec;

use crate::pull::half_join_state::HalfJoinState;
use crate::pull::{FusedPull, Pull, PullStep};
use crate::{Context, No, Toggle, Yes};

pin_project! {
    /// Pull combinator for symmetric hash join operations.
    ///
    /// Joins two pulls on a common key, producing tuples of matched values.
    /// Items are processed as they arrive, with matches emitted immediately.
    #[must_use = "`Pull`s do nothing unless polled"]
    #[derive(Clone, Debug, Default)]
    pub struct SymmetricHashJoin<Lhs, Rhs, LhsState, RhsState, LhsStateInner, RhsStateInner> {
        #[pin]
        lhs: Lhs,
        #[pin]
        rhs: Rhs,

        lhs_state: LhsState,
        rhs_state: RhsState,

        _phantom: PhantomData<(LhsStateInner, RhsStateInner)>,
    }
}

impl<Lhs, Rhs, LhsState, RhsState, LhsStateInner, RhsStateInner>
    SymmetricHashJoin<Lhs, Rhs, LhsState, RhsState, LhsStateInner, RhsStateInner>
where
    Self: Pull,
{
    /// Creates a new symmetric hash join Pull from two input Pulls and their join states.
    pub(crate) const fn new(lhs: Lhs, rhs: Rhs, lhs_state: LhsState, rhs_state: RhsState) -> Self {
        Self {
            lhs,
            rhs,
            lhs_state,
            rhs_state,
            _phantom: PhantomData,
        }
    }
}

impl<Key, Lhs, V1, Rhs, V2, LhsState, RhsState, LhsStateInner, RhsStateInner> Pull
    for SymmetricHashJoin<Lhs, Rhs, LhsState, RhsState, LhsStateInner, RhsStateInner>
where
    Key: Eq + std::hash::Hash + Clone,
    V1: Clone,
    V2: Clone,
    Lhs: FusedPull<Item = (Key, V1), Meta = ()>,
    Rhs: FusedPull<Item = (Key, V2), Meta = ()>,
    LhsState: BorrowMut<LhsStateInner>,
    RhsState: BorrowMut<RhsStateInner>,
    LhsStateInner: HalfJoinState<Key, V1, V2>,
    RhsStateInner: HalfJoinState<Key, V2, V1>,
{
    type Ctx<'ctx> = <Lhs::Ctx<'ctx> as Context<'ctx>>::Merged<Rhs::Ctx<'ctx>>;

    type Item = (Key, (V1, V2));
    type Meta = ();
    type CanPend = <Lhs::CanPend as Toggle>::Or<Rhs::CanPend>;
    type CanEnd = <Lhs::CanEnd as Toggle>::And<Rhs::CanEnd>;

    fn pull(
        self: Pin<&mut Self>,
        ctx: &mut Self::Ctx<'_>,
    ) -> PullStep<Self::Item, Self::Meta, Self::CanPend, Self::CanEnd> {
        let mut this = self.project();
        let lhs_state = this.lhs_state.borrow_mut();
        let rhs_state = this.rhs_state.borrow_mut();

        loop {
            // First check for any pending matches from previous probes
            if let Some((k, v2, v1)) = lhs_state.pop_match() {
                return PullStep::Ready((k, (v1, v2)), ());
            }
            if let Some((k, v1, v2)) = rhs_state.pop_match() {
                return PullStep::Ready((k, (v1, v2)), ());
            }

            // Try to pull from lhs
            let lhs_step = this
                .lhs
                .as_mut()
                .pull(<Lhs::Ctx<'_> as Context<'_>>::unmerge_self(ctx));
            if let PullStep::Ready((k, v1), _meta) = lhs_step {
                if lhs_state.build(k.clone(), Cow::Borrowed(&v1))
                    && let Some((k, v1, v2)) = rhs_state.probe(&k, &v1)
                {
                    return PullStep::Ready((k, (v1, v2)), ());
                }
                continue;
            }

            // Try to pull from rhs
            let rhs_step = this
                .rhs
                .as_mut()
                .pull(<Lhs::Ctx<'_> as Context<'_>>::unmerge_other(ctx));
            if let PullStep::Ready((k, v2), _meta) = rhs_step {
                if rhs_state.build(k.clone(), Cow::Borrowed(&v2))
                    && let Some((k, v2, v1)) = lhs_state.probe(&k, &v2)
                {
                    return PullStep::Ready((k, (v1, v2)), ());
                }
                continue;
            }

            if lhs_step.is_pending() || rhs_step.is_pending() {
                return PullStep::pending();
            }

            // If we get here, both sides have ended.
            debug_assert!(lhs_step.is_ended());
            debug_assert!(rhs_step.is_ended());
            return PullStep::ended();
        }
    }
}

/// Iterator for new tick - iterates over all matches after both sides are drained.
pub struct NewTickJoinIter<'a, Key, V1, V2, LhsState, RhsState> {
    lhs_state: &'a LhsState,
    rhs_state: &'a RhsState,
    lhs_smaller: bool,
    // State for iteration
    outer_iter: Option<std::collections::hash_map::Iter<'a, Key, SmallVec<[V1; 1]>>>,
    outer_iter_rhs: Option<std::collections::hash_map::Iter<'a, Key, SmallVec<[V2; 1]>>>,
    current_key: Option<&'a Key>,
    outer_val_iter: Option<std::slice::Iter<'a, V1>>,
    outer_val_iter_rhs: Option<std::slice::Iter<'a, V2>>,
    current_outer_val: Option<&'a V1>,
    current_outer_val_rhs: Option<&'a V2>,
    inner_val_iter: Option<std::slice::Iter<'a, V2>>,
    inner_val_iter_rhs: Option<std::slice::Iter<'a, V1>>,
}

impl<'a, Key, V1, V2, LhsState, RhsState> NewTickJoinIter<'a, Key, V1, V2, LhsState, RhsState>
where
    Key: Eq + std::hash::Hash + Clone,
    V1: Clone,
    V2: Clone,
    LhsState: HalfJoinState<Key, V1, V2>,
    RhsState: HalfJoinState<Key, V2, V1>,
{
    fn new_lhs_smaller(lhs_state: &'a LhsState, rhs_state: &'a RhsState) -> Self {
        Self {
            lhs_state,
            rhs_state,
            lhs_smaller: true,
            outer_iter: Some(lhs_state.iter()),
            outer_iter_rhs: None,
            current_key: None,
            outer_val_iter: None,
            outer_val_iter_rhs: None,
            current_outer_val: None,
            current_outer_val_rhs: None,
            inner_val_iter: None,
            inner_val_iter_rhs: None,
        }
    }

    fn new_rhs_smaller(lhs_state: &'a LhsState, rhs_state: &'a RhsState) -> Self {
        Self {
            lhs_state,
            rhs_state,
            lhs_smaller: false,
            outer_iter: None,
            outer_iter_rhs: Some(rhs_state.iter()),
            current_key: None,
            outer_val_iter: None,
            outer_val_iter_rhs: None,
            current_outer_val: None,
            current_outer_val_rhs: None,
            inner_val_iter: None,
            inner_val_iter_rhs: None,
        }
    }
}

impl<'a, Key, V1, V2, LhsState, RhsState> Iterator
    for NewTickJoinIter<'a, Key, V1, V2, LhsState, RhsState>
where
    Key: Eq + std::hash::Hash + Clone,
    V1: Clone,
    V2: Clone,
    LhsState: HalfJoinState<Key, V1, V2>,
    RhsState: HalfJoinState<Key, V2, V1>,
{
    type Item = (Key, (V1, V2));

    fn next(&mut self) -> Option<Self::Item> {
        if self.lhs_smaller {
            self.next_lhs_smaller()
        } else {
            self.next_rhs_smaller()
        }
    }
}

impl<'a, Key, V1, V2, LhsState, RhsState> NewTickJoinIter<'a, Key, V1, V2, LhsState, RhsState>
where
    Key: Eq + std::hash::Hash + Clone,
    V1: Clone,
    V2: Clone,
    LhsState: HalfJoinState<Key, V1, V2>,
    RhsState: HalfJoinState<Key, V2, V1>,
{
    fn next_lhs_smaller(&mut self) -> Option<(Key, (V1, V2))> {
        loop {
            // Try to get next v2 for current v1
            if let Some(ref mut v2_iter) = self.inner_val_iter {
                if let Some(v2) = v2_iter.next() {
                    let key = self.current_key.unwrap();
                    let v1 = self.current_outer_val.unwrap();
                    return Some((key.clone(), (v1.clone(), v2.clone())));
                }
                self.inner_val_iter = None;
            }

            // Try to get next v1 for current key
            if let Some(ref mut v1_iter) = self.outer_val_iter {
                if let Some(v1) = v1_iter.next() {
                    self.current_outer_val = Some(v1);
                    let key = self.current_key.unwrap();
                    self.inner_val_iter = Some(self.rhs_state.full_probe(key));
                    continue;
                }
                self.outer_val_iter = None;
                self.current_key = None;
            }

            // Try to get next key from lhs
            if let Some(ref mut lhs_iter) = self.outer_iter {
                if let Some((key, values)) = lhs_iter.next() {
                    self.current_key = Some(key);
                    self.outer_val_iter = Some(values.iter());
                    continue;
                }
                self.outer_iter = None;
            }

            return None;
        }
    }

    fn next_rhs_smaller(&mut self) -> Option<(Key, (V1, V2))> {
        loop {
            // Try to get next v1 for current v2
            if let Some(ref mut v1_iter) = self.inner_val_iter_rhs {
                if let Some(v1) = v1_iter.next() {
                    let key = self.current_key.unwrap();
                    let v2 = self.current_outer_val_rhs.unwrap();
                    return Some((key.clone(), (v1.clone(), v2.clone())));
                }
                self.inner_val_iter_rhs = None;
            }

            // Try to get next v2 for current key
            if let Some(ref mut v2_iter) = self.outer_val_iter_rhs {
                if let Some(v2) = v2_iter.next() {
                    self.current_outer_val_rhs = Some(v2);
                    let key = self.current_key.unwrap();
                    self.inner_val_iter_rhs = Some(self.lhs_state.full_probe(key));
                    continue;
                }
                self.outer_val_iter_rhs = None;
                self.current_key = None;
            }

            // Try to get next key from rhs
            if let Some(ref mut rhs_iter) = self.outer_iter_rhs {
                if let Some((key, values)) = rhs_iter.next() {
                    self.current_key = Some(key);
                    self.outer_val_iter_rhs = Some(values.iter());
                    continue;
                }
                self.outer_iter_rhs = None;
            }

            return None;
        }
    }
}

pin_project! {
    /// Pull wrapper for the new tick iterator case.
    #[must_use = "`Pull`s do nothing unless polled"]
    pub struct NewTickJoinPull<'a, Key, V1, V2, LhsState, RhsState>
    where
        Key: Clone,
        V1: Clone,
        V2: Clone,
    {
        iter: NewTickJoinIter<'a, Key, V1, V2, LhsState, RhsState>,
    }
}

impl<'a, Key, V1, V2, LhsState, RhsState> Pull
    for NewTickJoinPull<'a, Key, V1, V2, LhsState, RhsState>
where
    Key: Eq + std::hash::Hash + Clone,
    V1: Clone,
    V2: Clone,
    LhsState: HalfJoinState<Key, V1, V2>,
    RhsState: HalfJoinState<Key, V2, V1>,
{
    type Ctx<'ctx> = ();

    type Item = (Key, (V1, V2));
    type Meta = ();
    type CanPend = No;
    type CanEnd = Yes;

    fn pull(
        self: Pin<&mut Self>,
        _ctx: &mut Self::Ctx<'_>,
    ) -> PullStep<Self::Item, Self::Meta, Self::CanPend, Self::CanEnd> {
        let this = self.project();
        match this.iter.next() {
            Some(item) => PullStep::Ready(item, ()),
            None => PullStep::Ended(Yes),
        }
    }
}

/// Type alias for the `Either` pull returned by [`symmetric_hash_join`].
pub type SymmetricHashJoinEither<'a, Key, V1, V2, Lhs, Rhs, LhsState, RhsState> = Either<
    NewTickJoinPull<'a, Key, V1, V2, LhsState, RhsState>,
    SymmetricHashJoin<Lhs, Rhs, &'a mut LhsState, &'a mut RhsState, LhsState, RhsState>,
>;

/// Creates a symmetric hash join Pull from two input Pulls and their join states.
///
/// For `is_new_tick = true`, this first drains both inputs into their respective states,
/// then returns an iterator over all matches.
///
/// For `is_new_tick = false`, this returns a streaming join that processes items as they arrive.
pub async fn symmetric_hash_join<'a, Key, Lhs, V1, Rhs, V2, LhsState, RhsState>(
    lhs: Lhs,
    rhs: Rhs,
    lhs_state: &'a mut LhsState,
    rhs_state: &'a mut RhsState,
    is_new_tick: bool,
) -> SymmetricHashJoinEither<'a, Key, V1, V2, Lhs, Rhs, LhsState, RhsState>
where
    Key: 'a + Eq + std::hash::Hash + Clone,
    V1: 'a + Clone,
    V2: 'a + Clone,
    Lhs: 'a + FusedPull<Item = (Key, V1), Meta = ()>,
    Rhs: 'a + FusedPull<Item = (Key, V2), Meta = ()>,
    LhsState: HalfJoinState<Key, V1, V2>,
    RhsState: HalfJoinState<Key, V2, V1>,
{
    if is_new_tick {
        // Drain both inputs first
        drain_pull_into_state(std::pin::pin!(lhs), lhs_state).await;
        drain_pull_into_state(std::pin::pin!(rhs), rhs_state).await;

        let iter = if lhs_state.len() < rhs_state.len() {
            NewTickJoinIter::new_lhs_smaller(lhs_state, rhs_state)
        } else {
            NewTickJoinIter::new_rhs_smaller(lhs_state, rhs_state)
        };
        SymmetricHashJoinEither::Left(NewTickJoinPull { iter })
    } else {
        SymmetricHashJoinEither::Right(SymmetricHashJoin::new(lhs, rhs, lhs_state, rhs_state))
    }
}

/// Helper to drain a Pull into state.
fn drain_pull_into_state<Key, ValBuild, ValProbe, P, State>(
    mut pull: Pin<&mut P>,
    state: &mut State,
) -> impl Future<Output = ()>
where
    Key: Eq + std::hash::Hash + Clone,
    ValBuild: Clone,
    P: Pull<Item = (Key, ValBuild)>,
    State: HalfJoinState<Key, ValBuild, ValProbe>,
{
    std::future::poll_fn(move |ctx| {
        let ctx = Context::from_task(ctx);
        loop {
            return match pull.as_mut().pull(ctx) {
                PullStep::Ready((k, v), _meta) => {
                    state.build(k, Cow::Owned(v));
                    continue;
                }
                PullStep::Pending(_) => std::task::Poll::Pending,
                PullStep::Ended(_) => std::task::Poll::Ready(()),
            };
        }
    })
}
