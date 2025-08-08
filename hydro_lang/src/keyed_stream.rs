use std::hash::Hash;
use std::marker::PhantomData;

use stageleft::{IntoQuotedMut, QuotedWithContext, q};

use crate::cycle::{CycleCollection, CycleComplete, ForwardRefMarker};
use crate::ir::HydroNode;
use crate::keyed_optional::KeyedOptional;
use crate::keyed_singleton::KeyedSingleton;
use crate::location::tick::NoAtomic;
use crate::location::{LocationId, NoTick, check_matching_location};
use crate::manual_expr::ManualExpr;
use crate::stream::ExactlyOnce;
use crate::{Atomic, Bounded, Location, NoOrder, Optional, Stream, Tick, TotalOrder, Unbounded};

/// Keyed Streams capture streaming elements of type `V` grouped by a key of type `K`,
/// where the order of keys is non-deterministic but the order *within* each group may
/// be deterministic.
///
/// Type Parameters:
/// - `K`: the type of the key for each group
/// - `V`: the type of the elements inside each group
/// - `Loc`: the [`Location`] where the keyed stream is materialized
/// - `Order`: tracks whether the elements within each group have deterministic order
///   ([`TotalOrder`]) or not ([`NoOrder`])
/// - `Retries`: tracks whether the elements within each group have deterministic cardinality
///   ([`ExactlyOnce`]) or may have non-deterministic retries ([`crate::stream::AtLeastOnce`])
pub struct KeyedStream<K, V, Loc, Bound, Order = TotalOrder, Retries = ExactlyOnce> {
    pub(crate) underlying: Stream<(K, V), Loc, Bound, NoOrder, Retries>,
    pub(crate) _phantom_order: PhantomData<Order>,
}

impl<'a, K, V, L, B, R> From<KeyedStream<K, V, L, B, TotalOrder, R>>
    for KeyedStream<K, V, L, B, NoOrder, R>
where
    L: Location<'a>,
{
    fn from(stream: KeyedStream<K, V, L, B, TotalOrder, R>) -> KeyedStream<K, V, L, B, NoOrder, R> {
        KeyedStream {
            underlying: stream.underlying,
            _phantom_order: Default::default(),
        }
    }
}

impl<'a, K: Clone, V: Clone, Loc: Location<'a>, Bound, Order, Retries> Clone
    for KeyedStream<K, V, Loc, Bound, Order, Retries>
{
    fn clone(&self) -> Self {
        KeyedStream {
            underlying: self.underlying.clone(),
            _phantom_order: PhantomData,
        }
    }
}

impl<'a, K, V, L, B, O, R> CycleCollection<'a, ForwardRefMarker> for KeyedStream<K, V, L, B, O, R>
where
    L: Location<'a> + NoTick,
{
    type Location = L;

    fn create_source(ident: syn::Ident, location: L) -> Self {
        Stream::create_source(ident, location).into_keyed()
    }
}

impl<'a, K, V, L, B, O, R> CycleComplete<'a, ForwardRefMarker> for KeyedStream<K, V, L, B, O, R>
where
    L: Location<'a> + NoTick,
{
    fn complete(self, ident: syn::Ident, expected_location: LocationId) {
        self.underlying.complete(ident, expected_location);
    }
}

impl<'a, K, V, L: Location<'a>, B, O, R> KeyedStream<K, V, L, B, O, R> {
    /// Explicitly "casts" the keyed stream to a type with a different ordering
    /// guarantee for each group. Useful in unsafe code where the ordering cannot be proven
    /// by the type-system.
    ///
    /// # Safety
    /// This function is used as an escape hatch, and any mistakes in the
    /// provided ordering guarantee will propagate into the guarantees
    /// for the rest of the program.
    pub unsafe fn assume_ordering<O2>(self) -> KeyedStream<K, V, L, B, O2, R> {
        KeyedStream {
            underlying: self.underlying,
            _phantom_order: PhantomData,
        }
    }

    /// Explicitly "casts" the keyed stream to a type with a different retries
    /// guarantee for each group. Useful in unsafe code where the lack of retries cannot
    /// be proven by the type-system.
    ///
    /// # Safety
    /// This function is used as an escape hatch, and any mistakes in the
    /// provided retries guarantee will propagate into the guarantees
    /// for the rest of the program.
    pub unsafe fn assume_retries<R2>(self) -> KeyedStream<K, V, L, B, O, R2> {
        KeyedStream {
            underlying: unsafe { self.underlying.assume_retries::<R2>() },
            _phantom_order: PhantomData,
        }
    }

    /// Flattens the keyed stream into a single stream of key-value pairs, with non-deterministic
    /// element ordering.
    ///
    /// # Example
    /// ```rust
    /// # use hydro_lang::*;
    /// # use futures::StreamExt;
    /// # tokio_test::block_on(test_util::stream_transform_test(|process| {
    /// process
    ///     .source_iter(q!(vec![(1, 2), (1, 3), (2, 4)]))
    ///     .into_keyed()
    ///     .entries()
    /// # }, |mut stream| async move {
    /// // (1, 2), (1, 3), (2, 4) in any order
    /// # for w in vec![(1, 2), (1, 3), (2, 4)] {
    /// #     assert_eq!(stream.next().await.unwrap(), w);
    /// # }
    /// # }));
    /// ```
    pub fn entries(self) -> Stream<(K, V), L, B, NoOrder, R> {
        self.underlying
    }

