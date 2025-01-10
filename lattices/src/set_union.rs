//! Module containing the [`SetUnion`] lattice and aliases for different datastructures.

use std::cmp::Ordering::{self, *};
use std::collections::{BTreeSet, HashSet};
use std::marker::PhantomData;

use cc_traits::SimpleCollectionRef;
use lazy_static::lazy_static;
use prometheus::{register_int_counter, IntCounter};
use crate::cc_traits::{Iter, Len, Set};
use crate::collections::{ArraySet, OptionSet, SingletonSet};
use crate::{Atomize, DeepReveal, IsBot, IsTop, LatticeBimorphism, LatticeFrom, LatticeOrd, Merge};

/// Set-union lattice.
///
/// Merging set-union lattices is done by unioning the keys.
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

    /// Reveal the inner value as a shared reference.
    pub fn as_reveal_ref(&self) -> &Set {
        &self.0
    }

    /// Reveal the inner value as an exclusive reference.
    pub fn as_reveal_mut(&mut self) -> &mut Set {
        &mut self.0
    }

    /// Gets the inner by value, consuming self.
    pub fn into_reveal(self) -> Set {
        self.0
    }
}

impl<Set> DeepReveal for SetUnion<Set> {
    type Revealed = Set;

    fn deep_reveal(self) -> Self::Revealed {
        self.0
    }
}


lazy_static! {
    pub static ref SET_MERGE_COUNTER: IntCounter =
        register_int_counter!("set_merges", "Counts the number of SetUnion Merges").unwrap();
}

impl<SetSelf, SetOther, Item> Merge<SetUnion<SetOther>> for SetUnion<SetSelf>
where
    SetSelf: Extend<Item> + Len,
    SetOther: IntoIterator<Item = Item>,
{
    fn merge(&mut self, other: SetUnion<SetOther>) -> bool {
        SET_MERGE_COUNTER.inc();
        let old_len = self.0.len();
        self.0.extend(other.0);
        self.0.len() > old_len
    }
}

impl<SetSelf, SetOther, Item> LatticeFrom<SetUnion<SetOther>> for SetUnion<SetSelf>
where
    SetSelf: FromIterator<Item>,
    SetOther: IntoIterator<Item = Item>,
{
    fn lattice_from(other: SetUnion<SetOther>) -> Self {
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

impl<Set> IsBot for SetUnion<Set>
where
    Set: Len,
{
    fn is_bot(&self) -> bool {
        self.0.is_empty()
    }
}

impl<Set> IsTop for SetUnion<Set> {
    fn is_top(&self) -> bool {
        false
    }
}

impl<Set, Item> Atomize for SetUnion<Set>
where
    Set: Len + IntoIterator<Item = Item> + Extend<Item>,
    Set::IntoIter: 'static,
    Item: 'static,
{
    type Atom = SetUnionSingletonSet<Item>;

    // TODO: use impl trait, then remove 'static.
    type AtomIter = Box<dyn Iterator<Item = Self::Atom>>;

    fn atomize(self) -> Self::AtomIter {
        Box::new(self.0.into_iter().map(SetUnionSingletonSet::new_from))
    }
}

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
pub type SetUnionOptionSet<Item> = SetUnion<OptionSet<Item>>;

/// Bimorphism for the cartesian product of two sets. Output is a set of all possible pairs of
/// items from the two input sets.
pub struct CartesianProductBimorphism<SetOut> {
    _phantom: PhantomData<fn() -> SetOut>,
}
impl<SetOut> Default for CartesianProductBimorphism<SetOut> {
    fn default() -> Self {
        Self {
            _phantom: Default::default(),
        }
    }
}
impl<SetA, SetB, SetOut> LatticeBimorphism<SetUnion<SetA>, SetUnion<SetB>>
    for CartesianProductBimorphism<SetOut>
where
    SetA: IntoIterator,
    SetB: Iter + SimpleCollectionRef,
    SetA::Item: Clone,
    SetB::Item: Clone,
    SetOut: FromIterator<(SetA::Item, SetB::Item)>,
{
    type Output = SetUnion<SetOut>;

    fn call(&mut self, lat_a: SetUnion<SetA>, lat_b: SetUnion<SetB>) -> Self::Output {
        let set_a = lat_a.into_reveal();
        let set_b = lat_b.into_reveal();
        let set_out = set_a
            .into_iter()
            .flat_map(|a_item| {
                set_b
                    .iter()
                    .map(<SetB as SimpleCollectionRef>::into_ref)
                    .cloned()
                    .map(move |b_item| (a_item.clone(), b_item))
            })
            .collect::<SetOut>();
        SetUnion::new(set_out)
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::test::{check_all, check_atomize_each, check_lattice_bimorphism};

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

    #[test]
    fn atomize() {
        check_atomize_each(&[
            SetUnionHashSet::new_from([]),
            SetUnionHashSet::new_from([0]),
            SetUnionHashSet::new_from([1]),
            SetUnionHashSet::new_from([0, 1]),
            SetUnionHashSet::new((0..10).collect()),
        ]);
    }

    #[test]
    fn cartesian_product() {
        let items_a = &[
            SetUnionHashSet::new_from([]),
            SetUnionHashSet::new_from([0]),
            SetUnionHashSet::new_from([1]),
            SetUnionHashSet::new_from([0, 1]),
        ];
        let items_b = &[
            SetUnionBTreeSet::new("hello".chars().collect()),
            SetUnionBTreeSet::new("world".chars().collect()),
        ];

        check_lattice_bimorphism(
            CartesianProductBimorphism::<HashSet<_>>::default(),
            items_a,
            items_a,
        );
        check_lattice_bimorphism(
            CartesianProductBimorphism::<HashSet<_>>::default(),
            items_a,
            items_b,
        );
        check_lattice_bimorphism(
            CartesianProductBimorphism::<HashSet<_>>::default(),
            items_b,
            items_a,
        );
        check_lattice_bimorphism(
            CartesianProductBimorphism::<HashSet<_>>::default(),
            items_b,
            items_b,
        );
    }
}
