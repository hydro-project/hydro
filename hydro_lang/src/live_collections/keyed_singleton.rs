//! Definitions for the [`KeyedSingleton`] live collection.

use std::hash::Hash;

use stageleft::{IntoQuotedMut, QuotedWithContext, q};

use super::boundedness::{Bounded, Boundedness, Unbounded};
use super::keyed_stream::KeyedStream;
use super::optional::Optional;
use super::singleton::Singleton;
use super::stream::{ExactlyOnce, NoOrder, Stream, TotalOrder};
use crate::forward_handle::ForwardRef;
#[cfg(stageleft_runtime)]
use crate::forward_handle::{CycleCollection, ReceiverComplete};
use crate::live_collections::stream::{Ordering, Retries};
use crate::location::dynamic::LocationId;
use crate::location::{Atomic, Location, NoTick, Tick};
use crate::manual_expr::ManualExpr;
use crate::nondet::{NonDet, nondet};

/// A marker trait indicating which components of a [`KeyedSingleton`] may change.
///
/// In addition to [`Bounded`] (all entries are fixed) and [`Unbounded`] (entries may be added /
/// removed / changed), this also includes an additional variant [`BoundedValue`], which indicates
/// that entries may be added over time, but once an entry is added it will never be removed and
/// its value will never change.
pub trait KeyedSingletonBound {
    /// The [`Boundedness`] of the [`Stream`] underlying the keyed singleton.
    type UnderlyingBound: Boundedness;
    /// The [`Boundedness`] of each entry's value; [`Bounded`] means it is immutable.
    type ValueBound: Boundedness;

    /// The type of the keyed singleton if the value for each key is immutable.
    type WithBoundedValue: KeyedSingletonBound<UnderlyingBound = Self::UnderlyingBound, ValueBound = Bounded>;

    /// The type of the keyed singleton if the value for each key may change asynchronously.
    type WithUnboundedValue: KeyedSingletonBound<UnderlyingBound = Self::UnderlyingBound, ValueBound = Unbounded>;
}

impl KeyedSingletonBound for Unbounded {
    type UnderlyingBound = Unbounded;
    type ValueBound = Unbounded;
    type WithBoundedValue = BoundedValue;
    type WithUnboundedValue = Unbounded;
}

impl KeyedSingletonBound for Bounded {
    type UnderlyingBound = Bounded;
    type ValueBound = Bounded;
    type WithBoundedValue = Bounded;
    type WithUnboundedValue = UnreachableBound;
}

/// A variation of boundedness specific to [`KeyedSingleton`], which indicates that once a key appears,
/// its value is bounded and will never change. If the `KeyBound` is [`Bounded`], then the entire set of entries
/// is bounded, but if it is [`Unbounded`], then new entries may appear asynchronously.
pub struct BoundedValue;

impl KeyedSingletonBound for BoundedValue {
    type UnderlyingBound = Unbounded;
    type ValueBound = Bounded;
    type WithBoundedValue = BoundedValue;
    type WithUnboundedValue = Unbounded;
}

#[doc(hidden)]
pub struct UnreachableBound;

impl KeyedSingletonBound for UnreachableBound {
    type UnderlyingBound = Bounded;
    type ValueBound = Unbounded;

    type WithBoundedValue = Bounded;
    type WithUnboundedValue = UnreachableBound;
}

/// Mapping from keys of type `K` to values of type `V`.
///
/// Keyed Singletons capture an asynchronously updated mapping from keys of the `K` to values of
/// type `V`, where the order of keys is non-deterministic. In addition to the standard boundedness
/// variants ([`Bounded`] for finite and immutable, [`Unbounded`] for asynchronously changing),
/// keyed singletons can use [`BoundedValue`] to declare that new keys may be added over time, but
/// keys cannot be removed and the value for each key is immutable.
///
/// Type Parameters:
/// - `K`: the type of the key for each entry
/// - `V`: the type of the value for each entry
/// - `Loc`: the [`Location`] where the keyed singleton is materialized
/// - `Bound`: tracks whether the entries are:
///     - [`Bounded`] (local and finite)
///     - [`Unbounded`] (asynchronous with entries added / removed / changed over time)
///     - [`BoundedValue`] (asynchronous with immutable values for each key and no removals)
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

