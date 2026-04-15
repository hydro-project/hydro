//! Definitions for the [`Singleton`] live collection.

use std::cell::RefCell;
use std::marker::PhantomData;
use std::ops::{Deref, Not};
use std::rc::Rc;

use sealed::sealed;
use stageleft::{IntoQuotedMut, QuotedWithContext, q};

use super::boundedness::{Bounded, Boundedness, IsBounded, Unbounded};
use super::optional::Optional;
use super::sliced::sliced;
use super::stream::{AtLeastOnce, ExactlyOnce, NoOrder, Stream, TotalOrder};
use crate::compile::builder::{CycleId, FlowState};
use crate::compile::ir::{
    CollectionKind, HydroIrOpMetadata, HydroNode, HydroRoot, SharedNode, SingletonBoundKind,
};
#[cfg(stageleft_runtime)]
use crate::forward_handle::{CycleCollection, CycleCollectionWithInitial, ReceiverComplete};
use crate::forward_handle::{ForwardRef, TickCycle};
#[cfg(stageleft_runtime)]
use crate::location::dynamic::{DynLocation, LocationId};
use crate::location::tick::{Atomic, NoAtomic};
use crate::location::{Location, NoTick, Tick, check_matching_location};
use crate::nondet::{NonDet, nondet};
use crate::properties::{ApplyMonotoneStream, Proved};

/// A marker trait indicating which components of a [`Singleton`] may change.
///
/// In addition to [`Bounded`] (immutable) and [`Unbounded`] (arbitrarily mutable), this also
/// includes an additional variant [`Monotonic`], which means that the value will only grow.
pub trait SingletonBound {
    /// The [`Boundedness`] that this [`Singleton`] would be erased to.
    type UnderlyingBound: Boundedness + ApplyMonotoneStream<Proved, Self::StreamToMonotone>;

    /// The [`Boundedness`] of this [`Singleton`] if it is produced from a [`Stream`] with [`Self`] boundedness.
    type StreamToMonotone: SingletonBound<UnderlyingBound = Self::UnderlyingBound>;

    /// Returns the [`SingletonBoundKind`] corresponding to this type.
    fn bound_kind() -> SingletonBoundKind;
}

impl SingletonBound for Unbounded {
    type UnderlyingBound = Unbounded;

    type StreamToMonotone = Monotonic;

    fn bound_kind() -> SingletonBoundKind {
        SingletonBoundKind::Unbounded
    }
}

impl SingletonBound for Bounded {
    type UnderlyingBound = Bounded;

    type StreamToMonotone = Bounded;

    fn bound_kind() -> SingletonBoundKind {
        SingletonBoundKind::Bounded
    }
}

/// Marks that the [`Singleton`] is monotonic, which means that its value will only grow over time.
pub struct Monotonic;

impl SingletonBound for Monotonic {
    type UnderlyingBound = Unbounded;

    type StreamToMonotone = Monotonic;

    fn bound_kind() -> SingletonBoundKind {
        SingletonBoundKind::Monotonic
    }
}

#[sealed]
#[diagnostic::on_unimplemented(
    message = "The input singleton must be monotonic (`Monotonic`) or bounded (`Bounded`), but has bound `{Self}`. Strengthen the monotonicity upstream or consider a different API.",
    label = "required here",
    note = "To intentionally process a non-deterministic snapshot or batch, you may want to use a `sliced!` region. This introduces non-determinism so avoid unless necessary."
)]
/// Marker trait that is implemented for the [`Monotonic`] boundedness guarantee.
pub trait IsMonotonic: SingletonBound {}

#[sealed]
#[diagnostic::do_not_recommend]
impl IsMonotonic for Monotonic {}

#[sealed]
#[diagnostic::do_not_recommend]
impl<B: IsBounded> IsMonotonic for B {}

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
pub struct Singleton<Type, Loc, Bound: SingletonBound> {
    pub(crate) location: Loc,
    pub(crate) ir_node: RefCell<HydroNode>,
    pub(crate) flow_state: FlowState,

    _phantom: PhantomData<(Type, Loc, Bound)>,
}

