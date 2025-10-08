//! Definitions for the [`KeyedSingleton`] live collection.

use std::cell::RefCell;
use std::collections::HashMap;
use std::hash::Hash;
use std::marker::PhantomData;
use std::ops::Deref;
use std::rc::Rc;

use stageleft::{IntoQuotedMut, QuotedWithContext, q};

use super::boundedness::{Bounded, Boundedness, Unbounded};
use super::keyed_stream::KeyedStream;
use super::optional::Optional;
use super::singleton::Singleton;
use super::stream::{ExactlyOnce, NoOrder, Stream, TotalOrder};
use crate::compile::ir::{
    CollectionKind, HydroIrOpMetadata, HydroNode, HydroRoot, KeyedSingletonBoundKind, TeeNode,
};
use crate::forward_handle::ForwardRef;
#[cfg(stageleft_runtime)]
use crate::forward_handle::{CycleCollection, ReceiverComplete};
use crate::live_collections::stream::{Ordering, Retries};
#[cfg(stageleft_runtime)]
use crate::location::dynamic::{DynLocation, LocationId};
use crate::location::{Atomic, Location, NoTick, Tick, check_matching_location};
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

    /// Returns the [`KeyedSingletonBoundKind`] corresponding to this type.
    fn bound_kind() -> KeyedSingletonBoundKind;
}

impl KeyedSingletonBound for Unbounded {
    type UnderlyingBound = Unbounded;
    type ValueBound = Unbounded;
    type WithBoundedValue = BoundedValue;
    type WithUnboundedValue = Unbounded;

    fn bound_kind() -> KeyedSingletonBoundKind {
        KeyedSingletonBoundKind::Unbounded
    }
}

impl KeyedSingletonBound for Bounded {
    type UnderlyingBound = Bounded;
    type ValueBound = Bounded;
    type WithBoundedValue = Bounded;
    type WithUnboundedValue = UnreachableBound;

    fn bound_kind() -> KeyedSingletonBoundKind {
        KeyedSingletonBoundKind::Bounded
    }
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

    fn bound_kind() -> KeyedSingletonBoundKind {
        KeyedSingletonBoundKind::BoundedValue
    }
}

#[doc(hidden)]
pub struct UnreachableBound;

impl KeyedSingletonBound for UnreachableBound {
    type UnderlyingBound = Bounded;
    type ValueBound = Unbounded;

    type WithBoundedValue = Bounded;
    type WithUnboundedValue = UnreachableBound;

    fn bound_kind() -> KeyedSingletonBoundKind {
        unreachable!("UnreachableBound cannot be instantiated")
    }
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
    pub(crate) location: Loc,
    pub(crate) ir_node: RefCell<HydroNode>,

    _phantom: PhantomData<(K, V, Loc, Bound)>,
}

