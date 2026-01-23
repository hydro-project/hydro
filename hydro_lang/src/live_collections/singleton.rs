//! Definitions for the [`Singleton`] live collection.

use std::cell::RefCell;
use std::marker::PhantomData;
use std::ops::Deref;
use std::rc::Rc;

use stageleft::{IntoQuotedMut, QuotedWithContext, q};

use super::boundedness::{Bounded, Boundedness, Unbounded};
use super::optional::Optional;
use super::sliced::sliced;
use super::stream::{AtLeastOnce, ExactlyOnce, NoOrder, Stream, TotalOrder};
use crate::compile::ir::{CollectionKind, HydroIrOpMetadata, HydroNode, HydroRoot, TeeNode};
#[cfg(stageleft_runtime)]
use crate::forward_handle::{CycleCollection, CycleCollectionWithInitial, ReceiverComplete};
use crate::forward_handle::{ForwardRef, TickCycle};
#[cfg(stageleft_runtime)]
use crate::location::dynamic::{DynLocation, LocationId};
use crate::location::tick::{Atomic, NoAtomic};
use crate::location::{Location, NoTick, Tick, check_matching_location};
use crate::nondet::{NonDet, nondet};

/// A single Rust value that can asynchronously change over time.
///
/// If the singleton is [`Bounded`], the value is frozen and will not change. But if it is
/// [`Unbounded`], the value will asynchronously change over time.
///
/// Singletons are often used to capture state in a Hydro program, such as an event counter which is
/// a single number that will asynchronously change as events are processed. Singletons also appear
/// when dealing with bounded collections, to perform regular Rust computations on concrete values,
/// such as getting the length of a batch of requests.
///
/// Type Parameters:
/// - `Type`: the type of the value in this singleton
/// - `Loc`: the [`Location`] where the singleton is materialized
/// - `Bound`: tracks whether the value is [`Bounded`] (fixed) or [`Unbounded`] (changing asynchronously)
pub struct Singleton<Type, Loc, Bound: Boundedness> {
    pub(crate) location: Loc,
    pub(crate) ir_node: RefCell<HydroNode>,

    _phantom: PhantomData<(Type, Loc, Bound)>,
}

impl<'a, T, L> From<Singleton<T, L, Bounded>> for Singleton<T, L, Unbounded>
where
    T: Clone,
    L: Location<'a> + NoTick,
{
    fn from(value: Singleton<T, L, Bounded>) -> Self {
        let tick = value.location().tick();
        value.clone_into_tick(&tick).latest()
    }
}

impl<'a, T, L> CycleCollectionWithInitial<'a, TickCycle> for Singleton<T, Tick<L>, Bounded>
where
    L: Location<'a>,
{
    type Location = Tick<L>;

    fn create_source_with_initial(ident: syn::Ident, initial: Self, location: Tick<L>) -> Self {
        let from_previous_tick: Optional<T, Tick<L>, Bounded> = Optional::new(
            location.clone(),
            HydroNode::DeferTick {
                input: Box::new(HydroNode::CycleSource {
                    ident,
                    metadata: location.new_node_metadata(Self::collection_kind()),
                }),
                metadata: location
                    .new_node_metadata(Optional::<T, Tick<L>, Bounded>::collection_kind()),
            },
        );

        from_previous_tick.unwrap_or(initial)
    }
}

impl<'a, T, L> ReceiverComplete<'a, TickCycle> for Singleton<T, Tick<L>, Bounded>
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

impl<'a, T, L> CycleCollection<'a, ForwardRef> for Singleton<T, Tick<L>, Bounded>
where
    L: Location<'a>,
{
    type Location = Tick<L>;

    fn create_source(ident: syn::Ident, location: Tick<L>) -> Self {
        Singleton::new(
            location.clone(),
            HydroNode::CycleSource {
                ident,
                metadata: location.new_node_metadata(Self::collection_kind()),
            },
        )
    }
}

impl<'a, T, L> ReceiverComplete<'a, ForwardRef> for Singleton<T, Tick<L>, Bounded>
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

impl<'a, T, L, B: Boundedness> CycleCollection<'a, ForwardRef> for Singleton<T, L, B>
where
    L: Location<'a> + NoTick,
{
    type Location = L;

    fn create_source(ident: syn::Ident, location: L) -> Self {
        Singleton::new(
            location.clone(),
            HydroNode::CycleSource {
                ident,
                metadata: location.new_node_metadata(Self::collection_kind()),
            },
        )
    }
}

impl<'a, T, L, B: Boundedness> ReceiverComplete<'a, ForwardRef> for Singleton<T, L, B>
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

