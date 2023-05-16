use std::cmp::Ordering;
use std::collections::BTreeMap;

use hydroflow::lattices::{ConvertFrom, LatticeOrd, Merge};
use itertools::Itertools;
use serde::{Deserialize, Serialize};

/// SealedSetOfIndexedValues is a lattice that represents a set of values with fixed size that gets set along the way.
#[derive(Debug, Clone, Eq, Serialize, Deserialize)] // TODO!!!!!!!!!! LATTICE PARTIAL EQ
pub struct SealedSetOfIndexedValues<T> {
    pub set: BTreeMap<usize, T>,
    pub seal: Option<usize>,
}

impl<T> Default for SealedSetOfIndexedValues<T> {
    fn default() -> Self {
        let (set, seal) = Default::default();
        Self { set, seal }
    }
}

impl<T: std::fmt::Debug + Eq> Merge<Self> for SealedSetOfIndexedValues<T> {
    fn merge(&mut self, delta: Self) -> bool {
        let mut changed = false;
        for (i, v) in delta.set {
            match self.set.entry(i) {
                std::collections::btree_map::Entry::Occupied(occupied) => {
                    // TODO: This is a bit of a hack. We should do this without erroring on conflict somehow.
                    assert_eq!(occupied.get(), &v);
                }
                std::collections::btree_map::Entry::Vacant(vacant) => {
                    vacant.insert(v);
                    changed = true;
                }
            }
        }
        self.seal = match (self.seal.take(), delta.seal) {
            (Some(self_seal_some), Some(delta_seal_some)) => {
                assert_eq!(self_seal_some, delta_seal_some);
                Some(self_seal_some)
            }
            (None, Some(delta_seal_some)) => {
                changed = true;
                Some(delta_seal_some)
            }
            (Some(self_seal_some), None) => Some(self_seal_some),
            (None, None) => None,
        };
        changed
    }
}

impl<T: Eq + std::fmt::Debug> PartialOrd<Self> for SealedSetOfIndexedValues<T> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        let mut self_greater = false;
        let mut other_greater = false;

        // Unsealed compare (None, None): return indexset compare on the keys
        // Both sealed compare (Some, Some): return indexset compare on the keys
        // One sealed compare (None, Some) or (Some, None): check indexset compare on the keys. If a tie (equal), break tie with the sealed as winner.

        // (.keys() is sorted, do mergesort merge)
        for key in self.set.keys().merge(other.set.keys()).dedup() {
            match (self.set.get(key), other.set.get(key)) {
                (Some(self_value), Some(other_value)) => {
                    assert_eq!(self_value, other_value);
                }
                (Some(_), None) => {
                    self_greater = true;
                }
                (None, Some(_)) => {
                    other_greater = true;
                }
                (None, None) => unreachable!(),
            }
        }
        match (&self.seal, &other.seal) {
            (Some(self_seal_some), Some(delta_seal_some)) => {
                assert_eq!(self_seal_some, delta_seal_some);
            }
            (None, Some(_)) => {
                other_greater = true;
            }
            (Some(_), None) => {
                self_greater = true;
            }
            (None, None) => {}
        };
        match (self_greater, other_greater) {
            (true, true) => None,
            (true, false) => Some(Ordering::Greater),
            (false, true) => Some(Ordering::Less),
            (false, false) => Some(Ordering::Equal),
        }
    }
}
impl<T: Eq> PartialEq<Self> for SealedSetOfIndexedValues<T> {
    fn eq(&self, other: &Self) -> bool {
        Some(Ordering::Equal) == self.partial_cmp(other)
    }
}
impl<T: Eq + std::fmt::Debug> LatticeOrd<Self> for SealedSetOfIndexedValues<T> {}

impl<T> ConvertFrom<Self> for SealedSetOfIndexedValues<T> {
    fn from(other: Self) -> Self {
        other
    }
}

/// VecPrefix is a lattice that represents prefixes of a fixed-length vector whose length is set along the way.
#[derive(Debug, Clone, Eq, Serialize, Deserialize)]
pub struct VecPrefix<T> {
    pub vec: Vec<T>,
    pub seal: Option<usize>,
}

impl<T> Default for VecPrefix<T> {
    fn default() -> Self {
        let (vec, seal) = Default::default();
        Self { vec, seal }
    }
}

impl<T: Eq> Merge<Self> for VecPrefix<T> {
    fn merge(&mut self, delta: Self) -> bool {
        let mut changed = false;
        self.seal = match (self.seal.take(), delta.seal) {
            (Some(self_seal_some), Some(delta_seal_some)) => {
                assert_eq!(self_seal_some, delta_seal_some);
                Some(self_seal_some)
            }
            (None, Some(delta_seal_some)) => {
                changed = true;
                Some(delta_seal_some)
            }
            (Some(self_seal_some), None) => Some(self_seal_some),
            (None, None) => None,
        };
        if delta.vec.starts_with(&self.vec) {
            if self.vec.len() < delta.vec.len() {
                self.vec = delta.vec;
                changed = true;
            }
        } else {
            assert!(self.vec.starts_with(&delta.vec));
        }
        if let Some(self_seal_some) = self.seal {
            assert!(self.vec.len() <= self_seal_some);
        }
        changed
    }
}

impl<T: Eq> PartialOrd<Self> for VecPrefix<T> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        if let (Some(self_seal_some), Some(other_seal_some)) = (self.seal, other.seal) {
            assert_eq!(self_seal_some, other_seal_some);
        }
        assert!(self
            .seal
            .map_or(true, |self_seal_some| self.vec.len() <= self_seal_some));
        assert!(other
            .seal
            .map_or(true, |other_seal_some| other.vec.len() <= other_seal_some));

        if other.vec.starts_with(&self.vec) {
            if self.vec.len() < other.vec.len() {
                Some(Ordering::Less)
            } else {
                Some(Ordering::Equal)
            }
        } else {
            assert!(self.vec.starts_with(&other.vec));
            Some(Ordering::Less)
        }
    }
}
impl<T: Eq> PartialEq<Self> for VecPrefix<T> {
    fn eq(&self, other: &Self) -> bool {
        Some(Ordering::Equal) == self.partial_cmp(other)
    }
}
impl<T: Eq> LatticeOrd<Self> for VecPrefix<T> {}

#[cfg(test)]
mod test {
    #[test]
    fn test_ssid() {
        let mut test_vec = Vec::new();

        for seal in [None, Some(2)] {
            for vec in [
                vec![],
                vec![(0, "hello")],
                vec![(1, "world")],
                vec![(0, "hello"), (1, "world")],
            ] {
                let set = vec.into_iter().collect();
                test_vec.push(SealedSetOfIndexedValues { set, seal })
            }
        }

        hydroflow::lattices::test::assert_partial_ord_identities(&test_vec);
        hydroflow::lattices::test::assert_lattice_identities(&test_vec);
    }
}
