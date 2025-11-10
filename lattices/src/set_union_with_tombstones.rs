//! Module containing the [`SetUnionWithTombstones`] lattice and aliases for different datastructures.

use std::cmp::Ordering::{self, *};
use std::collections::{BTreeSet, HashSet};
use std::hash::{Hash, Hasher};

use cc_traits::{Collection, Get, Remove};
use roaring::RoaringTreemap;

use crate::cc_traits::{Iter, Len, Set};
use crate::collections::{ArraySet, EmptySet, OptionSet, SingletonSet};
use crate::{IsBot, IsTop, LatticeFrom, LatticeOrd, Merge};

/// Set-union lattice with tombstones.
///
/// When an item is deleted from the SetUnionWithTombstones, it is removed from `set` and added to `tombstones`.
/// This also is an invariant, if an item appears in `tombstones` it must not also be in `set`.
///
/// Merging set-union lattices is done by unioning the keys of both the (set and tombstone) sets,
/// and then performing `set` = `set` - `tombstones`, to preserve the above invariant.
///
/// This implementation with two separate sets means that the actual set implementation can be decided 
/// for both the regular set and the tombstone set. This enables efficient storage strategies like using
/// [`RoaringTreemap`] for tombstones (see [`SetUnionWithTombstonesRoaring`]), which provides space-efficient
/// bitmap compression for the tombstone set while keeping the main set flexible.
///
/// Another possible implementation could be MapUnion<Key, WithTop<()>>, which would require fewer hash lookups
/// but provides less flexibility for specialized storage optimizations.
#[derive(Default, Clone, Debug)]
pub struct SetUnionWithTombstones<Set, TombstoneSet> {
    set: Set,
    tombstones: TombstoneSet,
}

impl<Set, TombstoneSet> SetUnionWithTombstones<Set, TombstoneSet> {
    /// Create a new `SetUnionWithTombstones` from a `Set` and `TombstoneSet`.
    pub fn new(set: Set, tombstones: TombstoneSet) -> Self {
        Self { set, tombstones }
    }

    /// Create a new `SetUnionWithTombstones` from an `Into<Set>` and an `Into<TombstonesSet>`.
    pub fn new_from(set: impl Into<Set>, tombstones: impl Into<TombstoneSet>) -> Self {
        Self::new(set.into(), tombstones.into())
    }

    /// Reveal the inner value as a shared reference.
    pub fn as_reveal_ref(&self) -> (&Set, &TombstoneSet) {
        (&self.set, &self.tombstones)
    }

    /// Reveal the inner value as an exclusive reference.
    pub fn as_reveal_mut(&mut self) -> (&mut Set, &mut TombstoneSet) {
        (&mut self.set, &mut self.tombstones)
    }

    /// Gets the inner by value, consuming self.
    pub fn into_reveal(self) -> (Set, TombstoneSet) {
        (self.set, self.tombstones)
    }
}

impl<Item, SetSelf, TombstoneSetSelf, SetOther, TombstoneSetOther>
    Merge<SetUnionWithTombstones<SetOther, TombstoneSetOther>>
    for SetUnionWithTombstones<SetSelf, TombstoneSetSelf>
where
    SetSelf: Extend<Item> + Len + for<'a> Remove<&'a Item>,
    SetOther: IntoIterator<Item = Item>,
    TombstoneSetSelf: Extend<Item> + Len + for<'a> Get<&'a Item>,
    TombstoneSetOther: IntoIterator<Item = Item>,
{
    fn merge(&mut self, other: SetUnionWithTombstones<SetOther, TombstoneSetOther>) -> bool {
        let old_set_len = self.set.len();
        let old_tombstones_len = self.tombstones.len();

        // Merge other set into self, don't include anything deleted by the current tombstone set.
        self.set.extend(
            other
                .set
                .into_iter()
                .filter(|x| !self.tombstones.contains(x)),
        );

        // Combine the tombstone sets. Also need to remove any items in the remote tombstone set that currently exist in the local set.
        self.tombstones
            .extend(other.tombstones.into_iter().inspect(|x| {
                self.set.remove(x);
            }));

        // if either there are new items in the real set, or the tombstone set increased
        old_set_len < self.set.len() || old_tombstones_len < self.tombstones.len()
    }
}

