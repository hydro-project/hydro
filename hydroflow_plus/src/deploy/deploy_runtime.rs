use std::collections::HashMap;

use hydroflow::util::deploy::{
    ConnectedDemux, ConnectedDirect, ConnectedSink, ConnectedSource, ConnectedTagged, DeployPorts,
};
use serde::{Deserialize, Serialize};
use stageleft::{q, Quoted, RuntimeData};

#[derive(Default, Serialize, Deserialize)]
pub struct HydroflowPlusMeta {
    pub clusters: HashMap<usize, Vec<u32>>,
    pub cluster_id: Option<u32>,
    pub subgraph_id: usize,
}

pub fn cluster_members(
    cli: RuntimeData<&DeployPorts<HydroflowPlusMeta>>,
    of_cluster: usize,
) -> impl Quoted<&Vec<u32>> + Copy {
    q!(cli.meta.clusters.get(&of_cluster).unwrap())
}

pub fn cluster_self_id(
    cli: RuntimeData<&DeployPorts<HydroflowPlusMeta>>,
) -> impl Quoted<u32> + Copy {
    q!(cli
        .meta
        .cluster_id
        .expect("Tried to read Cluster ID on a non-cluster node"))
}

pub fn deploy_o2o(
    env: RuntimeData<&DeployPorts<HydroflowPlusMeta>>,
    p1_port: &str,
    p2_port: &str,
) -> (syn::Expr, syn::Expr) {
    (
        {
            q!({
                env.port(p1_port)
                    .connect_local_blocking::<ConnectedDirect>()
                    .into_sink()
            })
            .splice_untyped()
        },
        {
            q!({
                env.port(p2_port)
                    .connect_local_blocking::<ConnectedDirect>()
                    .into_source()
            })
            .splice_untyped()
        },
    )
}

pub fn deploy_o2m(
    env: RuntimeData<&DeployPorts<HydroflowPlusMeta>>,
    p1_port: &str,
    c2_port: &str,
) -> (syn::Expr, syn::Expr) {
    (
        {
            q!({
                env.port(p1_port)
                    .connect_local_blocking::<ConnectedDemux<ConnectedDirect>>()
                    .into_sink()
            })
            .splice_untyped()
        },
        {
            q!({
                env.port(c2_port)
                    .connect_local_blocking::<ConnectedDirect>()
                    .into_source()
            })
            .splice_untyped()
        },
    )
}

pub fn deploy_m2o(
    env: RuntimeData<&DeployPorts<HydroflowPlusMeta>>,
    c1_port: &str,
    p2_port: &str,
) -> (syn::Expr, syn::Expr) {
    (
        {
            q!({
                env.port(c1_port)
                    .connect_local_blocking::<ConnectedDirect>()
                    .into_sink()
            })
            .splice_untyped()
        },
        {
            q!({
                env.port(p2_port)
                    .connect_local_blocking::<ConnectedTagged<ConnectedDirect>>()
                    .into_source()
            })
            .splice_untyped()
        },
    )
}

pub fn deploy_m2m(
    env: RuntimeData<&DeployPorts<HydroflowPlusMeta>>,
    c1_port: &str,
    c2_port: &str,
) -> (syn::Expr, syn::Expr) {
    (
        {
            q!({
                env.port(c1_port)
                    .connect_local_blocking::<ConnectedDemux<ConnectedDirect>>()
                    .into_sink()
            })
            .splice_untyped()
        },
        {
            q!({
                env.port(c2_port)
                    .connect_local_blocking::<ConnectedTagged<ConnectedDirect>>()
                    .into_source()
            })
            .splice_untyped()
        },
    )
}

pub fn deploy_e2o(
    env: RuntimeData<&DeployPorts<HydroflowPlusMeta>>,
    _e1_port: &str,
    p2_port: &str,
) -> syn::Expr {
    q!({
        env.port(p2_port)
            .connect_local_blocking::<ConnectedDirect>()
            .into_source()
    })
    .splice_untyped()
}

pub fn deploy_o2e(
    env: RuntimeData<&DeployPorts<HydroflowPlusMeta>>,
    p1_port: &str,
    _e2_port: &str,
) -> syn::Expr {
    q!({
        env.port(p1_port)
            .connect_local_blocking::<ConnectedDirect>()
            .into_sink()
    })
    .splice_untyped()
}
