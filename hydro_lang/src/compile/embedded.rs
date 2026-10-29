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
//! - Sinks (`EmbeddedNetworkOut`): one `FnMut(..)` field per outgoing channel.
//! - Sources (`EmbeddedNetworkIn`): one `Stream` field per incoming channel.
//!
//! Each location's module also has a marker type per channel end (`outgoing::*` and
//! `incoming::*`) implementing
//! [`EmbeddedChannel`](crate::compile::embedded_runtime::EmbeddedChannel), which describes
//! the field's channel (the guarantees its declared transport and fault model give it).
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

use super::deploy_provider::{
    ClusterSpec, Deploy, EmbeddedStreamGuarantees, ExternalSpec, Node, ProcessSpec, RegisterPort,
};
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

/// One end of a named network channel at a location, as recorded for the embedded backend.
///
/// Unlike an [`EmbeddedStreamPort`], whose guarantees are those of one stream, a channel has a
/// transport between its ends: the receiving end gets the guarantees the transport keeps
/// (`networking`) for the stream the sending end provides (`sent`).
#[derive(Clone)]
pub struct EmbeddedNetworkPort {
    /// The channel name, from `.name()` on the networking config.
    pub name: String,
    /// Whether the port is keyed by a `TaglessMemberId`: each element's destination on the
    /// sender side, or its sender on the receiver side.
    pub is_tagged: bool,
    /// `None` for bincode serialization (Hydro serializes to bytes), or the payload type for an
    /// unserialized `EMBEDDED` channel (the raw payload crosses the channel).
    pub external_type: Option<syn::Type>,
    /// The location at the other end of the channel.
    pub peer: LocationKey,
    /// The guarantees of the stream at this end: the stream the dataflow sends at the sending
    /// end, and the stream it receives at the receiving end. `None` until
    /// `register_embedded_network_guarantees` records them.
    pub guarantees: Option<EmbeddedStreamGuarantees>,
}

/// A stream input or output of an embedded dataflow: the field it fills and the guarantees of its
/// stream.
#[derive(Clone)]
pub struct EmbeddedStreamPort {
    /// The field's name: the name given to `embedded_input` or `embedded_output`.
    pub ident: syn::Ident,
    /// The stream's element type.
    pub element_type: syn::Type,
    /// The stream's guarantees.
    pub guarantees: EmbeddedStreamGuarantees,
}

