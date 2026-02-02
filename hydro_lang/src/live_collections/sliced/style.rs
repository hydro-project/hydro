//! Styled wrappers for live collections used with the `sliced!` macro.
//!
//! This module provides wrapper types that store both a collection and its associated
//! non-determinism guard, allowing the nondet to be properly passed through during slicing.

use super::Slicable;
#[cfg(stageleft_runtime)]
use crate::forward_handle::{CycleCollection, CycleCollectionWithInitial};
use crate::forward_handle::{TickCycle, TickCycleHandle};
use crate::live_collections::boundedness::{Bounded, Boundedness, Unbounded};
use crate::live_collections::keyed_singleton::BoundedValue;
use crate::live_collections::stream::{Ordering, Retries};
use crate::location::tick::{DeferTick, Tick};
use crate::location::{Location, NoTick};
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
/// The initial value is computed from a closure that receives the location
/// for the body of the slice.
///
/// The initial value is used on the first iteration, and subsequent iterations receive
/// the value assigned to the mutable binding at the end of the previous iteration.
#[cfg(stageleft_runtime)]
#[expect(
    private_bounds,
    reason = "only Hydro collections can implement CycleCollectionWithInitial"
)]
pub fn state<
    'a,
    S: CycleCollectionWithInitial<'a, TickCycle, Location = Tick<L>>,
    L: Location<'a> + NoTick,
>(
    tick: &Tick<L>,
    initial_fn: impl FnOnce(&Tick<L>) -> S,
) -> (TickCycleHandle<'a, S>, S) {
    let initial = initial_fn(tick);
    tick.cycle_with_initial(initial)
}

/// Creates a stateful cycle without an initial value for use in `sliced!`.
///
/// On the first iteration, the state will be null/empty. Subsequent iterations receive
/// the value assigned to the mutable binding at the end of the previous iteration.
#[cfg(stageleft_runtime)]
#[expect(
    private_bounds,
    reason = "only Hydro collections can implement CycleCollection"
)]
pub fn state_null<
    'a,
    S: CycleCollection<'a, TickCycle, Location = Tick<L>> + DeferTick,
    L: Location<'a> + NoTick,
>(
    tick: &Tick<L>,
) -> (TickCycleHandle<'a, S>, S) {
    tick.cycle::<S>()
}

// ============================================================================
// Default style Slicable implementations
// ============================================================================

impl<'a, T, L: Location<'a>, B: Boundedness, O: Ordering, R: Retries> Slicable<'a, L>
    for Default<crate::live_collections::Stream<T, L, B, O, R>>
{
    type Slice = crate::live_collections::Stream<T, Tick<L>, Bounded, O, R>;
    type Backtrace = crate::compile::ir::backtrace::Backtrace;

    fn get_location(&self) -> &L {
        self.collection.location()
    }

    fn preferred_tick(&self) -> Option<Tick<L>> {
        None
    }

    fn slice(self, tick: &Tick<L>, backtrace: Self::Backtrace) -> Self::Slice {
        let out = self.collection.batch(tick, self.nondet);
        out.ir_node.borrow_mut().op_metadata_mut().backtrace = backtrace;
        out
    }
}

impl<'a, T, L: Location<'a>, B: Boundedness> Slicable<'a, L>
    for Default<crate::live_collections::Singleton<T, L, B>>
{
    type Slice = crate::live_collections::Singleton<T, Tick<L>, Bounded>;
    type Backtrace = crate::compile::ir::backtrace::Backtrace;

    fn get_location(&self) -> &L {
        self.collection.location()
    }

    fn preferred_tick(&self) -> Option<Tick<L>> {
        None
    }

    fn slice(self, tick: &Tick<L>, backtrace: Self::Backtrace) -> Self::Slice {
        let out = self.collection.snapshot(tick, self.nondet);
        out.ir_node.borrow_mut().op_metadata_mut().backtrace = backtrace;
        out
    }
}

impl<'a, T, L: Location<'a>, B: Boundedness> Slicable<'a, L>
    for Default<crate::live_collections::Optional<T, L, B>>
{
    type Slice = crate::live_collections::Optional<T, Tick<L>, Bounded>;
    type Backtrace = crate::compile::ir::backtrace::Backtrace;

    fn get_location(&self) -> &L {
        self.collection.location()
    }

    fn preferred_tick(&self) -> Option<Tick<L>> {
        None
    }

    fn slice(self, tick: &Tick<L>, backtrace: Self::Backtrace) -> Self::Slice {
        let out = self.collection.snapshot(tick, self.nondet);
        out.ir_node.borrow_mut().op_metadata_mut().backtrace = backtrace;
        out
    }
}