// Specialized merge implementation for RoaringTombstoneSet with HashSet
// This is more efficient because we can OR the roaring bitmaps directly,
// and we iterate through the smaller set to check against tombstones.
impl<Item>
    Merge<SetUnionWithTombstones<HashSet<Item>, RoaringTombstoneSet<Item>>>
    for SetUnionWithTombstones<HashSet<Item>, RoaringTombstoneSet<Item>>
where
    Item: Hash + Eq + Clone,
{
    fn merge(&mut self, other: SetUnionWithTombstones<HashSet<Item>, RoaringTombstoneSet<Item>>) -> bool {
        let old_set_len = self.set.len();
        let old_tombstones_len = self.tombstones.len();

        // OR the roaring bitmaps together first - this is the key efficiency win!
        self.tombstones.bitmap = &self.tombstones.bitmap | &other.tombstones.bitmap;

        // Now merge the sets, removing any items that are in the combined tombstones
        // Determine which set is smaller to iterate through for tombstone checking
        if self.set.len() <= other.set.len() {
            // Self is smaller: iterate through self, check against combined tombstones, remove matches
            let to_remove: Vec<_> = self.set.iter()
                .filter(|item| self.tombstones.contains(item))
                .cloned()
                .collect();
            for item in to_remove {
                self.set.remove(&item);
            }
            
            // Add items from other that aren't in combined tombstones
            self.set.extend(
                other.set.into_iter()
                    .filter(|item| !self.tombstones.contains(item))
            );
        } else {
            // Other is smaller: add items from other that aren't in combined tombstones
            // Then check self for items in combined tombstones
            self.set.extend(
                other.set.into_iter()
                    .filter(|item| !self.tombstones.contains(item))
            );
            
            // Remove items from self that are in combined tombstones
            let to_remove: Vec<_> = self.set.iter()
                .filter(|item| self.tombstones.contains(item))
                .cloned()
                .collect();
            for item in to_remove {
                self.set.remove(&item);
            }
        }

        // Check if anything changed
        old_set_len != self.set.len() || old_tombstones_len < self.tombstones.len()
    }
}

impl<SetSelf, TombstoneSetSelf, SetOther, TombstoneSetOther, Item>
    LatticeFrom<SetUnionWithTombstones<SetOther, TombstoneSetOther>>
    for SetUnionWithTombstones<SetSelf, TombstoneSetSelf>
where
    SetSelf: FromIterator<Item>,
    SetOther: IntoIterator<Item = Item>,
    TombstoneSetSelf: FromIterator<Item>,
    TombstoneSetOther: IntoIterator<Item = Item>,
{
    fn lattice_from(other: SetUnionWithTombstones<SetOther, TombstoneSetOther>) -> Self {
        Self {
            set: other.set.into_iter().collect(),
            tombstones: other.tombstones.into_iter().collect(),
        }
    }
}

impl<SetSelf, TombstoneSetSelf, SetOther, TombstoneSetOther, Item>
    PartialOrd<SetUnionWithTombstones<SetOther, TombstoneSetOther>>
    for SetUnionWithTombstones<SetSelf, TombstoneSetSelf>
