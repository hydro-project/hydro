//! Definitions and core APIs for the [`Stream`] live collection.

use std::cell::RefCell;
use std::future::Future;
use std::hash::Hash;
use std::marker::PhantomData;
use std::ops::Deref;
use std::rc::Rc;

use stageleft::{IntoQuotedMut, QuotedWithContext, q};
use syn::parse_quote;
use tokio::time::Instant;

use super::boundedness::{Bounded, Boundedness, Unbounded};
use super::keyed_stream::KeyedStream;
use super::optional::Optional;
use super::singleton::Singleton;
use crate::compile::ir::{HydroIrOpMetadata, HydroNode, HydroRoot, TeeNode};
#[cfg(stageleft_runtime)]
use crate::forward_handle::{CycleCollection, ReceiverComplete};
use crate::forward_handle::{ForwardRef, TickCycle};
#[cfg(stageleft_runtime)]
use crate::location::dynamic::{DynLocation, LocationId};
use crate::location::tick::{Atomic, DeferTick, NoAtomic};
use crate::location::{Location, NoTick, Tick, check_matching_location};
use crate::nondet::{NonDet, nondet};

pub mod networking;

/// Marks the stream as being totally ordered, which means that there are
/// no sources of non-determinism (other than intentional ones) that will
/// affect the order of elements.
pub enum TotalOrder {}

/// Marks the stream as having no order, which means that the order of
/// elements may be affected by non-determinism.
///
/// This restricts certain operators, such as `fold` and `reduce`, to only
/// be used with commutative aggregation functions.
pub enum NoOrder {}

/// Helper trait for determining the weakest of two orderings.
#[sealed::sealed]
pub trait MinOrder<Other> {
    /// The weaker of the two orderings.
    type Min;
}

#[sealed::sealed]
impl<T> MinOrder<T> for T {
    type Min = T;
}

#[sealed::sealed]
impl MinOrder<NoOrder> for TotalOrder {
    type Min = NoOrder;
}

#[sealed::sealed]
impl MinOrder<TotalOrder> for NoOrder {
    type Min = NoOrder;
}

/// Marks the stream as having deterministic message cardinality, with no
/// possibility of duplicates.
pub enum ExactlyOnce {}

/// Marks the stream as having non-deterministic message cardinality, which
/// means that duplicates may occur, but messages will not be dropped.
pub enum AtLeastOnce {}

/// Helper trait for determining the weakest of two retry guarantees.
#[sealed::sealed]
pub trait MinRetries<Other> {
    /// The weaker of the two retry guarantees.
    type Min;
}

#[sealed::sealed]
impl<T> MinRetries<T> for T {
    type Min = T;
}

#[sealed::sealed]
impl MinRetries<ExactlyOnce> for AtLeastOnce {
    type Min = AtLeastOnce;
}

#[sealed::sealed]
impl MinRetries<AtLeastOnce> for ExactlyOnce {
    type Min = AtLeastOnce;
}

/// An ordered sequence stream of elements of type `T`.
///
/// Type Parameters:
/// - `Type`: the type of elements in the stream
/// - `Loc`: the location where the stream is being materialized
/// - `Bound`: the boundedness of the stream, which is either [`Bounded`]
///   or [`Unbounded`]
/// - `Order`: the ordering of the stream, which is either [`TotalOrder`]
///   or [`NoOrder`] (default is [`TotalOrder`])
pub struct Stream<Type, Loc, Bound: Boundedness, Order = TotalOrder, Retries = ExactlyOnce> {
    pub(crate) location: Loc,
    pub(crate) ir_node: RefCell<HydroNode>,

    _phantom: PhantomData<(Type, Loc, Bound, Order, Retries)>,
}

impl<'a, T, L, O, R> From<Stream<T, L, Bounded, O, R>> for Stream<T, L, Unbounded, O, R>
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

impl<'a, T, L, B: Boundedness, R> From<Stream<T, L, B, TotalOrder, R>>
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

impl<'a, T, L, B: Boundedness, O> From<Stream<T, L, B, O, ExactlyOnce>>
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

impl<'a, T, L, O, R> DeferTick for Stream<T, Tick<L>, Bounded, O, R>
where
    L: Location<'a>,
{
    fn defer_tick(self) -> Self {
        Stream::defer_tick(self)
    }
}

