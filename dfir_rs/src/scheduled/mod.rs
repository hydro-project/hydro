//! Hydroflow's outer scheduled layer. Deals with inter-subgraph runtime data-passing and scheduling.
//!
//! The most important item is the [`Hydroflow`](graph::Dfir) struct. Most of the items in this
//! module are supporting the implementation of the `Hydroflow` struct and its operation.

use crate::util::slot_vec::Key;

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

/// Tag for [`SubgraphId`].
pub enum SubgraphTag {}
/// A subgraph's ID. Invalid if used in a different [`graph::Dfir`]
/// instance than the original that created it.
pub type SubgraphId = Key<SubgraphTag>;

/// Tag for [`HandoffId`].
pub enum HandoffTag {}
/// A handoff's ID. Invalid if used in a different [`graph::Dfir`]
/// instance than the original that created it.
pub type HandoffId = Key<HandoffTag>;

/// A staten handle's ID. Invalid if used in a different [`graph::Dfir`]
/// instance than the original that created it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct StateId(pub(crate) usize);

/// Tag for [`LoopId`].
pub enum LoopTag {}
/// A loop's ID.
pub type LoopId = Key<LoopTag>;
