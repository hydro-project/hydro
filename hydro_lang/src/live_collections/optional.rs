//! Definitions for the [`Optional`] live collection.

use std::cell::RefCell;
use std::marker::PhantomData;
use std::ops::Deref;
use std::rc::Rc;

use sealed::sealed;
use stageleft::{IntoQuotedMut, QuotedWithContext, q};
use syn::parse_quote;

use super::OperatorContext;
use super::boundedness::{Bounded, Boundedness, IsBounded, Unbounded};
use super::singleton::Singleton;
use super::sliced::sliced;
use super::stream::{AtLeastOnce, ExactlyOnce, NoOrder, Stream, TotalOrder};
use crate::compile::builder::{CycleId, FlowState};
use crate::compile::ir::{
    CollectionKind, HydroIrOpMetadata, HydroNode, HydroRoot, OptionalBoundKind, SharedNode,
};
#[cfg(stageleft_runtime)]
use crate::forward_handle::{CycleCollection, CycleCollectionWithInitial, ReceiverComplete};
use crate::forward_handle::{ForwardRef, TickCycle};
use crate::live_collections::singleton::SingletonBound;
#[cfg(feature = "tokio")]
use crate::location::TopLevel;
#[cfg(stageleft_runtime)]
use crate::location::dynamic::{DynLocation, LocationId};
use crate::location::tick::{Atomic, DeferTick};
use crate::location::{Location, Tick, check_matching_location};
use crate::nondet::{NonDet, nondet};
use crate::prelude::KeyedSingleton;
use crate::properties::{StreamMapFuncAlgebra, ValidMutCommutativityFor, ValidMutIdempotenceFor};

/// A marker trait indicating the boundedness of an [`Optional`].
///
/// In addition to [`Bounded`] (immutable) and [`Unbounded`] (arbitrarily mutable, including
/// becoming null again), this also includes additional variants that constrain how the
/// optional's *presence* (whether it is null) and value evolve over time.
///
/// The currently defined variants form a hierarchy of increasing strength; variants other than
/// [`Bounded`] erase to [`Unbounded`]:
/// - [`Unbounded`]: the optional may become null and non-null arbitrarily, with an arbitrary
///   value whenever it is non-null.
/// - [`InitNone`]: the optional starts null, but once it becomes non-null it stays
///   non-null forever; the non-null value may still change arbitrarily.
pub trait OptionalBound {
    /// The [`Boundedness`] that this [`Optional`] would be erased to.
    type UnderlyingBound: Boundedness;

    /// The type of each update in this optional's top-level update feed (see
    /// [`CollectionKind::root_repr_is_tombstoned_update_feed`](crate::compile::ir::CollectionKind::root_repr_is_tombstoned_update_feed)):
    /// `Option<T>` (where `None` is a tombstone) for [`Unbounded`] optionals, which may become
    /// null after having a value, and plain `T` otherwise.
    type Update<T>;

    /// Returns the [`OptionalBoundKind`] corresponding to this type.
    fn bound_kind() -> OptionalBoundKind;
}

impl OptionalBound for Unbounded {
    type UnderlyingBound = Unbounded;
    type Update<T> = Option<T>;

    fn bound_kind() -> OptionalBoundKind {
        OptionalBoundKind::Unbounded
    }
}

impl OptionalBound for Bounded {
    type UnderlyingBound = Bounded;
    type Update<T> = T;

    fn bound_kind() -> OptionalBoundKind {
        OptionalBoundKind::Bounded
    }
}

/// Marks that the [`Optional`] is null only initially: once it becomes non-null it will remain
/// non-null forever, although the non-null value may still change arbitrarily over time.
///
/// This erases to [`Unbounded`], since the value (when present) is still asynchronously changing.
pub enum InitNone {}

impl OptionalBound for InitNone {
    type UnderlyingBound = Unbounded;
    type Update<T> = T;

    fn bound_kind() -> OptionalBoundKind {
        OptionalBoundKind::InitNone
    }
}

#[sealed]
#[diagnostic::on_unimplemented(
    message = "the optional must be null-only-initially (`InitNone`) or bounded (`Bounded`), but has bound `{Self}`",
    label = "required here",
    note = "strengthen the guarantee upstream, or consider a different API",
    note = "to intentionally process a non-deterministic snapshot or batch, you may want to use a `sliced!` region; this introduces non-determinism, so avoid it unless necessary"
)]
/// Marker trait that is implemented for [`OptionalBound`] types that are null only initially:
/// once the optional becomes non-null it remains non-null (or it is [`Bounded`]).
pub trait IsInitNone: OptionalBound {}

#[sealed]
#[diagnostic::do_not_recommend]
impl IsInitNone for InitNone {}

#[sealed]
#[diagnostic::do_not_recommend]
impl<B: IsBounded> IsInitNone for B {}

/// A *nullable* Rust value that can asynchronously change over time.
///
/// Optionals are the live collection equivalent of [`Option`]. If the optional is [`Bounded`],
/// the value is frozen and will not change. But if it is [`Unbounded`], the value will
/// asynchronously change over time, including becoming present of uninhabited.
///
/// Optionals are used in many of the same places as [`Singleton`], but when the value may be
/// nullable. For example, the first element of a [`Stream`] is exposed as an [`Optional`].
///
/// Type Parameters:
/// - `Type`: the type of the value in this optional (when it is not null)
/// - `Loc`: the [`Location`] where the optional is materialized
/// - `Bound`: tracks whether the value is [`Bounded`] (fixed) or [`Unbounded`] (changing
///   asynchronously) or something in between (see [`OptionalBoundKind`]).
pub struct Optional<Type, Loc, Bound: OptionalBound> {
    pub(crate) location: Loc,
    pub(crate) ir_node: Rc<RefCell<HydroNode>>,
    pub(crate) flow_state: FlowState,

    _phantom: PhantomData<(Type, Loc, Bound)>,
}

impl<T, L, B: OptionalBound> Drop for Optional<T, L, B> {
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

impl<'a, T, L> From<Optional<T, L, Bounded>> for Optional<T, L, Unbounded>
where
    T: Clone,
    L: Location<'a>,
{
    fn from(value: Optional<T, L, Bounded>) -> Self {
        let tick = value.location().tick();
        value.clone_into_tick(&tick).latest()
    }
}

impl<'a, T, L> From<Optional<T, L, InitNone>> for Optional<T, L, Unbounded>
where
    L: Location<'a>,
{
    fn from(value: Optional<T, L, InitNone>) -> Self {
        value.ignore_init_none()
    }
}

impl<'a, T, L> DeferTick for Optional<T, Tick<L>, Bounded>
where
    L: Location<'a>,
{
    fn defer_tick(self) -> Self {
        Optional::defer_tick(self)
    }
}

impl<'a, T, L> CycleCollection<'a, TickCycle> for Optional<T, Tick<L>, Bounded>
where
    L: Location<'a>,
{
    type Location = Tick<L>;

    fn create_source(cycle_id: CycleId, location: Tick<L>) -> Self {
        Optional::new(
            location.clone(),
            HydroNode::CycleSource {
                cycle_id,
                metadata: location.new_node_metadata(Self::collection_kind()),
            },
        )
    }
}

impl<'a, T, L> CycleCollectionWithInitial<'a, TickCycle> for Optional<T, Tick<L>, Bounded>
where
    L: Location<'a>,
{
    type Location = Tick<L>;

    fn location(&self) -> &Self::Location {
        self.location()
    }

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

        from_previous_tick.or(initial.filter_if(location.optional_first_tick(q!(())).is_some()))
    }
}

impl<'a, T, L> ReceiverComplete<'a, TickCycle> for Optional<T, Tick<L>, Bounded>
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

impl<'a, T, L, B: OptionalBound> CycleCollection<'a, ForwardRef> for Optional<T, L, B>
where
    L: Location<'a>,
{
    type Location = L;

    fn create_source(cycle_id: CycleId, location: L) -> Self {
        Optional::new(
            location.clone(),
            HydroNode::CycleSource {
                cycle_id,
                metadata: location.new_node_metadata(Self::collection_kind()),
            },
        )
    }
}

impl<'a, T, L, B: OptionalBound> ReceiverComplete<'a, ForwardRef> for Optional<T, L, B>
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

impl<'a, T, L, B: SingletonBound> From<Singleton<T, L, B>> for Optional<T, L, B::UnderlyingBound>
where
    L: Location<'a>,
{
    fn from(singleton: Singleton<T, L, B>) -> Self {
        Optional::new(
            singleton.location.clone(),
            HydroNode::Cast {
                inner: Box::new(singleton.ir_node.replace(HydroNode::Placeholder)),
                metadata: singleton
                    .location
                    .new_node_metadata(Self::collection_kind()),
            },
        )
    }
}

#[cfg(stageleft_runtime)]
pub(super) fn zip_inside_tick<'a, T, O, L: Location<'a>, B: Boundedness>(
    me: Optional<T, L, B>,
    other: Optional<O, L, B>,
) -> Optional<(T, O), L, B> {
    check_matching_location(&me.location, &other.location);

    Optional::new(
        me.location.clone(),
        HydroNode::CrossSingleton {
            left: Box::new(me.ir_node.replace(HydroNode::Placeholder)),
            right: Box::new(other.ir_node.replace(HydroNode::Placeholder)),
            metadata: me
                .location
                .new_node_metadata(Optional::<(T, O), L, B>::collection_kind()),
        },
    )
}

/// How [`Optional::or`] combines two [`Unbounded`] optionals located at `L`.
///
/// This is implemented for every type inside ticks and atomic regions. At root (process and
/// cluster) locations, the result holds the latest value of each side across updates, which
/// requires `T: Clone`.
#[sealed]
pub trait OrAt<'a, L: Location<'a>>: Sized {
    #[doc(hidden)]
    fn or_unbounded(
        me: Optional<Self, L, Unbounded>,
        other: Optional<Self, L, Unbounded>,
    ) -> Optional<Self, L, Unbounded>;
}

/// How [`Optional::zip`] (and [`Singleton::zip`]) combines two [`Bounded`] collections located
/// at `L`.
///
/// This is implemented for every pair of types inside ticks and atomic regions. At root (process
/// and cluster) locations, the result holds the latest value of each side across updates, which
/// requires `T: Clone` and `O: Clone`.
#[sealed]
pub trait ZipAt<'a, L: Location<'a>, O>: Sized {
    #[doc(hidden)]
    fn zip_bounded(
        me: Optional<Self, L, Bounded>,
        other: Optional<O, L, Bounded>,
    ) -> Optional<(Self, O), L, Bounded>;
}

#[sealed]
impl<'a, T, L: Location<'a>> OrAt<'a, Tick<L>> for T {
    fn or_unbounded(
        me: Optional<T, Tick<L>, Unbounded>,
        other: Optional<T, Tick<L>, Unbounded>,
    ) -> Optional<T, Tick<L>, Unbounded> {
        or_inside_tick(me, other)
    }
}

#[sealed]
impl<'a, T, L: Location<'a>> OrAt<'a, Atomic<L>> for T {
    fn or_unbounded(
        me: Optional<T, Atomic<L>, Unbounded>,
        other: Optional<T, Atomic<L>, Unbounded>,
    ) -> Optional<T, Atomic<L>, Unbounded> {
        // An atomic region is fused into its tick's loop: snapshot both sides into that tick.
        let self_location = me.location.clone();
        let tick = self_location.try_tick().unwrap();
        let out = or_inside_tick(
            me.snapshot(&tick, nondet!(/** eventually stabilizes */)),
            other.snapshot(&tick, nondet!(/** eventually stabilizes */)),
        )
        .latest();
        Optional::new(self_location, out.ir_node.replace(HydroNode::Placeholder))
    }
}

