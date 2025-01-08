use std::collections::hash_map::Entry::*;

use rustc_hash::FxHashMap;

use crate::util::clear::Clear;

pub struct HalfJoinStateReduce<K, A> {
    pub table: FxHashMap<K, A>,
}

impl<K, A> Default for HalfJoinStateReduce<K, A> {
    fn default() -> Self {
        Self {
            table: Default::default(),
        }
    }
}

impl<K, A> Clear for HalfJoinStateReduce<K, A> {
    fn clear(&mut self) {
        self.table.clear()
    }
}

impl<K, A> HalfJoinStateReduce<K, A>
where
    K: Eq + std::hash::Hash,
{
    pub fn reduce_into<X>(
        &mut self,
        iter: impl Iterator<Item = (K, A)>,
        mut reduce: impl FnMut(&mut A, A) -> X,
    ) {
        for (k, v) in iter {
            let entry = self.table.entry(k);

            match entry {
                Occupied(mut e) => {
                    (reduce)(e.get_mut(), v);
                }
                Vacant(e) => {
                    e.insert(v);
                }
            }
        }
    }
}
