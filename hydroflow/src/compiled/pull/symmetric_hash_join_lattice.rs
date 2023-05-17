use crate::lang::clear::Clear;
use lattices::map_union::MapUnion;
use lattices::{ConvertFrom, Merge};
use rustc_hash::{FxHashMap, FxHashSet};
use std::collections::hash_map::Entry;
use std::collections::hash_set;

pub struct HalfJoinStateLattice<K, Lattice> {
    table: MapUnion<FxHashMap<K, Lattice>>,
}

impl<K, Lattice> Default for HalfJoinStateLattice<K, Lattice> {
    fn default() -> Self {
        Self {
            table: Default::default(),
        }
    }
}

impl<K, Lattice> Clear for HalfJoinStateLattice<K, Lattice> {
    fn clear(&mut self) {
        self.table.0.clear()
    }
}

impl<K, Lattice> HalfJoinStateLattice<K, Lattice>
where
    K: Clone + Eq + std::hash::Hash,
{
    fn build<LatticeDelta>(&mut self, k: K, v: LatticeDelta) -> bool
    where
        Lattice: Merge<LatticeDelta> + ConvertFrom<LatticeDelta>,
    {
        let entry = self.table.0.entry(k);

        match entry {
            Entry::Occupied(mut e) => e.get_mut().merge(v),
            Entry::Vacant(e) => {
                e.insert(ConvertFrom::from(v));
                true
            }
        }
    }
}

pub type JoinStateLatticeMut<'a, K, LhsLattice, RhsLattice> = (
    &'a mut HalfJoinStateLattice<K, LhsLattice>,
    &'a mut HalfJoinStateLattice<K, RhsLattice>,
);

pub struct SymmetricHashJoinLattice<'a, K, LhsLattice, RhsLattice>
where
    K: Eq + std::hash::Hash + Clone,
{
    state: JoinStateLatticeMut<'a, K, LhsLattice, RhsLattice>,
    updated_keys: hash_set::Drain<'a, K>,
}

impl<'a, K, LhsLattice, RhsLattice> Iterator
    for SymmetricHashJoinLattice<'a, K, LhsLattice, RhsLattice>
where
    K: Eq + std::hash::Hash + Clone,
    LhsLattice: Clone,
    RhsLattice: Clone,
{
    type Item = (K, (LhsLattice, RhsLattice));

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(key) = self.updated_keys.next() {
            if let Some(lhs) = self.state.0.table.0.get(&key) {
                if let Some(rhs) = self.state.1.table.0.get(&key) {
                    return Some((key, (lhs.clone(), rhs.clone())));
                }
            }
        }

        None
    }
}
impl<'a, K, LhsLattice, RhsLattice> SymmetricHashJoinLattice<'a, K, LhsLattice, RhsLattice>
where
    K: Eq + std::hash::Hash + Clone,
{
    pub fn new_from_mut<I1, I2, LhsDelta, RhsDelta>(
        lhs: I1,
        rhs: I2,
        updated_keys: &'a mut FxHashSet<K>,
        state_lhs: &'a mut HalfJoinStateLattice<K, LhsLattice>,
        state_rhs: &'a mut HalfJoinStateLattice<K, RhsLattice>,
    ) -> Self
    where
        I1: Iterator<Item = (K, LhsDelta)>,
        I2: Iterator<Item = (K, RhsDelta)>,
        LhsLattice: Merge<LhsDelta> + ConvertFrom<LhsDelta>,
        RhsLattice: Merge<RhsDelta> + ConvertFrom<RhsDelta>,
    {
        for (k, v1) in lhs {
            if state_lhs.build(k.clone(), v1) {
                updated_keys.insert(k);
            }
        }

        for (k, v2) in rhs {
            if state_rhs.build(k.clone(), v2) {
                updated_keys.insert(k);
            }
        }

        Self {
            state: (state_lhs, state_rhs),
            updated_keys: updated_keys.drain(),
        }
    }
}

