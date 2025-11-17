#![allow(
    unused,
    reason = "unused in trybuild but the __staged version is needed"
)]

use std::collections::HashMap;
use std::marker::PhantomData;
use std::pin::Pin;

use bytes::{Bytes, BytesMut};
// use dfir_rs::pin_project_lite::pin_project;
// use dfir_rs::sinktools;
// use dfir_rs::sinktools::demux_map::DemuxMap;
use futures::sink::Buffer;
use futures::{Sink, SinkExt, StreamExt};
use hydro_deploy_integration::{
    ConnectedDemux, ConnectedDirect, ConnectedSink, ConnectedSource, ConnectedTagged, DeployPorts,
};
use serde::{Deserialize, Serialize};
use stageleft::{QuotedWithContext, RuntimeData, q};

use crate::location::cluster::ClusterIds;
use crate::location::dynamic::LocationId;
use crate::location::{MemberId, MembershipEvent};

#[derive(Default, Serialize, Deserialize)]
pub(super) struct HydroMeta {
    pub clusters: HashMap<usize, Vec<MemberId<()>>>,
    pub cluster_id: Option<MemberId<()>>,
    pub subgraph_id: usize,
}

pub(super) fn cluster_members(
    cli: RuntimeData<&DeployPorts<HydroMeta>>,
    of_cluster: usize,
) -> impl QuotedWithContext<'_, &[MemberId<()>], ()> + Clone {
    q!(cli
        .meta
        .clusters
        .get(&of_cluster)
        .map(|v| v.as_slice())
        .unwrap_or(&[])) // we default to empty slice because this is the scenario where the cluster is unused in the graph
}

pub(super) fn cluster_self_id(
    cli: RuntimeData<&DeployPorts<HydroMeta>>,
) -> impl QuotedWithContext<'_, MemberId<()>, ()> + Clone {
    // TRYBUILD=overwrite INSTA_FORCE_PASS=1 INSTA_UPDATE=always cargo nextest run cluster::paxos_bench::tests::paxos_some_throughput
    q!(cli
        .meta
        .cluster_id
        .clone()
        .expect("Tried to read Cluster Self ID on a non-cluster node"))
}

pub fn cluster_membership_stream<'a>(
    location_id: &LocationId,
) -> impl QuotedWithContext<
    'a,
    Box<dyn futures::Stream<Item = (MemberId<()>, MembershipEvent)> + Unpin>,
    (),
> {
    let cluster_ids = ClusterIds {
        id: location_id.raw_id(),
        _phantom: Default::default(),
    };

    q!(Box::new(futures::stream::iter(
        cluster_ids
            .iter()
            .cloned()
            .map(|member_id| (member_id, MembershipEvent::Joined))
    ))
        as Box<
            dyn futures::Stream<Item = (MemberId<()>, MembershipEvent)> + Unpin,
        >)
}

pub(super) fn deploy_o2o(
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

pub(super) fn deploy_o2m(
    env: RuntimeData<&DeployPorts<HydroMeta>>,
    p1_port: &str,
    c2_port: &str,
) -> (syn::Expr, syn::Expr) {
    (
        {
            q!(sinktools::map(
                |(k, v): (MemberId<()>, Bytes)| { (k.get_raw_id(), v) },
                env.port(p1_port)
                    .connect::<ConnectedDemux<ConnectedDirect>>()
                    .into_sink()
            ))
            .splice_untyped_ctx(&())
            // QuotedWithContext::<'a, MapAdapterTypeHinter<MemberId<Tag>>, ()>::splice_untyped_ctx(
            //     q!(MapAdapterTypeHinter {
            //         sink: env
            //             .port(p1_port)
            //             .connect::<ConnectedDemux<u32, ConnectedDirect>>()
            //             .into_sink(),
            //         mapper: Box::new(|id: MemberId<_>| id.get_raw_id()),
            //         _phantom: Default::default(),
            //     }),
            //     &(),
            // )
        },
        {
            q!(env.port(c2_port).connect::<ConnectedDirect>().into_source()).splice_untyped_ctx(&())
        },
    )
}

pub(super) fn deploy_m2o(
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
                    .map(|v| v.map(|(k, v)| (MemberId::<()>::from_raw_id(k), v)))
            })
            .splice_untyped_ctx(&())

            // QuotedWithContext::<'a, SourceAdapterTypeHinter<MemberId<Tag>>, ()>::splice_untyped_ctx(
            //     q!(SourceAdapterTypeHinter {
            //         stream: env
            //             .port(p2_port)
            //             .connect::<ConnectedTagged<u32, ConnectedDirect>>()
            //             .into_source(),
            //         mapper: Box::new(|k| MemberId::from_raw(k)),
            //         _phantom: Default::default(),
            //     }),
            //     &(),
            // )
        },
    )
}

pub(super) fn deploy_m2m(
    env: RuntimeData<&DeployPorts<HydroMeta>>,
    c1_port: &str,
    c2_port: &str,
) -> (syn::Expr, syn::Expr) {
    (
        {
            q!(sinktools::map(
                |(k, v): (MemberId<()>, Bytes)| { (k.get_raw_id(), v) },
                env.port(c1_port)
                    .connect::<ConnectedDemux<ConnectedDirect>>()
                    .into_sink()
            ))
            .splice_untyped_ctx(&())

            // QuotedWithContext::<'a, MapAdapterTypeHinter<MemberId<Tag>>, ()>::splice_untyped_ctx(
            //     q!(MapAdapterTypeHinter {
            //         sink: env
            //             .port(c1_port)
            //             .connect::<ConnectedDemux<u32, ConnectedDirect>>()
            //             .into_sink(),
            //         mapper: Box::new(|id: MemberId<_>| id.get_raw_id()),
            //         _phantom: Default::default(),
            //     }),
            //     &(),
            // )
        },
        {
            q!({
                env.port(c2_port)
                    .connect::<ConnectedTagged<ConnectedDirect>>()
                    .into_source()
                    .map(|v| v.map(|(k, v)| (MemberId::<()>::from_raw_id(k), v)))
            })
            .splice_untyped_ctx(&())

            // QuotedWithContext::<'a, SourceAdapterTypeHinter<MemberId<Tag>>, ()>::splice_untyped_ctx(
            //     q!(SourceAdapterTypeHinter {
            //         stream: env
            //             .port(c2_port)
            //             .connect::<ConnectedTagged<u32, ConnectedDirect>>()
            //             .into_source(),
            //         mapper: Box::new(|k| MemberId::from_raw(k)),
            //         _phantom: Default::default(),
            //     }),
            //     &(),
            // )
        },
    )
}

pub(super) fn deploy_e2o(
    env: RuntimeData<&DeployPorts<HydroMeta>>,
    _e1_port: &str,
    p2_port: &str,
) -> syn::Expr {
    q!(env.port(p2_port).connect::<ConnectedDirect>().into_source()).splice_untyped_ctx(&())
}

pub(super) fn deploy_o2e(
    env: RuntimeData<&DeployPorts<HydroMeta>>,
    p1_port: &str,
    _e2_port: &str,
) -> syn::Expr {
    q!(env.port(p1_port).connect::<ConnectedDirect>().into_sink()).splice_untyped_ctx(&())
}
