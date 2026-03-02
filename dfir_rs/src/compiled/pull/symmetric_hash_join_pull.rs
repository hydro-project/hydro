use std::borrow::Cow;
use std::pin::Pin;

use dfir_pipes::{Context, Pull, Step, Toggle, Yes};
use pin_project_lite::pin_project;

use super::HalfJoinState;

pin_project! {
    /// Pull combinator for symmetric hash join operations.
    #[must_use = "pulls do nothing unless polled"]
    pub struct SymmetricHashJoin<'a, Lhs, Rhs, LhsState, RhsState>
    {
        #[pin]
        lhs: Lhs,
        #[pin]
        rhs: Rhs,

        lhs_state: &'a mut LhsState,
        rhs_state: &'a mut RhsState,

        lhs_ended: bool,
        rhs_ended: bool,
    }
}

impl<'a, Lhs, Rhs, LhsState, RhsState> SymmetricHashJoin<'a, Lhs, Rhs, LhsState, RhsState> {
    /// Creates a new symmetric hash join Pull from two input Pulls and their join states.
    pub fn new(
        lhs: Lhs,
        rhs: Rhs,
        lhs_state: &'a mut LhsState,
        rhs_state: &'a mut RhsState,
    ) -> Self {
        Self {
            lhs,
            rhs,
            lhs_state,
            rhs_state,
            lhs_ended: false,
            rhs_ended: false,
        }
    }
}

impl<'a, Key, Lhs, V1, Rhs, V2, LhsState, RhsState> Pull
    for SymmetricHashJoin<'a, Lhs, Rhs, LhsState, RhsState>
where
    Key: Eq + std::hash::Hash + Clone,
    V1: Clone,
    V2: Clone,
    Lhs: Pull<Item = (Key, V1), Meta = ()>,
    Rhs: Pull<Item = (Key, V2), Meta = ()>,
    LhsState: HalfJoinState<Key, V1, V2>,
    RhsState: HalfJoinState<Key, V2, V1>,
{
    type Ctx<'ctx> = <Lhs::Ctx<'ctx> as Context<'ctx>>::Merged<Rhs::Ctx<'ctx>>;

    type Item = (Key, (V1, V2));
    type Meta = ();
    type CanPend = <Lhs::CanPend as Toggle>::Or<Rhs::CanPend>;
    type CanEnd = <Lhs::CanEnd as Toggle>::And<Rhs::CanEnd>;

    fn pull(
        self: Pin<&mut Self>,
        ctx: &mut Self::Ctx<'_>,
    ) -> Step<Self::Item, Self::Meta, Self::CanPend, Self::CanEnd> {
        let mut this = self.project();

        loop {
            // First check for any pending matches from previous probes
            if let Some((k, v2, v1)) = this.lhs_state.pop_match() {
                return Step::Ready((k, (v1, v2)), ());
            }
            if let Some((k, v1, v2)) = this.rhs_state.pop_match() {
                return Step::Ready((k, (v1, v2)), ());
            }

            // Both ended - return Ended
            if *this.lhs_ended && *this.rhs_ended {
                return Step::Ended(Toggle::convert_from(Yes));
            }

            // Try to pull from lhs if not ended
            if !*this.lhs_ended {
                match this
                    .lhs
                    .as_mut()
                    .pull(<Lhs::Ctx<'_> as Context<'_>>::unmerge_self(ctx))
                {
                    Step::Ready((k, v1), _meta) => {
                        if this.lhs_state.build(k.clone(), Cow::Borrowed(&v1)) {
                            if let Some((k, v1, v2)) = this.rhs_state.probe(&k, &v1) {
                                return Step::Ready((k, (v1, v2)), ());
                            }
                        }
                        continue;
                    }
                    Step::Pending(can_pend) => {
                        return Step::Pending(Toggle::convert_from(can_pend));
                    }
                    Step::Ended(_) => {
                        *this.lhs_ended = true;
                    }
                }
            }

            // Try to pull from rhs if not ended
            if !*this.rhs_ended {
                match this
                    .rhs
                    .as_mut()
                    .pull(<Lhs::Ctx<'_> as Context<'_>>::unmerge_other(ctx))
                {
                    Step::Ready((k, v2), _meta) => {
                        if this.rhs_state.build(k.clone(), Cow::Borrowed(&v2)) {
                            if let Some((k, v2, v1)) = this.lhs_state.probe(&k, &v2) {
                                return Step::Ready((k, (v1, v2)), ());
                            }
                        }
                        continue;
                    }
                    Step::Pending(can_pend) => {
                        return Step::Pending(Toggle::convert_from(can_pend));
                    }
                    Step::Ended(_) => {
                        *this.rhs_ended = true;
                    }
                }
            }

            // If we get here, both sides have ended this iteration
            if *this.lhs_ended && *this.rhs_ended {
                return Step::Ended(Toggle::convert_from(Yes));
            }
        }
    }
}

