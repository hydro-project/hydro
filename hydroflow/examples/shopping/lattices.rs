use std::cmp::Ordering;
use std::collections::BTreeMap;

use hydroflow::lattices::{ConvertFrom, LatticeOrd, Merge};
use itertools::Itertools;
use serde::{Deserialize, Serialize};

/// SealedSetofIndexedValues represents a vector of values indexed by integers [0..len-1].
/// We assume that the value at each index is unknown but fixed, and the len
/// is unknown but fixed. Over time the values and len are revealed in arbitrary order.
/// If we receive two distinct values for the same index, that is out of spec and we raise
/// an error. Similarly if we get receive two distinct values for len.
#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub struct SealedSetOfIndexedValues<T> {
    pub set: BTreeMap<usize, T>,
    pub len: Option<usize>,
}

impl<T> Default for SealedSetOfIndexedValues<T> {
    fn default() -> Self {
        let (set, seal) = Default::default();
        Self { set, len: seal }
    }
}

impl<T: std::fmt::Debug + Eq> Merge<Self> for SealedSetOfIndexedValues<T> {
    fn merge(&mut self, delta: Self) -> bool {
        let mut changed = false;
        for (i, v) in delta.set {
            match self.set.entry(i) {
                std::collections::btree_map::Entry::Occupied(occupied) => {
                    // TODO: This is a runtime type error -- we're trying to merge data from two differently
                    // defined lattices. Would be nice to do something more graceful than error out.
                    assert_eq!(occupied.get(), &v);
                }
                std::collections::btree_map::Entry::Vacant(vacant) => {
                    vacant.insert(v);
                    changed = true;
                }
            }
        }
        self.len = match (self.len.take(), delta.len) {
            (Some(self_len_some), Some(delta_len_some)) => {
                // TODO: If the lens don't match it's a runtime type error -- we're trying to merge data from
                // two differently defined lattices. Would be nice to do something more graceful than error out.
                assert_eq!(self_len_some, delta_len_some);
                Some(self_len_some)
            }
            (None, Some(delta_len_some)) => {
                changed = true;
                Some(delta_len_some)
            }
            (Some(self_len_some), None) => Some(self_len_some),
            (None, None) => None,
        };
        changed
    }
}

impl<T: Eq + std::fmt::Debug> PartialOrd<Self> for SealedSetOfIndexedValues<T> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        // `self` has some field indicating it is greater than `other`.
        let mut self_greater = false;
        // `other` has some field indicating is it greater than `self`.
        let mut other_greater = false;

        // Unsealed comparison (None, None): return indexset compare on the keys
        // Both-sealed comparison (Some(len), Some(len)): return indexset compare on the keys
        // One-sealed comparison (None, Some(len)) or (Some(len), None):
        //  if indexset of the one that is Some(len) is bigger or equal, return that one
        //  else (conflict of seal and indexset compare) return None

        // first compare the indexsets
        // (.keys() is sorted, do "mergesort"-style merge)
        for key in self.set.keys().merge(other.set.keys()).dedup() {
            match (self.set.get(key), other.set.get(key)) {
                (Some(self_value), Some(other_value)) => {
                    // TODO: This is a runtime type error -- we're trying to merge data from two differently
                    // defined lattices. Would be nice to do something more graceful than error out.
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
            // once a conflict is found we can stop comparing
            if self_greater && other_greater {
                return None;
            }
        }

        // next, compare the len's
        match (&self.len, &other.len) {
            (Some(self_len_some), Some(delta_len_some)) => {
                // TODO: This is a runtime type error -- we're trying to merge data from two differently
                // defined lattices. Would be nice to do something more graceful than error out.
                assert_eq!(self_len_some, delta_len_some);
            }
            (None, Some(_)) => {
                other_greater = true;
            }
            (Some(_), None) => {
                self_greater = true;
            }
            (None, None) => {}
        }

        match (self_greater, other_greater) {
            (true, true) => None,
            (true, false) => Some(Ordering::Greater),
            (false, true) => Some(Ordering::Less),
            (false, false) => Some(Ordering::Equal),
        }
    }
}
impl<T: Eq + std::fmt::Debug> LatticeOrd<Self> for SealedSetOfIndexedValues<T> {}

impl<T> ConvertFrom<Self> for SealedSetOfIndexedValues<T> {
    fn from(other: Self) -> Self {
        other
    }
}

/// BoundedPrefix is a lattice that represents prefixes of a fixed-length vector
/// whose length is defined along the way. We assume that the vector entries
/// and the length are unknown but fixed. Over time the entries are revealed in order,
/// and the len is revealed at any time.
/// If we receive two distinct values for the same entry, that is out of spec and we raise
/// an error. Similarly if we get receive two distinct values for len.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BoundedPrefix<T> {
    pub vec: Vec<T>,
    pub len: Option<usize>,
}

