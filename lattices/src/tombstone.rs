//! Shared tombstone set implementations for efficient storage of deleted keys.
//!
//! This module provides specialized tombstone storage implementations that can be used
//! with both [`crate::set_union_with_tombstones::SetUnionWithTombstones`] and
//! [`crate::map_union_with_tombstones::MapUnionWithTombstones`].

use fst::{IntoStreamer, Set as FstSet, SetBuilder, Streamer};
use roaring::RoaringTreemap;

use crate::cc_traits::Len;

/// A tombstone set backed by [`RoaringTreemap`] for u64 integer keys.
/// This provides space-efficient bitmap compression for integer tombstones.
#[derive(Default, Clone, Debug)]
pub struct RoaringTombstoneSet {
    pub(crate) bitmap: RoaringTreemap,
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
    pub(crate) fst: FstSet<Vec<u8>>,
    pub(crate) _phantom: std::marker::PhantomData<Item>,
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
