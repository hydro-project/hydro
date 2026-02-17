//! State management for symmetric hash joins.
//!
//! This module provides the [`HalfJoinState`] trait and implementations for
//! managing the state of each half of a symmetric hash join operation.

use std::borrow::Cow;

use smallvec::SmallVec;

mod multiset;
pub use multiset::HalfMultisetJoinState;

mod set;
pub use set::HalfSetJoinState;

/// State semantics for each half of a join.
///
/// This trait defines the interface for storing and probing join state.
/// Different implementations provide different semantics (set vs multiset).
pub trait HalfJoinState<Key, ValBuild, ValProbe>
where
    ValBuild: Clone,
{
    /// Insert a key-value pair into the join state.
    ///
    /// Returns `true` if the pair was inserted (implementation-defined for duplicates).
    fn build(&mut self, k: Key, v: Cow<'_, ValBuild>) -> bool;

    /// Probe the join state for matches with the given key and value.
    ///
    /// Returns the first match directly. Additional matches are stored internally
    /// and can be retrieved with [`pop_match`](Self::pop_match).
    fn probe(&mut self, k: &Key, v: &ValProbe) -> Option<(Key, ValProbe, ValBuild)>;

    /// Pop a stored match from previous [`probe`](Self::probe) calls.
    fn pop_match(&mut self) -> Option<(Key, ValProbe, ValBuild)>;

    /// Returns the number of keys in the join state.
    fn len(&self) -> usize;

    /// Returns `true` if the state is empty.
    fn is_empty(&self) -> bool {
        0 == self.len()
    }

    /// Returns an iterator over all entries in the state.
    fn iter(&self) -> std::collections::hash_map::Iter<'_, Key, SmallVec<[ValBuild; 1]>>;

    /// Returns an iterator over all values for a given key.
    fn full_probe(&self, k: &Key) -> std::slice::Iter<'_, ValBuild>;

    /// Clear the state without de-allocating.
    fn clear(&mut self);
}
