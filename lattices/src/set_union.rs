//! Set-union compound lattice.
//!
//! Merging set-union lattices is done by unioning the keys.

use std::cmp::Ordering::{self, *};
use std::collections::{BTreeSet, HashSet};

use crate::cc_traits::{Iter, Len, Set};
use crate::collections::{ArraySet, SingletonSet};
use crate::{ConvertFrom, LatticeOrd, Merge};

/// A set-union lattice.
///
/// `Tag` specifies what datastructure to use, allowing us to deal with different datastructures
/// generically.
#[repr(transparent)]
#[derive(Copy, Clone, Debug, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct SetUnion<Set>(pub Set);
impl<Set> SetUnion<Set> {
    /// Create a new `SetUnion` from a `Set`.
    pub fn new(val: Set) -> Self {
        Self(val)
    }

    /// Create a new `SetUnion` from an `Into<Set>`.
    pub fn new_from(val: impl Into<Set>) -> Self {
        Self::new(val.into())
    }
}

impl<SetSelf, SetOther, Item> Merge<SetUnion<SetOther>> for SetUnion<SetSelf>
where
    SetSelf: Extend<Item> + Len,
    SetOther: IntoIterator<Item = Item>,
{
    fn merge(&mut self, other: SetUnion<SetOther>) -> bool {
        let old_len = self.0.len();
        self.0.extend(other.0);
        self.0.len() > old_len
    }
}

impl<SetSelf, SetOther, Item> ConvertFrom<SetUnion<SetOther>> for SetUnion<SetSelf>
where
    SetSelf: FromIterator<Item>,
    SetOther: IntoIterator<Item = Item>,
{
    fn from(other: SetUnion<SetOther>) -> Self {
        Self(other.0.into_iter().collect())
    }
}

impl<SetSelf, SetOther, Item> PartialOrd<SetUnion<SetOther>> for SetUnion<SetSelf>
where
    SetSelf: Set<Item, Item = Item> + Iter,
    SetOther: Set<Item, Item = Item> + Iter,
{
    fn partial_cmp(&self, other: &SetUnion<SetOther>) -> Option<Ordering> {
        match self.0.len().cmp(&other.0.len()) {
            Greater => {
                if other.0.iter().all(|key| self.0.contains(&*key)) {
                    Some(Greater)
                } else {
                    None
                }
            }
            Equal => {
                if self.0.iter().all(|key| other.0.contains(&*key)) {
                    Some(Equal)
                } else {
                    None
                }
            }
            Less => {
                if self.0.iter().all(|key| other.0.contains(&*key)) {
                    Some(Less)
                } else {
                    None
                }
            }
        }
    }
}
impl<SetSelf, SetOther> LatticeOrd<SetUnion<SetOther>> for SetUnion<SetSelf> where
    Self: PartialOrd<SetUnion<SetOther>>
{
}

impl<SetSelf, SetOther, Item> PartialEq<SetUnion<SetOther>> for SetUnion<SetSelf>
where
    SetSelf: Set<Item, Item = Item> + Iter,
    SetOther: Set<Item, Item = Item> + Iter,
{
    fn eq(&self, other: &SetUnion<SetOther>) -> bool {
        if self.0.len() != other.0.len() {
            return false;
        }

        self.0.iter().all(|key| other.0.contains(&*key))
    }
}
impl<SetSelf> Eq for SetUnion<SetSelf> where Self: PartialEq {}

/// [`std::collections::HashSet`]-backed [`SetUnion`] lattice.
pub type SetUnionHashSet<Item> = SetUnion<HashSet<Item>>;

/// [`std::collections::BTreeSet`]-backed [`SetUnion`] lattice.
pub type SetUnionBTreeSet<Item> = SetUnion<BTreeSet<Item>>;

/// [`Vec`]-backed [`SetUnion`] lattice.
pub type SetUnionVec<Item> = SetUnion<Vec<Item>>;

/// [`crate::collections::ArraySet`]-backed [`SetUnion`] lattice.
pub type SetUnionArray<Item, const N: usize> = SetUnion<ArraySet<Item, N>>;

/// [`crate::collections::SingletonSet`]-backed [`SetUnion`] lattice.
pub type SetUnionSingletonSet<Item> = SetUnion<SingletonSet<Item>>;

/// [`Option`]-backed [`SetUnion`] lattice.
pub type SetUnionOption<Item> = SetUnion<Option<Item>>;

#[cfg(test)]
mod test {
    use super::*;
    use crate::collections::SingletonSet;
    use crate::test::check_all;

    #[test]
    fn test_set_union() {
        let mut my_set_a = SetUnionHashSet::<&str>::new(HashSet::new());
        let my_set_b = SetUnionBTreeSet::<&str>::new(BTreeSet::new());
        let my_set_c = SetUnionSingletonSet::new(SingletonSet("hello world"));

        assert_eq!(Some(Equal), my_set_a.partial_cmp(&my_set_a));
        assert_eq!(Some(Equal), my_set_a.partial_cmp(&my_set_b));
        assert_eq!(Some(Less), my_set_a.partial_cmp(&my_set_c));
        assert_eq!(Some(Equal), my_set_b.partial_cmp(&my_set_a));
        assert_eq!(Some(Equal), my_set_b.partial_cmp(&my_set_b));
        assert_eq!(Some(Less), my_set_b.partial_cmp(&my_set_c));
        assert_eq!(Some(Greater), my_set_c.partial_cmp(&my_set_a));
        assert_eq!(Some(Greater), my_set_c.partial_cmp(&my_set_b));
        assert_eq!(Some(Equal), my_set_c.partial_cmp(&my_set_c));

        assert!(!my_set_a.merge(my_set_b));
        assert!(my_set_a.merge(my_set_c));
    }

    #[test]
    fn test_singleton_example() {
        let mut my_hash_set = SetUnionHashSet::<&str>::default();
        let my_delta_set = SetUnionSingletonSet::new_from("hello world");
        let my_array_set = SetUnionArray::new_from(["hello world", "b", "c", "d"]);

        assert_eq!(Some(Equal), my_delta_set.partial_cmp(&my_delta_set));
        assert_eq!(Some(Less), my_delta_set.partial_cmp(&my_array_set));
        assert_eq!(Some(Greater), my_array_set.partial_cmp(&my_delta_set));
        assert_eq!(Some(Equal), my_array_set.partial_cmp(&my_array_set));

        assert!(my_hash_set.merge(my_array_set)); // Changes
        assert!(!my_hash_set.merge(my_delta_set)); // No changes
    }

    #[test]
    fn consistency() {
        check_all(&[
            SetUnionHashSet::new_from([]),
            SetUnionHashSet::new_from([0]),
            SetUnionHashSet::new_from([1]),
            SetUnionHashSet::new_from([0, 1]),
        ]);
    }
}
