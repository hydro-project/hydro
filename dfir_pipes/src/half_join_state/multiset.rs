use std::borrow::Cow;
use std::collections::VecDeque;
use std::collections::hash_map::Entry;

use smallvec::{SmallVec, smallvec};

use super::HalfJoinState;

type HashMap<K, V> = rustc_hash::FxHashMap<K, V>;

/// [`HalfJoinState`] with multiset semantics.
///
/// Allows duplicate key-value pairs to be stored.
#[derive(Debug)]
pub struct HalfMultisetJoinState<Key, ValBuild, ValProbe> {
    /// Table to probe, vec val contains all matches.
    table: HashMap<Key, SmallVec<[ValBuild; 1]>>,
    /// Not-yet emitted matches.
    current_matches: VecDeque<(Key, ValProbe, ValBuild)>,
    len: usize,
}

impl<Key, ValBuild, ValProbe> Default for HalfMultisetJoinState<Key, ValBuild, ValProbe> {
    fn default() -> Self {
        Self {
            table: HashMap::default(),
            current_matches: VecDeque::default(),
            len: 0,
        }
    }
}

impl<Key, ValBuild, ValProbe> HalfJoinState<Key, ValBuild, ValProbe>
    for HalfMultisetJoinState<Key, ValBuild, ValProbe>
where
    Key: Clone + Eq + std::hash::Hash,
    ValBuild: Clone,
    ValProbe: Clone,
{
    fn build(&mut self, k: Key, v: Cow<'_, ValBuild>) -> bool {
        let entry = self.table.entry(k);

        match entry {
            Entry::Occupied(mut e) => {
                e.get_mut().push(v.into_owned());
                self.len += 1;
            }
            Entry::Vacant(e) => {
                e.insert(smallvec![v.into_owned()]);
                self.len += 1;
            }
        };

        true
    }

    fn probe(&mut self, k: &Key, v: &ValProbe) -> Option<(Key, ValProbe, ValBuild)> {
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
        self.table.get(k).map_or([].iter(), |sv| sv.iter())
    }

    fn pop_match(&mut self) -> Option<(Key, ValProbe, ValBuild)> {
        self.current_matches.pop_front()
    }

    fn len(&self) -> usize {
        self.len
    }

    fn iter(&self) -> std::collections::hash_map::Iter<'_, Key, SmallVec<[ValBuild; 1]>> {
        #[expect(
            clippy::disallowed_methods,
            reason = "expect non-deterministic iteration order"
        )]
        self.table.iter()
    }

    fn clear(&mut self) {
        self.table.clear();
        self.current_matches.clear();
        self.len = 0;
    }
}