    /// Flattens the keyed stream into a single stream of only the values, with non-deterministic
    /// element ordering.
    ///
    /// # Example
    /// ```rust
    /// # use hydro_lang::*;
    /// # use futures::StreamExt;
    /// # tokio_test::block_on(test_util::stream_transform_test(|process| {
    /// process
    ///     .source_iter(q!(vec![(1, 2), (1, 3), (2, 4)]))
    ///     .into_keyed()
    ///     .values()
    /// # }, |mut stream| async move {
    /// // 2, 3, 4 in any order
    /// # for w in vec![2, 3, 4] {
    /// #     assert_eq!(stream.next().await.unwrap(), w);
    /// # }
    /// # }));
    /// ```
    pub fn values(self) -> Stream<V, L, B, NoOrder, R> {
        self.underlying.map(q!(|(_, v)| v))
    }

    /// Transforms each value by invoking `f` on each element, with keys staying the same
    /// after transformation. If you need access to the key, see [`KeyedStream::map_with_key`].
    ///
    /// If you do not want to modify the stream and instead only want to view
    /// each item use [`KeyedStream::inspect`] instead.
    ///
    /// # Example
    /// ```rust
    /// # use hydro_lang::*;
    /// # use futures::StreamExt;
    /// # tokio_test::block_on(test_util::stream_transform_test(|process| {
    /// process
    ///     .source_iter(q!(vec![(1, 2), (1, 3), (2, 4)]))
    ///     .into_keyed()
    ///     .map(q!(|v| v + 1))
    /// #   .entries()
    /// # }, |mut stream| async move {
    /// // { 1: [3, 4], 2: [5] }
    /// # for w in vec![(1, 3), (1, 4), (2, 5)] {
    /// #     assert_eq!(stream.next().await.unwrap(), w);
    /// # }
    /// # }));
    /// ```
    pub fn map<U, F>(self, f: impl IntoQuotedMut<'a, F, L> + Copy) -> KeyedStream<K, U, L, B, O, R>
    where
        F: Fn(V) -> U + 'a,
    {
        let f: ManualExpr<F, _> = ManualExpr::new(move |ctx: &L| f.splice_fn1_ctx(ctx));
        KeyedStream {
            underlying: self.underlying.map(q!({
                let orig = f;
                move |(k, v)| (k, orig(v))
            })),
            _phantom_order: Default::default(),
        }
    }

    /// Transforms each value by invoking `f` on each key-value pair. The resulting values are **not**
    /// re-grouped even they are tuples; instead they will be grouped under the original key.
    ///
    /// If you do not want to modify the stream and instead only want to view
    /// each item use [`KeyedStream::inspect_with_key`] instead.
    ///
    /// # Example
    /// ```rust
    /// # use hydro_lang::*;
    /// # use futures::StreamExt;
    /// # tokio_test::block_on(test_util::stream_transform_test(|process| {
    /// process
    ///     .source_iter(q!(vec![(1, 2), (1, 3), (2, 4)]))
    ///     .into_keyed()
    ///     .map_with_key(q!(|(k, v)| k + v))
    /// #   .entries()
    /// # }, |mut stream| async move {
    /// // { 1: [3, 4], 2: [6] }
    /// # for w in vec![(1, 3), (1, 4), (2, 6)] {
    /// #     assert_eq!(stream.next().await.unwrap(), w);
    /// # }
    /// # }));
    /// ```
    pub fn map_with_key<U, F>(
        self,
        f: impl IntoQuotedMut<'a, F, L> + Copy,
    ) -> KeyedStream<K, U, L, B, O, R>
    where
        F: Fn((K, V)) -> U + 'a,
        K: Clone,
    {
        let f: ManualExpr<F, _> = ManualExpr::new(move |ctx: &L| f.splice_fn1_ctx(ctx));
        KeyedStream {
            underlying: self.underlying.map(q!({
                let orig = f;
                move |(k, v)| {
                    let out = orig((k.clone(), v));
                    (k, out)
                }
            })),
            _phantom_order: Default::default(),
        }
    }

    /// An operator that both filters and maps each value, with keys staying the same.
    /// It yields only the items for which the supplied closure `f` returns `Some(value)`.
    /// If you need access to the key, see [`KeyedStream::filter_map_with_key`].
    ///
    /// # Example
    /// ```rust
    /// # use hydro_lang::*;
    /// # use futures::StreamExt;
    /// # tokio_test::block_on(test_util::stream_transform_test(|process| {
    /// process
    ///     .source_iter(q!(vec![(1, "2"), (1, "hello"), (2, "4")]))
    ///     .into_keyed()
    ///     .filter_map(q!(|s| s.parse::<usize>().ok()))
    /// #   .entries()
    /// # }, |mut stream| async move {
    /// // { 1: [2], 2: [4] }
    /// # for w in vec![(1, 2), (2, 4)] {
    /// #     assert_eq!(stream.next().await.unwrap(), w);
    /// # }
    /// # }));
    /// ```
    pub fn filter_map<U, F>(
        self,
        f: impl IntoQuotedMut<'a, F, L> + Copy,
    ) -> KeyedStream<K, U, L, B, O, R>
    where
        F: Fn(V) -> Option<U> + 'a,
    {
        let f: ManualExpr<F, _> = ManualExpr::new(move |ctx: &L| f.splice_fn1_ctx(ctx));
        KeyedStream {
            underlying: self.underlying.filter_map(q!({
                let orig = f;
                move |(k, v)| orig(v).map(|o| (k, o))
            })),
            _phantom_order: Default::default(),
        }
    }

    /// An operator that both filters and maps each key-value pair. The resulting values are **not**
    /// re-grouped even they are tuples; instead they will be grouped under the original key.
    /// It yields only the items for which the supplied closure `f` returns `Some(value)`.
    ///
    /// # Example
    /// ```rust
    /// # use hydro_lang::*;
    /// # use futures::StreamExt;
    /// # tokio_test::block_on(test_util::stream_transform_test(|process| {
    /// process
    ///     .source_iter(q!(vec![(1, "2"), (1, "hello"), (2, "2")]))
    ///     .into_keyed()
    ///     .filter_map_with_key(q!(|(k, s)| s.parse::<usize>().ok().filter(|v| v == &k)))
    /// #   .entries()
    /// # }, |mut stream| async move {
    /// // { 2: [2] }
    /// # for w in vec![(2, 2)] {
    /// #     assert_eq!(stream.next().await.unwrap(), w);
    /// # }
    /// # }));
    /// ```
    pub fn filter_map_with_key<U, F>(
        self,
        f: impl IntoQuotedMut<'a, F, L> + Copy,
    ) -> KeyedStream<K, U, L, B, O, R>
    where
        F: Fn((K, V)) -> Option<U> + 'a,
        K: Clone,
    {
        let f: ManualExpr<F, _> = ManualExpr::new(move |ctx: &L| f.splice_fn1_ctx(ctx));
        KeyedStream {
            underlying: self.underlying.filter_map(q!({
                let orig = f;
                move |(k, v)| {
                    let out = orig((k.clone(), v));
                    out.map(|o| (k, o))
                }
            })),
            _phantom_order: Default::default(),
        }
    }

    /// An operator which allows you to "inspect" each element of a stream without
    /// modifying it. The closure `f` is called on a reference to each value. This is
    /// mainly useful for debugging, and should not be used to generate side-effects.
    ///
    /// # Example
    /// ```rust
    /// # use hydro_lang::*;
    /// # use futures::StreamExt;
    /// # tokio_test::block_on(test_util::stream_transform_test(|process| {
    /// process
    ///     .source_iter(q!(vec![(1, 2), (1, 3), (2, 4)]))
    ///     .into_keyed()
    ///     .inspect(q!(|v| println!("{}", v)))
    /// #   .entries()
    /// # }, |mut stream| async move {
    /// # for w in vec![(1, 2), (1, 3), (2, 4)] {
    /// #     assert_eq!(stream.next().await.unwrap(), w);
    /// # }
    /// # }));
    /// ```
    pub fn inspect<F>(self, f: impl IntoQuotedMut<'a, F, L> + Copy) -> KeyedStream<K, V, L, B, O, R>
    where
        F: Fn(&V) + 'a,
    {
        let f: ManualExpr<F, _> = ManualExpr::new(move |ctx: &L| f.splice_fn1_borrow_ctx(ctx));
        KeyedStream {
            underlying: self.underlying.inspect(q!({
                let orig = f;
                move |(_k, v)| orig(v)
            })),
            _phantom_order: Default::default(),
        }
    }

    /// An operator which allows you to "inspect" each element of a stream without
    /// modifying it. The closure `f` is called on a reference to each key-value pair. This is
    /// mainly useful for debugging, and should not be used to generate side-effects.
    ///
    /// # Example
    /// ```rust
    /// # use hydro_lang::*;
    /// # use futures::StreamExt;
    /// # tokio_test::block_on(test_util::stream_transform_test(|process| {
    /// process
    ///     .source_iter(q!(vec![(1, 2), (1, 3), (2, 4)]))
    ///     .into_keyed()
    ///     .inspect(q!(|v| println!("{}", v)))
    /// #   .entries()
    /// # }, |mut stream| async move {
    /// # for w in vec![(1, 2), (1, 3), (2, 4)] {
    /// #     assert_eq!(stream.next().await.unwrap(), w);
    /// # }
    /// # }));
    /// ```
    pub fn inspect_with_key<F>(
        self,
        f: impl IntoQuotedMut<'a, F, L>,
    ) -> KeyedStream<K, V, L, B, O, R>
    where
        F: Fn(&(K, V)) + 'a,
    {
        KeyedStream {
            underlying: self.underlying.inspect(f),
            _phantom_order: Default::default(),
        }
    }
}

impl<'a, K, V, L, B> KeyedStream<K, V, L, B, TotalOrder, ExactlyOnce>
where
    K: Eq + Hash,
    L: Location<'a>,
{
    /// A special case of [`Stream::scan`] for keyd streams. For each key group the values are transformed via the `f` combinator.
    ///
    /// Unlike [`Stream::fold_keyed`] which only returns the final accumulated value, `scan` produces a new stream
    /// containing all intermediate accumulated values paired with the key. The scan operation can also terminate
    /// early by returning `None`.
    ///
    /// The function takes a mutable reference to the accumulator and the current element, and returns
    /// an `Option<U>`. If the function returns `Some(value)`, `value` is emitted to the output stream.
    /// If the function returns `None`, the stream is terminated and no more elements are processed.
    ///
    /// # Example
    /// ```rust
    /// # use hydro_lang::*;
    /// # use futures::StreamExt;
    /// # tokio_test::block_on(test_util::stream_transform_test(|process| {
    /// process
    ///     .source_iter(q!(vec![(0, 1), (0, 2), (1, 3), (1, 4)]))
    ///     .into_keyed()
    ///     .scan(
    ///         q!(|| 0),
    ///         q!(|acc, x| {
    ///             *acc += x;
    ///             Some(*acc)
    ///         }),
    ///     )
    /// #   .entries()
    /// # }, |mut stream| async move {
    /// // Output: { 0: [1, 3], 1: [3, 7] }
    /// # for w in vec![(0, 1), (0, 3), (1, 3), (1, 7)] {
    /// #     assert_eq!(stream.next().await.unwrap(), w);
    /// # }
    /// # }));
    /// ```
    pub fn scan<A, U, I, F>(
        self,
        init: impl IntoQuotedMut<'a, I, L> + Copy,
        f: impl IntoQuotedMut<'a, F, L> + Copy,
    ) -> KeyedStream<K, U, L, B, TotalOrder, ExactlyOnce>
    where
        K: Clone,
        I: Fn() -> A + 'a,
        F: Fn(&mut A, V) -> Option<U> + 'a,
    {
        KeyedStream {
            underlying: unsafe {
                // SAFETY: keyed scan does not rely on order of keys
                self.underlying
                    .assume_ordering::<TotalOrder>()
                    .scan_keyed(init, f)
                    .into()
            },
            _phantom_order: Default::default(),
        }
    }

    /// A variant of [`Stream::fold`], intended for keyed streams. The aggregation is executed in-order across the values
    /// in each group. But the aggregation function returns a boolean, which when true indicates that the aggregated
    /// result is complete and can be released to downstream computation. Unlike [`Stream::fold_keyed`], this means that
    /// even if the input stream is [`crate::Unbounded`], the outputs of the fold can be processed like normal stream elements.
    ///
    /// # Example
    /// ```rust
    /// # use hydro_lang::*;
    /// # use futures::StreamExt;
    /// # tokio_test::block_on(test_util::stream_transform_test(|process| {
    /// process
    ///     .source_iter(q!(vec![(0, 2), (0, 3), (1, 3), (1, 6)]))
    ///     .into_keyed()
    ///     .fold_early_stop(
    ///         q!(|| 0),
    ///         q!(|acc, x| {
    ///             *acc += x;
    ///             x % 2 == 0
    ///         }),
    ///     )
    /// #   .entries()
    /// # }, |mut stream| async move {
    /// // Output: { 0: 2, 1: 9 }
    /// # for w in vec![(0, 2), (1, 9)] {
    /// #     assert_eq!(stream.next().await.unwrap(), w);
    /// # }
    /// # }));
    /// ```
    pub fn fold_early_stop<A, I, F>(
        self,
        init: impl IntoQuotedMut<'a, I, L> + Copy,
        f: impl IntoQuotedMut<'a, F, L> + Copy,
    ) -> KeyedStream<K, A, L, B, TotalOrder, ExactlyOnce>
    where
        K: Clone,
        I: Fn() -> A + 'a,
        F: Fn(&mut A, V) -> bool + 'a,
    {
        KeyedStream {
            underlying: unsafe {
                // SAFETY: keyed scan does not rely on order of keys
                self.underlying
                    .assume_ordering::<TotalOrder>()
                    .fold_keyed_early_stop(init, f)
                    .into()
            },
            _phantom_order: Default::default(),
        }
    }

    /// Like [`Stream::fold`], aggregates the values in each group via the `comb` closure.
    ///
    /// Each group must have a [`TotalOrder`] guarantee, which means that the `comb` closure is allowed
    /// to depend on the order of elements in the group.
    ///
    /// If the input and output value types are the same and do not require initialization then use
    /// [`KeyedStream::reduce`].
    ///
    /// # Example
    /// ```rust
    /// # use hydro_lang::*;
    /// # use futures::StreamExt;
    /// # tokio_test::block_on(test_util::stream_transform_test(|process| {
    /// let tick = process.tick();
    /// let numbers = process
    ///     .source_iter(q!(vec![(1, 2), (2, 3), (1, 3), (2, 4)]))
    ///     .into_keyed();
    /// let batch = unsafe { numbers.batch(&tick) };
    /// batch
    ///     .fold(q!(|| 0), q!(|acc, x| *acc += x))
    ///     .entries()
    ///     .all_ticks()
    /// # }, |mut stream| async move {
    /// // (1, 5), (2, 7)
    /// # assert_eq!(stream.next().await.unwrap(), (1, 5));
    /// # assert_eq!(stream.next().await.unwrap(), (2, 7));
    /// # }));
    /// ```
    pub fn fold<A, I: Fn() -> A + 'a, F: Fn(&mut A, V)>(
        self,
        init: impl IntoQuotedMut<'a, I, L>,
        comb: impl IntoQuotedMut<'a, F, L>,
    ) -> KeyedSingleton<K, A, L, B> {
        let init = init.splice_fn0_ctx(&self.underlying.location).into();
        let comb = comb
            .splice_fn2_borrow_mut_ctx(&self.underlying.location)
            .into();

        let out_ir = HydroNode::FoldKeyed {
            init,
            acc: comb,
            input: Box::new(self.underlying.ir_node.into_inner()),
            metadata: self.underlying.location.new_node_metadata::<(K, V)>(),
        };

        KeyedSingleton {
            underlying: Stream::new(self.underlying.location, out_ir),
        }
    }

    /// Like [`Stream::reduce`], aggregates the values in each group via the `comb` closure.
    ///
    /// Each group must have a [`TotalOrder`] guarantee, which means that the `comb` closure is allowed
    /// to depend on the order of elements in the stream.
    ///
    /// If you need the accumulated value to have a different type than the input, use [`KeyedStream::fold`].
    ///
    /// # Example
    /// ```rust
    /// # use hydro_lang::*;
    /// # use futures::StreamExt;
    /// # tokio_test::block_on(test_util::stream_transform_test(|process| {
    /// let tick = process.tick();
    /// let numbers = process
    ///     .source_iter(q!(vec![(1, 2), (2, 3), (1, 3), (2, 4)]))
    ///     .into_keyed();
    /// let batch = unsafe { numbers.batch(&tick) };
    /// batch.reduce(q!(|acc, x| *acc += x)).entries().all_ticks()
    /// # }, |mut stream| async move {
    /// // (1, 5), (2, 7)
    /// # assert_eq!(stream.next().await.unwrap(), (1, 5));
    /// # assert_eq!(stream.next().await.unwrap(), (2, 7));
    /// # }));
    /// ```
    pub fn reduce<F: Fn(&mut V, V) + 'a>(
        self,
        comb: impl IntoQuotedMut<'a, F, L>,
    ) -> KeyedOptional<K, V, L, B> {
        let f = comb
            .splice_fn2_borrow_mut_ctx(&self.underlying.location)
            .into();

        let out_ir = HydroNode::ReduceKeyed {
            f,
            input: Box::new(self.underlying.ir_node.into_inner()),
            metadata: self.underlying.location.new_node_metadata::<(K, V)>(),
        };

        KeyedOptional {
            underlying: Stream::new(self.underlying.location, out_ir),
        }
    }