impl<'a, K, V, L: Location<'a>, O: Ordering, R: Retries> Slicable<'a, L>
    for Default<crate::live_collections::KeyedStream<K, V, L, Unbounded, O, R>>
{
    type Slice = crate::live_collections::KeyedStream<K, V, Tick<L>, Bounded, O, R>;
    type Backtrace = crate::compile::ir::backtrace::Backtrace;

    fn get_location(&self) -> &L {
        self.collection.location()
    }

    fn preferred_tick(&self) -> Option<Tick<L>> {
        None
    }

    fn slice(self, tick: &Tick<L>, backtrace: Self::Backtrace) -> Self::Slice {
        let out = self.collection.batch(tick, self.nondet);
        out.ir_node.borrow_mut().op_metadata_mut().backtrace = backtrace;
        out
    }
}

impl<'a, K, V, L: Location<'a>> Slicable<'a, L>
    for Default<crate::live_collections::KeyedSingleton<K, V, L, Unbounded>>
{
    type Slice = crate::live_collections::KeyedSingleton<K, V, Tick<L>, Bounded>;
    type Backtrace = crate::compile::ir::backtrace::Backtrace;

    fn get_location(&self) -> &L {
        self.collection.location()
    }

    fn preferred_tick(&self) -> Option<Tick<L>> {
        None
    }

    fn slice(self, tick: &Tick<L>, backtrace: Self::Backtrace) -> Self::Slice {
        let out = self.collection.snapshot(tick, self.nondet);
        out.ir_node.borrow_mut().op_metadata_mut().backtrace = backtrace;
        out
    }
}

impl<'a, K, V, L: Location<'a> + NoTick> Slicable<'a, L>
    for Default<crate::live_collections::KeyedSingleton<K, V, L, BoundedValue>>
{
    type Slice = crate::live_collections::KeyedSingleton<K, V, Tick<L>, Bounded>;
    type Backtrace = crate::compile::ir::backtrace::Backtrace;

    fn get_location(&self) -> &L {
        self.collection.location()
    }

    fn preferred_tick(&self) -> Option<Tick<L>> {
        None
    }

    fn slice(self, tick: &Tick<L>, backtrace: Self::Backtrace) -> Self::Slice {
        let out = self.collection.batch(tick, self.nondet);
        out.ir_node.borrow_mut().op_metadata_mut().backtrace = backtrace;
        out
    }
}

// ============================================================================
// Atomic style Slicable implementations
// ============================================================================

impl<'a, T, L: Location<'a> + NoTick, O: Ordering, R: Retries> Slicable<'a, L>
    for Atomic<crate::live_collections::Stream<T, crate::location::Atomic<L>, Unbounded, O, R>>
{
    type Slice = crate::live_collections::Stream<T, Tick<L>, Bounded, O, R>;
    type Backtrace = crate::compile::ir::backtrace::Backtrace;

    fn preferred_tick(&self) -> Option<Tick<L>> {
        Some(self.collection.location().tick.clone())
    }

    fn get_location(&self) -> &L {
        panic!("Atomic location has no accessible inner location")
    }

    fn slice(self, tick: &Tick<L>, backtrace: Self::Backtrace) -> Self::Slice {
        assert_eq!(
            self.collection.location().tick.id(),
            tick.id(),
            "Mismatched tick for atomic slicing"
        );

        let out = self.collection.batch_atomic(self.nondet);
        out.ir_node.borrow_mut().op_metadata_mut().backtrace = backtrace;
        out
    }
}

impl<'a, T, L: Location<'a> + NoTick> Slicable<'a, L>
    for Atomic<crate::live_collections::Singleton<T, crate::location::Atomic<L>, Unbounded>>
{
    type Slice = crate::live_collections::Singleton<T, Tick<L>, Bounded>;
    type Backtrace = crate::compile::ir::backtrace::Backtrace;

    fn preferred_tick(&self) -> Option<Tick<L>> {
        Some(self.collection.location().tick.clone())
    }

    fn get_location(&self) -> &L {
        panic!("Atomic location has no accessible inner location")
    }

    fn slice(self, tick: &Tick<L>, backtrace: Self::Backtrace) -> Self::Slice {
        assert_eq!(
            self.collection.location().tick.id(),
            tick.id(),
            "Mismatched tick for atomic slicing"
        );

        let out = self.collection.snapshot_atomic(self.nondet);
        out.ir_node.borrow_mut().op_metadata_mut().backtrace = backtrace;
        out
    }
}