/// Iterator for new tick - iterates over all matches after both sides are drained.
pub struct NewTickJoinIter<'a, Key, V1, V2, LhsState, RhsState> {
    lhs_state: &'a LhsState,
    rhs_state: &'a RhsState,
    lhs_smaller: bool,
    // State for iteration
    outer_iter: Option<std::collections::hash_map::Iter<'a, Key, smallvec::SmallVec<[V1; 1]>>>,
    outer_iter_rhs: Option<std::collections::hash_map::Iter<'a, Key, smallvec::SmallVec<[V2; 1]>>>,
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
    type CanPend = dfir_pipes::No;
    type CanEnd = Yes;

    fn pull(
        self: Pin<&mut Self>,
        _ctx: &mut Self::Ctx<'_>,
    ) -> Step<Self::Item, Self::Meta, Self::CanPend, Self::CanEnd> {
        let this = self.project();
        match this.iter.next() {
            Some(item) => Step::Ready(item, ()),
            None => Step::Ended(Yes),
        }
    }
}

pin_project! {
    #[project = SymmetricHashJoinEitherProj]
    /// Enum to hold either the new-tick Pull or the streaming Pull.
    pub enum SymmetricHashJoinEither<'a, Key, V1, V2, Lhs, Rhs, LhsState, RhsState>
    where
        Key: Clone,
        V1: Clone,
        V2: Clone,
    {
        NewTick {
            #[pin]
            pull: NewTickJoinPull<'a, Key, V1, V2, LhsState, RhsState>,
        },
        Streaming {
            #[pin]
            pull: SymmetricHashJoin<'a, Lhs, Rhs, LhsState, RhsState>,
        },
    }
}

impl<'a, Key, V1, V2, Lhs, Rhs, LhsState, RhsState> Pull
    for SymmetricHashJoinEither<'a, Key, V1, V2, Lhs, Rhs, LhsState, RhsState>
where
    Key: Eq + std::hash::Hash + Clone,
    V1: Clone,
    V2: Clone,
    Lhs: Pull<Item = (Key, V1), Meta = ()>,
    Rhs: Pull<Item = (Key, V2), Meta = ()>,
    LhsState: HalfJoinState<Key, V1, V2>,
    RhsState: HalfJoinState<Key, V2, V1>,
{
    type Ctx<'ctx> = <Lhs::Ctx<'ctx> as Context<'ctx>>::Merged<Rhs::Ctx<'ctx>>;

    type Item = (Key, (V1, V2));
    type Meta = ();
    type CanPend = <SymmetricHashJoin<'a, Lhs, Rhs, LhsState, RhsState> as Pull>::CanPend;
    type CanEnd = Yes;

    fn pull(
        self: Pin<&mut Self>,
        ctx: &mut Self::Ctx<'_>,
    ) -> Step<Self::Item, Self::Meta, Self::CanPend, Self::CanEnd> {
        match self.project() {
            SymmetricHashJoinEitherProj::NewTick { pull } => pull.pull(&mut ()).remap(),
            SymmetricHashJoinEitherProj::Streaming { pull } => pull.pull(ctx).remap(),
        }
    }
}

/// Helper trait to allow remapping Step types
trait StepRemap<Item, Meta, CanPend: Toggle, CanEnd: Toggle> {
    fn remap<NewPend: Toggle, NewEnd: Toggle>(self) -> Step<Item, Meta, NewPend, NewEnd>;
}

