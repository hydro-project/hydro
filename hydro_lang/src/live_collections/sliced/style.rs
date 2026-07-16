//! Styled wrappers for live collections used with the `sliced!` macro.
//!
//! This module provides wrapper types that store both a collection and its associated
//! non-determinism guard, allowing the nondet to be properly passed through during slicing.

#[cfg(stageleft_runtime)]
use std::marker::PhantomData;

use super::Slicable;
#[cfg(stageleft_runtime)]
use crate::forward_handle::{CycleCollection, CycleCollectionWithInitial};
use crate::forward_handle::{TickCycle, TickCycleHandle};
use crate::live_collections::boundedness::{Bounded, Boundedness, Unbounded};
use crate::live_collections::keyed_singleton::{BoundedValue, KeyedSingletonBound};
use crate::live_collections::singleton::SingletonBound;
use crate::live_collections::stream::{Ordering, Retries};
use crate::location::Location;
use crate::location::tick::{DeferTick, Tick};
use crate::nondet::NonDet;

/// Default style wrapper that stores a collection and its non-determinism guard.
///
/// This is used by the `sliced!` macro when no explicit style is specified.
pub struct Default<T> {
    pub(crate) collection: T,
    pub(crate) nondet: NonDet,
}

impl<T> Default<T> {
    /// Creates a new default-styled wrapper.
    pub fn new(collection: T, nondet: NonDet) -> Self {
        Self { collection, nondet }
    }
}

/// Helper function for unstyled `use` in `sliced!` macro - wraps the collection in Default style.
#[doc(hidden)]
pub fn default<T>(t: T, nondet: NonDet) -> Default<T> {
    Default::new(t, nondet)
}

/// Atomic style wrapper that stores a collection and its non-determinism guard.
///
/// This is used by the `sliced!` macro when `use::atomic(...)` is specified.
pub struct Atomic<T> {
    pub(crate) collection: T,
    pub(crate) nondet: NonDet,
}

impl<T> Atomic<T> {
    /// Creates a new atomic-styled wrapper.
    pub fn new(collection: T, nondet: NonDet) -> Self {
        Self { collection, nondet }
    }
}

/// Wraps a live collection to be treated atomically during slicing.
pub fn atomic<T>(t: T, nondet: NonDet) -> Atomic<T> {
    Atomic::new(t, nondet)
}

/// Creates a stateful cycle with an initial value for use in `sliced!`.
///
/// The tick (which is the source of truth for lifetimes) is bound first, returning a
/// [`StateBuilder`] which accepts the user-provided initializer via [`StateBuilder::build`].
/// This two-step layout ensures that type errors caused by a bad initializer are attributed
/// to the initializer argument rather than the tick or the entire macro invocation.
///
/// The initial value is computed from a closure that receives the location
/// for the body of the slice.
///
/// The initial value is used on the first iteration, and subsequent iterations receive
/// the value assigned to the mutable binding at the end of the previous iteration.
#[cfg(stageleft_runtime)]
pub fn state<'t, S, L>(tick: &'t Tick<L>) -> StateBuilder<'t, S, L> {
    StateBuilder {
        tick,
        _phantom: PhantomData,
    }
}

/// Builder returned by [`state`], which accepts the user-provided initializer.
#[cfg(stageleft_runtime)]
pub struct StateBuilder<'t, S, L> {
    tick: &'t Tick<L>,
    _phantom: PhantomData<S>,
}

#[cfg(stageleft_runtime)]
impl<'t, 'a, S, L: Location<'a>> StateBuilder<'t, S, L> {
    /// Supplies the initializer closure and creates the stateful cycle.
    ///
    /// The initializer takes the tick at the builder's `'t` lifetime (rather than a
    /// higher-ranked `for<'x>` bound), since the builder already stores the tick reference.
    /// This way, an initializer that requires a specific tick reference lifetime produces a
    /// borrow error directly on the tick, instead of a confusing "implementation of `Fn` is
    /// not general enough" error that blames an unrelated variable.
    #[expect(
        private_bounds,
        reason = "only Hydro collections can implement CycleCollectionWithInitial"
    )]
    pub fn build(self, initial_fn: impl FnOnce(&'t Tick<L>) -> S) -> (TickCycleHandle<'a, S>, S)
    where
        S: CycleCollectionWithInitial<'a, TickCycle, Location = Tick<L::DropConsistency>>,
    {
        let initial = initial_fn(self.tick);
        initial.location().clone().cycle_with_initial(initial)
    }
}