where
    SetSelf: Set<Item, Item = Item> + Iter,
    SetOther: Set<Item, Item = Item> + Iter,
    TombstoneSetSelf: Set<Item, Item = Item> + Iter,
    TombstoneSetOther: Set<Item, Item = Item> + Iter,
{
    fn partial_cmp(
        &self,
        other: &SetUnionWithTombstones<SetOther, TombstoneSetOther>,
    ) -> Option<Ordering> {
        fn set_cmp<I, A, B>(a: &A, b: &B) -> Option<Ordering>
        where
            A: Collection<Item = I> + Iter + for<'a> Get<&'a I> + Len,
            B: Collection<Item = I> + Iter + for<'a> Get<&'a I> + Len,
        {
            match a.len().cmp(&b.len()) {
                Less => {
                    if a.iter().all(|key| b.contains(&*key)) {
                        Some(Less)
                    } else {
                        None
                    }
                }
                Equal => {
                    if a.iter().all(|key| b.contains(&*key)) {
                        Some(Equal)
                    } else {
                        None
                    }
                }
                Greater => {
                    if b.iter().all(|key| a.contains(&*key)) {
                        Some(Greater)
                    } else {
                        None
                    }
                }
            }
        }

        fn set_cmp_filter<I, A, B, C, D>(a: &A, b: &B, f1: &C, f2: &D) -> Option<Ordering>
        where
            A: Collection<Item = I> + Iter + for<'a> Get<&'a I> + Len,
            B: Collection<Item = I> + Iter + for<'a> Get<&'a I> + Len,
            C: for<'a> Get<&'a I>,
            D: for<'a> Get<&'a I>,
        {
            let is_a_greater_than_b = a
                .iter()
                .filter(|key| !f2.contains(key))
                .any(|key| !b.contains(&*key));

            let is_b_greater_than_a = b
                .iter()
                .filter(|key| !f1.contains(key))
                .any(|key| !a.contains(&*key));

            match (is_a_greater_than_b, is_b_greater_than_a) {
                (true, true) => None,
                (true, false) => Some(Greater),
                (false, true) => Some(Less),
                (false, false) => Some(Equal),
            }
        }

        match set_cmp(&self.tombstones, &other.tombstones) {
            Some(Less) => {
                match set_cmp_filter(&self.set, &other.set, &self.tombstones, &other.tombstones) {
                    Some(Greater) => None,
                    Some(Less) => Some(Less),
                    Some(Equal) => Some(Less),
                    None => None,
                }
            }
            Some(Equal) => set_cmp(&self.set, &other.set),
            Some(Greater) => {
                match set_cmp_filter(&self.set, &other.set, &self.tombstones, &other.tombstones) {
                    Some(Greater) => Some(Greater),
                    Some(Equal) => Some(Greater),
                    Some(Less) => None,
                    None => None,
                }
            }
            None => None,
        }
    }
}
impl<SetSelf, TombstoneSetSelf, SetOther, TombstoneSetOther>
    LatticeOrd<SetUnionWithTombstones<SetOther, TombstoneSetOther>>
    for SetUnionWithTombstones<SetSelf, TombstoneSetSelf>
where
    Self: PartialOrd<SetUnionWithTombstones<SetOther, TombstoneSetOther>>,
{
}

impl<SetSelf, TombstoneSetSelf, SetOther, TombstoneSetOther, Item>
    PartialEq<SetUnionWithTombstones<SetOther, TombstoneSetOther>>
    for SetUnionWithTombstones<SetSelf, TombstoneSetSelf>
where
    SetSelf: Set<Item, Item = Item> + Iter,
    SetOther: Set<Item, Item = Item> + Iter,
    TombstoneSetSelf: Set<Item, Item = Item> + Iter,
    TombstoneSetOther: Set<Item, Item = Item> + Iter,
{
    fn eq(&self, other: &SetUnionWithTombstones<SetOther, TombstoneSetOther>) -> bool {
        if self.set.len() != other.set.len() || self.tombstones.len() != other.tombstones.len() {
            return false;
        }

        self.set.iter().all(|key| other.set.contains(&*key))
            && self
                .tombstones
                .iter()
                .all(|key| other.tombstones.contains(&*key))
    }
}
impl<SetSelf, TombstoneSetSelf> Eq for SetUnionWithTombstones<SetSelf, TombstoneSetSelf> where
    Self: PartialEq
{
}

impl<Set, TombstoneSet> IsBot for SetUnionWithTombstones<Set, TombstoneSet>
where
    Set: Len,
    TombstoneSet: Len,
{
    fn is_bot(&self) -> bool {
        self.set.is_empty() && self.tombstones.is_empty()
    }
}

impl<Set, TombstoneSet> IsTop for SetUnionWithTombstones<Set, TombstoneSet> {
    fn is_top(&self) -> bool {
        false
    }
}