impl<'a, K, V, L, B: KeyedSingletonBound> CycleCollection<'a, ForwardRef>
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

impl<'a, K, V, L, B: KeyedSingletonBound> ReceiverComplete<'a, ForwardRef>
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
    /// Flattens the keyed singleton into an unordered stream of key-value pairs.
    ///
    /// The value for each key must be bounded, otherwise the resulting stream elements would be
    /// non-determinstic. As new entries are added to the keyed singleton, they will be streamed
    /// into the output.
    ///
    /// # Example
    /// ```rust
    /// # use hydro_lang::prelude::*;
    /// # use futures::StreamExt;
    /// # tokio_test::block_on(hydro_lang::test_util::stream_transform_test(|process| {
    /// let keyed_singleton = // { 1: 2, 2: 4 }
    /// # process
    /// #     .source_iter(q!(vec![(1, 2), (2, 4)]))
    /// #     .into_keyed()
    /// #     .first();
    /// keyed_singleton.entries()
    /// # }, |mut stream| async move {
    /// // (1, 2), (2, 4) in any order
    /// # let mut results = Vec::new();
    /// # for _ in 0..2 {
    /// #     results.push(stream.next().await.unwrap());
    /// # }
    /// # results.sort();
    /// # assert_eq!(results, vec![(1, 2), (2, 4)]);
    /// # }));
    /// ```
    pub fn entries(self) -> Stream<(K, V), L, B::UnderlyingBound, NoOrder, ExactlyOnce> {
        self.underlying
    }

    /// Flattens the keyed singleton into an unordered stream of just the values.
    ///
    /// The value for each key must be bounded, otherwise the resulting stream elements would be
    /// non-determinstic. As new entries are added to the keyed singleton, they will be streamed
    /// into the output.
    ///
    /// # Example
    /// ```rust
    /// # use hydro_lang::prelude::*;
    /// # use futures::StreamExt;
    /// # tokio_test::block_on(hydro_lang::test_util::stream_transform_test(|process| {
    /// let keyed_singleton = // { 1: 2, 2: 4 }
    /// # process
    /// #     .source_iter(q!(vec![(1, 2), (2, 4)]))
    /// #     .into_keyed()
    /// #     .first();
    /// keyed_singleton.values()
    /// # }, |mut stream| async move {
    /// // 2, 4 in any order
    /// # let mut results = Vec::new();
    /// # for _ in 0..2 {
    /// #     results.push(stream.next().await.unwrap());
    /// # }
    /// # results.sort();
    /// # assert_eq!(results, vec![2, 4]);
    /// # }));
    /// ```
    pub fn values(self) -> Stream<V, L, B::UnderlyingBound, NoOrder, ExactlyOnce> {
        self.entries().map(q!(|(_, v)| v))
    }

    /// Flattens the keyed singleton into an unordered stream of just the keys.
    ///
    /// The value for each key must be bounded, otherwise the removal of keys would result in
    /// non-determinism. As new entries are added to the keyed singleton, they will be streamed
    /// into the output.
    ///
    /// # Example
    /// ```rust
    /// # use hydro_lang::prelude::*;
    /// # use futures::StreamExt;
    /// # tokio_test::block_on(hydro_lang::test_util::stream_transform_test(|process| {
    /// let keyed_singleton = // { 1: 2, 2: 4 }
    /// # process
    /// #     .source_iter(q!(vec![(1, 2), (2, 4)]))
    /// #     .into_keyed()
    /// #     .first();
    /// keyed_singleton.keys()
    /// # }, |mut stream| async move {
    /// // 1, 2 in any order
    /// # let mut results = Vec::new();
    /// # for _ in 0..2 {
    /// #     results.push(stream.next().await.unwrap());
    /// # }
    /// # results.sort();
    /// # assert_eq!(results, vec![1, 2]);
    /// # }));
    /// ```
    pub fn keys(self) -> Stream<K, L, B::UnderlyingBound, NoOrder, ExactlyOnce> {
        self.entries().map(q!(|(k, _)| k))
    }

    /// Given a bounded stream of keys `K`, returns a new keyed singleton containing only the
    /// entries whose keys are not in the provided stream.
    ///
    /// # Example
    /// ```rust
    /// # use hydro_lang::prelude::*;
    /// # use futures::StreamExt;
    /// # tokio_test::block_on(hydro_lang::test_util::stream_transform_test(|process| {
    /// let tick = process.tick();
    /// let keyed_singleton = // { 1: 2, 2: 4 }
    /// # process
    /// #     .source_iter(q!(vec![(1, 2), (2, 4)]))
    /// #     .into_keyed()
    /// #     .first()
    /// #     .batch(&tick, nondet!(/** test */));
    /// let keys_to_remove = process
    ///     .source_iter(q!(vec![1]))
    ///     .batch(&tick, nondet!(/** test */));
    /// keyed_singleton.filter_key_not_in(keys_to_remove)
    /// #   .entries().all_ticks()
    /// # }, |mut stream| async move {
    /// // { 2: 4 }
    /// # for w in vec![(2, 4)] {
    /// #     assert_eq!(stream.next().await.unwrap(), w);
    /// # }
    /// # }));
    /// ```
    pub fn filter_key_not_in<O2: Ordering, R2: Retries>(
        self,
        other: Stream<K, L, Bounded, O2, R2>,
    ) -> Self
    where
        K: Hash + Eq,
    {
        KeyedSingleton {
            underlying: self.entries().anti_join(other),
        }
    }

    /// An operator which allows you to "inspect" each value of a keyed singleton without
    /// modifying it. The closure `f` is called on a reference to each value. This is
    /// mainly useful for debugging, and should not be used to generate side-effects.
    ///
    /// # Example
    /// ```rust
    /// # use hydro_lang::prelude::*;
    /// # use futures::StreamExt;
    /// # tokio_test::block_on(hydro_lang::test_util::stream_transform_test(|process| {
    /// let keyed_singleton = // { 1: 2, 2: 4 }
    /// # process
    /// #     .source_iter(q!(vec![(1, 2), (2, 4)]))
    /// #     .into_keyed()
    /// #     .first();
    /// keyed_singleton
    ///     .inspect(q!(|v| println!("{}", v)))
    /// #   .entries()
    /// # }, |mut stream| async move {
    /// // { 1: 2, 2: 4 }
    /// # for w in vec![(1, 2), (2, 4)] {
    /// #     assert_eq!(stream.next().await.unwrap(), w);
    /// # }
    /// # }));
    /// ```
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

    /// An operator which allows you to "inspect" each entry of a keyed singleton without
    /// modifying it. The closure `f` is called on a reference to each key-value pair. This is
    /// mainly useful for debugging, and should not be used to generate side-effects.
    ///
    /// # Example
    /// ```rust
    /// # use hydro_lang::prelude::*;
    /// # use futures::StreamExt;
    /// # tokio_test::block_on(hydro_lang::test_util::stream_transform_test(|process| {
    /// let keyed_singleton = // { 1: 2, 2: 4 }
    /// # process
    /// #     .source_iter(q!(vec![(1, 2), (2, 4)]))
    /// #     .into_keyed()
    /// #     .first();
    /// keyed_singleton
    ///     .inspect_with_key(q!(|(k, v)| println!("{}: {}", k, v)))
    /// #   .entries()
    /// # }, |mut stream| async move {
    /// // { 1: 2, 2: 4 }
    /// # for w in vec![(1, 2), (2, 4)] {
    /// #     assert_eq!(stream.next().await.unwrap(), w);
    /// # }
    /// # }));
    /// ```
    pub fn inspect_with_key<F>(self, f: impl IntoQuotedMut<'a, F, L>) -> KeyedSingleton<K, V, L, B>
    where
        F: Fn(&(K, V)) + 'a,
    {
        KeyedSingleton {
            underlying: self.underlying.inspect(f),
        }
    }

    /// Converts this keyed singleton into a [`KeyedStream`] with each group having a single
    /// element, the value.
    ///
    /// This is the equivalent of [`Singleton::into_stream`] but keyed.
    ///
    /// # Example
    /// ```rust
    /// # use hydro_lang::prelude::*;
    /// # use futures::StreamExt;
    /// # tokio_test::block_on(hydro_lang::test_util::stream_transform_test(|process| {
    /// let keyed_singleton = // { 1: 2, 2: 4 }
    /// # process
    /// #     .source_iter(q!(vec![(1, 2), (2, 4)]))
    /// #     .into_keyed()
    /// #     .first();
    /// keyed_singleton
    ///     .clone()
    ///     .into_keyed_stream()
    ///     .interleave(
    ///         keyed_singleton.into_keyed_stream()
    ///     )
    /// #   .entries()
    /// # }, |mut stream| async move {
    /// /// // { 1: [2, 2], 2: [4, 4] }
    /// # for w in vec![(1, 2), (2, 4), (1, 2), (2, 4)] {
    /// #     assert_eq!(stream.next().await.unwrap(), w);
    /// # }
    /// # }));
    /// ```
    pub fn into_keyed_stream(
        self,
    ) -> KeyedStream<K, V, L, B::UnderlyingBound, TotalOrder, ExactlyOnce> {
        self.underlying
            .into_keyed()
            .assume_ordering(nondet!(/** only one element per key */))
    }
}

