use std::cell::RefCell;
use std::future::Future;
use std::hash::Hash;
use std::marker::PhantomData;
use std::ops::Deref;
use std::rc::Rc;

use dfir_rs::bytes::Bytes;
use dfir_rs::futures;
use serde::Serialize;
use serde::de::DeserializeOwned;
use stageleft::{IntoQuotedMut, QuotedWithContext, q};
use syn::parse_quote;
use tokio::time::Instant;

use crate::builder::FLOW_USED_MESSAGE;
use crate::cycle::{CycleCollection, CycleComplete, DeferTick, ForwardRefMarker, TickCycleMarker};
use crate::ir::{DebugInstantiate, HydroLeaf, HydroNode, TeeNode};
use crate::location::external_process::{ExternalBincodeStream, ExternalBytesPort};
use crate::location::tick::{Atomic, NoAtomic};
use crate::location::{
    CanSend, ExternalProcess, Location, LocationId, NoTick, Tick, check_matching_location,
};
use crate::staging_util::get_this_crate;
use crate::{Bounded, Cluster, ClusterId, Optional, Singleton, Unbounded};

/// Marks the stream as being totally ordered, which means that there are
/// no sources of non-determinism (other than intentional ones) that will
/// affect the order of elements.
pub struct TotalOrder {}

/// Marks the stream as having no order, which means that the order of
/// elements may be affected by non-determinism.
///
/// This restricts certain operators, such as `fold` and `reduce`, to only
/// be used with commutative aggregation functions.
pub struct NoOrder {}

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

/// An ordered sequence stream of elements of type `T`.
///
/// Type Parameters:
/// - `T`: the type of elements in the stream
/// - `L`: the location where the stream is being materialized
/// - `B`: the boundedness of the stream, which is either [`Bounded`]
///   or [`Unbounded`]
/// - `Order`: the ordering of the stream, which is either [`TotalOrder`]
///   or [`NoOrder`] (default is [`TotalOrder`])
pub struct Stream<T, L, B, Order = TotalOrder> {
    location: L,
    pub(crate) ir_node: RefCell<HydroNode>,

    _phantom: PhantomData<(T, L, B, Order)>,
}

impl<'a, T, L: Location<'a>, O> From<Stream<T, L, Bounded, O>> for Stream<T, L, Unbounded, O> {
    fn from(stream: Stream<T, L, Bounded, O>) -> Stream<T, L, Unbounded, O> {
        Stream {
            location: stream.location,
            ir_node: stream.ir_node,
            _phantom: PhantomData,
        }
    }
}

impl<'a, T, L: Location<'a>, B> From<Stream<T, L, B, TotalOrder>> for Stream<T, L, B, NoOrder> {
    fn from(stream: Stream<T, L, B, TotalOrder>) -> Stream<T, L, B, NoOrder> {
        Stream {
            location: stream.location,
            ir_node: stream.ir_node,
            _phantom: PhantomData,
        }
    }
}

impl<'a, T, L: Location<'a>, B, Order> Stream<T, L, B, Order> {
    fn location_kind(&self) -> LocationId {
        self.location.id()
    }
}

impl<'a, T, L: Location<'a>, Order> DeferTick for Stream<T, Tick<L>, Bounded, Order> {
    fn defer_tick(self) -> Self {
        Stream::defer_tick(self)
    }
}

impl<'a, T, L: Location<'a>, Order> CycleCollection<'a, TickCycleMarker>
    for Stream<T, Tick<L>, Bounded, Order>
{
    type Location = Tick<L>;

    fn create_source(ident: syn::Ident, location: Tick<L>) -> Self {
        let location_id = location.id();
        Stream::new(
            location.clone(),
            HydroNode::CycleSource {
                ident,
                location_kind: location_id,
                metadata: location.new_node_metadata::<T>(),
            },
        )
    }
}

impl<'a, T, L: Location<'a>, Order> CycleComplete<'a, TickCycleMarker>
    for Stream<T, Tick<L>, Bounded, Order>
{
    fn complete(self, ident: syn::Ident, expected_location: LocationId) {
        assert_eq!(
            self.location.id(),
            expected_location,
            "locations do not match"
        );
        self.location
            .flow_state()
            .borrow_mut()
            .leaves
            .as_mut()
            .expect(FLOW_USED_MESSAGE)
            .push(HydroLeaf::CycleSink {
                ident,
                location_kind: self.location_kind(),
                input: Box::new(self.ir_node.into_inner()),
                metadata: self.location.new_node_metadata::<T>(),
            });
    }
}

impl<'a, T, L: Location<'a> + NoTick, B, Order> CycleCollection<'a, ForwardRefMarker>
    for Stream<T, L, B, Order>
{
    type Location = L;

    fn create_source(ident: syn::Ident, location: L) -> Self {
        let location_id = location.id();

        Stream::new(
            location.clone(),
            HydroNode::Persist {
                inner: Box::new(HydroNode::CycleSource {
                    ident,
                    location_kind: location_id,
                    metadata: location.new_node_metadata::<T>(),
                }),
                metadata: location.new_node_metadata::<T>(),
            },
        )
    }
}

impl<'a, T, L: Location<'a> + NoTick, B, Order> CycleComplete<'a, ForwardRefMarker>
    for Stream<T, L, B, Order>
{
    fn complete(self, ident: syn::Ident, expected_location: LocationId) {
        assert_eq!(
            self.location.id(),
            expected_location,
            "locations do not match"
        );
        let metadata = self.location.new_node_metadata::<T>();
        self.location
            .flow_state()
            .borrow_mut()
            .leaves
            .as_mut()
            .expect(FLOW_USED_MESSAGE)
            .push(HydroLeaf::CycleSink {
                ident,
                location_kind: self.location_kind(),
                input: Box::new(HydroNode::Unpersist {
                    inner: Box::new(self.ir_node.into_inner()),
                    metadata: metadata.clone(),
                }),
                metadata,
            });
    }
}

impl<'a, T, L: Location<'a>, B, Order> Stream<T, L, B, Order> {
    pub(crate) fn new(location: L, ir_node: HydroNode) -> Self {
        Stream {
            location,
            ir_node: RefCell::new(ir_node),
            _phantom: PhantomData,
        }
    }
}

