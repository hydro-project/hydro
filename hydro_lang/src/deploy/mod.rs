//! Infrastructure for deploying Hydro programs to the cloud using [`hydro_deploy`].

#[cfg(feature = "deploy_integration")]
pub(crate) mod deploy_runtime;

#[cfg(feature = "docker_runtime")]
pub mod deploy_runtime_containerized;

#[cfg(feature = "ecs_runtime")]
pub mod deploy_runtime_containerized_ecs;

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
#[cfg(feature = "docker_deploy")]
#[cfg_attr(docsrs, doc(cfg(feature = "docker_deploy")))]
pub mod deploy_graph_containerized;

#[cfg(stageleft_runtime)]
#[cfg(feature = "docker_deploy")]
#[cfg_attr(docsrs, doc(cfg(feature = "docker_deploy")))]
pub use deploy_graph_containerized::*;

#[cfg(stageleft_runtime)]
#[cfg(feature = "ecs_deploy")]
#[cfg_attr(docsrs, doc(cfg(feature = "ecs_deploy")))]
pub mod deploy_graph_containerized_ecs;

#[cfg(stageleft_runtime)]
#[cfg(feature = "ecs_deploy")]
#[cfg_attr(docsrs, doc(cfg(feature = "ecs_deploy")))]
pub use deploy_graph_containerized_ecs::*;