#[cfg(stageleft_runtime)]
fn key_count_inside_tick<'a, K, V, L: Location<'a>>(
    me: KeyedSingleton<K, V, L, Bounded>,
) -> Singleton<usize, L, Bounded> {
    me.underlying.count()
}

impl<'a, K, V, L: Location<'a>, B: KeyedSingletonBound> KeyedSingleton<K, V, L, B> {
    /// Transforms each value by invoking `f` on each element, with keys staying the same
    /// after transformation. If you need access to the key, see [`KeyedStream::map_with_key`].
    ///
    /// If you do not want to modify the stream and instead only want to view
    /// each item use [`KeyedStream::inspect`] instead.
    ///
    /// # Example
    /// ```rust
    /// # use hydro_lang::prelude::*;
    /// # use futures::StreamExt;
    /// # tokio_test::block_on(hydro_lang::test_util::stream_transform_test(|process| {
    /// let keyed_singleton = // { 1: 2, 2: 4 }
    /// # process
    /// #     .source_iter(q!(vec![(1, 2), (2, 4)]))
    /// #     .into_keyed()
    /// #     .first();
    /// keyed_singleton.map(q!(|v| v + 1))
    /// #   .entries()
    /// # }, |mut stream| async move {
    /// // { 1: 3, 2: 5 }
    /// # let mut results = Vec::new();
    /// # for _ in 0..2 {
    /// #     results.push(stream.next().await.unwrap());
    /// # }
    /// # results.sort();
    /// # assert_eq!(results, vec![(1, 3), (2, 5)]);
    /// # }));
    /// ```
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

