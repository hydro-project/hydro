use std::hash::Hash;

use stageleft::{IntoQuotedMut, QuotedWithContext, q};

use crate::cycle::{CycleCollection, CycleComplete, ForwardRefMarker};
use crate::location::tick::NoAtomic;
use crate::location::{LocationId, NoTick};
use crate::manual_expr::ManualExpr;
use crate::stream::ExactlyOnce;
use crate::unsafety::NonDet;
use crate::{
    Atomic, Bounded, KeyedStream, Location, NoOrder, Optional, Singleton, Stream, Tick, TotalOrder,
    nondet,
};

pub struct KeyedSingleton<K, V, Loc, Bound> {
    pub(crate) underlying: Stream<(K, V), Loc, Bound, NoOrder, ExactlyOnce>,
}

impl<'a, K: Clone, V: Clone, Loc: Location<'a>, Bound> Clone for KeyedSingleton<K, V, Loc, Bound> {
    fn clone(&self) -> Self {
        KeyedSingleton {
            underlying: self.underlying.clone(),
        }
    }
}

impl<'a, K, V, L, B> CycleCollection<'a, ForwardRefMarker> for KeyedSingleton<K, V, L, B>
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

impl<'a, K, V, L, B> CycleComplete<'a, ForwardRefMarker> for KeyedSingleton<K, V, L, B>
where
    L: Location<'a> + NoTick,
{
    fn complete(self, ident: syn::Ident, expected_location: LocationId) {
        self.underlying.complete(ident, expected_location);
    }
}

impl<'a, K, V, L: Location<'a>> KeyedSingleton<K, V, Tick<L>, Bounded> {
    pub fn entries(self) -> Stream<(K, V), Tick<L>, Bounded, NoOrder, ExactlyOnce> {
        self.underlying
    }

    pub fn values(self) -> Stream<V, Tick<L>, Bounded, NoOrder, ExactlyOnce> {
        self.underlying.map(q!(|(_, v)| v))
    }

    pub fn keys(self) -> Stream<K, Tick<L>, Bounded, NoOrder, ExactlyOnce> {
        self.underlying.map(q!(|(k, _)| k))
    }
}

impl<'a, K, V, L: Location<'a>, B> KeyedSingleton<K, V, L, B> {
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

    pub fn key_count(self) -> Singleton<usize, L, B> {
        self.underlying.count()
    }
}

impl<'a, K: Hash + Eq, V, L: Location<'a>> KeyedSingleton<K, V, Tick<L>, Bounded> {
    /// Gets the value associated with a specific key from the keyed singleton.
    ///
    /// # Example
    /// ```rust
    /// # use hydro_lang::*;
    /// # use futures::StreamExt;
    /// # tokio_test::block_on(test_util::stream_transform_test(|process| {
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
    /// of the request.
    ///
    /// # Example
    /// ```rust
    /// # use hydro_lang::*;
    /// # use futures::StreamExt;
    /// # tokio_test::block_on(test_util::stream_transform_test(|process| {
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
    /// keyed_data.get_many(other_data).entries().all_ticks()
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
    pub fn get_many<O2, R2, V2>(
        self,
        with: KeyedStream<K, V2, Tick<L>, Bounded, O2, R2>,
    ) -> KeyedStream<K, (V, V2), Tick<L>, Bounded, NoOrder, R2> {
        self.entries()
            .weaker_retries()
            .join(with.entries())
            .into_keyed()
    }
}

impl<'a, K, V, L, B> KeyedSingleton<K, V, L, B>
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

impl<'a, K, V, L, B> KeyedSingleton<K, V, Atomic<L>, B>
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
}