/// A tombstone set backed by [`RoaringTreemap`] that stores hashes of items.
/// This provides space-efficient storage for tombstones of any hashable type.
/// 
/// Note: This uses hashing, so there's a theoretical possibility of hash collisions,
/// though this is extremely unlikely with 64-bit hashes.
#[derive(Default, Clone, Debug)]
pub struct RoaringTombstoneSet<Item> {
    bitmap: RoaringTreemap,
    _phantom: std::marker::PhantomData<Item>,
}

impl<Item> RoaringTombstoneSet<Item> {
    /// Create a new empty `RoaringTombstoneSet`.
    pub fn new() -> Self {
        Self {
            bitmap: RoaringTreemap::new(),
            _phantom: std::marker::PhantomData,
        }
    }

    /// Hash an item to u64 for storage in the bitmap.
    fn hash_item(item: &Item) -> u64
    where
        Item: Hash,
    {
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        item.hash(&mut hasher);
        hasher.finish()
    }
    
    /// Check if an item is in the tombstone set.
    pub fn contains(&self, item: &Item) -> bool
    where
        Item: Hash,
    {
        self.bitmap.contains(Self::hash_item(item))
    }
    
    /// Insert an item into the tombstone set.
    pub fn insert(&mut self, item: &Item) -> bool
    where
        Item: Hash,
    {
        self.bitmap.insert(Self::hash_item(item))
    }
}

impl<Item: Hash> Extend<Item> for RoaringTombstoneSet<Item> {
    fn extend<T: IntoIterator<Item = Item>>(&mut self, iter: T) {
        for item in iter {
            self.bitmap.insert(Self::hash_item(&item));
        }
    }
}

impl<Item> Len for RoaringTombstoneSet<Item> {
    fn len(&self) -> usize {
        self.bitmap.len() as usize
    }
}

/// [`std::collections::HashSet`]-backed [`SetUnionWithTombstones`] lattice.
pub type SetUnionWithTombstonesHashSet<Item> = SetUnionWithTombstones<HashSet<Item>, HashSet<Item>>;

/// [`std::collections::BTreeSet`]-backed [`SetUnionWithTombstones`] lattice.
pub type SetUnionWithTombstonesBTreeSet<Item> =
    SetUnionWithTombstones<BTreeSet<Item>, BTreeSet<Item>>;

/// [`Vec`]-backed [`SetUnionWithTombstones`] lattice.
pub type SetUnionWithTombstonesVec<Item> = SetUnionWithTombstones<Vec<Item>, Vec<Item>>;

/// [`crate::collections::ArraySet`]-backed [`SetUnionWithTombstones`] lattice.
pub type SetUnionWithTombstonesArray<Item, const N: usize> =
    SetUnionWithTombstones<ArraySet<Item, N>, ArraySet<Item, N>>;

/// [`crate::collections::SingletonSet`]-backed [`SetUnionWithTombstones`] lattice.
pub type SetUnionWithTombstonesSingletonSet<Item> =
    SetUnionWithTombstones<SingletonSet<Item>, SingletonSet<Item>>;

/// [`Option`]-backed [`SetUnionWithTombstones`] lattice.
pub type SetUnionWithTombstonesOptionSet<Item> =
    SetUnionWithTombstones<OptionSet<Item>, OptionSet<Item>>;

/// [`crate::collections::SingletonSet`]-backed [`SetUnionWithTombstones`] lattice.
pub type SetUnionWithTombstonesTombstoneOnlySet<Item> =
    SetUnionWithTombstones<EmptySet<Item>, SingletonSet<Item>>;

/// [`RoaringTreemap`]-backed tombstone set with [`std::collections::HashSet`] for the main set.
/// Provides space-efficient tombstone storage for any hashable type.
pub type SetUnionWithTombstonesRoaring<Item> =
    SetUnionWithTombstones<HashSet<Item>, RoaringTombstoneSet<Item>>;

#[cfg(test)]
mod test {
    use super::*;
    use crate::test::check_all;

    #[test]
    fn delete_one() {
        let mut x = SetUnionWithTombstonesHashSet::new_from([1], []);
        let y = SetUnionWithTombstonesTombstoneOnlySet::new_from(EmptySet::default(), 1);

        assert_eq!(x.partial_cmp(&y), Some(Less));

        x.merge(y);

        assert!(x.as_reveal_mut().1.contains(&1));
    }

