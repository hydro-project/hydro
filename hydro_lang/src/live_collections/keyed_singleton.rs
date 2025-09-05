use std::hash::Hash;

use stageleft::{IntoQuotedMut, QuotedWithContext, q};

use super::boundedness::{Bounded, Boundedness, Unbounded};
use super::keyed_stream::KeyedStream;
use super::optional::Optional;
use super::singleton::Singleton;
use super::stream::{ExactlyOnce, NoOrder, Stream, TotalOrder};
use crate::cycle::{CycleCollection, CycleComplete, ForwardRefMarker};
use crate::location::tick::NoAtomic;
use crate::location::{Atomic, Location, LocationId, NoTick, Tick};
use crate::manual_expr::ManualExpr;
use crate::nondet::{NonDet, nondet};

pub trait KeyedSingletonBound {
    type UnderlyingBound: Boundedness;
    type ValueBound: Boundedness;
}

impl KeyedSingletonBound for Unbounded {
    type UnderlyingBound = Unbounded;
    type ValueBound = Unbounded;
}

impl KeyedSingletonBound for Bounded {
    type UnderlyingBound = Bounded;
    type ValueBound = Bounded;
}

/// A variation of boundedness specific to [`KeyedSingleton`], which indicates that once a key appears,
/// its value is bounded and will never change. If the `KeyBound` is [`Bounded`], then the entire set of entries
/// is bounded, but if it is [`Unbounded`], then new entries may appear asynchronously.
pub struct BoundedValue;

impl KeyedSingletonBound for BoundedValue {
    type UnderlyingBound = Unbounded;
    type ValueBound = Bounded;
}

pub struct KeyedSingleton<K, V, Loc, Bound: KeyedSingletonBound> {
    pub(crate) underlying: Stream<(K, V), Loc, Bound::UnderlyingBound, NoOrder, ExactlyOnce>,
}

impl<'a, K: Clone, V: Clone, Loc: Location<'a>, Bound: KeyedSingletonBound> Clone
    for KeyedSingleton<K, V, Loc, Bound>
{
    fn clone(&self) -> Self {
        KeyedSingleton {
            underlying: self.underlying.clone(),
        }
    }
}

impl<'a, K, V, L, B: KeyedSingletonBound> CycleCollection<'a, ForwardRefMarker>
    for KeyedSingleton<K, V, L, B>
where
    L: Location<'a> + NoTick,
{
    type Location = L;

    fn create_source(ident: syn::Ident, location: L) -> Self {
        KeyedSingleton {
            underlying: Stream::create_source(ident, location),
        }
    }
}

impl<'a, K, V, L, B: KeyedSingletonBound> CycleComplete<'a, ForwardRefMarker>
    for KeyedSingleton<K, V, L, B>
where
    L: Location<'a> + NoTick,
{
    fn complete(self, ident: syn::Ident, expected_location: LocationId) {
        self.underlying.complete(ident, expected_location);
    }
}

