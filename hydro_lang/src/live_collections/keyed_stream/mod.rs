//! Definitions for the [`KeyedStream`] live collection.

use std::cell::RefCell;
use std::collections::HashMap;
use std::hash::Hash;
use std::marker::PhantomData;
use std::ops::Deref;
use std::rc::Rc;

use stageleft::{IntoQuotedMut, QuotedWithContext, q};

use super::boundedness::{Bounded, Boundedness, Unbounded};
use super::keyed_singleton::KeyedSingleton;
use super::optional::Optional;
use super::stream::{ExactlyOnce, MinOrder, MinRetries, NoOrder, Stream, TotalOrder};
use crate::compile::ir::{
    CollectionKind, HydroIrOpMetadata, HydroNode, HydroRoot, StreamOrder, StreamRetry, TeeNode,
};
#[cfg(stageleft_runtime)]
use crate::forward_handle::{CycleCollection, ReceiverComplete};
use crate::forward_handle::{ForwardRef, TickCycle};
use crate::live_collections::stream::{Ordering, Retries};
#[cfg(stageleft_runtime)]
use crate::location::dynamic::{DynLocation, LocationId};
use crate::location::tick::DeferTick;
use crate::location::{Atomic, Location, NoTick, Tick, check_matching_location};
use crate::manual_expr::ManualExpr;
use crate::nondet::{NonDet, nondet};

pub mod networking;

/// Streaming elements of type `V` grouped by a key of type `K`.
///
/// Keyed Streams capture streaming elements of type `V` grouped by a key of type `K`, where the
/// order of keys is non-deterministic but the order *within* each group may be deterministic.
///
/// Although keyed streams are conceptually grouped by keys, values are not immediately grouped
/// into buckets when constructing a keyed stream. Instead, keyed streams defer grouping until an
/// operator such as [`KeyedStream::fold`] is called, which requires `K: Hash + Eq`.
///
/// Type Parameters:
/// - `K`: the type of the key for each group
/// - `V`: the type of the elements inside each group
/// - `Loc`: the [`Location`] where the keyed stream is materialized
/// - `Bound`: tracks whether the entries are [`Bounded`] (local and finite) or [`Unbounded`] (asynchronous and possibly infinite)
/// - `Order`: tracks whether the elements within each group have deterministic order
///   ([`TotalOrder`]) or not ([`NoOrder`])
/// - `Retries`: tracks whether the elements within each group have deterministic cardinality
///   ([`ExactlyOnce`]) or may have non-deterministic retries ([`crate::live_collections::stream::AtLeastOnce`])
pub struct KeyedStream<
    K,
    V,
    Loc,
    Bound: Boundedness = Unbounded,
    Order: Ordering = TotalOrder,
    Retry: Retries = ExactlyOnce,
> {
    pub(crate) location: Loc,
    pub(crate) ir_node: RefCell<HydroNode>,

    _phantom: PhantomData<(K, V, Loc, Bound, Order, Retry)>,
}

impl<'a, K, V, L, B: Boundedness, R: Retries> From<KeyedStream<K, V, L, B, TotalOrder, R>>
    for KeyedStream<K, V, L, B, NoOrder, R>
where
    L: Location<'a>,
{
    fn from(stream: KeyedStream<K, V, L, B, TotalOrder, R>) -> KeyedStream<K, V, L, B, NoOrder, R> {
        KeyedStream {
            location: stream.location,
            ir_node: stream.ir_node,
            _phantom: PhantomData,
        }
    }
}

impl<'a, K, V, L, O: Ordering, R: Retries> DeferTick for KeyedStream<K, V, Tick<L>, Bounded, O, R>
where
    L: Location<'a>,
{
    fn defer_tick(self) -> Self {
        KeyedStream::defer_tick(self)
    }
}

impl<'a, K, V, L, O: Ordering, R: Retries> CycleCollection<'a, TickCycle>
    for KeyedStream<K, V, Tick<L>, Bounded, O, R>
where
    L: Location<'a>,
{
    type Location = Tick<L>;

    fn create_source(ident: syn::Ident, location: Tick<L>) -> Self {
        KeyedStream {
            location: location.clone(),
            ir_node: RefCell::new(HydroNode::CycleSource {
                ident,
                metadata: location.new_node_metadata(
                    KeyedStream::<K, V, Tick<L>, Bounded, O, R>::collection_kind(),
                ),
            }),
            _phantom: PhantomData,
        }
    }
}

impl<'a, K, V, L, O: Ordering, R: Retries> ReceiverComplete<'a, TickCycle>
    for KeyedStream<K, V, Tick<L>, Bounded, O, R>
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

impl<'a, K, V, L, B: Boundedness, O: Ordering, R: Retries> CycleCollection<'a, ForwardRef>
    for KeyedStream<K, V, L, B, O, R>
where
    L: Location<'a> + NoTick,
{
    type Location = L;

    fn create_source(ident: syn::Ident, location: L) -> Self {
        KeyedStream {
            location: location.clone(),
            ir_node: RefCell::new(HydroNode::CycleSource {
                ident,
                metadata: location
                    .new_node_metadata(KeyedStream::<K, V, L, B, O, R>::collection_kind()),
            }),
            _phantom: PhantomData,
        }
    }
}

impl<'a, K, V, L, B: Boundedness, O: Ordering, R: Retries> ReceiverComplete<'a, ForwardRef>
    for KeyedStream<K, V, L, B, O, R>
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

