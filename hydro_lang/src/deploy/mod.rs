//! Infrastructure for deploying Hydro programs to the cloud using [`hydro_deploy`].

#[cfg(feature = "deploy_integration")]
mod deploy_runtime;

#[cfg(feature = "deploy_integration")]
mod deploy_runtime_containerized;

#[cfg(stageleft_runtime)]
#[cfg(feature = "deploy")]
#[cfg_attr(docsrs, doc(cfg(feature = "deploy")))]
pub use crate::compile::init_test;

#[cfg(stageleft_runtime)]
#[cfg(feature = "deploy")]
#[cfg_attr(docsrs, doc(cfg(feature = "deploy")))]
pub mod deploy_graph;

#[cfg(stageleft_runtime)]
#[cfg(feature = "deploy")]
#[cfg_attr(docsrs, doc(cfg(feature = "deploy")))]
pub use deploy_graph::*;

#[cfg(stageleft_runtime)]
#[cfg(feature = "deploy")]
#[cfg_attr(docsrs, doc(cfg(feature = "deploy")))]
pub mod deploy_graph_containerized;

#[cfg(stageleft_runtime)]
#[cfg(feature = "deploy")]
#[cfg_attr(docsrs, doc(cfg(feature = "deploy")))]
pub use deploy_graph_containerized::*;
