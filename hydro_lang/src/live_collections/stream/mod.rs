//! Definitions for the [`Stream`] live collection.

use std::cell::RefCell;
use std::future::Future;
use std::hash::Hash;
use std::marker::PhantomData;
use std::ops::Deref;
use std::rc::Rc;

use stageleft::{IntoQuotedMut, QuotedWithContext, q};
use tokio::time::Instant;

use super::boundedness::{Bounded, Boundedness, Unbounded};
use super::keyed_singleton::KeyedSingleton;
use super::keyed_stream::KeyedStream;
use super::optional::Optional;
use super::singleton::Singleton;
use crate::compile::ir::{
    CollectionKind, HydroIrOpMetadata, HydroNode, HydroRoot, StreamOrder, StreamRetry, TeeNode,
};
#[cfg(stageleft_runtime)]
use crate::forward_handle::{CycleCollection, ReceiverComplete};
use crate::forward_handle::{ForwardRef, TickCycle};
#[cfg(stageleft_runtime)]
use crate::location::dynamic::{DynLocation, LocationId};
use crate::location::tick::{Atomic, DeferTick, NoAtomic};
use crate::location::{Location, NoTick, Tick, check_matching_location};
use crate::nondet::{NonDet, nondet};

pub mod networking;

/// A trait implemented by valid ordering markers ([`TotalOrder`] and [`NoOrder`]).
#[sealed::sealed]
pub trait Ordering:
    MinOrder<Self, Min = Self> + MinOrder<TotalOrder, Min = Self> + MinOrder<NoOrder, Min = NoOrder>
{
    /// The [`StreamOrder`] corresponding to this type.
    const ORDERING_KIND: StreamOrder;
}

/// Marks the stream as being totally ordered, which means that there are
/// no sources of non-determinism (other than intentional ones) that will
/// affect the order of elements.
pub enum TotalOrder {}

#[sealed::sealed]
impl Ordering for TotalOrder {
    const ORDERING_KIND: StreamOrder = StreamOrder::TotalOrder;
}

/// Marks the stream as having no order, which means that the order of
/// elements may be affected by non-determinism.
///
/// This restricts certain operators, such as `fold` and `reduce`, to only
/// be used with commutative aggregation functions.
pub enum NoOrder {}

#[sealed::sealed]
impl Ordering for NoOrder {
    const ORDERING_KIND: StreamOrder = StreamOrder::NoOrder;
}

/// Helper trait for determining the weakest of two orderings.
#[sealed::sealed]
pub trait MinOrder<Other: ?Sized> {
    /// The weaker of the two orderings.
    type Min: Ordering;
}

#[sealed::sealed]
impl MinOrder<NoOrder> for TotalOrder {
    type Min = NoOrder;
}

#[sealed::sealed]
impl MinOrder<TotalOrder> for TotalOrder {
    type Min = TotalOrder;
}

#[sealed::sealed]
impl MinOrder<TotalOrder> for NoOrder {
    type Min = NoOrder;
}

#[sealed::sealed]
impl MinOrder<NoOrder> for NoOrder {
    type Min = NoOrder;
}

/// A trait implemented by valid retries markers ([`ExactlyOnce`] and [`AtLeastOnce`]).
#[sealed::sealed]
pub trait Retries:
    MinRetries<Self, Min = Self>
    + MinRetries<ExactlyOnce, Min = Self>
    + MinRetries<AtLeastOnce, Min = AtLeastOnce>
{
    /// The [`StreamRetry`] corresponding to this type.
    const RETRIES_KIND: StreamRetry;
}

/// Marks the stream as having deterministic message cardinality, with no
/// possibility of duplicates.
pub enum ExactlyOnce {}

#[sealed::sealed]
impl Retries for ExactlyOnce {
    const RETRIES_KIND: StreamRetry = StreamRetry::ExactlyOnce;
}

/// Marks the stream as having non-deterministic message cardinality, which
/// means that duplicates may occur, but messages will not be dropped.
pub enum AtLeastOnce {}

#[sealed::sealed]
impl Retries for AtLeastOnce {
    const RETRIES_KIND: StreamRetry = StreamRetry::AtLeastOnce;
}

/// Helper trait for determining the weakest of two retry guarantees.
#[sealed::sealed]
pub trait MinRetries<Other: ?Sized> {
    /// The weaker of the two retry guarantees.
    type Min: Retries;
}

#[sealed::sealed]
impl MinRetries<AtLeastOnce> for ExactlyOnce {
    type Min = AtLeastOnce;
}

#[sealed::sealed]
impl MinRetries<ExactlyOnce> for ExactlyOnce {
    type Min = ExactlyOnce;
}

#[sealed::sealed]
impl MinRetries<ExactlyOnce> for AtLeastOnce {
    type Min = AtLeastOnce;
}

#[sealed::sealed]
impl MinRetries<AtLeastOnce> for AtLeastOnce {
    type Min = AtLeastOnce;
}

/// Streaming sequence of elements with type `Type`.
///
/// This live collection represents a growing sequence of elements, with new elements being
/// asynchronously appended to the end of the sequence. This can be used to model the arrival
/// of network input, such as API requests, or streaming ingestion.
///
/// By default, all streams have deterministic ordering and each element is materialized exactly
/// once. But streams can also capture non-determinism via the `Order` and `Retries` type
/// parameters. When the ordering / retries guarantee is relaxed, fewer APIs will be available
/// on the stream. For example, if the stream is unordered, you cannot invoke [`Stream::first`].
///
/// Type Parameters:
/// - `Type`: the type of elements in the stream
/// - `Loc`: the location where the stream is being materialized
/// - `Bound`: the boundedness of the stream, which is either [`Bounded`] or [`Unbounded`]
/// - `Order`: the ordering of the stream, which is either [`TotalOrder`] or [`NoOrder`]
///   (default is [`TotalOrder`])
/// - `Retries`: the retry guarantee of the stream, which is either [`ExactlyOnce`] or
///   [`AtLeastOnce`] (default is [`ExactlyOnce`])
pub struct Stream<
    Type,
    Loc,
    Bound: Boundedness = Unbounded,
    Order: Ordering = TotalOrder,
    Retry: Retries = ExactlyOnce,
> {
    pub(crate) location: Loc,
    pub(crate) ir_node: RefCell<HydroNode>,

    _phantom: PhantomData<(Type, Loc, Bound, Order, Retry)>,
}

impl<'a, T, L, O: Ordering, R: Retries> From<Stream<T, L, Bounded, O, R>>
    for Stream<T, L, Unbounded, O, R>
where
    L: Location<'a>,
{
    fn from(stream: Stream<T, L, Bounded, O, R>) -> Stream<T, L, Unbounded, O, R> {
        Stream {
            location: stream.location,
            ir_node: stream.ir_node,
            _phantom: PhantomData,
        }
    }
}

impl<'a, T, L, B: Boundedness, R: Retries> From<Stream<T, L, B, TotalOrder, R>>
    for Stream<T, L, B, NoOrder, R>
where
    L: Location<'a>,
{
    fn from(stream: Stream<T, L, B, TotalOrder, R>) -> Stream<T, L, B, NoOrder, R> {
        Stream {
            location: stream.location,
            ir_node: stream.ir_node,
            _phantom: PhantomData,
        }
    }
}

impl<'a, T, L, B: Boundedness, O: Ordering> From<Stream<T, L, B, O, ExactlyOnce>>
    for Stream<T, L, B, O, AtLeastOnce>
where
    L: Location<'a>,
{
    fn from(stream: Stream<T, L, B, O, ExactlyOnce>) -> Stream<T, L, B, O, AtLeastOnce> {
        Stream {
            location: stream.location,
            ir_node: stream.ir_node,
            _phantom: PhantomData,
        }
    }
}

impl<'a, T, L, O: Ordering, R: Retries> DeferTick for Stream<T, Tick<L>, Bounded, O, R>
where
    L: Location<'a>,
{
    fn defer_tick(self) -> Self {
        Stream::defer_tick(self)
    }
}

impl<'a, T, L, O: Ordering, R: Retries> CycleCollection<'a, TickCycle>
    for Stream<T, Tick<L>, Bounded, O, R>
where
    L: Location<'a>,
{
    type Location = Tick<L>;

    fn create_source(ident: syn::Ident, location: Tick<L>) -> Self {
        Stream::new(
            location.clone(),
            HydroNode::CycleSource {
                ident,
                metadata: location.new_node_metadata(Self::collection_kind()),
            },
        )
    }
}

impl<'a, T, L, O: Ordering, R: Retries> ReceiverComplete<'a, TickCycle>
    for Stream<T, Tick<L>, Bounded, O, R>
where
    L: Location<'a>,
{
    fn complete(self, ident: syn::Ident, expected_location: LocationId) {
        assert_eq!(
            Location::id(&self.location),
            expected_location,
            "locations do not match"
        );
        self.location
            .flow_state()
            .borrow_mut()
            .push_root(HydroRoot::CycleSink {
                ident,
                input: Box::new(self.ir_node.into_inner()),
                op_metadata: HydroIrOpMetadata::new(),
            });
    }
}

impl<'a, T, L, B: Boundedness, O: Ordering, R: Retries> CycleCollection<'a, ForwardRef>
    for Stream<T, L, B, O, R>
where
    L: Location<'a> + NoTick,
{
    type Location = L;

    fn create_source(ident: syn::Ident, location: L) -> Self {
        Stream::new(
            location.clone(),
            HydroNode::CycleSource {
                ident,
                metadata: location.new_node_metadata(Self::collection_kind()),
            },
        )
    }
}

impl<'a, T, L, B: Boundedness, O: Ordering, R: Retries> ReceiverComplete<'a, ForwardRef>
    for Stream<T, L, B, O, R>
where
    L: Location<'a> + NoTick,
{
    fn complete(self, ident: syn::Ident, expected_location: LocationId) {
        assert_eq!(
            Location::id(&self.location),
            expected_location,
            "locations do not match"
        );
        self.location
            .flow_state()
            .borrow_mut()
            .push_root(HydroRoot::CycleSink {
                ident,
                input: Box::new(self.ir_node.into_inner()),
                op_metadata: HydroIrOpMetadata::new(),
            });
    }
}

impl<'a, T, L, B: Boundedness, O: Ordering, R: Retries> Clone for Stream<T, L, B, O, R>
where
    T: Clone,
    L: Location<'a>,
{
    fn clone(&self) -> Self {
        if !matches!(self.ir_node.borrow().deref(), HydroNode::Tee { .. }) {
            let orig_ir_node = self.ir_node.replace(HydroNode::Placeholder);
            *self.ir_node.borrow_mut() = HydroNode::Tee {
                inner: TeeNode(Rc::new(RefCell::new(orig_ir_node))),
                metadata: self.location.new_node_metadata(Self::collection_kind()),
            };
        }

        if let HydroNode::Tee { inner, metadata } = self.ir_node.borrow().deref() {
            Stream {
                location: self.location.clone(),
                ir_node: HydroNode::Tee {
                    inner: TeeNode(inner.0.clone()),
                    metadata: metadata.clone(),
                }
                .into(),
                _phantom: PhantomData,
            }
        } else {
            unreachable!()
        }
    }
}