impl<'a, K: Clone, V: Clone, Loc: Location<'a>, Bound: Boundedness, Order: Ordering, R: Retries>
    Clone for KeyedStream<K, V, Loc, Bound, Order, R>
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
            KeyedStream {
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

impl<'a, K, V, L: Location<'a>, B: Boundedness, O: Ordering, R: Retries>
    KeyedStream<K, V, L, B, O, R>
{
    pub(crate) fn new(location: L, ir_node: HydroNode) -> Self {
        debug_assert_eq!(ir_node.metadata().location_kind, Location::id(&location));
        debug_assert_eq!(ir_node.metadata().collection_kind, Self::collection_kind());

        KeyedStream {
            location,
            ir_node: RefCell::new(ir_node),
            _phantom: PhantomData,
        }
    }

    /// Returns the [`CollectionKind`] corresponding to this type.
    pub fn collection_kind() -> CollectionKind {
        CollectionKind::KeyedStream {
            bound: B::BOUND_KIND,
            value_order: O::ORDERING_KIND,
            value_retry: R::RETRIES_KIND,
            key_type: stageleft::quote_type::<K>().into(),
            value_type: stageleft::quote_type::<V>().into(),
        }
    }

    /// Returns the [`Location`] where this keyed stream is being materialized.
    pub fn location(&self) -> &L {
        &self.location
    }

    /// Explicitly "casts" the keyed stream to a type with a different ordering
    /// guarantee for each group. Useful in unsafe code where the ordering cannot be proven
    /// by the type-system.
    ///
    /// # Non-Determinism
    /// This function is used as an escape hatch, and any mistakes in the
    /// provided ordering guarantee will propagate into the guarantees
    /// for the rest of the program.
    pub fn assume_ordering<O2: Ordering>(self, _nondet: NonDet) -> KeyedStream<K, V, L, B, O2, R> {
        if O::ORDERING_KIND == O2::ORDERING_KIND {
            KeyedStream::new(self.location, self.ir_node.into_inner())
        } else if O2::ORDERING_KIND == StreamOrder::NoOrder {
            // We can always weaken the ordering guarantee
            KeyedStream::new(
                self.location.clone(),
                HydroNode::Cast {
                    inner: Box::new(self.ir_node.into_inner()),
                    metadata: self
                        .location
                        .new_node_metadata(KeyedStream::<K, V, L, B, O2, R>::collection_kind()),
                },
            )
        } else {
            KeyedStream::new(
                self.location.clone(),
                HydroNode::ObserveNonDet {
                    inner: Box::new(self.ir_node.into_inner()),
                    trusted: false,
                    metadata: self
                        .location
                        .new_node_metadata(KeyedStream::<K, V, L, B, O2, R>::collection_kind()),
                },
            )
        }
    }

    fn assume_ordering_trusted<O2: Ordering>(
        self,
        _nondet: NonDet,
    ) -> KeyedStream<K, V, L, B, O2, R> {
        if O::ORDERING_KIND == O2::ORDERING_KIND {
            KeyedStream::new(self.location, self.ir_node.into_inner())
        } else if O2::ORDERING_KIND == StreamOrder::NoOrder {
            // We can always weaken the ordering guarantee
            KeyedStream::new(
                self.location.clone(),
                HydroNode::Cast {
                    inner: Box::new(self.ir_node.into_inner()),
                    metadata: self
                        .location
                        .new_node_metadata(KeyedStream::<K, V, L, B, O2, R>::collection_kind()),
                },
            )
        } else {
            KeyedStream::new(
                self.location.clone(),
                HydroNode::ObserveNonDet {
                    inner: Box::new(self.ir_node.into_inner()),
                    trusted: true,
                    metadata: self
                        .location
                        .new_node_metadata(KeyedStream::<K, V, L, B, O2, R>::collection_kind()),
                },
            )
        }
    }

    /// Weakens the ordering guarantee provided by the stream to [`NoOrder`],
    /// which is always safe because that is the weakest possible guarantee.
    pub fn weakest_ordering(self) -> KeyedStream<K, V, L, B, NoOrder, R> {
        let nondet = nondet!(/** this is a weaker ordering guarantee, so it is safe to assume */);
        self.assume_ordering::<NoOrder>(nondet)
    }

    /// Explicitly "casts" the keyed stream to a type with a different retries
    /// guarantee for each group. Useful in unsafe code where the lack of retries cannot
    /// be proven by the type-system.
    ///
    /// # Non-Determinism
    /// This function is used as an escape hatch, and any mistakes in the
    /// provided retries guarantee will propagate into the guarantees
    /// for the rest of the program.
    pub fn assume_retries<R2: Retries>(self, _nondet: NonDet) -> KeyedStream<K, V, L, B, O, R2> {
        if R::RETRIES_KIND == R2::RETRIES_KIND {
            KeyedStream::new(self.location, self.ir_node.into_inner())
        } else if R2::RETRIES_KIND == StreamRetry::AtLeastOnce {
            // We can always weaken the retries guarantee
            KeyedStream::new(
                self.location.clone(),
                HydroNode::Cast {
                    inner: Box::new(self.ir_node.into_inner()),
                    metadata: self
                        .location
                        .new_node_metadata(KeyedStream::<K, V, L, B, O, R2>::collection_kind()),
                },
            )
        } else {
            KeyedStream::new(
                self.location.clone(),
                HydroNode::ObserveNonDet {
                    inner: Box::new(self.ir_node.into_inner()),
                    trusted: false,
                    metadata: self
                        .location
                        .new_node_metadata(KeyedStream::<K, V, L, B, O, R2>::collection_kind()),
                },
            )
        }
    }

    /// Flattens the keyed stream into an unordered stream of key-value pairs.
    ///
    /// # Example
    /// ```rust
    /// # #[cfg(feature = "deploy")] {
    /// # use hydro_lang::prelude::*;
    /// # use futures::StreamExt;
    /// # tokio_test::block_on(hydro_lang::test_util::stream_transform_test(|process| {
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
    /// # }
    /// ```
    pub fn entries(self) -> Stream<(K, V), L, B, NoOrder, R> {
        Stream::new(
            self.location.clone(),
            HydroNode::Cast {
                inner: Box::new(self.ir_node.into_inner()),
                metadata: self
                    .location
                    .new_node_metadata(Stream::<(K, V), L, B, NoOrder, R>::collection_kind()),
            },
        )
    }

    /// Flattens the keyed stream into an unordered stream of only the values.
    ///
    /// # Example
    /// ```rust
    /// # #[cfg(feature = "deploy")] {
    /// # use hydro_lang::prelude::*;
    /// # use futures::StreamExt;
    /// # tokio_test::block_on(hydro_lang::test_util::stream_transform_test(|process| {
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
    /// # }
    /// ```
    pub fn values(self) -> Stream<V, L, B, NoOrder, R> {
        self.entries().map(q!(|(_, v)| v))
    }

    /// Transforms each value by invoking `f` on each element, with keys staying the same
    /// after transformation. If you need access to the key, see [`KeyedStream::map_with_key`].
    ///
    /// If you do not want to modify the stream and instead only want to view
    /// each item use [`KeyedStream::inspect`] instead.
    ///
    /// # Example
    /// ```rust
    /// # #[cfg(feature = "deploy")] {
    /// # use hydro_lang::prelude::*;
    /// # use futures::StreamExt;
    /// # tokio_test::block_on(hydro_lang::test_util::stream_transform_test(|process| {
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
    /// # }
    /// ```
    pub fn map<U, F>(self, f: impl IntoQuotedMut<'a, F, L> + Copy) -> KeyedStream<K, U, L, B, O, R>
    where
        F: Fn(V) -> U + 'a,
    {
        let f: ManualExpr<F, _> = ManualExpr::new(move |ctx: &L| f.splice_fn1_ctx(ctx));
        let map_f = q!({
            let orig = f;
            move |(k, v)| (k, orig(v))
        })
        .splice_fn1_ctx::<(K, V), (K, U)>(&self.location)
        .into();

        KeyedStream::new(
            self.location.clone(),
            HydroNode::Map {
                f: map_f,
                input: Box::new(self.ir_node.into_inner()),
                metadata: self
                    .location
                    .new_node_metadata(KeyedStream::<K, U, L, B, O, R>::collection_kind()),
            },
        )
    }

    /// Transforms each value by invoking `f` on each key-value pair. The resulting values are **not**
    /// re-grouped even they are tuples; instead they will be grouped under the original key.
    ///
    /// If you do not want to modify the stream and instead only want to view
    /// each item use [`KeyedStream::inspect_with_key`] instead.
    ///
    /// # Example
    /// ```rust
    /// # #[cfg(feature = "deploy")] {
    /// # use hydro_lang::prelude::*;
    /// # use futures::StreamExt;
    /// # tokio_test::block_on(hydro_lang::test_util::stream_transform_test(|process| {
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
    /// # }
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
        let map_f = q!({
            let orig = f;
            move |(k, v)| {
                let out = orig((Clone::clone(&k), v));
                (k, out)
            }
        })
        .splice_fn1_ctx::<(K, V), (K, U)>(&self.location)
        .into();

        KeyedStream::new(
            self.location.clone(),
            HydroNode::Map {
                f: map_f,
                input: Box::new(self.ir_node.into_inner()),
                metadata: self
                    .location
                    .new_node_metadata(KeyedStream::<K, U, L, B, O, R>::collection_kind()),
            },
        )
    }

    /// Prepends a new value to the key of each element in the stream, producing a new
    /// keyed stream with compound keys. Because the original key is preserved, no re-grouping
    /// occurs and the elements in each group preserve their original order.
    ///
    /// # Example
    /// ```rust
    /// # #[cfg(feature = "deploy")] {
    /// # use hydro_lang::prelude::*;
    /// # use futures::StreamExt;
    /// # tokio_test::block_on(hydro_lang::test_util::stream_transform_test(|process| {
    /// process
    ///     .source_iter(q!(vec![(1, 2), (1, 3), (2, 4)]))
    ///     .into_keyed()
    ///     .prefix_key(q!(|&(k, _)| k % 2))
    /// #   .entries()
    /// # }, |mut stream| async move {
    /// // { (1, 1): [2, 3], (0, 2): [4] }
    /// # for w in vec![((1, 1), 2), ((1, 1), 3), ((0, 2), 4)] {
    /// #     assert_eq!(stream.next().await.unwrap(), w);
    /// # }
    /// # }));
    /// # }
    /// ```
    pub fn prefix_key<K2, F>(
        self,
        f: impl IntoQuotedMut<'a, F, L> + Copy,
    ) -> KeyedStream<(K2, K), V, L, B, O, R>
    where
        F: Fn(&(K, V)) -> K2 + 'a,
    {
        let f: ManualExpr<F, _> = ManualExpr::new(move |ctx: &L| f.splice_fn1_borrow_ctx(ctx));
        let map_f = q!({
            let orig = f;
            move |kv| {
                let out = orig(&kv);
                ((out, kv.0), kv.1)
            }
        })
        .splice_fn1_ctx::<(K, V), ((K2, K), V)>(&self.location)
        .into();

        KeyedStream::new(
            self.location.clone(),
            HydroNode::Map {
                f: map_f,
                input: Box::new(self.ir_node.into_inner()),
                metadata: self
                    .location
                    .new_node_metadata(KeyedStream::<(K2, K), V, L, B, O, R>::collection_kind()),
            },
        )
    }

    /// Creates a stream containing only the elements of each group stream that satisfy a predicate
    /// `f`, preserving the order of the elements within the group.
    ///
    /// The closure `f` receives a reference `&V` rather than an owned value `v` because filtering does
    /// not modify or take ownership of the values. If you need to modify the values while filtering
    /// use [`KeyedStream::filter_map`] instead.
    ///
    /// # Example
    /// ```rust
    /// # #[cfg(feature = "deploy")] {
    /// # use hydro_lang::prelude::*;
    /// # use futures::StreamExt;
    /// # tokio_test::block_on(hydro_lang::test_util::stream_transform_test(|process| {
    /// process
    ///     .source_iter(q!(vec![(1, 2), (1, 3), (2, 4)]))
    ///     .into_keyed()
    ///     .filter(q!(|&x| x > 2))
    /// #   .entries()
    /// # }, |mut stream| async move {
    /// // { 1: [3], 2: [4] }
    /// # for w in vec![(1, 3), (2, 4)] {
    /// #     assert_eq!(stream.next().await.unwrap(), w);
    /// # }
    /// # }));
    /// # }
    /// ```
    pub fn filter<F>(self, f: impl IntoQuotedMut<'a, F, L> + Copy) -> KeyedStream<K, V, L, B, O, R>
    where
        F: Fn(&V) -> bool + 'a,
    {
        let f: ManualExpr<F, _> = ManualExpr::new(move |ctx: &L| f.splice_fn1_borrow_ctx(ctx));
        let filter_f = q!({
            let orig = f;
            move |t: &(_, _)| orig(&t.1)
        })
        .splice_fn1_borrow_ctx::<(K, V), bool>(&self.location)
        .into();

        KeyedStream::new(
            self.location.clone(),
            HydroNode::Filter {
                f: filter_f,
                input: Box::new(self.ir_node.into_inner()),
                metadata: self.location.new_node_metadata(Self::collection_kind()),
            },
        )
    }

    /// Creates a stream containing only the elements of each group stream that satisfy a predicate
    /// `f` (which receives the key-value tuple), preserving the order of the elements within the group.
    ///
    /// The closure `f` receives a reference `&(K, V)` rather than an owned value `(K, V)` because filtering does
    /// not modify or take ownership of the values. If you need to modify the values while filtering
    /// use [`KeyedStream::filter_map_with_key`] instead.
    ///
    /// # Example
    /// ```rust
    /// # #[cfg(feature = "deploy")] {
    /// # use hydro_lang::prelude::*;
    /// # use futures::StreamExt;
    /// # tokio_test::block_on(hydro_lang::test_util::stream_transform_test(|process| {
    /// process
    ///     .source_iter(q!(vec![(1, 2), (1, 3), (2, 4)]))
    ///     .into_keyed()
    ///     .filter_with_key(q!(|&(k, v)| v - k == 2))
    /// #   .entries()
    /// # }, |mut stream| async move {
    /// // { 1: [3], 2: [4] }
    /// # for w in vec![(1, 3), (2, 4)] {
    /// #     assert_eq!(stream.next().await.unwrap(), w);
    /// # }
    /// # }));
    /// # }
    /// ```
    pub fn filter_with_key<F>(
        self,
        f: impl IntoQuotedMut<'a, F, L> + Copy,
    ) -> KeyedStream<K, V, L, B, O, R>
    where
        F: Fn(&(K, V)) -> bool + 'a,
    {
        let filter_f = f
            .splice_fn1_borrow_ctx::<(K, V), bool>(&self.location)
            .into();

        KeyedStream::new(
            self.location.clone(),
            HydroNode::Filter {
                f: filter_f,
                input: Box::new(self.ir_node.into_inner()),
                metadata: self.location.new_node_metadata(Self::collection_kind()),
            },
        )
    }

    /// An operator that both filters and maps each value, with keys staying the same.
    /// It yields only the items for which the supplied closure `f` returns `Some(value)`.
    /// If you need access to the key, see [`KeyedStream::filter_map_with_key`].
    ///
    /// # Example
    /// ```rust
    /// # #[cfg(feature = "deploy")] {
    /// # use hydro_lang::prelude::*;
    /// # use futures::StreamExt;
    /// # tokio_test::block_on(hydro_lang::test_util::stream_transform_test(|process| {
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
    /// # }
    /// ```
    pub fn filter_map<U, F>(
        self,
        f: impl IntoQuotedMut<'a, F, L> + Copy,
    ) -> KeyedStream<K, U, L, B, O, R>
    where
        F: Fn(V) -> Option<U> + 'a,
    {
        let f: ManualExpr<F, _> = ManualExpr::new(move |ctx: &L| f.splice_fn1_ctx(ctx));
        let filter_map_f = q!({
            let orig = f;
            move |(k, v)| orig(v).map(|o| (k, o))
        })
        .splice_fn1_ctx::<(K, V), Option<(K, U)>>(&self.location)
        .into();

        KeyedStream::new(
            self.location.clone(),
            HydroNode::FilterMap {
                f: filter_map_f,
                input: Box::new(self.ir_node.into_inner()),
                metadata: self
                    .location
                    .new_node_metadata(KeyedStream::<K, U, L, B, O, R>::collection_kind()),
            },
        )
    }

    /// An operator that both filters and maps each key-value pair. The resulting values are **not**
    /// re-grouped even they are tuples; instead they will be grouped under the original key.
    /// It yields only the items for which the supplied closure `f` returns `Some(value)`.
    ///
    /// # Example
    /// ```rust
    /// # #[cfg(feature = "deploy")] {
    /// # use hydro_lang::prelude::*;
    /// # use futures::StreamExt;
    /// # tokio_test::block_on(hydro_lang::test_util::stream_transform_test(|process| {
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
    /// # }
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
        let filter_map_f = q!({
            let orig = f;
            move |(k, v)| {
                let out = orig((Clone::clone(&k), v));
                out.map(|o| (k, o))
            }
        })
        .splice_fn1_ctx::<(K, V), Option<(K, U)>>(&self.location)
        .into();

        KeyedStream::new(
            self.location.clone(),
            HydroNode::FilterMap {
                f: filter_map_f,
                input: Box::new(self.ir_node.into_inner()),
                metadata: self
                    .location
                    .new_node_metadata(KeyedStream::<K, U, L, B, O, R>::collection_kind()),
            },
        )
    }

    /// Generates a keyed stream that maps each value `v` to a tuple `(v, x)`,
    /// where `v` is the value of `other`, a bounded [`super::singleton::Singleton`] or
    /// [`Optional`]. If `other` is an empty [`Optional`], no values will be produced.
    ///
    /// # Example
    /// ```rust
    /// # #[cfg(feature = "deploy")] {
    /// # use hydro_lang::prelude::*;
    /// # use futures::StreamExt;
    /// # tokio_test::block_on(hydro_lang::test_util::stream_transform_test(|process| {
    /// let tick = process.tick();
    /// let batch = process
    ///   .source_iter(q!(vec![(1, 123), (1, 456), (2, 123)]))
    ///   .into_keyed()
    ///   .batch(&tick, nondet!(/** test */));
    /// let count = batch.clone().entries().count(); // `count()` returns a singleton
    /// batch.cross_singleton(count).all_ticks().entries()
    /// # }, |mut stream| async move {
    /// // { 1: [(123, 3), (456, 3)], 2: [(123, 3)] }
    /// # for w in vec![(1, (123, 3)), (1, (456, 3)), (2, (123, 3))] {
    /// #     assert_eq!(stream.next().await.unwrap(), w);
    /// # }
    /// # }));
    /// # }
    /// ```
    pub fn cross_singleton<O2>(
        self,
        other: impl Into<Optional<O2, L, Bounded>>,
    ) -> KeyedStream<K, (V, O2), L, B, O, R>
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
                    .new_node_metadata(Stream::<((K, V), O2), L, B, O, R>::collection_kind()),
            },
        )
        .map(q!(|((k, v), o2)| (k, (v, o2))))
        .into_keyed()
    }

    /// For each value `v` in each group, transform `v` using `f` and then treat the
    /// result as an [`Iterator`] to produce values one by one within the same group.
    /// The implementation for [`Iterator`] for the output type `I` must produce items
    /// in a **deterministic** order.
    ///
    /// For example, `I` could be a `Vec`, but not a `HashSet`. If the order of the items in `I` is
    /// not deterministic, use [`KeyedStream::flat_map_unordered`] instead.
    ///
    /// # Example
    /// ```rust
    /// # #[cfg(feature = "deploy")] {
    /// # use hydro_lang::prelude::*;
    /// # use futures::StreamExt;
    /// # tokio_test::block_on(hydro_lang::test_util::stream_transform_test(|process| {
    /// process
    ///     .source_iter(q!(vec![(1, vec![2, 3]), (1, vec![4]), (2, vec![5, 6])]))
    ///     .into_keyed()
    ///     .flat_map_ordered(q!(|x| x))
    /// #   .entries()
    /// # }, |mut stream| async move {
    /// // { 1: [2, 3, 4], 2: [5, 6] }
    /// # for w in vec![(1, 2), (1, 3), (1, 4), (2, 5), (2, 6)] {
    /// #     assert_eq!(stream.next().await.unwrap(), w);
    /// # }
    /// # }));
    /// # }
    /// ```
    pub fn flat_map_ordered<U, I, F>(
        self,
        f: impl IntoQuotedMut<'a, F, L> + Copy,
    ) -> KeyedStream<K, U, L, B, O, R>
    where
        I: IntoIterator<Item = U>,
        F: Fn(V) -> I + 'a,
        K: Clone,
    {
        let f: ManualExpr<F, _> = ManualExpr::new(move |ctx: &L| f.splice_fn1_ctx(ctx));
        let flat_map_f = q!({
            let orig = f;
            move |(k, v)| orig(v).into_iter().map(move |u| (Clone::clone(&k), u))
        })
        .splice_fn1_ctx::<(K, V), _>(&self.location)
        .into();

        KeyedStream::new(
            self.location.clone(),
            HydroNode::FlatMap {
                f: flat_map_f,
                input: Box::new(self.ir_node.into_inner()),
                metadata: self
                    .location
                    .new_node_metadata(KeyedStream::<K, U, L, B, O, R>::collection_kind()),
            },
        )
    }

    /// Like [`KeyedStream::flat_map_ordered`], but allows the implementation of [`Iterator`]
    /// for the output type `I` to produce items in any order.
    ///
    /// # Example
    /// ```rust
    /// # #[cfg(feature = "deploy")] {
    /// # use hydro_lang::{prelude::*, live_collections::stream::{NoOrder, ExactlyOnce}};
    /// # use futures::StreamExt;
    /// # tokio_test::block_on(hydro_lang::test_util::stream_transform_test::<_, _, NoOrder, ExactlyOnce>(|process| {
    /// process
    ///     .source_iter(q!(vec![
    ///         (1, std::collections::HashSet::<i32>::from_iter(vec![2, 3])),
    ///         (2, std::collections::HashSet::from_iter(vec![4, 5]))
    ///     ]))
    ///     .into_keyed()
    ///     .flat_map_unordered(q!(|x| x))
    /// #   .entries()
    /// # }, |mut stream| async move {
    /// // { 1: [2, 3], 2: [4, 5] } with values in each group in unknown order
    /// # let mut results = Vec::new();
    /// # for _ in 0..4 {
    /// #     results.push(stream.next().await.unwrap());
    /// # }
    /// # results.sort();
    /// # assert_eq!(results, vec![(1, 2), (1, 3), (2, 4), (2, 5)]);
    /// # }));
    /// # }
    /// ```
    pub fn flat_map_unordered<U, I, F>(
        self,
        f: impl IntoQuotedMut<'a, F, L> + Copy,
    ) -> KeyedStream<K, U, L, B, NoOrder, R>
    where
        I: IntoIterator<Item = U>,
        F: Fn(V) -> I + 'a,
        K: Clone,
    {
        let f: ManualExpr<F, _> = ManualExpr::new(move |ctx: &L| f.splice_fn1_ctx(ctx));
        let flat_map_f = q!({
            let orig = f;
            move |(k, v)| orig(v).into_iter().map(move |u| (Clone::clone(&k), u))
        })
        .splice_fn1_ctx::<(K, V), _>(&self.location)
        .into();

        KeyedStream::new(
            self.location.clone(),
            HydroNode::FlatMap {
                f: flat_map_f,
                input: Box::new(self.ir_node.into_inner()),
                metadata: self
                    .location
                    .new_node_metadata(KeyedStream::<K, U, L, B, NoOrder, R>::collection_kind()),
            },
        )
    }

    /// For each value `v` in each group, treat `v` as an [`Iterator`] and produce its items one by one
    /// within the same group. The implementation for [`Iterator`] for the value type `V` must produce
    /// items in a **deterministic** order.
    ///
    /// For example, `V` could be a `Vec`, but not a `HashSet`. If the order of the items in `V` is
    /// not deterministic, use [`KeyedStream::flatten_unordered`] instead.
    ///
    /// # Example
    /// ```rust
    /// # #[cfg(feature = "deploy")] {
    /// # use hydro_lang::prelude::*;
    /// # use futures::StreamExt;
    /// # tokio_test::block_on(hydro_lang::test_util::stream_transform_test(|process| {
    /// process
    ///     .source_iter(q!(vec![(1, vec![2, 3]), (1, vec![4]), (2, vec![5, 6])]))
    ///     .into_keyed()
    ///     .flatten_ordered()
    /// #   .entries()
    /// # }, |mut stream| async move {
    /// // { 1: [2, 3, 4], 2: [5, 6] }
    /// # for w in vec![(1, 2), (1, 3), (1, 4), (2, 5), (2, 6)] {
    /// #     assert_eq!(stream.next().await.unwrap(), w);
    /// # }
    /// # }));
    /// # }
    /// ```
    pub fn flatten_ordered<U>(self) -> KeyedStream<K, U, L, B, O, R>
    where
        V: IntoIterator<Item = U>,
        K: Clone,
    {
        self.flat_map_ordered(q!(|d| d))
    }

    /// Like [`KeyedStream::flatten_ordered`], but allows the implementation of [`Iterator`]
    /// for the value type `V` to produce items in any order.
    ///
    /// # Example
    /// ```rust
    /// # #[cfg(feature = "deploy")] {
    /// # use hydro_lang::{prelude::*, live_collections::stream::{NoOrder, ExactlyOnce}};
    /// # use futures::StreamExt;
    /// # tokio_test::block_on(hydro_lang::test_util::stream_transform_test::<_, _, NoOrder, ExactlyOnce>(|process| {
    /// process
    ///     .source_iter(q!(vec![
    ///         (1, std::collections::HashSet::<i32>::from_iter(vec![2, 3])),
    ///         (2, std::collections::HashSet::from_iter(vec![4, 5]))
    ///     ]))
    ///     .into_keyed()
    ///     .flatten_unordered()
    /// #   .entries()
    /// # }, |mut stream| async move {
    /// // { 1: [2, 3], 2: [4, 5] } with values in each group in unknown order
    /// # let mut results = Vec::new();
    /// # for _ in 0..4 {
    /// #     results.push(stream.next().await.unwrap());
    /// # }
    /// # results.sort();
    /// # assert_eq!(results, vec![(1, 2), (1, 3), (2, 4), (2, 5)]);
    /// # }));
    /// # }
    /// ```
    pub fn flatten_unordered<U>(self) -> KeyedStream<K, U, L, B, NoOrder, R>
    where
        V: IntoIterator<Item = U>,
        K: Clone,
    {
        self.flat_map_unordered(q!(|d| d))
    }

    /// An operator which allows you to "inspect" each element of a stream without
    /// modifying it. The closure `f` is called on a reference to each value. This is
    /// mainly useful for debugging, and should not be used to generate side-effects.
    ///
    /// # Example
    /// ```rust
    /// # #[cfg(feature = "deploy")] {
    /// # use hydro_lang::prelude::*;
    /// # use futures::StreamExt;
    /// # tokio_test::block_on(hydro_lang::test_util::stream_transform_test(|process| {
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
    /// # }
    /// ```
    pub fn inspect<F>(self, f: impl IntoQuotedMut<'a, F, L> + Copy) -> Self
    where
        F: Fn(&V) + 'a,
    {
        let f: ManualExpr<F, _> = ManualExpr::new(move |ctx: &L| f.splice_fn1_borrow_ctx(ctx));
        let inspect_f = q!({
            let orig = f;
            move |t: &(_, _)| orig(&t.1)
        })
        .splice_fn1_borrow_ctx::<(K, V), ()>(&self.location)
        .into();

        KeyedStream::new(
            self.location.clone(),
            HydroNode::Inspect {
                f: inspect_f,
                input: Box::new(self.ir_node.into_inner()),
                metadata: self.location.new_node_metadata(Self::collection_kind()),
            },
        )
    }

    /// An operator which allows you to "inspect" each element of a stream without
    /// modifying it. The closure `f` is called on a reference to each key-value pair. This is
    /// mainly useful for debugging, and should not be used to generate side-effects.
    ///
    /// # Example
    /// ```rust
    /// # #[cfg(feature = "deploy")] {
    /// # use hydro_lang::prelude::*;
    /// # use futures::StreamExt;
    /// # tokio_test::block_on(hydro_lang::test_util::stream_transform_test(|process| {
    /// process
    ///     .source_iter(q!(vec![(1, 2), (1, 3), (2, 4)]))
    ///     .into_keyed()
    ///     .inspect_with_key(q!(|(k, v)| println!("{}: {}", k, v)))
    /// #   .entries()
    /// # }, |mut stream| async move {
    /// # for w in vec![(1, 2), (1, 3), (2, 4)] {
    /// #     assert_eq!(stream.next().await.unwrap(), w);
    /// # }
    /// # }));
    /// # }
    /// ```
    pub fn inspect_with_key<F>(self, f: impl IntoQuotedMut<'a, F, L>) -> Self
    where
        F: Fn(&(K, V)) + 'a,
    {
        let inspect_f = f.splice_fn1_borrow_ctx::<(K, V), ()>(&self.location).into();

        KeyedStream::new(
            self.location.clone(),
            HydroNode::Inspect {
                f: inspect_f,
                input: Box::new(self.ir_node.into_inner()),
                metadata: self.location.new_node_metadata(Self::collection_kind()),
            },
        )
    }

    /// An operator which allows you to "name" a `HydroNode`.
    /// This is only used for testing, to correlate certain `HydroNode`s with IDs.
    pub fn ir_node_named(self, name: &str) -> KeyedStream<K, V, L, B, O, R> {
        {
            let mut node = self.ir_node.borrow_mut();
            let metadata = node.metadata_mut();
            metadata.tag = Some(name.to_string());
        }
        self
    }
}

impl<'a, K1, K2, V, L: Location<'a>, B: Boundedness, O: Ordering, R: Retries>
    KeyedStream<(K1, K2), V, L, B, O, R>
{
    /// Produces a new keyed stream by dropping the first element of the compound key.
    ///
    /// Because multiple keys may share the same suffix, this operation results in re-grouping
    /// of the values under the new keys. The values across groups with the same new key
    /// will be interleaved, so the resulting stream has [`NoOrder`] within each group.
    ///
    /// # Example
    /// ```rust
    /// # #[cfg(feature = "deploy")] {
    /// # use hydro_lang::prelude::*;
    /// # use futures::StreamExt;
    /// # tokio_test::block_on(hydro_lang::test_util::stream_transform_test(|process| {
    /// process
    ///     .source_iter(q!(vec![((1, 10), 2), ((1, 10), 3), ((2, 20), 4)]))
    ///     .into_keyed()
    ///     .drop_key_prefix()
    /// #   .entries()
    /// # }, |mut stream| async move {
    /// // { 10: [2, 3], 20: [4] }
    /// # for w in vec![(10, 2), (10, 3), (20, 4)] {
    /// #     assert_eq!(stream.next().await.unwrap(), w);
    /// # }
    /// # }));
    /// # }
    /// ```
    pub fn drop_key_prefix(self) -> KeyedStream<K2, V, L, B, NoOrder, R> {
        self.entries()
            .map(q!(|((_k1, k2), v)| (k2, v)))
            .into_keyed()
    }
}

impl<'a, K, V, L: Location<'a> + NoTick, O: Ordering, R: Retries>
    KeyedStream<K, V, L, Unbounded, O, R>
{
    /// Produces a new keyed stream that "merges" the inputs by interleaving the elements
    /// of any overlapping groups. The result has [`NoOrder`] on each group because the
    /// order of interleaving is not guaranteed. If the keys across both inputs do not overlap,
    /// the ordering will be deterministic and you can safely use [`Self::assume_ordering`].
    ///
    /// Currently, both input streams must be [`Unbounded`].
    ///
    /// # Example
    /// ```rust
    /// # #[cfg(feature = "deploy")] {
    /// # use hydro_lang::prelude::*;
    /// # use futures::StreamExt;
    /// # tokio_test::block_on(hydro_lang::test_util::stream_transform_test(|process| {
    /// let numbers1 = process.source_iter(q!(vec![(1, 2), (3, 4)])).into_keyed();
    /// let numbers2 = process.source_iter(q!(vec![(1, 3), (3, 5)])).into_keyed();
    /// numbers1.interleave(numbers2)
    /// #   .entries()
    /// # }, |mut stream| async move {
    /// // { 1: [2, 3], 3: [4, 5] } with each group in unknown order
    /// # for w in vec![(1, 2), (3, 4), (1, 3), (3, 5)] {
    /// #     assert_eq!(stream.next().await.unwrap(), w);
    /// # }
    /// # }));
    /// # }
    /// ```
    pub fn interleave<O2: Ordering, R2: Retries>(
        self,
        other: KeyedStream<K, V, L, Unbounded, O2, R2>,
    ) -> KeyedStream<K, V, L, Unbounded, NoOrder, <R as MinRetries<R2>>::Min>
    where
        R: MinRetries<R2>,
    {
        let tick = self.location.tick();
        // Because the outputs are unordered, we can interleave batches from both streams.
        let nondet_batch_interleaving = nondet!(/** output stream is NoOrder, can interleave */);
        self.batch(&tick, nondet_batch_interleaving)
            .weakest_ordering()
            .chain(
                other
                    .batch(&tick, nondet_batch_interleaving)
                    .weakest_ordering(),
            )
            .all_ticks()
    }
}

/// The output of a Hydro generator created with [`KeyedStream::generator`], which can yield elements and
/// control the processing of future elements.
pub enum Generate<T> {
    /// Emit the provided element, and keep processing future inputs.
    Yield(T),
    /// Emit the provided element as the _final_ element, do not process future inputs.
    Return(T),
    /// Do not emit anything, but continue processing future inputs.
    Continue,
    /// Do not emit anything, and do not process further inputs.
    Break,
}

impl<'a, K, V, L, B: Boundedness> KeyedStream<K, V, L, B, TotalOrder, ExactlyOnce>
where
    L: Location<'a>,
{
    /// A special case of [`Stream::scan`] for keyed streams. For each key group the values are transformed via the `f` combinator.
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
    /// # #[cfg(feature = "deploy")] {
    /// # use hydro_lang::prelude::*;
    /// # use futures::StreamExt;
    /// # tokio_test::block_on(hydro_lang::test_util::stream_transform_test(|process| {
    /// process
    ///     .source_iter(q!(vec![(0, 1), (0, 3), (1, 3), (1, 4)]))
    ///     .into_keyed()
    ///     .scan(
    ///         q!(|| 0),
    ///         q!(|acc, x| {
    ///             *acc += x;
    ///             if *acc % 2 == 0 { None } else { Some(*acc) }
    ///         }),
    ///     )
    /// #   .entries()
    /// # }, |mut stream| async move {
    /// // Output: { 0: [1], 1: [3, 7] }
    /// # for w in vec![(0, 1), (1, 3), (1, 7)] {
    /// #     assert_eq!(stream.next().await.unwrap(), w);
    /// # }
    /// # }));
    /// # }
    /// ```
    pub fn scan<A, U, I, F>(
        self,
        init: impl IntoQuotedMut<'a, I, L> + Copy,
        f: impl IntoQuotedMut<'a, F, L> + Copy,
    ) -> KeyedStream<K, U, L, B, TotalOrder, ExactlyOnce>
    where
        K: Clone + Eq + Hash,
        I: Fn() -> A + 'a,
        F: Fn(&mut A, V) -> Option<U> + 'a,
    {
        let f: ManualExpr<F, _> = ManualExpr::new(move |ctx: &L| f.splice_fn2_borrow_mut_ctx(ctx));
        self.generator(
            init,
            q!({
                let orig = f;
                move |state, v| {
                    if let Some(out) = orig(state, v) {
                        Generate::Yield(out)
                    } else {
                        Generate::Break
                    }
                }
            }),
        )
    }

    /// Iteratively processes the elements in each group using a state machine that can yield
    /// elements as it processes its inputs. This is designed to mirror the unstable generator
    /// syntax in Rust, without requiring special syntax.
    ///
    /// Like [`KeyedStream::scan`], this function takes in an initializer that emits the initial
    /// state for each group. The second argument defines the processing logic, taking in a
    /// mutable reference to the group's state and the value to be processed. It emits a
    /// [`Generate`] value, whose variants define what is emitted and whether further inputs
    /// should be processed.
    ///
    /// # Example
    /// ```rust
    /// # #[cfg(feature = "deploy")] {
    /// # use hydro_lang::prelude::*;
    /// # use futures::StreamExt;
    /// # tokio_test::block_on(hydro_lang::test_util::stream_transform_test(|process| {
    /// process
    ///     .source_iter(q!(vec![(0, 1), (0, 3), (0, 100), (0, 10), (1, 3), (1, 4), (1, 3)]))
    ///     .into_keyed()
    ///     .generator(
    ///         q!(|| 0),
    ///         q!(|acc, x| {
    ///             *acc += x;
    ///             if *acc > 100 {
    ///                 hydro_lang::live_collections::keyed_stream::Generate::Return(
    ///                     "done!".to_string()
    ///                 )
    ///             } else if *acc % 2 == 0 {
    ///                 hydro_lang::live_collections::keyed_stream::Generate::Yield(
    ///                     "even".to_string()
    ///                 )
    ///             } else {
    ///                 hydro_lang::live_collections::keyed_stream::Generate::Continue
    ///             }
    ///         }),
    ///     )
    /// #   .entries()
    /// # }, |mut stream| async move {
    /// // Output: { 0: ["even", "done!"], 1: ["even"] }
    /// # for w in vec![(0, "even".to_string()), (0, "done!".to_string()), (1, "even".to_string())] {
    /// #     assert_eq!(stream.next().await.unwrap(), w);
    /// # }
    /// # }));
    /// # }
    /// ```
    pub fn generator<A, U, I, F>(
        self,
        init: impl IntoQuotedMut<'a, I, L> + Copy,
        f: impl IntoQuotedMut<'a, F, L> + Copy,
    ) -> KeyedStream<K, U, L, B, TotalOrder, ExactlyOnce>
    where
        K: Clone + Eq + Hash,
        I: Fn() -> A + 'a,
        F: Fn(&mut A, V) -> Generate<U> + 'a,
    {
        let init: ManualExpr<I, _> = ManualExpr::new(move |ctx: &L| init.splice_fn0_ctx(ctx));
        let f: ManualExpr<F, _> = ManualExpr::new(move |ctx: &L| f.splice_fn2_borrow_mut_ctx(ctx));

        let scan_init = q!(|| HashMap::new())
            .splice_fn0_ctx::<HashMap<K, Option<A>>>(&self.location)
            .into();
        let scan_f = q!(move |acc: &mut HashMap<_, _>, (k, v)| {
            let existing_state = acc.entry(Clone::clone(&k)).or_insert_with(|| Some(init()));
            if let Some(existing_state_value) = existing_state {
                match f(existing_state_value, v) {
                    Generate::Yield(out) => Some(Some((k, out))),
                    Generate::Return(out) => {
                        let _ = existing_state.take(); // TODO(shadaj): garbage collect with termination markers
                        Some(Some((k, out)))
                    }
                    Generate::Break => {
                        let _ = existing_state.take(); // TODO(shadaj): garbage collect with termination markers
                        Some(None)
                    }
                    Generate::Continue => Some(None),
                }
            } else {
                Some(None)
            }
        })
        .splice_fn2_borrow_mut_ctx::<HashMap<K, Option<A>>, (K, V), _>(&self.location)
        .into();

        let scan_node = HydroNode::Scan {
            init: scan_init,
            acc: scan_f,
            input: Box::new(self.ir_node.into_inner()),
            metadata: self.location.new_node_metadata(Stream::<
                Option<(K, U)>,
                L,
                B,
                TotalOrder,
                ExactlyOnce,
            >::collection_kind()),
        };

        let flatten_f = q!(|d| d)
            .splice_fn1_ctx::<Option<(K, U)>, _>(&self.location)
            .into();
        let flatten_node = HydroNode::FlatMap {
            f: flatten_f,
            input: Box::new(scan_node),
            metadata: self.location.new_node_metadata(KeyedStream::<
                K,
                U,
                L,
                B,
                TotalOrder,
                ExactlyOnce,
            >::collection_kind()),
        };

        KeyedStream::new(self.location.clone(), flatten_node)
    }

    /// A variant of [`Stream::fold`], intended for keyed streams. The aggregation is executed
    /// in-order across the values in each group. But the aggregation function returns a boolean,
    /// which when true indicates that the aggregated result is complete and can be released to
    /// downstream computation. Unlike [`Stream::fold_keyed`], this means that even if the input
    /// stream is [`super::boundedness::Unbounded`], the outputs of the fold can be processed like
    /// normal stream elements.
    ///
    /// # Example
    /// ```rust
    /// # #[cfg(feature = "deploy")] {
    /// # use hydro_lang::prelude::*;
    /// # use futures::StreamExt;
    /// # tokio_test::block_on(hydro_lang::test_util::stream_transform_test(|process| {
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
    /// # }
    /// ```
    pub fn fold_early_stop<A, I, F>(
        self,
        init: impl IntoQuotedMut<'a, I, L> + Copy,
        f: impl IntoQuotedMut<'a, F, L> + Copy,
    ) -> KeyedSingleton<K, A, L, B::WhenValueBounded>
    where
        K: Clone + Eq + Hash,
        I: Fn() -> A + 'a,
        F: Fn(&mut A, V) -> bool + 'a,
    {
        let init: ManualExpr<I, _> = ManualExpr::new(move |ctx: &L| init.splice_fn0_ctx(ctx));
        let f: ManualExpr<F, _> = ManualExpr::new(move |ctx: &L| f.splice_fn2_borrow_mut_ctx(ctx));
        let out_without_bound_cast = self.generator(
            q!(move || Some(init())),
            q!(move |key_state, v| {
                if let Some(key_state_value) = key_state.as_mut() {
                    if f(key_state_value, v) {
                        Generate::Return(key_state.take().unwrap())
                    } else {
                        Generate::Continue
                    }
                } else {
                    unreachable!()
                }
            }),
        );

        KeyedSingleton::new(
            out_without_bound_cast.location.clone(),
            HydroNode::Cast {
                inner: Box::new(out_without_bound_cast.ir_node.into_inner()),
                metadata: out_without_bound_cast
                    .location
                    .new_node_metadata(
                        KeyedSingleton::<K, A, L, B::WhenValueBounded>::collection_kind(),
                    ),
            },
        )
    }

    /// Gets the first element inside each group of values as a [`KeyedSingleton`] that preserves
    /// the original group keys. Requires the input stream to have [`TotalOrder`] guarantees,
    /// otherwise the first element would be non-deterministic.
    ///
    /// # Example
    /// ```rust
    /// # #[cfg(feature = "deploy")] {
    /// # use hydro_lang::prelude::*;
    /// # use futures::StreamExt;
    /// # tokio_test::block_on(hydro_lang::test_util::stream_transform_test(|process| {
    /// process
    ///     .source_iter(q!(vec![(0, 2), (0, 3), (1, 3), (1, 6)]))
    ///     .into_keyed()
    ///     .first()
    /// #   .entries()
    /// # }, |mut stream| async move {
    /// // Output: { 0: 2, 1: 3 }
    /// # for w in vec![(0, 2), (1, 3)] {
    /// #     assert_eq!(stream.next().await.unwrap(), w);
    /// # }
    /// # }));
    /// # }
    /// ```
    pub fn first(self) -> KeyedSingleton<K, V, L, B::WhenValueBounded>
    where
        K: Clone + Eq + Hash,
    {
        self.fold_early_stop(
            q!(|| None),
            q!(|acc, v| {
                *acc = Some(v);
                true
            }),
        )
        .map(q!(|v| v.unwrap()))
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
    /// # #[cfg(feature = "deploy")] {
    /// # use hydro_lang::prelude::*;
    /// # use futures::StreamExt;
    /// # tokio_test::block_on(hydro_lang::test_util::stream_transform_test(|process| {
    /// let tick = process.tick();
    /// let numbers = process
    ///     .source_iter(q!(vec![(1, 2), (2, 3), (1, 3), (2, 4)]))
    ///     .into_keyed();
    /// let batch = numbers.batch(&tick, nondet!(/** test */));
    /// batch
    ///     .fold(q!(|| 0), q!(|acc, x| *acc += x))
    ///     .entries()
    ///     .all_ticks()
    /// # }, |mut stream| async move {
    /// // (1, 5), (2, 7)
    /// # assert_eq!(stream.next().await.unwrap(), (1, 5));
    /// # assert_eq!(stream.next().await.unwrap(), (2, 7));
    /// # }));
    /// # }
    /// ```
    pub fn fold<A, I: Fn() -> A + 'a, F: Fn(&mut A, V)>(
        self,
        init: impl IntoQuotedMut<'a, I, L>,
        comb: impl IntoQuotedMut<'a, F, L>,
    ) -> KeyedSingleton<K, A, L, B::WhenValueUnbounded>
    where
        K: Eq + Hash,
    {
        let init = init.splice_fn0_ctx(&self.location).into();
        let comb = comb.splice_fn2_borrow_mut_ctx(&self.location).into();

        KeyedSingleton::new(
            self.location.clone(),
            HydroNode::FoldKeyed {
                init,
                acc: comb,
                input: Box::new(self.ir_node.into_inner()),
                metadata: self.location.new_node_metadata(KeyedSingleton::<
                    K,
                    A,
                    L,
                    B::WhenValueUnbounded,
                >::collection_kind()),
            },
        )
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
    /// # #[cfg(feature = "deploy")] {
    /// # use hydro_lang::prelude::*;
    /// # use futures::StreamExt;
    /// # tokio_test::block_on(hydro_lang::test_util::stream_transform_test(|process| {
    /// let tick = process.tick();
    /// let numbers = process
    ///     .source_iter(q!(vec![(1, 2), (2, 3), (1, 3), (2, 4)]))
    ///     .into_keyed();
    /// let batch = numbers.batch(&tick, nondet!(/** test */));
    /// batch.reduce(q!(|acc, x| *acc += x)).entries().all_ticks()
    /// # }, |mut stream| async move {
    /// // (1, 5), (2, 7)
    /// # assert_eq!(stream.next().await.unwrap(), (1, 5));
    /// # assert_eq!(stream.next().await.unwrap(), (2, 7));
    /// # }));
    /// # }
    /// ```
    pub fn reduce<F: Fn(&mut V, V) + 'a>(
        self,
        comb: impl IntoQuotedMut<'a, F, L>,
    ) -> KeyedSingleton<K, V, L, B::WhenValueUnbounded>
    where
        K: Eq + Hash,
    {
        let f = comb.splice_fn2_borrow_mut_ctx(&self.location).into();

        KeyedSingleton::new(
            self.location.clone(),
            HydroNode::ReduceKeyed {
                f,
                input: Box::new(self.ir_node.into_inner()),
                metadata: self.location.new_node_metadata(KeyedSingleton::<
                    K,
                    V,
                    L,
                    B::WhenValueUnbounded,
                >::collection_kind()),
            },
        )
    }

    /// A special case of [`KeyedStream::reduce`] where tuples with keys less than the watermark are automatically deleted.
    ///
    /// Each group must have a [`TotalOrder`] guarantee, which means that the `comb` closure is allowed
    /// to depend on the order of elements in the stream.
    ///
    /// # Example
    /// ```rust
    /// # #[cfg(feature = "deploy")] {
    /// # use hydro_lang::prelude::*;
    /// # use futures::StreamExt;
    /// # tokio_test::block_on(hydro_lang::test_util::stream_transform_test(|process| {
    /// let tick = process.tick();
    /// let watermark = tick.singleton(q!(1));
    /// let numbers = process
    ///     .source_iter(q!([(0, 100), (1, 101), (2, 102), (2, 102)]))
    ///     .into_keyed();
    /// let batch = numbers.batch(&tick, nondet!(/** test */));
    /// batch
    ///     .reduce_watermark(watermark, q!(|acc, x| *acc += x))
    ///     .entries()
    ///     .all_ticks()
    /// # }, |mut stream| async move {
    /// // (2, 204)
    /// # assert_eq!(stream.next().await.unwrap(), (2, 204));
    /// # }));
    /// # }
    /// ```
    pub fn reduce_watermark<O, F>(
        self,
        other: impl Into<Optional<O, Tick<L::Root>, Bounded>>,
        comb: impl IntoQuotedMut<'a, F, L>,
    ) -> KeyedSingleton<K, V, L, B::WhenValueUnbounded>
    where
        K: Eq + Hash,
        O: Clone,
        F: Fn(&mut V, V) + 'a,
    {
        let other: Optional<O, Tick<L::Root>, Bounded> = other.into();
        check_matching_location(&self.location.root(), other.location.outer());
        let f = comb.splice_fn2_borrow_mut_ctx(&self.location).into();

        KeyedSingleton::new(
            self.location.clone(),
            HydroNode::ReduceKeyedWatermark {
                f,
                input: Box::new(self.ir_node.into_inner()),
                watermark: Box::new(other.ir_node.into_inner()),
                metadata: self.location.new_node_metadata(KeyedSingleton::<
                    K,
                    V,
                    L,
                    B::WhenValueUnbounded,
                >::collection_kind()),
            },
        )
    }
}

impl<'a, K, V, L, B: Boundedness, O: Ordering> KeyedStream<K, V, L, B, O, ExactlyOnce>
where
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
    /// # #[cfg(feature = "deploy")] {
    /// # use hydro_lang::prelude::*;
    /// # use futures::StreamExt;
    /// # tokio_test::block_on(hydro_lang::test_util::stream_transform_test(|process| {
    /// let tick = process.tick();
    /// let numbers = process
    ///     .source_iter(q!(vec![(1, 2), (2, 3), (1, 3), (2, 4)]))
    ///     .into_keyed();
    /// let batch = numbers.batch(&tick, nondet!(/** test */));
    /// batch
    ///     .fold_commutative(q!(|| 0), q!(|acc, x| *acc += x))
    ///     .entries()
    ///     .all_ticks()
    /// # }, |mut stream| async move {
    /// // (1, 5), (2, 7)
    /// # assert_eq!(stream.next().await.unwrap(), (1, 5));
    /// # assert_eq!(stream.next().await.unwrap(), (2, 7));
    /// # }));
    /// # }
    /// ```
    pub fn fold_commutative<A, I: Fn() -> A + 'a, F: Fn(&mut A, V)>(
        self,
        init: impl IntoQuotedMut<'a, I, L>,
        comb: impl IntoQuotedMut<'a, F, L>,
    ) -> KeyedSingleton<K, A, L, B::WhenValueUnbounded>
    where
        K: Eq + Hash,
    {
        self.assume_ordering::<TotalOrder>(nondet!(/** the combinator function is commutative */))
            .fold(init, comb)
    }

    /// Like [`Stream::reduce_commutative`], aggregates the values in each group via the `comb` closure.
    ///
    /// The `comb` closure must be **commutative**, as the order of input items is not guaranteed.
    ///
    /// If you need the accumulated value to have a different type than the input, use [`KeyedStream::fold_commutative`].
    ///
    /// # Example
    /// ```rust
    /// # #[cfg(feature = "deploy")] {
    /// # use hydro_lang::prelude::*;
    /// # use futures::StreamExt;
    /// # tokio_test::block_on(hydro_lang::test_util::stream_transform_test(|process| {
    /// let tick = process.tick();
    /// let numbers = process
    ///     .source_iter(q!(vec![(1, 2), (2, 3), (1, 3), (2, 4)]))
    ///     .into_keyed();
    /// let batch = numbers.batch(&tick, nondet!(/** test */));
    /// batch
    ///     .reduce_commutative(q!(|acc, x| *acc += x))
    ///     .entries()
    ///     .all_ticks()
    /// # }, |mut stream| async move {
    /// // (1, 5), (2, 7)
    /// # assert_eq!(stream.next().await.unwrap(), (1, 5));
    /// # assert_eq!(stream.next().await.unwrap(), (2, 7));
    /// # }));
    /// # }
    /// ```
    pub fn reduce_commutative<F: Fn(&mut V, V) + 'a>(
        self,
        comb: impl IntoQuotedMut<'a, F, L>,
    ) -> KeyedSingleton<K, V, L, B::WhenValueUnbounded>
    where
        K: Eq + Hash,
    {
        self.assume_ordering::<TotalOrder>(nondet!(/** the combinator function is commutative */))
            .reduce(comb)
    }

    /// A special case of [`KeyedStream::reduce_commutative`] where tuples with keys less than the watermark are automatically deleted.
    ///
    /// The `comb` closure must be **commutative**, as the order of input items is not guaranteed.
    ///
    /// # Example
    /// ```rust
    /// # #[cfg(feature = "deploy")] {
    /// # use hydro_lang::prelude::*;
    /// # use futures::StreamExt;
    /// # tokio_test::block_on(hydro_lang::test_util::stream_transform_test(|process| {
    /// let tick = process.tick();
    /// let watermark = tick.singleton(q!(1));
    /// let numbers = process
    ///     .source_iter(q!([(0, 100), (1, 101), (2, 102), (2, 102)]))
    ///     .into_keyed();
    /// let batch = numbers.batch(&tick, nondet!(/** test */));
    /// batch
    ///     .reduce_watermark_commutative(watermark, q!(|acc, x| *acc += x))
    ///     .entries()
    ///     .all_ticks()
    /// # }, |mut stream| async move {
    /// // (2, 204)
    /// # assert_eq!(stream.next().await.unwrap(), (2, 204));
    /// # }));
    /// # }
    /// ```
    pub fn reduce_watermark_commutative<O2, F>(
        self,
        other: impl Into<Optional<O2, Tick<L::Root>, Bounded>>,
        comb: impl IntoQuotedMut<'a, F, L>,
    ) -> KeyedSingleton<K, V, L, B::WhenValueUnbounded>
    where
        K: Eq + Hash,
        O2: Clone,
        F: Fn(&mut V, V) + 'a,
    {
        self.assume_ordering::<TotalOrder>(nondet!(/** the combinator function is commutative */))
            .reduce_watermark(other, comb)
    }

    /// Counts the number of elements in each group, producing a [`KeyedSingleton`] with the counts.
    ///
    /// # Example
    /// ```rust
    /// # #[cfg(feature = "deploy")] {
    /// # use hydro_lang::prelude::*;
    /// # use futures::StreamExt;
    /// # tokio_test::block_on(hydro_lang::test_util::stream_transform_test(|process| {
    /// let tick = process.tick();
    /// let numbers = process
    ///     .source_iter(q!(vec![(1, 2), (2, 3), (1, 3), (2, 4), (1, 5)]))
    ///     .into_keyed();
    /// let batch = numbers.batch(&tick, nondet!(/** test */));
    /// batch
    ///     .value_counts()
    ///     .entries()
    ///     .all_ticks()
    /// # }, |mut stream| async move {
    /// // (1, 3), (2, 2)
    /// # assert_eq!(stream.next().await.unwrap(), (1, 3));
    /// # assert_eq!(stream.next().await.unwrap(), (2, 2));
    /// # }));
    /// # }
    /// ```
    pub fn value_counts(self) -> KeyedSingleton<K, usize, L, B::WhenValueUnbounded>
    where
        K: Eq + Hash,
    {
        self.assume_ordering_trusted(
            nondet!(/** ordering within each group affects neither result nor intermediates */),
        )
        .fold(q!(|| 0), q!(|acc, _| *acc += 1))
    }
}