impl<'a, T, L, B: Boundedness> Clone for Singleton<T, L, B>
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
            Singleton {
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

#[cfg(stageleft_runtime)]
fn zip_inside_tick<'a, T, L: Location<'a>, B: Boundedness, O>(
    me: Singleton<T, Tick<L>, B>,
    other: O,
) -> <Singleton<T, Tick<L>, B> as ZipResult<'a, O>>::Out
where
    Singleton<T, Tick<L>, B>: ZipResult<'a, O, Location = Tick<L>>,
{
    check_matching_location(
        &me.location,
        &Singleton::<T, Tick<L>, B>::other_location(&other),
    );

    Singleton::<T, Tick<L>, B>::make(
        me.location.clone(),
        HydroNode::CrossSingleton {
            left: Box::new(me.ir_node.into_inner()),
            right: Box::new(Singleton::<T, Tick<L>, B>::other_ir_node(other)),
            metadata: me.location.new_node_metadata(CollectionKind::Singleton {
                bound: B::BOUND_KIND,
                element_type: stageleft::quote_type::<
                    <Singleton<T, Tick<L>, B> as ZipResult<'a, O>>::ElementType,
                >()
                .into(),
            }),
        },
    )
}

impl<'a, T, L, B: Boundedness> Singleton<T, L, B>
where
    L: Location<'a>,
{
    pub(crate) fn new(location: L, ir_node: HydroNode) -> Self {
        debug_assert_eq!(ir_node.metadata().location_id, Location::id(&location));
        debug_assert_eq!(ir_node.metadata().collection_kind, Self::collection_kind());
        Singleton {
            location,
            ir_node: RefCell::new(ir_node),
            _phantom: PhantomData,
        }
    }

    pub(crate) fn collection_kind() -> CollectionKind {
        CollectionKind::Singleton {
            bound: B::BOUND_KIND,
            element_type: stageleft::quote_type::<T>().into(),
        }
    }

    /// Returns the [`Location`] where this singleton is being materialized.
    pub fn location(&self) -> &L {
        &self.location
    }

    /// Transforms the singleton value by applying a function `f` to it,
    /// continuously as the input is updated.
    ///
    /// # Example
    /// ```rust
    /// # #[cfg(feature = "deploy")] {
    /// # use hydro_lang::prelude::*;
    /// # use futures::StreamExt;
    /// # tokio_test::block_on(hydro_lang::test_util::stream_transform_test(|process| {
    /// let tick = process.tick();
    /// let singleton = tick.singleton(q!(5));
    /// singleton.map(q!(|v| v * 2)).all_ticks()
    /// # }, |mut stream| async move {
    /// // 10
    /// # assert_eq!(stream.next().await.unwrap(), 10);
    /// # }));
    /// # }
    /// ```
    pub fn map<U, F>(self, f: impl IntoQuotedMut<'a, F, L>) -> Singleton<U, L, B>
    where
        F: Fn(T) -> U + 'a,
    {
        let f = f.splice_fn1_ctx(&self.location).into();
        Singleton::new(
            self.location.clone(),
            HydroNode::Map {
                f,
                input: Box::new(self.ir_node.into_inner()),
                metadata: self
                    .location
                    .new_node_metadata(Singleton::<U, L, B>::collection_kind()),
            },
        )
    }

    /// Transforms the singleton value by applying a function `f` to it and then flattening
    /// the result into a stream, preserving the order of elements.
    ///
    /// The function `f` is applied to the singleton value to produce an iterator, and all items
    /// from that iterator are emitted in the output stream in deterministic order.
    ///
    /// The implementation of [`Iterator`] for the output type `I` must produce items in a
    /// **deterministic** order. For example, `I` could be a `Vec`, but not a `HashSet`.
    /// If the order is not deterministic, use [`Singleton::flat_map_unordered`] instead.
    ///
    /// # Example
    /// ```rust
    /// # #[cfg(feature = "deploy")] {
    /// # use hydro_lang::prelude::*;
    /// # use futures::StreamExt;
    /// # tokio_test::block_on(hydro_lang::test_util::stream_transform_test(|process| {
    /// let tick = process.tick();
    /// let singleton = tick.singleton(q!(vec![1, 2, 3]));
    /// singleton.flat_map_ordered(q!(|v| v)).all_ticks()
    /// # }, |mut stream| async move {
    /// // 1, 2, 3
    /// # for w in vec![1, 2, 3] {
    /// #     assert_eq!(stream.next().await.unwrap(), w);
    /// # }
    /// # }));
    /// # }
    /// ```
    pub fn flat_map_ordered<U, I, F>(
        self,
        f: impl IntoQuotedMut<'a, F, L>,
    ) -> Stream<U, L, B, TotalOrder, ExactlyOnce>
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
                metadata: self.location.new_node_metadata(
                    Stream::<U, L, B, TotalOrder, ExactlyOnce>::collection_kind(),
                ),
            },
        )
    }

    /// Like [`Singleton::flat_map_ordered`], but allows the implementation of [`Iterator`]
    /// for the output type `I` to produce items in any order.
    ///
    /// The function `f` is applied to the singleton value to produce an iterator, and all items
    /// from that iterator are emitted in the output stream in non-deterministic order.
    ///
    /// # Example
    /// ```rust
    /// # #[cfg(feature = "deploy")] {
    /// # use hydro_lang::{prelude::*, live_collections::stream::{NoOrder, ExactlyOnce}};
    /// # use futures::StreamExt;
    /// # tokio_test::block_on(hydro_lang::test_util::stream_transform_test::<_, _, _, NoOrder, ExactlyOnce>(|process| {
    /// let tick = process.tick();
    /// let singleton = tick.singleton(q!(
    ///     std::collections::HashSet::<i32>::from_iter(vec![1, 2, 3])
    /// ));
    /// singleton.flat_map_unordered(q!(|v| v)).all_ticks()
    /// # }, |mut stream| async move {
    /// // 1, 2, 3, but in no particular order
    /// # let mut results = Vec::new();
    /// # for _ in 0..3 {
    /// #     results.push(stream.next().await.unwrap());
    /// # }
    /// # results.sort();
    /// # assert_eq!(results, vec![1, 2, 3]);
    /// # }));
    /// # }
    /// ```
    pub fn flat_map_unordered<U, I, F>(
        self,
        f: impl IntoQuotedMut<'a, F, L>,
    ) -> Stream<U, L, B, NoOrder, ExactlyOnce>
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
                    .new_node_metadata(Stream::<U, L, B, NoOrder, ExactlyOnce>::collection_kind()),
            },
        )
    }

    /// Flattens the singleton value into a stream, preserving the order of elements.
    ///
    /// The singleton value must implement [`IntoIterator`], and all items from that iterator
    /// are emitted in the output stream in deterministic order.
    ///
    /// The implementation of [`Iterator`] for the element type `T` must produce items in a
    /// **deterministic** order. For example, `T` could be a `Vec`, but not a `HashSet`.
    /// If the order is not deterministic, use [`Singleton::flatten_unordered`] instead.
    ///
    /// # Example
    /// ```rust
    /// # #[cfg(feature = "deploy")] {
    /// # use hydro_lang::prelude::*;
    /// # use futures::StreamExt;
    /// # tokio_test::block_on(hydro_lang::test_util::stream_transform_test(|process| {
    /// let tick = process.tick();
    /// let singleton = tick.singleton(q!(vec![1, 2, 3]));
    /// singleton.flatten_ordered().all_ticks()
    /// # }, |mut stream| async move {
    /// // 1, 2, 3
    /// # for w in vec![1, 2, 3] {
    /// #     assert_eq!(stream.next().await.unwrap(), w);
    /// # }
    /// # }));
    /// # }
    /// ```
    pub fn flatten_ordered<U>(self) -> Stream<U, L, B, TotalOrder, ExactlyOnce>
    where
        T: IntoIterator<Item = U>,
    {
        self.flat_map_ordered(q!(|x| x))
    }

    /// Like [`Singleton::flatten_ordered`], but allows the implementation of [`Iterator`]
    /// for the element type `T` to produce items in any order.
    ///
    /// The singleton value must implement [`IntoIterator`], and all items from that iterator
    /// are emitted in the output stream in non-deterministic order.
    ///
    /// # Example
    /// ```rust
    /// # #[cfg(feature = "deploy")] {
    /// # use hydro_lang::{prelude::*, live_collections::stream::{NoOrder, ExactlyOnce}};
    /// # use futures::StreamExt;
    /// # tokio_test::block_on(hydro_lang::test_util::stream_transform_test::<_, _, _, NoOrder, ExactlyOnce>(|process| {
    /// let tick = process.tick();
    /// let singleton = tick.singleton(q!(
    ///     std::collections::HashSet::<i32>::from_iter(vec![1, 2, 3])
    /// ));
    /// singleton.flatten_unordered().all_ticks()
    /// # }, |mut stream| async move {
    /// // 1, 2, 3, but in no particular order
    /// # let mut results = Vec::new();
    /// # for _ in 0..3 {
    /// #     results.push(stream.next().await.unwrap());
    /// # }
    /// # results.sort();
    /// # assert_eq!(results, vec![1, 2, 3]);
    /// # }));
    /// # }
    /// ```
    pub fn flatten_unordered<U>(self) -> Stream<U, L, B, NoOrder, ExactlyOnce>
    where
        T: IntoIterator<Item = U>,
    {
        self.flat_map_unordered(q!(|x| x))
    }

    /// Creates an optional containing the singleton value if it satisfies a predicate `f`.
    ///
    /// If the predicate returns `true`, the output optional contains the same value.
    /// If the predicate returns `false`, the output optional is empty.
    ///
    /// The closure `f` receives a reference `&T` rather than an owned value `T` because filtering does
    /// not modify or take ownership of the value. If you need to modify the value while filtering
    /// use [`Singleton::filter_map`] instead.
    ///
    /// # Example
    /// ```rust
    /// # #[cfg(feature = "deploy")] {
    /// # use hydro_lang::prelude::*;
    /// # use futures::StreamExt;
    /// # tokio_test::block_on(hydro_lang::test_util::stream_transform_test(|process| {
    /// let tick = process.tick();
    /// let singleton = tick.singleton(q!(5));
    /// singleton.filter(q!(|&x| x > 3)).all_ticks()
    /// # }, |mut stream| async move {
    /// // 5
    /// # assert_eq!(stream.next().await.unwrap(), 5);
    /// # }));
    /// # }
    /// ```
    pub fn filter<F>(self, f: impl IntoQuotedMut<'a, F, L>) -> Optional<T, L, B>
    where
        F: Fn(&T) -> bool + 'a,
    {
        let f = f.splice_fn1_borrow_ctx(&self.location).into();
        Optional::new(
            self.location.clone(),
            HydroNode::Filter {
                f,
                input: Box::new(self.ir_node.into_inner()),
                metadata: self
                    .location
                    .new_node_metadata(Optional::<T, L, B>::collection_kind()),
            },
        )
    }

    /// An operator that both filters and maps. It yields the value only if the supplied
    /// closure `f` returns `Some(value)`.
    ///
    /// If the closure returns `Some(new_value)`, the output optional contains `new_value`.
    /// If the closure returns `None`, the output optional is empty.
    ///
    /// # Example
    /// ```rust
    /// # #[cfg(feature = "deploy")] {
    /// # use hydro_lang::prelude::*;
    /// # use futures::StreamExt;
    /// # tokio_test::block_on(hydro_lang::test_util::stream_transform_test(|process| {
    /// let tick = process.tick();
    /// let singleton = tick.singleton(q!("42"));
    /// singleton
    ///     .filter_map(q!(|s| s.parse::<i32>().ok()))
    ///     .all_ticks()
    /// # }, |mut stream| async move {
    /// // 42
    /// # assert_eq!(stream.next().await.unwrap(), 42);
    /// # }));
    /// # }
    /// ```
    pub fn filter_map<U, F>(self, f: impl IntoQuotedMut<'a, F, L>) -> Optional<U, L, B>
    where
        F: Fn(T) -> Option<U> + 'a,
    {
        let f = f.splice_fn1_ctx(&self.location).into();
        Optional::new(
            self.location.clone(),
            HydroNode::FilterMap {
                f,
                input: Box::new(self.ir_node.into_inner()),
                metadata: self
                    .location
                    .new_node_metadata(Optional::<U, L, B>::collection_kind()),
            },
        )
    }

    /// Combines this singleton with another [`Singleton`] or [`Optional`] by tupling their values.
    ///
    /// If the other value is a [`Singleton`], the output will be a [`Singleton`], but if it is an
    /// [`Optional`], the output will be an [`Optional`] that is non-null only if the argument is
    /// non-null. This is useful for combining several pieces of state together.
    ///
    /// # Example
    /// ```rust
    /// # #[cfg(feature = "deploy")] {
    /// # use hydro_lang::prelude::*;
    /// # use futures::StreamExt;
    /// # tokio_test::block_on(hydro_lang::test_util::stream_transform_test(|process| {
    /// let tick = process.tick();
    /// let numbers = process
    ///   .source_iter(q!(vec![123, 456]))
    ///   .batch(&tick, nondet!(/** test */));
    /// let count = numbers.clone().count(); // Singleton
    /// let max = numbers.max(); // Optional
    /// count.zip(max).all_ticks()
    /// # }, |mut stream| async move {
    /// // [(2, 456)]
    /// # for w in vec![(2, 456)] {
    /// #     assert_eq!(stream.next().await.unwrap(), w);
    /// # }
    /// # }));
    /// # }
    /// ```
    pub fn zip<O>(self, other: O) -> <Self as ZipResult<'a, O>>::Out
    where
        Self: ZipResult<'a, O, Location = L>,
    {
        check_matching_location(&self.location, &Self::other_location(&other));

        if L::is_top_level()
            && let Some(tick) = self.location.try_tick()
        {
            let out = zip_inside_tick(
                self.snapshot(&tick, nondet!(/** eventually stabilizes */)),
                Optional::<<Self as ZipResult<'a, O>>::OtherType, L, B>::new(
                    Self::other_location(&other),
                    Self::other_ir_node(other),
                )
                .snapshot(&tick, nondet!(/** eventually stabilizes */)),
            )
            .latest();

            Self::make(out.location, out.ir_node.into_inner())
        } else {
            Self::make(
                self.location.clone(),
                HydroNode::CrossSingleton {
                    left: Box::new(self.ir_node.into_inner()),
                    right: Box::new(Self::other_ir_node(other)),
                    metadata: self.location.new_node_metadata(CollectionKind::Optional {
                        bound: B::BOUND_KIND,
                        element_type: stageleft::quote_type::<
                            <Self as ZipResult<'a, O>>::ElementType,
                        >()
                        .into(),
                    }),
                },
            )
        }
    }

    /// Filters this singleton into an [`Optional`], passing through the singleton value if the
    /// argument (a [`Bounded`] [`Optional`]`) is non-null, otherwise the output is null.
    ///
    /// Useful for conditionally processing, such as only emitting a singleton's value outside
    /// a tick if some other condition is satisfied.
    ///
    /// # Example
    /// ```rust
    /// # #[cfg(feature = "deploy")] {
    /// # use hydro_lang::prelude::*;
    /// # use futures::StreamExt;
    /// # tokio_test::block_on(hydro_lang::test_util::stream_transform_test(|process| {
    /// let tick = process.tick();
    /// // ticks are lazy by default, forces the second tick to run
    /// tick.spin_batch(q!(1)).all_ticks().for_each(q!(|_| {}));
    ///
    /// let batch_first_tick = process
    ///   .source_iter(q!(vec![1]))
    ///   .batch(&tick, nondet!(/** test */));
    /// let batch_second_tick = process
    ///   .source_iter(q!(vec![1, 2, 3]))
    ///   .batch(&tick, nondet!(/** test */))
    ///   .defer_tick(); // appears on the second tick
    /// let some_on_first_tick = tick.optional_first_tick(q!(()));
    /// batch_first_tick.chain(batch_second_tick).count()
    ///   .filter_if_some(some_on_first_tick)
    ///   .all_ticks()
    /// # }, |mut stream| async move {
    /// // [1]
    /// # for w in vec![1] {
    /// #     assert_eq!(stream.next().await.unwrap(), w);
    /// # }
    /// # }));
    /// # }
    /// ```
    pub fn filter_if_some<U>(self, signal: Optional<U, L, B>) -> Optional<T, L, B> {
        self.zip::<Optional<(), L, B>>(signal.map(q!(|_u| ())))
            .map(q!(|(d, _signal)| d))
    }

    /// Filters this singleton into an [`Optional`], passing through the singleton value if the
    /// argument (a [`Bounded`] [`Optional`]`) is null, otherwise the output is null.
    ///
    /// Like [`Singleton::filter_if_some`], this is useful for conditional processing, but inverts
    /// the condition.
    ///
    /// # Example
    /// ```rust
    /// # #[cfg(feature = "deploy")] {
    /// # use hydro_lang::prelude::*;
    /// # use futures::StreamExt;
    /// # tokio_test::block_on(hydro_lang::test_util::stream_transform_test(|process| {
    /// let tick = process.tick();
    /// // ticks are lazy by default, forces the second tick to run
    /// tick.spin_batch(q!(1)).all_ticks().for_each(q!(|_| {}));
    ///
    /// let batch_first_tick = process
    ///   .source_iter(q!(vec![1]))
    ///   .batch(&tick, nondet!(/** test */));
    /// let batch_second_tick = process
    ///   .source_iter(q!(vec![1, 2, 3]))
    ///   .batch(&tick, nondet!(/** test */))
    ///   .defer_tick(); // appears on the second tick
    /// let some_on_first_tick = tick.optional_first_tick(q!(()));
    /// batch_first_tick.chain(batch_second_tick).count()
    ///   .filter_if_none(some_on_first_tick)
    ///   .all_ticks()
    /// # }, |mut stream| async move {
    /// // [3]
    /// # for w in vec![3] {
    /// #     assert_eq!(stream.next().await.unwrap(), w);
    /// # }
    /// # }));
    /// # }
    /// ```
    pub fn filter_if_none<U>(self, other: Optional<U, L, B>) -> Optional<T, L, B> {
        self.filter_if_some(
            other
                .map(q!(|_| ()))
                .into_singleton()
                .filter(q!(|o| o.is_none())),
        )
    }

    /// An operator which allows you to "name" a `HydroNode`.
    /// This is only used for testing, to correlate certain `HydroNode`s with IDs.
    pub fn ir_node_named(self, name: &str) -> Singleton<T, L, B> {
        {
            let mut node = self.ir_node.borrow_mut();
            let metadata = node.metadata_mut();
            metadata.tag = Some(name.to_string());
        }
        self
    }
}