    /// Transforms each value by invoking `f` on each key-value pair, with keys staying the same
    /// after transformation. Unlike [`KeyedSingleton::map`], this gives access to both the key and value.
    ///
    /// The closure `f` receives a tuple `(K, V)` containing both the key and value, and returns
    /// the new value `U`. The key remains unchanged in the output.
    ///
    /// # Example
    /// ```rust
    /// # use hydro_lang::prelude::*;
    /// # use futures::StreamExt;
    /// # tokio_test::block_on(hydro_lang::test_util::stream_transform_test(|process| {
    /// let keyed_singleton = // { 1: 2, 2: 4 }
    /// # process
    /// #     .source_iter(q!(vec![(1, 2), (2, 4)]))
    /// #     .into_keyed()
    /// #     .first();
    /// keyed_singleton.map_with_key(q!(|(k, v)| k + v))
    /// #   .entries()
    /// # }, |mut stream| async move {
    /// // { 1: 3, 2: 6 }
    /// # let mut results = Vec::new();
    /// # for _ in 0..2 {
    /// #     results.push(stream.next().await.unwrap());
    /// # }
    /// # results.sort();
    /// # assert_eq!(results, vec![(1, 3), (2, 6)]);
    /// # }));
    /// ```
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

    /// Creates a keyed singleton containing only the key-value pairs where the value satisfies a predicate `f`.
    ///
    /// The closure `f` receives a reference `&V` to each value and returns a boolean. If the predicate
    /// returns `true`, the key-value pair is included in the output. If it returns `false`, the pair
    /// is filtered out.
    ///
    /// The closure `f` receives a reference `&V` rather than an owned value `V` because filtering does
    /// not modify or take ownership of the values. If you need to modify the values while filtering
    /// use [`KeyedSingleton::filter_map`] instead.
    ///
    /// # Example
    /// ```rust
    /// # use hydro_lang::prelude::*;
    /// # use futures::StreamExt;
    /// # tokio_test::block_on(hydro_lang::test_util::stream_transform_test(|process| {
    /// let keyed_singleton = // { 1: 2, 2: 4, 3: 1 }
    /// # process
    /// #     .source_iter(q!(vec![(1, 2), (2, 4), (3, 1)]))
    /// #     .into_keyed()
    /// #     .first();
    /// keyed_singleton.filter(q!(|&v| v > 1))
    /// #   .entries()
    /// # }, |mut stream| async move {
    /// // { 1: 2, 2: 4 }
    /// # let mut results = Vec::new();
    /// # for _ in 0..2 {
    /// #     results.push(stream.next().await.unwrap());
    /// # }
    /// # results.sort();
    /// # assert_eq!(results, vec![(1, 2), (2, 4)]);
    /// # }));
    /// ```
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