impl<'a, K, V, L, B: Boundedness, R: Retries> KeyedStream<K, V, L, B, TotalOrder, R>
where
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
    /// # #[cfg(feature = "deploy")] {
    /// # use hydro_lang::prelude::*;
    /// # use futures::StreamExt;
    /// # tokio_test::block_on(hydro_lang::test_util::stream_transform_test(|process| {
    /// let tick = process.tick();
    /// let numbers = process
    ///     .source_iter(q!(vec![(1, false), (2, true), (1, false), (2, false)]))
    ///     .into_keyed();
    /// let batch = numbers.batch(&tick, nondet!(/** test */));
    /// batch
    ///     .fold_idempotent(q!(|| false), q!(|acc, x| *acc |= x))
    ///     .entries()
    ///     .all_ticks()
    /// # }, |mut stream| async move {
    /// // (1, false), (2, true)
    /// # assert_eq!(stream.next().await.unwrap(), (1, false));
    /// # assert_eq!(stream.next().await.unwrap(), (2, true));
    /// # }));
    /// # }
    /// ```
    pub fn fold_idempotent<A, I: Fn() -> A + 'a, F: Fn(&mut A, V)>(
        self,
        init: impl IntoQuotedMut<'a, I, L>,
        comb: impl IntoQuotedMut<'a, F, L>,
    ) -> KeyedSingleton<K, A, L, B::WhenValueUnbounded>
    where
        K: Eq + Hash,
    {
        self.assume_retries::<ExactlyOnce>(nondet!(/** the combinator function is idempotent */))
            .fold(init, comb)
    }

    /// Like [`Stream::reduce_idempotent`], aggregates the values in each group via the `comb` closure.
    ///
    /// The `comb` closure must be **idempotent**, as there may be non-deterministic duplicates.
    ///
    /// If you need the accumulated value to have a different type than the input, use [`KeyedStream::fold_idempotent`].
    ///
    /// # Example
    /// ```rust
    /// # #[cfg(feature = "deploy")] {
    /// # use hydro_lang::prelude::*;
    /// # use futures::StreamExt;
    /// # tokio_test::block_on(hydro_lang::test_util::stream_transform_test(|process| {
    /// let tick = process.tick();
    /// let numbers = process
    ///     .source_iter(q!(vec![(1, false), (2, true), (1, false), (2, false)]))
    ///     .into_keyed();
    /// let batch = numbers.batch(&tick, nondet!(/** test */));
    /// batch
    ///     .reduce_idempotent(q!(|acc, x| *acc |= x))
    ///     .entries()
    ///     .all_ticks()
    /// # }, |mut stream| async move {
    /// // (1, false), (2, true)
    /// # assert_eq!(stream.next().await.unwrap(), (1, false));
    /// # assert_eq!(stream.next().await.unwrap(), (2, true));
    /// # }));
    /// # }
    /// ```
    pub fn reduce_idempotent<F: Fn(&mut V, V) + 'a>(
        self,
        comb: impl IntoQuotedMut<'a, F, L>,
    ) -> KeyedSingleton<K, V, L, B::WhenValueUnbounded>
    where
        K: Eq + Hash,
    {
        self.assume_retries::<ExactlyOnce>(nondet!(/** the combinator function is idempotent */))
            .reduce(comb)
    }

    /// A special case of [`KeyedStream::reduce_idempotent`] where tuples with keys less than the watermark are automatically deleted.
    ///
    /// The `comb` closure must be **idempotent**, as there may be non-deterministic duplicates.
    ///
    /// # Example
    /// ```rust
    /// # #[cfg(feature = "deploy")] {
    /// # use hydro_lang::prelude::*;
    /// # use futures::StreamExt;
    /// # tokio_test::block_on(hydro_lang::test_util::stream_transform_test(|process| {
    /// let tick = process.tick();
    /// let watermark = tick.singleton(q!(1));
    /// let numbers = process
    ///     .source_iter(q!([(0, false), (1, false), (2, false), (2, true)]))
    ///     .into_keyed();
    /// let batch = numbers.batch(&tick, nondet!(/** test */));
    /// batch
    ///     .reduce_watermark_idempotent(watermark, q!(|acc, x| *acc |= x))
    ///     .entries()
    ///     .all_ticks()
    /// # }, |mut stream| async move {
    /// // (2, true)
    /// # assert_eq!(stream.next().await.unwrap(), (2, true));
    /// # }));
    /// # }
    /// ```
    pub fn reduce_watermark_idempotent<O2, F>(
        self,
        other: impl Into<Optional<O2, Tick<L::Root>, Bounded>>,
        comb: impl IntoQuotedMut<'a, F, L>,
    ) -> KeyedSingleton<K, V, L, B::WhenValueUnbounded>
    where
        K: Eq + Hash,
        O2: Clone,
        F: Fn(&mut V, V) + 'a,
    {
        self.assume_retries::<ExactlyOnce>(nondet!(/** the combinator function is idempotent */))
            .reduce_watermark(other, comb)
    }
}

