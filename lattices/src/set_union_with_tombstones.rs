//! Module containing the [`SetUnionWithTombstones`] lattice and aliases for different datastructures.
//!
//! # Choosing a Tombstone Implementation
//!
//! This module provides several specialized tombstone storage implementations optimized for different key types:
//!
//! ## For Integer Keys (u64)
//! Use [`SetUnionWithTombstonesRoaring`] with [`RoaringTombstoneSet`]:
//! - Extremely space-efficient bitmap compression
//! - Fast O(1) lookups and efficient bitmap OR operations during merge
//! - Works with u64 keys (other integer types can be cast to u64)
//! - Example: `SetUnionWithTombstonesRoaring::new_from(HashSet::from([1u64, 2, 3]), RoaringTombstoneSet::new())`
//!
//! ## For String Keys
//! Use [`SetUnionWithTombstonesFstString`] with [`FstTombstoneSet<String>`]:
//! - Compressed finite state transducer storage
//! - Zero false positives (collision-free)
//! - Efficient union operations for merging
//! - Maintains sorted order
//! - Example: `SetUnionWithTombstonesFstString::new_from(HashSet::from(["a".to_string()]), FstTombstoneSet::new())`
//!
//! ## For Byte Array Keys
//! Use [`SetUnionWithTombstonesFstBytes`] with [`FstTombstoneSet<Vec<u8>>`]:
//! - Same benefits as FST for strings
//! - Works with arbitrary byte sequences
//! - Example: `SetUnionWithTombstonesFstBytes::new_from(HashSet::from([vec![1, 2, 3]]), FstTombstoneSet::new())`
//!
//! ## For Other Types
//! Use the generic [`SetUnionWithTombstones`] with [`HashSet`] for both sets:
//! - Works with any `Hash + Eq` type
//! - No compression, but simple and flexible
//! - Example: `SetUnionWithTombstonesHashSet::new_from([custom_type], [])`
//!
//! ## Performance Characteristics
//!
//! | Implementation | Space Efficiency | Merge Speed | Lookup Speed | False Positives |
//! |----------------|------------------|-------------|--------------|-----------------|
//! | RoaringBitmap  | Excellent        | Excellent   | Excellent    | None            |
//! | FST            | Very Good        | Good        | Very Good    | None            |
//! | HashSet        | Poor             | Good        | Excellent    | None            |

use std::cmp::Ordering::{self, *};
use std::collections::{BTreeSet, HashSet};

use cc_traits::{Collection, Get, Remove};
use fst::{IntoStreamer, Set as FstSet, SetBuilder, Streamer};
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

// Specialized merge implementation for RoaringTombstoneSet with HashSet<u64>
// This is highly efficient because:
// 1. We can OR the roaring bitmaps directly (very fast)
// 2. We can use the bitmap's efficient contains() for lookups
impl Merge<SetUnionWithTombstones<HashSet<u64>, RoaringTombstoneSet>>
    for SetUnionWithTombstones<HashSet<u64>, RoaringTombstoneSet>
{
    fn merge(&mut self, other: SetUnionWithTombstones<HashSet<u64>, RoaringTombstoneSet>) -> bool {
        let old_set_len = self.set.len();
        let old_tombstones_len = self.tombstones.len();

        // OR the roaring bitmaps together - O(n) where n is bitmap size, very fast!
        self.tombstones.bitmap = &self.tombstones.bitmap | &other.tombstones.bitmap;

        // Merge other.set into self.set, filtering out tombstoned items
        self.set.extend(
            other
                .set
                .into_iter()
                .filter(|item| !self.tombstones.contains(item)),
        );

        // Remove any items from self.set that are now tombstoned
        self.set.retain(|item| !self.tombstones.contains(item));

        // Check if anything changed
        old_set_len != self.set.len() || old_tombstones_len < self.tombstones.len()
    }
}