impl<'a, T, L, B: Boundedness> Singleton<T, Atomic<L>, B>
where
    L: Location<'a> + NoTick,
{
    /// Returns a singleton value corresponding to the latest snapshot of the singleton
    /// being atomically processed. The snapshot at tick `t + 1` is guaranteed to include
    /// at least all relevant data that contributed to the snapshot at tick `t`. Furthermore,
    /// all snapshots of this singleton into the atomic-associated tick will observe the
    /// same value each tick.
    ///
    /// # Non-Determinism
    /// Because this picks a snapshot of a singleton whose value is continuously changing,
    /// the output singleton has a non-deterministic value since the snapshot can be at an
    /// arbitrary point in time.
    pub fn snapshot_atomic(self, _nondet: NonDet) -> Singleton<T, Tick<L>, Bounded> {
        Singleton::new(
            self.location.clone().tick,
            HydroNode::Batch {
                inner: Box::new(self.ir_node.into_inner()),
                metadata: self
                    .location
                    .tick
                    .new_node_metadata(Singleton::<T, Tick<L>, Bounded>::collection_kind()),
            },
        )
    }

    /// Returns this singleton back into a top-level, asynchronous execution context where updates
    /// to the value will be asynchronously propagated.
    pub fn end_atomic(self) -> Singleton<T, L, B> {
        Singleton::new(
            self.location.tick.l.clone(),
            HydroNode::EndAtomic {
                inner: Box::new(self.ir_node.into_inner()),
                metadata: self
                    .location
                    .tick
                    .l
                    .new_node_metadata(Singleton::<T, L, B>::collection_kind()),
            },
        )
    }
}