impl<T> Default for BoundedPrefix<T> {
    fn default() -> Self {
        let (vec, len) = Default::default();
        Self { vec, len }
    }
}

impl<T: Eq> Merge<Self> for BoundedPrefix<T> {
    fn merge(&mut self, delta: Self) -> bool {
        let mut changed = false;
        self.len = match (self.len.take(), delta.len) {
            (Some(self_len_some), Some(delta_len_some)) => {
                // TODO: If the assertion fails, this is a runtime type error -- we're trying to merge
                // data from two differently defined lattices. Would be nice to do something more graceful than error out.
                assert_eq!(self_len_some, delta_len_some);
                Some(self_len_some)
            }
            (None, Some(delta_len_some)) => {
                changed = true;
                Some(delta_len_some)
            }
            (Some(self_len_some), None) => Some(self_len_some),
            (None, None) => None,
        };
        if delta.vec.starts_with(&self.vec) {
            if self.vec.len() < delta.vec.len() {
                self.vec = delta.vec;
                changed = true;
            }
        } else {
            // TODO: This is a runtime type error -- if the assertion fails, we're trying to merge
            // data from two differently defined lattices. Would be nice to do something more graceful than error out.
            assert!(self.vec.starts_with(&delta.vec));
        }
        if let Some(self_len_some) = self.len {
            // TODO: This is a runtime type error -- if the assertion fails, we're trying to merge
            // data from two differently defined lattices. Would be nice to do something more graceful than error out.
            assert!(self.vec.len() <= self_len_some);
        }
        changed
    }
}

impl<T: Eq> PartialOrd<Self> for BoundedPrefix<T> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        // ensure each vec is not in internal conflict with its len
        assert!(self
            .len
            .map_or(true, |self_len_some| self.vec.len() <= self_len_some));
        assert!(other
            .len
            .map_or(true, |other_len_some| other.vec.len() <= other_len_some));

        // `self` has some field indicating it is greater than `other`.
        let mut self_greater = false;
        // `other` has some field indicating is it greater than `self`.
        let mut other_greater = false;

        // check if one vec is a prefix of the other
        if other.vec.starts_with(&self.vec) && self.vec.len() < other.vec.len() {
            other_greater = true;
        } else {
            // TODO: This is a runtime type error -- if the assertion fails, we're trying to merge
            // data from two differently defined lattices. Would be nice to do something more graceful than error out.
            assert!(self.vec.starts_with(&other.vec));
            if self.vec.len() > other.vec.len() {
                self_greater = true;
            }
        }

        // vecs are the same, so compare on presence of len
        match (self.len, other.len) {
            (Some(_), None) => {
                self_greater = true;
            }
            (None, Some(_)) => {
                other_greater = true;
            }
            (Some(self_len), Some(other_len)) => {
                // ensure the two len's are not in conflict with each other
                assert_eq!(self_len, other_len);
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
impl<T: Eq> LatticeOrd<Self> for BoundedPrefix<T> {}

// use Hydroflow's built-in lattice tests to see if our lattices behave well
#[cfg(test)]
mod test {
    use super::{BoundedPrefix, SealedSetOfIndexedValues};

    #[test]
    fn test_ssiv() {
        let mut test_vec: Vec<SealedSetOfIndexedValues<&str>> = Vec::new();

        for len in [None, Some(2)] {
            for vec in [
                vec![],
                vec![(0, "hello")],
                vec![(1, "world")],
                vec![(0, "hello"), (1, "world")],
            ] {
                let set = vec.into_iter().collect();
                test_vec.push(SealedSetOfIndexedValues::<&str> { set, len })
            }
        }

        hydroflow::lattices::test::check_lattice_ord(&test_vec);
        hydroflow::lattices::test::check_partial_ord_properties(&test_vec);
        hydroflow::lattices::test::check_lattice_properties(&test_vec);
    }

    #[test]
    fn test_vec_prefix() {
        let mut test_vec: Vec<BoundedPrefix<&str>> = Vec::new();

        for len in [None, Some(2)] {
            for vec in [vec![], vec!["hello"], vec!["hello", "world"]] {
                test_vec.push(BoundedPrefix::<&str> { vec, len })
            }
        }

        hydroflow::lattices::test::check_lattice_ord(&test_vec);
        hydroflow::lattices::test::check_partial_ord_properties(&test_vec);
        hydroflow::lattices::test::check_lattice_properties(&test_vec);
    }
}