impl<T, L, B: SingletonBound> Drop for Singleton<T, L, B> {
    fn drop(&mut self) {
        let ir_node = self.ir_node.replace(HydroNode::Placeholder);
        if !matches!(ir_node, HydroNode::Placeholder) && !ir_node.is_shared_with_others() {
            self.flow_state.borrow_mut().try_push_root(HydroRoot::Null {
                input: Box::new(ir_node),
                op_metadata: HydroIrOpMetadata::new(),
            });
        }
    }
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

    fn create_source_with_initial(cycle_id: CycleId, initial: Self, location: Tick<L>) -> Self {
        let from_previous_tick: Optional<T, Tick<L>, Bounded> = Optional::new(
            location.clone(),
            HydroNode::DeferTick {
                input: Box::new(HydroNode::CycleSource {
                    cycle_id,
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
    fn complete(self, cycle_id: CycleId, expected_location: LocationId) {
        assert_eq!(
            Location::id(&self.location),
            expected_location,
            "locations do not match"
        );
        self.location
            .flow_state()
            .borrow_mut()
            .push_root(HydroRoot::CycleSink {
                cycle_id,
                input: Box::new(self.ir_node.replace(HydroNode::Placeholder)),
                op_metadata: HydroIrOpMetadata::new(),
            });
    }
}

impl<'a, T, L> CycleCollection<'a, ForwardRef> for Singleton<T, Tick<L>, Bounded>
where
    L: Location<'a>,
{
    type Location = Tick<L>;

    fn create_source(cycle_id: CycleId, location: Tick<L>) -> Self {
        Singleton::new(
            location.clone(),
            HydroNode::CycleSource {
                cycle_id,
                metadata: location.new_node_metadata(Self::collection_kind()),
            },
        )
    }
}

impl<'a, T, L> ReceiverComplete<'a, ForwardRef> for Singleton<T, Tick<L>, Bounded>
where
    L: Location<'a>,
{
    fn complete(self, cycle_id: CycleId, expected_location: LocationId) {
        assert_eq!(
            Location::id(&self.location),
            expected_location,
            "locations do not match"
        );
        self.location
            .flow_state()
            .borrow_mut()
            .push_root(HydroRoot::CycleSink {
                cycle_id,
                input: Box::new(self.ir_node.replace(HydroNode::Placeholder)),
                op_metadata: HydroIrOpMetadata::new(),
            });
    }
}

impl<'a, T, L, B: SingletonBound> CycleCollection<'a, ForwardRef> for Singleton<T, L, B>
where
    L: Location<'a> + NoTick,
{
    type Location = L;

    fn create_source(cycle_id: CycleId, location: L) -> Self {
        Singleton::new(
            location.clone(),
            HydroNode::CycleSource {
                cycle_id,
                metadata: location.new_node_metadata(Self::collection_kind()),
            },
        )
    }
}

impl<'a, T, L, B: SingletonBound> ReceiverComplete<'a, ForwardRef> for Singleton<T, L, B>
where
    L: Location<'a> + NoTick,
{
    fn complete(self, cycle_id: CycleId, expected_location: LocationId) {
        assert_eq!(
            Location::id(&self.location),
            expected_location,
            "locations do not match"
        );
        self.location
            .flow_state()
            .borrow_mut()
            .push_root(HydroRoot::CycleSink {
                cycle_id,
                input: Box::new(self.ir_node.replace(HydroNode::Placeholder)),
                op_metadata: HydroIrOpMetadata::new(),
            });
    }
}

impl<'a, T, L, B: SingletonBound> Clone for Singleton<T, L, B>
where
    T: Clone,
    L: Location<'a>,
{
    fn clone(&self) -> Self {
        if !matches!(self.ir_node.borrow().deref(), HydroNode::Tee { .. }) {
            let orig_ir_node = self.ir_node.replace(HydroNode::Placeholder);
            *self.ir_node.borrow_mut() = HydroNode::Tee {
                inner: SharedNode(Rc::new(RefCell::new(orig_ir_node))),
                metadata: self.location.new_node_metadata(Self::collection_kind()),
            };
        }

        if let HydroNode::Tee { inner, metadata } = self.ir_node.borrow().deref() {
            Singleton {
                location: self.location.clone(),
                flow_state: self.flow_state.clone(),
                ir_node: HydroNode::Tee {
                    inner: SharedNode(inner.0.clone()),
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
fn zip_inside_tick<'a, T, L: Location<'a>, B: SingletonBound, O>(
    me: Singleton<T, Tick<L>, B>,
    other: Optional<O, Tick<L>, B::UnderlyingBound>,
) -> Optional<(T, O), Tick<L>, B::UnderlyingBound> {
    let me_as_optional: Optional<T, Tick<L>, B::UnderlyingBound> = me.into();
    super::optional::zip_inside_tick(me_as_optional, other)
}

impl<'a, T, L, B: SingletonBound> Singleton<T, L, B>
where
    L: Location<'a>,
{
    pub(crate) fn new(location: L, ir_node: HydroNode) -> Self {
        debug_assert_eq!(ir_node.metadata().location_id, Location::id(&location));
        debug_assert_eq!(ir_node.metadata().collection_kind, Self::collection_kind());
        let flow_state = location.flow_state().clone();
        Singleton {
            location,
            flow_state,
            ir_node: RefCell::new(ir_node),
            _phantom: PhantomData,
        }
    }

    pub(crate) fn collection_kind() -> CollectionKind {
        CollectionKind::Singleton {
            bound: B::bound_kind(),
            element_type: stageleft::quote_type::<T>().into(),
        }
    }

    /// Returns the [`Location`] where this singleton is being materialized.
    pub fn location(&self) -> &L {
        &self.location
    }

    /// Drops the monotonicity property of the [`Singleton`].
    pub fn ignore_monotonic(self) -> Singleton<T, L, B::UnderlyingBound> {
        if B::bound_kind() == B::UnderlyingBound::bound_kind() {
            Singleton::new(
                self.location.clone(),
                self.ir_node.replace(HydroNode::Placeholder),
            )
        } else {
            Singleton::new(
                self.location.clone(),
                HydroNode::Cast {
                    inner: Box::new(self.ir_node.replace(HydroNode::Placeholder)),
                    metadata:
                        self.location.new_node_metadata(
                            Singleton::<T, L, B::UnderlyingBound>::collection_kind(),
                        ),
                },
            )
        }
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
    pub fn map<U, F>(self, f: impl IntoQuotedMut<'a, F, L>) -> Singleton<U, L, B::UnderlyingBound>
    where
        F: Fn(T) -> U + 'a,
    {
        let f = f.splice_fn1_ctx(&self.location).into();
        Singleton::new(
            self.location.clone(),
            HydroNode::Map {
                f,
                input: Box::new(self.ir_node.replace(HydroNode::Placeholder)),
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
    ) -> Stream<U, L, Bounded, TotalOrder, ExactlyOnce>
    where
        B: IsBounded,
        I: IntoIterator<Item = U>,
        F: Fn(T) -> I + 'a,
    {
        self.into_stream().flat_map_ordered(f)
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
    ) -> Stream<U, L, Bounded, NoOrder, ExactlyOnce>
    where
        B: IsBounded,
        I: IntoIterator<Item = U>,
        F: Fn(T) -> I + 'a,
    {
        self.into_stream().flat_map_unordered(f)
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
    pub fn flatten_ordered<U>(self) -> Stream<U, L, Bounded, TotalOrder, ExactlyOnce>
    where
        B: IsBounded,
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
    pub fn flatten_unordered<U>(self) -> Stream<U, L, Bounded, NoOrder, ExactlyOnce>
    where
        B: IsBounded,
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
    pub fn filter<F>(self, f: impl IntoQuotedMut<'a, F, L>) -> Optional<T, L, B::UnderlyingBound>
    where
        F: Fn(&T) -> bool + 'a,
    {
        let f = f.splice_fn1_borrow_ctx(&self.location).into();
        Optional::new(
            self.location.clone(),
            HydroNode::Filter {
                f,
                input: Box::new(self.ir_node.replace(HydroNode::Placeholder)),
                metadata: self
                    .location
                    .new_node_metadata(Optional::<T, L, B::UnderlyingBound>::collection_kind()),
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
    pub fn filter_map<U, F>(
        self,
        f: impl IntoQuotedMut<'a, F, L>,
    ) -> Optional<U, L, B::UnderlyingBound>
    where
        F: Fn(T) -> Option<U> + 'a,
    {
        let f = f.splice_fn1_ctx(&self.location).into();
        Optional::new(
            self.location.clone(),
            HydroNode::FilterMap {
                f,
                input: Box::new(self.ir_node.replace(HydroNode::Placeholder)),
                metadata: self
                    .location
                    .new_node_metadata(Optional::<U, L, B::UnderlyingBound>::collection_kind()),
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
        B: IsBounded,
    {
        check_matching_location(&self.location, &Self::other_location(&other));

        if L::is_top_level()
            && let Some(tick) = self.location.try_tick()
        {
            let other_location = <Self as ZipResult<'a, O>>::other_location(&other);
            let out = zip_inside_tick(
                self.snapshot(&tick, nondet!(/** eventually stabilizes */)),
                Optional::<<Self as ZipResult<'a, O>>::OtherType, L, B>::new(
                    other_location.clone(),
                    HydroNode::Cast {
                        inner: Box::new(Self::other_ir_node(other)),
                        metadata: other_location.new_node_metadata(Optional::<
                            <Self as ZipResult<'a, O>>::OtherType,
                            Tick<L>,
                            Bounded,
                        >::collection_kind(
                        )),
                    },
                )
                .snapshot(&tick, nondet!(/** eventually stabilizes */)),
            )
            .latest();

            Self::make(
                out.location.clone(),
                out.ir_node.replace(HydroNode::Placeholder),
            )
        } else {
            Self::make(
                self.location.clone(),
                HydroNode::CrossSingleton {
                    left: Box::new(self.ir_node.replace(HydroNode::Placeholder)),
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
    /// boolean signal is `true`, otherwise the output is null.
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
    /// let signal = tick.optional_first_tick(q!(())).is_some(); // true on tick 1, false on tick 2
    /// let batch_first_tick = process
    ///   .source_iter(q!(vec![1]))
    ///   .batch(&tick, nondet!(/** test */));
    /// let batch_second_tick = process
    ///   .source_iter(q!(vec![1, 2, 3]))
    ///   .batch(&tick, nondet!(/** test */))
    ///   .defer_tick();
    /// batch_first_tick.chain(batch_second_tick).count()
    ///   .filter_if(signal)
    ///   .all_ticks()
    /// # }, |mut stream| async move {
    /// // [1]
    /// # for w in vec![1] {
    /// #     assert_eq!(stream.next().await.unwrap(), w);
    /// # }
    /// # }));
    /// # }
    /// ```
    pub fn filter_if(
        self,
        signal: Singleton<bool, L, B>,
    ) -> Optional<T, L, <B as SingletonBound>::UnderlyingBound>
    where
        B: IsBounded,
    {
        self.zip(signal.filter(q!(|b| *b))).map(q!(|(d, _)| d))
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
    #[deprecated(note = "use `filter_if` with `Optional::is_some()` instead")]
    pub fn filter_if_some<U>(
        self,
        signal: Optional<U, L, B>,
    ) -> Optional<T, L, <B as SingletonBound>::UnderlyingBound>
    where
        B: IsBounded,
    {
        self.filter_if(signal.is_some())
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
    #[deprecated(note = "use `filter_if` with `!Optional::is_some()` instead")]
    pub fn filter_if_none<U>(
        self,
        other: Optional<U, L, B>,
    ) -> Optional<T, L, <B as SingletonBound>::UnderlyingBound>
    where
        B: IsBounded,
    {
        self.filter_if(other.is_none())
    }

    /// Returns a [`Singleton`] containing `true` if this singleton's value equals the other's.
    ///
    /// # Example
    /// ```rust
    /// # #[cfg(feature = "deploy")] {
    /// # use hydro_lang::prelude::*;
    /// # use futures::StreamExt;
    /// # tokio_test::block_on(hydro_lang::test_util::stream_transform_test(|process| {
    /// let tick = process.tick();
    /// let a = tick.singleton(q!(5));
    /// let b = tick.singleton(q!(5));
    /// a.equals(b).all_ticks()
    /// # }, |mut stream| async move {
    /// // [true]
    /// # assert_eq!(stream.next().await.unwrap(), true);
    /// # }));
    /// # }
    /// ```
    pub fn equals(self, other: Singleton<T, L, B>) -> Singleton<bool, L, B>
    where
        T: PartialEq,
        B: IsBounded,
    {
        self.zip(other).map(q!(|(a, b)| a == b))
    }

    /// Returns a [`Stream`] that emits an event the first time the singleton has a value that is
    /// greater than or equal to the provided threshold. The event will have the value of the
    /// given threshold.
    ///
    /// This requires the incoming singleton to be monotonic, because otherwise the detection of
    /// the threshold would be non-deterministic.
    ///
    /// # Example
    /// ```rust
    /// # #[cfg(feature = "deploy")] {
    /// # use hydro_lang::prelude::*;
    /// # use futures::StreamExt;
    /// # tokio_test::block_on(hydro_lang::test_util::stream_transform_test(|process| {
    /// let a = // singleton 1 ~> 5 ~> 10
    /// # process.singleton(q!(5));
    /// let b = process.singleton(q!(4));
    /// a.threshold_greater_or_equal(b)
    /// # }, |mut stream| async move {
    /// // [4]
    /// # assert_eq!(stream.next().await.unwrap(), 4);
    /// # }));
    /// # }
    /// ```
    pub fn threshold_greater_or_equal<B2: IsBounded>(
        self,
        threshold: Singleton<T, L, B2>,
    ) -> Stream<T, L, B::UnderlyingBound>
    where
        T: Clone + PartialOrd,
        B: IsMonotonic,
    {
        let threshold = threshold.make_bounded();
        match self.try_make_bounded() {
            Ok(bounded) => {
                let uncasted = threshold
                    .zip(bounded)
                    .into_stream()
                    .filter_map(q!(|(t, m)| if m < t { None } else { Some(t) }));

                Stream::new(
                    uncasted.location.clone(),
                    uncasted.ir_node.replace(HydroNode::Placeholder),
                )
            }
            Err(me) => {
                let uncasted = sliced! {
                    let me = use(me, nondet!(/** thresholds are deterministic */));
                    let mut remaining_threshold = use::state(|l| {
                        let as_option: Optional<_, _, _> = threshold.clone_into_tick(l).into();
                        as_option
                    });

                    let (not_passed, passed) = remaining_threshold.zip(me).into_stream().partition(q!(|(t, m)| m < t));
                    remaining_threshold = not_passed.first().map(q!(|(t, _)| t));
                    passed.map(q!(|(t, _)| t))
                };

                Stream::new(
                    uncasted.location.clone(),
                    uncasted.ir_node.replace(HydroNode::Placeholder),
                )
            }
        }
    }

    /// An operator which allows you to "name" a `HydroNode`.
    /// This is only used for testing, to correlate certain `HydroNode`s with IDs.
    pub fn ir_node_named(self, name: &str) -> Singleton<T, L, B> {
        {
            let mut node = self.ir_node.borrow_mut();
            let metadata = node.metadata_mut();
            metadata.tag = Some(name.to_owned());
        }
        self
    }
}

impl<'a, L: Location<'a>, B: SingletonBound> Not for Singleton<bool, L, B> {
    type Output = Singleton<bool, L, B::UnderlyingBound>;

    fn not(self) -> Self::Output {
        self.map(q!(|b| !b))
    }
}

impl<'a, T, L, B: SingletonBound> Singleton<Option<T>, L, B>
where
    L: Location<'a>,
{
    /// Converts a `Singleton<Option<U>, L, B>` into an `Optional<U, L, B>` by unwrapping
    /// the inner `Option`.
    ///
    /// This is implemented as an identity [`Singleton::filter_map`], passing through the
    /// `Option<U>` directly. If the singleton's value is `Some(v)`, the resulting
    /// [`Optional`] contains `v`; if `None`, the [`Optional`] is empty.
    ///
    /// # Example
    /// ```rust
    /// # #[cfg(feature = "deploy")] {
    /// # use hydro_lang::prelude::*;
    /// # use futures::StreamExt;
    /// # tokio_test::block_on(hydro_lang::test_util::stream_transform_test(|process| {
    /// let tick = process.tick();
    /// let singleton = tick.singleton(q!(Some(42)));
    /// singleton.into_optional().all_ticks()
    /// # }, |mut stream| async move {
    /// // 42
    /// # assert_eq!(stream.next().await.unwrap(), 42);
    /// # }));
    /// # }
    /// ```
    pub fn into_optional(self) -> Optional<T, L, B::UnderlyingBound> {
        self.filter_map(q!(|v| v))
    }
}

impl<'a, L, B: SingletonBound> Singleton<bool, L, B>
where
    L: Location<'a>,
{
    /// Returns a [`Singleton`] containing the logical AND of this and another boolean singleton.
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
    /// let a = tick.optional_first_tick(q!(())).is_some(); // true, false
    /// let b = tick.singleton(q!(true)); // true, true
    /// a.and(b).all_ticks()
    /// # }, |mut stream| async move {
    /// // [true, false]
    /// # for w in vec![true, false] {
    /// #     assert_eq!(stream.next().await.unwrap(), w);
    /// # }
    /// # }));
    /// # }
    /// ```
    pub fn and(self, other: Singleton<bool, L, B>) -> Singleton<bool, L, Bounded>
    where
        B: IsBounded,
    {
        self.zip(other).map(q!(|(a, b)| a && b)).make_bounded()
    }

    /// Returns a [`Singleton`] containing the logical OR of this and another boolean singleton.
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
    /// let a = tick.optional_first_tick(q!(())).is_some(); // true, false
    /// let b = tick.singleton(q!(false)); // false, false
    /// a.or(b).all_ticks()
    /// # }, |mut stream| async move {
    /// // [true, false]
    /// # for w in vec![true, false] {
    /// #     assert_eq!(stream.next().await.unwrap(), w);
    /// # }
    /// # }));
    /// # }
    /// ```
    pub fn or(self, other: Singleton<bool, L, B>) -> Singleton<bool, L, Bounded>
    where
        B: IsBounded,
    {
        self.zip(other).map(q!(|(a, b)| a || b)).make_bounded()
    }
}

impl<'a, T, L, B: SingletonBound> Singleton<T, Atomic<L>, B>
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
    pub fn snapshot_atomic(
        self,
        tick: &Tick<L>,
        _nondet: NonDet,
    ) -> Singleton<T, Tick<L>, Bounded> {
        Singleton::new(
            tick.clone(),
            HydroNode::Batch {
                inner: Box::new(self.ir_node.replace(HydroNode::Placeholder)),
                metadata: tick
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
                inner: Box::new(self.ir_node.replace(HydroNode::Placeholder)),
                metadata: self
                    .location
                    .tick
                    .l
                    .new_node_metadata(Singleton::<T, L, B>::collection_kind()),
            },
        )
    }
}

impl<'a, T, L, B: SingletonBound> Singleton<T, L, B>
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
    pub fn atomic(self) -> Singleton<T, Atomic<L>, B> {
        let id = self.location.flow_state().borrow_mut().next_clock_id();
        let out_location = Atomic {
            tick: Tick {
                id,
                l: self.location.clone(),
            },
        };
        Singleton::new(
            out_location.clone(),
            HydroNode::BeginAtomic {
                inner: Box::new(self.ir_node.replace(HydroNode::Placeholder)),
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
                inner: Box::new(self.ir_node.replace(HydroNode::Placeholder)),
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
        .weaken_retries()
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

            snapshot.filter_if(sample_batch.first().is_some()).into_stream()
        }
        .weaken_retries()
    }

    /// Strengthens the boundedness guarantee to `Bounded`, given that `B: IsBounded`, which
    /// implies that `B == Bounded`.
    pub fn make_bounded(self) -> Singleton<T, L, Bounded>
    where
        B: IsBounded,
    {
        Singleton::new(
            self.location.clone(),
            self.ir_node.replace(HydroNode::Placeholder),
        )
    }

    #[expect(clippy::result_large_err, reason = "internal use only")]
    fn try_make_bounded(self) -> Result<Singleton<T, L, Bounded>, Singleton<T, L, B>> {
        if B::UnderlyingBound::BOUNDED {
            Ok(Singleton::new(
                self.location.clone(),
                self.ir_node.replace(HydroNode::Placeholder),
            ))
        } else {
            Err(self)
        }
    }

    /// Clones this bounded singleton into a tick, returning a singleton that has the
    /// same value as the outer singleton. Because the outer singleton is bounded, this
    /// is deterministic because there is only a single immutable version.
    pub fn clone_into_tick(self, tick: &Tick<L>) -> Singleton<T, Tick<L>, Bounded>
    where
        B: IsBounded,
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
    pub fn into_stream(self) -> Stream<T, L, Bounded, TotalOrder, ExactlyOnce>
    where
        B: IsBounded,
    {
        Stream::new(
            self.location.clone(),
            HydroNode::Cast {
                inner: Box::new(self.ir_node.replace(HydroNode::Placeholder)),
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

    /// Resolves the singleton's [`Future`] value by blocking until it completes,
    /// producing a singleton of the resolved output.
    ///
    /// This is useful when the singleton contains an async computation that must
    /// be awaited before further processing. The future is polled to completion
    /// before the output value is emitted.
    ///
    /// # Example
    /// ```rust
    /// # #[cfg(feature = "deploy")] {
    /// # use hydro_lang::prelude::*;
    /// # use futures::StreamExt;
    /// # tokio_test::block_on(hydro_lang::test_util::stream_transform_test(|process| {
    /// let tick = process.tick();
    /// let singleton = tick.singleton(q!(5));
    /// singleton
    ///     .map(q!(|v| async move { v * 2 }))
    ///     .resolve_future_blocking()
    ///     .all_ticks()
    /// # }, |mut stream| async move {
    /// // 10
    /// # assert_eq!(stream.next().await.unwrap(), 10);
    /// # }));
    /// # }
    /// ```
    pub fn resolve_future_blocking(
        self,
    ) -> Singleton<T::Output, L, <B as SingletonBound>::UnderlyingBound>
    where
        T: Future,
        B: IsBounded,
    {
        Singleton::new(
            self.location.clone(),
            HydroNode::ResolveFuturesBlocking {
                input: Box::new(self.ir_node.replace(HydroNode::Placeholder)),
                metadata: self
                    .location
                    .new_node_metadata(Singleton::<T::Output, L, B>::collection_kind()),
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
                inner: Box::new(self.ir_node.replace(HydroNode::Placeholder)),
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
                inner: Box::new(self.ir_node.replace(HydroNode::Placeholder)),
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
impl<'a, T, U, L, B: SingletonBound> ZipResult<'a, Singleton<U, L, B>> for Singleton<T, L, B>
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
        other.ir_node.replace(HydroNode::Placeholder)
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
impl<'a, T, U, L, B: SingletonBound> ZipResult<'a, Optional<U, L, B::UnderlyingBound>>
    for Singleton<T, L, B>
where
    L: Location<'a>,
{
    type Out = Optional<(T, U), L, B::UnderlyingBound>;
    type ElementType = (T, U);
    type OtherType = U;
    type Location = L;

    fn other_location(other: &Optional<U, L, B::UnderlyingBound>) -> L {
        other.location.clone()
    }

    fn other_ir_node(other: Optional<U, L, B::UnderlyingBound>) -> HydroNode {
        other.ir_node.replace(HydroNode::Placeholder)
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
            .filter_if(
                input
                    .batch(&node_tick, nondet!(/** testing */))
                    .first()
                    .is_some(),
            )
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

    #[cfg(feature = "sim")]
    #[test]
    fn inside_tick_singleton_zip() {
        use crate::live_collections::Stream;
        use crate::live_collections::sliced::sliced;

        let mut flow = FlowBuilder::new();
        let node = flow.process::<()>();

        let source_iter: Stream<_, _> = node.source_iter(q!(vec![1, 2])).into();
        let folded = source_iter.fold(q!(|| 0), q!(|a, b| *a += b));

        let out_recv = sliced! {
            let v = use(folded, nondet!(/** test */));
            v.clone().zip(v).into_stream()
        }
        .sim_output();

        let count = flow.sim().exhaustive(async || {
            let out = out_recv.collect::<Vec<_>>().await;
            assert_eq!(out.last(), Some(&(3, 3)));
        });

        assert_eq!(count, 4);
    }
}