    /// An operator that both filters and maps values. It yields only the key-value pairs where
    /// the supplied closure `f` returns `Some(value)`.
    ///
    /// The closure `f` receives each value `V` and returns `Option<U>`. If the closure returns
    /// `Some(new_value)`, the key-value pair `(key, new_value)` is included in the output.
    /// If it returns `None`, the key-value pair is filtered out.
    ///
    /// # Example
    /// ```rust
    /// # use hydro_lang::prelude::*;
    /// # use futures::StreamExt;
    /// # tokio_test::block_on(hydro_lang::test_util::stream_transform_test(|process| {
    /// let keyed_singleton = // { 1: "42", 2: "hello", 3: "100" }
    /// # process
    /// #     .source_iter(q!(vec![(1, "42"), (2, "hello"), (3, "100")]))
    /// #     .into_keyed()
    /// #     .first();
    /// keyed_singleton.filter_map(q!(|s| s.parse::<i32>().ok()))
    /// #   .entries()
    /// # }, |mut stream| async move {
    /// // { 1: 42, 3: 100 }
    /// # let mut results = Vec::new();
    /// # for _ in 0..2 {
    /// #     results.push(stream.next().await.unwrap());
    /// # }
    /// # results.sort();
    /// # assert_eq!(results, vec![(1, 42), (3, 100)]);
    /// # }));
    /// ```
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

