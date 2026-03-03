//! Definition of the [`Process`] location type, representing a single-node
//! compute location in a distributed Hydro program.
//!
//! A [`Process`] is the simplest kind of location: it corresponds to exactly one
//! machine (or OS process) and all live collections placed on it are materialized
//! on that single node. Use a process when the computation does not need to be
//! replicated or partitioned across multiple nodes.
//!
//! Processes are created via [`FlowBuilder::process`](crate::compile::builder::FlowBuilder::process)
//! and are parameterized by a **tag type** (`ProcessTag`) that lets the type
//! system distinguish different processes at compile time.

use std::fmt::{Debug, Formatter};
use std::marker::PhantomData;

use super::{Location, LocationId};
use crate::compile::builder::FlowState;
use crate::location::LocationKey;
use crate::staging_util::Invariant;

/// A single-node location in a distributed Hydro program.
///
/// `Process` represents exactly one machine (or OS process) and is one of the
/// core location types that implements the [`Location`] trait. Live collections
/// placed on a `Process` are materialized entirely on that single node.
///
/// The type parameter `ProcessTag` is a compile-time marker that differentiates
/// distinct processes in the same dataflow graph (e.g. `Process<'a, Leader>` vs
/// `Process<'a, Follower>`). It defaults to `()` when only one process is
/// needed.
///
/// # Creating a Process
/// ```rust,ignore
/// let mut flow = FlowBuilder::new();
/// let node = flow.process::<MyTag>();
/// ```
pub struct Process<'a, ProcessTag = ()> {
    pub(crate) key: LocationKey,
    pub(crate) flow_state: FlowState,
    pub(crate) _phantom: Invariant<'a, ProcessTag>,
}

impl<P> Debug for Process<'_, P> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "Process({})", self.key)
    }
}

impl<P> Eq for Process<'_, P> {}
impl<P> PartialEq for Process<'_, P> {
    fn eq(&self, other: &Self) -> bool {
        self.key == other.key && FlowState::ptr_eq(&self.flow_state, &other.flow_state)
    }
}

impl<P> Clone for Process<'_, P> {
    fn clone(&self) -> Self {
        Process {
            key: self.key,
            flow_state: self.flow_state.clone(),
            _phantom: PhantomData,
        }
    }
}

impl<'a, P> super::dynamic::DynLocation for Process<'a, P> {
    fn id(&self) -> LocationId {
        LocationId::Process(self.key)
    }

    fn flow_state(&self) -> &FlowState {
        &self.flow_state
    }

    fn is_top_level() -> bool {
        true
    }

    fn multiversioned(&self) -> bool {
        false // processes are always single-versioned
    }
}

impl<'a, P> Location<'a> for Process<'a, P> {
    type Root = Self;

    fn root(&self) -> Self::Root {
        self.clone()
    }
}