#[sealed]
impl<'a, T: Clone + 'a, P> OrAt<'a, crate::location::Process<'a, P>> for T {
    fn or_unbounded(
        me: Optional<T, crate::location::Process<'a, P>, Unbounded>,
        other: Optional<T, crate::location::Process<'a, P>, Unbounded>,
    ) -> Optional<T, crate::location::Process<'a, P>, Unbounded> {
        let location = me.location.clone();
        let out = or_at_root(me, other);
        Optional::new(location, out.ir_node.replace(HydroNode::Placeholder))
    }
}

#[sealed]
impl<'a, T: Clone + 'a, C, Con: crate::location::cluster::Consistency>
    OrAt<'a, crate::location::Cluster<'a, C, Con>> for T
{
    fn or_unbounded(
        me: Optional<T, crate::location::Cluster<'a, C, Con>, Unbounded>,
        other: Optional<T, crate::location::Cluster<'a, C, Con>, Unbounded>,
    ) -> Optional<T, crate::location::Cluster<'a, C, Con>, Unbounded> {
        let location = me.location.clone();
        let out = or_at_root(me, other);
        Optional::new(location, out.ir_node.replace(HydroNode::Placeholder))
    }
}

#[sealed]
impl<'a, T, O, L: Location<'a>> ZipAt<'a, Tick<L>, O> for T {
    fn zip_bounded(
        me: Optional<T, Tick<L>, Bounded>,
        other: Optional<O, Tick<L>, Bounded>,
    ) -> Optional<(T, O), Tick<L>, Bounded> {
        zip_inside_tick(me, other)
    }
}

#[sealed]
impl<'a, T, O, L: Location<'a>> ZipAt<'a, Atomic<L>, O> for T {
    fn zip_bounded(
        me: Optional<T, Atomic<L>, Bounded>,
        other: Optional<O, Atomic<L>, Bounded>,
    ) -> Optional<(T, O), Atomic<L>, Bounded> {
        // An atomic region is fused into its tick's loop: snapshot both sides into that tick.
        let self_location = me.location.clone();
        let tick = self_location.try_tick().unwrap();
        let out = zip_inside_tick(
            me.snapshot(&tick, nondet!(/** eventually stabilizes */)),
            other.snapshot(&tick, nondet!(/** eventually stabilizes */)),
        )
        .latest();
        cast_unbounded_zip_to_bounded(self_location, out)
    }
}

#[sealed]
impl<'a, T: Clone + 'a, O: Clone + 'a, P> ZipAt<'a, crate::location::Process<'a, P>, O> for T {
    fn zip_bounded(
        me: Optional<T, crate::location::Process<'a, P>, Bounded>,
        other: Optional<O, crate::location::Process<'a, P>, Bounded>,
    ) -> Optional<(T, O), crate::location::Process<'a, P>, Bounded> {
        let location = me.location.clone();
        let out = zip_at_root(me, other);
        cast_unbounded_zip_to_bounded(location, out)
    }
}

#[sealed]
impl<'a, T: Clone + 'a, O: Clone + 'a, C, Con: crate::location::cluster::Consistency>
    ZipAt<'a, crate::location::Cluster<'a, C, Con>, O> for T
{
    fn zip_bounded(
        me: Optional<T, crate::location::Cluster<'a, C, Con>, Bounded>,
        other: Optional<O, crate::location::Cluster<'a, C, Con>, Bounded>,
    ) -> Optional<(T, O), crate::location::Cluster<'a, C, Con>, Bounded> {
        let location = me.location.clone();
        let out = zip_at_root(me, other);
        cast_unbounded_zip_to_bounded(location, out)
    }
}

/// Casts the `Unbounded` result of a top-level zip back to the (bounded) output kind of `zip`.
/// The cast is not just metadata: at the root, unbounded optionals are tombstoned update feeds
/// while bounded ones are plain values.
#[cfg(stageleft_runtime)]
fn cast_unbounded_zip_to_bounded<'a, T, O, L: Location<'a>, L2: Location<'a>>(
    location: L,
    out: Optional<(T, O), L2, Unbounded>,
) -> Optional<(T, O), L, Bounded> {
    Optional::new(
        location.clone(),
        HydroNode::Cast {
            inner: Box::new(out.ir_node.replace(HydroNode::Placeholder)),
            metadata: location.new_node_metadata(Optional::<(T, O), L, Bounded>::collection_kind()),
        },
    )
}

/// [`Optional::zip`] of two bounded optionals for `T: Clone, O: Clone`, dispatching on the
/// location at build time instead of through [`ZipAt`]. Used by internal combinators whose
/// element types are always `Clone` (e.g. `bool`), so that they do not need a [`ZipAt`] bound for
/// a generic location.
#[cfg(stageleft_runtime)]
pub(crate) fn zip_cloned<'a, T: Clone + 'a, O: Clone + 'a, L: Location<'a>>(
    me: Optional<T, L, Bounded>,
    other: Optional<O, L, Bounded>,
) -> Optional<(T, O), L, Bounded> {
    check_matching_location(&me.location, &other.location);
    let location = me.location.clone();
    if !L::is_top_level() {
        zip_inside_tick(me, other)
    } else if location.id().is_root() {
        let out = zip_at_root(me, other);
        cast_unbounded_zip_to_bounded(location, out)
    } else {
        // An atomic region is fused into its tick's loop: snapshot both sides into that tick.
        let tick = location.try_tick().unwrap();
        let out = zip_inside_tick(
            me.snapshot(&tick, nondet!(/** eventually stabilizes */)),
            other.snapshot(&tick, nondet!(/** eventually stabilizes */)),
        )
        .latest();
        cast_unbounded_zip_to_bounded(location, out)
    }
}

/// [`OrAt`] at a root location: batch the update streams of both sides into a tick, hold the
/// latest value of each side across ticks, and emit the first non-null one.
#[cfg(stageleft_runtime)]
fn or_at_root<'a, T: Clone + 'a, L: Location<'a>>(
    me: Optional<T, L, Unbounded>,
    other: Optional<T, L, Unbounded>,
) -> Optional<T, L::DropConsistency, Unbounded> {
    let left_updates = me.into_update_stream(nondet!(
        /// Only the latest update of each side is held, so stuttering re-sets the same value and
        /// skipping only skips intermediate results.
    ));
    let right_updates = other.into_update_stream(nondet!(
        /// Only the latest update of each side is held, so stuttering re-sets the same value and
        /// skipping only skips intermediate results.
    ));
    sliced! {
        let left_batch = use::batch(left_updates, nondet!(
            /// Batching only affects which intermediate results are produced.
        ));
        let right_batch = use::batch(right_updates, nondet!(
            /// Batching only affects which intermediate results are produced.
        ));
        let mut latest_left = use::state(|l| l.singleton(q!(None)));
        let mut latest_right = use::state(|l| l.singleton(q!(None)));
        latest_left = left_batch.last().unwrap_or(latest_left);
        latest_right = right_batch.last().unwrap_or(latest_right);

        latest_left
            .clone()
            .zip(latest_right.clone())
            .map(q!(|(left, right)| left.or(right)))
    }
    // The tick yields a (non-null) `Singleton<Option<_>>`; convert it to an optional at the top
    // level, where a `None` result becomes a tombstone (and the result is null until the tick
    // first runs).
    .filter_map(q!(|result| result))
}

/// [`ZipAt`] at a root location: batch the update streams of both sides into a tick, hold the
/// latest value of each side across ticks, and emit the pair once both are present.
#[cfg(stageleft_runtime)]
fn zip_at_root<'a, T: Clone + 'a, O: Clone + 'a, L: Location<'a>>(
    me: Optional<T, L, Bounded>,
    other: Optional<O, L, Bounded>,
) -> Optional<(T, O), L::DropConsistency, Unbounded> {
    let left_updates = me.into_update_stream(nondet!(
        /// Only the latest value of each side is held, so stuttering re-sets the same value and
        /// skipping only skips intermediate pairs.
    ));
    let right_updates = other.into_update_stream(nondet!(
        /// Only the latest value of each side is held, so stuttering re-sets the same value and
        /// skipping only skips intermediate pairs.
    ));
    sliced! {
        let left_batch = use::batch(left_updates, nondet!(
            /// Batching only affects which intermediate pairs are produced.
        ));
        let right_batch = use::batch(right_updates, nondet!(
            /// Batching only affects which intermediate pairs are produced.
        ));
        let mut latest_left = use::state(|l| l.singleton(q!(None)));
        let mut latest_right = use::state(|l| l.singleton(q!(None)));
        latest_left = left_batch.last().map(q!(|v| Some(v))).unwrap_or(latest_left);
        latest_right = right_batch.last().map(q!(|v| Some(v))).unwrap_or(latest_right);

        latest_left
            .clone()
            .zip(latest_right.clone())
            .map(q!(|(left, right)| left.zip(right)))
    }
    // The tick yields a (non-null) `Singleton<Option<_>>`; convert it to an optional at the top
    // level, where a `None` result becomes a tombstone (and the result is null until the tick
    // first runs).
    .filter_map(q!(|result| result))
}

/// [`Optional::unwrap_or`] for a location that is fused into a tick's loop (an atomic region,
/// where presence is re-derived on every firing), without the [`OrAt`] dispatch: a per-firing
/// `ChainFirst`, cast to a [`Singleton`].
#[cfg(stageleft_runtime)]
pub(super) fn unwrap_or_in_atomic<'a, T, L: Location<'a>, B: OptionalBound, B2: SingletonBound>(
    me: Optional<T, L, B>,
    other: Singleton<T, L, B2>,
) -> Singleton<T, L, B2> {
    check_matching_location(&me.location, &other.location);
    let location = me.location.clone();
    Singleton::new(
        location.clone(),
        HydroNode::Cast {
            inner: Box::new(HydroNode::ChainFirst {
                first: Box::new(me.ir_node.replace(HydroNode::Placeholder)),
                second: Box::new(other.ir_node.replace(HydroNode::Placeholder)),
                metadata: location.new_node_metadata(Optional::<T, L, B>::collection_kind()),
            }),
            metadata: location.new_node_metadata(Singleton::<T, L, B2>::collection_kind()),
        },
    )
}

#[cfg(stageleft_runtime)]
fn or_inside_tick<'a, T, L: Location<'a>, B: Boundedness>(
    me: Optional<T, L, B>,
    other: Optional<T, L, B>,
) -> Optional<T, L, B> {
    check_matching_location(&me.location, &other.location);

    Optional::new(
        me.location.clone(),
        HydroNode::ChainFirst {
            first: Box::new(me.ir_node.replace(HydroNode::Placeholder)),
            second: Box::new(other.ir_node.replace(HydroNode::Placeholder)),
            metadata: me
                .location
                .new_node_metadata(Optional::<T, L, B>::collection_kind()),
        },
    )
}

impl<'a, T, L, B: OptionalBound> Clone for Optional<T, L, B>
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
            Optional {
                location: self.location.clone(),
                flow_state: self.flow_state.clone(),
                ir_node: super::tracked_ir_node(
                    &self.flow_state,
                    HydroNode::Tee {
                        inner: SharedNode(inner.0.clone()),
                        metadata: metadata.clone(),
                    },
                ),
                _phantom: PhantomData,
            }
        } else {
            unreachable!()
        }
    }
}