    /// A special case of [`KeyedStream::reduce`] where tuples with keys less than the watermark are automatically deleted.
    ///
    /// Each group must have a [`TotalOrder`] guarantee, which means that the `comb` closure is allowed
    /// to depend on the order of elements in the stream.
    ///
    /// # Example
    /// ```rust
    /// # use hydro_lang::*;
    /// # use futures::StreamExt;
    /// # tokio_test::block_on(test_util::stream_transform_test(|process| {
    /// let tick = process.tick();
    /// let watermark = tick.singleton(q!(1));
    /// let numbers = process
    ///     .source_iter(q!([(0, 100), (1, 101), (2, 102), (2, 102)]))
    ///     .into_keyed();
    /// let batch = unsafe { numbers.batch(&tick) };
    /// batch
    ///     .reduce_watermark(watermark, q!(|acc, x| *acc += x))
    ///     .entries()
    ///     .all_ticks()
    /// # }, |mut stream| async move {
    /// // (2, 204)
    /// # assert_eq!(stream.next().await.unwrap(), (2, 204));
    /// # }));
    /// ```
    pub fn reduce_watermark<O, F>(
        self,
        other: impl Into<Optional<O, Tick<L::Root>, Bounded>>,
        comb: impl IntoQuotedMut<'a, F, L>,
    ) -> KeyedOptional<K, V, L, B>
    where
        O: Clone,
        F: Fn(&mut V, V) + 'a,
    {
        let other: Optional<O, Tick<L::Root>, Bounded> = other.into();
        check_matching_location(&self.underlying.location.root(), other.location.outer());
        let f = comb
            .splice_fn2_borrow_mut_ctx(&self.underlying.location)
            .into();

        let out_ir = Stream::new(
            self.underlying.location.clone(),
            HydroNode::ReduceKeyedWatermark {
                f,
                input: Box::new(self.underlying.ir_node.into_inner()),
                watermark: Box::new(other.ir_node.into_inner()),
                metadata: self.underlying.location.new_node_metadata::<(K, V)>(),
            },
        );

        KeyedOptional { underlying: out_ir }
    }
}

