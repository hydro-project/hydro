//! DFIR runtime module. Contains the inline execution engine, context, metrics, and state.

use crate::util::slot_vec::Key;

pub mod context;
pub mod metrics;
pub mod net;
pub mod state;

pub mod ticks;

/// Tag for [`SubgraphId`].
pub enum SubgraphTag {}
/// A subgraph's ID. Used as a key for metrics tracking.
pub type SubgraphId = Key<SubgraphTag>;

/// Tag for [`HandoffId`].
pub enum HandoffTag {}
/// A handoff's ID. Used as a key for metrics tracking.
pub type HandoffId = Key<HandoffTag>;

/// Tag for [`StateId`].
pub enum StateTag {}
/// A state handle's ID.
pub type StateId = Key<StateTag>;

/// Tag for [`LoopId`].
pub enum LoopTag {}
/// A loop's ID.
pub type LoopId = Key<LoopTag>;

/// Defines when state should be reset.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StateLifespan {
    /// Always reset, associated with the subgraph.
    Subgraph(SubgraphId),
    /// Reset between loop executions.
    Loop(LoopId),
    /// Reset between ticks.
    Tick,
    /// Never reset.
    Static,
}
