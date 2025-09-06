//! Mechanisms for introducing forward references and cycles in Hydro.

use sealed::sealed;

use crate::location::Location;
use crate::location::dynamic::LocationId;
use crate::staging_util::Invariant;

#[sealed]
pub(crate) trait ReceiverKind {}

pub enum ForwardRef {}

#[sealed]
impl ReceiverKind for ForwardRef {}

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

/// A handle that can be used to fulfill a forward reference or tick cycle.
///
/// See [`crate::builder::FlowBuilder`] for an explainer on the type parameters.
pub struct ForwardHandle<'a, Stream> {
    pub(crate) completed: bool,
    pub(crate) ident: syn::Ident,
    pub(crate) expected_location: LocationId,
    pub(crate) _phantom: Invariant<'a, Stream>,
}

impl<'a, S> Drop for ForwardHandle<'a, S> {
    fn drop(&mut self) {
        if !self.completed {
            panic!("ForwardReceiver dropped without being completed");
        }
    }
}

#[expect(
    private_bounds,
    reason = "only Hydro collections can implement ReceiverComplete"
)]
impl<'a, S> ForwardHandle<'a, S>
where
    S: ReceiverComplete<'a, ForwardRef>,
{
    pub fn complete(mut self, stream: impl Into<S>) {
        self.completed = true;
        let ident = self.ident.clone();
        S::complete(stream.into(), ident, self.expected_location.clone())
    }
}

#[expect(
    private_bounds,
    reason = "only Hydro collections can implement ReceiverComplete"
)]
impl<'a, S> ForwardHandle<'a, S>
where
    S: ReceiverComplete<'a, TickCycle>,
{
    pub fn complete_next_tick(mut self, stream: impl Into<S>) {
        self.completed = true;
        let ident = self.ident.clone();
        S::complete(stream.into(), ident, self.expected_location.clone())
    }
}