impl<'a, T, L, B: OptionalBound> Optional<T, L, B>
where
    L: Location<'a>,
{
    pub(crate) fn new(location: L, ir_node: HydroNode) -> Self {
        debug_assert_eq!(ir_node.metadata().location_id, Location::id(&location));
        debug_assert_eq!(ir_node.metadata().collection_kind, Self::collection_kind());
        let flow_state = location.flow_state().clone();
        let ir_node = super::tracked_ir_node(&flow_state, ir_node);
        Optional {
            location,
            flow_state,
            ir_node,
            _phantom: PhantomData,
        }
    }

    pub(crate) fn collection_kind() -> CollectionKind {
        CollectionKind::Optional {
            bound: <B as OptionalBound>::bound_kind(),
            element_type: stageleft::quote_type::<T>().into(),
        }
    }

    /// Exposes this collection's top-level update feed as a stream whose elements are exactly
    /// the feed's updates, in the order they were emitted.
    ///
    /// **Contract.** Relative to "every version of the collection, exactly once, in order", the
    /// stream may *stutter* (repeat the current version, consecutively) and *skip*
    /// (omit non-final versions); the final version is always eventually delivered. Which
    /// versions appear depends on timing, so the caller's `nondet` must justify that its use is
    /// invariant to stuttering and skipping (e.g. an idempotent "latest value" fold, or a
    /// monotone threshold). The simulator emits every version exactly once.
    ///
    /// The collection must be located at a root (process or cluster) location.
    ///
    /// Each update is a [`OptionalBound::Update`]: `Option<T>` (with `None` a tombstone) for an
    /// [`Unbounded`] optional, and plain `T` otherwise.
    pub(crate) fn into_update_stream(
        self,
        _nondet: NonDet,
    ) -> Stream<B::Update<T>, L::DropConsistency, Unbounded, TotalOrder, ExactlyOnce> {
        assert!(
            self.location.id().is_root(),
            "`into_update_stream` requires a root location, got {:?}",
            self.location.id()
        );
        let target_location = self.location.drop_consistency();
        Stream::new(
            target_location.clone(),
            HydroNode::CastUpdateStream {
                inner: Box::new(self.ir_node.replace(HydroNode::Placeholder)),
                metadata: target_location.new_node_metadata(Stream::<
                    B::Update<T>,
                    L::DropConsistency,
                    Unbounded,
                    TotalOrder,
                    ExactlyOnce,
                >::collection_kind()),
            },
        )
    }

    /// Returns the [`Location`] where this optional is being materialized.
    pub fn location(&self) -> &L {
        &self.location
    }

    /// Creates a shared reference handle to this optional that can be captured inside `q!()`
    /// closures. The handle resolves to `&Option<T>` at runtime.
    ///
    /// The optional must be bounded, otherwise reading it would be non-deterministic.
    /// The handle can only be captured in closures passed to operators on collections at
    /// the same location with **matching boundedness**; capturing it in a closure over an
    /// unbounded collection is rejected at compile time.
    pub fn by_ref(&self) -> crate::handoff_ref::OptionalRef<'a, '_, T, L, B>
    where
        B: IsBounded,
    {
        crate::handoff_ref::OptionalRef::new(&self.ir_node)
    }

    /// Returns a mutable reference handle to this optional that can be captured inside `q!()`
    /// closures. The handle resolves to `&mut Option<T>` at runtime.
    pub fn by_mut(&self) -> crate::handoff_ref::OptionalMut<'a, '_, T, L, B>
    where
        B: IsBounded,
    {
        crate::handoff_ref::OptionalMut::new(&self.ir_node)
    }

    /// Weakens the consistency of this live collection to not guarantee any consistency across
    /// cluster members (if this collection is on a cluster).
    pub fn weaken_consistency(self) -> Optional<T, L::DropConsistency, B>
    where
        L: Location<'a>,
    {
        if L::consistency()
            .is_none_or(|c| c == crate::location::dynamic::ClusterConsistency::NoConsistency)
        {
            // already no consistency
            Optional::new(
                self.location.drop_consistency(),
                self.ir_node.replace(HydroNode::Placeholder),
            )
        } else {
            Optional::new(
                self.location.drop_consistency(),
                HydroNode::Cast {
                    inner: Box::new(self.ir_node.replace(HydroNode::Placeholder)),
                    metadata: self
                        .location
                        .clone()
                        .drop_consistency()
                        .new_node_metadata(Optional::<T, L::DropConsistency, B>::collection_kind()),
                },
            )
        }
    }

    /// Casts this live collection to have the consistency guarantees specified in the given
    /// location type parameter. The developer must ensure that the strengthened consistency
    /// is actually guaranteed, via the proof field (see [`crate::prelude::manual_proof`]).
    pub fn assert_has_consistency_of<L2: Location<'a, DropConsistency = L::DropConsistency>>(
        self,
        _proof: impl crate::properties::ConsistencyProof,
    ) -> Optional<T, L2, B>
    where
        L: Location<'a>,
    {
        if L::consistency() == L2::consistency() {
            Optional::new(
                self.location.with_consistency_of(),
                self.ir_node.replace(HydroNode::Placeholder),
            )
        } else {
            Optional::new(
                self.location.with_consistency_of(),
                HydroNode::AssertIsConsistent {
                    inner: Box::new(self.ir_node.replace(HydroNode::Placeholder)),
                    trusted: false,
                    metadata: self
                        .location
                        .clone()
                        .with_consistency_of::<L2>()
                        .new_node_metadata(Optional::<T, L2, B>::collection_kind()),
                },
            )
        }
    }

    /// Transforms the optional value by applying a function `f` to it,
    /// continuously as the input is updated.
    ///
    /// Whenever the optional is empty, the output optional is also empty.
    ///
    /// # Example
    /// ```rust
    /// # #[cfg(feature = "deploy")] {
    /// # use hydro_lang::prelude::*;
    /// # use futures::StreamExt;
    /// # tokio_test::block_on(hydro_lang::test_util::stream_transform_test(|process| {
    /// let tick = process.tick();
    /// let optional = tick.optional_first_tick(q!(1));
    /// optional.map(q!(|v| v + 1)).all_ticks()
    /// # }, |mut stream| async move {
    /// // 2
    /// # assert_eq!(stream.next().await.unwrap(), 2);
    /// # }));
    /// # }
    /// ```
    pub fn map<U, F>(
        self,
        f: impl IntoQuotedMut<'a, F, OperatorContext<L, B::UnderlyingBound>>,
    ) -> Optional<U, L, B>
    where
        F: Fn(T) -> U + 'a,
    {
        let f = f
            .splice_fn1_ctx(&OperatorContext::<L, B::UnderlyingBound>::new(
                &self.location,
            ))
            .into();
        Optional::new(
            self.location.clone(),
            HydroNode::Map {
                f,
                input: Box::new(self.ir_node.replace(HydroNode::Placeholder)),
                metadata: self
                    .location
                    .new_node_metadata(Optional::<U, L, B>::collection_kind()),
            },
        )
    }

    /// Transforms the optional value by applying a function `f` to it and then flattening
    /// the result into a stream, preserving the order of elements.
    ///
    /// If the optional is empty, the output stream is also empty. If the optional contains
    /// a value, `f` is applied to produce an iterator, and all items from that iterator
    /// are emitted in the output stream in deterministic order.
    ///
    /// The implementation of [`Iterator`] for the output type `I` must produce items in a
    /// **deterministic** order. For example, `I` could be a `Vec`, but not a `HashSet`.
    /// If the order is not deterministic, use [`Optional::flat_map_unordered`] instead.
    ///
    /// # Example
    /// ```rust
    /// # #[cfg(feature = "deploy")] {
    /// # use hydro_lang::prelude::*;
    /// # use futures::StreamExt;
    /// # tokio_test::block_on(hydro_lang::test_util::stream_transform_test(|process| {
    /// let tick = process.tick();
    /// let optional = tick.optional_first_tick(q!(vec![1, 2, 3]));
    /// optional.flat_map_ordered(q!(|v| v)).all_ticks()
    /// # }, |mut stream| async move {
    /// // 1, 2, 3
    /// # for w in vec![1, 2, 3] {
    /// #     assert_eq!(stream.next().await.unwrap(), w);
    /// # }
    /// # }));
    /// # }
    /// ```
    pub fn flat_map_ordered<U, I, F, C, Idemp, const WAS_MUT: bool>(
        self,
        f: impl IntoQuotedMut<
            'a,
            F,
            OperatorContext<L, Bounded>,
            StreamMapFuncAlgebra<T, Bounded, C, Idemp, L::SimHookScope>,
        >,
    ) -> Stream<U, L, Bounded, TotalOrder, ExactlyOnce>
    where
        B: IsBounded,
        I: IntoIterator<Item = U>,
        F: FnMut(T) -> I + 'a,
        C: ValidMutCommutativityFor<F, T, I, TotalOrder, WAS_MUT>,
        Idemp: ValidMutIdempotenceFor<F, T, I, ExactlyOnce, WAS_MUT>,
    {
        self.into_stream().flat_map_ordered(f)
    }

    /// Like [`Optional::flat_map_ordered`], but allows the implementation of [`Iterator`]
    /// for the output type `I` to produce items in any order.
    ///
    /// If the optional is empty, the output stream is also empty. If the optional contains
    /// a value, `f` is applied to produce an iterator, and all items from that iterator
    /// are emitted in the output stream in non-deterministic order.
    ///
    /// # Example
    /// ```rust
    /// # #[cfg(feature = "deploy")] {
    /// # use hydro_lang::{prelude::*, live_collections::stream::{NoOrder, ExactlyOnce}};
    /// # use futures::StreamExt;
    /// # tokio_test::block_on(hydro_lang::test_util::stream_transform_test::<_, _, _, NoOrder, ExactlyOnce>(|process| {
    /// let tick = process.tick();
    /// let optional = tick.optional_first_tick(q!(
    ///     std::collections::HashSet::<i32>::from_iter(vec![1, 2, 3])
    /// ));
    /// optional.flat_map_unordered(q!(|v| v)).all_ticks()
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
    pub fn flat_map_unordered<U, I, F, C, Idemp, const WAS_MUT: bool>(
        self,
        f: impl IntoQuotedMut<
            'a,
            F,
            OperatorContext<L, Bounded>,
            StreamMapFuncAlgebra<T, Bounded, C, Idemp, L::SimHookScope>,
        >,
    ) -> Stream<U, L, Bounded, NoOrder, ExactlyOnce>
    where
        B: IsBounded,
        I: IntoIterator<Item = U>,
        F: FnMut(T) -> I + 'a,
        C: ValidMutCommutativityFor<F, T, I, TotalOrder, WAS_MUT>,
        Idemp: ValidMutIdempotenceFor<F, T, I, ExactlyOnce, WAS_MUT>,
    {
        self.into_stream().flat_map_unordered(f)
    }

    /// Flattens the optional value into a stream, preserving the order of elements.
    ///
    /// If the optional is empty, the output stream is also empty. If the optional contains
    /// a value that implements [`IntoIterator`], all items from that iterator are emitted
    /// in the output stream in deterministic order.
    ///
    /// The implementation of [`Iterator`] for the element type `T` must produce items in a
    /// **deterministic** order. For example, `T` could be a `Vec`, but not a `HashSet`.
    /// If the order is not deterministic, use [`Optional::flatten_unordered`] instead.
    ///
    /// # Example
    /// ```rust
    /// # #[cfg(feature = "deploy")] {
    /// # use hydro_lang::prelude::*;
    /// # use futures::StreamExt;
    /// # tokio_test::block_on(hydro_lang::test_util::stream_transform_test(|process| {
    /// let tick = process.tick();
    /// let optional = tick.optional_first_tick(q!(vec![1, 2, 3]));
    /// optional.flatten_ordered().all_ticks()
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
        self.flat_map_ordered(q!(|v| v))
    }

    /// Like [`Optional::flatten_ordered`], but allows the implementation of [`Iterator`]
    /// for the element type `T` to produce items in any order.
    ///
    /// If the optional is empty, the output stream is also empty. If the optional contains
    /// a value that implements [`IntoIterator`], all items from that iterator are emitted
    /// in the output stream in non-deterministic order.
    ///
    /// # Example
    /// ```rust
    /// # #[cfg(feature = "deploy")] {
    /// # use hydro_lang::{prelude::*, live_collections::stream::{NoOrder, ExactlyOnce}};
    /// # use futures::StreamExt;
    /// # tokio_test::block_on(hydro_lang::test_util::stream_transform_test::<_, _, _, NoOrder, ExactlyOnce>(|process| {
    /// let tick = process.tick();
    /// let optional = tick.optional_first_tick(q!(
    ///     std::collections::HashSet::<i32>::from_iter(vec![1, 2, 3])
    /// ));
    /// optional.flatten_unordered().all_ticks()
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
        self.flat_map_unordered(q!(|v| v))
    }

    /// Creates an optional containing only the value if it satisfies a predicate `f`.
    ///
    /// If the optional is empty, the output optional is also empty. If the optional contains
    /// a value and the predicate returns `true`, the output optional contains the same value.
    /// If the predicate returns `false`, the output optional is empty.
    ///
    /// The closure `f` receives a reference `&T` rather than an owned value `T` because filtering does
    /// not modify or take ownership of the value. If you need to modify the value while filtering
    /// use [`Optional::filter_map`] instead.
    ///
    /// # Example
    /// ```rust
    /// # #[cfg(feature = "deploy")] {
    /// # use hydro_lang::prelude::*;
    /// # use futures::StreamExt;
    /// # tokio_test::block_on(hydro_lang::test_util::stream_transform_test(|process| {
    /// let tick = process.tick();
    /// let optional = tick.optional_first_tick(q!(5));
    /// optional.filter(q!(|&x| x > 3)).all_ticks()
    /// # }, |mut stream| async move {
    /// // 5
    /// # assert_eq!(stream.next().await.unwrap(), 5);
    /// # }));
    /// # }
    /// ```
    pub fn filter<F>(
        self,
        f: impl IntoQuotedMut<'a, F, OperatorContext<L, B::UnderlyingBound>>,
    ) -> Optional<T, L, B::UnderlyingBound>
    where
        F: Fn(&T) -> bool + 'a,
    {
        let f = f
            .splice_fn1_borrow_ctx(&OperatorContext::<L, B::UnderlyingBound>::new(
                &self.location,
            ))
            .into();
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

    /// An operator that both filters and maps. It yields only the value if the supplied
    /// closure `f` returns `Some(value)`.
    ///
    /// If the optional is empty, the output optional is also empty. If the optional contains
    /// a value and the closure returns `Some(new_value)`, the output optional contains `new_value`.
    /// If the closure returns `None`, the output optional is empty.
    ///
    /// # Example
    /// ```rust
    /// # #[cfg(feature = "deploy")] {
    /// # use hydro_lang::prelude::*;
    /// # use futures::StreamExt;
    /// # tokio_test::block_on(hydro_lang::test_util::stream_transform_test(|process| {
    /// let tick = process.tick();
    /// let optional = tick.optional_first_tick(q!("42"));
    /// optional
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
        f: impl IntoQuotedMut<'a, F, OperatorContext<L, B::UnderlyingBound>>,
    ) -> Optional<U, L, B::UnderlyingBound>
    where
        F: Fn(T) -> Option<U> + 'a,
    {
        let f = f
            .splice_fn1_ctx(&OperatorContext::<L, B::UnderlyingBound>::new(
                &self.location,
            ))
            .into();
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
    /// If the other value is a [`Optional`], the output will be non-null only if the argument is
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
    ///   .source_iter(q!(vec![123, 456, 789]))
    ///   .batch(&tick, nondet!(/** test */));
    /// let min = numbers.clone().min(); // Optional
    /// let max = numbers.max(); // Optional
    /// min.zip(max).all_ticks()
    /// # }, |mut stream| async move {
    /// // [(123, 789)]
    /// # for w in vec![(123, 789)] {
    /// #     assert_eq!(stream.next().await.unwrap(), w);
    /// # }
    /// # }));
    /// # }
    /// ```
    pub fn zip<O>(self, other: impl Into<Optional<O, L, B>>) -> Optional<(T, O), L, B>
    where
        B: IsBounded,
        T: ZipAt<'a, L, O>,
    {
        let other: Optional<O, L, B> = other.into();
        check_matching_location(&self.location, &other.location);

        let location = self.location.clone();
        let me: Optional<T, L, Bounded> =
            Optional::new(location.clone(), self.ir_node.replace(HydroNode::Placeholder));
        let other: Optional<O, L, Bounded> = Optional::new(
            other.location.clone(),
            other.ir_node.replace(HydroNode::Placeholder),
        );
        let out = T::zip_bounded(me, other);
        Optional::new(location, out.ir_node.replace(HydroNode::Placeholder))
    }

    /// Passes through `self` when it has a value, otherwise passes through `other`.
    ///
    /// Like [`Option::or`], this is helpful for defining a fallback for an [`Optional`], when the
    /// fallback itself is an [`Optional`]. If the fallback is a [`Singleton`], you can use
    /// [`Optional::unwrap_or`] to ensure that the output is always non-null.
    ///
    /// If the inputs are [`Unbounded`], the output will be asynchronously updated as the contents
    /// of the inputs change (including to/from null states).
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
    /// let some_first_tick = tick.optional_first_tick(q!(123));
    /// let some_second_tick = tick.optional_first_tick(q!(456)).defer_tick();
    /// some_first_tick.or(some_second_tick).all_ticks()
    /// # }, |mut stream| async move {
    /// // [123 /* first tick */, 456 /* second tick */]
    /// # for w in vec![123, 456] {
    /// #     assert_eq!(stream.next().await.unwrap(), w);
    /// # }
    /// # }));
    /// # }
    /// ```
    pub fn or(
        self,
        other: Optional<T, L, B::UnderlyingBound>,
    ) -> Optional<T, L, B::UnderlyingBound>
    where
        T: OrAt<'a, L>,
    {
        let me = self.ignore_init_none();
        check_matching_location(&me.location, &other.location);

        if <B::UnderlyingBound as Boundedness>::BOUNDED {
            Optional::new(
                me.location.clone(),
                HydroNode::ChainFirst {
                    first: Box::new(me.ir_node.replace(HydroNode::Placeholder)),
                    second: Box::new(other.ir_node.replace(HydroNode::Placeholder)),
                    metadata: me
                        .location
                        .new_node_metadata(Optional::<T, L, B::UnderlyingBound>::collection_kind()),
                },
            )
        } else {
            let location = me.location.clone();
            let me: Optional<T, L, Unbounded> =
                Optional::new(location.clone(), me.ir_node.replace(HydroNode::Placeholder));
            let other: Optional<T, L, Unbounded> = Optional::new(
                other.location.clone(),
                other.ir_node.replace(HydroNode::Placeholder),
            );
            let out = T::or_unbounded(me, other);
            Optional::new(location, out.ir_node.replace(HydroNode::Placeholder))
        }
    }

    /// [`Optional::or`] for a `T: Clone`, dispatching on the location at build time instead of
    /// through [`OrAt`]. Used by internal combinators whose element type is always `Clone`
    /// (e.g. `()`), so that they do not need an [`OrAt`] bound for a generic location.
    pub(crate) fn or_cloned(
        self,
        other: Optional<T, L, B::UnderlyingBound>,
    ) -> Optional<T, L, B::UnderlyingBound>
    where
        T: Clone + 'a,
    {
        let me = self.ignore_init_none();
        check_matching_location(&me.location, &other.location);

        if <B::UnderlyingBound as Boundedness>::BOUNDED || !L::is_top_level() {
            Optional::new(
                me.location.clone(),
                HydroNode::ChainFirst {
                    first: Box::new(me.ir_node.replace(HydroNode::Placeholder)),
                    second: Box::new(other.ir_node.replace(HydroNode::Placeholder)),
                    metadata: me
                        .location
                        .new_node_metadata(Optional::<T, L, B::UnderlyingBound>::collection_kind()),
                },
            )
        } else {
            let location = me.location.clone();
            let me: Optional<T, L, Unbounded> =
                Optional::new(location.clone(), me.ir_node.replace(HydroNode::Placeholder));
            let other: Optional<T, L, Unbounded> = Optional::new(
                other.location.clone(),
                other.ir_node.replace(HydroNode::Placeholder),
            );
            let out = if location.id().is_root() {
                let out = or_at_root(me, other);
                Optional::new(location.clone(), out.ir_node.replace(HydroNode::Placeholder))
            } else {
                // An atomic region is fused into its tick's loop: snapshot both sides into
                // that tick.
                let tick = location.try_tick().unwrap();
                let out = or_inside_tick(
                    me.snapshot(&tick, nondet!(/** eventually stabilizes */)),
                    other.snapshot(&tick, nondet!(/** eventually stabilizes */)),
                )
                .latest();
                Optional::<T, L, Unbounded>::new(
                    location.clone(),
                    out.ir_node.replace(HydroNode::Placeholder),
                )
            };
            Optional::new(location, out.ir_node.replace(HydroNode::Placeholder))
        }
    }

    /// [`Optional::into_singleton`] for a `T: Clone`, via [`Optional::or_cloned`].
    pub(crate) fn into_singleton_cloned(self) -> Singleton<Option<T>, L, B::UnderlyingBound>
    where
        T: Clone + 'a,
    {
        if self.location.id().is_root() && !<B::UnderlyingBound as Boundedness>::BOUNDED {
            // At the root, the singleton's versions are exactly the optional's updates (as
            // `Option<T>`, with a tombstone becoming `None`), starting from `None`: hold the
            // latest update.
            let location = self.location.clone();
            let ir_node = self.ir_node.replace(HydroNode::Placeholder);
            let updates: Stream<Option<T>, L::DropConsistency, Unbounded, TotalOrder, ExactlyOnce> =
                if <B as OptionalBound>::bound_kind() == OptionalBoundKind::Unbounded {
                    let me: Optional<T, L, Unbounded> = Optional::new(location.clone(), ir_node);
                    me.into_update_stream(nondet!(
                        /// The singleton holds the latest update, so stuttering re-sets the same
                        /// value and skipping only skips intermediate versions.
                    ))
                } else {
                    let me: Optional<T, L, InitNone> = Optional::new(location.clone(), ir_node);
                    me.into_update_stream(nondet!(
                        /// The singleton holds the latest update, so stuttering re-sets the same
                        /// value and skipping only skips intermediate versions.
                    ))
                    .map(q!(|v| Some(v)))
                };
            let singleton: Singleton<Option<T>, _, Unbounded> =
                updates.fold(q!(|| None), q!(|current, update| *current = update));
            return Singleton::new(location, singleton.ir_node.replace(HydroNode::Placeholder));
        }

        let none: syn::Expr = parse_quote!(::std::option::Option::None);
        let none_singleton: Singleton<Option<T>, L, B::UnderlyingBound> = Singleton::new(
            self.location.clone(),
            HydroNode::SingletonSource {
                value: none.into(),
                first_tick_only: false,
                metadata: self.location.new_node_metadata(Singleton::<
                    Option<T>,
                    L,
                    B::UnderlyingBound,
                >::collection_kind()),
            },
        );
        let res_option = self.map(q!(|v| Some(v))).or_cloned(none_singleton.into());
        Singleton::new(
            res_option.location.clone(),
            HydroNode::Cast {
                inner: Box::new(res_option.ir_node.replace(HydroNode::Placeholder)),
                metadata: res_option.location.new_node_metadata(Singleton::<
                    Option<T>,
                    L,
                    B::UnderlyingBound,
                >::collection_kind()),
            },
        )
    }

    /// Gets the contents of `self` when it has a value, otherwise passes through `other`.
    ///
    /// Like [`Option::unwrap_or`], this is helpful for defining a fallback for an [`Optional`].
    /// If the fallback is not always defined (an [`Optional`]), you can use [`Optional::or`].
    ///
    /// If the inputs are [`Unbounded`], the output will be asynchronously updated as the contents
    /// of the inputs change (including to/from null states).
    ///
    /// # Example
    /// ```rust
    /// # #[cfg(feature = "deploy")] {
    /// # use hydro_lang::prelude::*;
    /// # use futures::StreamExt;
    /// # tokio_test::block_on(hydro_lang::test_util::stream_transform_test(|process| {
    /// let tick = process.tick();
    /// // ticks are lazy by default, forces the later ticks to run
    /// tick.spin_batch(q!(1)).all_ticks().for_each(q!(|_| {}));
    ///
    /// let some_first_tick = tick.optional_first_tick(q!(123));
    /// some_first_tick
    ///     .unwrap_or(tick.singleton(q!(456)))
    ///     .all_ticks()
    /// # }, |mut stream| async move {
    /// // [123 /* first tick */, 456 /* second tick */, 456 /* third tick */, 456, ...]
    /// # for w in vec![123, 456, 456, 456] {
    /// #     assert_eq!(stream.next().await.unwrap(), w);
    /// # }
    /// # }));
    /// # }
    /// ```
    pub fn unwrap_or(
        self,
        other: Singleton<T, L, B::UnderlyingBound>,
    ) -> Singleton<T, L, B::UnderlyingBound>
    where
        T: OrAt<'a, L>,
    {
        let res_option = self.or(other.into());
        Singleton::new(
            res_option.location.clone(),
            HydroNode::Cast {
                inner: Box::new(res_option.ir_node.replace(HydroNode::Placeholder)),
                metadata: res_option.location.new_node_metadata(Singleton::<
                    T,
                    L,
                    B::UnderlyingBound,
                >::collection_kind(
                )),
            },
        )
    }

    /// Gets the contents of `self` when it has a value, otherwise returns the default value of `T`.
    ///
    /// Like [`Option::unwrap_or_default`], this is helpful for defining a fallback for an
    /// [`Optional`] when the default value of the type is a suitable fallback.
    ///
    /// # Example
    /// ```rust
    /// # #[cfg(feature = "deploy")] {
    /// # use hydro_lang::prelude::*;
    /// # use futures::StreamExt;
    /// # tokio_test::block_on(hydro_lang::test_util::stream_transform_test(|process| {
    /// let tick = process.tick();
    /// // ticks are lazy by default, forces the later ticks to run
    /// tick.spin_batch(q!(1)).all_ticks().for_each(q!(|_| {}));
    ///
    /// let some_first_tick = tick.optional_first_tick(q!(123i32));
    /// some_first_tick.unwrap_or_default().all_ticks()
    /// # }, |mut stream| async move {
    /// // [123 /* first tick */, 0 /* second tick */, 0 /* third tick */, 0, ...]
    /// # for w in vec![123, 0, 0, 0] {
    /// #     assert_eq!(stream.next().await.unwrap(), w);
    /// # }
    /// # }));
    /// # }
    /// ```
    pub fn unwrap_or_default(self) -> Singleton<T, L, B::UnderlyingBound>
    where
        T: Default + Clone + 'a,
    {
        self.into_singleton().map(q!(|v| v.unwrap_or_default()))
    }

    /// Converts this optional into a [`Singleton`] with a Rust [`Option`] as its contents.
    ///
    /// Useful for writing custom Rust code that needs to interact with both the null and non-null
    /// states of the [`Optional`]. When possible, you should use the native APIs on [`Optional`]
    /// so that Hydro can skip any computation on null values.
    ///
    /// # Example
    /// ```rust
    /// # #[cfg(feature = "deploy")] {
    /// # use hydro_lang::prelude::*;
    /// # use futures::StreamExt;
    /// # tokio_test::block_on(hydro_lang::test_util::stream_transform_test(|process| {
    /// let tick = process.tick();
    /// // ticks are lazy by default, forces the later ticks to run
    /// tick.spin_batch(q!(1)).all_ticks().for_each(q!(|_| {}));
    ///
    /// let some_first_tick = tick.optional_first_tick(q!(123));
    /// some_first_tick.into_singleton().all_ticks()
    /// # }, |mut stream| async move {
    /// // [Some(123) /* first tick */, None /* second tick */, None /* third tick */, None, ...]
    /// # for w in vec![Some(123), None, None, None] {
    /// #     assert_eq!(stream.next().await.unwrap(), w);
    /// # }
    /// # }));
    /// # }
    /// ```
    pub fn into_singleton(self) -> Singleton<Option<T>, L, B::UnderlyingBound>
    where
        T: Clone + 'a,
    {
        self.into_singleton_cloned()
    }

    /// Returns a [`Singleton`] containing `true` if this optional has a value, `false` otherwise.
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
    /// let some_first_tick = tick.optional_first_tick(q!(42));
    /// some_first_tick.is_some().all_ticks()
    /// # }, |mut stream| async move {
    /// // [true /* first tick */, false /* second tick */, ...]
    /// # for w in vec![true, false] {
    /// #     assert_eq!(stream.next().await.unwrap(), w);
    /// # }
    /// # }));
    /// # }
    /// ```
    #[expect(clippy::wrong_self_convention, reason = "Stream naming")]
    pub fn is_some(self) -> Singleton<bool, L, B::UnderlyingBound> {
        self.map(q!(|_| ()))
            .into_singleton_cloned()
            .map(q!(|o| o.is_some()))
    }

    /// Returns a [`Singleton`] containing `true` if this optional is null, `false` otherwise.
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
    /// let some_first_tick = tick.optional_first_tick(q!(42));
    /// some_first_tick.is_none().all_ticks()
    /// # }, |mut stream| async move {
    /// // [false /* first tick */, true /* second tick */, ...]
    /// # for w in vec![false, true] {
    /// #     assert_eq!(stream.next().await.unwrap(), w);
    /// # }
    /// # }));
    /// # }
    /// ```
    #[expect(clippy::wrong_self_convention, reason = "Stream naming")]
    pub fn is_none(self) -> Singleton<bool, L, B::UnderlyingBound> {
        self.map(q!(|_| ()))
            .into_singleton_cloned()
            .map(q!(|o| o.is_none()))
    }

    /// Returns a [`Singleton`] containing `true` if both optionals are non-null and their
    /// values are equal, `false` otherwise (including when either is null).
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
    /// let a = tick.optional_first_tick(q!(5)); // Some(5), None
    /// let b = tick.optional_first_tick(q!(5)); // Some(5), None
    /// a.is_some_and_equals(b).all_ticks()
    /// # }, |mut stream| async move {
    /// // [true, false]
    /// # for w in vec![true, false] {
    /// #     assert_eq!(stream.next().await.unwrap(), w);
    /// # }
    /// # }));
    /// # }
    /// ```
    #[expect(clippy::wrong_self_convention, reason = "Stream naming")]
    pub fn is_some_and_equals(self, other: Optional<T, L, B>) -> Singleton<bool, L, B>
    where
        T: PartialEq + Clone + 'a,
        B: IsBounded,
        Option<T>: ZipAt<'a, L, Option<T>>,
    {
        self.into_singleton()
            .zip(other.into_singleton())
            .map(q!(|(a, b)| a.is_some() && a == b))
    }

    /// An operator which allows you to "name" a `HydroNode`.
    /// This is only used for testing, to correlate certain `HydroNode`s with IDs.
    pub fn ir_node_named(self, name: &str) -> Optional<T, L, B> {
        {
            let mut node = self.ir_node.borrow_mut();
            let metadata = node.metadata_mut();
            metadata.tag = Some(name.to_owned());
        }
        self
    }

    /// Drops the [`InitNone`] guarantee of the [`Optional`], erasing it to its
    /// [`OptionalBound::UnderlyingBound`] (i.e. [`Unbounded`] for [`InitNone`]).
    pub fn ignore_init_none(self) -> Optional<T, L, B::UnderlyingBound> {
        if <B as OptionalBound>::bound_kind() == <B::UnderlyingBound as OptionalBound>::bound_kind()
        {
            Optional::new(
                self.location.clone(),
                self.ir_node.replace(HydroNode::Placeholder),
            )
        } else {
            Optional::new(
                self.location.clone(),
                HydroNode::Cast {
                    inner: Box::new(self.ir_node.replace(HydroNode::Placeholder)),
                    metadata: self
                        .location
                        .new_node_metadata(Optional::<T, L, B::UnderlyingBound>::collection_kind()),
                },
            )
        }
    }

    /// Strengthens the boundedness guarantee to `Bounded`, given that `B: IsBounded`, which
    /// implies that `B == Bounded`.
    pub fn make_bounded(self) -> Optional<T, L, Bounded>
    where
        B: IsBounded,
    {
        Optional::new(
            self.location.clone(),
            self.ir_node.replace(HydroNode::Placeholder),
        )
    }

    /// Clones this bounded optional into a tick, returning a optional that has the
    /// same value as the outer optional. Because the outer optional is bounded, this
    /// is deterministic because there is only a single immutable version.
    pub fn clone_into_tick(self, tick: &Tick<L>) -> Optional<T, Tick<L>, Bounded>
    where
        B: IsBounded,
        T: Clone,
    {
        // TODO(shadaj): avoid printing simulator logs for this snapshot
        let inner = self.snapshot(
            tick,
            nondet!(/** bounded top-level optional so deterministic */),
        );
        Optional::new(tick.clone(), inner.ir_node.replace(HydroNode::Placeholder))
    }

    /// Converts this optional into a [`Stream`] containing a single element, the value, if it is
    /// non-null. Otherwise, the stream is empty.
    ///
    /// # Example
    /// ```rust
    /// # #[cfg(feature = "deploy")] {
    /// # use hydro_lang::prelude::*;
    /// # use futures::StreamExt;
    /// # tokio_test::block_on(hydro_lang::test_util::stream_transform_test(|process| {
    /// # let tick = process.tick();
    /// # // ticks are lazy by default, forces the second tick to run
    /// # tick.spin_batch(q!(1)).all_ticks().for_each(q!(|_| {}));
    /// # let batch_first_tick = process
    /// #   .source_iter(q!(vec![]))
    /// #   .batch(&tick, nondet!(/** test */));
    /// # let batch_second_tick = process
    /// #   .source_iter(q!(vec![123, 456]))
    /// #   .batch(&tick, nondet!(/** test */))
    /// #   .defer_tick(); // appears on the second tick
    /// # let input_batch = batch_first_tick.chain(batch_second_tick);
    /// input_batch // first tick: [], second tick: [123, 456]
    ///     .clone()
    ///     .max()
    ///     .into_stream()
    ///     .chain(input_batch)
    ///     .all_ticks()
    /// # }, |mut stream| async move {
    /// // [456, 123, 456]
    /// # for w in vec![456, 123, 456] {
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

    /// Filters this optional, passing through the value if the boolean signal is `true`,
    /// otherwise the output is null.
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
    /// let some_first_tick = tick.optional_first_tick(q!(()));
    /// let signal = some_first_tick.is_some(); // true on first tick, false on second
    /// let batch_first_tick = process
    ///   .source_iter(q!(vec![456]))
    ///   .batch(&tick, nondet!(/** test */));
    /// let batch_second_tick = process
    ///   .source_iter(q!(vec![789]))
    ///   .batch(&tick, nondet!(/** test */))
    ///   .defer_tick();
    /// batch_first_tick.chain(batch_second_tick).first()
    ///   .filter_if(signal)
    ///   .unwrap_or(tick.singleton(q!(0)))
    ///   .all_ticks()
    /// # }, |mut stream| async move {
    /// // [456, 0]
    /// # for w in vec![456, 0] {
    /// #     assert_eq!(stream.next().await.unwrap(), w);
    /// # }
    /// # }));
    /// # }
    /// ```
    pub fn filter_if(self, signal: Singleton<bool, L, B>) -> Optional<T, L, B>
    where
        B: IsBounded,
        T: ZipAt<'a, L, bool>,
    {
        self.zip(signal.filter(q!(|b| *b))).map(q!(|(d, _)| d))
    }

    /// Filters this optional, passing through the optional value if it is non-null **and** the
    /// argument (a [`Bounded`] [`Optional`]) is non-null, otherwise the output is null.
    ///
    /// Useful for conditionally processing, such as only emitting an optional's value outside
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
    ///   .source_iter(q!(vec![]))
    ///   .batch(&tick, nondet!(/** test */));
    /// let batch_second_tick = process
    ///   .source_iter(q!(vec![456]))
    ///   .batch(&tick, nondet!(/** test */))
    ///   .defer_tick(); // appears on the second tick
    /// let some_on_first_tick = tick.optional_first_tick(q!(()));
    /// batch_first_tick.chain(batch_second_tick).first()
    ///   .filter_if_some(some_on_first_tick)
    ///   .unwrap_or(tick.singleton(q!(789)))
    ///   .all_ticks()
    /// # }, |mut stream| async move {
    /// // [789, 789]
    /// # for w in vec![789, 789] {
    /// #     assert_eq!(stream.next().await.unwrap(), w);
    /// # }
    /// # }));
    /// # }
    /// ```
    #[deprecated(note = "use `filter_if` with `Optional::is_some()` instead")]
    pub fn filter_if_some<U>(self, signal: Optional<U, L, B>) -> Optional<T, L, B>
    where
        B: IsBounded,
        T: ZipAt<'a, L, bool>,
    {
        self.filter_if(signal.is_some())
    }

    /// Filters this optional, passing through the optional value if it is non-null **and** the
    /// argument (a [`Bounded`] [`Optional`]) is _null_, otherwise the output is null.
    ///
    /// Useful for conditionally processing, such as only emitting an optional's value outside
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
    ///   .source_iter(q!(vec![]))
    ///   .batch(&tick, nondet!(/** test */));
    /// let batch_second_tick = process
    ///   .source_iter(q!(vec![456]))
    ///   .batch(&tick, nondet!(/** test */))
    ///   .defer_tick(); // appears on the second tick
    /// let some_on_first_tick = tick.optional_first_tick(q!(()));
    /// batch_first_tick.chain(batch_second_tick).first()
    ///   .filter_if_none(some_on_first_tick)
    ///   .unwrap_or(tick.singleton(q!(789)))
    ///   .all_ticks()
    /// # }, |mut stream| async move {
    /// // [789, 789]
    /// # for w in vec![789, 456] {
    /// #     assert_eq!(stream.next().await.unwrap(), w);
    /// # }
    /// # }));
    /// # }
    /// ```
    #[deprecated(note = "use `filter_if` with `!Optional::is_some()` instead")]
    pub fn filter_if_none<U>(self, other: Optional<U, L, B>) -> Optional<T, L, B>
    where
        B: IsBounded,
        T: ZipAt<'a, L, bool>,
    {
        self.filter_if(other.is_none())
    }

    /// If `self` is null, emits a null optional, but if it non-null, emits `value`.
    ///
    /// Useful for gating the release of a [`Singleton`] on a condition of the [`Optional`]
    /// having a value, such as only releasing a piece of state if the node is the leader.
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
    /// let some_on_first_tick = tick.optional_first_tick(q!(()));
    /// some_on_first_tick
    ///     .if_some_then(tick.singleton(q!(456)))
    ///     .unwrap_or(tick.singleton(q!(123)))
    /// # .all_ticks()
    /// # }, |mut stream| async move {
    /// // 456 (first tick) ~> 123 (second tick onwards)
    /// # for w in vec![456, 123, 123] {
    /// #     assert_eq!(stream.next().await.unwrap(), w);
    /// # }
    /// # }));
    /// # }
    /// ```
    #[deprecated(note = "use `filter_if` with `Optional::is_some()` instead")]
    pub fn if_some_then<U>(self, value: Singleton<U, L, B>) -> Optional<U, L, B>
    where
        B: IsBounded,
        U: ZipAt<'a, L, bool>,
    {
        value.filter_if(self.is_some())
    }
}

