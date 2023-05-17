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
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize)]
#[repr(transparent)]
pub struct HandoffId(pub(crate) usize);
impl Display for HandoffId {
    fn fmt(&self, f: &mut Formatter) -> Result {
        write!(f, "{}", self.0)
    }
}

/// A staten handle's ID. Invalid if used in a different [`graph::Hydroflow`]
/// instance than the original that created it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct StateId(pub(crate) usize);