    #[test]
    fn test_specific_cases() {
        assert_eq!(
            SetUnionWithTombstonesHashSet::new_from([], [0])
                .partial_cmp(&SetUnionWithTombstonesHashSet::new_from([0], [])),
            Some(Greater),
        );

        assert_eq!(
            SetUnionWithTombstonesHashSet::new_from([0], [1])
                .partial_cmp(&SetUnionWithTombstonesHashSet::new_from([], [])),
            Some(Greater),
        );

        assert_eq!(
            SetUnionWithTombstonesHashSet::new_from([], [0])
                .partial_cmp(&SetUnionWithTombstonesHashSet::new_from([], [])),
            Some(Greater),
        );

        assert_eq!(
            SetUnionWithTombstonesHashSet::new_from([], [0])
                .partial_cmp(&SetUnionWithTombstonesHashSet::new_from([], [])),
            Some(Greater),
        );
    }

    #[test]
    fn consistency() {
        check_all(&[
            SetUnionWithTombstonesHashSet::new_from([], []),
            SetUnionWithTombstonesHashSet::new_from([0], []),
            SetUnionWithTombstonesHashSet::new_from([], [0]),
            SetUnionWithTombstonesHashSet::new_from([1], []),
            SetUnionWithTombstonesHashSet::new_from([], [1]),
            SetUnionWithTombstonesHashSet::new_from([0, 1], []),
            SetUnionWithTombstonesHashSet::new_from([], [0, 1]),
            SetUnionWithTombstonesHashSet::new_from([0], [1]),
            SetUnionWithTombstonesHashSet::new_from([1], [0]),
        ]);
    }

    #[test]
    fn roaring_basic() {
        let mut x = SetUnionWithTombstonesRoaring::new_from(
            HashSet::from([1, 2, 3]), 
            RoaringTombstoneSet::new()
        );
        let mut y = SetUnionWithTombstonesRoaring::new_from(
            HashSet::from([2, 3, 4]), 
            RoaringTombstoneSet::new()
        );
        
        // Add tombstone for 2
        y.as_reveal_mut().1.insert(&2);
        
        x.merge(y);
        
        // Should have 1, 3, 4 (2 is tombstoned)
        assert!(!x.as_reveal_ref().0.contains(&2));
        assert!(x.as_reveal_ref().0.contains(&1));
        assert!(x.as_reveal_ref().0.contains(&3));
        assert!(x.as_reveal_ref().0.contains(&4));
        assert!(x.as_reveal_ref().1.contains(&2));
    }

    #[test]
    fn roaring_merge_efficiency() {
        // Test that merging roaring bitmaps works correctly
        let mut x = SetUnionWithTombstonesRoaring::new_from(
            HashSet::from([1, 2, 3, 4, 5]), 
            RoaringTombstoneSet::new()
        );
        x.as_reveal_mut().1.insert(&10);
        x.as_reveal_mut().1.insert(&20);
        
        let mut y = SetUnionWithTombstonesRoaring::new_from(
            HashSet::from([6, 7, 8]), 
            RoaringTombstoneSet::new()
        );
        y.as_reveal_mut().1.insert(&30);
        y.as_reveal_mut().1.insert(&2); // Tombstone for 2
        
        x.merge(y);
        
        // Should have all tombstones
        assert!(x.as_reveal_ref().1.contains(&10));
        assert!(x.as_reveal_ref().1.contains(&20));
        assert!(x.as_reveal_ref().1.contains(&30));
        assert!(x.as_reveal_ref().1.contains(&2));
        
        // Should not have 2 in the set
        assert!(!x.as_reveal_ref().0.contains(&2));
        
        // Should have all other items
        assert!(x.as_reveal_ref().0.contains(&1));
        assert!(x.as_reveal_ref().0.contains(&3));
        assert!(x.as_reveal_ref().0.contains(&6));
        assert!(x.as_reveal_ref().0.contains(&7));
    }
}