impl<'a, T, L, O, R> CycleCollection<'a, TickCycle> for Stream<T, Tick<L>, Bounded, O, R>
where
    L: Location<'a>,
{
    type Location = Tick<L>;

    fn create_source(ident: syn::Ident, location: Tick<L>) -> Self {
        Stream::new(
            location.clone(),
            HydroNode::CycleSource {
                ident,
                metadata: location.new_node_metadata::<T>(),
            },
        )
    }
}

impl<'a, T, L, O, R> ReceiverComplete<'a, TickCycle> for Stream<T, Tick<L>, Bounded, O, R>
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
                out_location: Location::id(&self.location),
                op_metadata: HydroIrOpMetadata::new(),
            });
    }
}

impl<'a, T, L, B: Boundedness, O, R> CycleCollection<'a, ForwardRef> for Stream<T, L, B, O, R>
where
    L: Location<'a> + NoTick,
{
    type Location = L;

    fn create_source(ident: syn::Ident, location: L) -> Self {
        Stream::new(
            location.clone(),
            HydroNode::Persist {
                inner: Box::new(HydroNode::CycleSource {
                    ident,
                    metadata: location.new_node_metadata::<T>(),
                }),
                metadata: location.new_node_metadata::<T>(),
            },
        )
    }
}

impl<'a, T, L, B: Boundedness, O, R> ReceiverComplete<'a, ForwardRef> for Stream<T, L, B, O, R>
where
    L: Location<'a> + NoTick,
{
    fn complete(self, ident: syn::Ident, expected_location: LocationId) {
        assert_eq!(
            Location::id(&self.location),
            expected_location,
            "locations do not match"
        );
        let metadata = self.location.new_node_metadata::<T>();
        self.location
            .flow_state()
            .borrow_mut()
            .push_root(HydroRoot::CycleSink {
                ident,
                input: Box::new(HydroNode::Unpersist {
                    inner: Box::new(self.ir_node.into_inner()),
                    metadata: metadata.clone(),
                }),
                out_location: Location::id(&self.location),
                op_metadata: HydroIrOpMetadata::new(),
            });
    }
}