    /// Gets the number of keys in the keyed singleton.
    ///
    /// The output singleton will be unbounded if the input is [`Unbounded`] or [`BoundedValue`],
    /// since keys may be added / removed over time. When the set of keys changes, the count will
    /// be asynchronously updated.
    ///
    /// # Example
    /// ```rust
    /// # use hydro_lang::prelude::*;
    /// # use futures::StreamExt;
    /// # tokio_test::block_on(hydro_lang::test_util::stream_transform_test(|process| {
    /// # let tick = process.tick();
    /// let keyed_singleton = // { 1: "a", 2: "b", 3: "c" }
    /// # process
    /// #     .source_iter(q!(vec![(1, "a"), (2, "b"), (3, "c")]))
    /// #     .into_keyed()
    /// #     .batch(&tick, nondet!(/** test */))
    /// #     .first();
    /// keyed_singleton.key_count()
    /// # .all_ticks()
    /// # }, |mut stream| async move {
    /// // 3
    /// # assert_eq!(stream.next().await.unwrap(), 3);
    /// # }));
    /// ```
    pub fn key_count(self) -> Singleton<usize, L, B::UnderlyingBound> {
        if L::is_top_level()
            && let Some(tick) = self.underlying.location.try_tick()
        {
            if B::ValueBound::is_bounded() {
                let me: KeyedSingleton<K, V, L, B::WithBoundedValue> = KeyedSingleton {
                    underlying: self.underlying,
                };

                me.entries().count()
            } else {
                let me: KeyedSingleton<K, V, L, B::WithUnboundedValue> = KeyedSingleton {
                    underlying: self.underlying,
                };

                let out = key_count_inside_tick(
                    me.snapshot(&tick, nondet!(/** eventually stabilizes */)),
                )
                .latest();
                Singleton::new(out.location, out.ir_node.into_inner())
            }
        } else {
            self.underlying.count()
        }
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
    ///     .first();
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
    ///     .first();
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
    pub fn get_many_if_present<O2: Ordering, R2: Retries, V2>(
        self,
        requests: KeyedStream<K, V2, Tick<L>, Bounded, O2, R2>,
    ) -> KeyedStream<K, (V, V2), Tick<L>, Bounded, NoOrder, R2> {
        self.entries()
            .weaker_retries::<R2>()
            .join(requests.entries())
            .into_keyed()
    }

    /// For each entry in `self`, looks up the entry in the `from` with a key that matches the
    /// **value** of the entry in `self`. The output is a keyed singleton with tuple values
    /// containing the value from `self` and an option of the value from `from`. If the key is not
    /// present in `from`, the option will be [`None`].
    ///
    /// # Example
    /// ```rust
    /// # use hydro_lang::prelude::*;
    /// # use futures::StreamExt;
    /// # tokio_test::block_on(hydro_lang::test_util::stream_transform_test(|process| {
    /// # let tick = process.tick();
    /// let requests = // { 1: 10, 2: 20 }
    /// # process
    /// #     .source_iter(q!(vec![(1, 10), (2, 20)]))
    /// #     .into_keyed()
    /// #     .batch(&tick, nondet!(/** test */))
    /// #     .first();
    /// let other_data = // { 10: 100, 11: 101 }
    /// # process
    /// #     .source_iter(q!(vec![(10, 100), (11, 101)]))
    /// #     .into_keyed()
    /// #     .batch(&tick, nondet!(/** test */))
    /// #     .first();
    /// requests.get_from(other_data)
    /// # .entries().all_ticks()
    /// # }, |mut stream| async move {
    /// // { 1: (10, Some(100)), 2: (20, None) }
    /// # let mut results = vec![];
    /// # for _ in 0..2 {
    /// #     results.push(stream.next().await.unwrap());
    /// # }
    /// # results.sort();
    /// # assert_eq!(results, vec![(1, (10, Some(100))), (2, (20, None))]);
    /// # }));
    /// ```
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
    L: Location<'a>,
{
    /// Shifts this keyed singleton into an atomic context, which guarantees that any downstream logic
    /// will all be executed synchronously before any outputs are yielded (in [`KeyedSingleton::end_atomic`]).
    ///
    /// This is useful to enforce local consistency constraints, such as ensuring that a write is
    /// processed before an acknowledgement is emitted. Entering an atomic section requires a [`Tick`]
    /// argument that declares where the keyed singleton will be atomically processed. Batching a
    /// keyed singleton into the _same_ [`Tick`] will preserve the synchronous execution, while
    /// batching into a different [`Tick`] will introduce asynchrony.
    pub fn atomic(self, tick: &Tick<L>) -> KeyedSingleton<K, V, Atomic<L>, B> {
        KeyedSingleton {
            underlying: self.underlying.atomic(tick),
        }
    }
}

impl<'a, K, V, L, B: KeyedSingletonBound> KeyedSingleton<K, V, Atomic<L>, B>
where
    L: Location<'a> + NoTick,
{
    /// Yields the elements of this keyed singleton back into a top-level, asynchronous execution context.
    /// See [`KeyedSingleton::atomic`] for more details.
    pub fn end_atomic(self) -> KeyedSingleton<K, V, L, B> {
        KeyedSingleton {
            underlying: self.underlying.end_atomic(),
        }
    }
}

impl<'a, K, V, L: Location<'a>> KeyedSingleton<K, V, Tick<L>, Bounded> {
    /// Asynchronously yields this keyed singleton outside the tick, which will
    /// be asynchronously updated with the latest set of entries inside the tick.
    ///
    /// This converts a bounded value _inside_ a tick into an asynchronous value outside the
    /// tick that tracks the inner value. This is useful for getting the value as of the
    /// "most recent" tick, but note that updates are propagated asynchronously outside the tick.
    ///
    /// The entire set of entries are propagated on each tick, which means that if a tick
    /// does not have a key "XYZ" that was present in the previous tick, the entry for "XYZ" will
    /// also be removed from the output.
    ///
    /// # Example
    /// ```rust
    /// # use hydro_lang::prelude::*;
    /// # use futures::StreamExt;
    /// # tokio_test::block_on(hydro_lang::test_util::stream_transform_test(|process| {
    /// let tick = process.tick();
    /// # // ticks are lazy by default, forces the second tick to run
    /// # tick.spin_batch(q!(1)).all_ticks().for_each(q!(|_| {}));
    /// # let batch_first_tick = process
    /// #   .source_iter(q!(vec![(1, 2), (2, 3)]))
    /// #   .batch(&tick, nondet!(/** test */))
    /// #   .into_keyed();
    /// # let batch_second_tick = process
    /// #   .source_iter(q!(vec![(2, 4), (3, 5)]))
    /// #   .batch(&tick, nondet!(/** test */))
    /// #   .into_keyed()
    /// #   .defer_tick(); // appears on the second tick
    /// # let input_batch = batch_first_tick.chain(batch_second_tick).first();
    /// input_batch // first tick: { 1: 2, 2: 3 }, second tick: { 2: 4, 3: 5 }
    ///     .latest()
    /// # .snapshot(&tick, nondet!(/** test */))
    /// # .entries()
    /// # .all_ticks()
    /// # }, |mut stream| async move {
    /// // asynchronously changes from { 1: 2, 2: 3 } ~> { 2: 4, 3: 5 }
    /// # for w in vec![(1, 2), (2, 3), (2, 4), (3, 5)] {
    /// #     assert_eq!(stream.next().await.unwrap(), w);
    /// # }
    /// # }));
    /// ```
    pub fn latest(self) -> KeyedSingleton<K, V, L, Unbounded> {
        KeyedSingleton {
            underlying: self.underlying.all_ticks(),
        }
    }

    /// Synchronously yields this keyed singleton outside the tick as an unbounded keyed singleton,
    /// which will be updated with the latest set of entries inside the tick.
    ///
    /// Unlike [`KeyedSingleton::latest`], this preserves synchronous execution, as the output
    /// keyed singleton is emitted in an [`Atomic`] context that will process elements synchronously
    /// with the input keyed singleton's [`Tick`] context.
    pub fn latest_atomic(self) -> KeyedSingleton<K, V, Atomic<L>, Unbounded> {
        KeyedSingleton {
            underlying: self.underlying.all_ticks_atomic(),
        }
    }

    #[expect(missing_docs, reason = "TODO")]
    pub fn defer_tick(self) -> KeyedSingleton<K, V, Tick<L>, Bounded> {
        KeyedSingleton {
            underlying: self.underlying.defer_tick(),
        }
    }
}

impl<'a, K, V, L, B: KeyedSingletonBound<ValueBound = Unbounded>> KeyedSingleton<K, V, L, B>
where
    L: Location<'a>,
{
    /// Returns a keyed singleton with a snapshot of each key-value entry at a non-deterministic
    /// point in time.
    ///
    /// # Non-Determinism
    /// Because this picks a snapshot of each entry, which is continuously changing, each output has a
    /// non-deterministic set of entries since each snapshot can be at an arbitrary point in time.
    pub fn snapshot(
        self,
        tick: &Tick<L>,
        nondet: NonDet,
    ) -> KeyedSingleton<K, V, Tick<L>, Bounded> {
        KeyedSingleton {
            underlying: self.underlying.batch(tick, nondet),
        }
    }
}

impl<'a, K, V, L, B: KeyedSingletonBound<ValueBound = Unbounded>> KeyedSingleton<K, V, Atomic<L>, B>
where
    L: Location<'a> + NoTick,
{
    /// Returns a keyed singleton with a snapshot of each key-value entry, consistent with the
    /// state of the keyed singleton being atomically processed.
    ///
    /// # Non-Determinism
    /// Because this picks a snapshot of each entry, which is continuously changing, each output has a
    /// non-deterministic set of entries since each snapshot can be at an arbitrary point in time.
    pub fn snapshot_atomic(self, nondet: NonDet) -> KeyedSingleton<K, V, Tick<L>, Bounded> {
        KeyedSingleton {
            underlying: self.underlying.batch_atomic(nondet),
        }
    }
}

impl<'a, K, V, L, B: KeyedSingletonBound<ValueBound = Bounded>> KeyedSingleton<K, V, L, B>
where
    L: Location<'a> + NoTick,
{
    /// Returns a keyed singleton with entries consisting of _new_ key-value pairs that have
    /// arrived since the previous batch was released.
    ///
    /// Currently, there is no `all_ticks` dual on [`KeyedSingleton`], instead you may want to use
    /// [`KeyedSingleton::into_keyed_stream`] then yield with [`KeyedStream::all_ticks`].
    ///
    /// # Non-Determinism
    /// Because this picks a batch of asynchronously added entries, each output keyed singleton
    /// has a non-deterministic set of key-value pairs.
    pub fn batch(self, tick: &Tick<L>, nondet: NonDet) -> KeyedSingleton<K, V, Tick<L>, Bounded> {
        self.atomic(tick).batch_atomic(nondet)
    }
}

impl<'a, K, V, L, B: KeyedSingletonBound<ValueBound = Bounded>> KeyedSingleton<K, V, Atomic<L>, B>
where
    L: Location<'a> + NoTick,
{
    /// Returns a keyed singleton with entries consisting of _new_ key-value pairs that are being
    /// atomically processed.
    ///
    /// Currently, there is no dual to asynchronously yield back outside the tick, instead you
    /// should use [`KeyedSingleton::into_keyed_stream`] and yield a [`KeyedStream`].
    ///
    /// # Non-Determinism
    /// Because this picks a batch of asynchronously added entries, each output keyed singleton
    /// has a non-deterministic set of key-value pairs.
    pub fn batch_atomic(self, nondet: NonDet) -> KeyedSingleton<K, V, Tick<L>, Bounded> {
        KeyedSingleton {
            underlying: self.underlying.batch_atomic(nondet),
        }
    }
}

#[cfg(test)]
mod tests {
    use futures::{SinkExt, StreamExt};
    use hydro_deploy::Deployment;
    use stageleft::q;

    use crate::compile::builder::FlowBuilder;
    use crate::location::Location;
    use crate::nondet::nondet;

    #[tokio::test]
    async fn key_count_bounded_value() {
        let mut deployment = Deployment::new();

        let flow = FlowBuilder::new();
        let node = flow.process::<()>();
        let external = flow.external::<()>();

        let (input_port, input) = node.source_external_bincode(&external);
        let out = input
            .into_keyed()
            .first()
            .key_count()
            .sample_eager(nondet!(/** test */))
            .send_bincode_external(&external);

        let nodes = flow
            .with_process(&node, deployment.Localhost())
            .with_external(&external, deployment.Localhost())
            .deploy(&mut deployment);

        deployment.deploy().await.unwrap();

        let mut external_in = nodes.connect_sink_bincode(input_port).await;
        let mut external_out = nodes.connect_source_bincode(out).await;

        deployment.start().await.unwrap();

        assert_eq!(external_out.next().await.unwrap(), 0);

        external_in.send((1, 1)).await.unwrap();
        assert_eq!(external_out.next().await.unwrap(), 1);

        external_in.send((2, 2)).await.unwrap();
        assert_eq!(external_out.next().await.unwrap(), 2);
    }

    #[tokio::test]
    async fn key_count_unbounded_value() {
        let mut deployment = Deployment::new();

        let flow = FlowBuilder::new();
        let node = flow.process::<()>();
        let external = flow.external::<()>();

        let (input_port, input) = node.source_external_bincode(&external);
        let out = input
            .into_keyed()
            .fold(q!(|| 0), q!(|acc, _| *acc += 1))
            .key_count()
            .sample_eager(nondet!(/** test */))
            .send_bincode_external(&external);

        let nodes = flow
            .with_process(&node, deployment.Localhost())
            .with_external(&external, deployment.Localhost())
            .deploy(&mut deployment);

        deployment.deploy().await.unwrap();

        let mut external_in = nodes.connect_sink_bincode(input_port).await;
        let mut external_out = nodes.connect_source_bincode(out).await;

        deployment.start().await.unwrap();

        assert_eq!(external_out.next().await.unwrap(), 0);

        external_in.send((1, 1)).await.unwrap();
        assert_eq!(external_out.next().await.unwrap(), 1);

        external_in.send((1, 2)).await.unwrap();
        assert_eq!(external_out.next().await.unwrap(), 1);

        external_in.send((2, 2)).await.unwrap();
        assert_eq!(external_out.next().await.unwrap(), 2);

        external_in.send((1, 1)).await.unwrap();
        assert_eq!(external_out.next().await.unwrap(), 2);

        external_in.send((3, 1)).await.unwrap();
        assert_eq!(external_out.next().await.unwrap(), 3);
    }
}
