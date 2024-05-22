//! Hydroflow's outer scheduled layer. Deals with inter-subgraph runtime data-passing and scheduling.
//!
//! The most important item is the [`Hydroflow`](graph::Hydroflow) struct. Most of the items in this
//! module are supporting the implementation of the `Hydroflow` struct and its operation.

use std::fmt::{Display, Formatter, Result};

use serde::Serialize;

pub mod context;
pub mod graph;
pub mod graph_ext;
pub mod handoff;
pub mod input;
pub mod net;
pub mod port;
pub mod query;
pub mod reactor;
pub mod state;
pub(crate) mod subgraph;

pub mod ticks;

/// A subgraph's ID. Invalid if used in a different [`graph::Hydroflow`]
/// instance than the original that created it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize)]
#[repr(transparent)]
pub struct SubgraphId(pub(crate) usize);
impl Display for SubgraphId {
    fn fmt(&self, f: &mut Formatter) -> Result {
        write!(f, "{}", self.0)
    }
}

/// A handoff's ID. Invalid if used in a different [`graph::Hydroflow`]
/// instance than the original that created it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
#[repr(transparent)]
pub struct HandoffId(pub(crate) usize);
impl Display for HandoffId {
    fn fmt(&self, f: &mut Formatter) -> Result {
        write!(f, "{}", self.0)
    }
}

/// A staten handle's ID. Invalid if used in a different [`graph::Hydroflow`]
/// instance than the original that created it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct StateId(pub(crate) usize);
