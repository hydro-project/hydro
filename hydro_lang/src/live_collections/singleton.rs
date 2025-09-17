//! Definitions for the [`Singleton`] live collection.

use std::cell::RefCell;
use std::marker::PhantomData;
use std::ops::Deref;
use std::rc::Rc;

use stageleft::{IntoQuotedMut, QuotedWithContext, q};

use super::boundedness::{Bounded, Boundedness, Unbounded};
use super::optional::Optional;
use super::stream::{AtLeastOnce, ExactlyOnce, NoOrder, Stream, TotalOrder};
use crate::compile::ir::{HydroIrOpMetadata, HydroNode, HydroRoot, TeeNode};
#[cfg(stageleft_runtime)]
use crate::forward_handle::{CycleCollection, CycleCollectionWithInitial, ReceiverComplete};
use crate::forward_handle::{ForwardRef, TickCycle};
#[cfg(stageleft_runtime)]
use crate::location::dynamic::{DynLocation, LocationId};
use crate::location::tick::{Atomic, DeferTick, NoAtomic};
use crate::location::{Location, NoTick, Tick, check_matching_location};
use crate::nondet::NonDet;

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
    L: Location<'a>,
{
    fn from(singleton: Singleton<T, L, Bounded>) -> Self {
        Singleton::new(singleton.location, singleton.ir_node.into_inner())
    }
}

impl<'a, T, L> DeferTick for Singleton<T, Tick<L>, Bounded>
where
    L: Location<'a>,
{
    fn defer_tick(self) -> Self {
        Singleton::defer_tick(self)
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
                    metadata: location.new_node_metadata::<T>(),
                }),
                metadata: location.new_node_metadata::<T>(),
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
                out_location: Location::id(&self.location),
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
                metadata: location.new_node_metadata::<T>(),
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
                out_location: Location::id(&self.location),
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
                metadata: self.location.new_node_metadata::<T>(),
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