impl<'a, K, V, L, B: OptionalBound> Optional<(K, V), L, B>
where
    L: Location<'a>,
{
    /// Converts this optional into a [`KeyedSingleton`] containing a single entry with the
    /// key-value pair of this [`Optional`].
    ///
    /// If this [`Optional`] is [`Bounded`], the [`KeyedSingleton`] will be [`Bounded`] as well
    /// if it is [`Unbounded`], the [`KeyedSingleton`] will be [`Unbounded`], which means that
    /// the entry will be updated and appear / disappear according to the state of the
    /// [`Optional`].
    ///
    /// The `K: Clone + PartialEq` bounds are needed because when the optional is [`Unbounded`]
    /// at a top-level location, the compiled representation tracks the previously emitted key
    /// so the entry can be removed when the optional becomes null or moves to a different key.
    pub fn into_keyed_singleton(self) -> KeyedSingleton<K, V, L, B::UnderlyingBound>
    where
        K: Clone + PartialEq,
    {
        KeyedSingleton::new(
            self.location.clone(),
            HydroNode::Cast {
                inner: Box::new(self.ir_node.replace(HydroNode::Placeholder)),
                metadata: self.location.new_node_metadata(KeyedSingleton::<
                    K,
                    V,
                    L,
                    B::UnderlyingBound,
                >::collection_kind()),
            },
        )
    }
}