// Specialized merge implementation for FstTombstoneSet with HashSet<String>
// This is efficient because:
// 1. We can union the FSTs directly (compressed set operation)
// 2. FST provides fast membership testing with zero false positives
impl Merge<SetUnionWithTombstones<HashSet<String>, FstTombstoneSet<String>>>
    for SetUnionWithTombstones<HashSet<String>, FstTombstoneSet<String>>
{
    fn merge(
        &mut self,
        other: SetUnionWithTombstones<HashSet<String>, FstTombstoneSet<String>>,
    ) -> bool {
        let old_set_len = self.set.len();
        let old_tombstones_len = self.tombstones.len();

        // Union the FSTs together - efficient compressed set union
        self.tombstones = self.tombstones.union(&other.tombstones);

        // Merge other.set into self.set, filtering out tombstoned items
        self.set.extend(
            other
                .set
                .into_iter()
                .filter(|item| !self.tombstones.contains(item.as_bytes())),
        );

        // Remove any items from self.set that are now tombstoned
        self.set
            .retain(|item| !self.tombstones.contains(item.as_bytes()));

        // Check if anything changed
        old_set_len != self.set.len() || old_tombstones_len < self.tombstones.len()
    }
}

// Specialized merge implementation for FstTombstoneSet with HashSet<Vec<u8>>
// This is efficient because:
// 1. We can union the FSTs directly (compressed set operation)
// 2. FST provides fast membership testing with zero false positives
impl Merge<SetUnionWithTombstones<HashSet<Vec<u8>>, FstTombstoneSet<Vec<u8>>>>
    for SetUnionWithTombstones<HashSet<Vec<u8>>, FstTombstoneSet<Vec<u8>>>
{
    fn merge(
        &mut self,
        other: SetUnionWithTombstones<HashSet<Vec<u8>>, FstTombstoneSet<Vec<u8>>>,
    ) -> bool {
        let old_set_len = self.set.len();
        let old_tombstones_len = self.tombstones.len();

        // Union the FSTs together - efficient compressed set union
        self.tombstones = self.tombstones.union(&other.tombstones);

        // Merge other.set into self.set, filtering out tombstoned items
        self.set.extend(
            other
                .set
                .into_iter()
                .filter(|item| !self.tombstones.contains(item)),
        );

        // Remove any items from self.set that are now tombstoned
        self.set.retain(|item| !self.tombstones.contains(item));

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

/// A tombstone set backed by [`RoaringTreemap`] for u64 integer keys.
/// This provides space-efficient bitmap compression for integer tombstones.
#[derive(Default, Clone, Debug)]
pub struct RoaringTombstoneSet {
    bitmap: RoaringTreemap,
}

impl RoaringTombstoneSet {
    /// Create a new empty `RoaringTombstoneSet`.
    pub fn new() -> Self {
        Self {
            bitmap: RoaringTreemap::new(),
        }
    }

    /// Check if an item is in the tombstone set.
    pub fn contains(&self, item: &u64) -> bool {
        self.bitmap.contains(*item)
    }

    /// Insert an item into the tombstone set.
    pub fn insert(&mut self, item: u64) -> bool {
        self.bitmap.insert(item)
    }
}

impl Extend<u64> for RoaringTombstoneSet {
    fn extend<T: IntoIterator<Item = u64>>(&mut self, iter: T) {
        self.bitmap.extend(iter);
    }
}

impl Len for RoaringTombstoneSet {
    fn len(&self) -> usize {
        self.bitmap.len() as usize
    }
}

impl IntoIterator for RoaringTombstoneSet {
    type Item = u64;
    type IntoIter = roaring::treemap::IntoIter;

    fn into_iter(self) -> Self::IntoIter {
        self.bitmap.into_iter()
    }
}

impl FromIterator<u64> for RoaringTombstoneSet {
    fn from_iter<T: IntoIterator<Item = u64>>(iter: T) -> Self {
        Self {
            bitmap: RoaringTreemap::from_iter(iter),
        }
    }
}

/// A tombstone set backed by FST (Finite State Transducer) for byte string keys.
/// This provides space-efficient storage with zero false positives for any type
/// that can be serialized to bytes (strings, serialized structs, etc.).
/// FST maintains keys in sorted order and supports efficient set operations.
#[derive(Clone, Debug)]
pub struct FstTombstoneSet<Item> {
    fst: FstSet<Vec<u8>>,
    _phantom: std::marker::PhantomData<Item>,
}

impl<Item> Default for FstTombstoneSet<Item> {
    fn default() -> Self {
        Self::new()
    }
}

impl<Item> FstTombstoneSet<Item> {
    /// Create a new empty `FstTombstoneSet`.
    pub fn new() -> Self {
        Self {
            fst: FstSet::default(),
            _phantom: std::marker::PhantomData,
        }
    }

    /// Create from an existing FST set.
    fn from_fst(fst: FstSet<Vec<u8>>) -> Self {
        Self {
            fst,
            _phantom: std::marker::PhantomData,
        }
    }

    /// Check if an item is in the tombstone set.
    pub fn contains(&self, item: &[u8]) -> bool {
        self.fst.contains(item)
    }

    /// Get the number of items in the set.
    pub fn len(&self) -> usize {
        self.fst.len()
    }

    /// Check if the set is empty.
    pub fn is_empty(&self) -> bool {
        self.fst.is_empty()
    }

    /// Union this FST with another, returning a new FST.
    pub fn union(&self, other: &Self) -> Self {
        let union_stream = self.fst.op().add(&other.fst).union();
        let mut builder = SetBuilder::memory();
        let mut stream = union_stream.into_stream();
        while let Some(key) = stream.next() {
            builder.insert(key).unwrap();
        }
        Self::from_fst(FstSet::new(builder.into_inner().unwrap()).unwrap())
    }
}

impl Len for FstTombstoneSet<Vec<u8>> {
    fn len(&self) -> usize {
        self.fst.len()
    }
}

impl Len for FstTombstoneSet<String> {
    fn len(&self) -> usize {
        self.fst.len()
    }
}

// For Vec<u8> items
impl Extend<Vec<u8>> for FstTombstoneSet<Vec<u8>> {
    fn extend<T: IntoIterator<Item = Vec<u8>>>(&mut self, iter: T) {
        let mut keys: Vec<_> = self.fst.stream().into_strs().unwrap();
        keys.extend(iter.into_iter().map(|v| String::from_utf8_lossy(&v).into_owned()));
        keys.sort();
        keys.dedup();

        let mut builder = SetBuilder::memory();
        for key in keys {
            builder.insert(key).unwrap();
        }
        self.fst = FstSet::new(builder.into_inner().unwrap()).unwrap();
    }
}

// For String items
impl Extend<String> for FstTombstoneSet<String> {
    fn extend<T: IntoIterator<Item = String>>(&mut self, iter: T) {
        let mut keys: Vec<_> = self.fst.stream().into_strs().unwrap();
        keys.extend(iter);
        keys.sort();
        keys.dedup();

        let mut builder = SetBuilder::memory();
        for key in keys {
            builder.insert(key).unwrap();
        }
        self.fst = FstSet::new(builder.into_inner().unwrap()).unwrap();
    }
}

impl FromIterator<Vec<u8>> for FstTombstoneSet<Vec<u8>> {
    fn from_iter<T: IntoIterator<Item = Vec<u8>>>(iter: T) -> Self {
        let mut keys: Vec<_> = iter.into_iter().collect();
        keys.sort();
        keys.dedup();

        let mut builder = SetBuilder::memory();
        for key in keys {
            builder.insert(key).unwrap();
        }
        Self::from_fst(FstSet::new(builder.into_inner().unwrap()).unwrap())
    }
}

impl FromIterator<String> for FstTombstoneSet<String> {
    fn from_iter<T: IntoIterator<Item = String>>(iter: T) -> Self {
        let mut keys: Vec<_> = iter.into_iter().collect();
        keys.sort();
        keys.dedup();

        let mut builder = SetBuilder::memory();
        for key in keys {
            builder.insert(key).unwrap();
        }
        Self::from_fst(FstSet::new(builder.into_inner().unwrap()).unwrap())
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
/// Provides space-efficient tombstone storage for u64 integer keys.
pub type SetUnionWithTombstonesRoaring = SetUnionWithTombstones<HashSet<u64>, RoaringTombstoneSet>;

/// FST-backed tombstone set with [`std::collections::HashSet`] for the main set.
/// Provides space-efficient, collision-free tombstone storage for String keys.
pub type SetUnionWithTombstonesFstString = SetUnionWithTombstones<HashSet<String>, FstTombstoneSet<String>>;

/// FST-backed tombstone set with [`std::collections::HashSet`] for the main set.
/// Provides space-efficient, collision-free tombstone storage for Vec<u8> keys.
pub type SetUnionWithTombstonesFstBytes = SetUnionWithTombstones<HashSet<Vec<u8>>, FstTombstoneSet<Vec<u8>>>;

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
            RoaringTombstoneSet::new(),
        );
        let mut y = SetUnionWithTombstonesRoaring::new_from(
            HashSet::from([2, 3, 4]),
            RoaringTombstoneSet::new(),
        );

        // Add tombstone for 2
        y.as_reveal_mut().1.insert(2);

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
            RoaringTombstoneSet::new(),
        );
        x.as_reveal_mut().1.insert(10);
        x.as_reveal_mut().1.insert(20);

        let mut y = SetUnionWithTombstonesRoaring::new_from(
            HashSet::from([6, 7, 8]),
            RoaringTombstoneSet::new(),
        );
        y.as_reveal_mut().1.insert(30);
        y.as_reveal_mut().1.insert(2); // Tombstone for 2

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

    #[test]
    fn fst_string_basic() {
        let mut x = SetUnionWithTombstonesFstString::new_from(
            HashSet::from(["apple".to_string(), "banana".to_string(), "cherry".to_string()]),
            FstTombstoneSet::new(),
        );
        let mut y = SetUnionWithTombstonesFstString::new_from(
            HashSet::from(["banana".to_string(), "date".to_string()]),
            FstTombstoneSet::new(),
        );

        // Add tombstone for "banana"
        y.as_reveal_mut().1.extend(vec!["banana".to_string()]);

        x.merge(y);

        // Should have apple, cherry, date (banana is tombstoned)
        assert!(!x.as_reveal_ref().0.contains("banana"));
        assert!(x.as_reveal_ref().0.contains("apple"));
        assert!(x.as_reveal_ref().0.contains("cherry"));
        assert!(x.as_reveal_ref().0.contains("date"));
        assert!(x.as_reveal_ref().1.contains(b"banana"));
    }

    #[test]
    fn fst_bytes_basic() {
        let mut x = SetUnionWithTombstonesFstBytes::new_from(
            HashSet::from([vec![1, 2, 3], vec![4, 5, 6]]),
            FstTombstoneSet::new(),
        );
        let mut y = SetUnionWithTombstonesFstBytes::new_from(
            HashSet::from([vec![4, 5, 6], vec![7, 8, 9]]),
            FstTombstoneSet::new(),
        );

        // Add tombstone for [4, 5, 6]
        y.as_reveal_mut().1.extend(vec![vec![4, 5, 6]]);

        x.merge(y);

        // Should have [1,2,3] and [7,8,9] but not [4,5,6]
        assert!(!x.as_reveal_ref().0.contains(&vec![4, 5, 6]));
        assert!(x.as_reveal_ref().0.contains(&vec![1, 2, 3]));
        assert!(x.as_reveal_ref().0.contains(&vec![7, 8, 9]));
        assert!(x.as_reveal_ref().1.contains(&[4, 5, 6]));
    }

    #[test]
    fn fst_merge_efficiency() {
        // Test that FST union works correctly with multiple tombstones
        let mut x = SetUnionWithTombstonesFstString::new_from(
            HashSet::from([
                "a".to_string(),
                "b".to_string(),
                "c".to_string(),
                "d".to_string(),
            ]),
            FstTombstoneSet::from_iter(vec!["x".to_string(), "y".to_string()]),
        );

        let y = SetUnionWithTombstonesFstString::new_from(
            HashSet::from(["e".to_string(), "f".to_string()]),
            FstTombstoneSet::from_iter(vec!["z".to_string(), "b".to_string()]),
        );

        x.merge(y);

        // Should have all tombstones
        assert!(x.as_reveal_ref().1.contains(b"x"));
        assert!(x.as_reveal_ref().1.contains(b"y"));
        assert!(x.as_reveal_ref().1.contains(b"z"));
        assert!(x.as_reveal_ref().1.contains(b"b"));

        // Should not have "b" in the set
        assert!(!x.as_reveal_ref().0.contains("b"));

        // Should have all other items
        assert!(x.as_reveal_ref().0.contains("a"));
        assert!(x.as_reveal_ref().0.contains("c"));
        assert!(x.as_reveal_ref().0.contains("d"));
        assert!(x.as_reveal_ref().0.contains("e"));
        assert!(x.as_reveal_ref().0.contains("f"));
    }
}