impl<Item, Meta, CanPend: Toggle, CanEnd: Toggle> StepRemap<Item, Meta, CanPend, CanEnd>
    for Step<Item, Meta, CanPend, CanEnd>
{
    fn remap<NewPend: Toggle, NewEnd: Toggle>(self) -> Step<Item, Meta, NewPend, NewEnd> {
        match self {
            Step::Ready(item, meta) => Step::Ready(item, meta),
            Step::Pending(can_pend) => Step::Pending(Toggle::convert_from(can_pend)),
            Step::Ended(can_end) => Step::Ended(Toggle::convert_from(can_end)),
        }
    }
}

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
    Lhs: 'a + Pull<Item = (Key, V1), Meta = ()>,
    Rhs: 'a + Pull<Item = (Key, V2), Meta = ()>,
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
        SymmetricHashJoinEither::NewTick {
            pull: NewTickJoinPull { iter }, // TODO(mingwei): pre-build the state the old way.
        }
    } else {
        SymmetricHashJoinEither::Streaming {
            pull: SymmetricHashJoin::new(lhs, rhs, lhs_state, rhs_state),
        }
    }
}

/// Helper to drain a Pull into state (synchronous only)
fn drain_pull_into_state<'a, Key, ValBuild, ValProbe, P, State>(
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
                Step::Ready((k, v), _meta) => {
                    state.build(k, Cow::Owned(v));
                    continue;
                }
                Step::Pending(_) => std::task::Poll::Pending,
                Step::Ended(_) => std::task::Poll::Ready(()),
            };
        }
    })
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;
    use std::pin::pin;

    use dfir_pipes::{Pull, Step};

    use super::super::HalfSetJoinState;
    use super::*;

    #[tokio::test]
    async fn hash_join() {
        let lhs = dfir_pipes::from_iter((0..10).map(|x| (x, format!("left {}", x))));
        let rhs = dfir_pipes::from_iter((6..15).map(|x| (x / 2, format!("right {} / 2", x))));

        let (mut lhs_state, mut rhs_state) =
            (HalfSetJoinState::default(), HalfSetJoinState::default());
        let join = symmetric_hash_join(lhs, rhs, &mut lhs_state, &mut rhs_state, true).await;

        let mut pinned = pin!(join);
        let mut joined = HashSet::new();
        loop {
            match pinned.as_mut().pull(&mut ()) {
                Step::Ready(item, _) => {
                    joined.insert(item);
                }
                Step::Ended(_) => break,
                Step::Pending(_) => unreachable!(),
            }
        }

        assert!(joined.contains(&(3, ("left 3".into(), "right 6 / 2".into()))));
        assert!(joined.contains(&(3, ("left 3".into(), "right 7 / 2".into()))));
        assert!(joined.contains(&(4, ("left 4".into(), "right 8 / 2".into()))));
        assert!(joined.contains(&(4, ("left 4".into(), "right 9 / 2".into()))));
        assert!(joined.contains(&(5, ("left 5".into(), "right 10 / 2".into()))));
        assert!(joined.contains(&(5, ("left 5".into(), "right 11 / 2".into()))));
        assert!(joined.contains(&(6, ("left 6".into(), "right 12 / 2".into()))));
        assert!(joined.contains(&(6, ("left 6".into(), "right 13 / 2".into()))));
        assert!(joined.contains(&(7, ("left 7".into(), "right 14 / 2".into()))));
        assert_eq!(9, joined.len());
    }

    #[tokio::test]
    async fn hash_join_streaming() {
        // Test the streaming (non-new-tick) case
        let lhs = dfir_pipes::from_iter(vec![(1, "a"), (2, "b"), (1, "c")]);
        let rhs = dfir_pipes::from_iter(vec![(1, 10), (3, 30), (1, 11)]);

        let (mut lhs_state, mut rhs_state): (HalfSetJoinState<_, _, _>, HalfSetJoinState<_, _, _>) =
            (HalfSetJoinState::default(), HalfSetJoinState::default());
        let join = symmetric_hash_join(lhs, rhs, &mut lhs_state, &mut rhs_state, false).await;

        let mut pinned = pin!(join);
        let mut joined = HashSet::new();
        loop {
            match pinned.as_mut().pull(&mut ()) {
                Step::Ready(item, _) => {
                    joined.insert(item);
                }
                Step::Ended(_) => break,
                Step::Pending(_) => unreachable!(),
            }
        }

        // Should have matches for key 1
        assert!(joined.contains(&(1, ("a", 10))));
        assert!(joined.contains(&(1, ("a", 11))));
        assert!(joined.contains(&(1, ("c", 10))));
        assert!(joined.contains(&(1, ("c", 11))));
        assert_eq!(4, joined.len());
    }
}