impl<'a, T: Clone, L: Location<'a>, B, Order> Clone for Stream<T, L, B, Order> {
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

impl<'a, T, L: Location<'a>, B, Order> Stream<T, L, B, Order> {
    /// Produces a stream based on invoking `f` on each element in order.
    /// If you do not want to modify the stream and instead only want to view
    /// each item use [`Stream::inspect`] instead.
    ///
    /// # Example
    /// ```rust
    /// # use hydro_lang::*;
    /// # use dfir_rs::futures::StreamExt;
    /// # tokio_test::block_on(test_util::stream_transform_test(|process| {
    /// let words = process.source_iter(q!(vec!["hello", "world"]));
    /// words.map(q!(|x| x.to_uppercase()))
    /// # }, |mut stream| async move {
    /// # for w in vec!["HELLO", "WORLD"] {
    /// #     assert_eq!(stream.next().await.unwrap(), w);
    /// # }
    /// # }));
    /// ```
    pub fn map<U, F: Fn(T) -> U + 'a>(
        self,
        f: impl IntoQuotedMut<'a, F, L>,
    ) -> Stream<U, L, B, Order> {
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
    /// # use hydro_lang::*;
    /// # use dfir_rs::futures::StreamExt;
    /// # tokio_test::block_on(test_util::stream_transform_test(|process| {
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
    pub fn flat_map_ordered<U, I: IntoIterator<Item = U>, F: Fn(T) -> I + 'a>(
        self,
        f: impl IntoQuotedMut<'a, F, L>,
    ) -> Stream<U, L, B, Order> {
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
    /// # use hydro_lang::*;
    /// # use dfir_rs::futures::StreamExt;
    /// # tokio_test::block_on(test_util::stream_transform_test::<_, _, NoOrder>(|process| {
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
    pub fn flat_map_unordered<U, I: IntoIterator<Item = U>, F: Fn(T) -> I + 'a>(
        self,
        f: impl IntoQuotedMut<'a, F, L>,
    ) -> Stream<U, L, B, NoOrder> {
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
    /// # use hydro_lang::*;
    /// # use dfir_rs::futures::StreamExt;
    /// # tokio_test::block_on(test_util::stream_transform_test(|process| {
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
    pub fn flatten_ordered<U>(self) -> Stream<U, L, B, Order>
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
    /// # use hydro_lang::*;
    /// # use dfir_rs::futures::StreamExt;
    /// # tokio_test::block_on(test_util::stream_transform_test::<_, _, NoOrder>(|process| {
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
    pub fn flatten_unordered<U>(self) -> Stream<U, L, B, NoOrder>
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
    /// # use hydro_lang::*;
    /// # use dfir_rs::futures::StreamExt;
    /// # tokio_test::block_on(test_util::stream_transform_test(|process| {
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
    pub fn filter<F: Fn(&T) -> bool + 'a>(
        self,
        f: impl IntoQuotedMut<'a, F, L>,
    ) -> Stream<T, L, B, Order> {
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
    /// # use hydro_lang::*;
    /// # use dfir_rs::futures::StreamExt;
    /// # tokio_test::block_on(test_util::stream_transform_test(|process| {
    /// process
    ///     .source_iter(q!(vec!["1", "hello", "world", "2"]))
    ///     .filter_map(q!(|s| s.parse::<usize>().ok()))
    /// # }, |mut stream| async move {
    /// // 1, 2
    /// # for w in (1..3) {
    /// #     assert_eq!(stream.next().await.unwrap(), w);
    /// # }
    /// # }));
    pub fn filter_map<U, F: Fn(T) -> Option<U> + 'a>(
        self,
        f: impl IntoQuotedMut<'a, F, L>,
    ) -> Stream<U, L, B, Order> {
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
    /// # use hydro_lang::*;
    /// # use dfir_rs::futures::StreamExt;
    /// # tokio_test::block_on(test_util::stream_transform_test(|process| {
    /// let tick = process.tick();
    /// let batch = unsafe {
    ///     process
    ///         .source_iter(q!(vec![1, 2, 3, 4]))
    ///         .tick_batch(&tick)
    /// };
    /// let count = batch.clone().count(); // `count()` returns a singleton
    /// batch.cross_singleton(count).all_ticks()
    /// # }, |mut stream| async move {
    /// // (1, 4), (2, 4), (3, 4), (4, 4)
    /// # for w in vec![(1, 4), (2, 4), (3, 4), (4, 4)] {
    /// #     assert_eq!(stream.next().await.unwrap(), w);
    /// # }
    /// # }));
    pub fn cross_singleton<O>(
        self,
        other: impl Into<Optional<O, L, Bounded>>,
    ) -> Stream<(T, O), L, B, Order>
    where
        O: Clone,
    {
        let other: Optional<O, L, Bounded> = other.into();
        check_matching_location(&self.location, &other.location);

        Stream::new(
            self.location.clone(),
            HydroNode::CrossSingleton {
                left: Box::new(self.ir_node.into_inner()),
                right: Box::new(other.ir_node.into_inner()),
                metadata: self.location.new_node_metadata::<(T, O)>(),
            },
        )
    }

    /// Allow this stream through if the argument (a Bounded Optional) is non-empty, otherwise the output is empty.
    pub fn continue_if<U>(self, signal: Optional<U, L, Bounded>) -> Stream<T, L, B, Order> {
        self.cross_singleton(signal.map(q!(|_u| ())))
            .map(q!(|(d, _signal)| d))
    }

    /// Allow this stream through if the argument (a Bounded Optional) is empty, otherwise the output is empty.
    pub fn continue_unless<U>(self, other: Optional<U, L, Bounded>) -> Stream<T, L, B, Order> {
        self.continue_if(other.into_stream().count().filter(q!(|c| *c == 0)))
    }

    /// Forms the cross-product (Cartesian product, cross-join) of the items in the 2 input streams, returning all
    /// tupled pairs in a non-deterministic order.
    ///
    /// # Example
    /// ```rust
    /// # use hydro_lang::*;
    /// # use std::collections::HashSet;
    /// # use dfir_rs::futures::StreamExt;
    /// # tokio_test::block_on(test_util::stream_transform_test(|process| {
    /// let tick = process.tick();
    /// let stream1 = process.source_iter(q!(vec!['a', 'b', 'c']));
    /// let stream2 = process.source_iter(q!(vec![1, 2, 3]));
    /// stream1.cross_product(stream2)
    /// # }, |mut stream| async move {
    /// # let expected = HashSet::from([('a', 1), ('b', 1), ('c', 1), ('a', 2), ('b', 2), ('c', 2), ('a', 3), ('b', 3), ('c', 3)]);
    /// # stream.map(|i| assert!(expected.contains(&i)));
    /// # }));
    pub fn cross_product<O>(self, other: Stream<O, L, B, Order>) -> Stream<(T, O), L, B, NoOrder>
    where
        T: Clone,
        O: Clone,
    {
        check_matching_location(&self.location, &other.location);

        Stream::new(
            self.location.clone(),
            HydroNode::CrossProduct {
                left: Box::new(self.ir_node.into_inner()),
                right: Box::new(other.ir_node.into_inner()),
                metadata: self.location.new_node_metadata::<(T, O)>(),
            },
        )
    }

    /// Takes one stream as input and filters out any duplicate occurrences. The output
    /// contains all unique values from the input.
    ///
    /// # Example
    /// ```rust
    /// # use hydro_lang::*;
    /// # use dfir_rs::futures::StreamExt;
    /// # tokio_test::block_on(test_util::stream_transform_test(|process| {
    /// let tick = process.tick();
    ///     process.source_iter(q!(vec![1, 2, 3, 2, 1, 4])).unique()
    /// # }, |mut stream| async move {
    /// # for w in vec![1, 2, 3, 4] {
    /// #     assert_eq!(stream.next().await.unwrap(), w);
    /// # }
    /// # }));
    pub fn unique(self) -> Stream<T, L, B, Order>
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
    /// # use hydro_lang::*;
    /// # use dfir_rs::futures::StreamExt;
    /// # tokio_test::block_on(test_util::stream_transform_test(|process| {
    /// let tick = process.tick();
    /// let stream = unsafe {
    ///    process
    ///    .source_iter(q!(vec![ 1, 2, 3, 4 ]))
    ///    .tick_batch(&tick)
    /// };
    /// let batch = unsafe {
    ///     process
    ///         .source_iter(q!(vec![1, 2]))
    ///         .tick_batch(&tick)
    /// };
    /// stream.filter_not_in(batch).all_ticks()
    /// # }, |mut stream| async move {
    /// # for w in vec![3, 4] {
    /// #     assert_eq!(stream.next().await.unwrap(), w);
    /// # }
    /// # }));
    pub fn filter_not_in<O2>(self, other: Stream<T, L, Bounded, O2>) -> Stream<T, L, Bounded, Order>
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
    /// # use hydro_lang::*;
    /// # use dfir_rs::futures::StreamExt;
    /// # tokio_test::block_on(test_util::stream_transform_test(|process| {
    /// let nums = process.source_iter(q!(vec![1, 2]));
    /// // prints "1 * 10 = 10" and "2 * 10 = 20"
    /// nums.inspect(q!(|x| println!("{} * 10 = {}", x, x * 10)))
    /// # }, |mut stream| async move {
    /// # for w in vec![1, 2] {
    /// #     assert_eq!(stream.next().await.unwrap(), w);
    /// # }
    /// # }));
    /// ```
    pub fn inspect<F: Fn(&T) + 'a>(
        self,
        f: impl IntoQuotedMut<'a, F, L>,
    ) -> Stream<T, L, B, Order> {
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

    /// Explicitly "casts" the stream to a type with a different ordering
    /// guarantee. Useful in unsafe code where the ordering cannot be proven
    /// by the type-system.
    ///
    /// # Safety
    /// This function is used as an escape hatch, and any mistakes in the
    /// provided ordering guarantee will propagate into the guarantees
    /// for the rest of the program.
    ///
    /// # Example
    /// # TODO: more sensible code after Shadaj merges
    /// ```rust
    /// # use hydro_lang::*;
    /// # use std::collections::HashSet;
    /// # use dfir_rs::futures::StreamExt;
    /// # tokio_test::block_on(test_util::stream_transform_test(|process| {
    /// let nums = process.source_iter(q!({
    ///     let now = std::time::SystemTime::now();
    ///     match now.elapsed().unwrap().as_secs() % 2 {
    ///         0 => vec![5, 4, 3, 2, 1],
    ///         _ => vec![1, 2, 3, 4, 5],
    ///     }
    ///     .into_iter()
    /// }));
    /// // despite being generated by `source_iter`, the order of `nums` across runs is non-deterministic
    /// let stream = unsafe { nums.assume_ordering::<NoOrder>() };
    /// stream
    /// # }, |mut stream| async move {
    /// # for w in vec![1, 2, 3, 4, 5] {
    /// #     assert!((1..=5).contains(&stream.next().await.unwrap()));
    /// # }
    /// # }));
    /// ```
    pub unsafe fn assume_ordering<O>(self) -> Stream<T, L, B, O> {
        Stream::new(self.location, self.ir_node.into_inner())
    }
}

impl<'a, T, L: Location<'a>, B, Order> Stream<&T, L, B, Order> {
    /// Clone each element of the stream; akin to `map(q!(|d| d.clone()))`.
    ///
    /// # Example
    /// ```rust
    /// # use hydro_lang::*;
    /// # use dfir_rs::futures::StreamExt;
    /// # tokio_test::block_on(test_util::stream_transform_test(|process| {
    /// process.source_iter(q!(&[1, 2, 3])).cloned()
    /// # }, |mut stream| async move {
    /// // 1, 2, 3
    /// # for w in vec![1, 2, 3] {
    /// #     assert_eq!(stream.next().await.unwrap(), w);
    /// # }
    /// # }));
    /// ```
    pub fn cloned(self) -> Stream<T, L, B, Order>
    where
        T: Clone,
    {
        self.map(q!(|d| d.clone()))
    }
}

impl<'a, T, L: Location<'a>, B, Order> Stream<T, L, B, Order>
where
    Order: MinOrder<NoOrder, Min = NoOrder>,
{
    /// Combines elements of the stream into a [`Singleton`], by starting with an initial value,
    /// generated by the `init` closure, and then applying the `comb` closure to each element in the stream.
    /// Unlike iterators, `comb` takes the accumulator by `&mut` reference, so that it can be modified in place.
    ///
    /// The `comb` closure must be **commutative**, as the order of input items is not guaranteed.
    ///
    /// # Example
    /// ```rust
    /// # use hydro_lang::*;
    /// # use dfir_rs::futures::StreamExt;
    /// # tokio_test::block_on(test_util::stream_transform_test(|process| {
    /// let tick = process.tick();
    /// let numbers = process.source_iter(q!(vec![1, 2, 3, 4]));
    /// let batch = unsafe { numbers.tick_batch(&tick) };
    /// batch
    ///     .fold_commutative(q!(|| 0), q!(|acc, x| *acc += x))
    ///     .all_ticks()
    /// # }, |mut stream| async move {
    /// // 10
    /// # assert_eq!(stream.next().await.unwrap(), 10);
    /// # }));
    /// ```
    pub fn fold_commutative<A, I: Fn() -> A + 'a, F: Fn(&mut A, T)>(
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

    /// Combines elements of the stream into a [`Optional`], by starting with the first element in the stream,
    /// and then applying the `comb` closure to each element in the stream. The [`Optional`] will be empty
    /// until the first element in the input arrives. Unlike iterators, `comb` takes the accumulator by `&mut`
    /// reference, so that it can be modified in place.
    ///
    /// The `comb` closure must be **commutative**, as the order of input items is not guaranteed.
    ///
    /// # Example
    /// ```rust
    /// # use hydro_lang::*;
    /// # use dfir_rs::futures::StreamExt;
    /// # tokio_test::block_on(test_util::stream_transform_test(|process| {
    /// let tick = process.tick();
    /// let numbers = process.source_iter(q!(vec![1, 2, 3, 4]));
    /// let batch = unsafe { numbers.tick_batch(&tick) };
    /// batch
    ///     .reduce_commutative(q!(|curr, new| *curr += new))
    ///     .all_ticks()
    /// # }, |mut stream| async move {
    /// // 10
    /// # assert_eq!(stream.next().await.unwrap(), 10);
    /// # }));
    /// ```
    pub fn reduce_commutative<F: Fn(&mut T, T) + 'a>(
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

    /// Computes the maximum element in the stream as an [`Optional`], which
    /// will be empty until the first element in the input arrives.
    ///
    /// # Example
    /// ```rust
    /// # use hydro_lang::*;
    /// # use dfir_rs::futures::StreamExt;
    /// # tokio_test::block_on(test_util::stream_transform_test(|process| {
    /// let tick = process.tick();
    /// let numbers = process.source_iter(q!(vec![1, 2, 3, 4]));
    /// let batch = unsafe { numbers.tick_batch(&tick) };
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
        self.reduce_commutative(q!(|curr, new| {
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
    /// # use hydro_lang::*;
    /// # use dfir_rs::futures::StreamExt;
    /// # tokio_test::block_on(test_util::stream_transform_test(|process| {
    /// let tick = process.tick();
    /// let numbers = process.source_iter(q!(vec![1, 2, 3, 4]));
    /// let batch = unsafe { numbers.tick_batch(&tick) };
    /// batch.max_by_key(q!(|x| -x)).all_ticks()
    /// # }, |mut stream| async move {
    /// // 1
    /// # assert_eq!(stream.next().await.unwrap(), 1);
    /// # }));
    /// ```
    pub fn max_by_key<K: Ord, F: Fn(&T) -> K + 'a>(
        self,
        key: impl IntoQuotedMut<'a, F, L> + Copy,
    ) -> Optional<T, L, B> {
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
    /// # use hydro_lang::*;
    /// # use dfir_rs::futures::StreamExt;
    /// # tokio_test::block_on(test_util::stream_transform_test(|process| {
    /// let tick = process.tick();
    /// let numbers = process.source_iter(q!(vec![1, 2, 3, 4]));
    /// let batch = unsafe { numbers.tick_batch(&tick) };
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
        self.reduce_commutative(q!(|curr, new| {
            if new < *curr {
                *curr = new;
            }
        }))
    }

    /// Computes the number of elements in the stream as a [`Singleton`].
    ///
    /// # Example
    /// ```rust
    /// # use hydro_lang::*;
    /// # use dfir_rs::futures::StreamExt;
    /// # tokio_test::block_on(test_util::stream_transform_test(|process| {
    /// let tick = process.tick();
    /// let numbers = process.source_iter(q!(vec![1, 2, 3, 4]));
    /// let batch = unsafe { numbers.tick_batch(&tick) };
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

impl<'a, T, L: Location<'a>, B> Stream<T, L, B, TotalOrder> {
    /// Returns a stream with the current count tupled with each element in the input stream.
    ///
    /// # Example
    /// ```rust
    /// # use hydro_lang::*;
    /// # use dfir_rs::futures::StreamExt;
    /// # tokio_test::block_on(test_util::stream_transform_test::<_, _, TotalOrder>(|process| {
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
    pub fn enumerate(self) -> Stream<(usize, T), L, B, TotalOrder> {
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

    /// Computes the first element in the stream as an [`Optional`], which
    /// will be empty until the first element in the input arrives.
    ///
    /// This requires the stream to have a [`TotalOrder`] guarantee, otherwise
    /// re-ordering of elements may cause the first element to change.
    ///
    /// # Example
    /// ```rust
    /// # use hydro_lang::*;
    /// # use dfir_rs::futures::StreamExt;
    /// # tokio_test::block_on(test_util::stream_transform_test(|process| {
    /// let tick = process.tick();
    /// let numbers = process.source_iter(q!(vec![1, 2, 3, 4]));
    /// let batch = unsafe { numbers.tick_batch(&tick) };
    /// batch.first().all_ticks()
    /// # }, |mut stream| async move {
    /// // 1
    /// # assert_eq!(stream.next().await.unwrap(), 1);
    /// # }));
    /// ```
    pub fn first(self) -> Optional<T, L, B> {
        Optional::new(self.location, self.ir_node.into_inner())
    }

    /// Computes the last element in the stream as an [`Optional`], which
    /// will be empty until an element in the input arrives.
    ///
    /// This requires the stream to have a [`TotalOrder`] guarantee, otherwise
    /// re-ordering of elements may cause the last element to change.
    ///
    /// # Example
    /// ```rust
    /// # use hydro_lang::*;
    /// # use dfir_rs::futures::StreamExt;
    /// # tokio_test::block_on(test_util::stream_transform_test(|process| {
    /// let tick = process.tick();
    /// let numbers = process.source_iter(q!(vec![1, 2, 3, 4]));
    /// let batch = unsafe { numbers.tick_batch(&tick) };
    /// batch.last().all_ticks()
    /// # }, |mut stream| async move {
    /// // 4
    /// # assert_eq!(stream.next().await.unwrap(), 4);
    /// # }));
    /// ```
    pub fn last(self) -> Optional<T, L, B> {
        self.reduce(q!(|curr, new| *curr = new))
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
    /// # use hydro_lang::*;
    /// # use dfir_rs::futures::StreamExt;
    /// # tokio_test::block_on(test_util::stream_transform_test(|process| {
    /// let tick = process.tick();
    /// let words = process.source_iter(q!(vec!["HELLO", "WORLD"]));
    /// let batch = unsafe { words.tick_batch(&tick) };
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

    /// Combines elements of the stream into an [`Optional`], by starting with the first element in the stream,
    /// and then applying the `comb` closure to each element in the stream. The [`Optional`] will be empty
    /// until the first element in the input arrives.
    ///
    /// The input stream must have a [`TotalOrder`] guarantee, which means that the `comb` closure is allowed
    /// to depend on the order of elements in the stream.
    ///
    /// # Example
    /// ```rust
    /// # use hydro_lang::*;
    /// # use dfir_rs::futures::StreamExt;
    /// # tokio_test::block_on(test_util::stream_transform_test(|process| {
    /// let tick = process.tick();
    /// let words = process.source_iter(q!(vec!["HELLO", "WORLD"]));
    /// let batch = unsafe { words.tick_batch(&tick) };
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

impl<'a, T, L: Location<'a> + NoTick + NoAtomic, O> Stream<T, L, Unbounded, O> {
    /// Produces a new stream that interleaves the elements of the two input streams.
    /// The result has [`NoOrder`] because the order of interleaving is not guaranteed.
    ///
    /// Currently, both input streams must be [`Unbounded`]. When the streams are
    /// [`Bounded`], you can use [`Stream::chain`] instead.
    ///
    /// # Example
    /// ```rust
    /// # use hydro_lang::*;
    /// # use dfir_rs::futures::StreamExt;
    /// # tokio_test::block_on(test_util::stream_transform_test(|process| {
    /// let numbers = process.source_iter(q!(vec![1, 2, 3, 4]));
    /// numbers.clone().map(q!(|x| x + 1)).union(numbers)
    /// # }, |mut stream| async move {
    /// // 2, 3, 4, 5, and 1, 2, 3, 4 interleaved in unknown order
    /// # for w in vec![2, 3, 4, 5, 1, 2, 3, 4] {
    /// #     assert_eq!(stream.next().await.unwrap(), w);
    /// # }
    /// # }));
    /// ```
    pub fn union<O2>(self, other: Stream<T, L, Unbounded, O2>) -> Stream<T, L, Unbounded, NoOrder> {
        let tick = self.location.tick();
        unsafe {
            // SAFETY: Because the outputs are unordered,
            // we can interleave batches from both streams.
            self.tick_batch(&tick)
                .assume_ordering::<NoOrder>()
                .chain(other.tick_batch(&tick).assume_ordering::<NoOrder>())
                .all_ticks()
                .assume_ordering()
        }
    }
}

impl<'a, T, L: Location<'a>, Order> Stream<T, L, Bounded, Order> {
    /// Produces a new stream that emits the input elements in sorted order.
    ///
    /// The input stream can have any ordering guarantee, but the output stream
    /// will have a [`TotalOrder`] guarantee. This operator will block until all
    /// elements in the input stream are available, so it requires the input stream
    /// to be [`Bounded`].
    ///
    /// # Example
    /// ```rust
    /// # use hydro_lang::*;
    /// # use dfir_rs::futures::StreamExt;
    /// # tokio_test::block_on(test_util::stream_transform_test(|process| {
    /// let tick = process.tick();
    /// let numbers = process.source_iter(q!(vec![4, 2, 3, 1]));
    /// let batch = unsafe { numbers.tick_batch(&tick) };
    /// batch.sort().all_ticks()
    /// # }, |mut stream| async move {
    /// // 1, 2, 3, 4
    /// # for w in (1..5) {
    /// #     assert_eq!(stream.next().await.unwrap(), w);
    /// # }
    /// # }));
    /// ```
    pub fn sort(self) -> Stream<T, L, Bounded, TotalOrder>
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
    /// # use hydro_lang::*;
    /// # use dfir_rs::futures::StreamExt;
    /// # tokio_test::block_on(test_util::stream_transform_test(|process| {
    /// let tick = process.tick();
    /// let numbers = process.source_iter(q!(vec![1, 2, 3, 4]));
    /// let batch = unsafe { numbers.tick_batch(&tick) };
    /// batch.clone().map(q!(|x| x + 1)).chain(batch).all_ticks()
    /// # }, |mut stream| async move {
    /// // 2, 3, 4, 5, 1, 2, 3, 4
    /// # for w in vec![2, 3, 4, 5, 1, 2, 3, 4] {
    /// #     assert_eq!(stream.next().await.unwrap(), w);
    /// # }
    /// # }));
    /// ```
    pub fn chain<O2>(self, other: Stream<T, L, Bounded, O2>) -> Stream<T, L, Bounded, Order::Min>
    where
        Order: MinOrder<O2>,
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
}

impl<'a, K, V1, L: Location<'a>, B, Order> Stream<(K, V1), L, B, Order> {
    /// Given two streams of pairs `(K, V1)` and `(K, V2)`, produces a new stream of nested pairs `(K, (V1, V2))`
    /// by equi-joining the two streams on the key attribute `K`.
    ///
    /// # Example
    /// ```rust
    /// # use hydro_lang::*;
    /// # use std::collections::HashSet;
    /// # use dfir_rs::futures::StreamExt;
    /// # tokio_test::block_on(test_util::stream_transform_test(|process| {
    /// let tick = process.tick();
    /// let stream1 = process.source_iter(q!(vec![(1, 'a'), (2, 'b')]));
    /// let stream2 = process.source_iter(q!(vec![(1, 'x'), (2, 'y')]));
    /// stream1.join(stream2)
    /// # }, |mut stream| async move {
    /// // (1, ('a', 'x')), (2, ('b', 'y'))
    /// # let expected = HashSet::from([(1, ('a', 'x')), (2, ('b', 'y'))]);
    /// # stream.map(|i| assert!(expected.contains(&i)));
    /// # }));
    pub fn join<V2, O2>(self, n: Stream<(K, V2), L, B, O2>) -> Stream<(K, (V1, V2)), L, B, NoOrder>
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
    /// # use hydro_lang::*;
    /// # use dfir_rs::futures::StreamExt;
    /// # tokio_test::block_on(test_util::stream_transform_test(|process| {
    /// let tick = process.tick();
    /// let stream = unsafe {
    ///    process
    ///    .source_iter(q!(vec![ (1, 'a'), (2, 'b'), (3, 'c'), (4, 'd') ]))
    ///    .tick_batch(&tick)
    /// };
    /// let batch = unsafe {
    ///     process
    ///         .source_iter(q!(vec![1, 2]))
    ///         .tick_batch(&tick)
    /// };
    /// stream.anti_join(batch).all_ticks()
    /// # }, |mut stream| async move {
    /// # for w in vec![(3, 'c'), (4, 'd')] {
    /// #     assert_eq!(stream.next().await.unwrap(), w);
    /// # }
    /// # }));
    pub fn anti_join<O2>(self, n: Stream<K, L, Bounded, O2>) -> Stream<(K, V1), L, B, Order>
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

impl<'a, K: Eq + Hash, V, L: Location<'a>> Stream<(K, V), Tick<L>, Bounded> {
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
    /// # use hydro_lang::*;
    /// # use dfir_rs::futures::StreamExt;
    /// # tokio_test::block_on(test_util::stream_transform_test(|process| {
    /// let tick = process.tick();
    /// let numbers = process.source_iter(q!(vec![(1, 2), (2, 3), (1, 3), (2, 4)]));
    /// let batch = unsafe { numbers.tick_batch(&tick) };
    /// batch
    ///     .fold_keyed(q!(|| 0), q!(|acc, x| *acc += x))
    ///     .all_ticks()
    /// # }, |mut stream| async move {
    /// // (1, 5), (2, 7)
    /// # assert_eq!(stream.next().await.unwrap(), (1, 5));
    /// # assert_eq!(stream.next().await.unwrap(), (2, 7));
    /// # }));
    /// ```
    pub fn fold_keyed<A, I: Fn() -> A + 'a, F: Fn(&mut A, V) + 'a>(
        self,
        init: impl IntoQuotedMut<'a, I, Tick<L>>,
        comb: impl IntoQuotedMut<'a, F, Tick<L>>,
    ) -> Stream<(K, A), Tick<L>, Bounded> {
        let init = init.splice_fn0_ctx(&self.location).into();
        let comb = comb.splice_fn2_borrow_mut_ctx(&self.location).into();

        Stream::new(
            self.location.clone(),
            HydroNode::FoldKeyed {
                init,
                acc: comb,
                input: Box::new(self.ir_node.into_inner()),
                metadata: self.location.new_node_metadata::<(K, A)>(),
            },
        )
    }

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
    /// # use hydro_lang::*;
    /// # use dfir_rs::futures::StreamExt;
    /// # tokio_test::block_on(test_util::stream_transform_test(|process| {
    /// let tick = process.tick();
    /// let numbers = process.source_iter(q!(vec![(1, 2), (2, 3), (1, 3), (2, 4)]));
    /// let batch = unsafe { numbers.tick_batch(&tick) };
    /// batch.reduce_keyed(q!(|acc, x| *acc += x)).all_ticks()
    /// # }, |mut stream| async move {
    /// // (1, 5), (2, 7)
    /// # assert_eq!(stream.next().await.unwrap(), (1, 5));
    /// # assert_eq!(stream.next().await.unwrap(), (2, 7));
    /// # }));
    /// ```
    pub fn reduce_keyed<F: Fn(&mut V, V) + 'a>(
        self,
        comb: impl IntoQuotedMut<'a, F, Tick<L>>,
    ) -> Stream<(K, V), Tick<L>, Bounded> {
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

impl<'a, K: Eq + Hash, V, L: Location<'a>, Order> Stream<(K, V), Tick<L>, Bounded, Order> {
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
    /// # use hydro_lang::*;
    /// # use dfir_rs::futures::StreamExt;
    /// # tokio_test::block_on(test_util::stream_transform_test(|process| {
    /// let tick = process.tick();
    /// let numbers = process.source_iter(q!(vec![(1, 2), (2, 3), (1, 3), (2, 4)]));
    /// let batch = unsafe { numbers.tick_batch(&tick) };
    /// batch
    ///     .fold_keyed_commutative(q!(|| 0), q!(|acc, x| *acc += x))
    ///     .all_ticks()
    /// # }, |mut stream| async move {
    /// // (1, 5), (2, 7)
    /// # assert_eq!(stream.next().await.unwrap(), (1, 5));
    /// # assert_eq!(stream.next().await.unwrap(), (2, 7));
    /// # }));
    /// ```
    pub fn fold_keyed_commutative<A, I: Fn() -> A + 'a, F: Fn(&mut A, V) + 'a>(
        self,
        init: impl IntoQuotedMut<'a, I, Tick<L>>,
        comb: impl IntoQuotedMut<'a, F, Tick<L>>,
    ) -> Stream<(K, A), Tick<L>, Bounded, Order> {
        let init = init.splice_fn0_ctx(&self.location).into();
        let comb = comb.splice_fn2_borrow_mut_ctx(&self.location).into();

        Stream::new(
            self.location.clone(),
            HydroNode::FoldKeyed {
                init,
                acc: comb,
                input: Box::new(self.ir_node.into_inner()),
                metadata: self.location.new_node_metadata::<(K, A)>(),
            },
        )
    }

    /// Given a stream of pairs `(K, V)`, produces a new stream of unique keys `K`.
    /// # Example
    /// ```rust
    /// # use hydro_lang::*;
    /// # use dfir_rs::futures::StreamExt;
    /// # tokio_test::block_on(test_util::stream_transform_test(|process| {
    /// let tick = process.tick();
    /// let numbers = process.source_iter(q!(vec![(1, 2), (2, 3), (1, 3), (2, 4)]));
    /// let batch = unsafe { numbers.tick_batch(&tick) };
    /// batch.keys().all_ticks()
    /// # }, |mut stream| async move {
    /// // 1, 2
    /// # assert_eq!(stream.next().await.unwrap(), 1);
    /// # assert_eq!(stream.next().await.unwrap(), 2);
    /// # }));
    /// ```
    pub fn keys(self) -> Stream<K, Tick<L>, Bounded, Order> {
        self.fold_keyed_commutative(q!(|| ()), q!(|_, _| {}))
            .map(q!(|(k, _)| k))
    }

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
    /// # use hydro_lang::*;
    /// # use dfir_rs::futures::StreamExt;
    /// # tokio_test::block_on(test_util::stream_transform_test(|process| {
    /// let tick = process.tick();
    /// let numbers = process.source_iter(q!(vec![(1, 2), (2, 3), (1, 3), (2, 4)]));
    /// let batch = unsafe { numbers.tick_batch(&tick) };
    /// batch
    ///     .reduce_keyed_commutative(q!(|acc, x| *acc += x))
    ///     .all_ticks()
    /// # }, |mut stream| async move {
    /// // (1, 5), (2, 7)
    /// # assert_eq!(stream.next().await.unwrap(), (1, 5));
    /// # assert_eq!(stream.next().await.unwrap(), (2, 7));
    /// # }));
    /// ```
    pub fn reduce_keyed_commutative<F: Fn(&mut V, V) + 'a>(
        self,
        comb: impl IntoQuotedMut<'a, F, Tick<L>>,
    ) -> Stream<(K, V), Tick<L>, Bounded, Order> {
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

impl<'a, T, L: Location<'a> + NoTick, B, Order> Stream<T, Atomic<L>, B, Order> {
    /// Returns a stream corresponding to the latest batch of elements being atomically
    /// processed. These batches are guaranteed to be contiguous across ticks and preserve
    /// the order of the input.
    ///
    /// # Safety
    /// The batch boundaries are non-deterministic and may change across executions.
    pub unsafe fn tick_batch(self) -> Stream<T, Tick<L>, Bounded, Order> {
        Stream::new(
            self.location.clone().tick,
            HydroNode::Unpersist {
                inner: Box::new(self.ir_node.into_inner()),
                metadata: self.location.new_node_metadata::<T>(),
            },
        )
    }

    pub fn end_atomic(self) -> Stream<T, L, B, Order> {
        Stream::new(self.location.tick.l, self.ir_node.into_inner())
    }

    pub fn atomic_source(&self) -> Tick<L> {
        self.location.tick.clone()
    }
}

impl<'a, T, L: Location<'a> + NoTick + NoAtomic, B, Order> Stream<T, L, B, Order> {
    pub fn atomic(self, tick: &Tick<L>) -> Stream<T, Atomic<L>, B, Order> {
        Stream::new(Atomic { tick: tick.clone() }, self.ir_node.into_inner())
    }

    /// Consumes a stream of `Future<T>`, produces a new stream of the resulting `T` outputs.
    /// Future outputs are produced as available, regardless of input arrival order.
    ///
    /// # Example
    /// ```rust
    /// # use std::collections::HashSet;
    /// # use dfir_rs::futures::StreamExt;
    /// # use hydro_lang::*;
    /// # tokio_test::block_on(test_util::stream_transform_test(|process| {
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
    pub fn resolve_futures<T2>(self) -> Stream<T2, L, B, NoOrder>
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

    /// Consumes a stream of `Future<T>`, produces a new stream of the resulting `T` outputs.
    /// Future outputs are produced in the same order as the input stream.
    ///
    /// # Example
    /// ```rust
    /// # use std::collections::HashSet;
    /// # use dfir_rs::futures::StreamExt;
    /// # use hydro_lang::*;
    /// # tokio_test::block_on(test_util::stream_transform_test(|process| {
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
    pub fn resolve_futures_ordered<T2>(self) -> Stream<T2, L, B, Order>
    where
        T: Future<Output = T2>,
    {
        Stream::new(
            self.location.clone(),
            HydroNode::ResolveFuturesOrdered {
                input: Box::new(self.ir_node.into_inner()),
                metadata: self.location.new_node_metadata::<T2>(),
            },
        )
    }

    /// Given a tick, returns a stream corresponding to a batch of elements segmented by
    /// that tick. These batches are guaranteed to be contiguous across ticks and preserve
    /// the order of the input.
    ///
    /// # Safety
    /// The batch boundaries are non-deterministic and may change across executions.
    pub unsafe fn tick_batch(self, tick: &Tick<L>) -> Stream<T, Tick<L>, Bounded, Order> {
        unsafe { self.atomic(tick).tick_batch() }
    }

    /// Given a time interval, returns a stream corresponding to samples taken from the
    /// stream roughly at that interval. The output will have elements in the same order
    /// as the input, but with arbitrary elements skipped between samples. There is also
    /// no guarantee on the exact timing of the samples.
    ///
    /// # Safety
    /// The output stream is non-deterministic in which elements are sampled, since this
    /// is controlled by a clock.
    pub unsafe fn sample_every(
        self,
        interval: impl QuotedWithContext<'a, std::time::Duration, L> + Copy + 'a,
    ) -> Stream<T, L, Unbounded, Order> {
        let samples = unsafe {
            // SAFETY: source of intentional non-determinism
            self.location.source_interval(interval)
        };

        let tick = self.location.tick();
        unsafe {
            // SAFETY: source of intentional non-determinism
            self.tick_batch(&tick)
                .continue_if(samples.tick_batch(&tick).first())
                .all_ticks()
        }
    }

    /// Given a timeout duration, returns an [`Optional`]  which will have a value if the
    /// stream has not emitted a value since that duration.
    ///
    /// # Safety
    /// Timeout relies on non-deterministic sampling of the stream, so depending on when
    /// samples take place, timeouts may be non-deterministically generated or missed,
    /// and the notification of the timeout may be delayed as well. There is also no
    /// guarantee on how long the [`Optional`] will have a value after the timeout is
    /// detected based on when the next sample is taken.
    pub unsafe fn timeout(
        self,
        duration: impl QuotedWithContext<'a, std::time::Duration, Tick<L>> + Copy + 'a,
    ) -> Optional<(), L, Unbounded>
    where
        Order: MinOrder<NoOrder, Min = NoOrder>,
    {
        let tick = self.location.tick();

        let latest_received = self.fold_commutative(
            q!(|| None),
            q!(|latest, _| {
                *latest = Some(Instant::now());
            }),
        );

        unsafe {
            // SAFETY: Non-deterministic delay in detecting a timeout is expected.
            latest_received.latest_tick(&tick)
        }
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

impl<'a, T, L: Location<'a> + NoTick, B, Order> Stream<T, L, B, Order> {
    pub fn for_each<F: Fn(T) + 'a>(self, f: impl IntoQuotedMut<'a, F, L>) {
        let f = f.splice_fn1_ctx(&self.location).into();
        let metadata = self.location.new_node_metadata::<T>();
        self.location
            .flow_state()
            .borrow_mut()
            .leaves
            .as_mut()
            .expect(FLOW_USED_MESSAGE)
            .push(HydroLeaf::ForEach {
                input: Box::new(HydroNode::Unpersist {
                    inner: Box::new(self.ir_node.into_inner()),
                    metadata: metadata.clone(),
                }),
                f,
                metadata,
            });
    }

    pub fn dest_sink<S: Unpin + futures::Sink<T> + 'a>(
        self,
        sink: impl QuotedWithContext<'a, S, L>,
    ) {
        self.location
            .flow_state()
            .borrow_mut()
            .leaves
            .as_mut()
            .expect(FLOW_USED_MESSAGE)
            .push(HydroLeaf::DestSink {
                sink: sink.splice_typed_ctx(&self.location).into(),
                input: Box::new(self.ir_node.into_inner()),
                metadata: self.location.new_node_metadata::<T>(),
            });
    }
}

impl<'a, T, L: Location<'a>, Order> Stream<T, Tick<L>, Bounded, Order> {
    pub fn all_ticks(self) -> Stream<T, L, Unbounded, Order> {
        Stream::new(
            self.location.outer().clone(),
            HydroNode::Persist {
                inner: Box::new(self.ir_node.into_inner()),
                metadata: self.location.new_node_metadata::<T>(),
            },
        )
    }

    pub fn all_ticks_atomic(self) -> Stream<T, Atomic<L>, Unbounded, Order> {
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

    pub fn persist(self) -> Stream<T, Tick<L>, Bounded, Order>
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

    pub fn defer_tick(self) -> Stream<T, Tick<L>, Bounded, Order> {
        Stream::new(
            self.location.clone(),
            HydroNode::DeferTick {
                input: Box::new(self.ir_node.into_inner()),
                metadata: self.location.new_node_metadata::<T>(),
            },
        )
    }

    pub fn delta(self) -> Stream<T, Tick<L>, Bounded, Order> {
        Stream::new(
            self.location.clone(),
            HydroNode::Delta {
                inner: Box::new(self.ir_node.into_inner()),
                metadata: self.location.new_node_metadata::<T>(),
            },
        )
    }
}

pub fn serialize_bincode_with_type(is_demux: bool, t_type: syn::Type) -> syn::Expr {
    let root = get_this_crate();

    if is_demux {
        parse_quote! {
            ::#root::runtime_support::stageleft::runtime_support::fn1_type_hint::<(#root::ClusterId<_>, #t_type), _>(
                |(id, data)| {
                    (id.raw_id, #root::runtime_support::bincode::serialize(&data).unwrap().into())
                }
            )
        }
    } else {
        parse_quote! {
            ::#root::runtime_support::stageleft::runtime_support::fn1_type_hint::<#t_type, _>(
                |data| {
                    #root::runtime_support::bincode::serialize(&data).unwrap().into()
                }
            )
        }
    }
}

fn serialize_bincode<T: Serialize>(is_demux: bool) -> syn::Expr {
    serialize_bincode_with_type(is_demux, stageleft::quote_type::<T>())
}

pub fn deserialize_bincode_with_type(tagged: Option<syn::Type>, t_type: syn::Type) -> syn::Expr {
    let root = get_this_crate();

    if let Some(c_type) = tagged {
        parse_quote! {
            |res| {
                let (id, b) = res.unwrap();
                (#root::ClusterId::<#c_type>::from_raw(id), #root::runtime_support::bincode::deserialize::<#t_type>(&b).unwrap())
            }
        }
    } else {
        parse_quote! {
            |res| {
                #root::runtime_support::bincode::deserialize::<#t_type>(&res.unwrap()).unwrap()
            }
        }
    }
}

pub(super) fn deserialize_bincode<T: DeserializeOwned>(tagged: Option<syn::Type>) -> syn::Expr {
    deserialize_bincode_with_type(tagged, stageleft::quote_type::<T>())
}

impl<'a, T, L: Location<'a> + NoTick, B, Order> Stream<T, L, B, Order> {
    pub fn send_bincode<L2: Location<'a>, CoreType>(
        self,
        other: &L2,
    ) -> Stream<<L::Root as CanSend<'a, L2>>::Out<CoreType>, L2, Unbounded, Order::Min>
    where
        L::Root: CanSend<'a, L2, In<CoreType> = T>,
        CoreType: Serialize + DeserializeOwned,
        Order: MinOrder<<L::Root as CanSend<'a, L2>>::OutStrongestOrder<Order>>,
    {
        let serialize_pipeline = Some(serialize_bincode::<CoreType>(L::Root::is_demux()));

        let deserialize_pipeline = Some(deserialize_bincode::<CoreType>(L::Root::tagged_type()));

        Stream::new(
            other.clone(),
            HydroNode::Network {
                from_key: None,
                to_location: other.id(),
                to_key: None,
                serialize_fn: serialize_pipeline.map(|e| e.into()),
                instantiate_fn: DebugInstantiate::Building,
                deserialize_fn: deserialize_pipeline.map(|e| e.into()),
                input: Box::new(self.ir_node.into_inner()),
                metadata: other.new_node_metadata::<CoreType>(),
            },
        )
    }

    pub fn send_bincode_external<L2: 'a, CoreType>(
        self,
        other: &ExternalProcess<L2>,
    ) -> ExternalBincodeStream<L::Out<CoreType>>
    where
        L: CanSend<'a, ExternalProcess<'a, L2>, In<CoreType> = T, Out<CoreType> = CoreType>,
        CoreType: Serialize + DeserializeOwned,
        // for now, we restirct Out<CoreType> to be CoreType, which means no tagged cluster -> external
    {
        let serialize_pipeline = Some(serialize_bincode::<CoreType>(L::is_demux()));

        let metadata = other.new_node_metadata::<CoreType>();

        let mut flow_state_borrow = self.location.flow_state().borrow_mut();

        let external_key = flow_state_borrow.next_external_out;
        flow_state_borrow.next_external_out += 1;

        let leaves = flow_state_borrow.leaves.as_mut().expect("Attempted to add a leaf to a flow that has already been finalized. No leaves can be added after the flow has been compiled()");

        let dummy_f: syn::Expr = syn::parse_quote!(());

        leaves.push(HydroLeaf::ForEach {
            f: dummy_f.into(),
            input: Box::new(HydroNode::Network {
                from_key: None,
                to_location: other.id(),
                to_key: Some(external_key),
                serialize_fn: serialize_pipeline.map(|e| e.into()),
                instantiate_fn: DebugInstantiate::Building,
                deserialize_fn: None,
                input: Box::new(self.ir_node.into_inner()),
                metadata: metadata.clone(),
            }),
            metadata,
        });

        ExternalBincodeStream {
            process_id: other.id,
            port_id: external_key,
            _phantom: PhantomData,
        }
    }

    pub fn send_bytes<L2: Location<'a>>(
        self,
        other: &L2,
    ) -> Stream<<L::Root as CanSend<'a, L2>>::Out<Bytes>, L2, Unbounded, Order::Min>
    where
        L::Root: CanSend<'a, L2, In<Bytes> = T>,
        Order: MinOrder<<L::Root as CanSend<'a, L2>>::OutStrongestOrder<Order>>,
    {
        let root = get_this_crate();
        Stream::new(
            other.clone(),
            HydroNode::Network {
                from_key: None,
                to_location: other.id(),
                to_key: None,
                serialize_fn: None,
                instantiate_fn: DebugInstantiate::Building,
                deserialize_fn: if let Some(c_type) = L::Root::tagged_type() {
                    let expr: syn::Expr = parse_quote!(|(id, b)| (#root::ClusterId<#c_type>::from_raw(id), b.unwrap().freeze()));
                    Some(expr.into())
                } else {
                    let expr: syn::Expr = parse_quote!(|b| b.unwrap().freeze());
                    Some(expr.into())
                },
                input: Box::new(self.ir_node.into_inner()),
                metadata: other.new_node_metadata::<Bytes>(),
            },
        )
    }

    pub fn send_bytes_external<L2: 'a>(self, other: &ExternalProcess<L2>) -> ExternalBytesPort
    where
        L::Root: CanSend<'a, ExternalProcess<'a, L2>, In<Bytes> = T, Out<Bytes> = Bytes>,
    {
        let metadata = other.new_node_metadata::<Bytes>();

        let mut flow_state_borrow = self.location.flow_state().borrow_mut();
        let external_key = flow_state_borrow.next_external_out;
        flow_state_borrow.next_external_out += 1;

        let leaves = flow_state_borrow.leaves.as_mut().expect("Attempted to add a leaf to a flow that has already been finalized. No leaves can be added after the flow has been compiled()");

        let dummy_f: syn::Expr = syn::parse_quote!(());

        leaves.push(HydroLeaf::ForEach {
            f: dummy_f.into(),
            input: Box::new(HydroNode::Network {
                from_key: None,
                to_location: other.id(),
                to_key: Some(external_key),
                serialize_fn: None,
                instantiate_fn: DebugInstantiate::Building,
                deserialize_fn: None,
                input: Box::new(self.ir_node.into_inner()),
                metadata: metadata.clone(),
            }),
            metadata,
        });

        ExternalBytesPort {
            process_id: other.id,
            port_id: external_key,
        }
    }

    pub fn send_bincode_anonymous<L2: Location<'a>, Tag, CoreType>(
        self,
        other: &L2,
    ) -> Stream<CoreType, L2, Unbounded, Order::Min>
    where
        L::Root: CanSend<'a, L2, In<CoreType> = T, Out<CoreType> = (Tag, CoreType)>,
        CoreType: Serialize + DeserializeOwned,
        Order: MinOrder<<L::Root as CanSend<'a, L2>>::OutStrongestOrder<Order>>,
    {
        self.send_bincode::<L2, CoreType>(other).map(q!(|(_, b)| b))
    }

    pub fn send_bytes_anonymous<L2: Location<'a>, Tag>(
        self,
        other: &L2,
    ) -> Stream<Bytes, L2, Unbounded, Order::Min>
    where
        L::Root: CanSend<'a, L2, In<Bytes> = T, Out<Bytes> = (Tag, Bytes)>,
        Order: MinOrder<<L::Root as CanSend<'a, L2>>::OutStrongestOrder<Order>>,
    {
        self.send_bytes::<L2>(other).map(q!(|(_, b)| b))
    }

    #[expect(clippy::type_complexity, reason = "ordering semantics for broadcast")]
    pub fn broadcast_bincode<C2: 'a>(
        self,
        other: &Cluster<'a, C2>,
    ) -> Stream<
        <L::Root as CanSend<'a, Cluster<'a, C2>>>::Out<T>,
        Cluster<'a, C2>,
        Unbounded,
        Order::Min,
    >
    where
        L::Root: CanSend<'a, Cluster<'a, C2>, In<T> = (ClusterId<C2>, T)>,
        T: Clone + Serialize + DeserializeOwned,
        Order: MinOrder<<L::Root as CanSend<'a, Cluster<'a, C2>>>::OutStrongestOrder<Order>>,
    {
        let ids = other.members();

        self.flat_map_ordered(q!(|b| ids.iter().map(move |id| (
            ::std::clone::Clone::clone(id),
            ::std::clone::Clone::clone(&b)
        ))))
        .send_bincode(other)
    }

    pub fn broadcast_bincode_anonymous<C2: 'a, Tag>(
        self,
        other: &Cluster<'a, C2>,
    ) -> Stream<T, Cluster<'a, C2>, Unbounded, Order::Min>
    where
        L::Root: CanSend<'a, Cluster<'a, C2>, In<T> = (ClusterId<C2>, T), Out<T> = (Tag, T)> + 'a,
        T: Clone + Serialize + DeserializeOwned,
        Order: MinOrder<<L::Root as CanSend<'a, Cluster<'a, C2>>>::OutStrongestOrder<Order>>,
    {
        self.broadcast_bincode(other).map(q!(|(_, b)| b))
    }

    #[expect(clippy::type_complexity, reason = "ordering semantics for broadcast")]
    pub fn broadcast_bytes<C2: 'a>(
        self,
        other: &Cluster<'a, C2>,
    ) -> Stream<
        <L::Root as CanSend<'a, Cluster<'a, C2>>>::Out<Bytes>,
        Cluster<'a, C2>,
        Unbounded,
        Order::Min,
    >
    where
        L::Root: CanSend<'a, Cluster<'a, C2>, In<Bytes> = (ClusterId<C2>, T)> + 'a,
        T: Clone,
        Order: MinOrder<<L::Root as CanSend<'a, Cluster<'a, C2>>>::OutStrongestOrder<Order>>,
    {
        let ids = other.members();

        self.flat_map_ordered(q!(|b| ids.iter().map(move |id| (
            ::std::clone::Clone::clone(id),
            ::std::clone::Clone::clone(&b)
        ))))
        .send_bytes(other)
    }

    pub fn broadcast_bytes_anonymous<C2: 'a, Tag>(
        self,
        other: &Cluster<'a, C2>,
    ) -> Stream<Bytes, Cluster<'a, C2>, Unbounded, Order::Min>
    where
        L::Root: CanSend<'a, Cluster<'a, C2>, In<Bytes> = (ClusterId<C2>, T), Out<Bytes> = (Tag, Bytes)>
            + 'a,
        T: Clone,
        Order: MinOrder<<L::Root as CanSend<'a, Cluster<'a, C2>>>::OutStrongestOrder<Order>>,
    {
        self.broadcast_bytes(other).map(q!(|(_, b)| b))
    }
}

#[expect(clippy::type_complexity, reason = "ordering semantics for round-robin")]
impl<'a, T, L: Location<'a> + NoTick, B> Stream<T, L, B, TotalOrder> {
    pub fn round_robin_bincode<C2: 'a>(
        self,
        other: &Cluster<'a, C2>,
    ) -> Stream<
        <L::Root as CanSend<'a, Cluster<'a, C2>>>::Out<T>,
        Cluster<'a, C2>,
        Unbounded,
        <TotalOrder as MinOrder<
            <L::Root as CanSend<'a, Cluster<'a, C2>>>::OutStrongestOrder<TotalOrder>,
        >>::Min,
    >
    where
        L::Root: CanSend<'a, Cluster<'a, C2>, In<T> = (ClusterId<C2>, T)>,
        T: Clone + Serialize + DeserializeOwned,
        TotalOrder:
            MinOrder<<L::Root as CanSend<'a, Cluster<'a, C2>>>::OutStrongestOrder<TotalOrder>>,
    {
        let ids = other.members();

        self.enumerate()
            .map(q!(|(i, w)| (ids[i % ids.len()], w)))
            .send_bincode(other)
    }

    pub fn round_robin_bincode_anonymous<C2: 'a, Tag>(
        self,
        other: &Cluster<'a, C2>,
    ) -> Stream<
        T,
        Cluster<'a, C2>,
        Unbounded,
        <TotalOrder as MinOrder<
            <L::Root as CanSend<'a, Cluster<'a, C2>>>::OutStrongestOrder<TotalOrder>,
        >>::Min,
    >
    where
        L::Root: CanSend<'a, Cluster<'a, C2>, In<T> = (ClusterId<C2>, T), Out<T> = (Tag, T)> + 'a,
        T: Clone + Serialize + DeserializeOwned,
        TotalOrder:
            MinOrder<<L::Root as CanSend<'a, Cluster<'a, C2>>>::OutStrongestOrder<TotalOrder>>,
    {
        self.round_robin_bincode(other).map(q!(|(_, b)| b))
    }

    pub fn round_robin_bytes<C2: 'a>(
        self,
        other: &Cluster<'a, C2>,
    ) -> Stream<
        <L::Root as CanSend<'a, Cluster<'a, C2>>>::Out<Bytes>,
        Cluster<'a, C2>,
        Unbounded,
        <TotalOrder as MinOrder<
            <L::Root as CanSend<'a, Cluster<'a, C2>>>::OutStrongestOrder<TotalOrder>,
        >>::Min,
    >
    where
        L::Root: CanSend<'a, Cluster<'a, C2>, In<Bytes> = (ClusterId<C2>, T)> + 'a,
        T: Clone,
        TotalOrder:
            MinOrder<<L::Root as CanSend<'a, Cluster<'a, C2>>>::OutStrongestOrder<TotalOrder>>,
    {
        let ids = other.members();

        self.enumerate()
            .map(q!(|(i, w)| (ids[i % ids.len()], w)))
            .send_bytes(other)
    }

    pub fn round_robin_bytes_anonymous<C2: 'a, Tag>(
        self,
        other: &Cluster<'a, C2>,
    ) -> Stream<
        Bytes,
        Cluster<'a, C2>,
        Unbounded,
        <TotalOrder as MinOrder<
            <L::Root as CanSend<'a, Cluster<'a, C2>>>::OutStrongestOrder<TotalOrder>,
        >>::Min,
    >
    where
        L::Root: CanSend<'a, Cluster<'a, C2>, In<Bytes> = (ClusterId<C2>, T), Out<Bytes> = (Tag, Bytes)>
            + 'a,
        T: Clone,
        TotalOrder:
            MinOrder<<L::Root as CanSend<'a, Cluster<'a, C2>>>::OutStrongestOrder<TotalOrder>>,
    {
        self.round_robin_bytes(other).map(q!(|(_, b)| b))
    }
}

#[cfg(test)]
mod tests {
    use dfir_rs::futures::StreamExt;
    use hydro_deploy::Deployment;
    use serde::{Deserialize, Serialize};
    use stageleft::q;

    use crate::FlowBuilder;
    use crate::location::Location;

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
        let external = flow.external_process::<P2>();

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
}
