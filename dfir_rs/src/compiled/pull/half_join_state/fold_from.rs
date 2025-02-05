use std::collections::hash_map::Entry::*;

use rustc_hash::FxHashMap;

use crate::util::clear::Clear;

pub struct HalfJoinStateFoldFrom<K, A> {
    pub table: FxHashMap<K, A>,
}

impl<K, A> Default for HalfJoinStateFoldFrom<K, A> {
    fn default() -> Self {
        Self {
            table: Default::default(),
        }
    }
}

impl<K, A> Clear for HalfJoinStateFoldFrom<K, A> {
    fn clear(&mut self) {
        self.table.clear()
    }
}

impl<K, A> HalfJoinStateFoldFrom<K, A>
where
    K: Eq + std::hash::Hash,
{
    pub fn fold_into<V, X>(
        &mut self,
        iter: impl Iterator<Item = (K, V)>,
        mut fold: impl FnMut(&mut A, V) -> X,
        mut from: impl FnMut(V) -> A,
    ) {
        for (k, v) in iter {
            let entry = self.table.entry(k);

            match entry {
                Occupied(mut e) => {
                    (fold)(e.get_mut(), v);
                }
                Vacant(e) => {
                    e.insert((from)(v));
                }
            }
        }
    }
}