impl<'a, T, L, B: Boundedness, O: Ordering, R: Retries> Stream<T, L, B, O, R>
where
    L: Location<'a>,
{
    pub(crate) fn new(location: L, ir_node: HydroNode) -> Self {
        debug_assert_eq!(ir_node.metadata().location_kind, Location::id(&location));
        debug_assert_eq!(ir_node.metadata().collection_kind, Self::collection_kind());

        Stream {
            location,
            ir_node: RefCell::new(ir_node),
            _phantom: PhantomData,
        }
    }

    /// Returns the [`Location`] where this stream is being materialized.
    pub fn location(&self) -> &L {
        &self.location
    }

    pub(crate) fn collection_kind() -> CollectionKind {
        CollectionKind::Stream {
            bound: B::BOUND_KIND,
            order: O::ORDERING_KIND,
            retry: R::RETRIES_KIND,
            element_type: stageleft::quote_type::<T>().into(),
        }
    }

    /// Produces a stream based on invoking `f` on each element.
    /// If you do not want to modify the stream and instead only want to view
    /// each item use [`Stream::inspect`] instead.
    ///
    /// # Example
    /// ```rust
    /// # use hydro_lang::prelude::*;
    /// # use futures::StreamExt;
    /// # tokio_test::block_on(hydro_lang::test_util::stream_transform_test(|process| {
    /// let words = process.source_iter(q!(vec!["hello", "world"]));
    /// words.map(q!(|x| x.to_uppercase()))
    /// # }, |mut stream| async move {
    /// # for w in vec!["HELLO", "WORLD"] {
    /// #     assert_eq!(stream.next().await.unwrap(), w);
    /// # }
    /// # }));
    /// ```
    pub fn map<U, F>(self, f: impl IntoQuotedMut<'a, F, L>) -> Stream<U, L, B, O, R>
    where
        F: Fn(T) -> U + 'a,
    {
        let f = f.splice_fn1_ctx(&self.location).into();
        Stream::new(
            self.location.clone(),
            HydroNode::Map {
                f,
                input: Box::new(self.ir_node.into_inner()),
                metadata: self
                    .location
                    .new_node_metadata(Stream::<U, L, B, O, R>::collection_kind()),
            },
        )
    }

    /// For each item `i` in the input stream, transform `i` using `f` and then treat the
    /// result as an [`Iterator`] to produce items one by one. The implementation for [`Iterator`]
    /// for the output type `U` must produce items in a **deterministic** order.
    ///
    /// For example, `U` could be a `Vec`, but not a `HashSet`. If the order of the items in `U` is
    /// not deterministic, use [`Stream::flat_map_unordered`] instead.
    ///
    /// # Example
    /// ```rust
    /// # use hydro_lang::prelude::*;
    /// # use futures::StreamExt;
    /// # tokio_test::block_on(hydro_lang::test_util::stream_transform_test(|process| {
    /// process
    ///     .source_iter(q!(vec![vec![1, 2], vec![3, 4]]))
    ///     .flat_map_ordered(q!(|x| x))
    /// # }, |mut stream| async move {
    /// // 1, 2, 3, 4
    /// # for w in (1..5) {
    /// #     assert_eq!(stream.next().await.unwrap(), w);
    /// # }
    /// # }));
    /// ```
    pub fn flat_map_ordered<U, I, F>(self, f: impl IntoQuotedMut<'a, F, L>) -> Stream<U, L, B, O, R>
    where
        I: IntoIterator<Item = U>,
        F: Fn(T) -> I + 'a,
    {
        let f = f.splice_fn1_ctx(&self.location).into();
        Stream::new(
            self.location.clone(),
            HydroNode::FlatMap {
                f,
                input: Box::new(self.ir_node.into_inner()),
                metadata: self
                    .location
                    .new_node_metadata(Stream::<U, L, B, O, R>::collection_kind()),
            },
        )
    }

    /// Like [`Stream::flat_map_ordered`], but allows the implementation of [`Iterator`]
    /// for the output type `U` to produce items in any order.
    ///
    /// # Example
    /// ```rust
    /// # use hydro_lang::{prelude::*, live_collections::stream::{NoOrder, ExactlyOnce}};
    /// # use futures::StreamExt;
    /// # tokio_test::block_on(hydro_lang::test_util::stream_transform_test::<_, _, NoOrder, ExactlyOnce>(|process| {
    /// process
    ///     .source_iter(q!(vec![
    ///         std::collections::HashSet::<i32>::from_iter(vec![1, 2]),
    ///         std::collections::HashSet::from_iter(vec![3, 4]),
    ///     ]))
    ///     .flat_map_unordered(q!(|x| x))
    /// # }, |mut stream| async move {
    /// // 1, 2, 3, 4, but in no particular order
    /// # let mut results = Vec::new();
    /// # for w in (1..5) {
    /// #     results.push(stream.next().await.unwrap());
    /// # }
    /// # results.sort();
    /// # assert_eq!(results, vec![1, 2, 3, 4]);
    /// # }));
    /// ```
    pub fn flat_map_unordered<U, I, F>(
        self,
        f: impl IntoQuotedMut<'a, F, L>,
    ) -> Stream<U, L, B, NoOrder, R>
    where
        I: IntoIterator<Item = U>,
        F: Fn(T) -> I + 'a,
    {
        let f = f.splice_fn1_ctx(&self.location).into();
        Stream::new(
            self.location.clone(),
            HydroNode::FlatMap {
                f,
                input: Box::new(self.ir_node.into_inner()),
                metadata: self
                    .location
                    .new_node_metadata(Stream::<U, L, B, NoOrder, R>::collection_kind()),
            },
        )
    }

    /// For each item `i` in the input stream, treat `i` as an [`Iterator`] and produce its items one by one.
    /// The implementation for [`Iterator`] for the element type `T` must produce items in a **deterministic** order.
    ///
    /// For example, `T` could be a `Vec`, but not a `HashSet`. If the order of the items in `T` is
    /// not deterministic, use [`Stream::flatten_unordered`] instead.
    ///
    /// ```rust
    /// # use hydro_lang::prelude::*;
    /// # use futures::StreamExt;
    /// # tokio_test::block_on(hydro_lang::test_util::stream_transform_test(|process| {
    /// process
    ///     .source_iter(q!(vec![vec![1, 2], vec![3, 4]]))
    ///     .flatten_ordered()
    /// # }, |mut stream| async move {
    /// // 1, 2, 3, 4
    /// # for w in (1..5) {
    /// #     assert_eq!(stream.next().await.unwrap(), w);
    /// # }
    /// # }));
    /// ```
    pub fn flatten_ordered<U>(self) -> Stream<U, L, B, O, R>
    where
        T: IntoIterator<Item = U>,
    {
        self.flat_map_ordered(q!(|d| d))
    }

    /// Like [`Stream::flatten_ordered`], but allows the implementation of [`Iterator`]
    /// for the element type `T` to produce items in any order.
    ///
    /// # Example
    /// ```rust
    /// # use hydro_lang::{prelude::*, live_collections::stream::{NoOrder, ExactlyOnce}};
    /// # use futures::StreamExt;
    /// # tokio_test::block_on(hydro_lang::test_util::stream_transform_test::<_, _, NoOrder, ExactlyOnce>(|process| {
    /// process
    ///     .source_iter(q!(vec![
    ///         std::collections::HashSet::<i32>::from_iter(vec![1, 2]),
    ///         std::collections::HashSet::from_iter(vec![3, 4]),
    ///     ]))
    ///     .flatten_unordered()
    /// # }, |mut stream| async move {
    /// // 1, 2, 3, 4, but in no particular order
    /// # let mut results = Vec::new();
    /// # for w in (1..5) {
    /// #     results.push(stream.next().await.unwrap());
    /// # }
    /// # results.sort();
    /// # assert_eq!(results, vec![1, 2, 3, 4]);
    /// # }));
    /// ```
    pub fn flatten_unordered<U>(self) -> Stream<U, L, B, NoOrder, R>
    where
        T: IntoIterator<Item = U>,
    {
        self.flat_map_unordered(q!(|d| d))
    }

    /// Creates a stream containing only the elements of the input stream that satisfy a predicate
    /// `f`, preserving the order of the elements.
    ///
    /// The closure `f` receives a reference `&T` rather than an owned value `T` because filtering does
    /// not modify or take ownership of the values. If you need to modify the values while filtering
    /// use [`Stream::filter_map`] instead.
    ///
    /// # Example
    /// ```rust
    /// # use hydro_lang::prelude::*;
    /// # use futures::StreamExt;
    /// # tokio_test::block_on(hydro_lang::test_util::stream_transform_test(|process| {
    /// process
    ///     .source_iter(q!(vec![1, 2, 3, 4]))
    ///     .filter(q!(|&x| x > 2))
    /// # }, |mut stream| async move {
    /// // 3, 4
    /// # for w in (3..5) {
    /// #     assert_eq!(stream.next().await.unwrap(), w);
    /// # }
    /// # }));
    /// ```
    pub fn filter<F>(self, f: impl IntoQuotedMut<'a, F, L>) -> Self
    where
        F: Fn(&T) -> bool + 'a,
    {
        let f = f.splice_fn1_borrow_ctx(&self.location).into();
        Stream::new(
            self.location.clone(),
            HydroNode::Filter {
                f,
                input: Box::new(self.ir_node.into_inner()),
                metadata: self.location.new_node_metadata(Self::collection_kind()),
            },
        )
    }

    /// An operator that both filters and maps. It yields only the items for which the supplied closure `f` returns `Some(value)`.
    ///
    /// # Example
    /// ```rust
    /// # use hydro_lang::prelude::*;
    /// # use futures::StreamExt;
    /// # tokio_test::block_on(hydro_lang::test_util::stream_transform_test(|process| {
    /// process
    ///     .source_iter(q!(vec!["1", "hello", "world", "2"]))
    ///     .filter_map(q!(|s| s.parse::<usize>().ok()))
    /// # }, |mut stream| async move {
    /// // 1, 2
    /// # for w in (1..3) {
    /// #     assert_eq!(stream.next().await.unwrap(), w);
    /// # }
    /// # }));
    /// ```
    pub fn filter_map<U, F>(self, f: impl IntoQuotedMut<'a, F, L>) -> Stream<U, L, B, O, R>
    where
        F: Fn(T) -> Option<U> + 'a,
    {
        let f = f.splice_fn1_ctx(&self.location).into();
        Stream::new(
            self.location.clone(),
            HydroNode::FilterMap {
                f,
                input: Box::new(self.ir_node.into_inner()),
                metadata: self
                    .location
                    .new_node_metadata(Stream::<U, L, B, O, R>::collection_kind()),
            },
        )
    }

    /// Generates a stream that maps each input element `i` to a tuple `(i, x)`,
    /// where `x` is the final value of `other`, a bounded [`Singleton`] or [`Optional`].
    /// If `other` is an empty [`Optional`], no values will be produced.
    ///
    /// # Example
    /// ```rust
    /// # use hydro_lang::prelude::*;
    /// # use futures::StreamExt;
    /// # tokio_test::block_on(hydro_lang::test_util::stream_transform_test(|process| {
    /// let tick = process.tick();
    /// let batch = process
    ///   .source_iter(q!(vec![1, 2, 3, 4]))
    ///   .batch(&tick, nondet!(/** test */));
    /// let count = batch.clone().count(); // `count()` returns a singleton
    /// batch.cross_singleton(count).all_ticks()
    /// # }, |mut stream| async move {
    /// // (1, 4), (2, 4), (3, 4), (4, 4)
    /// # for w in vec![(1, 4), (2, 4), (3, 4), (4, 4)] {
    /// #     assert_eq!(stream.next().await.unwrap(), w);
    /// # }
    /// # }));
    /// ```
    pub fn cross_singleton<O2>(
        self,
        other: impl Into<Optional<O2, L, Bounded>>,
    ) -> Stream<(T, O2), L, B, O, R>
    where
        O2: Clone,
    {
        let other: Optional<O2, L, Bounded> = other.into();
        check_matching_location(&self.location, &other.location);

        Stream::new(
            self.location.clone(),
            HydroNode::CrossSingleton {
                left: Box::new(self.ir_node.into_inner()),
                right: Box::new(other.ir_node.into_inner()),
                metadata: self
                    .location
                    .new_node_metadata(Stream::<(T, O2), L, B, O, R>::collection_kind()),
            },
        )
    }

    /// Passes this stream through if the argument (a [`Bounded`] [`Optional`]`) is non-null, otherwise the output is empty.
    ///
    /// Useful for gating the release of elements based on a condition, such as only processing requests if you are the
    /// leader of a cluster.
    ///
    /// # Example
    /// ```rust
    /// # use hydro_lang::prelude::*;
    /// # use futures::StreamExt;
    /// # tokio_test::block_on(hydro_lang::test_util::stream_transform_test(|process| {
    /// let tick = process.tick();
    /// // ticks are lazy by default, forces the second tick to run
    /// tick.spin_batch(q!(1)).all_ticks().for_each(q!(|_| {}));
    ///
    /// let batch_first_tick = process
    ///   .source_iter(q!(vec![1, 2, 3, 4]))
    ///   .batch(&tick, nondet!(/** test */));
    /// let batch_second_tick = process
    ///   .source_iter(q!(vec![5, 6, 7, 8]))
    ///   .batch(&tick, nondet!(/** test */))
    ///   .defer_tick(); // appears on the second tick
    /// let some_on_first_tick = tick.optional_first_tick(q!(()));
    /// batch_first_tick.chain(batch_second_tick)
    ///   .filter_if_some(some_on_first_tick)
    ///   .all_ticks()
    /// # }, |mut stream| async move {
    /// // [1, 2, 3, 4]
    /// # for w in vec![1, 2, 3, 4] {
    /// #     assert_eq!(stream.next().await.unwrap(), w);
    /// # }
    /// # }));
    /// ```
    pub fn filter_if_some<U>(self, signal: Optional<U, L, Bounded>) -> Stream<T, L, B, O, R> {
        self.cross_singleton(signal.map(q!(|_u| ())))
            .map(q!(|(d, _signal)| d))
    }

    /// Passes this stream through if the argument (a [`Bounded`] [`Optional`]`) is null, otherwise the output is empty.
    ///
    /// Useful for gating the release of elements based on a condition, such as triggering a protocol if you are missing
    /// some local state.
    ///
    /// # Example
    /// ```rust
    /// # use hydro_lang::prelude::*;
    /// # use futures::StreamExt;
    /// # tokio_test::block_on(hydro_lang::test_util::stream_transform_test(|process| {
    /// let tick = process.tick();
    /// // ticks are lazy by default, forces the second tick to run
    /// tick.spin_batch(q!(1)).all_ticks().for_each(q!(|_| {}));
    ///
    /// let batch_first_tick = process
    ///   .source_iter(q!(vec![1, 2, 3, 4]))
    ///   .batch(&tick, nondet!(/** test */));
    /// let batch_second_tick = process
    ///   .source_iter(q!(vec![5, 6, 7, 8]))
    ///   .batch(&tick, nondet!(/** test */))
    ///   .defer_tick(); // appears on the second tick
    /// let some_on_first_tick = tick.optional_first_tick(q!(()));
    /// batch_first_tick.chain(batch_second_tick)
    ///   .filter_if_none(some_on_first_tick)
    ///   .all_ticks()
    /// # }, |mut stream| async move {
    /// // [5, 6, 7, 8]
    /// # for w in vec![5, 6, 7, 8] {
    /// #     assert_eq!(stream.next().await.unwrap(), w);
    /// # }
    /// # }));
    /// ```
    pub fn filter_if_none<U>(self, other: Optional<U, L, Bounded>) -> Stream<T, L, B, O, R> {
        self.filter_if_some(
            other
                .map(q!(|_| ()))
                .into_singleton()
                .filter(q!(|o| o.is_none())),
        )
    }

    /// Forms the cross-product (Cartesian product, cross-join) of the items in the 2 input streams, returning all
    /// tupled pairs in a non-deterministic order.
    ///
    /// # Example
    /// ```rust
    /// # use hydro_lang::prelude::*;
    /// # use std::collections::HashSet;
    /// # use futures::StreamExt;
    /// # tokio_test::block_on(hydro_lang::test_util::stream_transform_test(|process| {
    /// let tick = process.tick();
    /// let stream1 = process.source_iter(q!(vec!['a', 'b', 'c']));
    /// let stream2 = process.source_iter(q!(vec![1, 2, 3]));
    /// stream1.cross_product(stream2)
    /// # }, |mut stream| async move {
    /// # let expected = HashSet::from([('a', 1), ('b', 1), ('c', 1), ('a', 2), ('b', 2), ('c', 2), ('a', 3), ('b', 3), ('c', 3)]);
    /// # stream.map(|i| assert!(expected.contains(&i)));
    /// # }));
    /// ```
    pub fn cross_product<T2, O2: Ordering>(
        self,
        other: Stream<T2, L, B, O2, R>,
    ) -> Stream<(T, T2), L, B, NoOrder, R>
    where
        T: Clone,
        T2: Clone,
    {
        check_matching_location(&self.location, &other.location);

        Stream::new(
            self.location.clone(),
            HydroNode::CrossProduct {
                left: Box::new(self.ir_node.into_inner()),
                right: Box::new(other.ir_node.into_inner()),
                metadata: self
                    .location
                    .new_node_metadata(Stream::<(T, T2), L, B, NoOrder, R>::collection_kind()),
            },
        )
    }

    /// Takes one stream as input and filters out any duplicate occurrences. The output
    /// contains all unique values from the input.
    ///
    /// # Example
    /// ```rust
    /// # use hydro_lang::prelude::*;
    /// # use futures::StreamExt;
    /// # tokio_test::block_on(hydro_lang::test_util::stream_transform_test(|process| {
    /// let tick = process.tick();
    /// process.source_iter(q!(vec![1, 2, 3, 2, 1, 4])).unique()
    /// # }, |mut stream| async move {
    /// # for w in vec![1, 2, 3, 4] {
    /// #     assert_eq!(stream.next().await.unwrap(), w);
    /// # }
    /// # }));
    /// ```
    pub fn unique(self) -> Stream<T, L, B, O, ExactlyOnce>
    where
        T: Eq + Hash,
    {
        Stream::new(
            self.location.clone(),
            HydroNode::Unique {
                input: Box::new(self.ir_node.into_inner()),
                metadata: self
                    .location
                    .new_node_metadata(Stream::<T, L, B, O, ExactlyOnce>::collection_kind()),
            },
        )
    }

    /// Outputs everything in this stream that is *not* contained in the `other` stream.
    ///
    /// The `other` stream must be [`Bounded`], since this function will wait until
    /// all its elements are available before producing any output.
    /// # Example
    /// ```rust
    /// # use hydro_lang::prelude::*;
    /// # use futures::StreamExt;
    /// # tokio_test::block_on(hydro_lang::test_util::stream_transform_test(|process| {
    /// let tick = process.tick();
    /// let stream = process
    ///   .source_iter(q!(vec![ 1, 2, 3, 4 ]))
    ///   .batch(&tick, nondet!(/** test */));
    /// let batch = process
    ///   .source_iter(q!(vec![1, 2]))
    ///   .batch(&tick, nondet!(/** test */));
    /// stream.filter_not_in(batch).all_ticks()
    /// # }, |mut stream| async move {
    /// # for w in vec![3, 4] {
    /// #     assert_eq!(stream.next().await.unwrap(), w);
    /// # }
    /// # }));
    /// ```
    pub fn filter_not_in<O2: Ordering>(self, other: Stream<T, L, Bounded, O2, R>) -> Self
    where
        T: Eq + Hash,
    {
        check_matching_location(&self.location, &other.location);

        Stream::new(
            self.location.clone(),
            HydroNode::Difference {
                pos: Box::new(self.ir_node.into_inner()),
                neg: Box::new(other.ir_node.into_inner()),
                metadata: self
                    .location
                    .new_node_metadata(Stream::<T, L, Bounded, O, R>::collection_kind()),
            },
        )
    }

    /// An operator which allows you to "inspect" each element of a stream without
    /// modifying it. The closure `f` is called on a reference to each item. This is
    /// mainly useful for debugging, and should not be used to generate side-effects.
    ///
    /// # Example
    /// ```rust
    /// # use hydro_lang::prelude::*;
    /// # use futures::StreamExt;
    /// # tokio_test::block_on(hydro_lang::test_util::stream_transform_test(|process| {
    /// let nums = process.source_iter(q!(vec![1, 2]));
    /// // prints "1 * 10 = 10" and "2 * 10 = 20"
    /// nums.inspect(q!(|x| println!("{} * 10 = {}", x, x * 10)))
    /// # }, |mut stream| async move {
    /// # for w in vec![1, 2] {
    /// #     assert_eq!(stream.next().await.unwrap(), w);
    /// # }
    /// # }));
    /// ```
    pub fn inspect<F>(self, f: impl IntoQuotedMut<'a, F, L>) -> Self
    where
        F: Fn(&T) + 'a,
    {
        let f = f.splice_fn1_borrow_ctx(&self.location).into();

        Stream::new(
            self.location.clone(),
            HydroNode::Inspect {
                f,
                input: Box::new(self.ir_node.into_inner()),
                metadata: self.location.new_node_metadata(Self::collection_kind()),
            },
        )
    }

    /// An operator which allows you to "name" a `HydroNode`.
    /// This is only used for testing, to correlate certain `HydroNode`s with IDs.
    pub fn ir_node_named(self, name: &str) -> Stream<T, L, B, O, R> {
        {
            let mut node = self.ir_node.borrow_mut();
            let metadata = node.metadata_mut();
            metadata.tag = Some(name.to_string());
        }
        self
    }

    /// Explicitly "casts" the stream to a type with a different ordering
    /// guarantee. Useful in unsafe code where the ordering cannot be proven
    /// by the type-system.
    ///
    /// # Non-Determinism
    /// This function is used as an escape hatch, and any mistakes in the
    /// provided ordering guarantee will propagate into the guarantees
    /// for the rest of the program.
    pub fn assume_ordering<O2: Ordering>(self, _nondet: NonDet) -> Stream<T, L, B, O2, R> {
        if O::ORDERING_KIND == O2::ORDERING_KIND {
            Stream::new(self.location, self.ir_node.into_inner())
        } else if O2::ORDERING_KIND == StreamOrder::NoOrder {
            // We can always weaken the ordering guarantee
            Stream::new(
                self.location.clone(),
                HydroNode::Cast {
                    inner: Box::new(self.ir_node.into_inner()),
                    metadata: self
                        .location
                        .new_node_metadata(Stream::<T, L, B, O2, R>::collection_kind()),
                },
            )
        } else {
            Stream::new(
                self.location.clone(),
                HydroNode::ObserveNonDet {
                    inner: Box::new(self.ir_node.into_inner()),
                    trusted: false,
                    metadata: self
                        .location
                        .new_node_metadata(Stream::<T, L, B, O2, R>::collection_kind()),
                },
            )
        }
    }

    // only for internal APIs that have been carefully vetted to ensure that the non-determinism
    // is not observable
    fn assume_ordering_trusted<O2: Ordering>(self, _nondet: NonDet) -> Stream<T, L, B, O2, R> {
        if O::ORDERING_KIND == O2::ORDERING_KIND {
            Stream::new(self.location, self.ir_node.into_inner())
        } else if O2::ORDERING_KIND == StreamOrder::NoOrder {
            // We can always weaken the ordering guarantee
            Stream::new(
                self.location.clone(),
                HydroNode::Cast {
                    inner: Box::new(self.ir_node.into_inner()),
                    metadata: self
                        .location
                        .new_node_metadata(Stream::<T, L, B, O2, R>::collection_kind()),
                },
            )
        } else {
            Stream::new(
                self.location.clone(),
                HydroNode::ObserveNonDet {
                    inner: Box::new(self.ir_node.into_inner()),
                    trusted: true,
                    metadata: self
                        .location
                        .new_node_metadata(Stream::<T, L, B, O2, R>::collection_kind()),
                },
            )
        }
    }

    /// Weakens the ordering guarantee provided by the stream to [`NoOrder`],
    /// which is always safe because that is the weakest possible guarantee.
    pub fn weakest_ordering(self) -> Stream<T, L, B, NoOrder, R> {
        let nondet = nondet!(/** this is a weaker ordering guarantee, so it is safe to assume */);
        self.assume_ordering::<NoOrder>(nondet)
    }

    /// Weakens the ordering guarantee provided by the stream to `O2`, with the type-system
    /// enforcing that `O2` is weaker than the input ordering guarantee.
    pub fn weaken_ordering<O2: Ordering + MinOrder<O, Min = O2>>(self) -> Stream<T, L, B, O2, R> {
        let nondet = nondet!(/** this is a weaker ordering guarantee, so it is safe to assume */);
        self.assume_ordering::<O2>(nondet)
    }

    /// Explicitly "casts" the stream to a type with a different retries
    /// guarantee. Useful in unsafe code where the lack of retries cannot
    /// be proven by the type-system.
    ///
    /// # Non-Determinism
    /// This function is used as an escape hatch, and any mistakes in the
    /// provided retries guarantee will propagate into the guarantees
    /// for the rest of the program.
    pub fn assume_retries<R2: Retries>(self, _nondet: NonDet) -> Stream<T, L, B, O, R2> {
        if R::RETRIES_KIND == R2::RETRIES_KIND {
            Stream::new(self.location, self.ir_node.into_inner())
        } else if R2::RETRIES_KIND == StreamRetry::AtLeastOnce {
            // We can always weaken the retries guarantee
            Stream::new(
                self.location.clone(),
                HydroNode::Cast {
                    inner: Box::new(self.ir_node.into_inner()),
                    metadata: self
                        .location
                        .new_node_metadata(Stream::<T, L, B, O, R2>::collection_kind()),
                },
            )
        } else {
            Stream::new(
                self.location.clone(),
                HydroNode::ObserveNonDet {
                    inner: Box::new(self.ir_node.into_inner()),
                    trusted: false,
                    metadata: self
                        .location
                        .new_node_metadata(Stream::<T, L, B, O, R2>::collection_kind()),
                },
            )
        }
    }

    // only for internal APIs that have been carefully vetted to ensure that the non-determinism
    // is not observable
    fn assume_retries_trusted<R2: Retries>(self, _nondet: NonDet) -> Stream<T, L, B, O, R2> {
        if R::RETRIES_KIND == R2::RETRIES_KIND {
            Stream::new(self.location, self.ir_node.into_inner())
        } else if R2::RETRIES_KIND == StreamRetry::AtLeastOnce {
            // We can always weaken the retries guarantee
            Stream::new(
                self.location.clone(),
                HydroNode::Cast {
                    inner: Box::new(self.ir_node.into_inner()),
                    metadata: self
                        .location
                        .new_node_metadata(Stream::<T, L, B, O, R2>::collection_kind()),
                },
            )
        } else {
            Stream::new(
                self.location.clone(),
                HydroNode::ObserveNonDet {
                    inner: Box::new(self.ir_node.into_inner()),
                    trusted: true,
                    metadata: self
                        .location
                        .new_node_metadata(Stream::<T, L, B, O, R2>::collection_kind()),
                },
            )
        }
    }

    /// Weakens the retries guarantee provided by the stream to [`AtLeastOnce`],
    /// which is always safe because that is the weakest possible guarantee.
    pub fn weakest_retries(self) -> Stream<T, L, B, O, AtLeastOnce> {
        let nondet = nondet!(/** this is a weaker retry guarantee, so it is safe to assume */);
        self.assume_retries::<AtLeastOnce>(nondet)
    }

    /// Weakens the retries guarantee provided by the stream to `R2`, with the type-system
    /// enforcing that `R2` is weaker than the input retries guarantee.
    pub fn weaken_retries<R2: Retries + MinRetries<R, Min = R2>>(self) -> Stream<T, L, B, O, R2> {
        let nondet = nondet!(/** this is a weaker retry guarantee, so it is safe to assume */);
        self.assume_retries::<R2>(nondet)
    }
}

impl<'a, T, L, B: Boundedness, O: Ordering> Stream<T, L, B, O, ExactlyOnce>
where
    L: Location<'a>,
{
    /// Given a stream with [`ExactlyOnce`] retry guarantees, weakens it to an arbitrary guarantee
    /// `R2`, which is safe because all guarantees are equal to or weaker than [`ExactlyOnce`]
    pub fn weaker_retries<R2: Retries>(self) -> Stream<T, L, B, O, R2> {
        self.assume_retries(
            nondet!(/** any retry ordering is the same or weaker than ExactlyOnce */),
        )
    }
}

impl<'a, T, L, B: Boundedness, O: Ordering, R: Retries> Stream<&T, L, B, O, R>
where
    L: Location<'a>,
{
    /// Clone each element of the stream; akin to `map(q!(|d| d.clone()))`.
    ///
    /// # Example
    /// ```rust
    /// # use hydro_lang::prelude::*;
    /// # use futures::StreamExt;
    /// # tokio_test::block_on(hydro_lang::test_util::stream_transform_test(|process| {
    /// process.source_iter(q!(&[1, 2, 3])).cloned()
    /// # }, |mut stream| async move {
    /// // 1, 2, 3
    /// # for w in vec![1, 2, 3] {
    /// #     assert_eq!(stream.next().await.unwrap(), w);
    /// # }
    /// # }));
    /// ```
    pub fn cloned(self) -> Stream<T, L, B, O, R>
    where
        T: Clone,
    {
        self.map(q!(|d| d.clone()))
    }
}

impl<'a, T, L, B: Boundedness, O: Ordering, R: Retries> Stream<T, L, B, O, R>
where
    L: Location<'a>,
{
    /// Combines elements of the stream into a [`Singleton`], by starting with an initial value,
    /// generated by the `init` closure, and then applying the `comb` closure to each element in the stream.
    /// Unlike iterators, `comb` takes the accumulator by `&mut` reference, so that it can be modified in place.
    ///
    /// The `comb` closure must be **commutative** AND **idempotent**, as the order of input items is not guaranteed
    /// and there may be duplicates.
    ///
    /// # Example
    /// ```rust
    /// # use hydro_lang::prelude::*;
    /// # use futures::StreamExt;
    /// # tokio_test::block_on(hydro_lang::test_util::stream_transform_test(|process| {
    /// let tick = process.tick();
    /// let bools = process.source_iter(q!(vec![false, true, false]));
    /// let batch = bools.batch(&tick, nondet!(/** test */));
    /// batch
    ///     .fold_commutative_idempotent(q!(|| false), q!(|acc, x| *acc |= x))
    ///     .all_ticks()
    /// # }, |mut stream| async move {
    /// // true
    /// # assert_eq!(stream.next().await.unwrap(), true);
    /// # }));
    /// ```
    pub fn fold_commutative_idempotent<A, I, F>(
        self,
        init: impl IntoQuotedMut<'a, I, L>,
        comb: impl IntoQuotedMut<'a, F, L>,
    ) -> Singleton<A, L, B>
    where
        I: Fn() -> A + 'a,
        F: Fn(&mut A, T),
    {
        let nondet = nondet!(/** the combinator function is commutative and idempotent */);
        self.assume_ordering(nondet)
            .assume_retries(nondet)
            .fold(init, comb)
    }

    /// Combines elements of the stream into an [`Optional`], by starting with the first element in the stream,
    /// and then applying the `comb` closure to each element in the stream. The [`Optional`] will be empty
    /// until the first element in the input arrives. Unlike iterators, `comb` takes the accumulator by `&mut`
    /// reference, so that it can be modified in place.
    ///
    /// The `comb` closure must be **commutative** AND **idempotent**, as the order of input items is not guaranteed
    /// and there may be duplicates.
    ///
    /// # Example
    /// ```rust
    /// # use hydro_lang::prelude::*;
    /// # use futures::StreamExt;
    /// # tokio_test::block_on(hydro_lang::test_util::stream_transform_test(|process| {
    /// let tick = process.tick();
    /// let bools = process.source_iter(q!(vec![false, true, false]));
    /// let batch = bools.batch(&tick, nondet!(/** test */));
    /// batch
    ///     .reduce_commutative_idempotent(q!(|acc, x| *acc |= x))
    ///     .all_ticks()
    /// # }, |mut stream| async move {
    /// // true
    /// # assert_eq!(stream.next().await.unwrap(), true);
    /// # }));
    /// ```
    pub fn reduce_commutative_idempotent<F>(
        self,
        comb: impl IntoQuotedMut<'a, F, L>,
    ) -> Optional<T, L, B>
    where
        F: Fn(&mut T, T) + 'a,
    {
        let nondet = nondet!(/** the combinator function is commutative and idempotent */);
        self.assume_ordering(nondet)
            .assume_retries(nondet)
            .reduce(comb)
    }

    // only for internal APIs that have been carefully vetted, will eventually be removed once we
    // have algebraic verification of these properties
    fn reduce_commutative_idempotent_trusted<F>(
        self,
        comb: impl IntoQuotedMut<'a, F, L>,
    ) -> Optional<T, L, B>
    where
        F: Fn(&mut T, T) + 'a,
    {
        let nondet = nondet!(/** the combinator function is commutative and idempotent */);

        let ordered = if B::BOUNDED {
            self.assume_ordering_trusted(nondet)
        } else {
            self.assume_ordering(nondet) // if unbounded, ordering affects intermediate states
        };

        ordered
            .assume_retries_trusted(nondet) // retries never affect intermediate states
            .reduce(comb)
    }

    /// Computes the maximum element in the stream as an [`Optional`], which
    /// will be empty until the first element in the input arrives.
    ///
    /// # Example
    /// ```rust
    /// # use hydro_lang::prelude::*;
    /// # use futures::StreamExt;
    /// # tokio_test::block_on(hydro_lang::test_util::stream_transform_test(|process| {
    /// let tick = process.tick();
    /// let numbers = process.source_iter(q!(vec![1, 2, 3, 4]));
    /// let batch = numbers.batch(&tick, nondet!(/** test */));
    /// batch.max().all_ticks()
    /// # }, |mut stream| async move {
    /// // 4
    /// # assert_eq!(stream.next().await.unwrap(), 4);
    /// # }));
    /// ```
    pub fn max(self) -> Optional<T, L, B>
    where
        T: Ord,
    {
        self.reduce_commutative_idempotent_trusted(q!(|curr, new| {
            if new > *curr {
                *curr = new;
            }
        }))
    }

    /// Computes the minimum element in the stream as an [`Optional`], which
    /// will be empty until the first element in the input arrives.
    ///
    /// # Example
    /// ```rust
    /// # use hydro_lang::prelude::*;
    /// # use futures::StreamExt;
    /// # tokio_test::block_on(hydro_lang::test_util::stream_transform_test(|process| {
    /// let tick = process.tick();
    /// let numbers = process.source_iter(q!(vec![1, 2, 3, 4]));
    /// let batch = numbers.batch(&tick, nondet!(/** test */));
    /// batch.min().all_ticks()
    /// # }, |mut stream| async move {
    /// // 1
    /// # assert_eq!(stream.next().await.unwrap(), 1);
    /// # }));
    /// ```
    pub fn min(self) -> Optional<T, L, B>
    where
        T: Ord,
    {
        self.reduce_commutative_idempotent_trusted(q!(|curr, new| {
            if new < *curr {
                *curr = new;
            }
        }))
    }
}

impl<'a, T, L, B: Boundedness, O: Ordering> Stream<T, L, B, O, ExactlyOnce>
where
    L: Location<'a>,
{
    /// Combines elements of the stream into a [`Singleton`], by starting with an initial value,
    /// generated by the `init` closure, and then applying the `comb` closure to each element in the stream.
    /// Unlike iterators, `comb` takes the accumulator by `&mut` reference, so that it can be modified in place.
    ///
    /// The `comb` closure must be **commutative**, as the order of input items is not guaranteed.
    ///
    /// # Example
    /// ```rust
    /// # use hydro_lang::prelude::*;
    /// # use futures::StreamExt;
    /// # tokio_test::block_on(hydro_lang::test_util::stream_transform_test(|process| {
    /// let tick = process.tick();
    /// let numbers = process.source_iter(q!(vec![1, 2, 3, 4]));
    /// let batch = numbers.batch(&tick, nondet!(/** test */));
    /// batch
    ///     .fold_commutative(q!(|| 0), q!(|acc, x| *acc += x))
    ///     .all_ticks()
    /// # }, |mut stream| async move {
    /// // 10
    /// # assert_eq!(stream.next().await.unwrap(), 10);
    /// # }));
    /// ```
    pub fn fold_commutative<A, I, F>(
        self,
        init: impl IntoQuotedMut<'a, I, L>,
        comb: impl IntoQuotedMut<'a, F, L>,
    ) -> Singleton<A, L, B>
    where
        I: Fn() -> A + 'a,
        F: Fn(&mut A, T),
    {
        let nondet = nondet!(/** the combinator function is commutative */);
        self.assume_ordering(nondet).fold(init, comb)
    }

    /// Combines elements of the stream into a [`Optional`], by starting with the first element in the stream,
    /// and then applying the `comb` closure to each element in the stream. The [`Optional`] will be empty
    /// until the first element in the input arrives. Unlike iterators, `comb` takes the accumulator by `&mut`
    /// reference, so that it can be modified in place.
    ///
    /// The `comb` closure must be **commutative**, as the order of input items is not guaranteed.
    ///
    /// # Example
    /// ```rust
    /// # use hydro_lang::prelude::*;
    /// # use futures::StreamExt;
    /// # tokio_test::block_on(hydro_lang::test_util::stream_transform_test(|process| {
    /// let tick = process.tick();
    /// let numbers = process.source_iter(q!(vec![1, 2, 3, 4]));
    /// let batch = numbers.batch(&tick, nondet!(/** test */));
    /// batch
    ///     .reduce_commutative(q!(|curr, new| *curr += new))
    ///     .all_ticks()
    /// # }, |mut stream| async move {
    /// // 10
    /// # assert_eq!(stream.next().await.unwrap(), 10);
    /// # }));
    /// ```
    pub fn reduce_commutative<F>(self, comb: impl IntoQuotedMut<'a, F, L>) -> Optional<T, L, B>
    where
        F: Fn(&mut T, T) + 'a,
    {
        let nondet = nondet!(/** the combinator function is commutative */);
        self.assume_ordering(nondet).reduce(comb)
    }

    /// Computes the number of elements in the stream as a [`Singleton`].
    ///
    /// # Example
    /// ```rust
    /// # use hydro_lang::prelude::*;
    /// # use futures::StreamExt;
    /// # tokio_test::block_on(hydro_lang::test_util::stream_transform_test(|process| {
    /// let tick = process.tick();
    /// let numbers = process.source_iter(q!(vec![1, 2, 3, 4]));
    /// let batch = numbers.batch(&tick, nondet!(/** test */));
    /// batch.count().all_ticks()
    /// # }, |mut stream| async move {
    /// // 4
    /// # assert_eq!(stream.next().await.unwrap(), 4);
    /// # }));
    /// ```
    pub fn count(self) -> Singleton<usize, L, B> {
        self.assume_ordering_trusted(nondet!(
            /// Order does not affect eventual count, and also does not affect intermediate states.
        ))
        .fold(q!(|| 0usize), q!(|count, _| *count += 1))
    }
}

impl<'a, T, L, B: Boundedness, R: Retries> Stream<T, L, B, TotalOrder, R>
where
    L: Location<'a>,
{
    /// Combines elements of the stream into a [`Singleton`], by starting with an initial value,
    /// generated by the `init` closure, and then applying the `comb` closure to each element in the stream.
    /// Unlike iterators, `comb` takes the accumulator by `&mut` reference, so that it can be modified in place.
    ///
    /// The `comb` closure must be **idempotent**, as there may be non-deterministic duplicates.
    ///
    /// # Example
    /// ```rust
    /// # use hydro_lang::prelude::*;
    /// # use futures::StreamExt;
    /// # tokio_test::block_on(hydro_lang::test_util::stream_transform_test(|process| {
    /// let tick = process.tick();
    /// let bools = process.source_iter(q!(vec![false, true, false]));
    /// let batch = bools.batch(&tick, nondet!(/** test */));
    /// batch
    ///     .fold_idempotent(q!(|| false), q!(|acc, x| *acc |= x))
    ///     .all_ticks()
    /// # }, |mut stream| async move {
    /// // true
    /// # assert_eq!(stream.next().await.unwrap(), true);
    /// # }));
    /// ```
    pub fn fold_idempotent<A, I, F>(
        self,
        init: impl IntoQuotedMut<'a, I, L>,
        comb: impl IntoQuotedMut<'a, F, L>,
    ) -> Singleton<A, L, B>
    where
        I: Fn() -> A + 'a,
        F: Fn(&mut A, T),
    {
        let nondet = nondet!(/** the combinator function is idempotent */);
        self.assume_retries(nondet).fold(init, comb)
    }

    /// Combines elements of the stream into an [`Optional`], by starting with the first element in the stream,
    /// and then applying the `comb` closure to each element in the stream. The [`Optional`] will be empty
    /// until the first element in the input arrives. Unlike iterators, `comb` takes the accumulator by `&mut`
    /// reference, so that it can be modified in place.
    ///
    /// The `comb` closure must be **idempotent**, as there may be non-deterministic duplicates.
    ///
    /// # Example
    /// ```rust
    /// # use hydro_lang::prelude::*;
    /// # use futures::StreamExt;
    /// # tokio_test::block_on(hydro_lang::test_util::stream_transform_test(|process| {
    /// let tick = process.tick();
    /// let bools = process.source_iter(q!(vec![false, true, false]));
    /// let batch = bools.batch(&tick, nondet!(/** test */));
    /// batch.reduce_idempotent(q!(|acc, x| *acc |= x)).all_ticks()
    /// # }, |mut stream| async move {
    /// // true
    /// # assert_eq!(stream.next().await.unwrap(), true);
    /// # }));
    /// ```
    pub fn reduce_idempotent<F>(self, comb: impl IntoQuotedMut<'a, F, L>) -> Optional<T, L, B>
    where
        F: Fn(&mut T, T) + 'a,
    {
        let nondet = nondet!(/** the combinator function is idempotent */);
        self.assume_retries(nondet).reduce(comb)
    }

    /// Computes the first element in the stream as an [`Optional`], which
    /// will be empty until the first element in the input arrives.
    ///
    /// This requires the stream to have a [`TotalOrder`] guarantee, otherwise
    /// re-ordering of elements may cause the first element to change.
    ///
    /// # Example
    /// ```rust
    /// # use hydro_lang::prelude::*;
    /// # use futures::StreamExt;
    /// # tokio_test::block_on(hydro_lang::test_util::stream_transform_test(|process| {
    /// let tick = process.tick();
    /// let numbers = process.source_iter(q!(vec![1, 2, 3, 4]));
    /// let batch = numbers.batch(&tick, nondet!(/** test */));
    /// batch.first().all_ticks()
    /// # }, |mut stream| async move {
    /// // 1
    /// # assert_eq!(stream.next().await.unwrap(), 1);
    /// # }));
    /// ```
    pub fn first(self) -> Optional<T, L, B> {
        self.reduce_idempotent(q!(|_, _| {}))
    }

    /// Computes the last element in the stream as an [`Optional`], which
    /// will be empty until an element in the input arrives.
    ///
    /// This requires the stream to have a [`TotalOrder`] guarantee, otherwise
    /// re-ordering of elements may cause the last element to change.
    ///
    /// # Example
    /// ```rust
    /// # use hydro_lang::prelude::*;
    /// # use futures::StreamExt;
    /// # tokio_test::block_on(hydro_lang::test_util::stream_transform_test(|process| {
    /// let tick = process.tick();
    /// let numbers = process.source_iter(q!(vec![1, 2, 3, 4]));
    /// let batch = numbers.batch(&tick, nondet!(/** test */));
    /// batch.last().all_ticks()
    /// # }, |mut stream| async move {
    /// // 4
    /// # assert_eq!(stream.next().await.unwrap(), 4);
    /// # }));
    /// ```
    pub fn last(self) -> Optional<T, L, B> {
        self.reduce_idempotent(q!(|curr, new| *curr = new))
    }
}

impl<'a, T, L, B: Boundedness> Stream<T, L, B, TotalOrder, ExactlyOnce>
where
    L: Location<'a>,
{
    /// Maps each element `x` of the stream to `(i, x)`, where `i` is the index of the element.
    ///
    /// # Example
    /// ```rust
    /// # use hydro_lang::{prelude::*, live_collections::stream::{TotalOrder, ExactlyOnce}};
    /// # use futures::StreamExt;
    /// # tokio_test::block_on(hydro_lang::test_util::stream_transform_test::<_, _, TotalOrder, ExactlyOnce>(|process| {
    /// let tick = process.tick();
    /// let numbers = process.source_iter(q!(vec![1, 2, 3, 4]));
    /// numbers.enumerate()
    /// # }, |mut stream| async move {
    /// // (0, 1), (1, 2), (2, 3), (3, 4)
    /// # for w in vec![(0, 1), (1, 2), (2, 3), (3, 4)] {
    /// #     assert_eq!(stream.next().await.unwrap(), w);
    /// # }
    /// # }));
    /// ```
    pub fn enumerate(self) -> Stream<(usize, T), L, B, TotalOrder, ExactlyOnce> {
        Stream::new(
            self.location.clone(),
            HydroNode::Enumerate {
                input: Box::new(self.ir_node.into_inner()),
                metadata: self.location.new_node_metadata(Stream::<
                    (usize, T),
                    L,
                    B,
                    TotalOrder,
                    ExactlyOnce,
                >::collection_kind()),
            },
        )
    }

    /// Combines elements of the stream into a [`Singleton`], by starting with an intitial value,
    /// generated by the `init` closure, and then applying the `comb` closure to each element in the stream.
    /// Unlike iterators, `comb` takes the accumulator by `&mut` reference, so that it can be modified in place.
    ///
    /// The input stream must have a [`TotalOrder`] guarantee, which means that the `comb` closure is allowed
    /// to depend on the order of elements in the stream.
    ///
    /// # Example
    /// ```rust
    /// # use hydro_lang::prelude::*;
    /// # use futures::StreamExt;
    /// # tokio_test::block_on(hydro_lang::test_util::stream_transform_test(|process| {
    /// let tick = process.tick();
    /// let words = process.source_iter(q!(vec!["HELLO", "WORLD"]));
    /// let batch = words.batch(&tick, nondet!(/** test */));
    /// batch
    ///     .fold(q!(|| String::new()), q!(|acc, x| acc.push_str(x)))
    ///     .all_ticks()
    /// # }, |mut stream| async move {
    /// // "HELLOWORLD"
    /// # assert_eq!(stream.next().await.unwrap(), "HELLOWORLD");
    /// # }));
    /// ```
    pub fn fold<A, I: Fn() -> A + 'a, F: Fn(&mut A, T)>(
        self,
        init: impl IntoQuotedMut<'a, I, L>,
        comb: impl IntoQuotedMut<'a, F, L>,
    ) -> Singleton<A, L, B> {
        let init = init.splice_fn0_ctx(&self.location).into();
        let comb = comb.splice_fn2_borrow_mut_ctx(&self.location).into();

        let core = HydroNode::Fold {
            init,
            acc: comb,
            input: Box::new(self.ir_node.into_inner()),
            metadata: self
                .location
                .new_node_metadata(Singleton::<A, L, B>::collection_kind()),
        };

        Singleton::new(self.location, core)
    }

    /// Collects all the elements of this stream into a single [`Vec`] element.
    ///
    /// If the input stream is [`Unbounded`], the output [`Singleton`] will be [`Unbounded`] as
    /// well, which means that the value of the [`Vec`] will asynchronously grow as new elements
    /// are added. On such a value, you can use [`Singleton::snapshot`] to grab an instance of
    /// the vector at an arbitrary point in time.
    ///
    /// # Example
    /// ```rust
    /// # use hydro_lang::prelude::*;
    /// # use futures::StreamExt;
    /// # tokio_test::block_on(hydro_lang::test_util::stream_transform_test(|process| {
    /// let tick = process.tick();
    /// let numbers = process.source_iter(q!(vec![1, 2, 3, 4]));
    /// let batch = numbers.batch(&tick, nondet!(/** test */));
    /// batch.collect_vec().all_ticks() // emit each tick's Vec into an unbounded stream
    /// # }, |mut stream| async move {
    /// // [ vec![1, 2, 3, 4] ]
    /// # for w in vec![vec![1, 2, 3, 4]] {
    /// #     assert_eq!(stream.next().await.unwrap(), w);
    /// # }
    /// # }));
    /// ```
    pub fn collect_vec(self) -> Singleton<Vec<T>, L, B> {
        self.fold(
            q!(|| vec![]),
            q!(|acc, v| {
                acc.push(v);
            }),
        )
    }

    /// Applies a function to each element of the stream, maintaining an internal state (accumulator)
    /// and emitting each intermediate result.
    ///
    /// Unlike `fold` which only returns the final accumulated value, `scan` produces a new stream
    /// containing all intermediate accumulated values. The scan operation can also terminate early
    /// by returning `None`.
    ///
    /// The function takes a mutable reference to the accumulator and the current element, and returns
    /// an `Option<U>`. If the function returns `Some(value)`, `value` is emitted to the output stream.
    /// If the function returns `None`, the stream is terminated and no more elements are processed.
    ///
    /// # Examples
    ///
    /// Basic usage - running sum:
    /// ```rust
    /// # use hydro_lang::prelude::*;
    /// # use futures::StreamExt;
    /// # tokio_test::block_on(hydro_lang::test_util::stream_transform_test(|process| {
    /// process.source_iter(q!(vec![1, 2, 3, 4])).scan(
    ///     q!(|| 0),
    ///     q!(|acc, x| {
    ///         *acc += x;
    ///         Some(*acc)
    ///     }),
    /// )
    /// # }, |mut stream| async move {
    /// // Output: 1, 3, 6, 10
    /// # for w in vec![1, 3, 6, 10] {
    /// #     assert_eq!(stream.next().await.unwrap(), w);
    /// # }
    /// # }));
    /// ```
    ///
    /// Early termination example:
    /// ```rust
    /// # use hydro_lang::prelude::*;
    /// # use futures::StreamExt;
    /// # tokio_test::block_on(hydro_lang::test_util::stream_transform_test(|process| {
    /// process.source_iter(q!(vec![1, 2, 3, 4])).scan(
    ///     q!(|| 1),
    ///     q!(|state, x| {
    ///         *state = *state * x;
    ///         if *state > 6 {
    ///             None // Terminate the stream
    ///         } else {
    ///             Some(-*state)
    ///         }
    ///     }),
    /// )
    /// # }, |mut stream| async move {
    /// // Output: -1, -2, -6
    /// # for w in vec![-1, -2, -6] {
    /// #     assert_eq!(stream.next().await.unwrap(), w);
    /// # }
    /// # }));
    /// ```
    pub fn scan<A, U, I, F>(
        self,
        init: impl IntoQuotedMut<'a, I, L>,
        f: impl IntoQuotedMut<'a, F, L>,
    ) -> Stream<U, L, B, TotalOrder, ExactlyOnce>
    where
        I: Fn() -> A + 'a,
        F: Fn(&mut A, T) -> Option<U> + 'a,
    {
        let init = init.splice_fn0_ctx(&self.location).into();
        let f = f.splice_fn2_borrow_mut_ctx(&self.location).into();

        Stream::new(
            self.location.clone(),
            HydroNode::Scan {
                init,
                acc: f,
                input: Box::new(self.ir_node.into_inner()),
                metadata: self.location.new_node_metadata(
                    Stream::<U, L, B, TotalOrder, ExactlyOnce>::collection_kind(),
                ),
            },
        )
    }

    /// Combines elements of the stream into an [`Optional`], by starting with the first element in the stream,
    /// and then applying the `comb` closure to each element in the stream. The [`Optional`] will be empty
    /// until the first element in the input arrives.
    ///
    /// The input stream must have a [`TotalOrder`] guarantee, which means that the `comb` closure is allowed
    /// to depend on the order of elements in the stream.
    ///
    /// # Example
    /// ```rust
    /// # use hydro_lang::prelude::*;
    /// # use futures::StreamExt;
    /// # tokio_test::block_on(hydro_lang::test_util::stream_transform_test(|process| {
    /// let tick = process.tick();
    /// let words = process.source_iter(q!(vec!["HELLO", "WORLD"]));
    /// let batch = words.batch(&tick, nondet!(/** test */));
    /// batch
    ///     .map(q!(|x| x.to_string()))
    ///     .reduce(q!(|curr, new| curr.push_str(&new)))
    ///     .all_ticks()
    /// # }, |mut stream| async move {
    /// // "HELLOWORLD"
    /// # assert_eq!(stream.next().await.unwrap(), "HELLOWORLD");
    /// # }));
    /// ```
    pub fn reduce<F: Fn(&mut T, T) + 'a>(
        self,
        comb: impl IntoQuotedMut<'a, F, L>,
    ) -> Optional<T, L, B> {
        let f = comb.splice_fn2_borrow_mut_ctx(&self.location).into();
        let core = HydroNode::Reduce {
            f,
            input: Box::new(self.ir_node.into_inner()),
            metadata: self
                .location
                .new_node_metadata(Optional::<T, L, B>::collection_kind()),
        };

        Optional::new(self.location, core)
    }
}

impl<'a, T, L: Location<'a> + NoTick, O: Ordering, R: Retries> Stream<T, L, Unbounded, O, R> {
    /// Produces a new stream that interleaves the elements of the two input streams.
    /// The result has [`NoOrder`] because the order of interleaving is not guaranteed.
    ///
    /// Currently, both input streams must be [`Unbounded`]. When the streams are
    /// [`Bounded`], you can use [`Stream::chain`] instead.
    ///
    /// # Example
    /// ```rust
    /// # use hydro_lang::prelude::*;
    /// # use futures::StreamExt;
    /// # tokio_test::block_on(hydro_lang::test_util::stream_transform_test(|process| {
    /// let numbers = process.source_iter(q!(vec![1, 2, 3, 4]));
    /// numbers.clone().map(q!(|x| x + 1)).interleave(numbers)
    /// # }, |mut stream| async move {
    /// // 2, 3, 4, 5, and 1, 2, 3, 4 interleaved in unknown order
    /// # for w in vec![2, 3, 4, 5, 1, 2, 3, 4] {
    /// #     assert_eq!(stream.next().await.unwrap(), w);
    /// # }
    /// # }));
    /// ```
    pub fn interleave<O2: Ordering, R2: Retries>(
        self,
        other: Stream<T, L, Unbounded, O2, R2>,
    ) -> Stream<T, L, Unbounded, NoOrder, <R as MinRetries<R2>>::Min>
    where
        R: MinRetries<R2>,
    {
        Stream::new(
            self.location.clone(),
            HydroNode::Chain {
                first: Box::new(self.ir_node.into_inner()),
                second: Box::new(other.ir_node.into_inner()),
                metadata: self.location.new_node_metadata(Stream::<
                    T,
                    L,
                    Unbounded,
                    NoOrder,
                    <R as MinRetries<R2>>::Min,
                >::collection_kind()),
            },
        )
    }
}

impl<'a, T, L, O: Ordering, R: Retries> Stream<T, L, Bounded, O, R>
where
    L: Location<'a>,
{
    /// Produces a new stream that emits the input elements in sorted order.
    ///
    /// The input stream can have any ordering guarantee, but the output stream
    /// will have a [`TotalOrder`] guarantee. This operator will block until all
    /// elements in the input stream are available, so it requires the input stream
    /// to be [`Bounded`].
    ///
    /// # Example
    /// ```rust
    /// # use hydro_lang::prelude::*;
    /// # use futures::StreamExt;
    /// # tokio_test::block_on(hydro_lang::test_util::stream_transform_test(|process| {
    /// let tick = process.tick();
    /// let numbers = process.source_iter(q!(vec![4, 2, 3, 1]));
    /// let batch = numbers.batch(&tick, nondet!(/** test */));
    /// batch.sort().all_ticks()
    /// # }, |mut stream| async move {
    /// // 1, 2, 3, 4
    /// # for w in (1..5) {
    /// #     assert_eq!(stream.next().await.unwrap(), w);
    /// # }
    /// # }));
    /// ```
    pub fn sort(self) -> Stream<T, L, Bounded, TotalOrder, R>
    where
        T: Ord,
    {
        Stream::new(
            self.location.clone(),
            HydroNode::Sort {
                input: Box::new(self.ir_node.into_inner()),
                metadata: self
                    .location
                    .new_node_metadata(Stream::<T, L, Bounded, TotalOrder, R>::collection_kind()),
            },
        )
    }

    /// Produces a new stream that first emits the elements of the `self` stream,
    /// and then emits the elements of the `other` stream. The output stream has
    /// a [`TotalOrder`] guarantee if and only if both input streams have a
    /// [`TotalOrder`] guarantee.
    ///
    /// Currently, both input streams must be [`Bounded`]. This operator will block
    /// on the first stream until all its elements are available. In a future version,
    /// we will relax the requirement on the `other` stream.
    ///
    /// # Example
    /// ```rust
    /// # use hydro_lang::prelude::*;
    /// # use futures::StreamExt;
    /// # tokio_test::block_on(hydro_lang::test_util::stream_transform_test(|process| {
    /// let tick = process.tick();
    /// let numbers = process.source_iter(q!(vec![1, 2, 3, 4]));
    /// let batch = numbers.batch(&tick, nondet!(/** test */));
    /// batch.clone().map(q!(|x| x + 1)).chain(batch).all_ticks()
    /// # }, |mut stream| async move {
    /// // 2, 3, 4, 5, 1, 2, 3, 4
    /// # for w in vec![2, 3, 4, 5, 1, 2, 3, 4] {
    /// #     assert_eq!(stream.next().await.unwrap(), w);
    /// # }
    /// # }));
    /// ```
    pub fn chain<O2: Ordering, R2: Retries>(
        self,
        other: Stream<T, L, Bounded, O2, R2>,
    ) -> Stream<T, L, Bounded, <O as MinOrder<O2>>::Min, <R as MinRetries<R2>>::Min>
    where
        O: MinOrder<O2>,
        R: MinRetries<R2>,
    {
        check_matching_location(&self.location, &other.location);

        Stream::new(
            self.location.clone(),
            HydroNode::Chain {
                first: Box::new(self.ir_node.into_inner()),
                second: Box::new(other.ir_node.into_inner()),
                metadata: self.location.new_node_metadata(Stream::<
                    T,
                    L,
                    Bounded,
                    <O as MinOrder<O2>>::Min,
                    <R as MinRetries<R2>>::Min,
                >::collection_kind()),
            },
        )
    }

    /// Forms the cross-product (Cartesian product, cross-join) of the items in the 2 input streams.
    /// Unlike [`Stream::cross_product`], the output order is totally ordered when the inputs are
    /// because this is compiled into a nested loop.
    pub fn cross_product_nested_loop<T2, O2: Ordering + MinOrder<O>>(
        self,
        other: Stream<T2, L, Bounded, O2, R>,
    ) -> Stream<(T, T2), L, Bounded, <O2 as MinOrder<O>>::Min, R>
    where
        T: Clone,
        T2: Clone,
    {
        check_matching_location(&self.location, &other.location);

        Stream::new(
            self.location.clone(),
            HydroNode::CrossProduct {
                left: Box::new(self.ir_node.into_inner()),
                right: Box::new(other.ir_node.into_inner()),
                metadata: self.location.new_node_metadata(Stream::<
                    (T, T2),
                    L,
                    Bounded,
                    <O2 as MinOrder<O>>::Min,
                    R,
                >::collection_kind()),
            },
        )
    }

    /// Creates a [`KeyedStream`] with the same set of keys as `keys`, but with the elements in
    /// `self` used as the values for *each* key.
    ///
    /// This is helpful when "broadcasting" a set of values so that all the keys have the same
    /// values. For example, it can be used to send the same set of elements to several cluster
    /// members, if the membership information is available as a [`KeyedSingleton`].
    ///
    /// # Example
    /// ```rust
    /// # use hydro_lang::prelude::*;
    /// # use futures::StreamExt;
    /// # tokio_test::block_on(hydro_lang::test_util::stream_transform_test(|process| {
    /// # let tick = process.tick();
    /// let keyed_singleton = // { 1: (), 2: () }
    /// # process
    /// #     .source_iter(q!(vec![(1, ()), (2, ())]))
    /// #     .into_keyed()
    /// #     .batch(&tick, nondet!(/** test */))
    /// #     .first();
    /// let stream = // [ "a", "b" ]
    /// # process
    /// #     .source_iter(q!(vec!["a".to_string(), "b".to_string()]))
    /// #     .batch(&tick, nondet!(/** test */));
    /// stream.repeat_with_keys(keyed_singleton)
    /// # .entries().all_ticks()
    /// # }, |mut stream| async move {
    /// // { 1: ["a", "b" ], 2: ["a", "b"] }
    /// # let mut results = Vec::new();
    /// # for _ in 0..4 {
    /// #     results.push(stream.next().await.unwrap());
    /// # }
    /// # results.sort();
    /// # assert_eq!(results, vec![(1, "a".to_string()), (1, "b".to_string()), (2, "a".to_string()), (2, "b".to_string())]);
    /// # }));
    /// ```
    pub fn repeat_with_keys<K, V2>(
        self,
        keys: KeyedSingleton<K, V2, L, Bounded>,
    ) -> KeyedStream<K, T, L, Bounded, O, R>
    where
        K: Clone,
        T: Clone,
    {
        keys.keys()
            .weaken_retries()
            .assume_ordering_trusted::<TotalOrder>(
                nondet!(/** keyed stream does not depend on ordering of keys */),
            )
            .cross_product_nested_loop(self)
            .into_keyed()
    }
}

impl<'a, K, V1, L, B: Boundedness, O: Ordering, R: Retries> Stream<(K, V1), L, B, O, R>
where
    L: Location<'a>,
{
    #[expect(clippy::type_complexity, reason = "ordering / retries propagation")]
    /// Given two streams of pairs `(K, V1)` and `(K, V2)`, produces a new stream of nested pairs `(K, (V1, V2))`
    /// by equi-joining the two streams on the key attribute `K`.
    ///
    /// # Example
    /// ```rust
    /// # use hydro_lang::prelude::*;
    /// # use std::collections::HashSet;
    /// # use futures::StreamExt;
    /// # tokio_test::block_on(hydro_lang::test_util::stream_transform_test(|process| {
    /// let tick = process.tick();
    /// let stream1 = process.source_iter(q!(vec![(1, 'a'), (2, 'b')]));
    /// let stream2 = process.source_iter(q!(vec![(1, 'x'), (2, 'y')]));
    /// stream1.join(stream2)
    /// # }, |mut stream| async move {
    /// // (1, ('a', 'x')), (2, ('b', 'y'))
    /// # let expected = HashSet::from([(1, ('a', 'x')), (2, ('b', 'y'))]);
    /// # stream.map(|i| assert!(expected.contains(&i)));
    /// # }));
    pub fn join<V2, O2: Ordering, R2: Retries>(
        self,
        n: Stream<(K, V2), L, B, O2, R2>,
    ) -> Stream<(K, (V1, V2)), L, B, NoOrder, <R as MinRetries<R2>>::Min>
    where
        K: Eq + Hash,
        R: MinRetries<R2>,
    {
        check_matching_location(&self.location, &n.location);

        Stream::new(
            self.location.clone(),
            HydroNode::Join {
                left: Box::new(self.ir_node.into_inner()),
                right: Box::new(n.ir_node.into_inner()),
                metadata: self.location.new_node_metadata(Stream::<
                    (K, (V1, V2)),
                    L,
                    B,
                    NoOrder,
                    <R as MinRetries<R2>>::Min,
                >::collection_kind()),
            },
        )
    }

    /// Given a stream of pairs `(K, V1)` and a bounded stream of keys `K`,
    /// computes the anti-join of the items in the input -- i.e. returns
    /// unique items in the first input that do not have a matching key
    /// in the second input.
    ///
    /// # Example
    /// ```rust
    /// # use hydro_lang::prelude::*;
    /// # use futures::StreamExt;
    /// # tokio_test::block_on(hydro_lang::test_util::stream_transform_test(|process| {
    /// let tick = process.tick();
    /// let stream = process
    ///   .source_iter(q!(vec![ (1, 'a'), (2, 'b'), (3, 'c'), (4, 'd') ]))
    ///   .batch(&tick, nondet!(/** test */));
    /// let batch = process
    ///   .source_iter(q!(vec![1, 2]))
    ///   .batch(&tick, nondet!(/** test */));
    /// stream.anti_join(batch).all_ticks()
    /// # }, |mut stream| async move {
    /// # for w in vec![(3, 'c'), (4, 'd')] {
    /// #     assert_eq!(stream.next().await.unwrap(), w);
    /// # }
    /// # }));
    pub fn anti_join<O2: Ordering, R2: Retries>(
        self,
        n: Stream<K, L, Bounded, O2, R2>,
    ) -> Stream<(K, V1), L, B, O, R>
    where
        K: Eq + Hash,
    {
        check_matching_location(&self.location, &n.location);

        Stream::new(
            self.location.clone(),
            HydroNode::AntiJoin {
                pos: Box::new(self.ir_node.into_inner()),
                neg: Box::new(n.ir_node.into_inner()),
                metadata: self
                    .location
                    .new_node_metadata(Stream::<(K, V1), L, B, O, R>::collection_kind()),
            },
        )
    }
}

impl<'a, K, V, L: Location<'a>, B: Boundedness, O: Ordering, R: Retries>
    Stream<(K, V), L, B, O, R>
{
    /// Transforms this stream into a [`KeyedStream`], where the first element of each tuple
    /// is used as the key and the second element is added to the entries associated with that key.
    ///
    /// Because [`KeyedStream`] lazily groups values into buckets, this operator has zero computational
    /// cost and _does not_ require that the key type is hashable. Keyed streams are useful for
    /// performing grouped aggregations, but also for more precise ordering guarantees such as
    /// total ordering _within_ each group but no ordering _across_ groups.
    ///
    /// # Example
    /// ```rust
    /// # use hydro_lang::prelude::*;
    /// # use futures::StreamExt;
    /// # tokio_test::block_on(hydro_lang::test_util::stream_transform_test(|process| {
    /// process
    ///     .source_iter(q!(vec![(1, 2), (1, 3), (2, 4)]))
    ///     .into_keyed()
    /// #   .entries()
    /// # }, |mut stream| async move {
    /// // { 1: [2, 3], 2: [4] }
    /// # for w in vec![(1, 2), (1, 3), (2, 4)] {
    /// #     assert_eq!(stream.next().await.unwrap(), w);
    /// # }
    /// # }));
    /// ```
    pub fn into_keyed(self) -> KeyedStream<K, V, L, B, O, R> {
        KeyedStream::new(
            self.location.clone(),
            HydroNode::Cast {
                inner: Box::new(self.ir_node.into_inner()),
                metadata: self
                    .location
                    .new_node_metadata(KeyedStream::<K, V, L, B, O, R>::collection_kind()),
            },
        )
    }
}

impl<'a, K, V, L> Stream<(K, V), Tick<L>, Bounded, TotalOrder, ExactlyOnce>
where
    K: Eq + Hash,
    L: Location<'a>,
{
    #[deprecated = "use .into_keyed().fold(...) instead"]
    /// A special case of [`Stream::fold`], in the spirit of SQL's GROUP BY and aggregation constructs. The input
    /// tuples are partitioned into groups by the first element ("keys"), and for each group the values
    /// in the second element are accumulated via the `comb` closure.
    ///
    /// The input stream must have a [`TotalOrder`] guarantee, which means that the `comb` closure is allowed
    /// to depend on the order of elements in the stream.
    ///
    /// If the input and output value types are the same and do not require initialization then use
    /// [`Stream::reduce_keyed`].
    ///
    /// # Example
    /// ```rust
    /// # use hydro_lang::prelude::*;
    /// # use futures::StreamExt;
    /// # tokio_test::block_on(hydro_lang::test_util::stream_transform_test(|process| {
    /// let tick = process.tick();
    /// let numbers = process.source_iter(q!(vec![(1, 2), (2, 3), (1, 3), (2, 4)]));
    /// let batch = numbers.batch(&tick, nondet!(/** test */));
    /// batch
    ///     .fold_keyed(q!(|| 0), q!(|acc, x| *acc += x))
    ///     .all_ticks()
    /// # }, |mut stream| async move {
    /// // (1, 5), (2, 7)
    /// # assert_eq!(stream.next().await.unwrap(), (1, 5));
    /// # assert_eq!(stream.next().await.unwrap(), (2, 7));
    /// # }));
    /// ```
    pub fn fold_keyed<A, I, F>(
        self,
        init: impl IntoQuotedMut<'a, I, Tick<L>>,
        comb: impl IntoQuotedMut<'a, F, Tick<L>>,
    ) -> Stream<(K, A), Tick<L>, Bounded, NoOrder, ExactlyOnce>
    where
        I: Fn() -> A + 'a,
        F: Fn(&mut A, V) + 'a,
    {
        self.into_keyed().fold(init, comb).entries()
    }

    #[deprecated = "use .into_keyed().reduce(...) instead"]
    /// A special case of [`Stream::reduce`], in the spirit of SQL's GROUP BY and aggregation constructs. The input
    /// tuples are partitioned into groups by the first element ("keys"), and for each group the values
    /// in the second element are accumulated via the `comb` closure.
    ///
    /// The input stream must have a [`TotalOrder`] guarantee, which means that the `comb` closure is allowed
    /// to depend on the order of elements in the stream.
    ///
    /// If you need the accumulated value to have a different type than the input, use [`Stream::fold_keyed`].
    ///
    /// # Example
    /// ```rust
    /// # use hydro_lang::prelude::*;
    /// # use futures::StreamExt;
    /// # tokio_test::block_on(hydro_lang::test_util::stream_transform_test(|process| {
    /// let tick = process.tick();
    /// let numbers = process.source_iter(q!(vec![(1, 2), (2, 3), (1, 3), (2, 4)]));
    /// let batch = numbers.batch(&tick, nondet!(/** test */));
    /// batch.reduce_keyed(q!(|acc, x| *acc += x)).all_ticks()
    /// # }, |mut stream| async move {
    /// // (1, 5), (2, 7)
    /// # assert_eq!(stream.next().await.unwrap(), (1, 5));
    /// # assert_eq!(stream.next().await.unwrap(), (2, 7));
    /// # }));
    /// ```
    pub fn reduce_keyed<F>(
        self,
        comb: impl IntoQuotedMut<'a, F, Tick<L>>,
    ) -> Stream<(K, V), Tick<L>, Bounded, NoOrder, ExactlyOnce>
    where
        F: Fn(&mut V, V) + 'a,
    {
        let f = comb.splice_fn2_borrow_mut_ctx(&self.location).into();

        Stream::new(
            self.location.clone(),
            HydroNode::ReduceKeyed {
                f,
                input: Box::new(self.ir_node.into_inner()),
                metadata: self.location.new_node_metadata(Stream::<
                    (K, V),
                    Tick<L>,
                    Bounded,
                    NoOrder,
                    ExactlyOnce,
                >::collection_kind()),
            },
        )
    }
}

impl<'a, K, V, L, O: Ordering, R: Retries> Stream<(K, V), Tick<L>, Bounded, O, R>
where
    K: Eq + Hash,
    L: Location<'a>,
{
    #[deprecated = "use .into_keyed().fold_commutative_idempotent(...) instead"]
    /// A special case of [`Stream::fold_commutative_idempotent`], in the spirit of SQL's GROUP BY and aggregation constructs.
    /// The input tuples are partitioned into groups by the first element ("keys"), and for each group the values
    /// in the second element are accumulated via the `comb` closure.
    ///
    /// The `comb` closure must be **commutative**, as the order of input items is not guaranteed, and **idempotent**,
    /// as there may be non-deterministic duplicates.
    ///
    /// If the input and output value types are the same and do not require initialization then use
    /// [`Stream::reduce_keyed_commutative_idempotent`].
    ///
    /// # Example
    /// ```rust
    /// # use hydro_lang::prelude::*;
    /// # use futures::StreamExt;
    /// # tokio_test::block_on(hydro_lang::test_util::stream_transform_test(|process| {
    /// let tick = process.tick();
    /// let numbers = process.source_iter(q!(vec![(1, false), (2, true), (1, false), (2, false)]));
    /// let batch = numbers.batch(&tick, nondet!(/** test */));
    /// batch
    ///     .fold_keyed_commutative_idempotent(q!(|| false), q!(|acc, x| *acc |= x))
    ///     .all_ticks()
    /// # }, |mut stream| async move {
    /// // (1, false), (2, true)
    /// # assert_eq!(stream.next().await.unwrap(), (1, false));
    /// # assert_eq!(stream.next().await.unwrap(), (2, true));
    /// # }));
    /// ```
    pub fn fold_keyed_commutative_idempotent<A, I, F>(
        self,
        init: impl IntoQuotedMut<'a, I, Tick<L>>,
        comb: impl IntoQuotedMut<'a, F, Tick<L>>,
    ) -> Stream<(K, A), Tick<L>, Bounded, NoOrder, ExactlyOnce>
    where
        I: Fn() -> A + 'a,
        F: Fn(&mut A, V) + 'a,
    {
        self.into_keyed()
            .fold_commutative_idempotent(init, comb)
            .entries()
    }

    /// Given a stream of pairs `(K, V)`, produces a new stream of unique keys `K`.
    /// # Example
    /// ```rust
    /// # use hydro_lang::prelude::*;
    /// # use futures::StreamExt;
    /// # tokio_test::block_on(hydro_lang::test_util::stream_transform_test(|process| {
    /// let tick = process.tick();
    /// let numbers = process.source_iter(q!(vec![(1, 2), (2, 3), (1, 3), (2, 4)]));
    /// let batch = numbers.batch(&tick, nondet!(/** test */));
    /// batch.keys().all_ticks()
    /// # }, |mut stream| async move {
    /// // 1, 2
    /// # assert_eq!(stream.next().await.unwrap(), 1);
    /// # assert_eq!(stream.next().await.unwrap(), 2);
    /// # }));
    /// ```
    pub fn keys(self) -> Stream<K, Tick<L>, Bounded, NoOrder, ExactlyOnce> {
        self.into_keyed()
            .fold_commutative_idempotent(q!(|| ()), q!(|_, _| {}))
            .keys()
    }

    #[deprecated = "use .into_keyed().reduce_commutative_idempotent(...) instead"]
    /// A special case of [`Stream::reduce_commutative_idempotent`], in the spirit of SQL's GROUP BY and aggregation constructs.
    /// The input tuples are partitioned into groups by the first element ("keys"), and for each group the values
    /// in the second element are accumulated via the `comb` closure.
    ///
    /// The `comb` closure must be **commutative**, as the order of input items is not guaranteed, and **idempotent**,
    /// as there may be non-deterministic duplicates.
    ///
    /// If you need the accumulated value to have a different type than the input, use [`Stream::fold_keyed_commutative_idempotent`].
    ///
    /// # Example
    /// ```rust
    /// # use hydro_lang::prelude::*;
    /// # use futures::StreamExt;
    /// # tokio_test::block_on(hydro_lang::test_util::stream_transform_test(|process| {
    /// let tick = process.tick();
    /// let numbers = process.source_iter(q!(vec![(1, false), (2, true), (1, false), (2, false)]));
    /// let batch = numbers.batch(&tick, nondet!(/** test */));
    /// batch
    ///     .reduce_keyed_commutative_idempotent(q!(|acc, x| *acc |= x))
    ///     .all_ticks()
    /// # }, |mut stream| async move {
    /// // (1, false), (2, true)
    /// # assert_eq!(stream.next().await.unwrap(), (1, false));
    /// # assert_eq!(stream.next().await.unwrap(), (2, true));
    /// # }));
    /// ```
    pub fn reduce_keyed_commutative_idempotent<F>(
        self,
        comb: impl IntoQuotedMut<'a, F, Tick<L>>,
    ) -> Stream<(K, V), Tick<L>, Bounded, NoOrder, ExactlyOnce>
    where
        F: Fn(&mut V, V) + 'a,
    {
        self.into_keyed()
            .reduce_commutative_idempotent(comb)
            .entries()
    }
}

impl<'a, K, V, L, O: Ordering> Stream<(K, V), Tick<L>, Bounded, O, ExactlyOnce>
where
    K: Eq + Hash,
    L: Location<'a>,
{
    #[deprecated = "use .into_keyed().fold_commutative(...) instead"]
    /// A special case of [`Stream::fold_commutative`], in the spirit of SQL's GROUP BY and aggregation constructs. The input
    /// tuples are partitioned into groups by the first element ("keys"), and for each group the values
    /// in the second element are accumulated via the `comb` closure.
    ///
    /// The `comb` closure must be **commutative**, as the order of input items is not guaranteed.
    ///
    /// If the input and output value types are the same and do not require initialization then use
    /// [`Stream::reduce_keyed_commutative`].
    ///
    /// # Example
    /// ```rust
    /// # use hydro_lang::prelude::*;
    /// # use futures::StreamExt;
    /// # tokio_test::block_on(hydro_lang::test_util::stream_transform_test(|process| {
    /// let tick = process.tick();
    /// let numbers = process.source_iter(q!(vec![(1, 2), (2, 3), (1, 3), (2, 4)]));
    /// let batch = numbers.batch(&tick, nondet!(/** test */));
    /// batch
    ///     .fold_keyed_commutative(q!(|| 0), q!(|acc, x| *acc += x))
    ///     .all_ticks()
    /// # }, |mut stream| async move {
    /// // (1, 5), (2, 7)
    /// # assert_eq!(stream.next().await.unwrap(), (1, 5));
    /// # assert_eq!(stream.next().await.unwrap(), (2, 7));
    /// # }));
    /// ```
    pub fn fold_keyed_commutative<A, I, F>(
        self,
        init: impl IntoQuotedMut<'a, I, Tick<L>>,
        comb: impl IntoQuotedMut<'a, F, Tick<L>>,
    ) -> Stream<(K, A), Tick<L>, Bounded, NoOrder, ExactlyOnce>
    where
        I: Fn() -> A + 'a,
        F: Fn(&mut A, V) + 'a,
    {
        self.into_keyed().fold_commutative(init, comb).entries()
    }

    #[deprecated = "use .into_keyed().reduce_commutative(...) instead"]
    /// A special case of [`Stream::reduce_commutative`], in the spirit of SQL's GROUP BY and aggregation constructs. The input
    /// tuples are partitioned into groups by the first element ("keys"), and for each group the values
    /// in the second element are accumulated via the `comb` closure.
    ///
    /// The `comb` closure must be **commutative**, as the order of input items is not guaranteed.
    ///
    /// If you need the accumulated value to have a different type than the input, use [`Stream::fold_keyed_commutative`].
    ///
    /// # Example
    /// ```rust
    /// # use hydro_lang::prelude::*;
    /// # use futures::StreamExt;
    /// # tokio_test::block_on(hydro_lang::test_util::stream_transform_test(|process| {
    /// let tick = process.tick();
    /// let numbers = process.source_iter(q!(vec![(1, 2), (2, 3), (1, 3), (2, 4)]));
    /// let batch = numbers.batch(&tick, nondet!(/** test */));
    /// batch
    ///     .reduce_keyed_commutative(q!(|acc, x| *acc += x))
    ///     .all_ticks()
    /// # }, |mut stream| async move {
    /// // (1, 5), (2, 7)
    /// # assert_eq!(stream.next().await.unwrap(), (1, 5));
    /// # assert_eq!(stream.next().await.unwrap(), (2, 7));
    /// # }));
    /// ```
    pub fn reduce_keyed_commutative<F>(
        self,
        comb: impl IntoQuotedMut<'a, F, Tick<L>>,
    ) -> Stream<(K, V), Tick<L>, Bounded, NoOrder, ExactlyOnce>
    where
        F: Fn(&mut V, V) + 'a,
    {
        self.into_keyed().reduce_commutative(comb).entries()
    }
}

impl<'a, K, V, L, R: Retries> Stream<(K, V), Tick<L>, Bounded, TotalOrder, R>
where
    K: Eq + Hash,
    L: Location<'a>,
{
    #[deprecated = "use .into_keyed().fold_idempotent(...) instead"]
    /// A special case of [`Stream::fold_idempotent`], in the spirit of SQL's GROUP BY and aggregation constructs.
    /// The input tuples are partitioned into groups by the first element ("keys"), and for each group the values
    /// in the second element are accumulated via the `comb` closure.
    ///
    /// The `comb` closure must be **idempotent** as there may be non-deterministic duplicates.
    ///
    /// If the input and output value types are the same and do not require initialization then use
    /// [`Stream::reduce_keyed_idempotent`].
    ///
    /// # Example
    /// ```rust
    /// # use hydro_lang::prelude::*;
    /// # use futures::StreamExt;
    /// # tokio_test::block_on(hydro_lang::test_util::stream_transform_test(|process| {
    /// let tick = process.tick();
    /// let numbers = process.source_iter(q!(vec![(1, false), (2, true), (1, false), (2, false)]));
    /// let batch = numbers.batch(&tick, nondet!(/** test */));
    /// batch
    ///     .fold_keyed_idempotent(q!(|| false), q!(|acc, x| *acc |= x))
    ///     .all_ticks()
    /// # }, |mut stream| async move {
    /// // (1, false), (2, true)
    /// # assert_eq!(stream.next().await.unwrap(), (1, false));
    /// # assert_eq!(stream.next().await.unwrap(), (2, true));
    /// # }));
    /// ```
    pub fn fold_keyed_idempotent<A, I, F>(
        self,
        init: impl IntoQuotedMut<'a, I, Tick<L>>,
        comb: impl IntoQuotedMut<'a, F, Tick<L>>,
    ) -> Stream<(K, A), Tick<L>, Bounded, NoOrder, ExactlyOnce>
    where
        I: Fn() -> A + 'a,
        F: Fn(&mut A, V) + 'a,
    {
        self.into_keyed().fold_idempotent(init, comb).entries()
    }

    #[deprecated = "use .into_keyed().reduce_idempotent(...) instead"]
    /// A special case of [`Stream::reduce_idempotent`], in the spirit of SQL's GROUP BY and aggregation constructs.
    /// The input tuples are partitioned into groups by the first element ("keys"), and for each group the values
    /// in the second element are accumulated via the `comb` closure.
    ///
    /// The `comb` closure must be **idempotent**, as there may be non-deterministic duplicates.
    ///
    /// If you need the accumulated value to have a different type than the input, use [`Stream::fold_keyed_idempotent`].
    ///
    /// # Example
    /// ```rust
    /// # use hydro_lang::prelude::*;
    /// # use futures::StreamExt;
    /// # tokio_test::block_on(hydro_lang::test_util::stream_transform_test(|process| {
    /// let tick = process.tick();
    /// let numbers = process.source_iter(q!(vec![(1, false), (2, true), (1, false), (2, false)]));
    /// let batch = numbers.batch(&tick, nondet!(/** test */));
    /// batch
    ///     .reduce_keyed_idempotent(q!(|acc, x| *acc |= x))
    ///     .all_ticks()
    /// # }, |mut stream| async move {
    /// // (1, false), (2, true)
    /// # assert_eq!(stream.next().await.unwrap(), (1, false));
    /// # assert_eq!(stream.next().await.unwrap(), (2, true));
    /// # }));
    /// ```
    pub fn reduce_keyed_idempotent<F>(
        self,
        comb: impl IntoQuotedMut<'a, F, Tick<L>>,
    ) -> Stream<(K, V), Tick<L>, Bounded, NoOrder, ExactlyOnce>
    where
        F: Fn(&mut V, V) + 'a,
    {
        self.into_keyed().reduce_idempotent(comb).entries()
    }
}

impl<'a, T, L, B: Boundedness, O: Ordering, R: Retries> Stream<T, Atomic<L>, B, O, R>
where
    L: Location<'a> + NoTick,
{
    /// Returns a stream corresponding to the latest batch of elements being atomically
    /// processed. These batches are guaranteed to be contiguous across ticks and preserve
    /// the order of the input.
    ///
    /// # Non-Determinism
    /// The batch boundaries are non-deterministic and may change across executions.
    pub fn batch_atomic(self, _nondet: NonDet) -> Stream<T, Tick<L>, Bounded, O, R> {
        Stream::new(
            self.location.clone().tick,
            HydroNode::Batch {
                inner: Box::new(self.ir_node.into_inner()),
                metadata: self
                    .location
                    .tick
                    .new_node_metadata(Stream::<T, Tick<L>, Bounded, O, R>::collection_kind()),
            },
        )
    }

    /// Yields the elements of this stream back into a top-level, asynchronous execution context.
    /// See [`Stream::atomic`] for more details.
    pub fn end_atomic(self) -> Stream<T, L, B, O, R> {
        Stream::new(
            self.location.tick.l.clone(),
            HydroNode::EndAtomic {
                inner: Box::new(self.ir_node.into_inner()),
                metadata: self
                    .location
                    .tick
                    .l
                    .new_node_metadata(Stream::<T, L, B, O, R>::collection_kind()),
            },
        )
    }
}

impl<'a, T, L, B: Boundedness, O: Ordering, R: Retries> Stream<T, L, B, O, R>
where
    L: Location<'a>,
{
    /// Shifts this stream into an atomic context, which guarantees that any downstream logic
    /// will all be executed synchronously before any outputs are yielded (in [`Stream::end_atomic`]).
    ///
    /// This is useful to enforce local consistency constraints, such as ensuring that a write is
    /// processed before an acknowledgement is emitted. Entering an atomic section requires a [`Tick`]
    /// argument that declares where the stream will be atomically processed. Batching a stream into
    /// the _same_ [`Tick`] will preserve the synchronous execution, while batching into a different
    /// [`Tick`] will introduce asynchrony.
    pub fn atomic(self, tick: &Tick<L>) -> Stream<T, Atomic<L>, B, O, R> {
        let out_location = Atomic { tick: tick.clone() };
        Stream::new(
            out_location.clone(),
            HydroNode::BeginAtomic {
                inner: Box::new(self.ir_node.into_inner()),
                metadata: out_location
                    .new_node_metadata(Stream::<T, Atomic<L>, B, O, R>::collection_kind()),
            },
        )
    }

    /// Given a tick, returns a stream corresponding to a batch of elements segmented by
    /// that tick. These batches are guaranteed to be contiguous across ticks and preserve
    /// the order of the input. The output stream will execute in the [`Tick`] that was
    /// used to create the atomic section.
    ///
    /// # Non-Determinism
    /// The batch boundaries are non-deterministic and may change across executions.
    pub fn batch(self, tick: &Tick<L>, _nondet: NonDet) -> Stream<T, Tick<L>, Bounded, O, R> {
        assert_eq!(Location::id(tick.outer()), Location::id(&self.location));
        Stream::new(
            tick.clone(),
            HydroNode::Batch {
                inner: Box::new(self.ir_node.into_inner()),
                metadata: tick
                    .new_node_metadata(Stream::<T, Tick<L>, Bounded, O, R>::collection_kind()),
            },
        )
    }

    /// Given a time interval, returns a stream corresponding to samples taken from the
    /// stream roughly at that interval. The output will have elements in the same order
    /// as the input, but with arbitrary elements skipped between samples. There is also
    /// no guarantee on the exact timing of the samples.
    ///
    /// # Non-Determinism
    /// The output stream is non-deterministic in which elements are sampled, since this
    /// is controlled by a clock.
    pub fn sample_every(
        self,
        interval: impl QuotedWithContext<'a, std::time::Duration, L> + Copy + 'a,
        nondet: NonDet,
    ) -> Stream<T, L, Unbounded, O, AtLeastOnce>
    where
        L: NoTick + NoAtomic,
    {
        let samples = self.location.source_interval(interval, nondet);

        let tick = self.location.tick();
        self.batch(&tick, nondet)
            .filter_if_some(samples.batch(&tick, nondet).first())
            .all_ticks()
            .weakest_retries()
    }

    /// Given a timeout duration, returns an [`Optional`]  which will have a value if the
    /// stream has not emitted a value since that duration.
    ///
    /// # Non-Determinism
    /// Timeout relies on non-deterministic sampling of the stream, so depending on when
    /// samples take place, timeouts may be non-deterministically generated or missed,
    /// and the notification of the timeout may be delayed as well. There is also no
    /// guarantee on how long the [`Optional`] will have a value after the timeout is
    /// detected based on when the next sample is taken.
    pub fn timeout(
        self,
        duration: impl QuotedWithContext<'a, std::time::Duration, Tick<L>> + Copy + 'a,
        nondet: NonDet,
    ) -> Optional<(), L, Unbounded>
    where
        L: NoTick + NoAtomic,
    {
        let tick = self.location.tick();

        let latest_received = self.assume_retries(nondet).fold_commutative(
            q!(|| None),
            q!(|latest, _| {
                *latest = Some(Instant::now());
            }),
        );

        latest_received
            .snapshot(&tick, nondet)
            .filter_map(q!(move |latest_received| {
                if let Some(latest_received) = latest_received {
                    if Instant::now().duration_since(latest_received) > duration {
                        Some(())
                    } else {
                        None
                    }
                } else {
                    Some(())
                }
            }))
            .latest()
    }
}

impl<'a, F, T, L, B: Boundedness, O: Ordering, R: Retries> Stream<F, L, B, O, R>
where
    L: Location<'a> + NoTick + NoAtomic,
    F: Future<Output = T>,
{
    /// Consumes a stream of `Future<T>`, produces a new stream of the resulting `T` outputs.
    /// Future outputs are produced as available, regardless of input arrival order.
    ///
    /// # Example
    /// ```rust
    /// # use std::collections::HashSet;
    /// # use futures::StreamExt;
    /// # use hydro_lang::prelude::*;
    /// # tokio_test::block_on(hydro_lang::test_util::stream_transform_test(|process| {
    /// process.source_iter(q!([2, 3, 1, 9, 6, 5, 4, 7, 8]))
    ///     .map(q!(|x| async move {
    ///         tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
    ///         x
    ///     }))
    ///     .resolve_futures()
    /// #   },
    /// #   |mut stream| async move {
    /// // 1, 2, 3, 4, 5, 6, 7, 8, 9 (in any order)
    /// #       let mut output = HashSet::new();
    /// #       for _ in 1..10 {
    /// #           output.insert(stream.next().await.unwrap());
    /// #       }
    /// #       assert_eq!(
    /// #           output,
    /// #           HashSet::<i32>::from_iter(1..10)
    /// #       );
    /// #   },
    /// # ));
    pub fn resolve_futures(self) -> Stream<T, L, B, NoOrder, R> {
        Stream::new(
            self.location.clone(),
            HydroNode::ResolveFutures {
                input: Box::new(self.ir_node.into_inner()),
                metadata: self
                    .location
                    .new_node_metadata(Stream::<T, L, B, NoOrder, R>::collection_kind()),
            },
        )
    }

    /// Consumes a stream of `Future<T>`, produces a new stream of the resulting `T` outputs.
    /// Future outputs are produced in the same order as the input stream.
    ///
    /// # Example
    /// ```rust
    /// # use std::collections::HashSet;
    /// # use futures::StreamExt;
    /// # use hydro_lang::prelude::*;
    /// # tokio_test::block_on(hydro_lang::test_util::stream_transform_test(|process| {
    /// process.source_iter(q!([2, 3, 1, 9, 6, 5, 4, 7, 8]))
    ///     .map(q!(|x| async move {
    ///         tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
    ///         x
    ///     }))
    ///     .resolve_futures_ordered()
    /// #   },
    /// #   |mut stream| async move {
    /// // 2, 3, 1, 9, 6, 5, 4, 7, 8
    /// #       let mut output = Vec::new();
    /// #       for _ in 1..10 {
    /// #           output.push(stream.next().await.unwrap());
    /// #       }
    /// #       assert_eq!(
    /// #           output,
    /// #           vec![2, 3, 1, 9, 6, 5, 4, 7, 8]
    /// #       );
    /// #   },
    /// # ));
    pub fn resolve_futures_ordered(self) -> Stream<T, L, B, O, R> {
        Stream::new(
            self.location.clone(),
            HydroNode::ResolveFuturesOrdered {
                input: Box::new(self.ir_node.into_inner()),
                metadata: self
                    .location
                    .new_node_metadata(Stream::<T, L, B, O, R>::collection_kind()),
            },
        )
    }
}

impl<'a, T, L, B: Boundedness> Stream<T, L, B, TotalOrder, ExactlyOnce>
where
    L: Location<'a> + NoTick,
{
    /// Executes the provided closure for every element in this stream.
    ///
    /// Because the closure may have side effects, the stream must have deterministic order
    /// ([`TotalOrder`]) and no retries ([`ExactlyOnce`]). If the side effects can tolerate
    /// out-of-order or duplicate execution, use [`Stream::assume_ordering`] and
    /// [`Stream::assume_retries`] with an explanation for why this is the case.
    pub fn for_each<F: Fn(T) + 'a>(self, f: impl IntoQuotedMut<'a, F, L>) {
        let f = f.splice_fn1_ctx(&self.location).into();
        self.location
            .flow_state()
            .borrow_mut()
            .push_root(HydroRoot::ForEach {
                input: Box::new(self.ir_node.into_inner()),
                f,
                op_metadata: HydroIrOpMetadata::new(),
            });
    }

    /// Sends all elements of this stream to a provided [`futures::Sink`], such as an external
    /// TCP socket to some other server. You should _not_ use this API for interacting with
    /// external clients, instead see [`Location::bidi_external_many_bytes`] and
    /// [`Location::bidi_external_many_bincode`]. This should be used for custom, low-level
    /// interaction with asynchronous sinks.
    pub fn dest_sink<S>(self, sink: impl QuotedWithContext<'a, S, L>)
    where
        S: 'a + futures::Sink<T> + Unpin,
    {
        self.location
            .flow_state()
            .borrow_mut()
            .push_root(HydroRoot::DestSink {
                sink: sink.splice_typed_ctx(&self.location).into(),
                input: Box::new(self.ir_node.into_inner()),
                op_metadata: HydroIrOpMetadata::new(),
            });
    }
}

impl<'a, T, L, O: Ordering, R: Retries> Stream<T, Tick<L>, Bounded, O, R>
where
    L: Location<'a>,
{
    /// Asynchronously yields this batch of elements outside the tick as an unbounded stream,
    /// which will stream all the elements across _all_ tick iterations by concatenating the batches.
    pub fn all_ticks(self) -> Stream<T, L, Unbounded, O, R> {
        Stream::new(
            self.location.outer().clone(),
            HydroNode::YieldConcat {
                inner: Box::new(self.ir_node.into_inner()),
                metadata: self
                    .location
                    .outer()
                    .new_node_metadata(Stream::<T, L, Unbounded, O, R>::collection_kind()),
            },
        )
    }

    /// Synchronously yields this batch of elements outside the tick as an unbounded stream,
    /// which will stream all the elements across _all_ tick iterations by concatenating the batches.
    ///
    /// Unlike [`Stream::all_ticks`], this preserves synchronous execution, as the output stream
    /// is emitted in an [`Atomic`] context that will process elements synchronously with the input
    /// stream's [`Tick`] context.
    pub fn all_ticks_atomic(self) -> Stream<T, Atomic<L>, Unbounded, O, R> {
        let out_location = Atomic {
            tick: self.location.clone(),
        };

        Stream::new(
            out_location.clone(),
            HydroNode::YieldConcat {
                inner: Box::new(self.ir_node.into_inner()),
                metadata: out_location
                    .new_node_metadata(Stream::<T, Atomic<L>, Unbounded, O, R>::collection_kind()),
            },
        )
    }

    /// Accumulates the elements of this stream **across ticks** by concatenating them together.
    ///
    /// The output stream in tick T will contain the elements of the input at tick 0, 1, ..., up to
    /// and including tick T. This is useful for accumulating streaming inputs across ticks, but be
    /// careful when using this operator, as its memory usage will grow linearly over time since it
    /// must store its inputs indefinitely.
    ///
    /// # Example
    /// ```rust
    /// # use hydro_lang::prelude::*;
    /// # use futures::StreamExt;
    /// # tokio_test::block_on(hydro_lang::test_util::stream_transform_test(|process| {
    /// let tick = process.tick();
    /// // ticks are lazy by default, forces the second tick to run
    /// tick.spin_batch(q!(1)).all_ticks().for_each(q!(|_| {}));
    ///
    /// let batch_first_tick = process
    ///   .source_iter(q!(vec![1, 2, 3, 4]))
    ///   .batch(&tick, nondet!(/** test */));
    /// let batch_second_tick = process
    ///   .source_iter(q!(vec![5, 6, 7, 8]))
    ///   .batch(&tick, nondet!(/** test */))
    ///   .defer_tick(); // appears on the second tick
    /// batch_first_tick.chain(batch_second_tick)
    ///   .persist()
    ///   .all_ticks()
    /// # }, |mut stream| async move {
    /// // [1, 2, 3, 4, 1, 2, 3, 4, 5, 6, 7, 8, 1, 2, 3, 4, 5, 6, 7, 8, ...]
    /// # for w in vec![1, 2, 3, 4, 1, 2, 3, 4, 5, 6, 7, 8, 1, 2, 3, 4, 5, 6, 7, 8] {
    /// #     assert_eq!(stream.next().await.unwrap(), w);
    /// # }
    /// # }));
    /// ```
    pub fn persist(self) -> Stream<T, Tick<L>, Bounded, O, R>
    where
        T: Clone,
    {
        Stream::new(
            self.location.clone(),
            HydroNode::Persist {
                inner: Box::new(self.ir_node.into_inner()),
                metadata: self
                    .location
                    .new_node_metadata(Stream::<T, Tick<L>, Bounded, O, R>::collection_kind()),
            },
        )
    }

    /// Shifts the elements in `self` to the **next tick**, so that the returned stream at tick `T`
    /// always has the elements of `self` at tick `T - 1`.
    ///
    /// At tick `0`, the output stream is empty, since there is no previous tick.
    ///
    /// This operator enables stateful iterative processing with ticks, by sending data from one
    /// tick to the next. For example, you can use it to compare inputs across consecutive batches.
    ///
    /// # Example
    /// ```rust
    /// # use hydro_lang::prelude::*;
    /// # use futures::StreamExt;
    /// # tokio_test::block_on(hydro_lang::test_util::stream_transform_test(|process| {
    /// let tick = process.tick();
    /// // ticks are lazy by default, forces the second tick to run
    /// tick.spin_batch(q!(1)).all_ticks().for_each(q!(|_| {}));
    ///
    /// let batch_first_tick = process
    ///   .source_iter(q!(vec![1, 2, 3, 4]))
    ///   .batch(&tick, nondet!(/** test */));
    /// let batch_second_tick = process
    ///   .source_iter(q!(vec![0, 3, 4, 5, 6]))
    ///   .batch(&tick, nondet!(/** test */))
    ///   .defer_tick(); // appears on the second tick
    /// let changes_across_ticks = batch_first_tick.chain(batch_second_tick);
    ///
    /// changes_across_ticks.clone().filter_not_in(
    ///     changes_across_ticks.defer_tick() // the elements from the previous tick
    /// ).all_ticks()
    /// # }, |mut stream| async move {
    /// // [1, 2, 3, 4 /* first tick */, 0, 5, 6 /* second tick */]
    /// # for w in vec![1, 2, 3, 4, 0, 5, 6] {
    /// #     assert_eq!(stream.next().await.unwrap(), w);
    /// # }
    /// # }));
    /// ```
    pub fn defer_tick(self) -> Stream<T, Tick<L>, Bounded, O, R> {
        Stream::new(
            self.location.clone(),
            HydroNode::DeferTick {
                input: Box::new(self.ir_node.into_inner()),
                metadata: self
                    .location
                    .new_node_metadata(Stream::<T, Tick<L>, Bounded, O, R>::collection_kind()),
            },
        )
    }
}

#[cfg(test)]
mod tests {
    #[cfg(feature = "deploy")]
    use futures::{SinkExt, StreamExt};
    #[cfg(feature = "deploy")]
    use hydro_deploy::Deployment;
    #[cfg(feature = "deploy")]
    use serde::{Deserialize, Serialize};
    #[cfg(feature = "deploy")]
    use stageleft::q;

    use crate::compile::builder::FlowBuilder;
    #[cfg(feature = "deploy")]
    use crate::live_collections::stream::ExactlyOnce;
    use crate::live_collections::stream::{NoOrder, TotalOrder};
    use crate::location::Location;
    use crate::nondet::nondet;

    mod backtrace_chained_ops;

    #[cfg(feature = "deploy")]
    struct P1 {}
    #[cfg(feature = "deploy")]
    struct P2 {}

    #[cfg(feature = "deploy")]
    #[derive(Serialize, Deserialize, Debug)]
    struct SendOverNetwork {
        n: u32,
    }

    #[cfg(feature = "deploy")]
    #[tokio::test]
    async fn first_ten_distributed() {
        let mut deployment = Deployment::new();

        let flow = FlowBuilder::new();
        let first_node = flow.process::<P1>();
        let second_node = flow.process::<P2>();
        let external = flow.external::<P2>();

        let numbers = first_node.source_iter(q!(0..10));
        let out_port = numbers
            .map(q!(|n| SendOverNetwork { n }))
            .send_bincode(&second_node)
            .send_bincode_external(&external);

        let nodes = flow
            .with_process(&first_node, deployment.Localhost())
            .with_process(&second_node, deployment.Localhost())
            .with_external(&external, deployment.Localhost())
            .deploy(&mut deployment);

        deployment.deploy().await.unwrap();

        let mut external_out = nodes.connect(out_port).await;

        deployment.start().await.unwrap();

        for i in 0..10 {
            assert_eq!(external_out.next().await.unwrap().n, i);
        }
    }

    #[cfg(feature = "deploy")]
    #[tokio::test]
    async fn first_cardinality() {
        let mut deployment = Deployment::new();

        let flow = FlowBuilder::new();
        let node = flow.process::<()>();
        let external = flow.external::<()>();

        let node_tick = node.tick();
        let count = node_tick
            .singleton(q!([1, 2, 3]))
            .into_stream()
            .flatten_ordered()
            .first()
            .into_stream()
            .count()
            .all_ticks()
            .send_bincode_external(&external);

        let nodes = flow
            .with_process(&node, deployment.Localhost())
            .with_external(&external, deployment.Localhost())
            .deploy(&mut deployment);

        deployment.deploy().await.unwrap();

        let mut external_out = nodes.connect(count).await;

        deployment.start().await.unwrap();

        assert_eq!(external_out.next().await.unwrap(), 1);
    }

    #[cfg(feature = "deploy")]
    #[tokio::test]
    async fn unbounded_reduce_remembers_state() {
        let mut deployment = Deployment::new();

        let flow = FlowBuilder::new();
        let node = flow.process::<()>();
        let external = flow.external::<()>();

        let (input_port, input) = node.source_external_bincode(&external);
        let out = input
            .reduce(q!(|acc, v| *acc += v))
            .sample_eager(nondet!(/** test */))
            .send_bincode_external(&external);

        let nodes = flow
            .with_process(&node, deployment.Localhost())
            .with_external(&external, deployment.Localhost())
            .deploy(&mut deployment);

        deployment.deploy().await.unwrap();

        let mut external_in = nodes.connect(input_port).await;
        let mut external_out = nodes.connect(out).await;

        deployment.start().await.unwrap();

        external_in.send(1).await.unwrap();
        assert_eq!(external_out.next().await.unwrap(), 1);

        external_in.send(2).await.unwrap();
        assert_eq!(external_out.next().await.unwrap(), 3);
    }

    #[cfg(feature = "deploy")]
    #[tokio::test]
    async fn atomic_fold_replays_each_tick() {
        let mut deployment = Deployment::new();

        let flow = FlowBuilder::new();
        let node = flow.process::<()>();
        let external = flow.external::<()>();

        let (input_port, input) =
            node.source_external_bincode::<_, _, TotalOrder, ExactlyOnce>(&external);
        let tick = node.tick();

        let out = input
            .batch(&tick, nondet!(/** test */))
            .cross_singleton(
                node.source_iter(q!(vec![1, 2, 3]))
                    .atomic(&tick)
                    .fold(q!(|| 0), q!(|acc, v| *acc += v))
                    .snapshot_atomic(nondet!(/** test */)),
            )
            .all_ticks()
            .send_bincode_external(&external);

        let nodes = flow
            .with_process(&node, deployment.Localhost())
            .with_external(&external, deployment.Localhost())
            .deploy(&mut deployment);

        deployment.deploy().await.unwrap();

        let mut external_in = nodes.connect(input_port).await;
        let mut external_out = nodes.connect(out).await;

        deployment.start().await.unwrap();

        external_in.send(1).await.unwrap();
        assert_eq!(external_out.next().await.unwrap(), (1, 6));

        external_in.send(2).await.unwrap();
        assert_eq!(external_out.next().await.unwrap(), (2, 6));
    }

    #[cfg(feature = "deploy")]
    #[tokio::test]
    async fn unbounded_scan_remembers_state() {
        let mut deployment = Deployment::new();

        let flow = FlowBuilder::new();
        let node = flow.process::<()>();
        let external = flow.external::<()>();

        let (input_port, input) = node.source_external_bincode(&external);
        let out = input
            .scan(
                q!(|| 0),
                q!(|acc, v| {
                    *acc += v;
                    Some(*acc)
                }),
            )
            .send_bincode_external(&external);

        let nodes = flow
            .with_process(&node, deployment.Localhost())
            .with_external(&external, deployment.Localhost())
            .deploy(&mut deployment);

        deployment.deploy().await.unwrap();

        let mut external_in = nodes.connect(input_port).await;
        let mut external_out = nodes.connect(out).await;

        deployment.start().await.unwrap();

        external_in.send(1).await.unwrap();
        assert_eq!(external_out.next().await.unwrap(), 1);

        external_in.send(2).await.unwrap();
        assert_eq!(external_out.next().await.unwrap(), 3);
    }

    #[cfg(feature = "deploy")]
    #[tokio::test]
    async fn unbounded_enumerate_remembers_state() {
        let mut deployment = Deployment::new();

        let flow = FlowBuilder::new();
        let node = flow.process::<()>();
        let external = flow.external::<()>();

        let (input_port, input) = node.source_external_bincode(&external);
        let out = input.enumerate().send_bincode_external(&external);

        let nodes = flow
            .with_process(&node, deployment.Localhost())
            .with_external(&external, deployment.Localhost())
            .deploy(&mut deployment);

        deployment.deploy().await.unwrap();

        let mut external_in = nodes.connect(input_port).await;
        let mut external_out = nodes.connect(out).await;

        deployment.start().await.unwrap();

        external_in.send(1).await.unwrap();
        assert_eq!(external_out.next().await.unwrap(), (0, 1));

        external_in.send(2).await.unwrap();
        assert_eq!(external_out.next().await.unwrap(), (1, 2));
    }

    #[cfg(feature = "deploy")]
    #[tokio::test]
    async fn unbounded_unique_remembers_state() {
        let mut deployment = Deployment::new();

        let flow = FlowBuilder::new();
        let node = flow.process::<()>();
        let external = flow.external::<()>();

        let (input_port, input) =
            node.source_external_bincode::<_, _, TotalOrder, ExactlyOnce>(&external);
        let out = input.unique().send_bincode_external(&external);

        let nodes = flow
            .with_process(&node, deployment.Localhost())
            .with_external(&external, deployment.Localhost())
            .deploy(&mut deployment);

        deployment.deploy().await.unwrap();

        let mut external_in = nodes.connect(input_port).await;
        let mut external_out = nodes.connect(out).await;

        deployment.start().await.unwrap();

        external_in.send(1).await.unwrap();
        assert_eq!(external_out.next().await.unwrap(), 1);

        external_in.send(2).await.unwrap();
        assert_eq!(external_out.next().await.unwrap(), 2);

        external_in.send(1).await.unwrap();
        external_in.send(3).await.unwrap();
        assert_eq!(external_out.next().await.unwrap(), 3);
    }

    #[test]
    #[should_panic]
    fn sim_batch_nondet_size() {
        let flow = FlowBuilder::new();
        let external = flow.external::<()>();
        let node = flow.process::<()>();

        let (port, input) = node.source_external_bincode::<_, _, TotalOrder, _>(&external);

        let tick = node.tick();
        let out_port = input
            .batch(&tick, nondet!(/** test */))
            .count()
            .all_ticks()
            .send_bincode_external(&external);

        flow.sim().exhaustive(async |mut compiled| {
            let in_send = compiled.connect(&port);
            let mut out_recv = compiled.connect(&out_port);
            compiled.launch();

            in_send.send(());
            in_send.send(());
            in_send.send(());

            assert_eq!(out_recv.next().await.unwrap(), 3); // fails with nondet batching
        });
    }

    #[test]
    fn sim_batch_preserves_order() {
        let flow = FlowBuilder::new();
        let external = flow.external::<()>();
        let node = flow.process::<()>();

        let (port, input) = node.source_external_bincode(&external);

        let tick = node.tick();
        let out_port = input
            .batch(&tick, nondet!(/** test */))
            .all_ticks()
            .send_bincode_external(&external);

        flow.sim().exhaustive(async |mut compiled| {
            let in_send = compiled.connect(&port);
            let out_recv = compiled.connect(&out_port);
            compiled.launch();

            in_send.send(1);
            in_send.send(2);
            in_send.send(3);

            out_recv.assert_yields_only([1, 2, 3]).await;
        });
    }

    #[test]
    #[should_panic]
    fn sim_batch_unordered_shuffles() {
        let flow = FlowBuilder::new();
        let external = flow.external::<()>();
        let node = flow.process::<()>();

        let (port, input) = node.source_external_bincode::<_, _, NoOrder, _>(&external);

        let tick = node.tick();
        let batch = input.batch(&tick, nondet!(/** test */));
        let out_port = batch
            .clone()
            .min()
            .zip(batch.max())
            .all_ticks()
            .send_bincode_external(&external);

        flow.sim().exhaustive(async |mut compiled| {
            let in_send = compiled.connect(&port);
            let out_recv = compiled.connect(&out_port);
            compiled.launch();

            in_send.send_many_unordered([1, 2, 3]).unwrap();

            if out_recv.collect::<Vec<_>>().await == vec![(1, 3), (2, 2)] {
                panic!("saw both (1, 3) and (2, 2), so batching must have shuffled the order");
            }
        });
    }

    #[test]
    fn sim_batch_unordered_shuffles_count() {
        let flow = FlowBuilder::new();
        let external = flow.external::<()>();
        let node = flow.process::<()>();

        let (port, input) = node.source_external_bincode::<_, _, NoOrder, _>(&external);

        let tick = node.tick();
        let batch = input.batch(&tick, nondet!(/** test */));
        let out_port = batch.all_ticks().send_bincode_external(&external);

        let instance_count = flow.sim().exhaustive(async |mut compiled| {
            let in_send = compiled.connect(&port);
            let out_recv = compiled.connect(&out_port);
            compiled.launch();

            in_send.send_many_unordered([1, 2, 3, 4]).unwrap();
            out_recv.assert_yields_only_unordered([1, 2, 3, 4]).await;
        });

        assert_eq!(
            instance_count,
            75 // ∑ (k=1 to 4) S(4,k) × k! = 75
        )
    }

    #[test]
    #[ignore = "assume_ordering not yet supported on bounded collections"]
    fn sim_observe_order_batched_count() {
        let flow = FlowBuilder::new();
        let external = flow.external::<()>();
        let node = flow.process::<()>();

        let (port, input) = node.source_external_bincode::<_, _, NoOrder, _>(&external);

        let tick = node.tick();
        let batch = input.batch(&tick, nondet!(/** test */));
        let out_port = batch
            .assume_ordering::<TotalOrder>(nondet!(/** test */))
            .all_ticks()
            .send_bincode_external(&external);

        let instance_count = flow.sim().exhaustive(async |mut compiled| {
            let in_send = compiled.connect(&port);
            let out_recv = compiled.connect(&out_port);
            compiled.launch();

            in_send.send_many_unordered([1, 2, 3, 4]).unwrap();
            let _ = out_recv.collect::<Vec<_>>().await;
        });

        assert_eq!(
            instance_count,
            192 // 4! * 2^{4 - 1}
        )
    }

    #[test]
    fn sim_unordered_count_instance_count() {
        let flow = FlowBuilder::new();
        let external = flow.external::<()>();
        let node = flow.process::<()>();

        let (port, input) = node.source_external_bincode::<_, _, NoOrder, _>(&external);

        let tick = node.tick();
        let out_port = input
            .count()
            .snapshot(&tick, nondet!(/** test */))
            .all_ticks()
            .send_bincode_external(&external);

        let instance_count = flow.sim().exhaustive(async |mut compiled| {
            let in_send = compiled.connect(&port);
            let out_recv = compiled.connect(&out_port);
            compiled.launch();

            in_send.send_many_unordered([1, 2, 3, 4]).unwrap();
            assert!(out_recv.collect::<Vec<_>>().await.last().unwrap() == &4);
        });

        assert_eq!(
            instance_count,
            16 // 2^4, { 0, 1, 2, 3 } can be a snapshot and 4 is always included
        )
    }
}