impl<'a, K, V, L, B, O> KeyedStream<K, V, L, B, O, ExactlyOnce>
where
    K: Eq + Hash,
    L: Location<'a>,
{
    /// Like [`Stream::fold_commutative`], aggregates the values in each group via the `comb` closure.
    ///
    /// The `comb` closure must be **commutative**, as the order of input items is not guaranteed.
    ///
    /// If the input and output value types are the same and do not require initialization then use
    /// [`KeyedStream::reduce_commutative`].
    ///
    /// # Example
    /// ```rust
    /// # use hydro_lang::*;
    /// # use futures::StreamExt;
    /// # tokio_test::block_on(test_util::stream_transform_test(|process| {
    /// let tick = process.tick();
    /// let numbers = process
    ///     .source_iter(q!(vec![(1, 2), (2, 3), (1, 3), (2, 4)]))
    ///     .into_keyed();
    /// let batch = unsafe { numbers.batch(&tick) };
    /// batch
    ///     .fold_commutative(q!(|| 0), q!(|acc, x| *acc += x))
    ///     .entries()
    ///     .all_ticks()
    /// # }, |mut stream| async move {
    /// // (1, 5), (2, 7)
    /// # assert_eq!(stream.next().await.unwrap(), (1, 5));
    /// # assert_eq!(stream.next().await.unwrap(), (2, 7));
    /// # }));
    /// ```
    pub fn fold_commutative<A, I: Fn() -> A + 'a, F: Fn(&mut A, V)>(
        self,
        init: impl IntoQuotedMut<'a, I, L>,
        comb: impl IntoQuotedMut<'a, F, L>,
    ) -> KeyedSingleton<K, A, L, B> {
        unsafe {
            // SAFETY: the combinator function is commutative
            self.assume_ordering::<TotalOrder>().fold(init, comb)
        }
    }

    /// Like [`Stream::reduce_commutative`], aggregates the values in each group via the `comb` closure.
    ///
    /// The `comb` closure must be **commutative**, as the order of input items is not guaranteed.
    ///
    /// If you need the accumulated value to have a different type than the input, use [`KeyedStream::fold_commutative`].
    ///
    /// # Example
    /// ```rust
    /// # use hydro_lang::*;
    /// # use futures::StreamExt;
    /// # tokio_test::block_on(test_util::stream_transform_test(|process| {
    /// let tick = process.tick();
    /// let numbers = process
    ///     .source_iter(q!(vec![(1, 2), (2, 3), (1, 3), (2, 4)]))
    ///     .into_keyed();
    /// let batch = unsafe { numbers.batch(&tick) };
    /// batch
    ///     .reduce_commutative(q!(|acc, x| *acc += x))
    ///     .entries()
    ///     .all_ticks()
    /// # }, |mut stream| async move {
    /// // (1, 5), (2, 7)
    /// # assert_eq!(stream.next().await.unwrap(), (1, 5));
    /// # assert_eq!(stream.next().await.unwrap(), (2, 7));
    /// # }));
    /// ```
    pub fn reduce_commutative<F: Fn(&mut V, V) + 'a>(
        self,
        comb: impl IntoQuotedMut<'a, F, L>,
    ) -> KeyedOptional<K, V, L, B> {
        unsafe {
            // SAFETY: the combinator function is commutative
            self.assume_ordering::<TotalOrder>().reduce(comb)
        }
    }

    /// A special case of [`KeyedStream::reduce_commutative`] where tuples with keys less than the watermark are automatically deleted.
    ///
    /// The `comb` closure must be **commutative**, as the order of input items is not guaranteed.
    ///
    /// # Example
    /// ```rust
    /// # use hydro_lang::*;
    /// # use futures::StreamExt;
    /// # tokio_test::block_on(test_util::stream_transform_test(|process| {
    /// let tick = process.tick();
    /// let watermark = tick.singleton(q!(1));
    /// let numbers = process
    ///     .source_iter(q!([(0, 100), (1, 101), (2, 102), (2, 102)]))
    ///     .into_keyed();
    /// let batch = unsafe { numbers.batch(&tick) };
    /// batch
    ///     .reduce_watermark_commutative(watermark, q!(|acc, x| *acc += x))
    ///     .entries()
    ///     .all_ticks()
    /// # }, |mut stream| async move {
    /// // (2, 204)
    /// # assert_eq!(stream.next().await.unwrap(), (2, 204));
    /// # }));
    /// ```
    pub fn reduce_watermark_commutative<O2, F>(
        self,
        other: impl Into<Optional<O2, Tick<L::Root>, Bounded>>,
        comb: impl IntoQuotedMut<'a, F, L>,
    ) -> KeyedOptional<K, V, L, B>
    where
        O2: Clone,
        F: Fn(&mut V, V) + 'a,
    {
        unsafe {
            // SAFETY: the combinator function is commutative
            self.assume_ordering::<TotalOrder>()
        }
        .reduce_watermark(other, comb)
    }
}

