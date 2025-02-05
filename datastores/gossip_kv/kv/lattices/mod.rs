use std::cmp::Ordering;
use std::collections::HashSet;
use std::hash::Hash;

use dfir_rs::lattices::{IsBot, IsTop, LatticeFrom, LatticeOrd, Merge};
use serde::{Deserialize, Serialize};

/// A bounded set union lattice with a fixed size N.
///
/// Once the set reaches size N, it becomes top. The items in the set are no longer tracked to
/// reclaim associated memory.
#[derive(Debug, Clone, Eq, Serialize, Deserialize)]

pub struct BoundedSetLattice<T, const N: usize>
where
    T: Eq + Hash,
{
    // The set of items in the lattice with invariant:
    // is_top => items.is_empty() ... i.e. the items are dropped when the lattice reaches top.
    items: HashSet<T>,
    is_top: bool,
}

impl<T, const N: usize> LatticeFrom<BoundedSetLattice<T, N>> for BoundedSetLattice<T, N>
where
    T: Eq + Hash,
{
    fn lattice_from(other: BoundedSetLattice<T, N>) -> Self {
        other
    }
}

impl<T, const N: usize> Default for BoundedSetLattice<T, N>
where
    T: Eq + Hash,
{
    fn default() -> Self {
        Self {
            items: HashSet::new(),
            is_top: N == 0, // This lattice is effectively a unit lattice `()`, if N == 0
        }
    }
}

impl<T> From<()> for BoundedSetLattice<T, 0>
where
    T: Eq + Hash,
{
    fn from(_: ()) -> Self {
        Default::default()
    }
}

impl<T> From<BoundedSetLattice<T, 0>> for ()
where
    T: Eq + Hash,
{
    fn from(_: BoundedSetLattice<T, 0>) -> Self {}
}

impl<T, const N: usize> BoundedSetLattice<T, N>
where
    T: Eq + Hash,
{
    pub fn new() -> Self {
        Default::default()
    }

    pub fn new_from<U>(items: U) -> Self
    where
        U: IntoIterator<Item = T>,
    {
        let mut lattice = Self::new();
        lattice.merge(items);
        lattice
    }
}

impl<T, const N: usize> IsBot for BoundedSetLattice<T, N>
where
    T: Eq + Hash,
{
    fn is_bot(&self) -> bool {
        match N {
            0 => true,
            _ => self.items.is_empty() && !self.is_top,
        }
    }
}

impl<T, const N: usize> IsTop for BoundedSetLattice<T, N>
where
    T: Eq + Hash,
{
    fn is_top(&self) -> bool {
        self.is_top
    }
}

impl<T, const N: usize, U> Merge<U> for BoundedSetLattice<T, N>
where
    U: IntoIterator<Item = T>,
    T: Eq + Hash,
{
    fn merge(&mut self, other: U) -> bool {
        if self.is_top {
            return false;
        }

        let old_len = self.items.len();
        self.items.extend(other);
        let new_len = self.items.len();

        if new_len >= N {
            self.is_top = true;
            self.items.clear();
        }

        new_len != old_len
    }
}

impl<T, const N: usize> PartialOrd<Self> for BoundedSetLattice<T, N>
where
    T: Eq + Hash,
{
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        match (self.is_top, other.is_top) {
            (true, true) => Some(Ordering::Equal),
            (true, false) => Some(Ordering::Greater),
            (false, true) => Some(Ordering::Less),
            (false, false) => match self.items.len().cmp(&other.items.len()) {
                Ordering::Greater => {
                    if other.items.iter().all(|key| self.items.contains(key)) {
                        Some(Ordering::Greater)
                    } else {
                        None
                    }
                }
                Ordering::Less => {
                    if self.items.iter().all(|key| other.items.contains(key)) {
                        Some(Ordering::Less)
                    } else {
                        None
                    }
                }
                Ordering::Equal => {
                    if self.items.iter().all(|key| other.items.contains(key)) {
                        Some(Ordering::Equal)
                    } else {
                        None
                    }
                }
            },
        }
    }
}

impl<T, const N: usize> PartialEq<Self> for BoundedSetLattice<T, N>
where
    T: Eq + Hash,
{
    fn eq(&self, other: &Self) -> bool {
        match (self.is_top, other.is_top) {
            (true, true) => true,
            (true, false) => false,
            (false, true) => false,
            (false, false) => self.items == other.items,
        }
    }
}

impl<T, const N: usize> LatticeOrd for BoundedSetLattice<T, N> where T: Eq + Hash {}

impl<T, const N: usize> Merge<BoundedSetLattice<T, N>> for BoundedSetLattice<T, N>
where
    T: Eq + Hash,
{
    fn merge(&mut self, other: BoundedSetLattice<T, N>) -> bool {
        match (self.is_top, other.is_top) {
            (true, _) => false,
            (false, true) => {
                self.is_top = true;
                self.items.clear();
                true
            }
            (false, false) => self.merge(other.items),
        }
    }
}

#[cfg(test)]
mod tests {
    use dfir_rs::lattices::test::check_all;

    use super::*;

    #[test]
    fn test_0_bounded_set_lattice() {
        let mut lat: BoundedSetLattice<i32, 0> = ().into();
        assert!(lat.is_bot() && lat.is_top());

        // Merges should always return false.
        assert!(!lat.merge([1]));

        // No changes to top/bot status.
        assert!(lat.is_bot() && lat.is_top());
    }

    #[test]
    fn test_1_bounded_set_lattice() {
        // The bounded lattice with N = 1 is effectively a WithBottom<T> lattice.
        let mut lat = BoundedSetLattice::<i32, 1>::new();
        assert!(lat.is_bot() && !lat.is_top());
        assert!(lat.items.is_empty());

        assert!(lat.merge([1]));
        assert!(!lat.is_bot() && lat.is_top());
        assert!(lat.items.is_empty()); // Check that the items were dropped.

        assert!(!lat.merge([2]));
    }

    #[test]
    fn test_2_bounded_set_lattice() {
        let mut a = BoundedSetLattice::<i32, 2>::new();
        let b: BoundedSetLattice<i32, 2> = BoundedSetLattice::new_from([1, 2]);

        assert!(a.is_bot() && !a.is_top());
        assert!(!b.is_bot() && b.is_top());

        assert!(a.merge(b));
        assert!(!a.is_bot() && a.is_top());

        assert!(!a.merge([3]));
    }

    #[test]
    fn test_lattice_properties() {
        check_all(&[
            Default::default(),
            BoundedSetLattice::<i32, 3>::new_from([1]),
            BoundedSetLattice::<i32, 3>::new_from([1, 2]),
            BoundedSetLattice::<i32, 3>::new_from([1, 2, 3]),
            BoundedSetLattice::<i32, 3>::new_from([1, 2, 3, 4]),
        ]);
    }
}