impl<'a, T, L, B: OptionalBound> Optional<T, Atomic<L>, B>
where
    L: Location<'a>,
{
    /// Returns an optional value corresponding to the latest snapshot of the optional
    /// being atomically processed. The snapshot at tick `t + 1` is guaranteed to include
    /// at least all relevant data that contributed to the snapshot at tick `t`. Furthermore,
    /// all snapshots of this optional into the atomic-associated tick will observe the
    /// same value each tick.
    ///
    /// # Non-Determinism
    /// Because this picks a snapshot of a optional whose value is continuously changing,
    /// the output optional has a non-deterministic value since the snapshot can be at an
    /// arbitrary point in time.
    pub fn snapshot_atomic<L2: Location<'a, DropConsistency = L::DropConsistency>>(
        self,
        tick: &Tick<L2>,
        _nondet: NonDet,
    ) -> Optional<T, Tick<L::DropConsistency>, Bounded> {
        assert_eq!(
            Location::id(tick.parent_location()),
            Location::id(self.location.tick.parent_location())
        );
        Optional::new(
            tick.drop_consistency(),
            HydroNode::Snapshot {
                inner: Box::new(self.ir_node.replace(HydroNode::Placeholder)),
                metadata: tick
                    .new_node_metadata(Optional::<T, Tick<L>, Bounded>::collection_kind()),
            },
        )
    }
}

