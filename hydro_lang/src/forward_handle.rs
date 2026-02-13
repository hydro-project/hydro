//! Mechanisms for introducing forward references and cycles in Hydro.

use sealed::sealed;

use crate::compile::builder::CycleId;
use crate::location::Location;
use crate::location::dynamic::LocationId;
use crate::staging_util::Invariant;

#[sealed]
pub(crate) trait ReceiverKind {}

/// Marks that the [`ForwardHandle`] is for a "forward reference" to a later-defined collection.
///
/// When the handle is completed, the provided collection must not depend _synchronously_
/// (in the same tick) on the forward reference that was created earlier.
pub enum ForwardRef {}

#[sealed]
impl ReceiverKind for ForwardRef {}

/// Marks that the [`ForwardHandle`] will send a live collection to the next tick.
///
/// Dependency cycles are permitted for this handle type, because the collection used
/// to complete this handle will appear on the source-side on the _next_ tick.
pub enum TickCycle {}

#[sealed]
impl ReceiverKind for TickCycle {}

pub(crate) trait ReceiverComplete<'a, Marker>
where
    Marker: ReceiverKind,
{
    fn complete(self, cycle_id: CycleId, expected_location: LocationId);
}

pub(crate) trait CycleCollection<'a, Kind>: ReceiverComplete<'a, Kind>
where
    Kind: ReceiverKind,
{
    type Location: Location<'a>;

    fn create_source(id: CycleId, location: Self::Location) -> Self;
}

pub(crate) trait CycleCollectionWithInitial<'a, Kind>: ReceiverComplete<'a, Kind>
where
    Kind: ReceiverKind,
{
    type Location: Location<'a>;

    fn create_source_with_initial(
        cycle_id: CycleId,
        initial: Self,
        location: Self::Location,
    ) -> Self;
}

/// A handle that can be used to fulfill a forward reference.
///
/// The `C` type parameter specifies the collection type that can be used to complete the handle.
#[expect(
    private_bounds,
    reason = "only Hydro collections can implement ReceiverComplete"
)]
pub struct ForwardHandle<'a, C: ReceiverComplete<'a, ForwardRef>> {
    completed: bool,
    cycle_id: CycleId,
    expected_location: LocationId,
    _phantom: Invariant<'a, C>,
}

#[expect(
    private_bounds,
    reason = "only Hydro collections can implement ReceiverComplete"
)]
impl<'a, C: ReceiverComplete<'a, ForwardRef>> ForwardHandle<'a, C> {
    pub(crate) fn new(cycle_id: CycleId, expected_location: LocationId) -> Self {
        Self {
            completed: false,
            cycle_id,
            expected_location,
            _phantom: std::marker::PhantomData,
        }
    }
}

impl<'a, C: ReceiverComplete<'a, ForwardRef>> Drop for ForwardHandle<'a, C> {
    fn drop(&mut self) {
        if !self.completed && !std::thread::panicking() {
            panic!("ForwardHandle dropped without being completed");
        }
    }
}

#[expect(
    private_bounds,
    reason = "only Hydro collections can implement ReceiverComplete"
)]
impl<'a, C: ReceiverComplete<'a, ForwardRef>> ForwardHandle<'a, C> {
    /// Completes the forward reference with the given live collection. The initial forward reference
    /// collection created in [`Location::forward_ref`] will resolve to this value.
    ///
    /// The provided value **must not** depend _synchronously_ (in the same tick) on the forward reference
    /// collection, as doing so would create a dependency cycle. Asynchronous cycles (outside a tick) are
    /// allowed, since the program can continue running while the cycle is processed.
    pub fn complete(mut self, stream: impl Into<C>) {
        self.completed = true;
        C::complete(stream.into(), self.cycle_id, self.expected_location.clone())
    }
}

/// A handle that can be used to complete a tick cycle by sending a collection to the next tick.
///
/// The `C` type parameter specifies the collection type that can be used to complete the handle.
#[expect(
    private_bounds,
    reason = "only Hydro collections can implement ReceiverComplete"
)]
pub struct TickCycleHandle<'a, C: ReceiverComplete<'a, TickCycle>> {
    completed: bool,
    cycle_id: CycleId,
    expected_location: LocationId,
    _phantom: Invariant<'a, C>,
}

#[expect(
    private_bounds,
    reason = "only Hydro collections can implement ReceiverComplete"
)]
impl<'a, C: ReceiverComplete<'a, TickCycle>> TickCycleHandle<'a, C> {
    pub(crate) fn new(cycle_id: CycleId, expected_location: LocationId) -> Self {
        Self {
            completed: false,
            cycle_id,
            expected_location,
            _phantom: std::marker::PhantomData,
        }
    }
}

impl<'a, C: ReceiverComplete<'a, TickCycle>> Drop for TickCycleHandle<'a, C> {
    fn drop(&mut self) {
        if !self.completed && !std::thread::panicking() {
            panic!("TickCycleHandle dropped without being completed");
        }
    }
}

#[expect(
    private_bounds,
    reason = "only Hydro collections can implement ReceiverComplete"
)]
impl<'a, C: ReceiverComplete<'a, TickCycle>> TickCycleHandle<'a, C> {
    /// Sends the provided collection to the next tick, where it will be materialized
    /// in the collection returned by [`crate::location::Tick::cycle`] or
    /// [`crate::location::Tick::cycle_with_initial`].
    pub fn complete_next_tick(mut self, stream: impl Into<C>) {
        self.completed = true;
        C::complete(stream.into(), self.cycle_id, self.expected_location.clone())
    }
}

/// A trait for completing a cycle handle with a state value.
/// Used internally by the `sliced!` macro for state management.
#[doc(hidden)]
pub trait CompleteCycle<S> {
    /// Completes the cycle with the given state value.
    fn complete_next_tick(self, state: S);
}

impl<'a, C: ReceiverComplete<'a, TickCycle>> CompleteCycle<C> for TickCycleHandle<'a, C> {
    fn complete_next_tick(self, state: C) {
        TickCycleHandle::complete_next_tick(self, state)
    }
}
