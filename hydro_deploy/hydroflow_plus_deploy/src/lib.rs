stageleft::stageleft_no_entry_crate!();

mod runtime;
use std::collections::HashMap;

#[cfg(feature = "deploy")]
pub(crate) mod trybuild;

pub use runtime::*;

#[allow(clippy::allow_attributes, unused, reason = "stageleft")]
pub(crate) mod deploy_runtime;

#[cfg(feature = "deploy")]
mod deploy;

#[cfg(feature = "deploy")]
pub use deploy::*;
use serde::{Deserialize, Serialize};

#[derive(Default, Serialize, Deserialize)]
pub struct HydroflowPlusMeta {
    pub clusters: HashMap<usize, Vec<u32>>,
    pub cluster_id: Option<u32>,
    pub subgraph_id: usize,
}