impl<'a, K, V, L, B: Boundedness, O: Ordering, R: Retries> KeyedStream<K, V, L, B, O, R>
where
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
    /// # #[cfg(feature = "deploy")] {
    /// # use hydro_lang::prelude::*;
    /// # use futures::StreamExt;
    /// # tokio_test::block_on(hydro_lang::test_util::stream_transform_test(|process| {
    /// let tick = process.tick();
    /// let numbers = process
    ///     .source_iter(q!(vec![(1, false), (2, true), (1, false), (2, false)]))
    ///     .into_keyed();
    /// let batch = numbers.batch(&tick, nondet!(/** test */));
    /// batch
    ///     .fold_commutative_idempotent(q!(|| false), q!(|acc, x| *acc |= x))
    ///     .entries()
    ///     .all_ticks()
    /// # }, |mut stream| async move {
    /// // (1, false), (2, true)
    /// # assert_eq!(stream.next().await.unwrap(), (1, false));
    /// # assert_eq!(stream.next().await.unwrap(), (2, true));
    /// # }));
    /// # }
    /// ```
    pub fn fold_commutative_idempotent<A, I: Fn() -> A + 'a, F: Fn(&mut A, V)>(
        self,
        init: impl IntoQuotedMut<'a, I, L>,
        comb: impl IntoQuotedMut<'a, F, L>,
    ) -> KeyedSingleton<K, A, L, B::WhenValueUnbounded>
    where
        K: Eq + Hash,
    {
        self.assume_ordering::<TotalOrder>(nondet!(/** the combinator function is commutative */))
            .assume_retries::<ExactlyOnce>(nondet!(/** the combinator function is idempotent */))
            .fold(init, comb)
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
    /// # #[cfg(feature = "deploy")] {
    /// # use hydro_lang::prelude::*;
    /// # use futures::StreamExt;
    /// # tokio_test::block_on(hydro_lang::test_util::stream_transform_test(|process| {
    /// let tick = process.tick();
    /// let numbers = process
    ///     .source_iter(q!(vec![(1, false), (2, true), (1, false), (2, false)]))
    ///     .into_keyed();
    /// let batch = numbers.batch(&tick, nondet!(/** test */));
    /// batch
    ///     .reduce_commutative_idempotent(q!(|acc, x| *acc |= x))
    ///     .entries()
    ///     .all_ticks()
    /// # }, |mut stream| async move {
    /// // (1, false), (2, true)
    /// # assert_eq!(stream.next().await.unwrap(), (1, false));
    /// # assert_eq!(stream.next().await.unwrap(), (2, true));
    /// # }));
    /// # }
    /// ```
    pub fn reduce_commutative_idempotent<F: Fn(&mut V, V) + 'a>(
        self,
        comb: impl IntoQuotedMut<'a, F, L>,
    ) -> KeyedSingleton<K, V, L, B::WhenValueUnbounded>
    where
        K: Eq + Hash,
    {
        self.assume_ordering::<TotalOrder>(nondet!(/** the combinator function is commutative */))
            .assume_retries::<ExactlyOnce>(nondet!(/** the combinator function is idempotent */))
            .reduce(comb)
    }

    /// A special case of [`Stream::reduce_keyed_commutative_idempotent`] where tuples with keys less than the watermark are automatically deleted.
    ///
    /// The `comb` closure must be **commutative**, as the order of input items is not guaranteed, and **idempotent**,
    /// as there may be non-deterministic duplicates.
    ///
    /// # Example
    /// ```rust
    /// # #[cfg(feature = "deploy")] {
    /// # use hydro_lang::prelude::*;
    /// # use futures::StreamExt;
    /// # tokio_test::block_on(hydro_lang::test_util::stream_transform_test(|process| {
    /// let tick = process.tick();
    /// let watermark = tick.singleton(q!(1));
    /// let numbers = process
    ///     .source_iter(q!([(0, false), (1, false), (2, false), (2, true)]))
    ///     .into_keyed();
    /// let batch = numbers.batch(&tick, nondet!(/** test */));
    /// batch
    ///     .reduce_watermark_commutative_idempotent(watermark, q!(|acc, x| *acc |= x))
    ///     .entries()
    ///     .all_ticks()
    /// # }, |mut stream| async move {
    /// // (2, true)
    /// # assert_eq!(stream.next().await.unwrap(), (2, true));
    /// # }));
    /// # }
    /// ```
    pub fn reduce_watermark_commutative_idempotent<O2, F>(
        self,
        other: impl Into<Optional<O2, Tick<L::Root>, Bounded>>,
        comb: impl IntoQuotedMut<'a, F, L>,
    ) -> KeyedSingleton<K, V, L, B::WhenValueUnbounded>
    where
        K: Eq + Hash,
        O2: Clone,
        F: Fn(&mut V, V) + 'a,
    {
        self.assume_ordering::<TotalOrder>(nondet!(/** the combinator function is commutative */))
            .assume_retries::<ExactlyOnce>(nondet!(/** the combinator function is idempotent */))
            .reduce_watermark(other, comb)
    }

    /// Given a bounded stream of keys `K`, returns a new keyed stream containing only the groups
    /// whose keys are not in the bounded stream.
    ///
    /// # Example
    /// ```rust
    /// # #[cfg(feature = "deploy")] {
    /// # use hydro_lang::prelude::*;
    /// # use futures::StreamExt;
    /// # tokio_test::block_on(hydro_lang::test_util::stream_transform_test(|process| {
    /// let tick = process.tick();
    /// let keyed_stream = process
    ///     .source_iter(q!(vec![ (1, 'a'), (2, 'b'), (3, 'c'), (4, 'd') ]))
    ///     .batch(&tick, nondet!(/** test */))
    ///     .into_keyed();
    /// let keys_to_remove = process
    ///     .source_iter(q!(vec![1, 2]))
    ///     .batch(&tick, nondet!(/** test */));
    /// keyed_stream.filter_key_not_in(keys_to_remove).all_ticks()
    /// #   .entries()
    /// # }, |mut stream| async move {
    /// // { 3: ['c'], 4: ['d'] }
    /// # for w in vec![(3, 'c'), (4, 'd')] {
    /// #     assert_eq!(stream.next().await.unwrap(), w);
    /// # }
    /// # }));
    /// # }
    /// ```
    pub fn filter_key_not_in<O2: Ordering, R2: Retries>(
        self,
        other: Stream<K, L, Bounded, O2, R2>,
    ) -> Self
    where
        K: Eq + Hash,
    {
        check_matching_location(&self.location, &other.location);

        KeyedStream::new(
            self.location.clone(),
            HydroNode::AntiJoin {
                pos: Box::new(self.ir_node.into_inner()),
                neg: Box::new(other.ir_node.into_inner()),
                metadata: self.location.new_node_metadata(Self::collection_kind()),
            },
        )
    }
}

