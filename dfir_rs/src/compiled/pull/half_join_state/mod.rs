/// State semantics for each half of a join.
use smallvec::SmallVec;

mod multiset;
pub use multiset::HalfMultisetJoinState;

mod set;
pub use set::HalfSetJoinState;

/// State semantics for each half of a join.
pub trait HalfJoinState<Key, ValBuild, ValProbe> {
    /// Insert a key value pair into the join state, currently this is always inserting into a hash table
    /// If the key-value pair exists then it is implementation defined what happens, usually either two copies are stored or only one copy is stored.
    fn build(&mut self, k: Key, v: &ValBuild) -> bool;

    /// This function does the actual joining part of the join. It looks up a key in the local join state and creates matches
    /// The first match is return directly to the caller, and any additional matches are stored internally to be retrieved later with `pop_match`
    fn probe(&mut self, k: &Key, v: &ValProbe) -> Option<(Key, ValProbe, ValBuild)>;

    /// If there are any stored matches from previous calls to probe then this function will remove them one at a time and return it.
    fn pop_match(&mut self) -> Option<(Key, ValProbe, ValBuild)>;

    /// Len of the join state in terms of number of keys.
    fn len(&self) -> usize;
    /// If the state is empty (`len() == 0`).
    fn is_empty(&self) -> bool {
        0 == self.len()
    }

    /// An iter over all entries of the state.
    fn iter(&self) -> std::collections::hash_map::Iter<'_, Key, SmallVec<[ValBuild; 1]>>;

    /// An iter over all the matches for a given key.
    fn full_probe(&self, k: &Key) -> std::slice::Iter<'_, ValBuild>;
}
