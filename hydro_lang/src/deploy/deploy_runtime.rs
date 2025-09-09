#![allow(
    unused,
    reason = "unused in trybuild but the __staged version is needed"
)]

use std::collections::HashMap;

use hydro_deploy_integration::{
    ConnectedDemux, ConnectedDirect, ConnectedSink, ConnectedSource, ConnectedTagged, DeployPorts,
};
use serde::{Deserialize, Serialize};
use stageleft::{QuotedWithContext, RuntimeData, q};

#[derive(Default, Serialize, Deserialize)]
pub(super) struct HydroMeta {
    pub clusters: HashMap<usize, Vec<u32>>,
    pub cluster_id: Option<u32>,
    pub subgraph_id: usize,
}

pub fn cluster_members(
    cli: RuntimeData<&DeployPorts<HydroMeta>>,
    of_cluster: usize,
) -> impl QuotedWithContext<'_, &[u32], ()> + Copy {
    q!(cli
        .meta
        .clusters
        .get(&of_cluster)
        .map(|v| v.as_slice())
        .unwrap_or(&[])) // we default to empty slice because this is the scenario where the cluster is unused in the graph
}

pub fn cluster_self_id(
    cli: RuntimeData<&DeployPorts<HydroMeta>>,
) -> impl QuotedWithContext<'_, u32, ()> + Copy {
    q!(cli
        .meta
        .cluster_id
        .expect("Tried to read Cluster Self ID on a non-cluster node"))
}

pub fn deploy_o2o(
    env: RuntimeData<&DeployPorts<HydroMeta>>,
    p1_port: &str,
    p2_port: &str,
) -> (syn::Expr, syn::Expr) {
    (
        { q!(env.port(p1_port).connect::<ConnectedDirect>().into_sink()).splice_untyped_ctx(&()) },
        {
            q!(env.port(p2_port).connect::<ConnectedDirect>().into_source()).splice_untyped_ctx(&())
        },
    )
}

pub fn deploy_o2m(
    env: RuntimeData<&DeployPorts<HydroMeta>>,
    p1_port: &str,
    c2_port: &str,
) -> (syn::Expr, syn::Expr) {
    (
        {
            q!({
                env.port(p1_port)
                    .connect::<ConnectedDemux<ConnectedDirect>>()
                    .into_sink()
            })
            .splice_untyped_ctx(&())
        },
        {
            q!(env.port(c2_port).connect::<ConnectedDirect>().into_source()).splice_untyped_ctx(&())
        },
    )
}

pub fn deploy_m2o(
    env: RuntimeData<&DeployPorts<HydroMeta>>,
    c1_port: &str,
    p2_port: &str,
) -> (syn::Expr, syn::Expr) {
    (
        { q!(env.port(c1_port).connect::<ConnectedDirect>().into_sink()).splice_untyped_ctx(&()) },
        {
            q!({
                env.port(p2_port)
                    .connect::<ConnectedTagged<ConnectedDirect>>()
                    .into_source()
            })
            .splice_untyped_ctx(&())
        },
    )
}

pub fn deploy_m2m(
    env: RuntimeData<&DeployPorts<HydroMeta>>,
    c1_port: &str,
    c2_port: &str,
) -> (syn::Expr, syn::Expr) {
    (
        {
            q!({
                env.port(c1_port)
                    .connect::<ConnectedDemux<ConnectedDirect>>()
                    .into_sink()
            })
            .splice_untyped_ctx(&())
        },
        {
            q!({
                env.port(c2_port)
                    .connect::<ConnectedTagged<ConnectedDirect>>()
                    .into_source()
            })
            .splice_untyped_ctx(&())
        },
    )
}

pub fn deploy_e2o(
    env: RuntimeData<&DeployPorts<HydroMeta>>,
    _e1_port: &str,
    p2_port: &str,
) -> syn::Expr {
    q!(env.port(p2_port).connect::<ConnectedDirect>().into_source()).splice_untyped_ctx(&())
}

pub fn deploy_o2e(
    env: RuntimeData<&DeployPorts<HydroMeta>>,
    p1_port: &str,
    _e2_port: &str,
) -> syn::Expr {
    q!(env.port(p1_port).connect::<ConnectedDirect>().into_sink()).splice_untyped_ctx(&())
}
