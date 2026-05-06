//! DFIR runtime module. Contains the inline execution engine, context, and metrics.

use crate::util::slot_vec::Key;

pub mod context;
pub mod metrics;
pub mod net;

pub mod ticks;

/// Tag for [`SubgraphId`].
pub enum SubgraphTag {}
/// A subgraph's ID.
pub type SubgraphId = Key<SubgraphTag>;

/// Tag for [`LoopId`].
pub enum LoopTag {}
/// A loop's ID.
pub type LoopId = Key<LoopTag>;
