//! Infrastructure for deploying Hydro programs to the cloud using [`hydro_deploy`].

#[cfg(feature = "deploy_integration")]
mod deploy_runtime;

#[cfg(stageleft_runtime)]
#[cfg(feature = "deploy")]
pub(crate) mod trybuild;

#[cfg(stageleft_runtime)]
#[cfg(feature = "deploy")]
mod trybuild_rewriters;

#[cfg(stageleft_runtime)]
#[cfg(feature = "deploy")]
#[cfg_attr(docsrs, doc(cfg(feature = "deploy")))]
pub use trybuild::init_test;

#[cfg(stageleft_runtime)]
#[cfg(feature = "deploy")]
#[cfg_attr(docsrs, doc(cfg(feature = "deploy")))]
pub mod deploy_graph;

#[cfg(stageleft_runtime)]
#[cfg(feature = "deploy")]
#[cfg_attr(docsrs, doc(cfg(feature = "deploy")))]
pub use deploy_graph::*;