/// Creates a stateful cycle without an initial value for use in `sliced!`.
///
/// The tick (which is the source of truth for lifetimes) is bound first, returning a
/// [`StateNullBuilder`] which creates the cycle via [`StateNullBuilder::build`].
///
/// On the first iteration, the state will be null/empty. Subsequent iterations receive
/// the value assigned to the mutable binding at the end of the previous iteration.
#[cfg(stageleft_runtime)]
pub fn state_null<'t, S, L>(tick: &'t Tick<L>) -> StateNullBuilder<'t, S, L> {
    StateNullBuilder {
        tick,
        _phantom: PhantomData,
    }
}

/// Builder returned by [`state_null`], which creates the cycle.
#[cfg(stageleft_runtime)]
pub struct StateNullBuilder<'t, S, L> {
    tick: &'t Tick<L>,
    _phantom: PhantomData<S>,
}

#[cfg(stageleft_runtime)]
impl<'t, 'a, S, L: Location<'a>> StateNullBuilder<'t, S, L> {
    /// Creates the stateful cycle, which starts as null/empty on the first iteration.
    #[expect(
        private_bounds,
        reason = "only Hydro collections can implement CycleCollection"
    )]
    pub fn build(self) -> (TickCycleHandle<'a, S>, S)
    where
        S: CycleCollection<'a, TickCycle, Location = Tick<L::DropConsistency>> + DeferTick,
    {
        self.tick.cycle::<S, _>()
    }
}

// ============================================================================
// Default style Slicable implementations
//
// All of these drop consistency because they are performing non-deterministic
// batching / snapshotting.
// ============================================================================

impl<'a, T, L: Location<'a>, B: Boundedness, O: Ordering, R: Retries>
    Slicable<'a, L::DropConsistency> for Default<crate::live_collections::Stream<T, L, B, O, R>>
{
    type Slice = crate::live_collections::Stream<T, Tick<L::DropConsistency>, Bounded, O, R>;
    type Backtrace = crate::compile::ir::backtrace::Backtrace;

    fn get_location(&self) -> L::DropConsistency {
        self.collection.location().drop_consistency()
    }
    fn slice(self, tick: &Tick<L::DropConsistency>, backtrace: Self::Backtrace) -> Self::Slice {
        let out = self.collection.batch(tick, self.nondet);
        out.ir_node.borrow_mut().op_metadata_mut().backtrace = backtrace;
        out
    }
}

impl<'a, T, L: Location<'a>, B: SingletonBound> Slicable<'a, L::DropConsistency>
    for Default<crate::live_collections::Singleton<T, L, B>>
{
    type Slice = crate::live_collections::Singleton<T, Tick<L::DropConsistency>, Bounded>;
    type Backtrace = crate::compile::ir::backtrace::Backtrace;

    fn get_location(&self) -> L::DropConsistency {
        self.collection.location().drop_consistency()
    }
    fn slice(self, tick: &Tick<L::DropConsistency>, backtrace: Self::Backtrace) -> Self::Slice {
        let out = self.collection.snapshot(tick, self.nondet);
        out.ir_node.borrow_mut().op_metadata_mut().backtrace = backtrace;
        out
    }
}