impl<'a, K: Clone, V: Clone, Loc: Location<'a>, Bound: KeyedSingletonBound> Clone
    for KeyedSingleton<K, V, Loc, Bound>
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
            KeyedSingleton {
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

impl<'a, K, V, L, B: KeyedSingletonBound> CycleCollection<'a, ForwardRef>
    for KeyedSingleton<K, V, L, B>
where
    L: Location<'a> + NoTick,
{
    type Location = L;

    fn create_source(ident: syn::Ident, location: L) -> Self {
        KeyedSingleton {
            location: location.clone(),
            ir_node: RefCell::new(HydroNode::CycleSource {
                ident,
                metadata: location.new_node_metadata(Self::collection_kind()),
            }),
            _phantom: PhantomData,
        }
    }
}

impl<'a, K, V, L, B: KeyedSingletonBound> ReceiverComplete<'a, ForwardRef>
    for KeyedSingleton<K, V, L, B>
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

impl<'a, K, V, L: Location<'a>, B: KeyedSingletonBound> KeyedSingleton<K, V, L, B> {
    pub(crate) fn new(location: L, ir_node: HydroNode) -> Self {
        debug_assert_eq!(ir_node.metadata().location_kind, Location::id(&location));
        debug_assert_eq!(ir_node.metadata().collection_kind, Self::collection_kind());

        KeyedSingleton {
            location,
            ir_node: RefCell::new(ir_node),
            _phantom: PhantomData,
        }
    }

    /// Returns the [`Location`] where this keyed singleton is being materialized.
    pub fn location(&self) -> &L {
        &self.location
    }
}

#[cfg(stageleft_runtime)]
fn key_count_inside_tick<'a, K, V, L: Location<'a>>(
    me: KeyedSingleton<K, V, L, Bounded>,
) -> Singleton<usize, L, Bounded> {
    me.entries().count()
}

#[cfg(stageleft_runtime)]
fn into_singleton_inside_tick<'a, K, V, L: Location<'a>>(
    me: KeyedSingleton<K, V, L, Bounded>,
) -> Singleton<HashMap<K, V>, L, Bounded>
where
    K: Eq + Hash,
{
    me.entries()
        .assume_ordering(nondet!(
            /// Because this is a keyed singleton, there is only one value per key.
        ))
        .fold(
            q!(|| HashMap::new()),
            q!(|map, (k, v)| {
                map.insert(k, v);
            }),
        )
}

impl<'a, K, V, L: Location<'a>, B: KeyedSingletonBound> KeyedSingleton<K, V, L, B> {
    pub(crate) fn collection_kind() -> CollectionKind {
        CollectionKind::KeyedSingleton {
            bound: B::bound_kind(),
            key_type: stageleft::quote_type::<K>().into(),
            value_type: stageleft::quote_type::<V>().into(),
        }
    }

    /// Transforms each value by invoking `f` on each element, with keys staying the same
    /// after transformation. If you need access to the key, see [`KeyedSingleton::map_with_key`].
    ///
    /// If you do not want to modify the stream and instead only want to view
    /// each item use [`KeyedSingleton::inspect`] instead.
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
        let map_f = q!({
            let orig = f;
            move |(k, v)| (k, orig(v))
        })
        .splice_fn1_ctx::<(K, V), (K, U)>(&self.location)
        .into();

        KeyedSingleton::new(
            self.location.clone(),
            HydroNode::Map {
                f: map_f,
                input: Box::new(self.ir_node.into_inner()),
                metadata: self
                    .location
                    .new_node_metadata(KeyedSingleton::<K, U, L, B>::collection_kind()),
            },
        )
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
        let map_f = q!({
            let orig = f;
            move |(k, v)| {
                let out = orig((Clone::clone(&k), v));
                (k, out)
            }
        })
        .splice_fn1_ctx::<(K, V), (K, U)>(&self.location)
        .into();

        KeyedSingleton::new(
            self.location.clone(),
            HydroNode::Map {
                f: map_f,
                input: Box::new(self.ir_node.into_inner()),
                metadata: self
                    .location
                    .new_node_metadata(KeyedSingleton::<K, U, L, B>::collection_kind()),
            },
        )
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
        let filter_f = q!({
            let orig = f;
            move |t: &(_, _)| orig(&t.1)
        })
        .splice_fn1_borrow_ctx::<(K, V), bool>(&self.location)
        .into();

        KeyedSingleton::new(
            self.location.clone(),
            HydroNode::Filter {
                f: filter_f,
                input: Box::new(self.ir_node.into_inner()),
                metadata: self
                    .location
                    .new_node_metadata(KeyedSingleton::<K, V, L, B>::collection_kind()),
            },
        )
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
        let filter_map_f = q!({
            let orig = f;
            move |(k, v)| orig(v).map(|o| (k, o))
        })
        .splice_fn1_ctx::<(K, V), Option<(K, U)>>(&self.location)
        .into();

        KeyedSingleton::new(
            self.location.clone(),
            HydroNode::FilterMap {
                f: filter_map_f,
                input: Box::new(self.ir_node.into_inner()),
                metadata: self
                    .location
                    .new_node_metadata(KeyedSingleton::<K, U, L, B>::collection_kind()),
            },
        )
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
        if B::ValueBound::BOUNDED {
            let me: KeyedSingleton<K, V, L, B::WithBoundedValue> = KeyedSingleton {
                location: self.location,
                ir_node: self.ir_node,
                _phantom: PhantomData,
            };

            me.entries().count()
        } else if L::is_top_level()
            && let Some(tick) = self.location.try_tick()
        {
            let me: KeyedSingleton<K, V, L, B::WithUnboundedValue> = KeyedSingleton {
                location: self.location,
                ir_node: self.ir_node,
                _phantom: PhantomData,
            };

            let out =
                key_count_inside_tick(me.snapshot(&tick, nondet!(/** eventually stabilizes */)))
                    .latest();
            Singleton::new(out.location, out.ir_node.into_inner())
        } else {
            panic!("Unbounded KeyedSingleton inside a tick");
        }
    }

    /// Converts this keyed singleton into a [`Singleton`] containing a `HashMap` from keys to values.
    ///
    /// As the values for each key are updated asynchronously, the `HashMap` will be updated
    /// asynchronously as well.
    ///
    /// # Example
    /// ```rust
    /// # use hydro_lang::prelude::*;
    /// # use futures::StreamExt;
    /// # tokio_test::block_on(hydro_lang::test_util::stream_transform_test(|process| {
    /// let keyed_singleton = // { 1: "a", 2: "b", 3: "c" }
    /// # process
    /// #     .source_iter(q!(vec![(1, "a".to_string()), (2, "b".to_string()), (3, "c".to_string())]))
    /// #     .into_keyed()
    /// #     .batch(&process.tick(), nondet!(/** test */))
    /// #     .first();
    /// keyed_singleton.into_singleton()
    /// # .all_ticks()
    /// # }, |mut stream| async move {
    /// // { 1: "a", 2: "b", 3: "c" }
    /// # assert_eq!(stream.next().await.unwrap(), vec![(1, "a".to_string()), (2, "b".to_string()), (3, "c".to_string())].into_iter().collect());
    /// # }));
    /// ```
    pub fn into_singleton(self) -> Singleton<HashMap<K, V>, L, B::UnderlyingBound>
    where
        K: Eq + Hash,
    {
        if B::ValueBound::BOUNDED {
            let me: KeyedSingleton<K, V, L, B::WithBoundedValue> = KeyedSingleton {
                location: self.location,
                ir_node: self.ir_node,
                _phantom: PhantomData,
            };

            me.entries()
                .assume_ordering(nondet!(
                    /// Because this is a keyed singleton, there is only one value per key.
                ))
                .fold(
                    q!(|| HashMap::new()),
                    q!(|map, (k, v)| {
                        // TODO(shadaj): make this commutative but really-debug-assert that there is no key overlap
                        map.insert(k, v);
                    }),
                )
        } else if L::is_top_level()
            && let Some(tick) = self.location.try_tick()
        {
            let me: KeyedSingleton<K, V, L, B::WithUnboundedValue> = KeyedSingleton {
                location: self.location,
                ir_node: self.ir_node,
                _phantom: PhantomData,
            };

            let out = into_singleton_inside_tick(
                me.snapshot(&tick, nondet!(/** eventually stabilizes */)),
            )
            .latest();
            Singleton::new(out.location, out.ir_node.into_inner())
        } else {
            panic!("Unbounded KeyedSingleton inside a tick");
        }
    }

    /// An operator which allows you to "name" a `HydroNode`.
    /// This is only used for testing, to correlate certain `HydroNode`s with IDs.
    pub fn ir_node_named(self, name: &str) -> KeyedSingleton<K, V, L, B> {
        {
            let mut node = self.ir_node.borrow_mut();
            let metadata = node.metadata_mut();
            metadata.tag = Some(name.to_string());
        }
        self
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
        self.into_keyed_stream().entries()
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
        let map_f = q!(|(_, v)| v)
            .splice_fn1_ctx::<(K, V), V>(&self.location)
            .into();

        Stream::new(
            self.location.clone(),
            HydroNode::Map {
                f: map_f,
                input: Box::new(self.ir_node.into_inner()),
                metadata: self.location.new_node_metadata(Stream::<
                    V,
                    L,
                    B::UnderlyingBound,
                    NoOrder,
                    ExactlyOnce,
                >::collection_kind()),
            },
        )
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
        check_matching_location(&self.location, &other.location);

        KeyedSingleton::new(
            self.location.clone(),
            HydroNode::AntiJoin {
                pos: Box::new(self.ir_node.into_inner()),
                neg: Box::new(other.ir_node.into_inner()),
                metadata: self.location.new_node_metadata(Self::collection_kind()),
            },
        )
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

        KeyedSingleton::new(
            self.location.clone(),
            HydroNode::Inspect {
                f: inspect_f,
                input: Box::new(self.ir_node.into_inner()),
                metadata: self.location.new_node_metadata(Self::collection_kind()),
            },
        )
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
    pub fn inspect_with_key<F>(self, f: impl IntoQuotedMut<'a, F, L>) -> Self
    where
        F: Fn(&(K, V)) + 'a,
    {
        let inspect_f = f.splice_fn1_borrow_ctx::<(K, V), ()>(&self.location).into();

        KeyedSingleton::new(
            self.location.clone(),
            HydroNode::Inspect {
                f: inspect_f,
                input: Box::new(self.ir_node.into_inner()),
                metadata: self.location.new_node_metadata(Self::collection_kind()),
            },
        )
    }

    /// Gets the key-value tuple with the largest key among all entries in this [`KeyedSingleton`].
    ///
    /// Because this method requires values to be bounded, the output [`Optional`] will only be
    /// asynchronously updated if a new key is added that is higher than the previous max key.
    ///
    /// # Example
    /// ```rust
    /// # use hydro_lang::prelude::*;
    /// # use futures::StreamExt;
    /// # tokio_test::block_on(hydro_lang::test_util::stream_transform_test(|process| {
    /// let tick = process.tick();
    /// let keyed_singleton = // { 1: 123, 2: 456, 0: 789 }
    /// # process
    /// #     .source_iter(q!(vec![(1, 123), (2, 456), (0, 789)]))
    /// #     .into_keyed()
    /// #     .first();
    /// keyed_singleton.get_max_key()
    /// # .sample_eager(nondet!(/** test */))
    /// # }, |mut stream| async move {
    /// // (2, 456)
    /// # assert_eq!(stream.next().await.unwrap(), (2, 456));
    /// # }));
    /// ```
    pub fn get_max_key(self) -> Optional<(K, V), L, B::UnderlyingBound>
    where
        K: Ord,
    {
        self.entries()
            .assume_ordering(nondet!(
                /// There is only one element associated with each key, and the keys are totallly
                /// ordered so we will produce a deterministic value. We can't call
                /// `reduce_commutative_idempotent` because the closure technically isn't commutative
                /// in the case where both passed entries have the same key but different values.
                ///
                /// In the future, we may want to have an `assume!(...)` statement in the UDF that
                /// the two inputs do not have the same key.
            ))
            .reduce_idempotent(q!({
                move |curr, new| {
                    if new.0 > curr.0 {
                        *curr = new;
                    }
                }
            }))
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
        KeyedStream::new(
            self.location.clone(),
            HydroNode::Cast {
                inner: Box::new(self.ir_node.into_inner()),
                metadata: self.location.new_node_metadata(KeyedStream::<
                    K,
                    V,
                    L,
                    B::UnderlyingBound,
                    TotalOrder,
                    ExactlyOnce,
                >::collection_kind()),
            },
        )
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
        let result_stream = lookup_result
            .entries()
            .map(q!(|(v, (v2, k))| (k, (v, Some(v2)))))
            .into_keyed()
            .chain(
                missing_values
                    .entries()
                    .map(q!(|(v, k)| (k, (v, None))))
                    .into_keyed(),
            );

        KeyedSingleton::new(
            result_stream.location.clone(),
            HydroNode::Cast {
                inner: Box::new(result_stream.ir_node.into_inner()),
                metadata: result_stream.location.new_node_metadata(KeyedSingleton::<
                    K,
                    (V, Option<V2>),
                    Tick<L>,
                    Bounded,
                >::collection_kind(
                )),
            },
        )
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
        let out_location = Atomic { tick: tick.clone() };
        KeyedSingleton::new(
            out_location.clone(),
            HydroNode::BeginAtomic {
                inner: Box::new(self.ir_node.into_inner()),
                metadata: out_location
                    .new_node_metadata(KeyedSingleton::<K, V, Atomic<L>, B>::collection_kind()),
            },
        )
    }
}

impl<'a, K, V, L, B: KeyedSingletonBound> KeyedSingleton<K, V, Atomic<L>, B>
where
    L: Location<'a> + NoTick,
{
    /// Yields the elements of this keyed singleton back into a top-level, asynchronous execution context.
    /// See [`KeyedSingleton::atomic`] for more details.
    pub fn end_atomic(self) -> KeyedSingleton<K, V, L, B> {
        KeyedSingleton::new(
            self.location.tick.l.clone(),
            HydroNode::EndAtomic {
                inner: Box::new(self.ir_node.into_inner()),
                metadata: self
                    .location
                    .tick
                    .l
                    .new_node_metadata(KeyedSingleton::<K, V, L, B>::collection_kind()),
            },
        )
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
        KeyedSingleton::new(
            self.location.outer().clone(),
            HydroNode::YieldConcat {
                inner: Box::new(self.ir_node.into_inner()),
                metadata: self.location.outer().new_node_metadata(KeyedSingleton::<
                    K,
                    V,
                    L,
                    Unbounded,
                >::collection_kind(
                )),
            },
        )
    }

    /// Synchronously yields this keyed singleton outside the tick as an unbounded keyed singleton,
    /// which will be updated with the latest set of entries inside the tick.
    ///
    /// Unlike [`KeyedSingleton::latest`], this preserves synchronous execution, as the output
    /// keyed singleton is emitted in an [`Atomic`] context that will process elements synchronously
    /// with the input keyed singleton's [`Tick`] context.
    pub fn latest_atomic(self) -> KeyedSingleton<K, V, Atomic<L>, Unbounded> {
        let out_location = Atomic {
            tick: self.location.clone(),
        };

        KeyedSingleton::new(
            out_location.clone(),
            HydroNode::YieldConcat {
                inner: Box::new(self.ir_node.into_inner()),
                metadata: out_location.new_node_metadata(KeyedSingleton::<
                    K,
                    V,
                    Atomic<L>,
                    Unbounded,
                >::collection_kind()),
            },
        )
    }

    /// Shifts the state in `self` to the **next tick**, so that the returned keyed singleton at
    /// tick `T` always has the entries of `self` at tick `T - 1`.
    ///
    /// At tick `0`, the output has no entries, since there is no previous tick.
    ///
    /// This operator enables stateful iterative processing with ticks, by sending data from one
    /// tick to the next. For example, you can use it to compare state across consecutive batches.
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
    /// let input_batch = // first tick: { 1: 2, 2: 3 }, second tick: { 2: 4, 3: 5 }
    /// # batch_first_tick.chain(batch_second_tick).first();
    /// input_batch.clone().filter_key_not_in(
    ///     input_batch.defer_tick().keys() // keys present in the previous tick
    /// )
    /// # .entries().all_ticks()
    /// # }, |mut stream| async move {
    /// // { 1: 2, 2: 3 } (first tick), { 3: 5 } (second tick)
    /// # for w in vec![(1, 2), (2, 3), (3, 5)] {
    /// #     assert_eq!(stream.next().await.unwrap(), w);
    /// # }
    /// # }));
    /// ```
    pub fn defer_tick(self) -> KeyedSingleton<K, V, Tick<L>, Bounded> {
        KeyedSingleton::new(
            self.location.clone(),
            HydroNode::DeferTick {
                input: Box::new(self.ir_node.into_inner()),
                metadata: self
                    .location
                    .new_node_metadata(KeyedSingleton::<K, V, Tick<L>, Bounded>::collection_kind()),
            },
        )
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
        _nondet: NonDet,
    ) -> KeyedSingleton<K, V, Tick<L>, Bounded> {
        assert_eq!(Location::id(tick.outer()), Location::id(&self.location));
        KeyedSingleton::new(
            tick.clone(),
            HydroNode::Batch {
                inner: Box::new(self.ir_node.into_inner()),
                metadata: tick
                    .new_node_metadata(KeyedSingleton::<K, V, Tick<L>, Bounded>::collection_kind()),
            },
        )
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
    pub fn snapshot_atomic(self, _nondet: NonDet) -> KeyedSingleton<K, V, Tick<L>, Bounded> {
        KeyedSingleton::new(
            self.location.clone().tick,
            HydroNode::Batch {
                inner: Box::new(self.ir_node.into_inner()),
                metadata: self.location.tick.new_node_metadata(KeyedSingleton::<
                    K,
                    V,
                    Tick<L>,
                    Bounded,
                >::collection_kind(
                )),
            },
        )
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
        let _ = nondet;
        KeyedSingleton::new(
            self.location.clone().tick,
            HydroNode::Batch {
                inner: Box::new(self.ir_node.into_inner()),
                metadata: self.location.tick.new_node_metadata(KeyedSingleton::<
                    K,
                    V,
                    Tick<L>,
                    Bounded,
                >::collection_kind(
                )),
            },
        )
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

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

        let mut external_in = nodes.connect(input_port).await;
        let mut external_out = nodes.connect(out).await;

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

        let mut external_in = nodes.connect(input_port).await;
        let mut external_out = nodes.connect(out).await;

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

    #[tokio::test]
    async fn into_singleton_bounded_value() {
        let mut deployment = Deployment::new();

        let flow = FlowBuilder::new();
        let node = flow.process::<()>();
        let external = flow.external::<()>();

        let (input_port, input) = node.source_external_bincode(&external);
        let out = input
            .into_keyed()
            .first()
            .into_singleton()
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

        assert_eq!(external_out.next().await.unwrap(), HashMap::new());

        external_in.send((1, 1)).await.unwrap();
        assert_eq!(
            external_out.next().await.unwrap(),
            vec![(1, 1)].into_iter().collect()
        );

        external_in.send((2, 2)).await.unwrap();
        assert_eq!(
            external_out.next().await.unwrap(),
            vec![(1, 1), (2, 2)].into_iter().collect()
        );
    }

    #[tokio::test]
    async fn into_singleton_unbounded_value() {
        let mut deployment = Deployment::new();

        let flow = FlowBuilder::new();
        let node = flow.process::<()>();
        let external = flow.external::<()>();

        let (input_port, input) = node.source_external_bincode(&external);
        let out = input
            .into_keyed()
            .fold(q!(|| 0), q!(|acc, _| *acc += 1))
            .into_singleton()
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

        assert_eq!(external_out.next().await.unwrap(), HashMap::new());

        external_in.send((1, 1)).await.unwrap();
        assert_eq!(
            external_out.next().await.unwrap(),
            vec![(1, 1)].into_iter().collect()
        );

        external_in.send((1, 2)).await.unwrap();
        assert_eq!(
            external_out.next().await.unwrap(),
            vec![(1, 2)].into_iter().collect()
        );

        external_in.send((2, 2)).await.unwrap();
        assert_eq!(
            external_out.next().await.unwrap(),
            vec![(1, 2), (2, 1)].into_iter().collect()
        );

        external_in.send((1, 1)).await.unwrap();
        assert_eq!(
            external_out.next().await.unwrap(),
            vec![(1, 3), (2, 1)].into_iter().collect()
        );

        external_in.send((3, 1)).await.unwrap();
        assert_eq!(
            external_out.next().await.unwrap(),
            vec![(1, 3), (2, 1), (3, 1)].into_iter().collect()
        );
    }
}
