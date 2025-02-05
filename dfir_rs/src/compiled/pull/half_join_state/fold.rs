use std::collections::hash_map::Entry::*;

use rustc_hash::FxHashMap;

use crate::util::clear::Clear;

pub struct HalfJoinStateFold<K, A> {
    pub table: FxHashMap<K, A>,
}

impl<K, A> Default for HalfJoinStateFold<K, A> {
    fn default() -> Self {
        Self {
            table: Default::default(),
        }
    }
}

impl<K, A> Clear for HalfJoinStateFold<K, A> {
    fn clear(&mut self) {
        self.table.clear()
    }
}

impl<K, A> HalfJoinStateFold<K, A>
where
    K: Eq + std::hash::Hash,
{
    pub fn fold_into<V, X>(
        &mut self,
        iter: impl Iterator<Item = (K, V)>,
        mut fold: impl FnMut(&mut A, V) -> X,
        mut default: impl FnMut() -> A,
    ) {
        for (k, v) in iter {
            let entry = self.table.entry(k);

            match entry {
                Occupied(mut e) => {
                    (fold)(e.get_mut(), v);
                }
                Vacant(e) => {
                    (fold)(e.insert((default)()), v);
                }
            }
        }
    }
}
