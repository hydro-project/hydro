//! Accumulator trait and implementors.

use core::future::Future;
use core::pin::Pin;
use core::task::Poll;
use std::collections::hash_map::Entry;
use std::hash::{BuildHasher, Hash};

use pin_project_lite::pin_project;

use crate::{Context, Pull, Step};

/// Generalization of fold, reduce, etc.
pub trait Accumulator<ValAccum, ValIn> {
    /// Accumulates a value into an either occupied or vacant table entry.
    fn accumulate<Key>(&mut self, entry: Entry<'_, Key, ValAccum>, item: ValIn);
}

/// Fold with an initialization and fold function.
#[derive(Clone, Debug)]
pub struct Fold<InitFn, FoldFn> {
    init_fn: InitFn,
    fold_fn: FoldFn,
}

impl<InitFn, FoldFn> Fold<InitFn, FoldFn> {
    /// Create a `Fold` [`Accumulator`] with the given `InitFn` and `FoldFn`.
    pub const fn new<Accum, Item>(init_fn: InitFn, fold_fn: FoldFn) -> Self
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
#[derive(Clone, Debug)]
pub struct Reduce<ReduceFn> {
    reduce_fn: ReduceFn,
}

impl<ReduceFn> Reduce<ReduceFn> {
    /// Create a `Reduce` [`Accumulator`] with the given `ReduceFn`.
    pub const fn new<Item>(reduce_fn: ReduceFn) -> Self
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
#[derive(Clone, Debug)]
pub struct FoldFrom<InitFn, FoldFn> {
    init_fn: InitFn,
    fold_fn: FoldFn,
}

impl<InitFn, FoldFn> FoldFrom<InitFn, FoldFn> {
    /// Create a `FoldFrom` [`Accumulator`] with the given `InitFn` and `FoldFn`.
    pub const fn new<Accum, Item>(init_fn: InitFn, fold_fn: FoldFn) -> Self
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

pin_project! {
    /// Future for [`accumulate_all`].
    #[must_use = "futures do nothing unless polled"]
        pub struct AccumulateAll<'a, Prev, Accum, Key, ValAccum, ValIn, S> {
        #[pin]
        prev: Prev,
        accum: &'a mut Accum,
        hash_map: &'a mut std::collections::HashMap<Key, ValAccum, S>,
        _marker: core::marker::PhantomData<ValIn>,
    }
}

impl<'a, Prev, Accum, Key, ValAccum, ValIn, S>
    AccumulateAll<'a, Prev, Accum, Key, ValAccum, ValIn, S>
where
    Self: Future,
{
    pub(crate) const fn new(
        prev: Prev,
        accum: &'a mut Accum,
        hash_map: &'a mut std::collections::HashMap<Key, ValAccum, S>,
    ) -> Self {
        Self {
            prev,
            accum,
            hash_map,
            _marker: core::marker::PhantomData,
        }
    }
}

impl<'a, Prev, Accum, Key, ValAccum, ValIn, S> Future
    for AccumulateAll<'a, Prev, Accum, Key, ValAccum, ValIn, S>
where
    Prev: Pull<Item = (Key, ValIn)>,
    Accum: Accumulator<ValAccum, ValIn>,
    Key: Eq + Hash,
    S: BuildHasher,
    for<'ctx> Prev::Ctx<'ctx>: Context<'ctx>,
{
    type Output = ();

    fn poll(self: Pin<&mut Self>, cx: &mut core::task::Context<'_>) -> Poll<Self::Output> {
        let mut this = self.project();
        let ctx = <Prev::Ctx<'_> as Context<'_>>::from_task(cx);
        loop {
            return match this.prev.as_mut().pull(ctx) {
                Step::Ready((key, item), _meta) => {
                    this.accum.accumulate(this.hash_map.entry(key), item);
                    continue;
                }
                Step::Pending(_) => Poll::Pending,
                Step::Ended(_) => Poll::Ready(()),
            };
        }
    }
}

/// Use the accumulator `accum` to accumulate all entries in the `Pull` `prev` into the `hash_map`.
pub const fn accumulate_all<'a, Key, ValAccum, ValIn, Accum, S, Prev>(
    accum: &'a mut Accum,
    hash_map: &'a mut std::collections::HashMap<Key, ValAccum, S>,
    prev: Prev,
) -> AccumulateAll<'a, Prev, Accum, Key, ValAccum, ValIn, S>
where
    Key: Eq + Hash,
    Accum: Accumulator<ValAccum, ValIn>,
    Prev: Pull<Item = (Key, ValIn)>,
    S: BuildHasher,
{
    AccumulateAll::new(prev, accum, hash_map)
}
