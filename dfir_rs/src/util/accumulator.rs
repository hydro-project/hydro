//! Accumulator trait and implementors.
use std::collections::hash_map::Entry;

/// Generalization of fold, reduce, etc.
pub trait Accumulator<Accum, Item> {
    /// Accumulates a value into an either occupied or vacant table entry.
    fn accumulate<Key>(&mut self, entry: Entry<'_, Key, Accum>, item: Item);
}

/// Fold with an initialization and fold function.
pub struct Fold<InitFn, FoldFn> {
    init_fn: InitFn,
    fold_fn: FoldFn,
}

impl<InitFn, FoldFn> Fold<InitFn, FoldFn> {
    /// Create a `Fold` [`Accumulator`] with the given `InitFn` and `FoldFn`.
    pub fn new<Accum, Item>(init_fn: InitFn, fold_fn: FoldFn) -> Self
    where
        Self: Accumulator<Accum, Item>,
    {
        Self { init_fn, fold_fn }
    }
}

impl<InitFn, FoldFn, Accum, Item> Accumulator<Accum, Item> for Fold<InitFn, FoldFn>
where
    InitFn: Fn() -> Accum,
    FoldFn: Fn(&mut Accum, Item),
{
    fn accumulate<Key>(&mut self, entry: Entry<'_, Key, Accum>, item: Item) {
        let prev_item = entry.or_insert_with(|| (self.init_fn)());
        let () = (self.fold_fn)(prev_item, item);
    }
}

/// Reduce with a reduce function.
pub struct Reduce<ReduceFn> {
    reduce_fn: ReduceFn,
}

impl<ReduceFn> Reduce<ReduceFn> {
    /// Create a `Reduce` [`Accumulator`] with the given `ReduceFn`.
    pub fn new<Item>(reduce_fn: ReduceFn) -> Self
    where
        Self: Accumulator<Item, Item>,
    {
        Self { reduce_fn }
    }
}

impl<ReduceFn, Item> Accumulator<Item, Item> for Reduce<ReduceFn>
where
    ReduceFn: Fn(&mut Item, Item),
{
    fn accumulate<Key>(&mut self, entry: Entry<'_, Key, Item>, item: Item) {
        match entry {
            Entry::Vacant(entry) => {
                entry.insert(item);
            }
            Entry::Occupied(mut entry) => {
                let prev_item = entry.get_mut();
                let () = (self.reduce_fn)(prev_item, item);
            }
        }
    }
}

/// Fold but with initialization by converting the first received item.
pub struct FoldFrom<InitFn, FoldFn> {
    init_fn: InitFn,
    fold_fn: FoldFn,
}

impl<InitFn, FoldFn> FoldFrom<InitFn, FoldFn> {
    /// Create a `FoldFrom` [`Accumulator`] with the given `InitFn` and `FoldFn`.
    pub fn new<Accum, Item>(init_fn: InitFn, fold_fn: FoldFn) -> Self
    where
        Self: Accumulator<Accum, Item>,
    {
        Self { init_fn, fold_fn }
    }
}

impl<InitFn, FoldFn, Accum, Item> Accumulator<Accum, Item> for FoldFrom<InitFn, FoldFn>
where
    InitFn: Fn(Item) -> Accum,
    FoldFn: Fn(&mut Accum, Item),
{
    fn accumulate<Key>(&mut self, entry: Entry<'_, Key, Accum>, item: Item) {
        match entry {
            Entry::Vacant(entry) => {
                entry.insert((self.init_fn)(item));
            }
            Entry::Occupied(mut entry) => {
                let prev_item = entry.get_mut();
                let () = (self.fold_fn)(prev_item, item);
            }
        }
    }
}