impl<'a, K, V, L, B: Boundedness, O: Ordering, R: Retries> KeyedStream<K, V, L, B, O, R>
where
    L: Location<'a>,
{
    /// Shifts this keyed stream into an atomic context, which guarantees that any downstream logic
    /// will all be executed synchronously before any outputs are yielded (in [`KeyedStream::end_atomic`]).
    ///
    /// This is useful to enforce local consistency constraints, such as ensuring that a write is
    /// processed before an acknowledgement is emitted. Entering an atomic section requires a [`Tick`]
    /// argument that declares where the stream will be atomically processed. Batching a stream into
    /// the _same_ [`Tick`] will preserve the synchronous execution, while batching into a different
    /// [`Tick`] will introduce asynchrony.
    pub fn atomic(self, tick: &Tick<L>) -> KeyedStream<K, V, Atomic<L>, B, O, R> {
        let out_location = Atomic { tick: tick.clone() };
        KeyedStream::new(
            out_location.clone(),
            HydroNode::BeginAtomic {
                inner: Box::new(self.ir_node.into_inner()),
                metadata: out_location
                    .new_node_metadata(KeyedStream::<K, V, Atomic<L>, B, O, R>::collection_kind()),
            },
        )
    }

    /// Given a tick, returns a keyed stream corresponding to a batch of elements segmented by
    /// that tick. These batches are guaranteed to be contiguous across ticks and preserve
    /// the order of the input.
    ///
    /// # Non-Determinism
    /// The batch boundaries are non-deterministic and may change across executions.
    pub fn batch(
        self,
        tick: &Tick<L>,
        nondet: NonDet,
    ) -> KeyedStream<K, V, Tick<L>, Bounded, O, R> {
        let _ = nondet;
        assert_eq!(Location::id(tick.outer()), Location::id(&self.location));
        KeyedStream::new(
            tick.clone(),
            HydroNode::Batch {
                inner: Box::new(self.ir_node.into_inner()),
                metadata: tick.new_node_metadata(
                    KeyedStream::<K, V, Tick<L>, Bounded, O, R>::collection_kind(),
                ),
            },
        )
    }
}