impl<'a, T, L, B: Boundedness> Singleton<T, L, B>
where
    L: Location<'a>,
{
    /// Shifts this singleton into an atomic context, which guarantees that any downstream logic
    /// will observe the same version of the value and will be executed synchronously before any
    /// outputs are yielded (in [`Optional::end_atomic`]).
    ///
    /// This is useful to enforce local consistency constraints, such as ensuring that several readers
    /// see a consistent version of local state (since otherwise each [`Singleton::snapshot`] may pick
    /// a different version).
    ///
    /// Entering an atomic section requires a [`Tick`] argument that declares where the singleton will
    /// be atomically processed. Snapshotting an singleton into the _same_ [`Tick`] will preserve the
    /// synchronous execution, and all such snapshots in the same [`Tick`] will have the same value.
    pub fn atomic(self, tick: &Tick<L>) -> Singleton<T, Atomic<L>, B> {
        let out_location = Atomic { tick: tick.clone() };
        Singleton::new(
            out_location.clone(),
            HydroNode::BeginAtomic {
                inner: Box::new(self.ir_node.into_inner()),
                metadata: out_location
                    .new_node_metadata(Singleton::<T, Atomic<L>, B>::collection_kind()),
            },
        )
    }

    /// Given a tick, returns a singleton value corresponding to a snapshot of the singleton
    /// as of that tick. The snapshot at tick `t + 1` is guaranteed to include at least all
    /// relevant data that contributed to the snapshot at tick `t`.
    ///
    /// # Non-Determinism
    /// Because this picks a snapshot of a singleton whose value is continuously changing,
    /// the output singleton has a non-deterministic value since the snapshot can be at an
    /// arbitrary point in time.
    pub fn snapshot(self, tick: &Tick<L>, _nondet: NonDet) -> Singleton<T, Tick<L>, Bounded> {
        assert_eq!(Location::id(tick.outer()), Location::id(&self.location));
        Singleton::new(
            tick.clone(),
            HydroNode::Batch {
                inner: Box::new(self.ir_node.into_inner()),
                metadata: tick
                    .new_node_metadata(Singleton::<T, Tick<L>, Bounded>::collection_kind()),
            },
        )
    }

    /// Eagerly samples the singleton as fast as possible, returning a stream of snapshots
    /// with order corresponding to increasing prefixes of data contributing to the singleton.
    ///
    /// # Non-Determinism
    /// At runtime, the singleton will be arbitrarily sampled as fast as possible, but due
    /// to non-deterministic batching and arrival of inputs, the output stream is
    /// non-deterministic.
    pub fn sample_eager(self, nondet: NonDet) -> Stream<T, L, Unbounded, TotalOrder, AtLeastOnce>
    where
        L: NoTick,
    {
        sliced! {
            let snapshot = use(self, nondet);
            snapshot.into_stream()
        }
        .weakest_retries()
    }

    /// Given a time interval, returns a stream corresponding to snapshots of the singleton
    /// value taken at various points in time. Because the input singleton may be
    /// [`Unbounded`], there are no guarantees on what these snapshots are other than they
    /// represent the value of the singleton given some prefix of the streams leading up to
    /// it.
    ///
    /// # Non-Determinism
    /// The output stream is non-deterministic in which elements are sampled, since this
    /// is controlled by a clock.
    pub fn sample_every(
        self,
        interval: impl QuotedWithContext<'a, std::time::Duration, L> + Copy + 'a,
        nondet: NonDet,
    ) -> Stream<T, L, Unbounded, TotalOrder, AtLeastOnce>
    where
        L: NoTick + NoAtomic,
    {
        let samples = self.location.source_interval(interval, nondet);
        sliced! {
            let snapshot = use(self, nondet);
            let sample_batch = use(samples, nondet);

            snapshot.filter_if_some(sample_batch.first()).into_stream()
        }
        .weakest_retries()
    }
}

