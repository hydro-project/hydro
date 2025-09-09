//! Definitions for interacting with locations using an untyped interface.
//!
//! Under the hood, locations are associated with a [`LocationId`] value that
//! uniquely identifies the location. Manipulating these values is useful for
//! observability and transforming the Hydro IR.

use serde::{Deserialize, Serialize};

#[cfg(stageleft_runtime)]
use crate::builder::{FlowState, ir::HydroIrMetadata};

#[expect(missing_docs, reason = "TODO")]
#[derive(PartialEq, Eq, Clone, Debug, Hash, Serialize, Deserialize)]
pub enum LocationId {
    Process(usize),
    Cluster(usize),
    Tick(usize, Box<LocationId>),
}

#[expect(missing_docs, reason = "TODO")]
impl LocationId {
    pub fn root(&self) -> &LocationId {
        match self {
            LocationId::Process(_) => self,
            LocationId::Cluster(_) => self,
            LocationId::Tick(_, id) => id.root(),
        }
    }

    pub fn is_root(&self) -> bool {
        match self {
            LocationId::Process(_) | LocationId::Cluster(_) => true,
            LocationId::Tick(_, _) => false,
        }
    }

    pub fn raw_id(&self) -> usize {
        match self {
            LocationId::Process(id) => *id,
            LocationId::Cluster(id) => *id,
            LocationId::Tick(_, _) => panic!("cannot get raw id for tick"),
        }
    }

    pub fn swap_root(&mut self, new_root: LocationId) {
        match self {
            LocationId::Tick(_, id) => {
                id.swap_root(new_root);
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

    fn new_node_metadata<T>(&self) -> HydroIrMetadata {
        use stageleft::quote_type;

        use crate::builder::ir::HydroIrOpMetadata;
        use crate::builder::ir::backtrace::Backtrace;

        HydroIrMetadata {
            location_kind: self.id(),
            output_type: Some(quote_type::<T>().into()),
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