impl<'a, K, V, L, B: Boundedness, O: Ordering, R: Retries> KeyedStream<K, V, Atomic<L>, B, O, R>
where
    L: Location<'a> + NoTick,
{
    /// Returns a keyed stream corresponding to the latest batch of elements being atomically
    /// processed. These batches are guaranteed to be contiguous across ticks and preserve
    /// the order of the input. The output keyed stream will execute in the [`Tick`] that was
    /// used to create the atomic section.
    ///
    /// # Non-Determinism
    /// The batch boundaries are non-deterministic and may change across executions.
    pub fn batch_atomic(self, nondet: NonDet) -> KeyedStream<K, V, Tick<L>, Bounded, O, R> {
        let _ = nondet;
        KeyedStream::new(
            self.location.clone().tick,
            HydroNode::Batch {
                inner: Box::new(self.ir_node.into_inner()),
                metadata: self.location.tick.new_node_metadata(KeyedStream::<
                    K,
                    V,
                    Tick<L>,
                    Bounded,
                    O,
                    R,
                >::collection_kind(
                )),
            },
        )
    }

    /// Yields the elements of this keyed stream back into a top-level, asynchronous execution context.
    /// See [`KeyedStream::atomic`] for more details.
    pub fn end_atomic(self) -> KeyedStream<K, V, L, B, O, R> {
        KeyedStream::new(
            self.location.tick.l.clone(),
            HydroNode::EndAtomic {
                inner: Box::new(self.ir_node.into_inner()),
                metadata: self
                    .location
                    .tick
                    .l
                    .new_node_metadata(KeyedStream::<K, V, L, B, O, R>::collection_kind()),
            },
        )
    }
}