impl<'a, T, L, B: OptionalBound> Optional<T, L, B>
where
    L: Location<'a>,
{
    /// Given a tick, returns a optional value corresponding to a snapshot of the optional
    /// as of that tick. The snapshot at tick `t + 1` is guaranteed to include at least all
    /// relevant data that contributed to the snapshot at tick `t`.
    ///
    /// # Non-Determinism
    /// Because this picks a snapshot of a optional whose value is continuously changing,
    /// the output optional has a non-deterministic value since the snapshot can be at an
    /// arbitrary point in time.
    pub fn snapshot<L2: Location<'a, DropConsistency = L::DropConsistency>>(
        self,
        tick: &Tick<L2>,
        _nondet: NonDet,
    ) -> Optional<T, Tick<L::DropConsistency>, Bounded> {
        assert_eq!(
            Location::id(tick.parent_location()),
            Location::id(&self.location)
        );
        Optional::new(
            tick.drop_consistency(),
            HydroNode::Snapshot {
                inner: Box::new(self.ir_node.replace(HydroNode::Placeholder)),
                metadata: tick
                    .new_node_metadata(Optional::<T, Tick<L>, Bounded>::collection_kind()),
            },
        )
    }

    /// Eagerly samples the optional as fast as possible, returning a stream of snapshots
    /// with order corresponding to increasing prefixes of data contributing to the optional.
    ///
    /// This requires the optional to be null-only-initially ([`IsInitNone`]): a general
    /// [`Unbounded`] optional can *become null again*, which a `Stream<T>` of samples cannot
    /// represent (dropping the null transitions would leave stale values observable). To
    /// sample an [`Unbounded`] optional, use [`Optional::into_singleton`] first to observe
    /// `Option<T>` snapshots (including `None`).
    ///
    /// # Non-Determinism
    /// At runtime, the optional will be arbitrarily sampled as fast as possible, but due
    /// to non-deterministic batching and arrival of inputs, the output stream is
    /// non-deterministic.
    pub fn sample_eager(
        self,
        nondet: NonDet,
    ) -> Stream<T, L::DropConsistency, Unbounded, TotalOrder, AtLeastOnce>
    where
        B: IsInitNone,
    {
        if self.location.id().is_root() {
            let me: Optional<T, L, InitNone> = Optional::new(
                self.location.clone(),
                self.ir_node.replace(HydroNode::Placeholder),
            );
            let updates = me.into_update_stream(nondet);
            // Sample the latest version in each batch of updates. Batching (explored by the
            // simulator) determines which versions are skipped.
            sliced! {
                let batch = use::batch(updates, nondet!(
                    /// which versions are sampled is captured by the caller's guard
                ));
                batch.last().into_stream()
            }
            .weaken_retries()
        } else {
            let tick = self.location.tick();
            self.snapshot(&tick, nondet).all_ticks().weaken_retries()
        }
    }

    /// Given a time interval, returns a stream corresponding to snapshots of the optional
    /// value taken at various points in time. Because the input optional may be
    /// [`Unbounded`], there are no guarantees on what these snapshots are other than they
    /// represent the value of the optional given some prefix of the streams leading up to
    /// it.
    ///
    /// # Non-Determinism
    /// The output stream is non-deterministic in which elements are sampled, since this
    /// is controlled by a clock.
    #[cfg(feature = "tokio")]
    pub fn sample_every(
        self,
        interval: impl QuotedWithContext<'a, std::time::Duration, L> + Copy + 'a,
        nondet: NonDet,
    ) -> Stream<T, L::DropConsistency, Unbounded, TotalOrder, AtLeastOnce>
    where
        L: TopLevel<'a>,
    {
        let samples = self.location.source_interval(interval);
        let tick = self.location.tick();

        self.snapshot(&tick, nondet)
            .filter_if(
                samples
                    .batch(
                        &tick,
                        nondet!(
                            /// sample timing is captured by the caller's guard
                            nondet
                        ),
                    )
                    .first()
                    .is_some(),
            )
            .all_ticks()
            .weaken_retries()
    }
}

