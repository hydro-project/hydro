//! "Embedded" deployment backend for Hydro.
//!
//! Instead of compiling each location into a standalone binary, this backend generates
//! a Rust source file containing one function per location. Each function returns a
//! `dfir_rs::scheduled::graph::Dfir` that can be manually driven by the caller.
//!
//! This is useful when you want full control over where and how the projected DFIR
//! code runs (e.g. embedding it into an existing application).
//!
//! # Networking
//!
//! Process and cluster networking (o2o, o2m, m2o, m2m) is supported. When a location has network
//! sends or receives, the generated function takes additional `network_out` and
//! `network_in` parameters whose types are generated structs with one field per
//! network port (named after the channel). Network channels must be named via
//! `.name()` on the networking config.
//!
//! - Sinks (`EmbeddedNetworkOut`): one `FnMut(..)` field per outgoing channel, wrapped in a
//!   [`EmbeddedNetworkOutputChannel`](crate::compile::embedded_runtime::EmbeddedNetworkOutputChannel).
//! - Sources (`EmbeddedNetworkIn`): one `Stream` field per incoming channel, wrapped in a
//!   [`EmbeddedNetworkInputChannel`](crate::compile::embedded_runtime::EmbeddedNetworkInputChannel).
//!
//! Each location's module also has a marker type per channel end (`outgoing::*` and
//! `incoming::*`) implementing
//! [`EmbeddedNetworkChannel`](crate::compile::embedded_runtime::EmbeddedNetworkChannel), which
//! describes the field's channel end: the guarantees of the stream there (the stream sent, or the
//! one delivered) and the channel's network config, which the host agrees to.
//!
//! The exact field types depend on the channel's serialization:
//! - With [`bincode`](crate::networking::Bincode) serialization, Hydro serializes to bytes, so
//!   sinks are `FnMut(Bytes)` and sources are `Stream<Item = Result<BytesMut, io::Error>>`.
//! - With [`embedded`](crate::networking::Embedded) serialization, the raw element type `T` flows
//!   across the channel, so sinks are `FnMut(T)` and sources are `Stream<Item = T>` (with no
//!   transport `Result` — the caller decides how to handle faults).
//!
//! Channels keyed by a cluster member (demux / cluster send) additionally carry a
//! `TaglessMemberId` alongside the payload in these types.
//!
//! The caller is responsible for wiring these together (e.g. via in-memory channels,
//! sockets, etc.). External ports are not supported.

use std::future::Future;
use std::io::Error;
use std::pin::Pin;

use bytes::{Bytes, BytesMut};
use dfir_lang::diagnostic::Diagnostics;
use dfir_lang::graph::{AsCodeOptions, DfirGraph};
use futures::{Sink, Stream};
use proc_macro2::Span;
use quote::quote;
use serde::Serialize;
use serde::de::DeserializeOwned;
use slotmap::SparseSecondaryMap;
use stageleft::{QuotedWithContext, q};

use super::deploy_provider::{ClusterSpec, Deploy, ExternalSpec, Node, ProcessSpec, RegisterPort};
use crate::compile::builder::ExternalPortId;
use crate::location::dynamic::LocationId;
use crate::location::member_id::TaglessMemberId;
use crate::location::{LocationKey, MembershipEvent, NetworkHint};

/// Marker type for the embedded deployment backend.
///
/// All networking methods panic — this backend only supports pure local computation.
pub enum EmbeddedDeploy {}

/// A trivial node type for embedded deployment. Stores a user-provided function name.
#[derive(Clone)]
pub struct EmbeddedNode {
    /// The function name to use in the generated code for this location.
    pub fn_name: String,
    /// The location key for this node, used to register network ports.
    pub location_key: LocationKey,
}

impl Node for EmbeddedNode {
    type Port = ();
    type Meta = ();
    type InstantiateEnv = EmbeddedInstantiateEnv;

    fn next_port(&self) -> Self::Port {}

    fn update_meta(&self, _meta: &Self::Meta) {}

    fn instantiate(
        &self,
        _env: &mut Self::InstantiateEnv,
        _meta: &mut Self::Meta,
        _graph: DfirGraph,
        _extra_stmts: &[syn::Stmt],
        _sidecars: &[syn::Expr],
        _as_code_options: &AsCodeOptions,
    ) {
        // No-op: embedded mode doesn't instantiate nodes at deploy time.
    }
}

impl<'a> RegisterPort<'a, EmbeddedDeploy> for EmbeddedNode {
    fn register(&self, _external_port_id: ExternalPortId, _port: Self::Port) {
        panic!("EmbeddedDeploy does not support external ports");
    }

    #[expect(clippy::manual_async_fn, reason = "false positive, involves lifetimes")]
    fn as_bytes_bidi(
        &self,
        _external_port_id: ExternalPortId,
    ) -> impl Future<
        Output = super::deploy_provider::DynSourceSink<Result<BytesMut, Error>, Bytes, Error>,
    > + 'a {
        async { panic!("EmbeddedDeploy does not support external ports") }
    }

    #[expect(clippy::manual_async_fn, reason = "false positive, involves lifetimes")]
    fn as_bincode_bidi<InT, OutT>(
        &self,
        _external_port_id: ExternalPortId,
    ) -> impl Future<Output = super::deploy_provider::DynSourceSink<OutT, InT, Error>> + 'a
    where
        InT: Serialize + 'static,
        OutT: DeserializeOwned + 'static,
    {
        async { panic!("EmbeddedDeploy does not support external ports") }
    }

    #[expect(clippy::manual_async_fn, reason = "false positive, involves lifetimes")]
    fn as_bincode_sink<T>(
        &self,
        _external_port_id: ExternalPortId,
    ) -> impl Future<Output = Pin<Box<dyn Sink<T, Error = Error>>>> + 'a
    where
        T: Serialize + 'static,
    {
        async { panic!("EmbeddedDeploy does not support external ports") }
    }

    #[expect(clippy::manual_async_fn, reason = "false positive, involves lifetimes")]
    fn as_bincode_source<T>(
        &self,
        _external_port_id: ExternalPortId,
    ) -> impl Future<Output = Pin<Box<dyn Stream<Item = T>>>> + 'a
    where
        T: DeserializeOwned + 'static,
    {
        async { panic!("EmbeddedDeploy does not support external ports") }
    }
}

impl<S: Into<String>> ProcessSpec<'_, EmbeddedDeploy> for S {
    fn build(self, location_key: LocationKey, _name_hint: &str) -> EmbeddedNode {
        EmbeddedNode {
            fn_name: self.into(),
            location_key,
        }
    }
}

impl<S: Into<String>> ClusterSpec<'_, EmbeddedDeploy> for S {
    fn build(self, location_key: LocationKey, _name_hint: &str) -> EmbeddedNode {
        EmbeddedNode {
            fn_name: self.into(),
            location_key,
        }
    }
}

impl<S: Into<String>> ExternalSpec<'_, EmbeddedDeploy> for S {
    fn build(self, location_key: LocationKey, _name_hint: &str) -> EmbeddedNode {
        EmbeddedNode {
            fn_name: self.into(),
            location_key,
        }
    }
}

/// One end of a named network channel at a location.
#[derive(Clone)]
struct EmbeddedNetworkPort {
    name: String,
    /// Whether each element is paired with a `TaglessMemberId` (its destination or sender).
    is_tagged: bool,
    /// The raw payload type of an unserialized `EMBEDDED` channel; `None` for bincode.
    external_type: Option<syn::Type>,
    /// The location at the other end.
    peer: LocationKey,
    /// The channel config's transport and serialization types, and the config written out.
    transport: syn::Type,
    serialization: syn::Type,
    config: String,
}