impl<'a, T, L: Location<'a> + NoTick> Slicable<'a, L>
    for Atomic<crate::live_collections::Optional<T, crate::location::Atomic<L>, Unbounded>>
{
    type Slice = crate::live_collections::Optional<T, Tick<L>, Bounded>;
    type Backtrace = crate::compile::ir::backtrace::Backtrace;

    fn preferred_tick(&self) -> Option<Tick<L>> {
        Some(self.collection.location().tick.clone())
    }

    fn get_location(&self) -> &L {
        panic!("Atomic location has no accessible inner location")
    }

    fn slice(self, tick: &Tick<L>, backtrace: Self::Backtrace) -> Self::Slice {
        assert_eq!(
            self.collection.location().tick.id(),
            tick.id(),
            "Mismatched tick for atomic slicing"
        );

        let out = self.collection.snapshot_atomic(self.nondet);
        out.ir_node.borrow_mut().op_metadata_mut().backtrace = backtrace;
        out
    }
}

impl<'a, K, V, L: Location<'a> + NoTick, O: Ordering, R: Retries> Slicable<'a, L>
    for Atomic<
        crate::live_collections::KeyedStream<K, V, crate::location::Atomic<L>, Unbounded, O, R>,
    >
{
    type Slice = crate::live_collections::KeyedStream<K, V, Tick<L>, Bounded, O, R>;
    type Backtrace = crate::compile::ir::backtrace::Backtrace;

    fn preferred_tick(&self) -> Option<Tick<L>> {
        Some(self.collection.location().tick.clone())
    }

    fn get_location(&self) -> &L {
        panic!("Atomic location has no accessible inner location")
    }

    fn slice(self, tick: &Tick<L>, backtrace: Self::Backtrace) -> Self::Slice {
        assert_eq!(
            self.collection.location().tick.id(),
            tick.id(),
            "Mismatched tick for atomic slicing"
        );

        let out = self.collection.batch_atomic(self.nondet);
        out.ir_node.borrow_mut().op_metadata_mut().backtrace = backtrace;
        out
    }
}

impl<'a, K, V, L: Location<'a> + NoTick> Slicable<'a, L>
    for Atomic<crate::live_collections::KeyedSingleton<K, V, crate::location::Atomic<L>, Unbounded>>
{
    type Slice = crate::live_collections::KeyedSingleton<K, V, Tick<L>, Bounded>;
    type Backtrace = crate::compile::ir::backtrace::Backtrace;

    fn preferred_tick(&self) -> Option<Tick<L>> {
        Some(self.collection.location().tick.clone())
    }

    fn get_location(&self) -> &L {
        panic!("Atomic location has no accessible inner location")
    }

    fn slice(self, tick: &Tick<L>, backtrace: Self::Backtrace) -> Self::Slice {
        assert_eq!(
            self.collection.location().tick.id(),
            tick.id(),
            "Mismatched tick for atomic slicing"
        );

        let out = self.collection.snapshot_atomic(self.nondet);
        out.ir_node.borrow_mut().op_metadata_mut().backtrace = backtrace;
        out
    }
}

impl<'a, K, V, L: Location<'a> + NoTick> Slicable<'a, L>
    for Atomic<
        crate::live_collections::KeyedSingleton<K, V, crate::location::Atomic<L>, BoundedValue>,
    >
{
    type Slice = crate::live_collections::KeyedSingleton<K, V, Tick<L>, Bounded>;
    type Backtrace = crate::compile::ir::backtrace::Backtrace;

    fn preferred_tick(&self) -> Option<Tick<L>> {
        Some(self.collection.location().tick.clone())
    }

    fn get_location(&self) -> &L {
        panic!("Atomic location has no accessible inner location")
    }

    fn slice(self, tick: &Tick<L>, backtrace: Self::Backtrace) -> Self::Slice {
        assert_eq!(
            self.collection.location().tick.id(),
            tick.id(),
            "Mismatched tick for atomic slicing"
        );

        let out = self.collection.batch_atomic(self.nondet);
        out.ir_node.borrow_mut().op_metadata_mut().backtrace = backtrace;
        out
    }
}