impl<'a, K, V, L, B, R> KeyedStream<K, V, L, B, TotalOrder, R>
where
    K: Eq + Hash,
    L: Location<'a>,
{
    /// Like [`Stream::fold_idempotent`], aggregates the values in each group via the `comb` closure.
    ///
    /// The `comb` closure must be **idempotent** as there may be non-deterministic duplicates.
    ///
    /// If the input and output value types are the same and do not require initialization then use
    /// [`KeyedStream::reduce_idempotent`].
    ///
    /// # Example
    /// ```rust
    /// # use hydro_lang::*;
    /// # use futures::StreamExt;
    /// # tokio_test::block_on(test_util::stream_transform_test(|process| {
    /// let tick = process.tick();
    /// let numbers = process
    ///     .source_iter(q!(vec![(1, false), (2, true), (1, false), (2, false)]))
    ///     .into_keyed();
    /// let batch = unsafe { numbers.batch(&tick) };
    /// batch
    ///     .fold_idempotent(q!(|| false), q!(|acc, x| *acc |= x))
    ///     .entries()
    ///     .all_ticks()
    /// # }, |mut stream| async move {
    /// // (1, false), (2, true)
    /// # assert_eq!(stream.next().await.unwrap(), (1, false));
    /// # assert_eq!(stream.next().await.unwrap(), (2, true));
    /// # }));
    /// ```
    pub fn fold_idempotent<A, I: Fn() -> A + 'a, F: Fn(&mut A, V)>(
        self,
        init: impl IntoQuotedMut<'a, I, L>,
        comb: impl IntoQuotedMut<'a, F, L>,
    ) -> KeyedSingleton<K, A, L, B> {
        unsafe {
            // SAFETY: the combinator function is idempotent
            self.assume_retries::<ExactlyOnce>().fold(init, comb)
        }
    }

    /// Like [`Stream::reduce_idempotent`], aggregates the values in each group via the `comb` closure.
    ///
    /// The `comb` closure must be **idempotent**, as there may be non-deterministic duplicates.
    ///
    /// If you need the accumulated value to have a different type than the input, use [`KeyedStream::fold_idempotent`].
    ///
    /// # Example
    /// ```rust
    /// # use hydro_lang::*;
    /// # use futures::StreamExt;
    /// # tokio_test::block_on(test_util::stream_transform_test(|process| {
    /// let tick = process.tick();
    /// let numbers = process
    ///     .source_iter(q!(vec![(1, false), (2, true), (1, false), (2, false)]))
    ///     .into_keyed();
    /// let batch = unsafe { numbers.batch(&tick) };
    /// batch
    ///     .reduce_idempotent(q!(|acc, x| *acc |= x))
    ///     .entries()
    ///     .all_ticks()
    /// # }, |mut stream| async move {
    /// // (1, false), (2, true)
    /// # assert_eq!(stream.next().await.unwrap(), (1, false));
    /// # assert_eq!(stream.next().await.unwrap(), (2, true));
    /// # }));
    /// ```
    pub fn reduce_idempotent<F: Fn(&mut V, V) + 'a>(
        self,
        comb: impl IntoQuotedMut<'a, F, L>,
    ) -> KeyedOptional<K, V, L, B> {
        unsafe {
            // SAFETY: the combinator function is idempotent
            self.assume_retries::<ExactlyOnce>().reduce(comb)
        }
    }

    /// A special case of [`KeyedStream::reduce_idempotent`] where tuples with keys less than the watermark are automatically deleted.
    ///
    /// The `comb` closure must be **idempotent**, as there may be non-deterministic duplicates.
    ///
    /// # Example
    /// ```rust
    /// # use hydro_lang::*;
    /// # use futures::StreamExt;
    /// # tokio_test::block_on(test_util::stream_transform_test(|process| {
    /// let tick = process.tick();
    /// let watermark = tick.singleton(q!(1));
    /// let numbers = process
    ///     .source_iter(q!([(0, false), (1, false), (2, false), (2, true)]))
    ///     .into_keyed();
    /// let batch = unsafe { numbers.batch(&tick) };
    /// batch
    ///     .reduce_watermark_idempotent(watermark, q!(|acc, x| *acc |= x))
    ///     .entries()
    ///     .all_ticks()
    /// # }, |mut stream| async move {
    /// // (2, true)
    /// # assert_eq!(stream.next().await.unwrap(), (2, true));
    /// # }));
    /// ```
    pub fn reduce_watermark_idempotent<O2, F>(
        self,
        other: impl Into<Optional<O2, Tick<L::Root>, Bounded>>,
        comb: impl IntoQuotedMut<'a, F, L>,
    ) -> KeyedOptional<K, V, L, B>
    where
        O2: Clone,
        F: Fn(&mut V, V) + 'a,
    {
        unsafe {
            // SAFETY: the combinator function is idempotent
            self.assume_retries::<ExactlyOnce>()
        }
        .reduce_watermark(other, comb)
    }
}

