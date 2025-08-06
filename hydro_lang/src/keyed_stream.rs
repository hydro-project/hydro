use std::hash::Hash;
use std::marker::PhantomData;

use stageleft::{IntoQuotedMut, QuotedWithContext, q};

use crate::cycle::{CycleCollection, CycleComplete, ForwardRefMarker};
use crate::ir::HydroNode;
use crate::keyed_optional::KeyedOptional;
use crate::keyed_singleton::KeyedSingleton;
use crate::location::tick::NoAtomic;
use crate::location::{LocationId, NoTick};
use crate::manual_expr::ManualExpr;
use crate::stream::ExactlyOnce;
use crate::{Atomic, Bounded, Location, NoOrder, Stream, Tick, TotalOrder, Unbounded};

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
}

impl<'a, K, V, L, B, O> KeyedStream<K, V, L, B, O, ExactlyOnce>
where
    K: Eq + Hash,
    L: Location<'a>,
{
    pub fn reduce_commutative<F: Fn(&mut V, V) + 'a>(
        self,
        comb: impl IntoQuotedMut<'a, F, L>,
    ) -> KeyedOptional<K, V, L, B> {
        unsafe {
            // SAFETY: the combinator function is commutative
            self.assume_ordering::<TotalOrder>().reduce(comb)
        }
    }
}

impl<'a, K, V, L, B, R> KeyedStream<K, V, L, B, TotalOrder, R>
where
    K: Eq + Hash,
    L: Location<'a>,
{
    pub fn reduce_idempotent<F: Fn(&mut V, V) + 'a>(
        self,
        comb: impl IntoQuotedMut<'a, F, L>,
    ) -> KeyedOptional<K, V, L, B> {
        unsafe {
            // SAFETY: the combinator function is idempotent
            self.assume_retries::<ExactlyOnce>().reduce(comb)
        }
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