impl<'a, T, L, B: Boundedness, O, R> Clone for Stream<T, L, B, O, R>
where
    T: Clone,
    L: Location<'a>,
{
    fn clone(&self) -> Self {
        if !matches!(self.ir_node.borrow().deref(), HydroNode::Tee { .. }) {
            let orig_ir_node = self.ir_node.replace(HydroNode::Placeholder);
            *self.ir_node.borrow_mut() = HydroNode::Tee {
                inner: TeeNode(Rc::new(RefCell::new(orig_ir_node))),
                metadata: self.location.new_node_metadata::<T>(),
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

impl<'a, T, L, B: Boundedness, O, R> Stream<T, L, B, O, R>
where
    L: Location<'a>,
{
    pub(crate) fn new(location: L, ir_node: HydroNode) -> Self {
        Stream {
            location,
            ir_node: RefCell::new(ir_node),
            _phantom: PhantomData,
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
                metadata: self.location.new_node_metadata::<U>(),
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
                metadata: self.location.new_node_metadata::<U>(),
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
                metadata: self.location.new_node_metadata::<U>(),
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
    pub fn filter<F>(self, f: impl IntoQuotedMut<'a, F, L>) -> Stream<T, L, B, O, R>
    where
        F: Fn(&T) -> bool + 'a,
    {
        let f = f.splice_fn1_borrow_ctx(&self.location).into();
        Stream::new(
            self.location.clone(),
            HydroNode::Filter {
                f,
                input: Box::new(self.ir_node.into_inner()),
                metadata: self.location.new_node_metadata::<T>(),
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
                metadata: self.location.new_node_metadata::<U>(),
            },
        )
    }

    /// Generates a stream that maps each input element `i` to a tuple `(i, x)`,
    /// where `x` is the final value of `other`, a bounded [`Singleton`].
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
                metadata: self.location.new_node_metadata::<(T, O2)>(),
            },
        )
    }

    /// Allow this stream through if the argument (a Bounded Optional) is non-empty, otherwise the output is empty.
    pub fn continue_if<U>(self, signal: Optional<U, L, Bounded>) -> Stream<T, L, B, O, R> {
        self.cross_singleton(signal.map(q!(|_u| ())))
            .map(q!(|(d, _signal)| d))
    }

    /// Allow this stream through if the argument (a Bounded Optional) is empty, otherwise the output is empty.
    pub fn continue_unless<U>(self, other: Optional<U, L, Bounded>) -> Stream<T, L, B, O, R> {
        self.continue_if(other.into_stream().count().filter(q!(|c| *c == 0)))
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
    pub fn cross_product<T2, O2>(
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
                metadata: self.location.new_node_metadata::<(T, T2)>(),
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
    ///     process.source_iter(q!(vec![1, 2, 3, 2, 1, 4])).unique()
    /// # }, |mut stream| async move {
    /// # for w in vec![1, 2, 3, 4] {
    /// #     assert_eq!(stream.next().await.unwrap(), w);
    /// # }
    /// # }));
    pub fn unique(self) -> Stream<T, L, B, O, ExactlyOnce>
    where
        T: Eq + Hash,
    {
        Stream::new(
            self.location.clone(),
            HydroNode::Unique {
                input: Box::new(self.ir_node.into_inner()),
                metadata: self.location.new_node_metadata::<T>(),
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
    pub fn filter_not_in<O2>(
        self,
        other: Stream<T, L, Bounded, O2, R>,
    ) -> Stream<T, L, Bounded, O, R>
    where
        T: Eq + Hash,
    {
        check_matching_location(&self.location, &other.location);

        Stream::new(
            self.location.clone(),
            HydroNode::Difference {
                pos: Box::new(self.ir_node.into_inner()),
                neg: Box::new(other.ir_node.into_inner()),
                metadata: self.location.new_node_metadata::<T>(),
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
    pub fn inspect<F>(self, f: impl IntoQuotedMut<'a, F, L>) -> Stream<T, L, B, O, R>
    where
        F: Fn(&T) + 'a,
    {
        let f = f.splice_fn1_borrow_ctx(&self.location).into();

        if L::is_top_level() {
            Stream::new(
                self.location.clone(),
                HydroNode::Persist {
                    inner: Box::new(HydroNode::Inspect {
                        f,
                        input: Box::new(HydroNode::Unpersist {
                            inner: Box::new(self.ir_node.into_inner()),
                            metadata: self.location.new_node_metadata::<T>(),
                        }),
                        metadata: self.location.new_node_metadata::<T>(),
                    }),
                    metadata: self.location.new_node_metadata::<T>(),
                },
            )
        } else {
            Stream::new(
                self.location.clone(),
                HydroNode::Inspect {
                    f,
                    input: Box::new(self.ir_node.into_inner()),
                    metadata: self.location.new_node_metadata::<T>(),
                },
            )
        }
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
    pub fn assume_ordering<O2>(self, _nondet: NonDet) -> Stream<T, L, B, O2, R> {
        Stream::new(self.location, self.ir_node.into_inner())
    }

    /// Weakens the ordering guarantee provided by the stream to [`NoOrder`],
    /// which is always safe because that is the weakest possible guarantee.
    pub fn weakest_ordering(self) -> Stream<T, L, B, NoOrder, R> {
        let nondet = nondet!(/** this is a weaker odering guarantee, so it is safe to assume */);
        self.assume_ordering::<NoOrder>(nondet)
    }

    /// Explicitly "casts" the stream to a type with a different retries
    /// guarantee. Useful in unsafe code where the lack of retries cannot
    /// be proven by the type-system.
    ///
    /// # Non-Determinism
    /// This function is used as an escape hatch, and any mistakes in the
    /// provided retries guarantee will propagate into the guarantees
    /// for the rest of the program.
    pub fn assume_retries<R2>(self, _nondet: NonDet) -> Stream<T, L, B, O, R2> {
        Stream::new(self.location, self.ir_node.into_inner())
    }

    /// Weakens the retries guarantee provided by the stream to [`AtLeastOnce`],
    /// which is always safe because that is the weakest possible guarantee.
    pub fn weakest_retries(self) -> Stream<T, L, B, O, AtLeastOnce> {
        let nondet = nondet!(/** this is a weaker retry guarantee, so it is safe to assume */);
        self.assume_retries::<AtLeastOnce>(nondet)
    }

    /// Weakens the retries guarantee provided by the stream to be the weaker of the
    /// current guarantee and `R2`. This is safe because the output guarantee will
    /// always be weaker than the input.
    pub fn weaken_retries<R2>(self) -> Stream<T, L, B, O, <R as MinRetries<R2>>::Min>
    where
        R: MinRetries<R2>,
    {
        let nondet = nondet!(/** this is a weaker retry guarantee, so it is safe to assume */);
        self.assume_retries::<<R as MinRetries<R2>>::Min>(nondet)
    }
}

impl<'a, T, L, B: Boundedness, O> Stream<T, L, B, O, ExactlyOnce>
where
    L: Location<'a>,
{
    /// Given a stream with [`ExactlyOnce`] retry guarantees, weakens it to an arbitrary guarantee
    /// `R2`, which is safe because all guarantees are equal to or weaker than [`ExactlyOnce`]
    pub fn weaker_retries<R2>(self) -> Stream<T, L, B, O, R2> {
        self.assume_retries(
            nondet!(/** any retry ordering is the same or weaker than ExactlyOnce */),
        )
    }
}

impl<'a, T, L, B: Boundedness, O, R> Stream<&T, L, B, O, R>
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

impl<'a, T, L, B: Boundedness, O, R> Stream<T, L, B, O, R>
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
        self.reduce_commutative_idempotent(q!(|curr, new| {
            if new > *curr {
                *curr = new;
            }
        }))
    }

    /// Computes the maximum element in the stream as an [`Optional`], where the
    /// maximum is determined according to the `key` function. The [`Optional`] will
    /// be empty until the first element in the input arrives.
    ///
    /// # Example
    /// ```rust
    /// # use hydro_lang::prelude::*;
    /// # use futures::StreamExt;
    /// # tokio_test::block_on(hydro_lang::test_util::stream_transform_test(|process| {
    /// let tick = process.tick();
    /// let numbers = process.source_iter(q!(vec![1, 2, 3, 4]));
    /// let batch = numbers.batch(&tick, nondet!(/** test */));
    /// batch.max_by_key(q!(|x| -x)).all_ticks()
    /// # }, |mut stream| async move {
    /// // 1
    /// # assert_eq!(stream.next().await.unwrap(), 1);
    /// # }));
    /// ```
    pub fn max_by_key<K, F>(self, key: impl IntoQuotedMut<'a, F, L> + Copy) -> Optional<T, L, B>
    where
        K: Ord,
        F: Fn(&T) -> K + 'a,
    {
        let f = key.splice_fn1_borrow_ctx(&self.location);

        let wrapped: syn::Expr = parse_quote!({
            let key_fn = #f;
            move |curr, new| {
                if key_fn(&new) > key_fn(&*curr) {
                    *curr = new;
                }
            }
        });

        let mut core = HydroNode::Reduce {
            f: wrapped.into(),
            input: Box::new(self.ir_node.into_inner()),
            metadata: self.location.new_node_metadata::<T>(),
        };

        if L::is_top_level() {
            core = HydroNode::Persist {
                inner: Box::new(core),
                metadata: self.location.new_node_metadata::<T>(),
            };
        }

        Optional::new(self.location, core)
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
        self.reduce_commutative_idempotent(q!(|curr, new| {
            if new < *curr {
                *curr = new;
            }
        }))
    }
}

impl<'a, T, L, B: Boundedness, O> Stream<T, L, B, O, ExactlyOnce>
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
        self.fold_commutative(q!(|| 0usize), q!(|count, _| *count += 1))
    }
}

impl<'a, T, L, B: Boundedness, R> Stream<T, L, B, TotalOrder, R>
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
    /// Returns a stream with the current count tupled with each element in the input stream.
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
        if L::is_top_level() {
            Stream::new(
                self.location.clone(),
                HydroNode::Persist {
                    inner: Box::new(HydroNode::Enumerate {
                        is_static: true,
                        input: Box::new(HydroNode::Unpersist {
                            inner: Box::new(self.ir_node.into_inner()),
                            metadata: self.location.new_node_metadata::<T>(),
                        }),
                        metadata: self.location.new_node_metadata::<(usize, T)>(),
                    }),
                    metadata: self.location.new_node_metadata::<(usize, T)>(),
                },
            )
        } else {
            Stream::new(
                self.location.clone(),
                HydroNode::Enumerate {
                    is_static: false,
                    input: Box::new(self.ir_node.into_inner()),
                    metadata: self.location.new_node_metadata::<(usize, T)>(),
                },
            )
        }
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

        let mut core = HydroNode::Fold {
            init,
            acc: comb,
            input: Box::new(self.ir_node.into_inner()),
            metadata: self.location.new_node_metadata::<A>(),
        };

        if L::is_top_level() {
            // top-level (possibly unbounded) singletons are represented as
            // a stream which produces all values from all ticks every tick,
            // so Unpersist will always give the lastest aggregation
            core = HydroNode::Persist {
                inner: Box::new(core),
                metadata: self.location.new_node_metadata::<A>(),
            };
        }

        Singleton::new(self.location, core)
    }

    #[expect(missing_docs, reason = "TODO")]
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

        if L::is_top_level() {
            Stream::new(
                self.location.clone(),
                HydroNode::Persist {
                    inner: Box::new(HydroNode::Scan {
                        init,
                        acc: f,
                        input: Box::new(HydroNode::Unpersist {
                            inner: Box::new(self.ir_node.into_inner()),
                            metadata: self.location.new_node_metadata::<U>(),
                        }),
                        metadata: self.location.new_node_metadata::<U>(),
                    }),
                    metadata: self.location.new_node_metadata::<U>(),
                },
            )
        } else {
            Stream::new(
                self.location.clone(),
                HydroNode::Scan {
                    init,
                    acc: f,
                    input: Box::new(self.ir_node.into_inner()),
                    metadata: self.location.new_node_metadata::<U>(),
                },
            )
        }
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
        let mut core = HydroNode::Reduce {
            f,
            input: Box::new(self.ir_node.into_inner()),
            metadata: self.location.new_node_metadata::<T>(),
        };

        if L::is_top_level() {
            core = HydroNode::Persist {
                inner: Box::new(core),
                metadata: self.location.new_node_metadata::<T>(),
            };
        }

        Optional::new(self.location, core)
    }
}

impl<'a, T, L: Location<'a> + NoTick + NoAtomic, O, R> Stream<T, L, Unbounded, O, R> {
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
    pub fn interleave<O2, R2: MinRetries<R>>(
        self,
        other: Stream<T, L, Unbounded, O2, R2>,
    ) -> Stream<T, L, Unbounded, NoOrder, R::Min>
    where
        R: MinRetries<R2, Min = R2::Min>,
    {
        let tick = self.location.tick();
        // Because the outputs are unordered, we can interleave batches from both streams.
        let nondet_batch_interleaving = nondet!(/** output stream is NoOrder, can interleave */);
        self.batch(&tick, nondet_batch_interleaving)
            .weakest_ordering()
            .weaken_retries::<R2>()
            .chain(
                other
                    .batch(&tick, nondet_batch_interleaving)
                    .weakest_ordering()
                    .weaken_retries::<R>(),
            )
            .all_ticks()
    }
}

impl<'a, T, L, O, R> Stream<T, L, Bounded, O, R>
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
                metadata: self.location.new_node_metadata::<T>(),
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
    pub fn chain<O2>(self, other: Stream<T, L, Bounded, O2, R>) -> Stream<T, L, Bounded, O::Min, R>
    where
        O: MinOrder<O2>,
    {
        check_matching_location(&self.location, &other.location);

        Stream::new(
            self.location.clone(),
            HydroNode::Chain {
                first: Box::new(self.ir_node.into_inner()),
                second: Box::new(other.ir_node.into_inner()),
                metadata: self.location.new_node_metadata::<T>(),
            },
        )
    }

    /// Forms the cross-product (Cartesian product, cross-join) of the items in the 2 input streams.
    /// Unlike [`Stream::cross_product`], the output order is totally ordered when the inputs are
    /// because this is compiled into a nested loop.
    pub fn cross_product_nested_loop<T2, O2>(
        self,
        other: Stream<T2, L, Bounded, O2, R>,
    ) -> Stream<(T, T2), L, Bounded, O::Min, R>
    where
        T: Clone,
        T2: Clone,
        O: MinOrder<O2>,
    {
        check_matching_location(&self.location, &other.location);

        Stream::new(
            self.location.clone(),
            HydroNode::CrossProduct {
                left: Box::new(self.ir_node.into_inner()),
                right: Box::new(other.ir_node.into_inner()),
                metadata: self.location.new_node_metadata::<(T, T2)>(),
            },
        )
    }
}

impl<'a, K, V1, L, B: Boundedness, O, R> Stream<(K, V1), L, B, O, R>
where
    L: Location<'a>,
{
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
    pub fn join<V2, O2>(
        self,
        n: Stream<(K, V2), L, B, O2, R>,
    ) -> Stream<(K, (V1, V2)), L, B, NoOrder, R>
    where
        K: Eq + Hash,
    {
        check_matching_location(&self.location, &n.location);

        Stream::new(
            self.location.clone(),
            HydroNode::Join {
                left: Box::new(self.ir_node.into_inner()),
                right: Box::new(n.ir_node.into_inner()),
                metadata: self.location.new_node_metadata::<(K, (V1, V2))>(),
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
    pub fn anti_join<O2, R2>(self, n: Stream<K, L, Bounded, O2, R2>) -> Stream<(K, V1), L, B, O, R>
    where
        K: Eq + Hash,
    {
        check_matching_location(&self.location, &n.location);

        Stream::new(
            self.location.clone(),
            HydroNode::AntiJoin {
                pos: Box::new(self.ir_node.into_inner()),
                neg: Box::new(n.ir_node.into_inner()),
                metadata: self.location.new_node_metadata::<(K, V1)>(),
            },
        )
    }
}

impl<'a, K, V, L: Location<'a>, B: Boundedness, O, R> Stream<(K, V), L, B, O, R> {
    #[expect(missing_docs, reason = "TODO")]
    pub fn into_keyed(self) -> KeyedStream<K, V, L, B, O, R> {
        KeyedStream {
            underlying: self.weakest_ordering(),
            _phantom_order: Default::default(),
        }
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
                metadata: self.location.new_node_metadata::<(K, V)>(),
            },
        )
    }
}

impl<'a, K, V, L, O, R> Stream<(K, V), Tick<L>, Bounded, O, R>
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

impl<'a, K, V, L, O> Stream<(K, V), Tick<L>, Bounded, O, ExactlyOnce>
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

impl<'a, K, V, L, R> Stream<(K, V), Tick<L>, Bounded, TotalOrder, R>
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

impl<'a, T, L, B: Boundedness, O, R> Stream<T, Atomic<L>, B, O, R>
where
    L: Location<'a> + NoTick,
{
    /// Returns a stream corresponding to the latest batch of elements being atomically
    /// processed. These batches are guaranteed to be contiguous across ticks and preserve
    /// the order of the input.
    ///
    /// # Non-Determinism
    /// The batch boundaries are non-deterministic and may change across executions.
    pub fn batch(self, _nondet: NonDet) -> Stream<T, Tick<L>, Bounded, O, R> {
        Stream::new(
            self.location.clone().tick,
            HydroNode::Unpersist {
                inner: Box::new(self.ir_node.into_inner()),
                metadata: self.location.new_node_metadata::<T>(),
            },
        )
    }

    #[expect(missing_docs, reason = "TODO")]
    pub fn end_atomic(self) -> Stream<T, L, B, O, R> {
        Stream::new(self.location.tick.l, self.ir_node.into_inner())
    }

    #[expect(missing_docs, reason = "TODO")]
    pub fn atomic_source(&self) -> Tick<L> {
        self.location.tick.clone()
    }
}

impl<'a, T, L, B: Boundedness, O, R> Stream<T, L, B, O, R>
where
    L: Location<'a> + NoTick + NoAtomic,
{
    #[expect(missing_docs, reason = "TODO")]
    pub fn atomic(self, tick: &Tick<L>) -> Stream<T, Atomic<L>, B, O, R> {
        Stream::new(Atomic { tick: tick.clone() }, self.ir_node.into_inner())
    }

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
    pub fn resolve_futures<T2>(self) -> Stream<T2, L, B, NoOrder, R>
    where
        T: Future<Output = T2>,
    {
        Stream::new(
            self.location.clone(),
            HydroNode::ResolveFutures {
                input: Box::new(self.ir_node.into_inner()),
                metadata: self.location.new_node_metadata::<T2>(),
            },
        )
    }

    /// Given a tick, returns a stream corresponding to a batch of elements segmented by
    /// that tick. These batches are guaranteed to be contiguous across ticks and preserve
    /// the order of the input.
    ///
    /// # Non-Determinism
    /// The batch boundaries are non-deterministic and may change across executions.
    pub fn batch(self, tick: &Tick<L>, nondet: NonDet) -> Stream<T, Tick<L>, Bounded, O, R> {
        self.atomic(tick).batch(nondet)
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
    ) -> Stream<T, L, Unbounded, O, AtLeastOnce> {
        let samples = self.location.source_interval(interval, nondet);

        let tick = self.location.tick();
        self.batch(&tick, nondet)
            .continue_if(samples.batch(&tick, nondet).first())
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
    ) -> Optional<(), L, Unbounded> {
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

impl<'a, F, T, L, B: Boundedness, O, R> Stream<F, L, B, O, R>
where
    L: Location<'a> + NoTick + NoAtomic,
    F: Future<Output = T>,
{
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
                metadata: self.location.new_node_metadata::<T>(),
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
        let metadata = self.location.new_node_metadata::<T>();
        self.location
            .flow_state()
            .borrow_mut()
            .push_root(HydroRoot::ForEach {
                input: Box::new(HydroNode::Unpersist {
                    inner: Box::new(self.ir_node.into_inner()),
                    metadata: metadata.clone(),
                }),
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

#[expect(missing_docs, reason = "TODO")]
impl<'a, T, L, O, R> Stream<T, Tick<L>, Bounded, O, R>
where
    L: Location<'a>,
{
    pub fn all_ticks(self) -> Stream<T, L, Unbounded, O, R> {
        Stream::new(
            self.location.outer().clone(),
            HydroNode::Persist {
                inner: Box::new(self.ir_node.into_inner()),
                metadata: self.location.new_node_metadata::<T>(),
            },
        )
    }

    pub fn all_ticks_atomic(self) -> Stream<T, Atomic<L>, Unbounded, O, R> {
        Stream::new(
            Atomic {
                tick: self.location.clone(),
            },
            HydroNode::Persist {
                inner: Box::new(self.ir_node.into_inner()),
                metadata: self.location.new_node_metadata::<T>(),
            },
        )
    }

    pub fn persist(self) -> Stream<T, Tick<L>, Bounded, O, R>
    where
        T: Clone,
    {
        Stream::new(
            self.location.clone(),
            HydroNode::Persist {
                inner: Box::new(self.ir_node.into_inner()),
                metadata: self.location.new_node_metadata::<T>(),
            },
        )
    }

    pub fn defer_tick(self) -> Stream<T, Tick<L>, Bounded, O, R> {
        Stream::new(
            self.location.clone(),
            HydroNode::DeferTick {
                input: Box::new(self.ir_node.into_inner()),
                metadata: self.location.new_node_metadata::<T>(),
            },
        )
    }

    pub fn delta(self) -> Stream<T, Tick<L>, Bounded, O, R> {
        Stream::new(
            self.location.clone(),
            HydroNode::Delta {
                inner: Box::new(self.ir_node.into_inner()),
                metadata: self.location.new_node_metadata::<T>(),
            },
        )
    }
}

#[cfg(test)]
mod tests {
    use futures::StreamExt;
    use hydro_deploy::Deployment;
    use serde::{Deserialize, Serialize};
    use stageleft::q;

    use crate::compile::builder::FlowBuilder;
    use crate::location::Location;

    mod backtrace_chained_ops;

    struct P1 {}
    struct P2 {}

    #[derive(Serialize, Deserialize, Debug)]
    struct SendOverNetwork {
        n: u32,
    }

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

        let mut external_out = nodes.connect_source_bincode(out_port).await;

        deployment.start().await.unwrap();

        for i in 0..10 {
            assert_eq!(external_out.next().await.unwrap().n, i);
        }
    }

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

        let mut external_out = nodes.connect_source_bincode(count).await;

        deployment.start().await.unwrap();

        assert_eq!(external_out.next().await.unwrap(), 1);
    }
}