impl<'a, K, V, L, B, O, R> KeyedStream<K, V, L, B, O, R>
where
    K: Eq + Hash,
    L: Location<'a>,
{
    /// Like [`Stream::fold_commutative_idempotent`], aggregates the values in each group via the `comb` closure.
    ///
    /// The `comb` closure must be **commutative**, as the order of input items is not guaranteed, and **idempotent**,
    /// as there may be non-deterministic duplicates.
    ///
    /// If the input and output value types are the same and do not require initialization then use
    /// [`KeyedStream::reduce_commutative_idempotent`].
    ///
    /// # Example
    /// ```rust
    /// # use hydro_lang::*;
    /// # use futures::StreamExt;
    /// # tokio_test::block_on(test_util::stream_transform_test(|process| {
    /// let tick = process.tick();
    /// let numbers = process
    ///     .source_iter(q!(vec![(1, false), (2, true), (1, false), (2, false)]))
    ///     .into_keyed();
    /// let batch = unsafe { numbers.batch(&tick) };
    /// batch
    ///     .fold_commutative_idempotent(q!(|| false), q!(|acc, x| *acc |= x))
    ///     .entries()
    ///     .all_ticks()
    /// # }, |mut stream| async move {
    /// // (1, false), (2, true)
    /// # assert_eq!(stream.next().await.unwrap(), (1, false));
    /// # assert_eq!(stream.next().await.unwrap(), (2, true));
    /// # }));
    /// ```
    pub fn fold_commutative_idempotent<A, I: Fn() -> A + 'a, F: Fn(&mut A, V)>(
        self,
        init: impl IntoQuotedMut<'a, I, L>,
        comb: impl IntoQuotedMut<'a, F, L>,
    ) -> KeyedSingleton<K, A, L, B> {
        unsafe {
            // SAFETY: the combinator function is idempotent
            self.assume_ordering::<TotalOrder>()
                .assume_retries::<ExactlyOnce>()
                .fold(init, comb)
        }
    }

    /// Like [`Stream::reduce_commutative_idempotent`], aggregates the values in each group via the `comb` closure.
    ///
    /// The `comb` closure must be **commutative**, as the order of input items is not guaranteed, and **idempotent**,
    /// as there may be non-deterministic duplicates.
    ///
    /// If you need the accumulated value to have a different type than the input, use [`KeyedStream::fold_commutative_idempotent`].
    ///
    /// # Example
    /// ```rust
    /// # use hydro_lang::*;
    /// # use futures::StreamExt;
    /// # tokio_test::block_on(test_util::stream_transform_test(|process| {
    /// let tick = process.tick();
    /// let numbers = process
    ///     .source_iter(q!(vec![(1, false), (2, true), (1, false), (2, false)]))
    ///     .into_keyed();
    /// let batch = unsafe { numbers.batch(&tick) };
    /// batch
    ///     .reduce_commutative_idempotent(q!(|acc, x| *acc |= x))
    ///     .entries()
    ///     .all_ticks()
    /// # }, |mut stream| async move {
    /// // (1, false), (2, true)
    /// # assert_eq!(stream.next().await.unwrap(), (1, false));
    /// # assert_eq!(stream.next().await.unwrap(), (2, true));
    /// # }));
    /// ```
    pub fn reduce_commutative_idempotent<F: Fn(&mut V, V) + 'a>(
        self,
        comb: impl IntoQuotedMut<'a, F, L>,
    ) -> KeyedOptional<K, V, L, B> {
        unsafe {
            // SAFETY: the combinator function is idempotent
            self.assume_ordering::<TotalOrder>()
                .assume_retries::<ExactlyOnce>()
                .reduce(comb)
        }
    }

    /// A special case of [`Stream::reduce_keyed_commutative_idempotent`] where tuples with keys less than the watermark are automatically deleted.
    ///
    /// The `comb` closure must be **commutative**, as the order of input items is not guaranteed, and **idempotent**,
    /// as there may be non-deterministic duplicates.
    ///
    /// # Example
    /// ```rust
    /// # use hydro_lang::*;
    /// # use futures::StreamExt;
    /// # tokio_test::block_on(test_util::stream_transform_test(|process| {
    /// let tick = process.tick();
    /// let watermark = tick.singleton(q!(1));
    /// let numbers = process
    ///     .source_iter(q!([(0, false), (1, false), (2, false), (2, true)]))
    ///     .into_keyed();
    /// let batch = unsafe { numbers.batch(&tick) };
    /// batch
    ///     .reduce_watermark_commutative_idempotent(watermark, q!(|acc, x| *acc |= x))
    ///     .entries()
    ///     .all_ticks()
    /// # }, |mut stream| async move {
    /// // (2, true)
    /// # assert_eq!(stream.next().await.unwrap(), (2, true));
    /// # }));
    /// ```
    pub fn reduce_watermark_commutative_idempotent<O2, F>(
        self,
        other: impl Into<Optional<O2, Tick<L::Root>, Bounded>>,
        comb: impl IntoQuotedMut<'a, F, L>,
    ) -> KeyedOptional<K, V, L, B>
    where
        O2: Clone,
        F: Fn(&mut V, V) + 'a,
    {
        unsafe {
            // SAFETY: the combinator function is commutative and idempotent
            self.assume_ordering::<TotalOrder>()
                .assume_retries::<ExactlyOnce>()
        }
        .reduce_watermark(other, comb)
    }
}