impl<'a, T, L, B: Boundedness> Singleton<T, L, B>
where
    L: Location<'a>,
{
    pub(crate) fn new(location: L, ir_node: HydroNode) -> Self {
        Singleton {
            location,
            ir_node: RefCell::new(ir_node),
            _phantom: PhantomData,
        }
    }

    /// Transforms the singleton value by applying a function `f` to it,
    /// continuously as the input is updated.
    ///
    /// # Example
    /// ```rust
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
                metadata: self.location.new_node_metadata::<U>(),
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
                metadata: self.location.new_node_metadata::<U>(),
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
    /// # use hydro_lang::{prelude::*, live_collections::stream::{NoOrder, ExactlyOnce}};
    /// # use futures::StreamExt;
    /// # tokio_test::block_on(hydro_lang::test_util::stream_transform_test::<_, _, NoOrder, ExactlyOnce>(|process| {
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
                metadata: self.location.new_node_metadata::<U>(),
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
    /// # use hydro_lang::{prelude::*, live_collections::stream::{NoOrder, ExactlyOnce}};
    /// # use futures::StreamExt;
    /// # tokio_test::block_on(hydro_lang::test_util::stream_transform_test::<_, _, NoOrder, ExactlyOnce>(|process| {
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
                metadata: self.location.new_node_metadata::<T>(),
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
                metadata: self.location.new_node_metadata::<U>(),
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
    /// ```
    pub fn zip<O>(self, other: O) -> <Self as ZipResult<'a, O>>::Out
    where
        Self: ZipResult<'a, O, Location = L>,
    {
        check_matching_location(&self.location, &Self::other_location(&other));

        if L::is_top_level() {
            let left_ir_node = self.ir_node.into_inner();
            let left_ir_node_metadata = left_ir_node.metadata().clone();
            let right_ir_node = Self::other_ir_node(other);
            let right_ir_node_metadata = right_ir_node.metadata().clone();

            Self::make(
                self.location.clone(),
                HydroNode::Persist {
                    inner: Box::new(HydroNode::CrossSingleton {
                        left: Box::new(HydroNode::Unpersist {
                            inner: Box::new(left_ir_node),
                            metadata: left_ir_node_metadata,
                        }),
                        right: Box::new(HydroNode::Unpersist {
                            inner: Box::new(right_ir_node),
                            metadata: right_ir_node_metadata,
                        }),
                        metadata: self
                            .location
                            .new_node_metadata::<<Self as ZipResult<'a, O>>::ElementType>(),
                    }),
                    metadata: self
                        .location
                        .new_node_metadata::<<Self as ZipResult<'a, O>>::ElementType>(),
                },
            )
        } else {
            Self::make(
                self.location.clone(),
                HydroNode::CrossSingleton {
                    left: Box::new(self.ir_node.into_inner()),
                    right: Box::new(Self::other_ir_node(other)),
                    metadata: self
                        .location
                        .new_node_metadata::<<Self as ZipResult<'a, O>>::ElementType>(),
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
    pub fn snapshot(self, _nondet: NonDet) -> Singleton<T, Tick<L>, Bounded> {
        Singleton::new(
            self.location.clone().tick,
            HydroNode::Unpersist {
                inner: Box::new(self.ir_node.into_inner()),
                metadata: self.location.new_node_metadata::<T>(),
            },
        )
    }

    /// Returns this singleton back into a top-level, asynchronous execution context where updates
    /// to the value will be asynchronously propagated.
    pub fn end_atomic(self) -> Optional<T, L, B> {
        Optional::new(self.location.tick.l, self.ir_node.into_inner())
    }
}

impl<'a, T, L, B: Boundedness> Singleton<T, L, B>
where
    L: Location<'a> + NoTick + NoAtomic,
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
        Singleton::new(Atomic { tick: tick.clone() }, self.ir_node.into_inner())
    }

    /// Given a tick, returns a singleton value corresponding to a snapshot of the singleton
    /// as of that tick. The snapshot at tick `t + 1` is guaranteed to include at least all
    /// relevant data that contributed to the snapshot at tick `t`.
    ///
    /// # Non-Determinism
    /// Because this picks a snapshot of a singleton whose value is continuously changing,
    /// the output singleton has a non-deterministic value since the snapshot can be at an
    /// arbitrary point in time.
    pub fn snapshot(self, tick: &Tick<L>, nondet: NonDet) -> Singleton<T, Tick<L>, Bounded>
    where
        L: NoTick,
    {
        self.atomic(tick).snapshot(nondet)
    }

    /// Eagerly samples the singleton as fast as possible, returning a stream of snapshots
    /// with order corresponding to increasing prefixes of data contributing to the singleton.
    ///
    /// # Non-Determinism
    /// At runtime, the singleton will be arbitrarily sampled as fast as possible, but due
    /// to non-deterministic batching and arrival of inputs, the output stream is
    /// non-deterministic.
    pub fn sample_eager(self, nondet: NonDet) -> Stream<T, L, Unbounded, TotalOrder, AtLeastOnce> {
        let tick = self.location.tick();
        self.snapshot(&tick, nondet).all_ticks().weakest_retries()
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
    ) -> Stream<T, L, Unbounded, TotalOrder, AtLeastOnce> {
        let samples = self.location.source_interval(interval, nondet);
        let tick = self.location.tick();

        self.snapshot(&tick, nondet)
            .filter_if_some(samples.batch(&tick, nondet).first())
            .all_ticks()
            .weakest_retries()
    }
}

#[expect(missing_docs, reason = "TODO")]
impl<'a, T, L> Singleton<T, Tick<L>, Bounded>
where
    L: Location<'a>,
{
    pub fn all_ticks(self) -> Stream<T, L, Unbounded, TotalOrder, ExactlyOnce> {
        Stream::new(
            self.location.outer().clone(),
            HydroNode::Persist {
                inner: Box::new(self.ir_node.into_inner()),
                metadata: self.location.new_node_metadata::<T>(),
            },
        )
    }

    pub fn all_ticks_atomic(self) -> Stream<T, Atomic<L>, Unbounded, TotalOrder, ExactlyOnce> {
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

    pub fn latest(self) -> Singleton<T, L, Unbounded> {
        Singleton::new(
            self.location.outer().clone(),
            HydroNode::Persist {
                inner: Box::new(self.ir_node.into_inner()),
                metadata: self.location.new_node_metadata::<T>(),
            },
        )
    }

    pub fn latest_atomic(self) -> Singleton<T, Atomic<L>, Unbounded> {
        Singleton::new(
            Atomic {
                tick: self.location.clone(),
            },
            HydroNode::Persist {
                inner: Box::new(self.ir_node.into_inner()),
                metadata: self.location.new_node_metadata::<T>(),
            },
        )
    }

    pub fn defer_tick(self) -> Singleton<T, Tick<L>, Bounded> {
        Singleton::new(
            self.location.clone(),
            HydroNode::DeferTick {
                input: Box::new(self.ir_node.into_inner()),
                metadata: self.location.new_node_metadata::<T>(),
            },
        )
    }

    pub fn persist(self) -> Stream<T, Tick<L>, Bounded, TotalOrder, ExactlyOnce> {
        Stream::new(
            self.location.clone(),
            HydroNode::Persist {
                inner: Box::new(self.ir_node.into_inner()),
                metadata: self.location.new_node_metadata::<T>(),
            },
        )
    }

    pub fn delta(self) -> Optional<T, Tick<L>, Bounded> {
        Optional::new(
            self.location.clone(),
            HydroNode::Delta {
                inner: Box::new(self.ir_node.into_inner()),
                metadata: self.location.new_node_metadata::<T>(),
            },
        )
    }

    pub fn into_stream(self) -> Stream<T, Tick<L>, Bounded, TotalOrder, ExactlyOnce> {
        Stream::new(self.location, self.ir_node.into_inner())
    }
}

#[expect(missing_docs, reason = "TODO")]
pub trait ZipResult<'a, Other> {
    type Out;
    type ElementType;
    type Location;

    fn other_location(other: &Other) -> Self::Location;
    fn other_ir_node(other: Other) -> HydroNode;

    fn make(location: Self::Location, ir_node: HydroNode) -> Self::Out;
}

impl<'a, T, U, L, B: Boundedness> ZipResult<'a, Singleton<U, L, B>> for Singleton<T, L, B>
where
    L: Location<'a>,
{
    type Out = Singleton<(T, U), L, B>;
    type ElementType = (T, U);
    type Location = L;

    fn other_location(other: &Singleton<U, L, B>) -> L {
        other.location.clone()
    }

    fn other_ir_node(other: Singleton<U, L, B>) -> HydroNode {
        other.ir_node.into_inner()
    }

    fn make(location: L, ir_node: HydroNode) -> Self::Out {
        Singleton::new(location, ir_node)
    }
}

impl<'a, T, U, L, B: Boundedness> ZipResult<'a, Optional<U, L, B>> for Singleton<T, L, B>
where
    L: Location<'a>,
{
    type Out = Optional<(T, U), L, B>;
    type ElementType = (T, U);
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
    use futures::{SinkExt, StreamExt};
    use hydro_deploy::Deployment;
    use stageleft::q;

    use crate::compile::builder::FlowBuilder;
    use crate::location::Location;
    use crate::nondet::nondet;

    #[tokio::test]
    async fn tick_cycle_cardinality() {
        let mut deployment = Deployment::new();

        let flow = FlowBuilder::new();
        let node = flow.process::<()>();
        let external = flow.external::<()>();

        let (input_send, input) = node.source_external_bincode(&external);

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

        let mut tick_trigger = nodes.connect_sink_bincode(input_send).await;
        let mut external_out = nodes.connect_source_bincode(counts).await;

        deployment.start().await.unwrap();

        tick_trigger.send(()).await.unwrap();

        assert_eq!(external_out.next().await.unwrap(), 1);

        tick_trigger.send(()).await.unwrap();

        assert_eq!(external_out.next().await.unwrap(), 1);
    }
}
