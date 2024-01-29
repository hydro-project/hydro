//! Module for [`StateHandle`], part of the "state API".

use std::marker::PhantomData;

use super::StateId;

/// A handle into a particular [`Hydroflow`](super::graph::Hydroflow) instance, referring to data
/// inserted by [`add_state`](super::graph::Hydroflow::add_state).
#[must_use]
#[derive(Debug)]
pub struct StateHandle<T> {
    /// A staten handle's ID. Invalid if used in a different [`graph::Hydroflow`]
    /// instance than the original that created it.
    pub state_id: StateId,
    pub(crate) _phantom: PhantomData<*mut T>,
}
impl<T> Copy for StateHandle<T> {}
impl<T> Clone for StateHandle<T> {
    fn clone(&self) -> Self {
        *self
    }
}