#[cfg(test)]
mod tests {
    pub type JoinStateLattice<K, LhsLattice, RhsLattice> = (
        HalfJoinStateLattice<K, LhsLattice>,
        HalfJoinStateLattice<K, RhsLattice>,
    );

    use super::{HalfJoinStateLattice, SymmetricHashJoinLattice};
    use lattices::ord::Max;
    use rustc_hash::FxHashSet;

    type MyLattice = Max<usize>;
    type JoinState = JoinStateLattice<usize, MyLattice, MyLattice>;

    fn join<
        Lhs: IntoIterator<Item = (usize, MyLattice)>,
        Rhs: IntoIterator<Item = (usize, MyLattice)>,
    >(
        state: &mut JoinState,
        lhs: Lhs,
        rhs: Rhs,
    ) -> Vec<(usize, (MyLattice, MyLattice))> {
        let mut updated_keys = FxHashSet::default();
        SymmetricHashJoinLattice::new_from_mut(
            lhs.into_iter(),
            rhs.into_iter(),
            &mut updated_keys,
            &mut state.0,
            &mut state.1,
        )
        .collect::<Vec<_>>()
    }

    #[test]
    fn produces_fully_merged_output() {
        let mut state = JoinState::default();

        let lhs = [(7, MyLattice::new(3)), (7, MyLattice::new(4))];
        let rhs = [(7, MyLattice::new(5)), (7, MyLattice::new(6))];
        assert_eq!(
            join(&mut state, lhs, rhs),
            vec![(7, (MyLattice::new(4), MyLattice::new(6)))]
        );
    }

    #[test]
    fn lattice_only_moves_forward() {
        let mut state = JoinState::default();

        let lhs = [(7, MyLattice::new(4)), (7, MyLattice::new(3))];
        let rhs = [(7, MyLattice::new(6)), (7, MyLattice::new(5))];
        assert_eq!(
            join(&mut state, lhs, rhs),
            vec![(7, (MyLattice::new(4), MyLattice::new(6)))]
        );
    }

    #[test]
    fn subsequent_ticks_dont_produce_if_nothing_has_changed() {
        let mut state = JoinState::default();

        let lhs = [(7, MyLattice::new(3))];
        let rhs = [(7, MyLattice::new(3))];
        assert_eq!(
            join(&mut state, lhs, rhs),
            vec![(7, (Max::new(3), Max::new(3)))]
        );

        let lhs = [(7, Max::new(3))];
        let rhs = [(7, Max::new(3))];
        assert_eq!(join(&mut state, lhs, rhs), vec![]);
    }

    #[test]
    fn subsequent_ticks_do_produce_if_something_has_changed() {
        let mut state = JoinState::default();

        let lhs = [(7, MyLattice::new(3))];
        let rhs = [(7, MyLattice::new(3))];
        assert_eq!(
            join(&mut state, lhs, rhs),
            vec![(7, (MyLattice::new(3), MyLattice::new(3)))]
        );

        let lhs = [(7, MyLattice::new(3))];
        let rhs = [(7, MyLattice::new(4))];
        assert_eq!(
            join(&mut state, lhs, rhs),
            vec![(7, (MyLattice::new(3), MyLattice::new(4)))]
        );
    }

    #[test]
    fn resetting_one_side_works() {
        let mut state = JoinState::default();

        let lhs = [(7, MyLattice::new(3))];
        let rhs = [(7, MyLattice::new(3))];
        assert_eq!(
            join(&mut state, lhs, rhs),
            vec![(7, (MyLattice::new(3), MyLattice::new(3)))]
        );

        std::mem::take(&mut state.1);

        let lhs = [(7, Max::new(3))];
        let rhs = [(7, Max::new(3))];
        assert_eq!(
            join(&mut state, lhs, rhs),
            vec![(7, (MyLattice::new(3), MyLattice::new(3)))]
        );
    }
}