impl<'a, T, L: Location<'a>, B: Boundedness> Slicable<'a, L::DropConsistency>
    for Default<crate::live_collections::Optional<T, L, B>>
{
    type Slice = crate::live_collections::Optional<T, Tick<L::DropConsistency>, Bounded>;
    type Backtrace = crate::compile::ir::backtrace::Backtrace;

    fn get_location(&self) -> L::DropConsistency {
        self.collection.location().drop_consistency()
    }
    fn slice(self, tick: &Tick<L::DropConsistency>, backtrace: Self::Backtrace) -> Self::Slice {
        let out = self.collection.snapshot(tick, self.nondet);
        out.ir_node.borrow_mut().op_metadata_mut().backtrace = backtrace;
        out
    }
}

impl<'a, K, V, L: Location<'a>, B: Boundedness, O: Ordering, R: Retries>
    Slicable<'a, L::DropConsistency>
    for Default<crate::live_collections::KeyedStream<K, V, L, B, O, R>>
{
    type Slice =
        crate::live_collections::KeyedStream<K, V, Tick<L::DropConsistency>, Bounded, O, R>;
    type Backtrace = crate::compile::ir::backtrace::Backtrace;

    fn get_location(&self) -> L::DropConsistency {
        self.collection.location().drop_consistency()
    }
    fn slice(self, tick: &Tick<L::DropConsistency>, backtrace: Self::Backtrace) -> Self::Slice {
        let out = self.collection.batch(tick, self.nondet);
        out.ir_node.borrow_mut().op_metadata_mut().backtrace = backtrace;
        out
    }
}

impl<'a, K, V, L: Location<'a>, B: KeyedSingletonBound<ValueBound = Unbounded>>
    Slicable<'a, L::DropConsistency>
    for Default<crate::live_collections::KeyedSingleton<K, V, L, B>>
{
    type Slice = crate::live_collections::KeyedSingleton<K, V, Tick<L::DropConsistency>, Bounded>;
    type Backtrace = crate::compile::ir::backtrace::Backtrace;

    fn get_location(&self) -> L::DropConsistency {
        self.collection.location().drop_consistency()
    }
    fn slice(self, tick: &Tick<L::DropConsistency>, backtrace: Self::Backtrace) -> Self::Slice {
        let out = self.collection.snapshot(tick, self.nondet);
        out.ir_node.borrow_mut().op_metadata_mut().backtrace = backtrace;
        out
    }
}

impl<'a, K, V, L: Location<'a>> Slicable<'a, L::DropConsistency>
    for Default<crate::live_collections::KeyedSingleton<K, V, L, BoundedValue>>
{
    type Slice = crate::live_collections::KeyedSingleton<K, V, Tick<L::DropConsistency>, Bounded>;
    type Backtrace = crate::compile::ir::backtrace::Backtrace;

    fn get_location(&self) -> L::DropConsistency {
        self.collection.location().drop_consistency()
    }
    fn slice(self, tick: &Tick<L::DropConsistency>, backtrace: Self::Backtrace) -> Self::Slice {
        let out = self.collection.batch(tick, self.nondet);
        out.ir_node.borrow_mut().op_metadata_mut().backtrace = backtrace;
        out
    }
}

// ============================================================================
// Atomic style Slicable implementations
// ============================================================================

impl<'a, T, L: Location<'a>, B: Boundedness, O: Ordering, R: Retries>
    Slicable<'a, L::DropConsistency>
    for Atomic<crate::live_collections::Stream<T, crate::location::Atomic<L>, B, O, R>>
{
    type Slice = crate::live_collections::Stream<T, Tick<L::DropConsistency>, Bounded, O, R>;
    type Backtrace = crate::compile::ir::backtrace::Backtrace;
    fn get_location(&self) -> L::DropConsistency {
        self.collection.location().tick.l.drop_consistency()
    }

    fn slice(self, tick: &Tick<L::DropConsistency>, backtrace: Self::Backtrace) -> Self::Slice {
        let out = self.collection.batch_atomic(tick, self.nondet);
        out.ir_node.borrow_mut().op_metadata_mut().backtrace = backtrace;
        out
    }
}

