use std::fmt::{Debug, Formatter};
use std::marker::PhantomData;

use super::{Location, LocationId};
use crate::builder::FlowState;
use crate::staging_util::Invariant;

pub struct Process<'a, ProcessTag = ()> {
    pub(crate) id: usize,
    pub(crate) flow_state: FlowState,
    pub(crate) _phantom: Invariant<'a, ProcessTag>,
}

impl<P> Debug for Process<'_, P> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "Process({})", self.id)
    }
}

impl<P> Eq for Process<'_, P> {}
impl<P> PartialEq for Process<'_, P> {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id && self.flow_state.as_ptr() == other.flow_state.as_ptr()
    }
}

impl<P> Clone for Process<'_, P> {
    fn clone(&self) -> Self {
        Process {
            id: self.id,
            flow_state: self.flow_state.clone(),
            _phantom: PhantomData,
        }
    }
}

impl<'a, P> Location<'a> for Process<'a, P> {
    type Root = Self;

    fn root(&self) -> Self::Root {
        self.clone()
    }

    fn id(&self) -> LocationId {
        LocationId::Process(self.id)
    }

    fn flow_state(&self) -> &FlowState {
        &self.flow_state
    }

    fn is_top_level() -> bool {
        true
    }
}