impl<'a, T, L> Singleton<T, L, Bounded>
where
    L: Location<'a>,
{
    /// Clones this bounded singleton into a tick, returning a singleton that has the
    /// same value as the outer singleton. Because the outer singleton is bounded, this
    /// is deterministic because there is only a single immutable version.
    pub fn clone_into_tick(self, tick: &Tick<L>) -> Singleton<T, Tick<L>, Bounded>
    where
        T: Clone,
    {
        // TODO(shadaj): avoid printing simulator logs for this snapshot
        self.snapshot(
            tick,
            nondet!(/** bounded top-level singleton so deterministic */),
        )
    }

    /// Converts this singleton into a [`Stream`] containing a single element, the value.
    ///
    /// # Example
    /// ```rust
    /// # #[cfg(feature = "deploy")] {
    /// # use hydro_lang::prelude::*;
    /// # use futures::StreamExt;
    /// # tokio_test::block_on(hydro_lang::test_util::stream_transform_test(|process| {
    /// let tick = process.tick();
    /// let batch_input = process
    ///   .source_iter(q!(vec![123, 456]))
    ///   .batch(&tick, nondet!(/** test */));
    /// batch_input.clone().chain(
    ///   batch_input.count().into_stream()
    /// ).all_ticks()
    /// # }, |mut stream| async move {
    /// // [123, 456, 2]
    /// # for w in vec![123, 456, 2] {
    /// #     assert_eq!(stream.next().await.unwrap(), w);
    /// # }
    /// # }));
    /// # }
    /// ```
    pub fn into_stream(self) -> Stream<T, L, Bounded, TotalOrder, ExactlyOnce> {
        Stream::new(
            self.location.clone(),
            HydroNode::Cast {
                inner: Box::new(self.ir_node.into_inner()),
                metadata: self.location.new_node_metadata(Stream::<
                    T,
                    Tick<L>,
                    Bounded,
                    TotalOrder,
                    ExactlyOnce,
                >::collection_kind()),
            },
        )
    }
}

