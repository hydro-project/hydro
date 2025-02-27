use std::cmp::Ordering::{self, *};

use cc_traits::Iter;

use crate::{DeepReveal, IsBot, IsTop, LatticeFrom, LatticeOrd, Merge};

/// Vec-union compound lattice.
///
/// Contains any number of `Lat` sub-lattices. Sub-lattices are indexed starting at zero, merging
/// combines corresponding sub-lattices and keeps any excess.
///
/// Similar to [`MapUnion<<usize, Lat>>`](super::map_union::MapUnion) but requires the key indices
/// start with `0`, `1`, `2`, etc: i.e. integers starting at zero with no gaps.
#[derive(Clone, Debug, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct VecUnion<Lat> {
    vec: Vec<Lat>,
}

impl<Lat> VecUnion<Lat> {
    /// Create a new `VecUnion` from a `Vec` of `Lat` instances.
    pub fn new(vec: Vec<Lat>) -> Self {
        Self { vec }
    }

    /// Create a new `VecUnion` from an `Into<Vec<Lat>>`.
    pub fn new_from(vec: impl Into<Vec<Lat>>) -> Self {
        Self::new(vec.into())
    }

    /// Reveal the inner value as a shared reference.
    pub fn as_reveal_ref(&self) -> &Vec<Lat> {
        &self.vec
    }

    /// Reveal the inner value as an exclusive reference.
    pub fn as_reveal_mut(&mut self) -> &mut Vec<Lat> {
        &mut self.vec
    }

    /// Gets the inner by value, consuming self.
    pub fn into_reveal(self) -> Vec<Lat> {
        self.vec
    }
}

impl<Lat> DeepReveal for VecUnion<Lat>
where
    Lat: DeepReveal,
{
    type Revealed = Vec<Lat::Revealed>;

    fn deep_reveal(self) -> Self::Revealed {
        self.vec.into_iter().map(DeepReveal::deep_reveal).collect()
    }
}

impl<Lat> Default for VecUnion<Lat> {
    fn default() -> Self {
        Self {
            vec: Default::default(),
        }
    }
}

impl<LatSelf, LatOther> Merge<VecUnion<LatOther>> for VecUnion<LatSelf>
where
    LatSelf: Merge<LatOther> + LatticeFrom<LatOther>,
{
    fn merge(&mut self, mut other: VecUnion<LatOther>) -> bool {
        let mut changed = false;
        // Extend `self` if `other` is longer.
        if self.vec.len() < other.vec.len() {
            self.vec
                .extend(other.vec.drain(self.vec.len()..).map(LatSelf::lattice_from));
            changed = true;
        }
        // Merge intersecting indices.
        for (self_val, other_val) in self.vec.iter_mut().zip(other.vec) {
            changed |= self_val.merge(other_val);
        }
        changed
    }
}

impl<LatSelf, LatOther> LatticeFrom<VecUnion<LatOther>> for VecUnion<LatSelf>
where
    LatSelf: LatticeFrom<LatOther>,
{
    fn lattice_from(other: VecUnion<LatOther>) -> Self {
        Self::new(other.vec.into_iter().map(LatSelf::lattice_from).collect())
    }
}

impl<LatSelf, LatOther> PartialEq<VecUnion<LatOther>> for VecUnion<LatSelf>
where
    LatSelf: PartialEq<LatOther>,
{
    fn eq(&self, other: &VecUnion<LatOther>) -> bool {
        if self.vec.len() != other.vec.len() {
            false
        } else {
            self.vec
                .iter()
                .zip(other.vec.iter())
                .all(|(val_self, val_other)| val_self == val_other)
        }
    }
}

impl<LatSelf, LatOther> PartialOrd<VecUnion<LatOther>> for VecUnion<LatSelf>
where
    LatSelf: PartialOrd<LatOther>,
{
    fn partial_cmp(&self, other: &VecUnion<LatOther>) -> Option<Ordering> {
        let (self_len, other_len) = (self.vec.len(), other.vec.len());
        let mut self_any_greater = other_len < self_len;
        let mut other_any_greater = self_len < other_len;
        for (self_val, other_val) in self.vec.iter().zip(other.vec.iter()) {
            match self_val.partial_cmp(other_val) {
                None => {
                    return None;
                }
                Some(Less) => {
                    other_any_greater = true;
                }
                Some(Greater) => {
                    self_any_greater = true;
                }
                Some(Equal) => {}
            }
            if self_any_greater && other_any_greater {
                return None;
            }
        }
        match (self_any_greater, other_any_greater) {
            (true, false) => Some(Greater),
            (false, true) => Some(Less),
            (false, false) => Some(Equal),
            // We check this one after each loop iteration above.
            (true, true) => unreachable!(),
        }
    }
}
impl<LatSelf, LatOther> LatticeOrd<VecUnion<LatOther>> for VecUnion<LatSelf> where
    Self: PartialOrd<VecUnion<LatOther>>
{
}

impl<Lat> IsBot for VecUnion<Lat> {
    fn is_bot(&self) -> bool {
        self.vec.is_empty()
    }
}

impl<Lat> IsTop for VecUnion<Lat> {
    fn is_top(&self) -> bool {
        false
    }
}

#[cfg(test)]
mod test {
    use std::collections::HashSet;

    use super::*;
    use crate::Max;
    use crate::set_union::SetUnionHashSet;
    use crate::test::{cartesian_power, check_all};

    #[test]
    fn basic() {
        let mut my_vec_a = VecUnion::<Max<usize>>::default();
        let my_vec_b = VecUnion::new(vec![Max::new(9), Max::new(4), Max::new(5)]);
        let my_vec_c = VecUnion::new(vec![Max::new(2), Max::new(5)]);

        assert!(my_vec_a.merge(my_vec_b.clone()));
        assert!(!my_vec_a.merge(my_vec_b));
        assert!(my_vec_a.merge(my_vec_c.clone()));
        assert!(!my_vec_a.merge(my_vec_c));
    }

    #[test]
    fn consistency() {
        let mut test_vec = vec![VecUnion::new(vec![] as Vec<SetUnionHashSet<_>>)];

        let vals = [vec![], vec![0], vec![1], vec![0, 1]]
            .map(HashSet::from_iter)
            .map(SetUnionHashSet::new);

        test_vec.extend(
            cartesian_power::<_, 1>(&vals)
                .map(|row| VecUnion::new(row.into_iter().cloned().collect())),
        );
        test_vec.extend(
            cartesian_power::<_, 2>(&vals)
                .map(|row| VecUnion::new(row.into_iter().cloned().collect())),
        );

        check_all(&test_vec);
    }
}