impl<'a, K, V, L: Location<'a>, B: KeyedSingletonBound<ValueBound = Bounded>>
    KeyedSingleton<K, V, L, B>
{
    pub fn entries(self) -> Stream<(K, V), L, B::UnderlyingBound, NoOrder, ExactlyOnce> {
        self.underlying
    }

    pub fn values(self) -> Stream<V, L, B::UnderlyingBound, NoOrder, ExactlyOnce> {
        self.entries().map(q!(|(_, v)| v))
    }

    pub fn keys(self) -> Stream<K, L, B::UnderlyingBound, NoOrder, ExactlyOnce> {
        self.entries().map(q!(|(k, _)| k))
    }

    pub fn filter_key_not_in<O2, R2>(self, other: Stream<K, L, Bounded, O2, R2>) -> Self
    where
        K: Hash + Eq,
    {
        KeyedSingleton {
            underlying: self.entries().anti_join(other),
        }
    }

    pub fn inspect<F>(self, f: impl IntoQuotedMut<'a, F, L> + Copy) -> KeyedSingleton<K, V, L, B>
    where
        F: Fn(&V) + 'a,
    {
        let f: ManualExpr<F, _> = ManualExpr::new(move |ctx: &L| f.splice_fn1_borrow_ctx(ctx));
        KeyedSingleton {
            underlying: self.underlying.inspect(q!({
                let orig = f;
                move |(_k, v)| orig(v)
            })),
        }
    }

    pub fn inspect_with_key<F>(self, f: impl IntoQuotedMut<'a, F, L>) -> KeyedSingleton<K, V, L, B>
    where
        F: Fn(&(K, V)) + 'a,
    {
        KeyedSingleton {
            underlying: self.underlying.inspect(f),
        }
    }

    pub fn into_keyed_stream(
        self,
    ) -> KeyedStream<K, V, L, B::UnderlyingBound, TotalOrder, ExactlyOnce> {
        self.underlying
            .into_keyed()
            .assume_ordering(nondet!(/** only one element per key */))
    }
}

impl<'a, K, V, L: Location<'a>, B: KeyedSingletonBound> KeyedSingleton<K, V, L, B> {
    pub fn map<U, F>(self, f: impl IntoQuotedMut<'a, F, L> + Copy) -> KeyedSingleton<K, U, L, B>
    where
        F: Fn(V) -> U + 'a,
    {
        let f: ManualExpr<F, _> = ManualExpr::new(move |ctx: &L| f.splice_fn1_ctx(ctx));
        KeyedSingleton {
            underlying: self.underlying.map(q!({
                let orig = f;
                move |(k, v)| (k, orig(v))
            })),
        }
    }

    pub fn map_with_key<U, F>(
        self,
        f: impl IntoQuotedMut<'a, F, L> + Copy,
    ) -> KeyedSingleton<K, U, L, B>
    where
        F: Fn((K, V)) -> U + 'a,
        K: Clone,
    {
        let f: ManualExpr<F, _> = ManualExpr::new(move |ctx: &L| f.splice_fn1_ctx(ctx));
        KeyedSingleton {
            underlying: self.underlying.map(q!({
                let orig = f;
                move |(k, v)| {
                    let out = orig((k.clone(), v));
                    (k, out)
                }
            })),
        }
    }

    pub fn filter<F>(self, f: impl IntoQuotedMut<'a, F, L> + Copy) -> KeyedSingleton<K, V, L, B>
    where
        F: Fn(&V) -> bool + 'a,
    {
        let f: ManualExpr<F, _> = ManualExpr::new(move |ctx: &L| f.splice_fn1_borrow_ctx(ctx));
        KeyedSingleton {
            underlying: self.underlying.filter(q!({
                let orig = f;
                move |(_k, v)| orig(v)
            })),
        }
    }

    pub fn filter_map<F, U>(
        self,
        f: impl IntoQuotedMut<'a, F, L> + Copy,
    ) -> KeyedSingleton<K, U, L, B>
    where
        F: Fn(V) -> Option<U> + 'a,
    {
        let f: ManualExpr<F, _> = ManualExpr::new(move |ctx: &L| f.splice_fn1_ctx(ctx));
        KeyedSingleton {
            underlying: self.underlying.filter_map(q!({
                let orig = f;
                move |(k, v)| orig(v).map(|v| (k, v))
            })),
        }
    }

    pub fn key_count(self) -> Singleton<usize, L, B::UnderlyingBound> {
        self.underlying.count()
    }

    /// An operator which allows you to "name" a `HydroNode`.
    /// This is only used for testing, to correlate certain `HydroNode`s with IDs.
    pub fn ir_node_named(self, name: &str) -> KeyedSingleton<K, V, L, B> {
        {
            let mut node = self.underlying.ir_node.borrow_mut();
            let metadata = node.metadata_mut();
            metadata.tag = Some(name.to_string());
        }
        self
    }
}

impl<'a, K, V, L: Location<'a>> KeyedSingleton<K, V, Tick<L>, Bounded> {
    pub fn latest(self) -> KeyedSingleton<K, V, L, Unbounded> {
        KeyedSingleton {
            underlying: Stream::new(
                self.underlying.location.outer().clone(),
                // no need to persist due to top-level replay
                self.underlying.ir_node.into_inner(),
            ),
        }
    }
}

impl<'a, K: Hash + Eq, V, L: Location<'a>> KeyedSingleton<K, V, Tick<L>, Bounded> {
    /// Gets the value associated with a specific key from the keyed singleton.
    ///
    /// # Example
    /// ```rust
    /// # use hydro_lang::prelude::*;
    /// # use futures::StreamExt;
    /// # tokio_test::block_on(hydro_lang::test_util::stream_transform_test(|process| {
    /// let tick = process.tick();
    /// let keyed_data = process
    ///     .source_iter(q!(vec![(1, 2), (2, 3)]))
    ///     .into_keyed()
    ///     .batch(&tick, nondet!(/** test */))
    ///     .fold(q!(|| 0), q!(|acc, x| *acc = x));
    /// let key = tick.singleton(q!(1));
    /// keyed_data.get(key).all_ticks()
    /// # }, |mut stream| async move {
    /// // 2
    /// # assert_eq!(stream.next().await.unwrap(), 2);
    /// # }));
    /// ```
    pub fn get(self, key: Singleton<K, Tick<L>, Bounded>) -> Optional<V, Tick<L>, Bounded> {
        self.entries()
            .join(key.into_stream().map(q!(|k| (k, ()))))
            .map(q!(|(_, (v, _))| v))
            .assume_ordering::<TotalOrder>(nondet!(/** only a single key, so totally ordered */))
            .first()
    }

    /// Given a keyed stream of lookup requests, where the key is the lookup and the value
    /// is some additional metadata, emits a keyed stream of lookup results where the key
    /// is the same as before, but the value is a tuple of the lookup result and the metadata
    /// of the request. If the key is not found, no output will be produced.
    ///
    /// # Example
    /// ```rust
    /// # use hydro_lang::prelude::*;
    /// # use futures::StreamExt;
    /// # tokio_test::block_on(hydro_lang::test_util::stream_transform_test(|process| {
    /// let tick = process.tick();
    /// let keyed_data = process
    ///     .source_iter(q!(vec![(1, 10), (2, 20)]))
    ///     .into_keyed()
    ///     .batch(&tick, nondet!(/** test */))
    ///     .fold(q!(|| 0), q!(|acc, x| *acc = x));
    /// let other_data = process
    ///     .source_iter(q!(vec![(1, 100), (2, 200), (1, 101)]))
    ///     .into_keyed()
    ///     .batch(&tick, nondet!(/** test */));
    /// keyed_data.get_many_if_present(other_data).entries().all_ticks()
    /// # }, |mut stream| async move {
    /// // { 1: [(10, 100), (10, 101)], 2: [(20, 200)] } in any order
    /// # let mut results = vec![];
    /// # for _ in 0..3 {
    /// #     results.push(stream.next().await.unwrap());
    /// # }
    /// # results.sort();
    /// # assert_eq!(results, vec![(1, (10, 100)), (1, (10, 101)), (2, (20, 200))]);
    /// # }));
    /// ```
    pub fn get_many_if_present<O2, R2, V2>(
        self,
        requests: KeyedStream<K, V2, Tick<L>, Bounded, O2, R2>,
    ) -> KeyedStream<K, (V, V2), Tick<L>, Bounded, NoOrder, R2> {
        self.entries()
            .weaker_retries()
            .join(requests.entries())
            .into_keyed()
    }

    pub fn get_from<V2: Clone>(
        self,
        from: KeyedSingleton<V, V2, Tick<L>, Bounded>,
    ) -> KeyedSingleton<K, (V, Option<V2>), Tick<L>, Bounded>
    where
        K: Clone,
        V: Hash + Eq + Clone,
    {
        let to_lookup = self.entries().map(q!(|(k, v)| (v, k))).into_keyed();
        let lookup_result = from.get_many_if_present(to_lookup.clone());
        let missing_values =
            to_lookup.filter_key_not_in(lookup_result.clone().entries().map(q!(|t| t.0)));
        KeyedSingleton {
            underlying: lookup_result
                .entries()
                .map(q!(|(v, (v2, k))| (k, (v, Some(v2)))))
                .chain(missing_values.entries().map(q!(|(v, k)| (k, (v, None))))),
        }
    }
}

impl<'a, K, V, L, B: KeyedSingletonBound> KeyedSingleton<K, V, L, B>
where
    L: Location<'a> + NoTick + NoAtomic,
{
    pub fn atomic(self, tick: &Tick<L>) -> KeyedSingleton<K, V, Atomic<L>, B> {
        KeyedSingleton {
            underlying: self.underlying.atomic(tick),
        }
    }

    /// Given a tick, returns a keyed singleton with a entries consisting of keys with
    /// snapshots of the value singleton.
    ///
    /// # Non-Determinism
    /// Because this picks a snapshot of each singleton whose value is continuously changing,
    /// the output singleton has a non-deterministic value since each snapshot can be at an
    /// arbitrary point in time.
    pub fn snapshot(
        self,
        tick: &Tick<L>,
        nondet: NonDet,
    ) -> KeyedSingleton<K, V, Tick<L>, Bounded> {
        self.atomic(tick).snapshot(nondet)
    }
}

impl<'a, K, V, L, B: KeyedSingletonBound<ValueBound = Bounded>> KeyedSingleton<K, V, L, B>
where
    L: Location<'a> + NoTick + NoAtomic,
{
    /// Returns a keyed singleton with entries consisting of _new_ key-value pairs that have
    /// arrived since the previous batch was released.
    ///
    /// # Non-Determinism
    /// Because this picks a batch of asynchronously added entries, each output keyed singleton
    /// has a non-deterministic set of key-value pairs.
    pub fn batch(self, tick: &Tick<L>, nondet: NonDet) -> KeyedSingleton<K, V, Tick<L>, Bounded> {
        self.atomic(tick).batch(nondet)
    }
}

impl<'a, K, V, L, B: KeyedSingletonBound> KeyedSingleton<K, V, Atomic<L>, B>
where
    L: Location<'a> + NoTick + NoAtomic,
{
    /// Returns a keyed singleton with a entries consisting of keys with snapshots of the value
    /// singleton being atomically processed.
    ///
    /// # Non-Determinism
    /// Because this picks a snapshot of each singleton whose value is continuously changing,
    /// each output singleton has a non-deterministic value since each snapshot can be at an
    /// arbitrary point in time.
    pub fn snapshot(self, _nondet: NonDet) -> KeyedSingleton<K, V, Tick<L>, Bounded> {
        KeyedSingleton {
            underlying: Stream::new(
                self.underlying.location.tick,
                // no need to unpersist due to top-level replay
                self.underlying.ir_node.into_inner(),
            ),
        }
    }

    pub fn end_atomic(self) -> KeyedSingleton<K, V, L, B> {
        KeyedSingleton {
            underlying: self.underlying.end_atomic(),
        }
    }
}

impl<'a, K, V, L, B: KeyedSingletonBound<ValueBound = Bounded>> KeyedSingleton<K, V, Atomic<L>, B>
where
    L: Location<'a> + NoTick + NoAtomic,
{
    /// Returns a keyed singleton with entries consisting of _new_ key-value pairs that have
    /// arrived since the previous batch was released.
    ///
    /// # Non-Determinism
    /// Because this picks a batch of asynchronously added entries, each output keyed singleton
    /// has a non-deterministic set of key-value pairs.
    pub fn batch(self, nondet: NonDet) -> KeyedSingleton<K, V, Tick<L>, Bounded> {
        KeyedSingleton {
            underlying: self.underlying.batch(nondet),
        }
    }
}