impl<'a, T, L> Singleton<T, Tick<L>, Bounded>
where
    L: Location<'a>,
{
    /// Asynchronously yields the value of this singleton outside the tick as an unbounded stream,
    /// which will stream the value computed in _each_ tick as a separate stream element.
    ///
    /// Unlike [`Singleton::latest`], the value computed in each tick is emitted separately,
    /// producing one element in the output for each tick. This is useful for batched computations,
    /// where the results from each tick must be combined together.
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
    /// #   .source_iter(q!(vec![1]))
    /// #   .batch(&tick, nondet!(/** test */));
    /// # let batch_second_tick = process
    /// #   .source_iter(q!(vec![1, 2, 3]))
    /// #   .batch(&tick, nondet!(/** test */))
    /// #   .defer_tick(); // appears on the second tick
    /// # let input_batch = batch_first_tick.chain(batch_second_tick);
    /// input_batch // first tick: [1], second tick: [1, 2, 3]
    ///     .count()
    ///     .all_ticks()
    /// # }, |mut stream| async move {
    /// // [1, 3]
    /// # for w in vec![1, 3] {
    /// #     assert_eq!(stream.next().await.unwrap(), w);
    /// # }
    /// # }));
    /// # }
    /// ```
    pub fn all_ticks(self) -> Stream<T, L, Unbounded, TotalOrder, ExactlyOnce> {
        self.into_stream().all_ticks()
    }

    /// Synchronously yields the value of this singleton outside the tick as an unbounded stream,
    /// which will stream the value computed in _each_ tick as a separate stream element.
    ///
    /// Unlike [`Singleton::all_ticks`], this preserves synchronous execution, as the output stream
    /// is emitted in an [`Atomic`] context that will process elements synchronously with the input
    /// singleton's [`Tick`] context.
    pub fn all_ticks_atomic(self) -> Stream<T, Atomic<L>, Unbounded, TotalOrder, ExactlyOnce> {
        self.into_stream().all_ticks_atomic()
    }

    /// Asynchronously yields this singleton outside the tick as an unbounded singleton, which will
    /// be asynchronously updated with the latest value of the singleton inside the tick.
    ///
    /// This converts a bounded value _inside_ a tick into an asynchronous value outside the
    /// tick that tracks the inner value. This is useful for getting the value as of the
    /// "most recent" tick, but note that updates are propagated asynchronously outside the tick.
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
    /// #   .source_iter(q!(vec![1]))
    /// #   .batch(&tick, nondet!(/** test */));
    /// # let batch_second_tick = process
    /// #   .source_iter(q!(vec![1, 2, 3]))
    /// #   .batch(&tick, nondet!(/** test */))
    /// #   .defer_tick(); // appears on the second tick
    /// # let input_batch = batch_first_tick.chain(batch_second_tick);
    /// input_batch // first tick: [1], second tick: [1, 2, 3]
    ///     .count()
    ///     .latest()
    /// # .sample_eager(nondet!(/** test */))
    /// # }, |mut stream| async move {
    /// // asynchronously changes from 1 ~> 3
    /// # for w in vec![1, 3] {
    /// #     assert_eq!(stream.next().await.unwrap(), w);
    /// # }
    /// # }));
    /// # }
    /// ```
    pub fn latest(self) -> Singleton<T, L, Unbounded> {
        Singleton::new(
            self.location.outer().clone(),
            HydroNode::YieldConcat {
                inner: Box::new(self.ir_node.into_inner()),
                metadata: self
                    .location
                    .outer()
                    .new_node_metadata(Singleton::<T, L, Unbounded>::collection_kind()),
            },
        )
    }

    /// Synchronously yields this singleton outside the tick as an unbounded singleton, which will
    /// be updated with the latest value of the singleton inside the tick.
    ///
    /// Unlike [`Singleton::latest`], this preserves synchronous execution, as the output singleton
    /// is emitted in an [`Atomic`] context that will process elements synchronously with the input
    /// singleton's [`Tick`] context.
    pub fn latest_atomic(self) -> Singleton<T, Atomic<L>, Unbounded> {
        let out_location = Atomic {
            tick: self.location.clone(),
        };
        Singleton::new(
            out_location.clone(),
            HydroNode::YieldConcat {
                inner: Box::new(self.ir_node.into_inner()),
                metadata: out_location
                    .new_node_metadata(Singleton::<T, Atomic<L>, Unbounded>::collection_kind()),
            },
        )
    }
}