impl<'a, T, L: Location<'a>, B: SingletonBound> Slicable<'a, L::DropConsistency>
    for Atomic<crate::live_collections::Singleton<T, crate::location::Atomic<L>, B>>
{
    type Slice = crate::live_collections::Singleton<T, Tick<L::DropConsistency>, Bounded>;
    type Backtrace = crate::compile::ir::backtrace::Backtrace;
    fn get_location(&self) -> L::DropConsistency {
        self.collection.location().tick.l.drop_consistency()
    }

    fn slice(self, tick: &Tick<L::DropConsistency>, backtrace: Self::Backtrace) -> Self::Slice {
        let out = self.collection.snapshot_atomic(tick, self.nondet);
        out.ir_node.borrow_mut().op_metadata_mut().backtrace = backtrace;
        out
    }
}

impl<'a, T, L: Location<'a>, B: Boundedness> Slicable<'a, L::DropConsistency>
    for Atomic<crate::live_collections::Optional<T, crate::location::Atomic<L>, B>>
{
    type Slice = crate::live_collections::Optional<T, Tick<L::DropConsistency>, Bounded>;
    type Backtrace = crate::compile::ir::backtrace::Backtrace;
    fn get_location(&self) -> L::DropConsistency {
        self.collection.location().tick.l.drop_consistency()
    }

    fn slice(self, tick: &Tick<L::DropConsistency>, backtrace: Self::Backtrace) -> Self::Slice {
        let out = self.collection.snapshot_atomic(tick, self.nondet);
        out.ir_node.borrow_mut().op_metadata_mut().backtrace = backtrace;
        out
    }
}

impl<'a, K, V, L: Location<'a>, B: Boundedness, O: Ordering, R: Retries>
    Slicable<'a, L::DropConsistency>
    for Atomic<crate::live_collections::KeyedStream<K, V, crate::location::Atomic<L>, B, O, R>>
{
    type Slice =
        crate::live_collections::KeyedStream<K, V, Tick<L::DropConsistency>, Bounded, O, R>;
    type Backtrace = crate::compile::ir::backtrace::Backtrace;
    fn get_location(&self) -> L::DropConsistency {
        self.collection.location().tick.l.drop_consistency()
    }

    fn slice(self, tick: &Tick<L::DropConsistency>, backtrace: Self::Backtrace) -> Self::Slice {
        let out = self.collection.batch_atomic(tick, self.nondet);
        out.ir_node.borrow_mut().op_metadata_mut().backtrace = backtrace;
        out
    }
}

impl<'a, K, V, L: Location<'a>, B: KeyedSingletonBound<ValueBound = Unbounded>>
    Slicable<'a, L::DropConsistency>
    for Atomic<crate::live_collections::KeyedSingleton<K, V, crate::location::Atomic<L>, B>>
{
    type Slice = crate::live_collections::KeyedSingleton<K, V, Tick<L::DropConsistency>, Bounded>;
    type Backtrace = crate::compile::ir::backtrace::Backtrace;
    fn get_location(&self) -> L::DropConsistency {
        self.collection.location().tick.l.drop_consistency()
    }

    fn slice(self, tick: &Tick<L::DropConsistency>, backtrace: Self::Backtrace) -> Self::Slice {
        let out = self.collection.snapshot_atomic(tick, self.nondet);
        out.ir_node.borrow_mut().op_metadata_mut().backtrace = backtrace;
        out
    }
}

impl<'a, K, V, L: Location<'a>> Slicable<'a, L::DropConsistency>
    for Atomic<
        crate::live_collections::KeyedSingleton<K, V, crate::location::Atomic<L>, BoundedValue>,
    >
{
    type Slice = crate::live_collections::KeyedSingleton<K, V, Tick<L::DropConsistency>, Bounded>;
    type Backtrace = crate::compile::ir::backtrace::Backtrace;
    fn get_location(&self) -> L::DropConsistency {
        self.collection.location().tick.l.drop_consistency()
    }

    fn slice(self, tick: &Tick<L::DropConsistency>, backtrace: Self::Backtrace) -> Self::Slice {
        let out = self.collection.batch_atomic(tick, self.nondet);
        out.ir_node.borrow_mut().op_metadata_mut().backtrace = backtrace;
        out
    }
}
