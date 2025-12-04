use std::borrow::Cow;
use std::collections::VecDeque;
use std::collections::hash_map::Entry;

use super::HalfJoinState;
use crate::util::clear::Clear;

type HashMap<K, V> = rustc_hash::FxHashMap<K, V>;

use smallvec::{SmallVec, smallvec};

/// [`HalfJoinState`] with set semantics.
#[derive(Debug)]
pub struct HalfSetJoinState<Key, ValBuild, ValProbe> {
    // Here a smallvec with inline storage of 1 is chosen.
    // The rationale for this decision is that, I speculate, that joins possibly have a bimodal distribution with regards to how much key contention they have.
    // That is, I think that there are many joins that have 1 value per key on LHS/RHS, and there are also a large category of joins that have multiple values per key.
    // For the category of joins that have multiple values per key, it's not clear why they would only have 2, 3, 4, or N specific number of values per key. So there's no good number to set the smallvec storage to.
    // Instead we can just focus on the first group of joins that have 1 value per key and get benefit there without hurting the other group too much with excessive memory usage.
    /// Table to probe, vec val contains all matches.
    table: HashMap<Key, SmallVec<[ValBuild; 1]>>,
    /// Not-yet emitted matches.
    current_matches: VecDeque<(Key, ValProbe, ValBuild)>,
    len: usize,
}
impl<Key, ValBuild, ValProbe> Default for HalfSetJoinState<Key, ValBuild, ValProbe> {
    fn default() -> Self {
        Self {
            table: HashMap::default(),
            current_matches: VecDeque::default(),
            len: 0,
        }
    }
}
impl<Key, ValBuild, ValProbe> Clear for HalfSetJoinState<Key, ValBuild, ValProbe> {
    fn clear(&mut self) {
        self.table.clear();
        self.current_matches.clear();
        self.len = 0;
    }
}
impl<Key, ValBuild, ValProbe> HalfJoinState<Key, ValBuild, ValProbe>
    for HalfSetJoinState<Key, ValBuild, ValProbe>
where
    Key: Clone + Eq + std::hash::Hash,
    ValBuild: Clone + Eq,
    ValProbe: Clone,
{
    fn build(&mut self, k: Key, v: Cow<'_, ValBuild>) -> bool {
        let entry = self.table.entry(k);

        match entry {
            Entry::Occupied(mut e) => {
                let vec = e.get_mut();

                if !vec.contains(v.as_ref()) {
                    vec.push(v.into_owned());
                    self.len += 1;
                    return true;
                }
            }
            Entry::Vacant(e) => {
                e.insert(smallvec![v.into_owned()]);
                self.len += 1;
                return true;
            }
        };

        false
    }

    fn probe(&mut self, k: &Key, v: &ValProbe) -> Option<(Key, ValProbe, ValBuild)> {
        // TODO: We currently don't free/shrink the self.current_matches vecdeque to save time.
        // This mean it will grow to eventually become the largest number of matches in a single probe call.
        // Maybe we should clear this memory at the beginning of every tick/periodically?
        let mut iter = self
            .table
            .get(k)?
            .iter()
            .map(|valbuild| (k.clone(), v.clone(), valbuild.clone()));

        let first = iter.next();

        self.current_matches.extend(iter);

        first
    }

    fn full_probe(&self, k: &Key) -> std::slice::Iter<'_, ValBuild> {
        let Some(sv) = self.table.get(k) else {
            return [].iter();
        };

        sv.iter()
    }

    fn pop_match(&mut self) -> Option<(Key, ValProbe, ValBuild)> {
        self.current_matches.pop_front()
    }

    fn len(&self) -> usize {
        self.len
    }

    fn iter(&self) -> std::collections::hash_map::Iter<'_, Key, SmallVec<[ValBuild; 1]>> {
        self.table.iter()
    }
}
