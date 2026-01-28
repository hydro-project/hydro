//! Shared tombstone set implementations for efficient storage of deleted keys.
//!
//! This module provides specialized tombstone storage implementations that can be used
//! with both [`crate::set_union_with_tombstones::SetUnionWithTombstones`] and
//! [`crate::map_union_with_tombstones::MapUnionWithTombstones`].
//!
//! # Choosing a Tombstone Implementation
//!
//! This module provides several specialized tombstone storage implementations optimized for different key types:
//!
//! ## For Integer Keys (u64)
//! Use [`RoaringTombstoneSet`]:
//! - Extremely space-efficient bitmap compression
//! - Fast O(1) lookups and efficient bitmap OR operations during merge
//! - Works with u64 keys (other integer types can be cast to u64)
//!
//! ## For String Keys
//! Use [`FstTombstoneSet<String>`]:
//! - Compressed finite state transducer storage
//! - Zero false positives (collision-free)
//! - Efficient union operations for merging
//! - Maintains sorted order
//! - **Note:** For arbitrary byte sequences, encode them as hex or base64 strings first
//!
//! ## For Other Types
//! Use [`std::collections::HashSet`] for tombstones:
//! - Works with any `Hash + Eq` key type
//! - No compression, but simple and flexible
//!
//! Alternatively, you can hash to 64-bit integers and follow instructions for that case above,
//! understanding there's a tiny risk of hash collision which could result in keys being
//! tombstoned (deleted) incorrectly.
//!
//! ## Performance Characteristics
//!
//! | Implementation | Space Efficiency | Merge Speed | Lookup Speed | False Positives |
//! |----------------|------------------|-------------|--------------|-----------------|
//! | RoaringBitmap  | Excellent        | Excellent   | Excellent    | None            |
//! | FST            | Very Good        | Good        | Very Good    | None            |
//! | HashSet        | Poor             | Good        | Excellent    | None            |
//!
//! # Performance Considerations
//!
//! - **RoaringBitmap:** Optimized for dense integer sets. Very fast for all operations.
//! - **FST:** The `extend()` operation rebuilds the entire FST, so batch your insertions.
//!   Use `from_iter()` when possible for better performance.
//!
//! # Thread Safety
//!
//! Both implementations are `Send` and `Sync` when their contained types are.
//! They do not use interior mutability.

use std::collections::HashSet;

use fst::{IntoStreamer, Set as FstSet, SetBuilder, Streamer};
use roaring::RoaringTreemap;

use crate::cc_traits::Len;

/// Trait for tombstone set implementations that support efficient union operations.
///
/// This trait abstracts over different tombstone storage strategies, allowing
/// specialized implementations like [`RoaringTombstoneSet`] and [`FstTombstoneSet`]
/// to provide optimized merge operations for their respective key types.
///
/// Implementors must provide:
/// - A way to check membership (`contains`)
/// - A way to union with another tombstone set (`union_with`)
/// - Standard collection traits (`Len`, `Extend`, etc.)
pub trait TombstoneSet<Key>: Len + Extend<Key> {
    /// Check if a key is in the tombstone set.
    fn contains(&self, key: &Key) -> bool;

    /// Union this tombstone set with another, modifying self in place.
    /// Returns the old length before the union.
    fn union_with(&mut self, other: &Self) -> usize;
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

impl TombstoneSet<u64> for RoaringTombstoneSet {
    fn contains(&self, key: &u64) -> bool {
        self.bitmap.contains(*key)
    }

    fn union_with(&mut self, other: &Self) -> usize {
        let old_len = self.len();
        self.bitmap = &self.bitmap | &other.bitmap;
        old_len
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
///
/// ## Performance Notes
/// - The `extend()` operation rebuilds the entire FST, so batch your insertions when possible
/// - Union operations are efficient and create a new compressed FST
/// - Lookups are very fast (logarithmic in the number of keys)
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
    pub(crate) fn from_fst(fst: FstSet<Vec<u8>>) -> Self {
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
    fn union_internal(&self, other: &Self) -> Self {
        let union_stream = self.fst.op().add(&other.fst).union();
        let mut builder = SetBuilder::memory();
        let mut stream = union_stream.into_stream();
        while let Some(key) = stream.next() {
            // Union stream produces sorted keys, so insert should not fail
            builder
                .insert(key)
                .expect("union stream keys are sorted, insert should not fail");
        }
        Self::from_fst(
            FstSet::new(
                builder
                    .into_inner()
                    .expect("memory builder should not fail"),
            )
            .expect("FST construction from valid builder should not fail"),
        )
    }
}

/// Helper trait to convert keys to byte slices for FST operations.
pub trait AsBytes {
    /// Convert the key to a byte slice.
    fn as_bytes(&self) -> &[u8];
}

impl AsBytes for String {
    fn as_bytes(&self) -> &[u8] {
        self.as_bytes()
    }
}

impl TombstoneSet<String> for FstTombstoneSet<String> {
    fn contains(&self, key: &String) -> bool {
        self.fst.contains(key.as_bytes())
    }

    fn union_with(&mut self, other: &Self) -> usize {
        let old_len = self.len();
        *self = self.union_internal(other);
        old_len
    }
}

impl Len for FstTombstoneSet<String> {
    fn len(&self) -> usize {
        self.fst.len()
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
            // FST builder insert only fails if keys are not sorted, which we ensure above
            builder
                .insert(key)
                .expect("keys are sorted, insert should not fail");
        }
        // Memory builder and FST construction should not fail for valid sorted keys
        self.fst = FstSet::new(
            builder
                .into_inner()
                .expect("memory builder should not fail"),
        )
        .expect("FST construction from valid builder should not fail");
    }
}

impl FromIterator<String> for FstTombstoneSet<String> {
    fn from_iter<T: IntoIterator<Item = String>>(iter: T) -> Self {
        let mut keys: Vec<_> = iter.into_iter().collect();
        keys.sort();
        keys.dedup();

        let mut builder = SetBuilder::memory();
        for key in keys {
            // FST builder insert only fails if keys are not sorted, which we ensure above
            builder
                .insert(key)
                .expect("keys are sorted, insert should not fail");
        }
        Self::from_fst(
            FstSet::new(
                builder
                    .into_inner()
                    .expect("memory builder should not fail"),
            )
            .expect("FST construction from valid builder should not fail"),
        )
    }
}

impl IntoIterator for FstTombstoneSet<String> {
    type Item = String;
    type IntoIter = std::vec::IntoIter<String>;

    fn into_iter(self) -> Self::IntoIter {
        // Convert FST keys to strings
        self.fst
            .stream()
            .into_strs()
            .expect("FST contains valid UTF-8 strings")
            .into_iter()
    }
}

// Implement TombstoneSet for HashSet to support generic key types
impl<K> TombstoneSet<K> for HashSet<K>
where
    K: Eq + std::hash::Hash + Clone,
{
    fn contains(&self, key: &K) -> bool {
        HashSet::contains(self, key)
    }

    fn union_with(&mut self, other: &Self) -> usize {
        let old_len = self.len();
        #[expect(
            clippy::disallowed_methods,
            reason = "nondeterministic iteration order, fine to insert into set"
        )]
        for item in other.iter() {
            self.insert(item.clone());
        }
        old_len
    }
}