impl<'a, K, V, L, B, O, R> KeyedStream<K, V, L, B, O, R>
where
    L: Location<'a> + NoTick + NoAtomic,
{
    pub fn atomic(self, tick: &Tick<L>) -> KeyedStream<K, V, Atomic<L>, B, O, R> {
        KeyedStream {
            underlying: self.underlying.atomic(tick),
            _phantom_order: Default::default(),
        }
    }

    /// Given a tick, returns a keyed stream corresponding to a batch of elements segmented by
    /// that tick. These batches are guaranteed to be contiguous across ticks and preserve
    /// the order of the input.
    ///
    /// # Safety
    /// The batch boundaries are non-deterministic and may change across executions.
    pub unsafe fn batch(self, tick: &Tick<L>) -> KeyedStream<K, V, Tick<L>, Bounded, O, R> {
        unsafe { self.atomic(tick).batch() }
    }
}

impl<'a, K, V, L, B, O, R> KeyedStream<K, V, Atomic<L>, B, O, R>
where
    L: Location<'a> + NoTick + NoAtomic,
{
    /// Returns a keyed stream corresponding to the latest batch of elements being atomically
    /// processed. These batches are guaranteed to be contiguous across ticks and preserve
    /// the order of the input.
    ///
    /// # Safety
    /// The batch boundaries are non-deterministic and may change across executions.
    pub unsafe fn batch(self) -> KeyedStream<K, V, Tick<L>, Bounded, O, R> {
        unsafe {
            KeyedStream {
                underlying: self.underlying.tick_batch(),
                _phantom_order: Default::default(),
            }
        }
    }
}