/// Collected embedded input/output registrations, keyed by location.
///
/// During `compile_network`, each `HydroSource::Embedded` and `HydroRoot::EmbeddedOutput`
/// IR node registers its ident, element type, and location key here.
/// `generate_embedded` then uses this to add the appropriate parameters
/// to each generated function.
#[derive(Default)]
pub struct EmbeddedInstantiateEnv {
    /// Stream inputs per location key: their guarantees are what the dataflow relies on, so the
    /// host must promise them.
    pub inputs: SparseSecondaryMap<LocationKey, Vec<EmbeddedStreamPort>>,
    /// (ident name, element type) pairs per location key, for singleton inputs.
    pub singleton_inputs: SparseSecondaryMap<LocationKey, Vec<(syn::Ident, syn::Type)>>,
    /// Outputs per location key: their guarantees are what the output stream provides to the
    /// host.
    pub outputs: SparseSecondaryMap<LocationKey, Vec<EmbeddedStreamPort>>,
    /// Network output ports per location key (sender side of channels). The sink is
    /// `FnMut(Bytes)` with bincode serialization (Hydro serializes before the sink) or
    /// `FnMut(payload)` with embedded serialization (see [`crate::networking::Embedded`]), with
    /// the payload paired with a `TaglessMemberId` when the port is tagged.
    pub network_outputs: SparseSecondaryMap<LocationKey, Vec<EmbeddedNetworkPort>>,
    /// Network input ports per location key (receiver side of channels). The source is
    /// `Stream<Item = Result<BytesMut, Error>>` with bincode serialization (Hydro unwraps the
    /// transport `Result` and deserializes) or `Stream<Item = payload>` with embedded
    /// serialization, with no transport `Result` (handling faults is left to the external code
    /// that produces the stream); again paired with a `TaglessMemberId` when tagged.
    pub network_inputs: SparseSecondaryMap<LocationKey, Vec<EmbeddedNetworkPort>>,
    /// Cluster membership streams needed per location key.
    /// Maps `location_key` -> vec of cluster [`LocationKey`]s whose membership is needed.
    pub membership_streams: SparseSecondaryMap<LocationKey, Vec<LocationKey>>,
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
        _networking_info: &crate::networking::NetworkingInfo,
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
            .push(EmbeddedNetworkPort {
                name: name.to_owned(),
                is_tagged: false,
                external_type: external_types.map(|(i, _)| i.clone()),
                peer: p2.location_key,
                // Filled in by `register_embedded_network_guarantees` once this returns.
                guarantees: None,
            });
        env.network_inputs
            .entry(p2.location_key)
            .unwrap()
            .or_default()
            .push(EmbeddedNetworkPort {
                name: name.to_owned(),
                is_tagged: false,
                external_type: external_types.map(|(_, o)| o.clone()),
                peer: p1.location_key,
                // Filled in by `register_embedded_network_guarantees` once this returns.
                guarantees: None,
            });

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
        _networking_info: &crate::networking::NetworkingInfo,
        external_types: Option<(&syn::Type, &syn::Type)>,
    ) -> (syn::Expr, syn::Expr) {
        let name = name.expect("EmbeddedDeploy o2m networking requires a channel name.");
        let sink_ident = syn::Ident::new(&format!("__network_out_{name}"), Span::call_site());
        let source_ident = syn::Ident::new(&format!("__network_in_{name}"), Span::call_site());
        env.network_outputs
            .entry(p1.location_key)
            .unwrap()
            .or_default()
            .push(EmbeddedNetworkPort {
                name: name.to_owned(),
                is_tagged: true,
                external_type: external_types.map(|(i, _)| i.clone()),
                peer: c2.location_key,
                // Filled in by `register_embedded_network_guarantees` once this returns.
                guarantees: None,
            });
        env.network_inputs
            .entry(c2.location_key)
            .unwrap()
            .or_default()
            .push(EmbeddedNetworkPort {
                name: name.to_owned(),
                is_tagged: false,
                external_type: external_types.map(|(_, o)| o.clone()),
                peer: p1.location_key,
                // Filled in by `register_embedded_network_guarantees` once this returns.
                guarantees: None,
            });
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
        _networking_info: &crate::networking::NetworkingInfo,
        external_types: Option<(&syn::Type, &syn::Type)>,
    ) -> (syn::Expr, syn::Expr) {
        let name = name.expect("EmbeddedDeploy m2o networking requires a channel name.");
        let sink_ident = syn::Ident::new(&format!("__network_out_{name}"), Span::call_site());
        let source_ident = syn::Ident::new(&format!("__network_in_{name}"), Span::call_site());
        env.network_outputs
            .entry(c1.location_key)
            .unwrap()
            .or_default()
            .push(EmbeddedNetworkPort {
                name: name.to_owned(),
                is_tagged: false,
                external_type: external_types.map(|(i, _)| i.clone()),
                peer: p2.location_key,
                // Filled in by `register_embedded_network_guarantees` once this returns.
                guarantees: None,
            });
        env.network_inputs
            .entry(p2.location_key)
            .unwrap()
            .or_default()
            .push(EmbeddedNetworkPort {
                name: name.to_owned(),
                is_tagged: true,
                external_type: external_types.map(|(_, o)| o.clone()),
                peer: c1.location_key,
                // Filled in by `register_embedded_network_guarantees` once this returns.
                guarantees: None,
            });
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
        _networking_info: &crate::networking::NetworkingInfo,
        external_types: Option<(&syn::Type, &syn::Type)>,
    ) -> (syn::Expr, syn::Expr) {
        let name = name.expect("EmbeddedDeploy m2m networking requires a channel name.");
        let sink_ident = syn::Ident::new(&format!("__network_out_{name}"), Span::call_site());
        let source_ident = syn::Ident::new(&format!("__network_in_{name}"), Span::call_site());
        env.network_outputs
            .entry(c1.location_key)
            .unwrap()
            .or_default()
            .push(EmbeddedNetworkPort {
                name: name.to_owned(),
                is_tagged: true,
                external_type: external_types.map(|(i, _)| i.clone()),
                peer: c2.location_key,
                // Filled in by `register_embedded_network_guarantees` once this returns.
                guarantees: None,
            });
        env.network_inputs
            .entry(c2.location_key)
            .unwrap()
            .or_default()
            .push(EmbeddedNetworkPort {
                name: name.to_owned(),
                is_tagged: true,
                external_type: external_types.map(|(_, o)| o.clone()),
                peer: c1.location_key,
                // Filled in by `register_embedded_network_guarantees` once this returns.
                guarantees: None,
            });
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
        guarantees: &EmbeddedStreamGuarantees,
    ) {
        env.inputs
            .entry(location_key)
            .unwrap()
            .or_default()
            .push(EmbeddedStreamPort {
                ident: ident.clone(),
                element_type: element_type.clone(),
                guarantees: guarantees.clone(),
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

    fn register_embedded_network_guarantees(
        env: &mut Self::InstantiateEnv,
        sender: LocationKey,
        receiver: LocationKey,
        name: Option<&str>,
        sent: &EmbeddedStreamGuarantees,
        received: &EmbeddedStreamGuarantees,
    ) {
        let name = name.expect("embedded networking requires a channel name");
        for (ports, location, peer, guarantees) in [
            (&mut env.network_outputs, sender, receiver, sent),
            (&mut env.network_inputs, receiver, sender, received),
        ] {
            let port = ports
                .get_mut(location)
                .and_then(|ports| {
                    ports
                        .iter_mut()
                        .find(|port| port.name == name && port.peer == peer)
                })
                .unwrap_or_else(|| panic!("embedded channel `{name}` was not instantiated"));
            port.guarantees = Some(guarantees.clone());
        }
    }

    fn register_embedded_output(
        env: &mut Self::InstantiateEnv,
        location_key: LocationKey,
        ident: &syn::Ident,
        element_type: &syn::Type,
        guarantees: &EmbeddedStreamGuarantees,
    ) {
        env.outputs
            .entry(location_key)
            .unwrap()
            .or_default()
            .push(EmbeddedStreamPort {
                ident: ident.clone(),
                element_type: element_type.clone(),
                guarantees: guarantees.clone(),
            });
    }
}

/// `generate_embedded` turns each location into a function the host calls. Among its parameters
/// are an `EmbeddedNetworkIn` struct, with a field per receiving end of the location's channels,
/// and an `EmbeddedNetworkOut` struct, with a field per sending end, each next to a module
/// (`incoming` / `outgoing`) of marker types describing those fields. Both directions are
/// generated the same way, so this builds, for one direction, the pieces that struct, its module,
/// and the function's signature are assembled from, one per port in port order.
struct NetworkPortTokens {
    fields: Vec<syn::Ident>,
    field_docs: Vec<String>,
    /// Each field's type: `InputChannel<Marker, S>` (receiving) or `OutputChannel<Marker, F>`
    /// (sending), for the port's marker type (which carries its guarantees) and generic parameter.
    field_types: Vec<proc_macro2::TokenStream>,
    generics: Vec<syn::Ident>,
    items: Vec<proc_macro2::TokenStream>,
    marker_defs: Vec<proc_macro2::TokenStream>,
}

impl NetworkPortTokens {
    fn new(
        ports: &[EmbeddedNetworkPort],
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
            let guarantees = port
                .guarantees
                .as_ref()
                .unwrap_or_else(|| panic!("embedded channel `{name}` has no recorded guarantees"));
            let mut doc = format!(
                "The `{name}` channel's end at this location; its other end is at `{peer}`."
            );
            if !sending {
                doc.push_str("\n\n");
                doc.push_str(&promise_doc(guarantees));
            }
            marker_defs.push(marker_def(&root, marker, &doc, guarantees));
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
        // The receiving end is supplied by the host, which promises the transport's guarantees;
        // the sending end takes what the dataflow provides, like an output.
        let wrapper = if sending {
            quote! { #root::compile::embedded_runtime::OutputChannel }
        } else {
            quote! { #root::compile::embedded_runtime::InputChannel }
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

/// The definition of the `marker` type of a channel crossing an embedded dataflow's
/// boundary, documented with `doc`, and implementing `EmbeddedChannel` with its `guarantees`.
///
/// Channels are named by strings, but the host's side of each is checked by the type checker,
/// so each needs a type: the marker, named as-is after its field (the channel's name).
fn marker_def(
    root: &proc_macro2::TokenStream,
    marker: &syn::Ident,
    doc: &str,
    guarantees: &EmbeddedStreamGuarantees,
) -> proc_macro2::TokenStream {
    let ordering = guarantees.order.type_tokens();
    let retries = guarantees.retry.type_tokens();
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
fn promise_doc(guarantees: &EmbeddedStreamGuarantees) -> String {
    use crate::compile::ir::{StreamOrder, StreamRetry};
    let promises = [
        (guarantees.order == StreamOrder::TotalOrder).then_some(".total_order(proof)"),
        (guarantees.retry == StreamRetry::ExactlyOnce).then_some(".exactly_once(proof)"),
    ]
    .into_iter()
    .flatten()
    .collect::<String>();
    format!("Promise its guarantees with `InputChannel::new(.., Promises::new(){promises})`.")
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
        let mut env = EmbeddedInstantiateEnv::default();
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

            // For cluster locations whose code reads their own member ID, add a self_id parameter.
            let self_id_ident = syn::Ident::new(
                &format!("__hydro_lang_cluster_self_id_{}", location_key),
                Span::call_site(),
            );
            if self.clusters.contains_key(location_key)
                && mentions_ident(&dfir_tokens, &self_id_ident)
            {
                cluster_params.push(quote! {
                    __cluster_self_id: &'a #root::location::member_id::TaglessMemberId
                });
                // Alias to the name the generated DFIR code expects.
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
                            promise_doc(&mem_guarantees)
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
                            pub #field: #root::compile::embedded_runtime::InputChannel<membership::#field, #generic>
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
                        marker_def(
                            &root,
                            &input.ident,
                            &format!(
                                "The `{}` stream input.\n\n{}",
                                input.ident,
                                promise_doc(&input.guarantees)
                            ),
                            &input.guarantees,
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
                            pub #ident: #root::compile::embedded_runtime::InputChannel<inputs::#ident, #generic>
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

            // Embedded outputs: one `OutputChannel` sink per output, over a marker whose guarantees
            // are what the output stream provides, which the sink's requirements must not exceed.
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
                    marker_defs.push(marker_def(
                        &root,
                        ident,
                        &format!("The `{ident}` output."),
                        &output.guarantees,
                    ));
                    let doc = format!("The `{ident}` output (marker type: [`outputs::{ident}`]).");
                    let sink_bound = quote! { FnMut(#element_type) };
                    struct_generics.push(quote! { #generic: #sink_bound });
                    extra_fn_generics.push(quote! { #generic: #sink_bound + 'a });
                    struct_fields.push(quote! {
                        #[doc = #doc]
                        pub #ident: #root::compile::embedded_runtime::OutputChannel<outputs::#ident, #generic>
                    });
                    generics.push(generic);
                    extra_destructure
                        .push(quote! { let mut #ident = __outputs.#ident.into_inner(); });
                }

                output_params.push(quote! {
                    __outputs: #fn_ident::#output_struct_ident<#(#generics),*>
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
                let channels =
                    NetworkPortTokens::new(&ports, &fn_names, "__NetOut", true, |port| {
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
                    });
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
                    __network_out: #fn_ident::EmbeddedNetworkOut<#(#generics),*>
                });

                for (port, field) in ports.iter().zip(fields) {
                    let var_ident =
                        syn::Ident::new(&format!("__network_out_{}", port.name), Span::call_site());
                    extra_destructure
                        .push(quote! { let mut #var_ident = __network_out.#field.into_inner(); });
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
                        #(#extra_destructure)*
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

/// Whether `tokens` contain `ident`, at any depth.
fn mentions_ident(tokens: &proc_macro2::TokenStream, ident: &syn::Ident) -> bool {
    tokens.clone().into_iter().any(|token| match token {
        proc_macro2::TokenTree::Ident(found) => found == *ident,
        proc_macro2::TokenTree::Group(group) => mentions_ident(&group.stream(), ident),
        _ => false,
    })
}
