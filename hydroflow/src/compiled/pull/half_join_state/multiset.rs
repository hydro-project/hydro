use std::collections::hash_map::Entry;
use std::collections::VecDeque;

use super::HalfJoinState;
use crate::lang::clear::Clear;

type HashMap<K, V> = rustc_hash::FxHashMap<K, V>;

use smallvec::{smallvec, SmallVec};
#[derive(Debug)]
pub struct HalfMultisetJoinState<Key, ValBuild, ValProbe> {
    // Here a smallvec with inline storage of 1 is chosen.
    // The rationale for this decision is that, I speculate, that joins possibly have a bimodal distribution with regards to how much key contention they have.
    // That is, I think that there are many joins that have 1 value per key on LHS/RHS, and there are also a large category of joins that have multiple values per key.
    // For the category of joins that have multiple values per key, it's not clear why they would only have 2, 3, 4, or N specific number of values per key. So there's no good number to set the smallvec storage to.
    // Instead we can just focus on the first group of joins that have 1 value per key and get benefit there without hurting the other group too much with excessive memory usage.
    /// Table to probe, vec val contains all matches.
    table: HashMap<Key, SmallVec<[ValBuild; 1]>>,
    /// Not-yet emitted matches.
    current_matches: VecDeque<(Key, ValProbe, ValBuild)>,
}
impl<Key, ValBuild, ValProbe> Default for HalfMultisetJoinState<Key, ValBuild, ValProbe> {
    fn default() -> Self {
        Self {
            table: HashMap::default(),
            current_matches: VecDeque::default(),
        }
    }
}
impl<Key, ValBuild, ValProbe> Clear for HalfMultisetJoinState<Key, ValBuild, ValProbe> {
    fn clear(&mut self) {
        self.table.clear();
        self.current_matches.clear();
    }
}
impl<Key, ValBuild, ValProbe> HalfJoinState<Key, ValBuild, ValProbe>
    for HalfMultisetJoinState<Key, ValBuild, ValProbe>
where
    Key: Clone + Eq + std::hash::Hash,
    ValBuild: Clone,
    ValProbe: Clone,
{
    fn build(&mut self, k: Key, v: &ValBuild) -> bool {
        let entry = self.table.entry(k);

        match entry {
            Entry::Occupied(mut e) => {
                let vec = e.get_mut();

                vec.push(v.clone());
            }
            Entry::Vacant(e) => {
                e.insert(smallvec![v.clone()]);
            }
        };

        true
    }

    fn probe(&mut self, k: &Key, v: &ValProbe) {
        if let Some(entry) = self.table.get(k) {
            // TODO: We currently don't free/shrink the self.current_matches vecdeque to save time.
            // This mean it will grow to eventually become the largest number of matches in a single probe call.
            // Maybe we should clear this memory at the beginning of every tick/periodically?
            self.current_matches.extend(
                entry
                    .iter()
                    .map(|valbuild| (k.clone(), v.clone(), valbuild.clone())),
            );
        }
    }

    fn pop_match(&mut self) -> Option<(Key, ValProbe, ValBuild)> {
        self.current_matches.pop_front()
    }
}