impl<'a, K, V, L, O, R> KeyedStream<K, V, Tick<L>, Bounded, O, R>
where
    L: Location<'a>,
{
    pub fn all_ticks(self) -> KeyedStream<K, V, L, Unbounded, O, R> {
        KeyedStream {
            underlying: self.underlying.all_ticks(),
            _phantom_order: Default::default(),
        }
    }
}

#[cfg(test)]
mod tests {
    use futures::{SinkExt, StreamExt};
    use hydro_deploy::Deployment;
    use stageleft::q;

    use crate::FlowBuilder;
    use crate::location::Location;

    #[tokio::test]
    async fn reduce_watermark_filter() {
        let mut deployment = Deployment::new();

        let flow = FlowBuilder::new();
        let node = flow.process::<()>();
        let external = flow.external::<()>();

        let node_tick = node.tick();
        let watermark = node_tick.singleton(q!(1));

        let sum = unsafe {
            node.source_iter(q!([(0, 100), (1, 101), (2, 102), (2, 102)]))
                .into_keyed()
                .reduce_watermark(
                    watermark,
                    q!(|acc, v| {
                        *acc += v;
                    }),
                )
                .snapshot(&node_tick)
                .entries()
        }
        .all_ticks()
        .send_bincode_external(&external);

        let nodes = flow
            .with_process(&node, deployment.Localhost())
            .with_external(&external, deployment.Localhost())
            .deploy(&mut deployment);

        deployment.deploy().await.unwrap();

        let mut out = nodes.connect_source_bincode(sum).await;

        deployment.start().await.unwrap();

        assert_eq!(out.next().await.unwrap(), (2, 204));
    }

    #[tokio::test]
    async fn reduce_watermark_garbage_collect() {
        let mut deployment = Deployment::new();

        let flow = FlowBuilder::new();
        let node = flow.process::<()>();
        let external = flow.external::<()>();
        let (tick_send, tick_trigger) = node.source_external_bincode(&external);

        let node_tick = node.tick();
        let (watermark_complete_cycle, watermark) =
            node_tick.cycle_with_initial(node_tick.singleton(q!(1)));
        let next_watermark = watermark.clone().map(q!(|v| v + 1));
        watermark_complete_cycle.complete_next_tick(next_watermark);

        let sum = unsafe {
            let tick_triggered_input = node
                .source_iter(q!([(3, 103)]))
                .tick_batch(&node_tick)
                .continue_if(tick_trigger.clone().tick_batch(&node_tick).first())
                .all_ticks();
            node.source_iter(q!([(0, 100), (1, 101), (2, 102), (2, 102)]))
                .union(tick_triggered_input)
                .into_keyed()
                .reduce_watermark_commutative(
                    watermark,
                    q!(|acc, v| {
                        *acc += v;
                    }),
                )
                .snapshot(&node_tick)
                .entries()
        }
        .all_ticks()
        .send_bincode_external(&external);

        let nodes = flow
            .with_default_optimize()
            .with_process(&node, deployment.Localhost())
            .with_external(&external, deployment.Localhost())
            .deploy(&mut deployment);

        deployment.deploy().await.unwrap();

        let mut tick_send = nodes.connect_sink_bincode(tick_send).await;
        let mut out_recv = nodes.connect_source_bincode(sum).await;

        deployment.start().await.unwrap();

        assert_eq!(out_recv.next().await.unwrap(), (2, 204));

        tick_send.send(()).await.unwrap();

        assert_eq!(out_recv.next().await.unwrap(), (3, 103));
    }
}