#[doc(hidden)]
/// Helper trait that determines the output collection type for [`Singleton::zip`].
///
/// The output will be an [`Optional`] if the second input is an [`Optional`], otherwise it is a
/// [`Singleton`].
#[sealed::sealed]
pub trait ZipResult<'a, Other> {
    /// The output collection type.
    type Out;
    /// The type of the tupled output value.
    type ElementType;
    /// The type of the other collection's value.
    type OtherType;
    /// The location where the tupled result will be materialized.
    type Location: Location<'a>;

    /// The location of the second input to the `zip`.
    fn other_location(other: &Other) -> Self::Location;
    /// The IR node of the second input to the `zip`.
    fn other_ir_node(other: Other) -> HydroNode;

    /// Constructs the output live collection given an IR node containing the zip result.
    fn make(location: Self::Location, ir_node: HydroNode) -> Self::Out;
}

#[sealed::sealed]
impl<'a, T, U, L, B: Boundedness> ZipResult<'a, Singleton<U, L, B>> for Singleton<T, L, B>
where
    L: Location<'a>,
{
    type Out = Singleton<(T, U), L, B>;
    type ElementType = (T, U);
    type OtherType = U;
    type Location = L;

    fn other_location(other: &Singleton<U, L, B>) -> L {
        other.location.clone()
    }

    fn other_ir_node(other: Singleton<U, L, B>) -> HydroNode {
        other.ir_node.into_inner()
    }

    fn make(location: L, ir_node: HydroNode) -> Self::Out {
        Singleton::new(
            location.clone(),
            HydroNode::Cast {
                inner: Box::new(ir_node),
                metadata: location.new_node_metadata(Self::Out::collection_kind()),
            },
        )
    }
}

