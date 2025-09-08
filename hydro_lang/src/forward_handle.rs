//! Mechanisms for introducing forward references and cycles in Hydro.

use sealed::sealed;

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
    fn complete(self, ident: syn::Ident, expected_location: LocationId);
}

pub(crate) trait CycleCollection<'a, Kind>: ReceiverComplete<'a, Kind>
where
    Kind: ReceiverKind,
{
    type Location: Location<'a>;

    fn create_source(ident: syn::Ident, location: Self::Location) -> Self;
}

pub(crate) trait CycleCollectionWithInitial<'a, Kind>: ReceiverComplete<'a, Kind>
where
    Kind: ReceiverKind,
{
    type Location: Location<'a>;

    fn create_source_with_initial(
        ident: syn::Ident,
        initial: Self,
        location: Self::Location,
    ) -> Self;
}

#[expect(
    private_bounds,
    reason = "only Hydro collections can implement ReceiverComplete"
)]
/// A handle that can be used to fulfill a forward reference.
///
/// The `C` type parameter specifies the collection type that can be used to complete the handle.
pub struct ForwardHandle<'a, C: ReceiverComplete<'a, ForwardRef>> {
    pub(crate) completed: bool,
    pub(crate) ident: syn::Ident,
    pub(crate) expected_location: LocationId,
    pub(crate) _phantom: Invariant<'a, C>,
}

impl<'a, C: ReceiverComplete<'a, ForwardRef>> Drop for ForwardHandle<'a, C> {
    fn drop(&mut self) {
        if !self.completed {
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
        let ident = self.ident.clone();
        C::complete(stream.into(), ident, self.expected_location.clone())
    }
}

#[expect(
    private_bounds,
    reason = "only Hydro collections can implement ReceiverComplete"
)]
/// A handle that can be used to complete a tick cycle by sending a collection to the next tick.
///
/// The `C` type parameter specifies the collection type that can be used to complete the handle.
pub struct TickCycleHandle<'a, C: ReceiverComplete<'a, TickCycle>> {
    pub(crate) completed: bool,
    pub(crate) ident: syn::Ident,
    pub(crate) expected_location: LocationId,
    pub(crate) _phantom: Invariant<'a, C>,
}

impl<'a, C: ReceiverComplete<'a, TickCycle>> Drop for TickCycleHandle<'a, C> {
    fn drop(&mut self) {
        if !self.completed {
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
        let ident = self.ident.clone();
        C::complete(stream.into(), ident, self.expected_location.clone())
    }
}