impl<'a, T, L> Optional<T, Tick<L>, Bounded>
where
    L: Location<'a>,
{
    /// Asynchronously yields the value of this singleton outside the tick as an unbounded stream,
    /// which will stream the value computed in _each_ tick as a separate stream element (skipping
    /// null values).
    ///
    /// Unlike [`Optional::latest`], the value computed in each tick is emitted separately,
    /// producing one element in the output for each (non-null) tick. This is useful for batched
    /// computations, where the results from each tick must be combined together.
    ///
    /// # Example
    /// ```rust
    /// # #[cfg(feature = "deploy")] {
    /// # use hydro_lang::prelude::*;
    /// # use futures::StreamExt;
    /// # tokio_test::block_on(hydro_lang::test_util::stream_transform_test(|process| {
    /// # let tick = process.tick();
    /// # // ticks are lazy by default, forces the second tick to run
    /// # tick.spin_batch(q!(1)).all_ticks().for_each(q!(|_| {}));
    /// # let batch_first_tick = process
    /// #   .source_iter(q!(vec![]))
    /// #   .batch(&tick, nondet!(/** test */));
    /// # let batch_second_tick = process
    /// #   .source_iter(q!(vec![1, 2, 3]))
    /// #   .batch(&tick, nondet!(/** test */))
    /// #   .defer_tick(); // appears on the second tick
    /// # let input_batch = batch_first_tick.chain(batch_second_tick);
    /// input_batch // first tick: [], second tick: [1, 2, 3]
    ///     .max()
    ///     .all_ticks()
    /// # }, |mut stream| async move {
    /// // [3]
    /// # for w in vec![3] {
    /// #     assert_eq!(stream.next().await.unwrap(), w);
    /// # }
    /// # }));
    /// # }
    /// ```
    pub fn all_ticks(self) -> Stream<T, L, Unbounded, TotalOrder, ExactlyOnce> {
        self.into_stream().all_ticks()
    }

    /// Synchronously yields the value of this optional outside the tick as an unbounded stream,
    /// which will stream the value computed in _each_ tick as a separate stream element.
    ///
    /// Unlike [`Optional::all_ticks`], this preserves synchronous execution, as the output stream
    /// is emitted in an [`Atomic`] context that will process elements synchronously with the input
    /// optional's [`Tick`] context.
    pub fn all_ticks_atomic(self) -> Stream<T, Atomic<L>, Unbounded, TotalOrder, ExactlyOnce> {
        self.into_stream().all_ticks_atomic()
    }

    /// Asynchronously yields this optional outside the tick as an unbounded optional, which will
    /// be asynchronously updated with the latest value of the optional inside the tick, including
    /// whether the optional is null or not.
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
    /// # let tick = process.tick();
    /// # // ticks are lazy by default, forces the second tick to run
    /// # tick.spin_batch(q!(1)).all_ticks().for_each(q!(|_| {}));
    /// # let batch_first_tick = process
    /// #   .source_iter(q!(vec![]))
    /// #   .batch(&tick, nondet!(/** test */));
    /// # let batch_second_tick = process
    /// #   .source_iter(q!(vec![1, 2, 3]))
    /// #   .batch(&tick, nondet!(/** test */))
    /// #   .defer_tick(); // appears on the second tick
    /// # let input_batch = batch_first_tick.chain(batch_second_tick);
    /// input_batch // first tick: [], second tick: [1, 2, 3]
    ///     .max()
    ///     .latest()
    /// # .into_singleton()
    /// # .sample_eager(nondet!(/** test */))
    /// # }, |mut stream| async move {
    /// // asynchronously changes from None ~> 3
    /// # for w in vec![None, Some(3)] {
    /// #     assert_eq!(stream.next().await.unwrap(), w);
    /// # }
    /// # }));
    /// # }
    /// ```
    pub fn latest(self) -> Optional<T, L, Unbounded> {
        Optional::new(
            self.location.parent_location().clone(),
            HydroNode::YieldConcat {
                inner: Box::new(self.ir_node.replace(HydroNode::Placeholder)),
                metadata: self
                    .location
                    .parent_location()
                    .new_node_metadata(Optional::<T, L, Unbounded>::collection_kind()),
            },
        )
    }

    /// Synchronously yields this optional outside the tick as an unbounded optional, which will
    /// be updated with the latest value of the optional inside the tick.
    ///
    /// Unlike [`Optional::latest`], this preserves synchronous execution, as the output optional
    /// is emitted in an [`Atomic`] context that will process elements synchronously with the input
    /// optional's [`Tick`] context.
    pub fn latest_atomic(self) -> Optional<T, Atomic<L>, Unbounded> {
        let out_location = Atomic {
            tick: self.location.clone(),
        };

        Optional::new(
            out_location.clone(),
            HydroNode::YieldConcat {
                inner: Box::new(self.ir_node.replace(HydroNode::Placeholder)),
                metadata: out_location
                    .new_node_metadata(Optional::<T, Atomic<L>, Unbounded>::collection_kind()),
            },
        )
    }

    /// Shifts the state in `self` to the **next tick**, so that the returned optional at tick `T`
    /// always has the state of `self` at tick `T - 1`.
    ///
    /// At tick `0`, the output optional is null, since there is no previous tick.
    ///
    /// This operator enables stateful iterative processing with ticks, by sending data from one
    /// tick to the next. For example, you can use it to compare state across consecutive batches.
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
    ///   .source_iter(q!(vec![1, 2]))
    ///   .batch(&tick, nondet!(/** test */));
    /// let batch_second_tick = process
    ///   .source_iter(q!(vec![3, 4]))
    ///   .batch(&tick, nondet!(/** test */))
    ///   .defer_tick(); // appears on the second tick
    /// let current_tick_sum = batch_first_tick.chain(batch_second_tick)
    ///   .reduce(q!(|state, v| *state += v));
    ///
    /// current_tick_sum.clone().into_singleton().zip(
    ///   current_tick_sum.defer_tick().into_singleton() // state from previous tick
    /// ).all_ticks()
    /// # }, |mut stream| async move {
    /// // [(Some(3), None) /* first tick */, (Some(7), Some(3)) /* second tick */]
    /// # for w in vec![(Some(3), None), (Some(7), Some(3))] {
    /// #     assert_eq!(stream.next().await.unwrap(), w);
    /// # }
    /// # }));
    /// # }
    /// ```
    pub fn defer_tick(self) -> Optional<T, Tick<L>, Bounded> {
        Optional::new(
            self.location.clone(),
            HydroNode::DeferTick {
                input: Box::new(self.ir_node.replace(HydroNode::Placeholder)),
                metadata: self.location.new_node_metadata(Self::collection_kind()),
            },
        )
    }
}

#[cfg(test)]
mod tests {
    #[cfg(feature = "deploy")]
    use futures::StreamExt;
    #[cfg(feature = "deploy")]
    use hydro_deploy::Deployment;
    #[cfg(any(feature = "deploy", feature = "sim"))]
    use stageleft::q;

    #[cfg(any(feature = "deploy", feature = "sim"))]
    use super::Optional;
    #[cfg(any(feature = "deploy", feature = "sim"))]
    use crate::compile::builder::FlowBuilder;
    #[cfg(any(feature = "deploy", feature = "sim"))]
    use crate::location::Location;
    #[cfg(feature = "deploy")]
    use crate::nondet::nondet;

    #[cfg(feature = "deploy")]
    #[tokio::test]
    async fn optional_or_cardinality() {
        let mut deployment = Deployment::new();

        let mut flow = FlowBuilder::new();
        let node = flow.process::<()>();
        let external = flow.external::<()>();

        let node_tick = node.tick();
        // Tick-located constants are lazy and never fire the tick on their own, so a spinning
        // batch drives `node_tick`.
        node_tick.spin_batch(q!(1)).all_ticks().for_each(q!(|_| {}));
        let tick_singleton = node_tick.singleton(q!(123));
        let tick_optional_inhabited: Optional<_, _, _> = tick_singleton.into();
        let counts = tick_optional_inhabited
            .clone()
            .or(tick_optional_inhabited)
            .into_stream()
            .count()
            .all_ticks()
            .send_bincode_external(&external);

        let nodes = flow
            .with_process(&node, deployment.Localhost())
            .with_external(&external, deployment.Localhost())
            .deploy(&mut deployment);

        deployment.deploy().await.unwrap();

        let mut external_out = nodes.connect(counts).await;

        deployment.start().await.unwrap();

        assert_eq!(external_out.next().await.unwrap(), 1);
    }

    #[cfg(feature = "deploy")]
    #[tokio::test]
    async fn into_singleton_top_level_none_cardinality() {
        let mut deployment = Deployment::new();

        let mut flow = FlowBuilder::new();
        let node = flow.process::<()>();
        let external = flow.external::<()>();

        let node_tick = node.tick();
        let top_level_none = node.singleton(q!(123)).filter(q!(|_| false));
        let into_singleton = top_level_none.into_singleton();

        let tick_driver = node.spin();

        let counts = into_singleton
            .snapshot(&node_tick, nondet!(/** test */))
            .into_stream()
            .count()
            .zip(tick_driver.batch(&node_tick, nondet!(/** test */)).count())
            .map(q!(|(c, _)| c))
            .all_ticks()
            .send_bincode_external(&external);

        let nodes = flow
            .with_process(&node, deployment.Localhost())
            .with_external(&external, deployment.Localhost())
            .deploy(&mut deployment);

        deployment.deploy().await.unwrap();

        let mut external_out = nodes.connect(counts).await;

        deployment.start().await.unwrap();

        assert_eq!(external_out.next().await.unwrap(), 1);
        assert_eq!(external_out.next().await.unwrap(), 1);
        assert_eq!(external_out.next().await.unwrap(), 1);
    }

    #[cfg(feature = "deploy")]
    #[tokio::test]
    async fn into_singleton_unbounded_top_level_none_cardinality() {
        let mut deployment = Deployment::new();

        let mut flow = FlowBuilder::new();
        let node = flow.process::<()>();
        let external = flow.external::<()>();

        let none_tick = node.tick();
        // Tick-located constants are lazy and never fire the tick on their own, so a spinning
        // batch drives `none_tick`.
        none_tick.spin_batch(q!(1)).all_ticks().for_each(q!(|_| {}));
        let top_level_none = none_tick
            .singleton(q!(123))
            .latest()
            .filter(q!(|_| false));
        let into_singleton = top_level_none.into_singleton();

        let tick_driver = node.spin();

        let tick_later = node.tick();
        let counts = into_singleton
            .snapshot(&tick_later, nondet!(/** test */))
            .into_stream()
            .count()
            .zip(tick_driver.batch(&tick_later, nondet!(/** test */)).count())
            .map(q!(|(c, _)| c))
            .all_ticks()
            .send_bincode_external(&external);

        let nodes = flow
            .with_process(&node, deployment.Localhost())
            .with_external(&external, deployment.Localhost())
            .deploy(&mut deployment);

        deployment.deploy().await.unwrap();

        let mut external_out = nodes.connect(counts).await;

        deployment.start().await.unwrap();

        assert_eq!(external_out.next().await.unwrap(), 1);
        assert_eq!(external_out.next().await.unwrap(), 1);
        assert_eq!(external_out.next().await.unwrap(), 1);
    }