#[sealed::sealed]
impl<'a, T, U, L, B: Boundedness> ZipResult<'a, Optional<U, L, B>> for Singleton<T, L, B>
where
    L: Location<'a>,
{
    type Out = Optional<(T, U), L, B>;
    type ElementType = (T, U);
    type OtherType = U;
    type Location = L;

    fn other_location(other: &Optional<U, L, B>) -> L {
        other.location.clone()
    }

    fn other_ir_node(other: Optional<U, L, B>) -> HydroNode {
        other.ir_node.into_inner()
    }

    fn make(location: L, ir_node: HydroNode) -> Self::Out {
        Optional::new(location, ir_node)
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
    #[cfg(any(feature = "deploy", feature = "sim"))]
    use crate::location::Location;
    #[cfg(any(feature = "deploy", feature = "sim"))]
    use crate::nondet::nondet;

    #[cfg(feature = "deploy")]
    #[tokio::test]
    async fn tick_cycle_cardinality() {
        let mut deployment = Deployment::new();

        let mut flow = FlowBuilder::new();
        let node = flow.process::<()>();
        let external = flow.external::<()>();

        let (input_send, input) = node.source_external_bincode::<_, _, _, ExactlyOnce>(&external);

        let node_tick = node.tick();
        let (complete_cycle, singleton) = node_tick.cycle_with_initial(node_tick.singleton(q!(0)));
        let counts = singleton
            .clone()
            .into_stream()
            .count()
            .filter_if_some(input.batch(&node_tick, nondet!(/** testing */)).first())
            .all_ticks()
            .send_bincode_external(&external);
        complete_cycle.complete_next_tick(singleton);

        let nodes = flow
            .with_process(&node, deployment.Localhost())
            .with_external(&external, deployment.Localhost())
            .deploy(&mut deployment);

        deployment.deploy().await.unwrap();

        let mut tick_trigger = nodes.connect(input_send).await;
        let mut external_out = nodes.connect(counts).await;

        deployment.start().await.unwrap();

        tick_trigger.send(()).await.unwrap();

        assert_eq!(external_out.next().await.unwrap(), 1);

        tick_trigger.send(()).await.unwrap();

        assert_eq!(external_out.next().await.unwrap(), 1);
    }

    #[cfg(feature = "sim")]
    #[test]
    #[should_panic]
    fn sim_fold_intermediate_states() {
        let mut flow = FlowBuilder::new();
        let node = flow.process::<()>();

        let source = node.source_stream(q!(tokio_stream::iter(vec![1, 2, 3, 4])));
        let folded = source.fold(q!(|| 0), q!(|a, b| *a += b));

        let tick = node.tick();
        let batch = folded.snapshot(&tick, nondet!(/** test */));
        let out_recv = batch.all_ticks().sim_output();

        flow.sim().exhaustive(async || {
            assert_eq!(out_recv.next().await.unwrap(), 10);
        });
    }

    #[cfg(feature = "sim")]
    #[test]
    fn sim_fold_intermediate_state_count() {
        let mut flow = FlowBuilder::new();
        let node = flow.process::<()>();

        let source = node.source_stream(q!(tokio_stream::iter(vec![1, 2, 3, 4])));
        let folded = source.fold(q!(|| 0), q!(|a, b| *a += b));

        let tick = node.tick();
        let batch = folded.snapshot(&tick, nondet!(/** test */));
        let out_recv = batch.all_ticks().sim_output();

        let instance_count = flow.sim().exhaustive(async || {
            let out = out_recv.collect::<Vec<_>>().await;
            assert_eq!(out.last(), Some(&10));
        });

        assert_eq!(
            instance_count,
            16 // 2^4 possible subsets of intermediates (including initial state)
        )
    }

    #[cfg(feature = "sim")]
    #[test]
    fn sim_fold_no_repeat_initial() {
        // check that we don't repeat the initial state of the fold in autonomous decisions

        let mut flow = FlowBuilder::new();
        let node = flow.process::<()>();

        let (in_port, input) = node.sim_input();
        let folded = input.fold(q!(|| 0), q!(|a, b| *a += b));

        let tick = node.tick();
        let batch = folded.snapshot(&tick, nondet!(/** test */));
        let out_recv = batch.all_ticks().sim_output();

        flow.sim().exhaustive(async || {
            assert_eq!(out_recv.next().await.unwrap(), 0);

            in_port.send(123);

            assert_eq!(out_recv.next().await.unwrap(), 123);
        });
    }

    #[cfg(feature = "sim")]
    #[test]
    #[should_panic]
    fn sim_fold_repeats_snapshots() {
        // when the tick is driven by a snapshot AND something else, the snapshot can
        // "stutter" and repeat the same state multiple times

        let mut flow = FlowBuilder::new();
        let node = flow.process::<()>();

        let source = node.source_stream(q!(tokio_stream::iter(vec![1, 2, 3, 4])));
        let folded = source.clone().fold(q!(|| 0), q!(|a, b| *a += b));

        let tick = node.tick();
        let batch = source
            .batch(&tick, nondet!(/** test */))
            .cross_singleton(folded.snapshot(&tick, nondet!(/** test */)));
        let out_recv = batch.all_ticks().sim_output();

        flow.sim().exhaustive(async || {
            if out_recv.next().await.unwrap() == (1, 3) && out_recv.next().await.unwrap() == (2, 3)
            {
                panic!("repeated snapshot");
            }
        });
    }

    #[cfg(feature = "sim")]
    #[test]
    fn sim_fold_repeats_snapshots_count() {
        // check the number of instances
        let mut flow = FlowBuilder::new();
        let node = flow.process::<()>();

        let source = node.source_stream(q!(tokio_stream::iter(vec![1, 2])));
        let folded = source.clone().fold(q!(|| 0), q!(|a, b| *a += b));

        let tick = node.tick();
        let batch = source
            .batch(&tick, nondet!(/** test */))
            .cross_singleton(folded.snapshot(&tick, nondet!(/** test */)));
        let out_recv = batch.all_ticks().sim_output();

        let count = flow.sim().exhaustive(async || {
            let _ = out_recv.collect::<Vec<_>>().await;
        });

        assert_eq!(count, 52);
        // don't have a combinatorial explanation for this number yet, but checked via logs
    }

    #[cfg(feature = "sim")]
    #[test]
    fn sim_top_level_singleton_exhaustive() {
        // ensures that top-level singletons have only one snapshot
        let mut flow = FlowBuilder::new();
        let node = flow.process::<()>();

        let singleton = node.singleton(q!(1));
        let tick = node.tick();
        let batch = singleton.snapshot(&tick, nondet!(/** test */));
        let out_recv = batch.all_ticks().sim_output();

        let count = flow.sim().exhaustive(async || {
            let _ = out_recv.collect::<Vec<_>>().await;
        });

        assert_eq!(count, 1);
    }

    #[cfg(feature = "sim")]
    #[test]
    fn sim_top_level_singleton_join_count() {
        // if a tick consumes a static snapshot and a stream batch, only the batch require space
        // exploration

        let mut flow = FlowBuilder::new();
        let node = flow.process::<()>();

        let source_iter = node.source_iter(q!(vec![1, 2, 3, 4]));
        let tick = node.tick();
        let batch = source_iter
            .batch(&tick, nondet!(/** test */))
            .cross_singleton(node.singleton(q!(123)).clone_into_tick(&tick));
        let out_recv = batch.all_ticks().sim_output();

        let instance_count = flow.sim().exhaustive(async || {
            let _ = out_recv.collect::<Vec<_>>().await;
        });

        assert_eq!(
            instance_count,
            16 // 2^4 ways to split up (including a possibly empty first batch)
        )
    }

    #[cfg(feature = "sim")]
    #[test]
    fn top_level_singleton_into_stream_no_replay() {
        let mut flow = FlowBuilder::new();
        let node = flow.process::<()>();

        let source_iter = node.source_iter(q!(vec![1, 2, 3, 4]));
        let folded = source_iter.fold(q!(|| 0), q!(|a, b| *a += b));

        let out_recv = folded.into_stream().sim_output();

        flow.sim().exhaustive(async || {
            out_recv.assert_yields_only([10]).await;
        });
    }
}
