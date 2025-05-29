use sealed::sealed;

use crate::location::{Location, LocationId};
use crate::staging_util::Invariant;

#[sealed]
pub trait CycleMarker {}

pub enum ForwardRefMarker {}

#[sealed]
impl CycleMarker for ForwardRefMarker {}

pub enum TickCycleMarker {}

#[sealed]
impl CycleMarker for TickCycleMarker {}

pub trait DeferTick {
    fn defer_tick(self) -> Self;
}

pub trait CycleComplete<'a, Marker>
where
    Marker: CycleMarker,
{
    fn complete(self, ident: syn::Ident, expected_location: LocationId);
}

pub trait CycleCollection<'a, Marker>: CycleComplete<'a, Marker>
where
    Marker: CycleMarker,
{
    type Location: Location<'a>;

    fn create_source(ident: syn::Ident, location: Self::Location) -> Self;
}

pub trait CycleCollectionWithInitial<'a, Marker>: CycleComplete<'a, Marker>
where
    Marker: CycleMarker,
{
    type Location: Location<'a>;

    fn create_source(ident: syn::Ident, initial: Self, location: Self::Location) -> Self;
}

/// Represents a forward reference in the graph that will be fulfilled
/// by a stream that is not yet known.
///
/// See [`crate::FlowBuilder`] for an explainer on the type parameters.
pub struct ForwardRef<'a, Stream>
where
    Stream: CycleComplete<'a, ForwardRefMarker>,
{
    pub(crate) completed: bool,
    pub(crate) ident: syn::Ident,
    pub(crate) expected_location: LocationId,
    pub(crate) _phantom: Invariant<'a, Stream>,
}

impl<'a, S> Drop for ForwardRef<'a, S>
where
    S: CycleComplete<'a, ForwardRefMarker>,
{
    fn drop(&mut self) {
        if !self.completed {
            panic!("ForwardRef dropped without being completed");
        }
    }
}

impl<'a, S> ForwardRef<'a, S>
where
    S: CycleComplete<'a, ForwardRefMarker>,
{
    pub fn complete(mut self, stream: S) {
        self.completed = true;
        let ident = self.ident.clone();
        S::complete(stream, ident, self.expected_location.clone())
    }
}

pub struct TickCycle<'a, Stream>
where
    Stream: CycleComplete<'a, TickCycleMarker> + DeferTick,
{
    pub(crate) completed: bool,
    pub(crate) ident: syn::Ident,
    pub(crate) expected_location: LocationId,
    pub(crate) _phantom: Invariant<'a, Stream>,
}

impl<'a, S> Drop for TickCycle<'a, S>
where
    S: CycleComplete<'a, TickCycleMarker> + DeferTick,
{
    fn drop(&mut self) {
        if !self.completed {
            panic!("TickCycle dropped without being completed");
        }
    }
}

impl<'a, S> TickCycle<'a, S>
where
    S: CycleComplete<'a, TickCycleMarker> + DeferTick,
{
    pub fn complete_next_tick(mut self, stream: S) {
        self.completed = true;
        let ident = self.ident.clone();
        S::complete(stream.defer_tick(), ident, self.expected_location.clone())
    }
}