impl EmbeddedNetworkPort {
    fn new(
        name: &str,
        is_tagged: bool,
        external_type: Option<&syn::Type>,
        peer: LocationKey,
        networking_info: &crate::networking::NetworkingInfo,
    ) -> Self {
        let (transport, serialization, config) =
            channel_config_types(networking_info, external_type.is_some());
        EmbeddedNetworkPort {
            name: name.to_owned(),
            is_tagged,
            external_type: external_type.cloned(),
            peer,
            transport,
            serialization,
            config,
        }
    }
}

/// The `Tr` and `S` types of the network config that `networking_info` describes, as the host's
/// `ChannelConfiguration` names them, and that config written out.
fn channel_config_types(
    networking_info: &crate::networking::NetworkingInfo,
    external: bool,
) -> (syn::Type, syn::Type, String) {
    use crate::compile::ir::{StreamOrder, StreamRetry};
    use crate::networking::{EmbeddedFault, NetworkingInfo, TcpFault, UdpFault};
    let root = crate::staging_util::get_this_crate();
    let fault = |name: &str| -> syn::Type {
        let ident = syn::Ident::new(name, Span::call_site());
        syn::parse_quote!(#root::networking::#ident)
    };
    let transport: syn::Type = match networking_info {
        NetworkingInfo::Tcp { fault: f } => {
            let f = fault(match f {
                TcpFault::FailStop => "FailStop",
                TcpFault::Lossy => "Lossy",
                TcpFault::LossyDelayedForever => "LossyDelayedForever",
            });
            syn::parse_quote!(#root::networking::Tcp<#f>)
        }
        NetworkingInfo::Udp { fault: f } => {
            let f = fault(match f {
                UdpFault::Lossy => "Lossy",
                UdpFault::LossyDelayedForever => "LossyDelayedForever",
            });
            syn::parse_quote!(#root::networking::Udp<#f>)
        }
        NetworkingInfo::Embedded {
            retries,
            ordering,
            fault: f,
        } => {
            let retries: syn::Type = match retries {
                StreamRetry::ExactlyOnce => {
                    syn::parse_quote!(#root::live_collections::stream::ExactlyOnce)
                }
                StreamRetry::AtLeastOnce => {
                    syn::parse_quote!(#root::live_collections::stream::AtLeastOnce)
                }
            };
            let ordering: syn::Type = match ordering {
                StreamOrder::TotalOrder => {
                    syn::parse_quote!(#root::live_collections::stream::TotalOrder)
                }
                StreamOrder::NoOrder => syn::parse_quote!(#root::live_collections::stream::NoOrder),
            };
            let f = fault(match f {
                EmbeddedFault::FailStop => "FailStop",
                EmbeddedFault::Lossy => "Lossy",
                EmbeddedFault::LossyDelayedForever => "LossyDelayedForever",
            });
            syn::parse_quote!(#root::networking::EmbeddedTransport<#retries, #ordering, #f>)
        }
    };
    let serialization: syn::Type = if external {
        syn::parse_quote!(#root::networking::Embedded)
    } else {
        syn::parse_quote!(#root::networking::Bincode)
    };
    (
        transport,
        serialization,
        config_doc(networking_info, external),
    )
}

/// The network config that `networking_info` describes, written out for the docs.
fn config_doc(networking_info: &crate::networking::NetworkingInfo, external: bool) -> String {
    use crate::compile::ir::{StreamOrder, StreamRetry};
    use crate::networking::{EmbeddedFault, NetworkingInfo, TcpFault, UdpFault};
    let serialization = if external { "" } else { ".bincode()" };
    match networking_info {
        NetworkingInfo::Tcp { fault } => {
            let fault = match fault {
                TcpFault::FailStop => ".fail_stop()",
                TcpFault::Lossy => ".lossy(nondet)",
                TcpFault::LossyDelayedForever => ".lossy_delayed_forever()",
            };
            format!("TCP{fault}{serialization}")
        }
        NetworkingInfo::Udp { fault } => {
            let fault = match fault {
                UdpFault::Lossy => ".lossy(nondet)",
                UdpFault::LossyDelayedForever => ".lossy_delayed_forever()",
            };
            format!("UDP{fault}{serialization}")
        }
        NetworkingInfo::Embedded {
            retries,
            ordering,
            fault,
        } => {
            // `EMBEDDED` with a `never_*` or `may_*` declaration for each anomaly.
            let reorder = match ordering {
                StreamOrder::TotalOrder => ".never_reorders()",
                StreamOrder::NoOrder => ".may_reorder()",
            };
            let duplicate = match retries {
                StreamRetry::ExactlyOnce => ".never_duplicates()",
                StreamRetry::AtLeastOnce => ".may_duplicate()",
            };
            let loss = match fault {
                EmbeddedFault::FailStop => ".never_drops()",
                EmbeddedFault::Lossy => ".may_drop(nondet)",
                EmbeddedFault::LossyDelayedForever => ".may_delay_forever()",
            };
            format!("EMBEDDED{reorder}{duplicate}{loss}")
        }
    }
}

/// A stream input or output, by its field name and element type.
#[derive(Clone)]
struct EmbeddedStreamPort {
    ident: syn::Ident,
    element_type: syn::Type,
}

/// Collected embedded input/output registrations, keyed by location.
///
/// During `compile_network`, each `HydroSource::Embedded` and `HydroRoot::EmbeddedOutput`
/// IR node registers its ident, element type, and location key here.
/// `generate_embedded` then uses this to add the appropriate parameters
/// to each generated function.
#[derive(Default)]
pub struct EmbeddedInstantiateEnv {
    inputs: SparseSecondaryMap<LocationKey, Vec<EmbeddedStreamPort>>,
    singleton_inputs: SparseSecondaryMap<LocationKey, Vec<(syn::Ident, syn::Type)>>,
    outputs: SparseSecondaryMap<LocationKey, Vec<EmbeddedStreamPort>>,
    /// Sending ends of network channels.
    network_outputs: SparseSecondaryMap<LocationKey, Vec<EmbeddedNetworkPort>>,
    /// Receiving ends of network channels.
    network_inputs: SparseSecondaryMap<LocationKey, Vec<EmbeddedNetworkPort>>,
    /// The clusters whose membership each location needs.
    membership_streams: SparseSecondaryMap<LocationKey, Vec<LocationKey>>,
    boundary: BoundaryGuarantees,
}

/// The ordering and retries of a stream crossing the boundary.
#[derive(Clone, Debug)]
struct EmbeddedStreamGuarantees {
    order: crate::compile::ir::StreamOrder,
    retry: crate::compile::ir::StreamRetry,
}

impl EmbeddedStreamGuarantees {
    fn of(metadata: &crate::compile::ir::HydroIrMetadata, what: &str) -> Self {
        use crate::compile::ir::CollectionKind;
        match &metadata.collection_kind {
            CollectionKind::Stream { order, retry, .. }
            | CollectionKind::KeyedStream {
                value_order: order,
                value_retry: retry,
                ..
            } => EmbeddedStreamGuarantees {
                order: order.clone(),
                retry: retry.clone(),
            },
            kind => panic!("{what} must be a stream or keyed stream, not {kind:?}"),
        }
    }
}

/// The guarantees of the stream sent on a network channel and of the stream it delivers.
#[derive(Clone, Debug)]
struct EmbeddedNetworkGuarantees {
    sent: EmbeddedStreamGuarantees,
    received: EmbeddedStreamGuarantees,
}

/// The guarantees of every stream crossing the boundary, read from the IR before compiling: only
/// the embedded backend needs them, so they do not go through [`Deploy`].
#[derive(Default)]
struct BoundaryGuarantees {
    inputs: std::collections::HashMap<(LocationKey, String), EmbeddedStreamGuarantees>,
    outputs: std::collections::HashMap<(LocationKey, String), EmbeddedStreamGuarantees>,
    networks: std::collections::HashMap<String, EmbeddedNetworkGuarantees>,
}

impl BoundaryGuarantees {
    fn collect(ir: &mut [crate::compile::ir::HydroRoot]) -> Self {
        use crate::compile::ir::{HydroNode, HydroRoot, HydroSource};
        let location_key = |id: &LocationId, what: &str| match id.root() {
            LocationId::Process(key) | LocationId::Cluster(key) => *key,
            _ => panic!("{what} must be on a process or cluster"),
        };
        let mut boundary = BoundaryGuarantees::default();
        let cell = std::cell::RefCell::new(&mut boundary);
        crate::compile::ir::transform_bottom_up(
            ir,
            &mut |root| {
                if let HydroRoot::EmbeddedOutput { ident, input, .. } = root {
                    let what = "an embedded output";
                    cell.borrow_mut().outputs.insert(
                        (
                            location_key(&input.metadata().location_id, what),
                            ident.to_string(),
                        ),
                        EmbeddedStreamGuarantees::of(input.metadata(), what),
                    );
                }
            },
            &mut |node| match node {
                HydroNode::Source {
                    source: HydroSource::Embedded(ident),
                    metadata,
                } => {
                    let what = "an embedded input";
                    cell.borrow_mut().inputs.insert(
                        (location_key(&metadata.location_id, what), ident.to_string()),
                        EmbeddedStreamGuarantees::of(metadata, what),
                    );
                }
                HydroNode::Network {
                    name: Some(name),
                    input,
                    metadata,
                    ..
                } => {
                    let what = "a network channel's stream";
                    cell.borrow_mut().networks.insert(
                        name.clone(),
                        EmbeddedNetworkGuarantees {
                            sent: EmbeddedStreamGuarantees::of(input.metadata(), what),
                            received: EmbeddedStreamGuarantees::of(metadata, what),
                        },
                    );
                }
                _ => {}
            },
            false,
        );
        boundary
    }

    fn input(&self, location_key: LocationKey, ident: &syn::Ident) -> EmbeddedStreamGuarantees {
        self.inputs[&(location_key, ident.to_string())].clone()
    }

    fn output(&self, location_key: LocationKey, ident: &syn::Ident) -> EmbeddedStreamGuarantees {
        self.outputs[&(location_key, ident.to_string())].clone()
    }

    fn network(&self, name: &str) -> EmbeddedNetworkGuarantees {
        self.networks[name].clone()
    }
}

impl<'a> Deploy<'a> for EmbeddedDeploy {
    type Meta = ();
    type InstantiateEnv = EmbeddedInstantiateEnv;

    type Process = EmbeddedNode;
    type Cluster = EmbeddedNode;
    type External = EmbeddedNode;

    const SUPPORTS_EXTERNAL_SERIALIZATION: bool = true;

    fn o2o_sink_source(
        env: &mut Self::InstantiateEnv,
        p1: &Self::Process,
        _p1_port: &(),
        p2: &Self::Process,
        _p2_port: &(),
        name: Option<&str>,
        networking_info: &crate::networking::NetworkingInfo,
        external_types: Option<(&syn::Type, &syn::Type)>,
    ) -> (syn::Expr, syn::Expr) {
        let name = name.expect(
            "EmbeddedDeploy o2o networking requires a channel name. Use `TCP.name(\"my_channel\")` to provide one.",
        );

        let sink_ident = syn::Ident::new(&format!("__network_out_{name}"), Span::call_site());
        let source_ident = syn::Ident::new(&format!("__network_in_{name}"), Span::call_site());

        env.network_outputs
            .entry(p1.location_key)
            .unwrap()
            .or_default()
            .push(EmbeddedNetworkPort::new(
                name,
                false,
                external_types.map(|(i, _)| i),
                p2.location_key,
                networking_info,
            ));
        env.network_inputs
            .entry(p2.location_key)
            .unwrap()
            .or_default()
            .push(EmbeddedNetworkPort::new(
                name,
                false,
                external_types.map(|(_, o)| o),
                p1.location_key,
                networking_info,
            ));

        (
            syn::parse_quote!(__root_dfir_rs::sinktools::for_each(#sink_ident)),
            syn::parse_quote!(#source_ident),
        )
    }

    fn o2o_connect(
        _p1: &Self::Process,
        _p1_port: &(),
        _p2: &Self::Process,
        _p2_port: &(),
    ) -> Box<dyn FnOnce()> {
        Box::new(|| {})
    }

    fn o2m_sink_source(
        env: &mut Self::InstantiateEnv,
        p1: &Self::Process,
        _p1_port: &(),
        c2: &Self::Cluster,
        _c2_port: &(),
        name: Option<&str>,
        networking_info: &crate::networking::NetworkingInfo,
        external_types: Option<(&syn::Type, &syn::Type)>,
    ) -> (syn::Expr, syn::Expr) {
        let name = name.expect("EmbeddedDeploy o2m networking requires a channel name.");
        let sink_ident = syn::Ident::new(&format!("__network_out_{name}"), Span::call_site());
        let source_ident = syn::Ident::new(&format!("__network_in_{name}"), Span::call_site());
        env.network_outputs
            .entry(p1.location_key)
            .unwrap()
            .or_default()
            .push(EmbeddedNetworkPort::new(
                name,
                true,
                external_types.map(|(i, _)| i),
                c2.location_key,
                networking_info,
            ));
        env.network_inputs
            .entry(c2.location_key)
            .unwrap()
            .or_default()
            .push(EmbeddedNetworkPort::new(
                name,
                false,
                external_types.map(|(_, o)| o),
                p1.location_key,
                networking_info,
            ));
        (
            syn::parse_quote!(__root_dfir_rs::sinktools::for_each(#sink_ident)),
            syn::parse_quote!(#source_ident),
        )
    }

    fn o2m_connect(
        _p1: &Self::Process,
        _p1_port: &(),
        _c2: &Self::Cluster,
        _c2_port: &(),
    ) -> Box<dyn FnOnce()> {
        Box::new(|| {})
    }

    fn m2o_sink_source(
        env: &mut Self::InstantiateEnv,
        c1: &Self::Cluster,
        _c1_port: &(),
        p2: &Self::Process,
        _p2_port: &(),
        name: Option<&str>,
        networking_info: &crate::networking::NetworkingInfo,
        external_types: Option<(&syn::Type, &syn::Type)>,
    ) -> (syn::Expr, syn::Expr) {
        let name = name.expect("EmbeddedDeploy m2o networking requires a channel name.");
        let sink_ident = syn::Ident::new(&format!("__network_out_{name}"), Span::call_site());
        let source_ident = syn::Ident::new(&format!("__network_in_{name}"), Span::call_site());
        env.network_outputs
            .entry(c1.location_key)
            .unwrap()
            .or_default()
            .push(EmbeddedNetworkPort::new(
                name,
                false,
                external_types.map(|(i, _)| i),
                p2.location_key,
                networking_info,
            ));
        env.network_inputs
            .entry(p2.location_key)
            .unwrap()
            .or_default()
            .push(EmbeddedNetworkPort::new(
                name,
                true,
                external_types.map(|(_, o)| o),
                c1.location_key,
                networking_info,
            ));
        (
            syn::parse_quote!(__root_dfir_rs::sinktools::for_each(#sink_ident)),
            syn::parse_quote!(#source_ident),
        )
    }

    fn m2o_connect(
        _c1: &Self::Cluster,
        _c1_port: &(),
        _p2: &Self::Process,
        _p2_port: &(),
    ) -> Box<dyn FnOnce()> {
        Box::new(|| {})
    }

    fn m2m_sink_source(
        env: &mut Self::InstantiateEnv,
        c1: &Self::Cluster,
        _c1_port: &(),
        c2: &Self::Cluster,
        _c2_port: &(),
        name: Option<&str>,
        networking_info: &crate::networking::NetworkingInfo,
        external_types: Option<(&syn::Type, &syn::Type)>,
    ) -> (syn::Expr, syn::Expr) {
        let name = name.expect("EmbeddedDeploy m2m networking requires a channel name.");
        let sink_ident = syn::Ident::new(&format!("__network_out_{name}"), Span::call_site());
        let source_ident = syn::Ident::new(&format!("__network_in_{name}"), Span::call_site());
        env.network_outputs
            .entry(c1.location_key)
            .unwrap()
            .or_default()
            .push(EmbeddedNetworkPort::new(
                name,
                true,
                external_types.map(|(i, _)| i),
                c2.location_key,
                networking_info,
            ));
        env.network_inputs
            .entry(c2.location_key)
            .unwrap()
            .or_default()
            .push(EmbeddedNetworkPort::new(
                name,
                true,
                external_types.map(|(_, o)| o),
                c1.location_key,
                networking_info,
            ));
        (
            syn::parse_quote!(__root_dfir_rs::sinktools::for_each(#sink_ident)),
            syn::parse_quote!(#source_ident),
        )
    }

    fn m2m_connect(
        _c1: &Self::Cluster,
        _c1_port: &(),
        _c2: &Self::Cluster,
        _c2_port: &(),
    ) -> Box<dyn FnOnce()> {
        Box::new(|| {})
    }

    fn e2o_many_source(
        _extra_stmts: &mut Vec<syn::Stmt>,
        _p2: &Self::Process,
        _p2_port: &(),
        _codec_type: &syn::Type,
        _shared_handle: String,
    ) -> syn::Expr {
        panic!("EmbeddedDeploy does not support networking (e2o)")
    }

    fn e2o_many_sink(_shared_handle: String) -> syn::Expr {
        panic!("EmbeddedDeploy does not support networking (e2o)")
    }

    fn e2o_source(
        _extra_stmts: &mut Vec<syn::Stmt>,
        _p1: &Self::External,
        _p1_port: &(),
        _p2: &Self::Process,
        _p2_port: &(),
        _codec_type: &syn::Type,
        _shared_handle: String,
    ) -> syn::Expr {
        panic!("EmbeddedDeploy does not support networking (e2o)")
    }

    fn e2o_connect(
        _p1: &Self::External,
        _p1_port: &(),
        _p2: &Self::Process,
        _p2_port: &(),
        _many: bool,
        _server_hint: NetworkHint,
    ) -> Box<dyn FnOnce()> {
        panic!("EmbeddedDeploy does not support networking (e2o)")
    }

    fn o2e_sink(
        _p1: &Self::Process,
        _p1_port: &(),
        _p2: &Self::External,
        _p2_port: &(),
        _shared_handle: String,
    ) -> syn::Expr {
        panic!("EmbeddedDeploy does not support networking (o2e)")
    }

    #[expect(
        unreachable_code,
        reason = "panic before q! which is only for return type"
    )]
    fn cluster_ids(
        _of_cluster: LocationKey,
    ) -> impl QuotedWithContext<'a, &'a [TaglessMemberId], ()> + Clone + 'a {
        panic!("EmbeddedDeploy does not support cluster IDs");
        q!(unreachable!("EmbeddedDeploy does not support cluster IDs"))
    }

    fn cluster_self_id() -> impl QuotedWithContext<'a, TaglessMemberId, ()> + Clone + 'a {
        super::embedded_runtime::embedded_cluster_self_id()
    }

    fn cluster_membership_stream(
        env: &mut Self::InstantiateEnv,
        at_location: &LocationId,
        location_id: &LocationId,
    ) -> impl QuotedWithContext<'a, Box<dyn Stream<Item = (TaglessMemberId, MembershipEvent)> + Unpin>, ()>
    {
        let at_key = match at_location {
            LocationId::Process(key) | LocationId::Cluster(key) => *key,
            _ => panic!("cluster_membership_stream must be called from a process or cluster"),
        };
        let cluster_key = match location_id {
            LocationId::Cluster(key) => *key,
            _ => panic!("cluster_membership_stream target must be a cluster"),
        };
        let vec = env.membership_streams.entry(at_key).unwrap().or_default();
        let idx = if let Some(pos) = vec.iter().position(|k| *k == cluster_key) {
            pos
        } else {
            vec.push(cluster_key);
            vec.len() - 1
        };

        super::embedded_runtime::embedded_cluster_membership_stream(idx)
    }

    fn register_embedded_stream_input(
        env: &mut Self::InstantiateEnv,
        location_key: LocationKey,
        ident: &syn::Ident,
        element_type: &syn::Type,
    ) {
        env.inputs
            .entry(location_key)
            .unwrap()
            .or_default()
            .push(EmbeddedStreamPort {
                ident: ident.clone(),
                element_type: element_type.clone(),
            });
    }

    fn register_embedded_singleton_input(
        env: &mut Self::InstantiateEnv,
        location_key: LocationKey,
        ident: &syn::Ident,
        element_type: &syn::Type,
    ) {
        env.singleton_inputs
            .entry(location_key)
            .unwrap()
            .or_default()
            .push((ident.clone(), element_type.clone()));
    }

    fn register_embedded_output(
        env: &mut Self::InstantiateEnv,
        location_key: LocationKey,
        ident: &syn::Ident,
        element_type: &syn::Type,
    ) {
        env.outputs
            .entry(location_key)
            .unwrap()
            .or_default()
            .push(EmbeddedStreamPort {
                ident: ident.clone(),
                element_type: element_type.clone(),
            });
    }
}

/// The generated pieces for one direction of a location's network channels: the
/// `EmbeddedNetworkIn` or `EmbeddedNetworkOut` struct, its module of marker types, and the
/// function parameter. Both directions are generated the same way, one piece per port.
struct NetworkPortTokens {
    fields: Vec<syn::Ident>,
    field_docs: Vec<String>,
    /// Each field's type: `EmbeddedNetworkInputChannel<Marker, S>` (receiving) or `EmbeddedNetworkOutputChannel<Marker, F>`
    /// (sending), for the port's marker type (which carries its stream's guarantees and the
    /// channel's configuration) and generic parameter.
    field_types: Vec<proc_macro2::TokenStream>,
    generics: Vec<syn::Ident>,
    items: Vec<proc_macro2::TokenStream>,
    marker_defs: Vec<proc_macro2::TokenStream>,
}

impl NetworkPortTokens {
    fn new(
        ports: &[EmbeddedNetworkPort],
        boundary: &BoundaryGuarantees,
        fn_names: &SparseSecondaryMap<LocationKey, &str>,
        generic_prefix: &str,
        sending: bool,
        item: impl Fn(&EmbeddedNetworkPort) -> proc_macro2::TokenStream,
    ) -> Self {
        let root = crate::staging_util::get_this_crate();
        let items: Vec<proc_macro2::TokenStream> = ports.iter().map(&item).collect();
        // Each port's field, which its marker type is named after.
        let fields: Vec<syn::Ident> = ports
            .iter()
            .map(|port| syn::Ident::new(&port.name, Span::call_site()))
            .collect();
        let mut marker_defs = Vec::with_capacity(ports.len());
        let mut field_docs = Vec::with_capacity(ports.len());
        for (port, marker) in ports.iter().zip(&fields) {
            let name = &port.name;
            let peer = fn_names[port.peer];
            let channel = boundary.network(name);
            let guarantees = if sending {
                &channel.sent
            } else {
                &channel.received
            };
            let doc = format!(
                "The `{name}` channel's end at this location; its other end is at `{peer}`.\n\n{}",
                network_end_doc(port, guarantees, sending)
            );
            let marker_def = marker_def(&root, marker, &doc, guarantees);
            let (transport, serialization) = (&port.transport, &port.serialization);
            marker_defs.push(quote! {
                #marker_def

                impl #root::compile::embedded_runtime::EmbeddedNetworkChannel for #marker {
                    type Transport = #transport;
                    type Serialization = #serialization;
                }
            });
            let (action, module) = if sending {
                (format!("Sends on `{name}` to `{peer}`"), "outgoing")
            } else {
                (format!("Receives `{name}` from `{peer}`"), "incoming")
            };
            field_docs.push(format!("{action} (marker type: [`{module}::{marker}`])."));
        }
        let generics: Vec<syn::Ident> = (0..ports.len())
            .map(|i| quote::format_ident!("{}{}", generic_prefix, i))
            .collect();
        let module = syn::Ident::new(
            if sending { "outgoing" } else { "incoming" },
            Span::call_site(),
        );
        // The host acknowledges the channel's configuration at both ends. At the sending end it
        // acknowledges the stream sent, as for an output; at the receiving end it promises the
        // stream delivered, as for an input.
        let wrapper = if sending {
            quote! { #root::compile::embedded_runtime::EmbeddedNetworkOutputChannel }
        } else {
            quote! { #root::compile::embedded_runtime::EmbeddedNetworkInputChannel }
        };
        let field_types = fields
            .iter()
            .zip(&generics)
            .map(|(marker, generic)| quote! { #wrapper<#module::#marker, #generic> })
            .collect();
        NetworkPortTokens {
            fields,
            field_docs,
            field_types,
            generics,
            items,
            marker_defs,
        }
    }
}

/// The marker type of a field crossing the boundary, implementing `EmbeddedChannel` with its
/// `guarantees`. Channels are named by strings, so each field gets a type, named after it, for
/// the type checker to check the host's side against.
fn marker_def(
    root: &proc_macro2::TokenStream,
    marker: &syn::Ident,
    doc: &str,
    guarantees: &EmbeddedStreamGuarantees,
) -> proc_macro2::TokenStream {
    let ordering = match guarantees.order {
        crate::compile::ir::StreamOrder::TotalOrder => {
            quote! { #root::live_collections::stream::TotalOrder }
        }
        crate::compile::ir::StreamOrder::NoOrder => {
            quote! { #root::live_collections::stream::NoOrder }
        }
    };
    let retries = match guarantees.retry {
        crate::compile::ir::StreamRetry::ExactlyOnce => {
            quote! { #root::live_collections::stream::ExactlyOnce }
        }
        crate::compile::ir::StreamRetry::AtLeastOnce => {
            quote! { #root::live_collections::stream::AtLeastOnce }
        }
    };
    quote! {
        #[doc = #doc]
        #[allow(non_camel_case_types, reason = "named after its field")]
        pub struct #marker;

        impl #root::compile::embedded_runtime::EmbeddedChannel for #marker {
            type Ordering = #ordering;
            type Retries = #retries;
        }
    }
}

/// How a host promises `guarantees` for a stream flowing into the dataflow, in words.
fn promise_doc(guarantees: &EmbeddedStreamGuarantees, wrapper: &str) -> String {
    format!(
        "Promise its guarantees with `{wrapper}::new(.., Promises::new(){})`.",
        promises_chain(guarantees)
    )
}

/// The `Promises` builder calls that promise exactly `guarantees`.
fn promises_chain(guarantees: &EmbeddedStreamGuarantees) -> String {
    use crate::compile::ir::{StreamOrder, StreamRetry};
    [
        (guarantees.order == StreamOrder::TotalOrder).then_some(".total_order(proof)"),
        (guarantees.retry == StreamRetry::ExactlyOnce).then_some(".exactly_once(proof)"),
    ]
    .into_iter()
    .flatten()
    .collect()
}

/// How a host acknowledges what an output with `guarantees` may do, in words.
fn acknowledge_doc(guarantees: &EmbeddedStreamGuarantees) -> String {
    format!(
        "Acknowledge what it may do with \
         `EmbeddedOutputChannel::new(.., Acknowledgements::new(){})`.",
        acknowledgements_chain(guarantees)
    )
}

/// The `Acknowledgements` calls for what a stream with `guarantees` may do.
fn acknowledgements_chain(guarantees: &EmbeddedStreamGuarantees) -> String {
    use crate::compile::ir::{StreamOrder, StreamRetry};
    [
        (guarantees.order == StreamOrder::NoOrder).then_some(".no_order()"),
        (guarantees.retry == StreamRetry::AtLeastOnce).then_some(".at_least_once()"),
    ]
    .into_iter()
    .flatten()
    .collect()
}

/// How the host builds the channel end `port`, for its marker's docs.
fn network_end_doc(
    port: &EmbeddedNetworkPort,
    guarantees: &EmbeddedStreamGuarantees,
    sending: bool,
) -> String {
    let config = &port.config;
    if sending {
        format!(
            "Acknowledge the channel's configuration and what the stream sent on it may do with \
             `EmbeddedNetworkOutputChannel::new(.., ChannelConfiguration::new({config}), \
             Acknowledgements::new(){})`.",
            acknowledgements_chain(guarantees)
        )
    } else {
        format!(
            "Acknowledge the channel's configuration and promise the guarantees of the stream it \
             delivers with `EmbeddedNetworkInputChannel::new(.., ChannelConfiguration::new({config}), \
             Promises::new(){})`.",
            promises_chain(guarantees)
        )
    }
}

/// Options controlling the Rust source emitted by
/// [`DeployFlow::generate_embedded_with_options`](super::deploy::DeployFlow::generate_embedded_with_options).
///
/// This is deliberately *not* the full set of DFIR codegen knobs ([`AsCodeOptions`]). Most of
/// those are decided by Hydro on the user's behalf: type guards are excluded because Hydro has
/// already type-checked the dataflow and injects its own type hints, the serialized meta graph is
/// excluded because nothing consumes it in embedded mode, and runtime metrics tracking is driven by
/// [sidecars](crate::telemetry::Sidecar) rather than a raw flag. Only the choices that genuinely
/// belong to the embedding developer are exposed here, which also keeps the embedded API stable
/// as the underlying DFIR options evolve.
#[derive(Clone, Debug, Default)]
#[non_exhaustive]
pub struct EmbeddedCodegenOptions {
    /// Emit a named wrapper function around every DFIR operator so that each operator gets its own
    /// stack frame in profiles/flamegraphs.
    ///
    /// Off by default: the wrappers are purely for profiler attribution (Hydro already type-checks
    /// the dataflow), and they make the generated code several times more verbose. Enable this
    /// when profiling an embedded Hydro program.
    pub require_flame_attribution: bool,
}

impl EmbeddedCodegenOptions {
    /// Returns options with [`Self::require_flame_attribution`] set.
    pub fn with_flame_attribution() -> Self {
        Self {
            require_flame_attribution: true,
        }
    }
}

impl super::deploy::DeployFlow<'_, EmbeddedDeploy> {
    /// Generates a `syn::File` containing one function per location in the flow.
    ///
    /// Each generated function has the signature:
    /// ```ignore
    /// pub fn <fn_name>() -> dfir_rs::scheduled::graph::Dfir<'static>
    /// ```
    /// where `fn_name` is the `String` passed to `with_process` / `with_cluster`.
    ///
    /// The returned `Dfir` can be manually executed by the caller.
    ///
    /// Uses the default [`EmbeddedCodegenOptions`]; see
    /// [`Self::generate_embedded_with_options`] to customize them.
    ///
    /// # Arguments
    ///
    /// * `crate_name` — the name of the crate containing the Hydro program (used for stageleft
    ///   re-exports). Hyphens will be replaced with underscores.
    ///
    /// # Usage
    ///
    /// Typically called from a `build.rs` in a wrapper crate:
    /// ```ignore
    /// // build.rs
    /// let deploy = flow.with_process(&process, "my_fn".to_string());
    /// let code = deploy.generate_embedded("my_hydro_crate");
    /// let out_dir = std::env::var("OUT_DIR").unwrap();
    /// std::fs::write(format!("{out_dir}/embedded.rs"), prettyplease::unparse(&code)).unwrap();
    /// ```
    ///
    /// Then in `lib.rs`:
    /// ```ignore
    /// include!(concat!(env!("OUT_DIR"), "/embedded.rs"));
    /// ```
    pub fn generate_embedded(self, crate_name: &str) -> syn::File {
        self.generate_embedded_with_options(crate_name, &EmbeddedCodegenOptions::default())
    }

    /// Like [`Self::generate_embedded`], but with explicit [`EmbeddedCodegenOptions`].
    pub fn generate_embedded_with_options(
        mut self,
        crate_name: &str,
        codegen_options: &EmbeddedCodegenOptions,
    ) -> syn::File {
        let mut env = EmbeddedInstantiateEnv {
            boundary: BoundaryGuarantees::collect(&mut self.ir),
            ..Default::default()
        };
        let compiled = self.compile_internal(&mut env);

        let root = crate::staging_util::get_this_crate();
        let orig_crate_name = quote::format_ident!("{}", crate_name.replace('-', "_"));

        let mut items: Vec<syn::Item> = Vec::new();

        // Sort location keys for deterministic output.
        let mut location_keys: Vec<_> = compiled.all_dfir().keys().collect();
        location_keys.sort();

        // Build a map from location key to fn_name for lookups.
        let fn_names: SparseSecondaryMap<LocationKey, &str> = location_keys
            .iter()
            .map(|&k| {
                let name = self
                    .processes
                    .get(k)
                    .map(|n| n.fn_name.as_str())
                    .or_else(|| self.clusters.get(k).map(|n| n.fn_name.as_str()))
                    .or_else(|| self.externals.get(k).map(|n| n.fn_name.as_str()))
                    .expect("location key not found in any node map");
                (k, name)
            })
            .collect();

        for location_key in location_keys {
            let graph = compiled.all_dfir()[location_key]
                .as_ref()
                .unwrap_or_else(|err| {
                    panic!(
                        "Failed to partition DFIR graph for location {location_key}: {}",
                        err.diagnostic
                    )
                });

            // Get the user-provided function name from the node.
            let fn_name = fn_names[location_key];
            let fn_ident = syn::Ident::new(fn_name, Span::call_site());

            // Get inputs for this location, sorted by name.
            let mut loc_inputs = env.inputs.get(location_key).cloned().unwrap_or_default();
            loc_inputs.sort_by(|a, b| a.ident.cmp(&b.ident));

            // Get outputs for this location, sorted by name.
            let mut loc_outputs = env.outputs.get(location_key).cloned().unwrap_or_default();
            loc_outputs.sort_by(|a, b| a.ident.cmp(&b.ident));

            let mut diagnostics = Diagnostics::new();
            // `as_code_options` is sparse: it only has entries for locations where a sidecar
            // edited the options. Locations without sidecars fall back to the defaults.
            let mut as_code_options = compiled
                .as_code_options
                .get(location_key)
                .cloned()
                .unwrap_or_default();
            // Hydro has already type-checked the dataflow and injects its own type hints into
            // the DFIR it emits, so the DFIR type guards are redundant here. Excluding them keeps
            // the generated (and user-visible) embedded code compact and reviewable. The
            // per-operator wrappers can still be requested for flamegraph attribution.
            as_code_options.exclude_type_guards = true;
            as_code_options.require_flame_attribution = codegen_options.require_flame_attribution;
            // The serialized meta graph + diagnostics JSON blobs are only used for runtime
            // introspection / debugging of hand-written DFIR; embedded Hydro programs have no use
            // for them, and they are large opaque string literals.
            as_code_options.exclude_meta = true;
            let dfir_tokens = graph
                .as_code_with_options(
                    &quote! { __root_dfir_rs },
                    &as_code_options,
                    quote!(),
                    &mut diagnostics,
                )
                .expect("DFIR inline code generation failed with diagnostics.");

            // --- Build module items (cluster info, output struct, network structs) ---
            let mut mod_items: Vec<proc_macro2::TokenStream> = Vec::new();
            let mut extra_fn_generics: Vec<proc_macro2::TokenStream> = Vec::new();
            let mut cluster_params: Vec<proc_macro2::TokenStream> = Vec::new();
            let mut output_params: Vec<proc_macro2::TokenStream> = Vec::new();
            let mut net_out_params: Vec<proc_macro2::TokenStream> = Vec::new();
            let mut net_in_params: Vec<proc_macro2::TokenStream> = Vec::new();
            let mut extra_destructure: Vec<proc_macro2::TokenStream> = Vec::new();

            // For cluster locations, add self_id parameter.
            if self.clusters.contains_key(location_key) {
                cluster_params.push(quote! {
                    __cluster_self_id: &'a #root::location::member_id::TaglessMemberId
                });
                // Alias to the name the generated DFIR code expects.
                let self_id_ident = syn::Ident::new(
                    &format!("__hydro_lang_cluster_self_id_{}", location_key),
                    Span::call_site(),
                );
                extra_destructure.push(quote! {
                    let #self_id_ident = __cluster_self_id;
                });
            }

            // For any location that needs cluster membership streams, add parameters.
            if let Some(loc_memberships) = env.membership_streams.get(location_key) {
                let membership_struct_ident =
                    syn::Ident::new("EmbeddedMembershipStreams", Span::call_site());

                let mem_generic_idents: Vec<syn::Ident> = loc_memberships
                    .iter()
                    .enumerate()
                    .map(|(i, _)| quote::format_ident!("__Mem{}", i))
                    .collect();

                let mem_field_names: Vec<syn::Ident> = loc_memberships
                    .iter()
                    .map(|k| {
                        let cluster_fn_name = fn_names[*k];
                        syn::Ident::new(cluster_fn_name, Span::call_site())
                    })
                    .collect();

                let mem_item = quote! {
                    (#root::location::member_id::TaglessMemberId, #root::location::MembershipEvent)
                };
                // Each member's events, in the order they happened, each once.
                let mem_guarantees = EmbeddedStreamGuarantees {
                    order: crate::compile::ir::StreamOrder::TotalOrder,
                    retry: crate::compile::ir::StreamRetry::ExactlyOnce,
                };
                let mem_marker_defs = mem_field_names.iter().map(|field| {
                    marker_def(
                        &root,
                        field,
                        &format!(
                            "The membership stream of the `{field}` cluster.\n\n{}",
                            promise_doc(&mem_guarantees, "EmbeddedInputChannel")
                        ),
                        &mem_guarantees,
                    )
                });

                let struct_fields: Vec<proc_macro2::TokenStream> = mem_field_names
                    .iter()
                    .zip(mem_generic_idents.iter())
                    .map(|(field, generic)| {
                        let doc = format!(
                            "Each member's `Joined` / `Left` events (marker type: [`membership::{field}`])."
                        );
                        quote! {
                            #[doc = #doc]
                            pub #field: #root::compile::embedded_runtime::EmbeddedInputChannel<membership::#field, #generic>
                        }
                    })
                    .collect();

                let struct_generics: Vec<proc_macro2::TokenStream> = mem_generic_idents
                    .iter()
                    .map(|generic| quote! { #generic: __root_dfir_rs::futures::Stream<Item = #mem_item> + Unpin })
                    .collect();

                for generic in &mem_generic_idents {
                    extra_fn_generics.push(
                        quote! { #generic: __root_dfir_rs::futures::Stream<Item = #mem_item> + Unpin + 'a },
                    );
                }

                cluster_params.push(quote! {
                    __membership: #fn_ident::#membership_struct_ident<#(#mem_generic_idents),*>
                });

                for (i, field) in mem_field_names.iter().enumerate() {
                    let var_ident =
                        syn::Ident::new(&format!("__membership_{}", i), Span::call_site());
                    extra_destructure.push(quote! {
                        let #var_ident = __membership.#field.into_inner();
                    });
                }

                mod_items.push(quote! {
                    pub struct #membership_struct_ident<#(#struct_generics),*> {
                        #(#struct_fields),*
                    }

                    /// Marker types for each membership stream, describing each field of
                    /// `EmbeddedMembershipStreams`.
                    pub mod membership {
                        use super::*;
                        #(#mem_marker_defs)*
                    }
                });
            }

            // Embedded inputs, passed by value in one struct with a named field per input: a
            // `Stream` for each stream input and a plain value for each singleton input.
            let mut loc_singleton_inputs = env
                .singleton_inputs
                .get(location_key)
                .cloned()
                .unwrap_or_default();
            loc_singleton_inputs.sort_by(|a, b| a.0.cmp(&b.0));

            let mut input_params: Vec<proc_macro2::TokenStream> = Vec::new();
            if !loc_inputs.is_empty() || !loc_singleton_inputs.is_empty() {
                let input_struct_ident = syn::Ident::new("EmbeddedInputs", Span::call_site());

                let input_generic_idents: Vec<syn::Ident> = loc_inputs
                    .iter()
                    .enumerate()
                    .map(|(i, _)| quote::format_ident!("__In{}", i))
                    .collect();

                // Every stream input must promise what its `embedded_input` declared, which its
                // marker's guarantees state.
                let input_marker_defs: Vec<proc_macro2::TokenStream> = loc_inputs
                    .iter()
                    .map(|input| {
                        let guarantees = env.boundary.input(location_key, &input.ident);
                        marker_def(
                            &root,
                            &input.ident,
                            &format!(
                                "The `{}` stream input.\n\n{}",
                                input.ident,
                                promise_doc(&guarantees, "EmbeddedInputChannel")
                            ),
                            &guarantees,
                        )
                    })
                    .collect();

                let struct_fields: Vec<proc_macro2::TokenStream> = loc_inputs
                    .iter()
                    .zip(input_generic_idents.iter())
                    .map(|(input, generic)| {
                        let ident = &input.ident;
                        let doc =
                            format!("The `{ident}` stream input (marker type: [`inputs::{ident}`]).");
                        quote! {
                            #[doc = #doc]
                            pub #ident: #root::compile::embedded_runtime::EmbeddedInputChannel<inputs::#ident, #generic>
                        }
                    })
                    .chain(
                        loc_singleton_inputs
                            .iter()
                            .map(|(ident, element_type)| quote! { pub #ident: #element_type }),
                    )
                    .collect();

                let struct_generics: Vec<proc_macro2::TokenStream> = loc_inputs
                    .iter()
                    .zip(input_generic_idents.iter())
                    .map(|(input, generic)| {
                        let element_type = &input.element_type;
                        quote! { #generic: __root_dfir_rs::futures::Stream<Item = #element_type> + Unpin }
                    })
                    .collect();

                for (input, generic) in loc_inputs.iter().zip(input_generic_idents.iter()) {
                    let element_type = &input.element_type;
                    extra_fn_generics.push(
                        quote! { #generic: __root_dfir_rs::futures::Stream<Item = #element_type> + Unpin + 'a },
                    );
                }

                input_params.push(quote! {
                    __inputs: #fn_ident::#input_struct_ident<#(#input_generic_idents),*>
                });

                for input in &loc_inputs {
                    let ident = &input.ident;
                    extra_destructure.push(quote! { let #ident = __inputs.#ident.into_inner(); });
                }
                for (ident, _) in &loc_singleton_inputs {
                    extra_destructure.push(quote! { let #ident = __inputs.#ident; });
                }

                let input_markers_mod = (!input_marker_defs.is_empty()).then(|| {
                    quote! {
                        /// Marker types for each stream input, describing each stream field of
                        /// `EmbeddedInputs`.
                        pub mod inputs {
                            use super::*;
                            #(#input_marker_defs)*
                        }
                    }
                });
                mod_items.push(quote! {
                    pub struct #input_struct_ident<#(#struct_generics),*> {
                        #(#struct_fields),*
                    }

                    #input_markers_mod
                });
            }

            // Embedded outputs: one `EmbeddedOutputChannel` sink per output, over a marker whose guarantees
            // are what the output stream provides, which the host's acknowledgements must cover.
            if !loc_outputs.is_empty() {
                let output_struct_ident = syn::Ident::new("EmbeddedOutputs", Span::call_site());

                let mut struct_generics: Vec<proc_macro2::TokenStream> = Vec::new();
                let mut struct_fields: Vec<proc_macro2::TokenStream> = Vec::new();
                let mut generics: Vec<syn::Ident> = Vec::new();
                let mut marker_defs: Vec<proc_macro2::TokenStream> = Vec::new();
                for (i, output) in loc_outputs.iter().enumerate() {
                    let ident = &output.ident;
                    let element_type = &output.element_type;
                    let generic = quote::format_ident!("__Out{}", i);
                    let guarantees = env.boundary.output(location_key, ident);
                    marker_defs.push(marker_def(
                        &root,
                        ident,
                        &format!("The `{ident}` output.\n\n{}", acknowledge_doc(&guarantees)),
                        &guarantees,
                    ));
                    let doc = format!("The `{ident}` output (marker type: [`outputs::{ident}`]).");
                    let sink_bound = quote! { FnMut(#element_type) };
                    struct_generics.push(quote! { #generic: #sink_bound });
                    extra_fn_generics.push(quote! { #generic: #sink_bound + 'a });
                    struct_fields.push(quote! {
                        #[doc = #doc]
                        pub #ident: #root::compile::embedded_runtime::EmbeddedOutputChannel<outputs::#ident, #generic>
                    });
                    generics.push(generic);
                    extra_destructure
                        .push(quote! { let mut #ident = __outputs.#ident.inner_mut(); });
                }

                output_params.push(quote! {
                    __outputs: &'a mut #fn_ident::#output_struct_ident<#(#generics),*>
                });

                mod_items.push(quote! {
                    pub struct #output_struct_ident<#(#struct_generics),*> {
                        #(#struct_fields),*
                    }

                    /// Marker types for each output, describing each field of `EmbeddedOutputs`.
                    pub mod outputs {
                        use super::*;
                        #(#marker_defs)*
                    }
                });
            }

            // Network outputs (sinks), plus a marker type per channel describing it.
            if let Some(mut ports) = env.network_outputs.remove(location_key) {
                ports.sort_by(|a, b| a.name.cmp(&b.name));
                let channels = NetworkPortTokens::new(
                    &ports,
                    &env.boundary,
                    &fn_names,
                    "__NetOut",
                    true,
                    |port| {
                        let payload = if let Some(ty) = &port.external_type {
                            quote! { #ty }
                        } else {
                            quote! { #root::runtime_support::dfir_rs::bytes::Bytes }
                        };
                        if port.is_tagged {
                            quote! { (#root::location::member_id::TaglessMemberId, #payload) }
                        } else {
                            payload
                        }
                    },
                );
                let NetworkPortTokens {
                    fields,
                    field_docs,
                    field_types,
                    generics,
                    items,
                    marker_defs,
                } = &channels;

                for (generic, item) in generics.iter().zip(items) {
                    extra_fn_generics.push(quote! { #generic: FnMut(#item) + 'a });
                }

                net_out_params.push(quote! {
                    __network_out: &'a mut #fn_ident::EmbeddedNetworkOut<#(#generics),*>
                });

                for (port, field) in ports.iter().zip(fields) {
                    let var_ident =
                        syn::Ident::new(&format!("__network_out_{}", port.name), Span::call_site());
                    extra_destructure
                        .push(quote! { let mut #var_ident = __network_out.#field.inner_mut(); });
                }

                mod_items.push(quote! {
                    pub struct EmbeddedNetworkOut<#(#generics: FnMut(#items)),*> {
                        #(#[doc = #field_docs] pub #fields: #field_types),*
                    }

                    /// Marker types for the sending end of each of this location's outgoing
                    /// channels, describing each field of `EmbeddedNetworkOut`.
                    pub mod outgoing {
                        use super::*;
                        #(#marker_defs)*
                    }
                });
            }

            // Network inputs (Stream sources), plus a marker type per channel describing it.
            if let Some(mut ports) = env.network_inputs.remove(location_key) {
                ports.sort_by(|a, b| a.name.cmp(&b.name));
                let channels = NetworkPortTokens::new(
                    &ports,
                    &env.boundary,
                    &fn_names,
                    "__NetIn",
                    false,
                    |port| {
                        // With embedded (external) serialization, the raw payload is delivered
                        // directly, with no transport `Result` wrapper.
                        match (&port.external_type, port.is_tagged) {
                            (Some(ty), true) => {
                                quote! { (#root::location::member_id::TaglessMemberId, #ty) }
                            }
                            (Some(ty), false) => quote! { #ty },
                            (None, true) => {
                                quote! { Result<(#root::location::member_id::TaglessMemberId, __root_dfir_rs::bytes::BytesMut), std::io::Error> }
                            }
                            (None, false) => {
                                quote! { Result<__root_dfir_rs::bytes::BytesMut, std::io::Error> }
                            }
                        }
                    },
                );
                let NetworkPortTokens {
                    fields,
                    field_docs,
                    field_types,
                    generics,
                    items,
                    marker_defs,
                } = &channels;

                for (generic, item) in generics.iter().zip(items) {
                    extra_fn_generics.push(
                        quote! { #generic: __root_dfir_rs::futures::Stream<Item = #item> + Unpin + 'a },
                    );
                }

                net_in_params.push(quote! {
                    __network_in: #fn_ident::EmbeddedNetworkIn<#(#generics),*>
                });

                for (port, field) in ports.iter().zip(fields) {
                    let var_ident =
                        syn::Ident::new(&format!("__network_in_{}", port.name), Span::call_site());
                    extra_destructure
                        .push(quote! { let #var_ident = __network_in.#field.into_inner(); });
                }

                mod_items.push(quote! {
                    pub struct EmbeddedNetworkIn<#(#generics: __root_dfir_rs::futures::Stream<Item = #items> + Unpin),*> {
                        #(#[doc = #field_docs] pub #fields: #field_types),*
                    }

                    /// Marker types for the receiving end of each of this location's incoming
                    /// channels, describing each field of `EmbeddedNetworkIn`.
                    pub mod incoming {
                        use super::*;
                        #(#marker_defs)*
                    }
                });
            }

            // Emit the location's module: its parameter structs and channel marker types.
            let output_mod: syn::Item = syn::parse_quote! {
                pub mod #fn_ident {
                    use super::*;

                    #(#mod_items)*
                }
            };
            items.push(output_mod);

            // Build the function.
            let all_params: Vec<proc_macro2::TokenStream> = cluster_params
                .into_iter()
                .chain(input_params)
                .chain(output_params)
                .chain(net_in_params)
                .chain(net_out_params)
                .collect();

            let dfir_metrics_ty = crate::compile::dfir_metrics_state_type(&as_code_options, &root);
            let ret_type: syn::Type = syn::parse_quote! { #root::runtime_support::dfir_rs::scheduled::context::Dfir<impl #root::runtime_support::dfir_rs::scheduled::context::TickClosure<Metrics = #dfir_metrics_ty> + 'a> };

            let func = if !extra_fn_generics.is_empty() {
                syn::parse_quote! {
                    #[allow(unused, non_snake_case, clippy::suspicious_else_formatting)]
                    pub fn #fn_ident<'a, #(#extra_fn_generics),*>(#(#all_params),*) -> #ret_type {
                        #(#extra_destructure)*
                        #dfir_tokens
                    }
                }
            } else {
                syn::parse_quote! {
                    #[allow(unused, non_snake_case, clippy::suspicious_else_formatting)]
                    pub fn #fn_ident<'a>(#(#all_params),*) -> #ret_type {
                        #dfir_tokens
                    }
                }
            };

            items.push(func);
        }

        syn::parse_quote! {
            use #orig_crate_name::__staged::__deps::*;
            use #root::prelude::*;
            use #root::runtime_support::dfir_rs as __root_dfir_rs;
            pub use #orig_crate_name::__staged;

            #( #items )*
        }
    }
}
