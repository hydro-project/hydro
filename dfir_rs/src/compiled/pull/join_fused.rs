/// State semantics for (each half of) the `join_fused` operator.
use std::{
    collections::{HashMap, hash_map::Entry},
    hash::{BuildHasher, Hash},
};

/// Generalization of fold, reduce, etc.
pub trait Accumulator<Accum, Item> {
    /// Accumulates a value into an either occupied or vacant table entry.
    fn accumulate<Key>(&mut self, entry: Entry<'_, Key, Accum>, item: Item);

    /// Accumulate all key-value pairs in an iterator into the hash_map.
    fn accumulate_all<Key>(
        &mut self,
        hash_map: &mut HashMap<Key, Accum, impl BuildHasher>,
        iter: impl Iterator<Item = (Key, Item)>,
    ) where
        Key: Eq + Hash,
    {
        for (key, item) in iter {
            self.accumulate(hash_map.entry(key), item);
        }
    }
}

/// Fold with an initialization and fold function.
pub struct Fold<InitFn, FoldFn>(pub InitFn, pub FoldFn);

impl<InitFn, FoldFn, Accum, Item> Accumulator<Accum, Item> for Fold<InitFn, FoldFn>
where
    InitFn: Fn() -> Accum,
    FoldFn: Fn(&mut Accum, Item),
{
    fn accumulate<Key>(&mut self, entry: Entry<'_, Key, Accum>, item: Item) {
        let prev_item = entry.or_insert_with(|| (self.0)());
        let () = (self.1)(prev_item, item);
    }
}

/// Reduce with a reduce function.
pub struct Reduce<ReduceFn>(pub ReduceFn);

impl<ReduceFn, Item> Accumulator<Item, Item> for Reduce<ReduceFn>
where
    ReduceFn: Fn(&mut Item, Item),
{
    fn accumulate<Key>(&mut self, entry: Entry<'_, Key, Item>, item: Item) {
        match entry {
            Entry::Occupied(mut entry) => {
                let prev_item = entry.get_mut();
                let () = (self.0)(prev_item, item);
            }
            Entry::Vacant(entry) => {
                entry.insert(item);
            }
        }
    }
}

/// Fold but with initialization by converting the first received item.
pub struct FoldFrom<InitFn, FoldFn>(pub InitFn, pub FoldFn);

impl<InitFn, FoldFn, Accum, Item> Accumulator<Accum, Item> for FoldFrom<InitFn, FoldFn>
where
    InitFn: Fn(Item) -> Accum,
    FoldFn: Fn(&mut Accum, Item),
{
    fn accumulate<Key>(&mut self, entry: Entry<'_, Key, Accum>, item: Item) {
        match entry {
            Entry::Occupied(mut entry) => {
                let prev_item = entry.get_mut();
                let () = (self.1)(prev_item, item);
            }
            Entry::Vacant(entry) => {
                entry.insert((self.0)(item));
            }
        }
    }
}
