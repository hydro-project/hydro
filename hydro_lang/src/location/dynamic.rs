//! Definitions for interacting with locations using an untyped interface.
//!
//! Under the hood, locations are associated with a [`LocationId`] value that
//! uniquely identifies the location. Manipulating these values is useful for
//! observability and transforming the Hydro IR.

use serde::{Deserialize, Serialize};

use super::LocationKey;
#[cfg(stageleft_runtime)]
use crate::compile::{
    builder::FlowState,
    ir::{CollectionKind, HydroIrMetadata},
};
use crate::location::LocationType;

/// An enumeration representing a location heirarchy, including "virtual" locations (atomic/tick).
#[derive(PartialEq, Eq, PartialOrd, Ord, Clone, Hash, Serialize, Deserialize)]
pub enum LocationId {
    /// A process root location (i.e. a single node).
    Process(LocationKey),
    /// A cluster root location (i.e. multiple nodes).
    Cluster(LocationKey),
    /// An atomic region, within a tick.
    Atomic(
        /// The tick that the atomic region is associated with.
        Box<LocationId>,
    ),
    /// A tick within a location.
    Tick(usize, Box<LocationId>),
}

/// Implement Debug to Display-print the key, reduces snapshot verbosity.
impl std::fmt::Debug for LocationId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LocationId::Process(key) => write!(f, "Process({key})"),
            LocationId::Cluster(key) => write!(f, "Cluster({key})"),
            LocationId::Atomic(tick) => write!(f, "Atomic({tick:?})"),
            LocationId::Tick(tick, id) => write!(f, "Tick({tick}, {id:?})"),
        }
    }
}

impl LocationId {
    /// The [`LocationType`] of this location ID. `None` if this is not a root location.
    pub fn location_type(&self) -> Option<LocationType> {
        match self {
            LocationId::Process(_) => Some(LocationType::Process),
            LocationId::Cluster(_) => Some(LocationType::Cluster),
            _ => None,
        }
    }
}

#[expect(missing_docs, reason = "TODO")]
impl LocationId {
    pub fn root(&self) -> &LocationId {
        match self {
            LocationId::Process(_) => self,
            LocationId::Cluster(_) => self,
            LocationId::Atomic(tick) => tick.root(),
            LocationId::Tick(_, id) => id.root(),
        }
    }

    pub fn is_root(&self) -> bool {
        match self {
            LocationId::Process(_) | LocationId::Cluster(_) => true,
            LocationId::Atomic(_) => false,
            LocationId::Tick(_, _) => false,
        }
    }

    pub fn is_top_level(&self) -> bool {
        match self {
            LocationId::Process(_) | LocationId::Cluster(_) => true,
            LocationId::Atomic(_) => true,
            LocationId::Tick(_, _) => false,
        }
    }

    pub fn key(&self) -> LocationKey {
        match self {
            LocationId::Process(id) => *id,
            LocationId::Cluster(id) => *id,
            LocationId::Atomic(_) => panic!("cannot get raw id for atomic"),
            LocationId::Tick(_, _) => panic!("cannot get raw id for tick"),
        }
    }

    pub fn swap_root(&mut self, new_root: LocationId) {
        match self {
            LocationId::Tick(_, id) => {
                id.swap_root(new_root);
            }
            LocationId::Atomic(tick) => {
                tick.swap_root(new_root);
            }
            _ => {
                assert!(new_root.is_root());
                *self = new_root;
            }
        }
    }
}

#[cfg(stageleft_runtime)]
pub(crate) trait DynLocation: Clone {
    fn id(&self) -> LocationId;

    fn flow_state(&self) -> &FlowState;
    fn is_top_level() -> bool;

    fn new_node_metadata(&self, collection_kind: CollectionKind) -> HydroIrMetadata {
        use crate::compile::ir::HydroIrOpMetadata;
        use crate::compile::ir::backtrace::Backtrace;

        HydroIrMetadata {
            location_id: self.id(),
            collection_kind,
            cardinality: None,
            tag: None,
            op: HydroIrOpMetadata {
                backtrace: Backtrace::get_backtrace(2),
                cpu_usage: None,
                network_recv_cpu_usage: None,
                id: None,
            },
        }
    }
}