impl<'a, K, V, L, O: Ordering, R: Retries> KeyedStream<K, V, L, Bounded, O, R>
where
    L: Location<'a>,
{
    /// Produces a new keyed stream that combines the groups of the inputs by first emitting the
    /// elements of the `self` stream, and then emits the elements of the `other` stream (if a key
    /// is only present in one of the inputs, its values are passed through as-is). The output has
    /// a [`TotalOrder`] guarantee if and only if both inputs have a [`TotalOrder`] guarantee.
    ///
    /// Currently, both input streams must be [`Bounded`]. This operator will block
    /// on the first stream until all its elements are available. In a future version,
    /// we will relax the requirement on the `other` stream.
    ///
    /// # Example
    /// ```rust
    /// # #[cfg(feature = "deploy")] {
    /// # use hydro_lang::prelude::*;
    /// # use futures::StreamExt;
    /// # tokio_test::block_on(hydro_lang::test_util::stream_transform_test(|process| {
    /// let tick = process.tick();
    /// let numbers = process.source_iter(q!(vec![(0, 1), (1, 3)])).into_keyed();
    /// let batch = numbers.batch(&tick, nondet!(/** test */));
    /// batch.clone().map(q!(|x| x + 1)).chain(batch).all_ticks()
    /// # .entries()
    /// # }, |mut stream| async move {
    /// // { 0: [2, 1], 1: [4, 3] }
    /// # for w in vec![(0, 2), (1, 4), (0, 1), (1, 3)] {
    /// #     assert_eq!(stream.next().await.unwrap(), w);
    /// # }
    /// # }));
    /// # }
    /// ```
    pub fn chain<O2: Ordering, R2: Retries>(
        self,
        other: KeyedStream<K, V, L, Bounded, O2, R2>,
    ) -> KeyedStream<K, V, L, Bounded, <O as MinOrder<O2>>::Min, <R as MinRetries<R2>>::Min>
    where
        O: MinOrder<O2>,
        R: MinRetries<R2>,
    {
        check_matching_location(&self.location, &other.location);

        KeyedStream::new(
            self.location.clone(),
            HydroNode::Chain {
                first: Box::new(self.ir_node.into_inner()),
                second: Box::new(other.ir_node.into_inner()),
                metadata: self.location.new_node_metadata(KeyedStream::<
                    K,
                    V,
                    L,
                    Bounded,
                    <O as MinOrder<O2>>::Min,
                    <R as MinRetries<R2>>::Min,
                >::collection_kind()),
            },
        )
    }
}

