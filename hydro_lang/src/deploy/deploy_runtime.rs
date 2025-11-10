#![allow(
    unused,
    reason = "unused in trybuild but the __staged version is needed"
)]

use std::collections::HashMap;
use std::marker::PhantomData;
use std::pin::Pin;

use bytes::{Bytes, BytesMut};
use dfir_rs::pin_project_lite::pin_project;
use futures::sink::Buffer;
use futures::{Sink, SinkExt, StreamExt};
use hydro_deploy_integration::hacks::{MapAdapterTypeHinter, SourceAdapterTypeHinter};
use hydro_deploy_integration::{
    ConnectedDemux, ConnectedDirect, ConnectedSink, ConnectedSource, ConnectedTagged, DeployPorts,
};
use serde::{Deserialize, Serialize};
use sinktools::demux_map::DemuxMap;
use stageleft::{QuotedWithContext, RuntimeData, q};

use crate::location::cluster::ClusterIds;
use crate::location::dynamic::LocationId;
use crate::location::{MemberId, MembershipEvent};

#[derive(Default, Serialize, Deserialize)]
pub(super) struct HydroMeta {
    pub clusters: HashMap<usize, Vec<u32>>,
    pub cluster_id: Option<u32>,
    pub subgraph_id: usize,
}

pub fn cluster_members(
    cli: RuntimeData<&DeployPorts<u32, HydroMeta>>,
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
    cli: RuntimeData<&DeployPorts<u32, HydroMeta>>,
) -> impl QuotedWithContext<'_, u32, ()> + Copy {
    q!(cli
        .meta
        .cluster_id
        .expect("Tried to read Cluster Self ID on a non-cluster node"))
}

pub fn cluster_membership_stream<'a>(
    location_id: &LocationId,
) -> impl QuotedWithContext<
    'a,
    Box<dyn futures::Stream<Item = (MemberId<()>, MembershipEvent)> + Unpin>,
    (),
> {
    // /// TODO:
    // pub fn legacy_membership_stream(
    //     iter: impl Iterator<Item = MemberId<()>>,
    // ) -> impl futures::Stream<Item = (MemberId<()>, MembershipEvent)> {
    //     futures::stream::iter(iter.map(|k| (MemberId::from_raw(k), MembershipEvent::Joined)))

    //     //     // impl<'a, C: 'a, Ctx> FreeVariableWithContext<Ctx> for ClusterIds<'a, C> {
    //     // //     type O = &'a [MemberId<C>];

    //     // //     fn to_tokens(self, _ctx: &Ctx) -> QuoteTokens
    //     // //     where
    //     // //         Self: Sized,
    //     // //     {
    //     // //         let ident = syn::Ident::new(
    //     // //             &format!("__hydro_lang_cluster_ids_{}", self.id),
    //     // //             Span::call_site(),
    //     // //         );
    //     // //         let root = get_this_crate();
    //     // //         let c_type = quote_type::<C>();

    //     // //         QuoteTokens {
    //     // //             prelude: None,
    //     // //             expr: Some(
    //     // //                 quote! { unsafe { ::std::mem::transmute::<_, &[#root::__staged::location::MemberId<#c_type>]>(#ident) } },
    //     // //             ),
    //     // //         }
    //     // //     }
    //     // // }

    //     // let ident = syn::Ident::new(
    //     //     &format!("__hydro_lang_cluster_ids_{}", location_id),
    //     //     Span::call_site(),
    //     // );

    //     // parse_quote! {
    //     //     #source_ident = source_iter((#ident).iter().cloned().map(|v| (v, MembershipEvent::Joined)));
    //     // }
    // }

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

pub fn deploy_o2o(
    env: RuntimeData<&DeployPorts<u32, HydroMeta>>,
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

// fn Mapper<Tag>((k, v): (MemberId<Tag>, Bytes)) -> (u32, Bytes) {
//     (k.get_raw_id(), v)
// }

// pin_project! {

pub fn makeit<Tag: Unpin>(
    si: DemuxMap<
        u32,
        Pin<Box<Buffer<Pin<Box<dyn Sink<Bytes, Error = std::io::Error> + Send + Sync>>, Bytes>>>,
    >,
) -> MapAdapterTypeHinter<MemberId<Tag>> {
    MapAdapterTypeHinter {
        sink: si,
        mapper: Box::new(|id| id.get_raw_id()),
        _phantom: Default::default(),
    }
}

pub fn deploy_o2m(
    env: RuntimeData<&DeployPorts<u32, HydroMeta>>,
    p1_port: &str,
    c2_port: &str,
) -> (syn::Expr, syn::Expr) {
    (
        {
            q!(sinktools::map(
                |(k, v): (MemberId<()>, Bytes)| { (k.get_raw_id(), v) },
                env.port(p1_port)
                    .connect::<ConnectedDemux<u32, ConnectedDirect>>()
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

pub fn mapper<Tag: Unpin>(id: MemberId<Tag>) -> (u32, ()) {
    (id.get_raw_id(), ())
}

pub fn deploy_m2o(
    env: RuntimeData<&DeployPorts<u32, HydroMeta>>,
    c1_port: &str,
    p2_port: &str,
) -> (syn::Expr, syn::Expr) {
    (
        { q!(env.port(c1_port).connect::<ConnectedDirect>().into_sink()).splice_untyped_ctx(&()) },
        {
            q!({
                env.port(p2_port)
                    .connect::<ConnectedTagged<u32, ConnectedDirect>>()
                    .into_source()
                    .map(|v| v.map(|(k, v)| (MemberId::<()>::from_raw(k), v)))
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

pub fn deploy_m2m(
    env: RuntimeData<&DeployPorts<u32, HydroMeta>>,
    c1_port: &str,
    c2_port: &str,
) -> (syn::Expr, syn::Expr) {
    (
        {
            q!(sinktools::map(
                |(k, v): (MemberId<()>, Bytes)| { (k.get_raw_id(), v) },
                env.port(c1_port)
                    .connect::<ConnectedDemux<u32, ConnectedDirect>>()
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
                    .connect::<ConnectedTagged<u32, ConnectedDirect>>()
                    .into_source()
                    .map(|v| v.map(|(k, v)| (MemberId::<()>::from_raw(k), v)))
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

pub fn deploy_e2o(
    env: RuntimeData<&DeployPorts<u32, HydroMeta>>,
    _e1_port: &str,
    p2_port: &str,
) -> syn::Expr {
    q!(env.port(p2_port).connect::<ConnectedDirect>().into_source()).splice_untyped_ctx(&())
}

pub fn deploy_o2e(
    env: RuntimeData<&DeployPorts<u32, HydroMeta>>,
    p1_port: &str,
    _e2_port: &str,
) -> syn::Expr {
    q!(env.port(p1_port).connect::<ConnectedDirect>().into_sink()).splice_untyped_ctx(&())
}