    #[cfg(feature = "sim")]
    #[test]
    fn top_level_optional_some_into_stream_no_replay() {
        let mut flow = FlowBuilder::new();
        let node = flow.process::<()>();

        let source_iter = node.source_iter(q!(vec![1, 2, 3, 4]));
        let folded = source_iter.fold(q!(|| 0), q!(|a, b| *a += b));
        let filtered_some = folded.filter(q!(|_| true));

        let out_recv = filtered_some.into_stream().sim_output();

        flow.sim().exhaustive(async || {
            out_recv.assert_yields_only([10]).await;
        });
    }

    #[cfg(feature = "sim")]
    #[test]
    fn top_level_optional_none_into_stream_no_replay() {
        let mut flow = FlowBuilder::new();
        let node = flow.process::<()>();

        let source_iter = node.source_iter(q!(vec![1, 2, 3, 4]));
        let folded = source_iter.fold(q!(|| 0), q!(|a, b| *a += b));
        let filtered_none = folded.filter(q!(|_| false));

        let out_recv = filtered_none.into_stream().sim_output();

        flow.sim().exhaustive(async || {
            out_recv.assert_yields_only([] as [i32; 0]).await;
        });
    }

    #[cfg(feature = "deploy")]
    #[tokio::test]
    async fn test_optional_ref() {
        let mut deployment = Deployment::new();

        let mut flow = FlowBuilder::new();
        let external = flow.external::<()>();
        let p1 = flow.process::<()>();

        // Create an optional: reduce 0..5 => Some(10) (sum via reduce)
        let my_opt = p1.source_iter(q!(0..5i32)).reduce(q!(|a, b| *a += b));

        let opt_ref = my_opt.by_ref();

        // Use the optional ref in a map: unwrap_or(0) + x
        let out_port = p1
            .source_iter(q!(1..=3i32))
            .map(q!(|x| x + opt_ref.unwrap_or(0)))
            .send_bincode_external(&external);

        let nodes = flow
            .with_default_optimize()
            .with_process(&p1, deployment.Localhost())
            .with_external(&external, deployment.Localhost())
            .deploy(&mut deployment);

        deployment.deploy().await.unwrap();

        let mut out_recv = nodes.connect(out_port).await;

        deployment.start().await.unwrap();

        let mut results = Vec::new();
        for _ in 0..3 {
            results.push(out_recv.next().await.unwrap());
        }
        results.sort();
        // reduce(0..5) = 10, so results should be 11, 12, 13
        assert_eq!(results, vec![11, 12, 13]);
    }

    #[cfg(feature = "deploy")]
    #[tokio::test]
    async fn test_optional_ref_none() {
        let mut deployment = Deployment::new();

        let mut flow = FlowBuilder::new();
        let external = flow.external::<()>();
        let p1 = flow.process::<()>();

        // Create an optional from an empty source => None
        let my_opt = p1
            .source_iter(q!(std::iter::empty::<i32>()))
            .reduce(q!(|a, b| *a += b));

        let opt_ref = my_opt.by_ref();

        // Use the optional ref: should be None, so unwrap_or(99)
        let out_port = p1
            .source_iter(q!(1..=2i32))
            .map(q!(|x| x + opt_ref.unwrap_or(99)))
            .send_bincode_external(&external);

        let nodes = flow
            .with_default_optimize()
            .with_process(&p1, deployment.Localhost())
            .with_external(&external, deployment.Localhost())
            .deploy(&mut deployment);

        deployment.deploy().await.unwrap();

        let mut out_recv = nodes.connect(out_port).await;

        deployment.start().await.unwrap();

        let mut results = Vec::new();
        for _ in 0..2 {
            results.push(out_recv.next().await.unwrap());
        }
        results.sort();
        // optional is None, so unwrap_or(99) => 100, 101
        assert_eq!(results, vec![100, 101]);
    }

    #[cfg(feature = "deploy")]
    #[tokio::test]
    async fn test_optional_ref_and_consume() {
        let mut deployment = Deployment::new();

        let mut flow = FlowBuilder::new();
        let external = flow.external::<()>();
        let p1 = flow.process::<()>();

        // Use reduce to produce an Optional
        let my_opt = p1.source_iter(q!(0..5i32)).reduce(q!(|a, b| *a += b));

        let opt_ref = my_opt.by_ref();

        // Reference path
        let out_port_ref = p1
            .source_iter(q!(1..=2i32))
            .map(q!(|x| x + opt_ref.unwrap_or(0)))
            .send_bincode_external(&external);

        let nodes = flow
            .with_default_optimize()
            .with_process(&p1, deployment.Localhost())
            .with_external(&external, deployment.Localhost())
            .deploy(&mut deployment);

        deployment.deploy().await.unwrap();

        let mut out_recv_ref = nodes.connect(out_port_ref).await;

        deployment.start().await.unwrap();

        let mut ref_results = Vec::new();
        for _ in 0..2 {
            ref_results.push(out_recv_ref.next().await.unwrap());
        }
        ref_results.sort();
        // reduce(0..5) = 10, so 1+10=11, 2+10=12
        assert_eq!(ref_results, vec![11, 12]);
    }

    /// A root `Unbounded` optional (a tombstoned update feed) that holds the latest value sent
    /// on `input`, where `None` makes it null again. Built from a `sliced!` state singleton so
    /// that it can be simulated.
    #[cfg(feature = "sim")]
    pub(crate) fn sim_unbounded_optional<'a>(
        node: &crate::location::Process<'a>,
        input: crate::live_collections::Stream<
            Option<u32>,
            crate::location::Process<'a>,
            crate::live_collections::boundedness::Unbounded,
        >,
    ) -> Optional<u32, crate::location::Process<'a>, crate::live_collections::boundedness::Unbounded>
    {
        use crate::live_collections::sliced::sliced;
        use crate::nondet::nondet;

        let _ = node;
        sliced! {
            let batch = use::batch(input, nondet!(/** test */));
            let mut latest = use::state(|l| l.singleton(q!(None)));
            latest = batch.last().unwrap_or(latest);
            latest.clone()
        }
        .filter_map(q!(|v| v))
    }

    /// `or` of two root `Unbounded` optionals switches to the fallback once the left side becomes
    /// null again, and both sides' values are observable.
    #[cfg(feature = "sim")]
    #[test]
    fn sim_top_level_or_unbounded() {
        use crate::live_collections::stream::{ExactlyOnce, TotalOrder};
        use crate::nondet::nondet;

        let mut flow = FlowBuilder::new();
        let node = flow.process::<()>();

        let (left_port, left_input) = node.sim_input::<Option<u32>, TotalOrder, ExactlyOnce>();
        let (right_port, right_input) = node.sim_input::<Option<u32>, TotalOrder, ExactlyOnce>();
        let (trigger_port, trigger) = node.sim_input::<(), TotalOrder, ExactlyOnce>();

        let left = sim_unbounded_optional(&node, left_input);
        let right = sim_unbounded_optional(&node, right_input);
        let result = left.or(right).into_singleton();

        let tick = node.tick();
        let output = trigger
            .batch(&tick, nondet!(/** test */))
            .cross_singleton(result.snapshot(&tick, nondet!(/** test */)))
            .map(q!(|(_, v)| v))
            .all_ticks()
            .sim_output();

        let mut observed = std::collections::HashSet::new();
        flow.sim().exhaustive(async || {
            left_port.send(Some(1));
            right_port.send(Some(2));
            left_port.send(None);
            trigger_port.send(());

            // The trigger may fire before the result has any version (snapshots are lazy), in
            // which case nothing is observed.
            let out = output.collect::<Vec<_>>().await;
            assert!(out.len() <= 1);
            observed.extend(out);
        });

        assert!(observed.is_subset(&[None, Some(1), Some(2)].into_iter().collect()));
        // The left value, and the fallback after the left side becomes null, are both reachable.
        assert!(observed.contains(&Some(1)));
        assert!(observed.contains(&Some(2)));
    }

    /// `zip` of two root bounded optionals produces the pair once both are present (never a
    /// partial pair), and `is_some` of a root `Unbounded` optional tracks presence, including
    /// becoming null again.
    #[cfg(feature = "sim")]
    #[test]
    fn sim_top_level_zip_bounded_and_is_some() {
        use crate::live_collections::stream::{ExactlyOnce, TotalOrder};
        use crate::nondet::nondet;

        let mut flow = FlowBuilder::new();
        let node = flow.process::<()>();

        let (input_port, input) = node.sim_input::<Option<u32>, TotalOrder, ExactlyOnce>();
        let (trigger_port, trigger) = node.sim_input::<(), TotalOrder, ExactlyOnce>();

        let zipped = node
            .source_iter(q!([1u32]))
            .first()
            .zip(node.source_iter(q!([2u32])).first())
            .into_singleton();
        let present = sim_unbounded_optional(&node, input).is_some();

        let tick = node.tick();
        let output = trigger
            .batch(&tick, nondet!(/** test */))
            .cross_singleton(zipped.snapshot(&tick, nondet!(/** test */)))
            .cross_singleton(present.snapshot(&tick, nondet!(/** test */)))
            .map(q!(|((_, zipped), present)| (zipped, present)))
            .all_ticks()
            .sim_output();

        let mut observed_zipped = std::collections::HashSet::new();
        let mut observed_present = std::collections::HashSet::new();
        flow.sim().exhaustive(async || {
            input_port.send(Some(7));
            input_port.send(None);
            trigger_port.send(());

            for (zipped, present) in output.collect::<Vec<_>>().await {
                assert!(zipped.is_none() || zipped == Some((1, 2)));
                observed_zipped.insert(zipped);
                observed_present.insert(present);
            }
        });

        assert!(observed_zipped.contains(&Some((1, 2))));
        assert_eq!(observed_present, [false, true].into_iter().collect());
    }

    /// `sample_eager` of a root `InitNone` optional samples versions in order: every run's
    /// samples (ignoring stutters) are an increasing subsequence of the possible versions ending
    /// with the final one, and the simulator explores skipping intermediate versions.
    #[cfg(feature = "sim")]
    #[test]
    fn sim_top_level_optional_sample_eager() {
        use crate::live_collections::sliced::sliced;
        use crate::live_collections::stream::{ExactlyOnce, TotalOrder};
        use crate::nondet::nondet;

        let mut flow = FlowBuilder::new();
        let node = flow.process::<()>();

        let (input_port, input) = node.sim_input::<u32, TotalOrder, ExactlyOnce>();
        // A running sum, null until the first input arrives.
        let sum = sliced! {
            let batch = use::batch(input, nondet!(/** test */));
            let mut sum = use::state(|l| l.singleton(q!(0u32)));
            sum = sum.zip(batch.fold(q!(|| 0u32), q!(|acc, v| *acc += v)))
                .map(q!(|(sum, batch)| sum + batch));
            sum.clone()
        };
        let output = sum
            .sample_eager(nondet!(/** test */))
            // Samples may repeat (stutter); the samples are strictly increasing versions, so
            // dropping repeats is exactly de-duplication.
            .unique()
            .sim_output();

        let mut observed = std::collections::HashSet::new();
        flow.sim().exhaustive(async || {
            input_port.send(1);
            input_port.send(2);
            input_port.send(3);

            let samples = output.collect::<Vec<_>>().await;
            assert!(
                samples.windows(2).all(|w| w[0] < w[1]),
                "samples are not increasing: {samples:?}"
            );
            assert!(samples.iter().all(|v| [0, 1, 3, 6].contains(v)));
            assert_eq!(samples.last(), Some(&6));
            observed.insert(samples);
        });

        // Skipping intermediate versions is explored.
        assert!(observed.iter().any(|samples| samples.len() > 1));
        assert!(observed.contains(&vec![6]));
    }
}