impl<'a, K, V, L, O: Ordering, R: Retries> KeyedStream<K, V, Tick<L>, Bounded, O, R>
where
    L: Location<'a>,
{
    /// Asynchronously yields this batch of keyed elements outside the tick as an unbounded keyed stream,
    /// which will stream all the elements across _all_ tick iterations by concatenating the batches for
    /// each key.
    pub fn all_ticks(self) -> KeyedStream<K, V, L, Unbounded, O, R> {
        KeyedStream::new(
            self.location.outer().clone(),
            HydroNode::YieldConcat {
                inner: Box::new(self.ir_node.into_inner()),
                metadata: self.location.outer().new_node_metadata(KeyedStream::<
                    K,
                    V,
                    L,
                    Unbounded,
                    O,
                    R,
                >::collection_kind(
                )),
            },
        )
    }

    /// Synchronously yields this batch of keyed elements outside the tick as an unbounded keyed stream,
    /// which will stream all the elements across _all_ tick iterations by concatenating the batches for
    /// each key.
    ///
    /// Unlike [`KeyedStream::all_ticks`], this preserves synchronous execution, as the output stream
    /// is emitted in an [`Atomic`] context that will process elements synchronously with the input
    /// stream's [`Tick`] context.
    pub fn all_ticks_atomic(self) -> KeyedStream<K, V, Atomic<L>, Unbounded, O, R> {
        let out_location = Atomic {
            tick: self.location.clone(),
        };

        KeyedStream::new(
            out_location.clone(),
            HydroNode::YieldConcat {
                inner: Box::new(self.ir_node.into_inner()),
                metadata: out_location.new_node_metadata(KeyedStream::<
                    K,
                    V,
                    Atomic<L>,
                    Unbounded,
                    O,
                    R,
                >::collection_kind()),
            },
        )
    }

    /// Shifts the entries in `self` to the **next tick**, so that the returned keyed stream at
    /// tick `T` always has the entries of `self` at tick `T - 1`.
    ///
    /// At tick `0`, the output keyed stream is empty, since there is no previous tick.
    ///
    /// This operator enables stateful iterative processing with ticks, by sending data from one
    /// tick to the next. For example, you can use it to combine inputs across consecutive batches.
    ///
    /// # Example
    /// ```rust
    /// # #[cfg(feature = "deploy")] {
    /// # use hydro_lang::prelude::*;
    /// # use futures::StreamExt;
    /// # tokio_test::block_on(hydro_lang::test_util::stream_transform_test(|process| {
    /// let tick = process.tick();
    /// # // ticks are lazy by default, forces the second tick to run
    /// # tick.spin_batch(q!(1)).all_ticks().for_each(q!(|_| {}));
    /// # let batch_first_tick = process
    /// #   .source_iter(q!(vec![(1, 2), (1, 3)]))
    /// #   .batch(&tick, nondet!(/** test */))
    /// #   .into_keyed();
    /// # let batch_second_tick = process
    /// #   .source_iter(q!(vec![(1, 4), (2, 5)]))
    /// #   .batch(&tick, nondet!(/** test */))
    /// #   .defer_tick()
    /// #   .into_keyed(); // appears on the second tick
    /// let changes_across_ticks = // { 1: [2, 3] } (first tick), { 1: [4], 2: [5] } (second tick)
    /// # batch_first_tick.chain(batch_second_tick);
    /// changes_across_ticks.clone().defer_tick().chain( // from the previous tick
    ///     changes_across_ticks // from the current tick
    /// )
    /// # .entries().all_ticks()
    /// # }, |mut stream| async move {
    /// // { 1: [2, 3] } (first tick), { 1: [2, 3, 4], 2: [5] } (second tick), { 1: [4], 2: [5] } (third tick)
    /// # for w in vec![(1, 2), (1, 3), (1, 2), (1, 3), (1, 4), (2, 5), (1, 4), (2, 5)] {
    /// #     assert_eq!(stream.next().await.unwrap(), w);
    /// # }
    /// # }));
    /// # }
    /// ```
    pub fn defer_tick(self) -> KeyedStream<K, V, Tick<L>, Bounded, O, R> {
        KeyedStream::new(
            self.location.clone(),
            HydroNode::DeferTick {
                input: Box::new(self.ir_node.into_inner()),
                metadata: self.location.new_node_metadata(KeyedStream::<
                    K,
                    V,
                    Tick<L>,
                    Bounded,
                    O,
                    R,
                >::collection_kind()),
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
    #[cfg(any(feature = "deploy", feature = "sim"))]
    use stageleft::q;

    #[cfg(any(feature = "deploy", feature = "sim"))]
    use crate::compile::builder::FlowBuilder;
    #[cfg(feature = "deploy")]
    use crate::live_collections::stream::ExactlyOnce;
    #[cfg(feature = "sim")]
    use crate::live_collections::stream::{NoOrder, TotalOrder};
    #[cfg(any(feature = "deploy", feature = "sim"))]
    use crate::location::Location;
    #[cfg(any(feature = "deploy", feature = "sim"))]
    use crate::nondet::nondet;

    #[cfg(feature = "deploy")]
    #[tokio::test]
    async fn reduce_watermark_filter() {
        let mut deployment = Deployment::new();

        let flow = FlowBuilder::new();
        let node = flow.process::<()>();
        let external = flow.external::<()>();

        let node_tick = node.tick();
        let watermark = node_tick.singleton(q!(1));

        let sum = node
            .source_iter(q!([(0, 100), (1, 101), (2, 102), (2, 102)]))
            .into_keyed()
            .reduce_watermark(
                watermark,
                q!(|acc, v| {
                    *acc += v;
                }),
            )
            .snapshot(&node_tick, nondet!(/** test */))
            .entries()
            .all_ticks()
            .send_bincode_external(&external);

        let nodes = flow
            .with_process(&node, deployment.Localhost())
            .with_external(&external, deployment.Localhost())
            .deploy(&mut deployment);

        deployment.deploy().await.unwrap();

        let mut out = nodes.connect(sum).await;

        deployment.start().await.unwrap();

        assert_eq!(out.next().await.unwrap(), (2, 204));
    }

    #[cfg(feature = "deploy")]
    #[tokio::test]
    async fn reduce_watermark_garbage_collect() {
        let mut deployment = Deployment::new();

        let flow = FlowBuilder::new();
        let node = flow.process::<()>();
        let external = flow.external::<()>();
        let (tick_send, tick_trigger) =
            node.source_external_bincode::<_, _, _, ExactlyOnce>(&external);

        let node_tick = node.tick();
        let (watermark_complete_cycle, watermark) =
            node_tick.cycle_with_initial(node_tick.singleton(q!(1)));
        let next_watermark = watermark.clone().map(q!(|v| v + 1));
        watermark_complete_cycle.complete_next_tick(next_watermark);

        let tick_triggered_input = node
            .source_iter(q!([(3, 103)]))
            .batch(&node_tick, nondet!(/** test */))
            .filter_if_some(
                tick_trigger
                    .clone()
                    .batch(&node_tick, nondet!(/** test */))
                    .first(),
            )
            .all_ticks();

        let sum = node
            .source_iter(q!([(0, 100), (1, 101), (2, 102), (2, 102)]))
            .interleave(tick_triggered_input)
            .into_keyed()
            .reduce_watermark_commutative(
                watermark,
                q!(|acc, v| {
                    *acc += v;
                }),
            )
            .snapshot(&node_tick, nondet!(/** test */))
            .entries()
            .all_ticks()
            .send_bincode_external(&external);

        let nodes = flow
            .with_default_optimize()
            .with_process(&node, deployment.Localhost())
            .with_external(&external, deployment.Localhost())
            .deploy(&mut deployment);

        deployment.deploy().await.unwrap();

        let mut tick_send = nodes.connect(tick_send).await;
        let mut out_recv = nodes.connect(sum).await;

        deployment.start().await.unwrap();

        assert_eq!(out_recv.next().await.unwrap(), (2, 204));

        tick_send.send(()).await.unwrap();

        assert_eq!(out_recv.next().await.unwrap(), (3, 103));
    }

    #[cfg(feature = "sim")]
    #[test]
    #[should_panic]
    fn sim_batch_nondet_size() {
        let flow = FlowBuilder::new();
        let node = flow.process::<()>();

        let input = node.source_iter(q!([(1, 1), (1, 2), (2, 3)])).into_keyed();

        let tick = node.tick();
        let out_recv = input
            .batch(&tick, nondet!(/** test */))
            .fold(q!(|| vec![]), q!(|acc, v| acc.push(v)))
            .entries()
            .all_ticks()
            .sim_output();

        flow.sim().exhaustive(async || {
            out_recv
                .assert_yields_only_unordered([(1, vec![1, 2])])
                .await;
        });
    }

    #[cfg(feature = "sim")]
    #[test]
    fn sim_batch_preserves_group_order() {
        let flow = FlowBuilder::new();
        let node = flow.process::<()>();

        let input = node.source_iter(q!([(1, 1), (1, 2), (2, 3)])).into_keyed();

        let tick = node.tick();
        let out_recv = input
            .batch(&tick, nondet!(/** test */))
            .all_ticks()
            .fold_early_stop(
                q!(|| 0),
                q!(|acc, v| {
                    *acc = std::cmp::max(v, *acc);
                    *acc >= 2
                }),
            )
            .entries()
            .sim_output();

        let instances = flow.sim().exhaustive(async || {
            out_recv
                .assert_yields_only_unordered([(1, 2), (2, 3)])
                .await;
        });

        assert_eq!(instances, 8);
        // - three cases: all three in a separate tick (pick where (2, 3) is)
        // - two cases: (1, 1) and (1, 2) together, (2, 3) before or after
        // - two cases: (1, 1) and (1, 2) separate, (2, 3) grouped with one of them
        // - one case: all three together
    }

    #[cfg(feature = "sim")]
    #[test]
    fn sim_batch_unordered_shuffles() {
        let flow = FlowBuilder::new();
        let node = flow.process::<()>();

        let input = node
            .source_iter(q!([(1, 1), (1, 2), (2, 3)]))
            .into_keyed()
            .weakest_ordering();

        let tick = node.tick();
        let out_recv = input
            .batch(&tick, nondet!(/** test */))
            .all_ticks()
            .entries()
            .sim_output();

        let instances = flow.sim().exhaustive(async || {
            out_recv
                .assert_yields_only_unordered([(1, 1), (1, 2), (2, 3)])
                .await;
        });

        assert_eq!(instances, 13);
        // - 6 (3 * 2) cases: all three in a separate tick (pick where (2, 3) is), and order of (1, 1), (1, 2)
        // - two cases: (1, 1) and (1, 2) together, (2, 3) before or after (order of (1, 1), (1, 2) doesn't matter because batched is still unordered)
        // - 4 (2 * 2) cases: (1, 1) and (1, 2) separate, (2, 3) grouped with one of them, and order of (1, 1), (1, 2)
        // - one case: all three together (order of (1, 1), (1, 2) doesn't matter because batched is still unordered)
    }

    #[cfg(feature = "sim")]
    #[test]
    #[should_panic]
    fn sim_observe_order_batched() {
        let flow = FlowBuilder::new();
        let node = flow.process::<()>();

        let (port, input) = node.sim_input::<_, NoOrder, _>();

        let tick = node.tick();
        let batch = input.into_keyed().batch(&tick, nondet!(/** test */));
        let out_recv = batch
            .assume_ordering::<TotalOrder>(nondet!(/** test */))
            .all_ticks()
            .first()
            .entries()
            .sim_output();

        flow.sim().exhaustive(async || {
            port.send_many_unordered([(1, 1), (1, 2), (2, 1), (2, 2)]);
            out_recv
                .assert_yields_only_unordered([(1, 1), (2, 1)])
                .await; // fails with assume_ordering
        });
    }

    #[cfg(feature = "sim")]
    #[test]
    fn sim_observe_order_batched_count() {
        let flow = FlowBuilder::new();
        let node = flow.process::<()>();

        let (port, input) = node.sim_input::<_, NoOrder, _>();

        let tick = node.tick();
        let batch = input.into_keyed().batch(&tick, nondet!(/** test */));
        let out_recv = batch
            .assume_ordering::<TotalOrder>(nondet!(/** test */))
            .all_ticks()
            .entries()
            .sim_output();

        let instance_count = flow.sim().exhaustive(async || {
            port.send_many_unordered([(1, 1), (1, 2), (2, 1), (2, 2)]);
            let _ = out_recv.collect_sorted::<Vec<_>>().await;
        });

        assert_eq!(instance_count, 104); // too complicated to enumerate here, but less than stream equivalent
    }
}
