//! Deployment backend for Hydro that uses [`hydro_deploy`] to provision and launch services.

use std::cell::RefCell;
use std::collections::HashMap;
use std::future::Future;
use std::io::Error;
use std::pin::Pin;
use std::rc::Rc;
use std::sync::Arc;

use bytes::{Bytes, BytesMut};
use dfir_lang::graph::DfirGraph;
use futures::{Sink, SinkExt, Stream, StreamExt};
use hydro_deploy::custom_service::CustomClientPort;
use hydro_deploy::rust_crate::RustCrateService;
use hydro_deploy::rust_crate::ports::{DemuxSink, RustCrateSink, RustCrateSource, TaggedSource};
use hydro_deploy::rust_crate::tracing_options::TracingOptions;
use hydro_deploy::{CustomService, Deployment, Host, RustCrate};
use hydro_deploy_integration::{ConnectedSink, ConnectedSource};
use nameof::name_of;
use proc_macro2::Span;
use serde::Serialize;
use serde::de::DeserializeOwned;
use slotmap::SparseSecondaryMap;
use stageleft::{QuotedWithContext, RuntimeData};
use syn::parse_quote;

use super::deploy_runtime::*;
use crate::compile::builder::ExternalPortId;
use crate::compile::deploy_provider::{
    ClusterSpec, Deploy, ExternalSpec, IntoProcessSpec, Node, ProcessSpec, RegisterPort,
};
use crate::compile::trybuild::generate::{
    HYDRO_RUNTIME_FEATURES, LinkingMode, create_graph_trybuild,
};
use crate::location::dynamic::LocationId;
use crate::location::member_id::TaglessMemberId;
use crate::location::{LocationKey, MembershipEvent, NetworkHint};
use crate::staging_util::get_this_crate;

/// Deployment backend that uses [`hydro_deploy`] for provisioning and launching.
///
/// Automatically used when you call [`crate::compile::builder::FlowBuilder::deploy`] and pass in
/// an `&mut` reference to [`hydro_deploy::Deployment`] as the deployment context.
pub enum HydroDeploy {}

impl<'a> Deploy<'a> for HydroDeploy {
    /// Map from Cluster location ID to member IDs.
    type Meta = SparseSecondaryMap<LocationKey, Vec<TaglessMemberId>>;
    type InstantiateEnv = Deployment;

    type Process = DeployNode;
    type Cluster = DeployCluster;
    type External = DeployExternal;

    fn o2o_sink_source(
        _p1: &Self::Process,
        p1_port: &<Self::Process as Node>::Port,
        _p2: &Self::Process,
        p2_port: &<Self::Process as Node>::Port,
    ) -> (syn::Expr, syn::Expr) {
        let p1_port = p1_port.as_str();
        let p2_port = p2_port.as_str();
        deploy_o2o(
            RuntimeData::new("__hydro_lang_trybuild_cli"),
            p1_port,
            p2_port,
        )
    }

    fn o2o_connect(
        p1: &Self::Process,
        p1_port: &<Self::Process as Node>::Port,
        p2: &Self::Process,
        p2_port: &<Self::Process as Node>::Port,
    ) -> Box<dyn FnOnce()> {
        let p1 = p1.clone();
        let p1_port = p1_port.clone();
        let p2 = p2.clone();
        let p2_port = p2_port.clone();

        Box::new(move || {
            let self_underlying_borrow = p1.underlying.borrow();
            let self_underlying = self_underlying_borrow.as_ref().unwrap();
            let source_port = self_underlying.get_port(p1_port.clone());

            let other_underlying_borrow = p2.underlying.borrow();
            let other_underlying = other_underlying_borrow.as_ref().unwrap();
            let recipient_port = other_underlying.get_port(p2_port.clone());

            source_port.send_to(&recipient_port)
        })
    }

    fn o2m_sink_source(
        _p1: &Self::Process,
        p1_port: &<Self::Process as Node>::Port,
        _c2: &Self::Cluster,
        c2_port: &<Self::Cluster as Node>::Port,
    ) -> (syn::Expr, syn::Expr) {
        let p1_port = p1_port.as_str();
        let c2_port = c2_port.as_str();
        deploy_o2m(
            RuntimeData::new("__hydro_lang_trybuild_cli"),
            p1_port,
            c2_port,
        )
    }

    fn o2m_connect(
        p1: &Self::Process,
        p1_port: &<Self::Process as Node>::Port,
        c2: &Self::Cluster,
        c2_port: &<Self::Cluster as Node>::Port,
    ) -> Box<dyn FnOnce()> {
        let p1 = p1.clone();
        let p1_port = p1_port.clone();
        let c2 = c2.clone();
        let c2_port = c2_port.clone();

        Box::new(move || {
            let self_underlying_borrow = p1.underlying.borrow();
            let self_underlying = self_underlying_borrow.as_ref().unwrap();
            let source_port = self_underlying.get_port(p1_port.clone());

            let recipient_port = DemuxSink {
                demux: c2
                    .members
                    .borrow()
                    .iter()
                    .enumerate()
                    .map(|(id, c)| {
                        (
                            id as u32,
                            Arc::new(c.underlying.get_port(c2_port.clone()))
                                as Arc<dyn RustCrateSink + 'static>,
                        )
                    })
                    .collect(),
            };

            source_port.send_to(&recipient_port)
        })
    }

    fn m2o_sink_source(
        _c1: &Self::Cluster,
        c1_port: &<Self::Cluster as Node>::Port,
        _p2: &Self::Process,
        p2_port: &<Self::Process as Node>::Port,
    ) -> (syn::Expr, syn::Expr) {
        let c1_port = c1_port.as_str();
        let p2_port = p2_port.as_str();
        deploy_m2o(
            RuntimeData::new("__hydro_lang_trybuild_cli"),
            c1_port,
            p2_port,
        )
    }

    fn m2o_connect(
        c1: &Self::Cluster,
        c1_port: &<Self::Cluster as Node>::Port,
        p2: &Self::Process,
        p2_port: &<Self::Process as Node>::Port,
    ) -> Box<dyn FnOnce()> {
        let c1 = c1.clone();
        let c1_port = c1_port.clone();
        let p2 = p2.clone();
        let p2_port = p2_port.clone();

        Box::new(move || {
            let other_underlying_borrow = p2.underlying.borrow();
            let other_underlying = other_underlying_borrow.as_ref().unwrap();
            let recipient_port = other_underlying.get_port(p2_port.clone()).merge();

            for (i, node) in c1.members.borrow().iter().enumerate() {
                let source_port = node.underlying.get_port(c1_port.clone());

                TaggedSource {
                    source: Arc::new(source_port),
                    tag: i as u32,
                }
                .send_to(&recipient_port);
            }
        })
    }

    fn m2m_sink_source(
        _c1: &Self::Cluster,
        c1_port: &<Self::Cluster as Node>::Port,
        _c2: &Self::Cluster,
        c2_port: &<Self::Cluster as Node>::Port,
    ) -> (syn::Expr, syn::Expr) {
        let c1_port = c1_port.as_str();
        let c2_port = c2_port.as_str();
        deploy_m2m(
            RuntimeData::new("__hydro_lang_trybuild_cli"),
            c1_port,
            c2_port,
        )
    }

    fn m2m_connect(
        c1: &Self::Cluster,
        c1_port: &<Self::Cluster as Node>::Port,
        c2: &Self::Cluster,
        c2_port: &<Self::Cluster as Node>::Port,
    ) -> Box<dyn FnOnce()> {
        let c1 = c1.clone();
        let c1_port = c1_port.clone();
        let c2 = c2.clone();
        let c2_port = c2_port.clone();

        Box::new(move || {
            for (i, sender) in c1.members.borrow().iter().enumerate() {
                let source_port = sender.underlying.get_port(c1_port.clone());

                let recipient_port = DemuxSink {
                    demux: c2
                        .members
                        .borrow()
                        .iter()
                        .enumerate()
                        .map(|(id, c)| {
                            (
                                id as u32,
                                Arc::new(c.underlying.get_port(c2_port.clone()).merge())
                                    as Arc<dyn RustCrateSink + 'static>,
                            )
                        })
                        .collect(),
                };

                TaggedSource {
                    source: Arc::new(source_port),
                    tag: i as u32,
                }
                .send_to(&recipient_port);
            }
        })
    }

    fn e2o_many_source(
        extra_stmts: &mut Vec<syn::Stmt>,
        _p2: &Self::Process,
        p2_port: &<Self::Process as Node>::Port,
        codec_type: &syn::Type,
        shared_handle: String,
    ) -> syn::Expr {
        let connect_ident = syn::Ident::new(
            &format!("__hydro_deploy_many_{}_connect", &shared_handle),
            Span::call_site(),
        );
        let source_ident = syn::Ident::new(
            &format!("__hydro_deploy_many_{}_source", &shared_handle),
            Span::call_site(),
        );
        let sink_ident = syn::Ident::new(
            &format!("__hydro_deploy_many_{}_sink", &shared_handle),
            Span::call_site(),
        );
        let membership_ident = syn::Ident::new(
            &format!("__hydro_deploy_many_{}_membership", &shared_handle),
            Span::call_site(),
        );

        let root = get_this_crate();

        extra_stmts.push(syn::parse_quote! {
            let #connect_ident = __hydro_lang_trybuild_cli
                .port(#p2_port)
                .connect::<#root::runtime_support::hydro_deploy_integration::multi_connection::ConnectedMultiConnection<_, _, #codec_type>>();
        });

        extra_stmts.push(syn::parse_quote! {
            let #source_ident = #connect_ident.source;
        });

        extra_stmts.push(syn::parse_quote! {
            let #sink_ident = #connect_ident.sink;
        });

        extra_stmts.push(syn::parse_quote! {
            let #membership_ident = #connect_ident.membership;
        });

        parse_quote!(#source_ident)
    }

    fn e2o_many_sink(shared_handle: String) -> syn::Expr {
        let sink_ident = syn::Ident::new(
            &format!("__hydro_deploy_many_{}_sink", &shared_handle),
            Span::call_site(),
        );
        parse_quote!(#sink_ident)
    }

    fn e2o_source(
        extra_stmts: &mut Vec<syn::Stmt>,
        _p1: &Self::External,
        _p1_port: &<Self::External as Node>::Port,
        _p2: &Self::Process,
        p2_port: &<Self::Process as Node>::Port,
        codec_type: &syn::Type,
        shared_handle: String,
    ) -> syn::Expr {
        let connect_ident = syn::Ident::new(
            &format!("__hydro_deploy_{}_connect", &shared_handle),
            Span::call_site(),
        );
        let source_ident = syn::Ident::new(
            &format!("__hydro_deploy_{}_source", &shared_handle),
            Span::call_site(),
        );
        let sink_ident = syn::Ident::new(
            &format!("__hydro_deploy_{}_sink", &shared_handle),
            Span::call_site(),
        );

        let root = get_this_crate();

        extra_stmts.push(syn::parse_quote! {
            let #connect_ident = __hydro_lang_trybuild_cli
                .port(#p2_port)
                .connect::<#root::runtime_support::hydro_deploy_integration::single_connection::ConnectedSingleConnection<_, _, #codec_type>>();
        });

        extra_stmts.push(syn::parse_quote! {
            let #source_ident = #connect_ident.source;
        });

        extra_stmts.push(syn::parse_quote! {
            let #sink_ident = #connect_ident.sink;
        });

        parse_quote!(#source_ident)
    }

    fn e2o_connect(
        p1: &Self::External,
        p1_port: &<Self::External as Node>::Port,
        p2: &Self::Process,
        p2_port: &<Self::Process as Node>::Port,
        _many: bool,
        server_hint: NetworkHint,
    ) -> Box<dyn FnOnce()> {
        let p1 = p1.clone();
        let p1_port = p1_port.clone();
        let p2 = p2.clone();
        let p2_port = p2_port.clone();

        Box::new(move || {
            let self_underlying_borrow = p1.underlying.borrow();
            let self_underlying = self_underlying_borrow.as_ref().unwrap();
            let source_port = self_underlying.declare_many_client();

            let other_underlying_borrow = p2.underlying.borrow();
            let other_underlying = other_underlying_borrow.as_ref().unwrap();
            let recipient_port = other_underlying.get_port_with_hint(
                p2_port.clone(),
                match server_hint {
                    NetworkHint::Auto => hydro_deploy::PortNetworkHint::Auto,
                    NetworkHint::TcpPort(p) => hydro_deploy::PortNetworkHint::TcpPort(p),
                },
            );

            source_port.send_to(&recipient_port);

            p1.client_ports
                .borrow_mut()
                .insert(p1_port.clone(), source_port);
        })
    }

    fn o2e_sink(
        _p1: &Self::Process,
        _p1_port: &<Self::Process as Node>::Port,
        _p2: &Self::External,
        _p2_port: &<Self::External as Node>::Port,
        shared_handle: String,
    ) -> syn::Expr {
        let sink_ident = syn::Ident::new(
            &format!("__hydro_deploy_{}_sink", &shared_handle),
            Span::call_site(),
        );
        parse_quote!(#sink_ident)
    }

    fn cluster_ids(
        of_cluster: LocationKey,
    ) -> impl QuotedWithContext<'a, &'a [TaglessMemberId], ()> + Clone + 'a {
        cluster_members(RuntimeData::new("__hydro_lang_trybuild_cli"), of_cluster)
    }

    fn cluster_self_id() -> impl QuotedWithContext<'a, TaglessMemberId, ()> + Clone + 'a {
        cluster_self_id(RuntimeData::new("__hydro_lang_trybuild_cli"))
    }

    fn cluster_membership_stream(
        location_id: &LocationId,
    ) -> impl QuotedWithContext<'a, Box<dyn Stream<Item = (TaglessMemberId, MembershipEvent)> + Unpin>, ()>
    {
        cluster_membership_stream(location_id)
    }
}

#[expect(missing_docs, reason = "TODO")]
pub trait DeployCrateWrapper {
    fn underlying(&self) -> Arc<RustCrateService>;

    fn stdout(&self) -> tokio::sync::mpsc::UnboundedReceiver<String> {
        self.underlying().stdout()
    }

    fn stderr(&self) -> tokio::sync::mpsc::UnboundedReceiver<String> {
        self.underlying().stderr()
    }

    fn stdout_filter(
        &self,
        prefix: impl Into<String>,
    ) -> tokio::sync::mpsc::UnboundedReceiver<String> {
        self.underlying().stdout_filter(prefix.into())
    }

    fn stderr_filter(
        &self,
        prefix: impl Into<String>,
    ) -> tokio::sync::mpsc::UnboundedReceiver<String> {
        self.underlying().stderr_filter(prefix.into())
    }
}

#[expect(missing_docs, reason = "TODO")]
#[derive(Clone)]
pub struct TrybuildHost {
    host: Arc<dyn Host>,
    display_name: Option<String>,
    rustflags: Option<String>,
    profile: Option<String>,
    additional_hydro_features: Vec<String>,
    features: Vec<String>,
    tracing: Option<TracingOptions>,
    build_envs: Vec<(String, String)>,
    name_hint: Option<String>,
    cluster_idx: Option<usize>,
}

impl From<Arc<dyn Host>> for TrybuildHost {
    fn from(host: Arc<dyn Host>) -> Self {
        Self {
            host,
            display_name: None,
            rustflags: None,
            profile: None,
            additional_hydro_features: vec![],
            features: vec![],
            tracing: None,
            build_envs: vec![],
            name_hint: None,
            cluster_idx: None,
        }
    }
}

impl<H: Host + 'static> From<Arc<H>> for TrybuildHost {
    fn from(host: Arc<H>) -> Self {
        Self {
            host,
            display_name: None,
            rustflags: None,
            profile: None,
            additional_hydro_features: vec![],
            features: vec![],
            tracing: None,
            build_envs: vec![],
            name_hint: None,
            cluster_idx: None,
        }
    }
}

#[expect(missing_docs, reason = "TODO")]
impl TrybuildHost {
    pub fn new(host: Arc<dyn Host>) -> Self {
        Self {
            host,
            display_name: None,
            rustflags: None,
            profile: None,
            additional_hydro_features: vec![],
            features: vec![],
            tracing: None,
            build_envs: vec![],
            name_hint: None,
            cluster_idx: None,
        }
    }

    pub fn display_name(self, display_name: impl Into<String>) -> Self {
        if self.display_name.is_some() {
            panic!("{} already set", name_of!(display_name in Self));
        }

        Self {
            display_name: Some(display_name.into()),
            ..self
        }
    }

    pub fn rustflags(self, rustflags: impl Into<String>) -> Self {
        if self.rustflags.is_some() {
            panic!("{} already set", name_of!(rustflags in Self));
        }

        Self {
            rustflags: Some(rustflags.into()),
            ..self
        }
    }

    pub fn profile(self, profile: impl Into<String>) -> Self {
        if self.profile.is_some() {
            panic!("{} already set", name_of!(profile in Self));
        }

        Self {
            profile: Some(profile.into()),
            ..self
        }
    }

    pub fn additional_hydro_features(self, additional_hydro_features: Vec<String>) -> Self {
        Self {
            additional_hydro_features,
            ..self
        }
    }

    pub fn features(self, features: Vec<String>) -> Self {
        Self {
            features: self.features.into_iter().chain(features).collect(),
            ..self
        }
    }

    pub fn tracing(self, tracing: TracingOptions) -> Self {
        if self.tracing.is_some() {
            panic!("{} already set", name_of!(tracing in Self));
        }

        Self {
            tracing: Some(tracing),
            ..self
        }
    }

    pub fn build_env(self, key: impl Into<String>, value: impl Into<String>) -> Self {
        Self {
            build_envs: self
                .build_envs
                .into_iter()
                .chain(std::iter::once((key.into(), value.into())))
                .collect(),
            ..self
        }
    }
}

impl IntoProcessSpec<'_, HydroDeploy> for Arc<dyn Host> {
    type ProcessSpec = TrybuildHost;
    fn into_process_spec(self) -> TrybuildHost {
        TrybuildHost {
            host: self,
            display_name: None,
            rustflags: None,
            profile: None,
            additional_hydro_features: vec![],
            features: vec![],
            tracing: None,
            build_envs: vec![],
            name_hint: None,
            cluster_idx: None,
        }
    }
}

impl<H: Host + 'static> IntoProcessSpec<'_, HydroDeploy> for Arc<H> {
    type ProcessSpec = TrybuildHost;
    fn into_process_spec(self) -> TrybuildHost {
        TrybuildHost {
            host: self,
            display_name: None,
            rustflags: None,
            profile: None,
            additional_hydro_features: vec![],
            features: vec![],
            tracing: None,
            build_envs: vec![],
            name_hint: None,
            cluster_idx: None,
        }
    }
}

#[expect(missing_docs, reason = "TODO")]
#[derive(Clone)]
pub struct DeployExternal {
    next_port: Rc<RefCell<usize>>,
    host: Arc<dyn Host>,
    underlying: Rc<RefCell<Option<Arc<CustomService>>>>,
    client_ports: Rc<RefCell<HashMap<String, CustomClientPort>>>,
    allocated_ports: Rc<RefCell<HashMap<ExternalPortId, String>>>,
}

impl DeployExternal {
    pub(crate) fn raw_port(&self, external_port_id: ExternalPortId) -> CustomClientPort {
        self.client_ports
            .borrow()
            .get(
                self.allocated_ports
                    .borrow()
                    .get(&external_port_id)
                    .unwrap(),
            )
            .unwrap()
            .clone()
    }
}

impl<'a> RegisterPort<'a, HydroDeploy> for DeployExternal {
    fn register(&self, external_port_id: ExternalPortId, port: Self::Port) {
        assert!(
            self.allocated_ports
                .borrow_mut()
                .insert(external_port_id, port.clone())
                .is_none_or(|old| old == port)
        );
    }

    fn as_bytes_bidi(
        &self,
        external_port_id: ExternalPortId,
    ) -> impl Future<
        Output = (
            Pin<Box<dyn Stream<Item = Result<BytesMut, Error>>>>,
            Pin<Box<dyn Sink<Bytes, Error = Error>>>,
        ),
    > + 'a {
        let port = self.raw_port(external_port_id);

        async move {
            let (source, sink) = port.connect().await.into_source_sink();
            (
                Box::pin(source) as Pin<Box<dyn Stream<Item = Result<BytesMut, Error>>>>,
                Box::pin(sink) as Pin<Box<dyn Sink<Bytes, Error = Error>>>,
            )
        }
    }

    fn as_bincode_bidi<InT, OutT>(
        &self,
        external_port_id: ExternalPortId,
    ) -> impl Future<
        Output = (
            Pin<Box<dyn Stream<Item = OutT>>>,
            Pin<Box<dyn Sink<InT, Error = Error>>>,
        ),
    > + 'a
    where
        InT: Serialize + 'static,
        OutT: DeserializeOwned + 'static,
    {
        let port = self.raw_port(external_port_id);
        async move {
            let (source, sink) = port.connect().await.into_source_sink();
            (
                Box::pin(source.map(|item| bincode::deserialize(&item.unwrap()).unwrap()))
                    as Pin<Box<dyn Stream<Item = OutT>>>,
                Box::pin(
                    sink.with(|item| async move { Ok(bincode::serialize(&item).unwrap().into()) }),
                ) as Pin<Box<dyn Sink<InT, Error = Error>>>,
            )
        }
    }

    fn as_bincode_sink<T: Serialize + 'static>(
        &self,
        external_port_id: ExternalPortId,
    ) -> impl Future<Output = Pin<Box<dyn Sink<T, Error = Error>>>> + 'a {
        let port = self.raw_port(external_port_id);
        async move {
            let sink = port.connect().await.into_sink();
            Box::pin(sink.with(|item| async move { Ok(bincode::serialize(&item).unwrap().into()) }))
                as Pin<Box<dyn Sink<T, Error = Error>>>
        }
    }

    fn as_bincode_source<T: DeserializeOwned + 'static>(
        &self,
        external_port_id: ExternalPortId,
    ) -> impl Future<Output = Pin<Box<dyn Stream<Item = T>>>> + 'a {
        let port = self.raw_port(external_port_id);
        async move {
            let source = port.connect().await.into_source();
            Box::pin(source.map(|item| bincode::deserialize(&item.unwrap()).unwrap()))
                as Pin<Box<dyn Stream<Item = T>>>
        }
    }
}

impl Node for DeployExternal {
    type Port = String;
    /// Map from Cluster location ID to member IDs.
    type Meta = SparseSecondaryMap<LocationKey, Vec<TaglessMemberId>>;
    type InstantiateEnv = Deployment;

    fn next_port(&self) -> Self::Port {
        let next_port = *self.next_port.borrow();
        *self.next_port.borrow_mut() += 1;

        format!("port_{}", next_port)
    }

    fn instantiate(
        &self,
        env: &mut Self::InstantiateEnv,
        _meta: &mut Self::Meta,
        _graph: DfirGraph,
        _extra_stmts: Vec<syn::Stmt>,
    ) {
        let service = env.CustomService(self.host.clone(), vec![]);
        *self.underlying.borrow_mut() = Some(service);
    }

    fn update_meta(&self, _meta: &Self::Meta) {}
}

impl ExternalSpec<'_, HydroDeploy> for Arc<dyn Host> {
    fn build(self, _key: LocationKey, _name_hint: &str) -> DeployExternal {
        DeployExternal {
            next_port: Rc::new(RefCell::new(0)),
            host: self,
            underlying: Rc::new(RefCell::new(None)),
            allocated_ports: Rc::new(RefCell::new(HashMap::new())),
            client_ports: Rc::new(RefCell::new(HashMap::new())),
        }
    }
}

impl<H: Host + 'static> ExternalSpec<'_, HydroDeploy> for Arc<H> {
    fn build(self, _key: LocationKey, _name_hint: &str) -> DeployExternal {
        DeployExternal {
            next_port: Rc::new(RefCell::new(0)),
            host: self,
            underlying: Rc::new(RefCell::new(None)),
            allocated_ports: Rc::new(RefCell::new(HashMap::new())),
            client_ports: Rc::new(RefCell::new(HashMap::new())),
        }
    }
}

pub(crate) enum CrateOrTrybuild {
    Crate(RustCrate, Arc<dyn Host>),
    Trybuild(TrybuildHost),
}

#[expect(missing_docs, reason = "TODO")]
#[derive(Clone)]
pub struct DeployNode {
    next_port: Rc<RefCell<usize>>,
    service_spec: Rc<RefCell<Option<CrateOrTrybuild>>>,
    underlying: Rc<RefCell<Option<Arc<RustCrateService>>>>,
}

impl DeployCrateWrapper for DeployNode {
    fn underlying(&self) -> Arc<RustCrateService> {
        Arc::clone(self.underlying.borrow().as_ref().unwrap())
    }
}

impl Node for DeployNode {
    type Port = String;
    /// Map from Cluster location ID to member IDs.
    type Meta = SparseSecondaryMap<LocationKey, Vec<TaglessMemberId>>;
    type InstantiateEnv = Deployment;

    fn next_port(&self) -> String {
        let next_port = *self.next_port.borrow();
        *self.next_port.borrow_mut() += 1;

        format!("port_{}", next_port)
    }

    fn update_meta(&self, meta: &Self::Meta) {
        let underlying_node = self.underlying.borrow();
        underlying_node.as_ref().unwrap().update_meta(HydroMeta {
            clusters: meta.clone(),
            cluster_id: None,
        });
    }

    fn instantiate(
        &self,
        env: &mut Self::InstantiateEnv,
        _meta: &mut Self::Meta,
        graph: DfirGraph,
        extra_stmts: Vec<syn::Stmt>,
    ) {
        let (service, host) = match self.service_spec.borrow_mut().take().unwrap() {
            CrateOrTrybuild::Crate(c, host) => (c, host),
            CrateOrTrybuild::Trybuild(trybuild) => {
                // Determine linking mode based on host target type
                let linking_mode = if !cfg!(target_os = "windows")
                    && trybuild.host.target_type() == hydro_deploy::HostTargetType::Local
                {
                    // When compiling for local, prefer dynamic linking to reduce binary size
                    // Windows is currently not supported due to https://github.com/bevyengine/bevy/pull/2016
                    LinkingMode::Dynamic
                } else {
                    LinkingMode::Static
                };
                let (bin_name, config) = create_graph_trybuild(
                    graph,
                    extra_stmts,
                    trybuild.name_hint.as_deref(),
                    false,
                    linking_mode,
                );
                let host = trybuild.host.clone();
                (
                    create_trybuild_service(
                        trybuild,
                        &config.project_dir,
                        &config.target_dir,
                        &config.features,
                        &bin_name,
                        &config.linking_mode,
                    ),
                    host,
                )
            }
        };

        *self.underlying.borrow_mut() = Some(env.add_service(service, host));
    }
}

#[expect(missing_docs, reason = "TODO")]
#[derive(Clone)]
pub struct DeployClusterNode {
    underlying: Arc<RustCrateService>,
}

impl DeployCrateWrapper for DeployClusterNode {
    fn underlying(&self) -> Arc<RustCrateService> {
        self.underlying.clone()
    }
}
#[expect(missing_docs, reason = "TODO")]
#[derive(Clone)]
pub struct DeployCluster {
    key: LocationKey,
    next_port: Rc<RefCell<usize>>,
    cluster_spec: Rc<RefCell<Option<Vec<CrateOrTrybuild>>>>,
    members: Rc<RefCell<Vec<DeployClusterNode>>>,
    name_hint: Option<String>,
}

impl DeployCluster {
    #[expect(missing_docs, reason = "TODO")]
    pub fn members(&self) -> Vec<DeployClusterNode> {
        self.members.borrow().clone()
    }
}

impl Node for DeployCluster {
    type Port = String;
    /// Map from Cluster location ID to member IDs.
    type Meta = SparseSecondaryMap<LocationKey, Vec<TaglessMemberId>>;
    type InstantiateEnv = Deployment;

    fn next_port(&self) -> String {
        let next_port = *self.next_port.borrow();
        *self.next_port.borrow_mut() += 1;

        format!("port_{}", next_port)
    }

    fn instantiate(
        &self,
        env: &mut Self::InstantiateEnv,
        meta: &mut Self::Meta,
        graph: DfirGraph,
        extra_stmts: Vec<syn::Stmt>,
    ) {
        let has_trybuild = self
            .cluster_spec
            .borrow()
            .as_ref()
            .unwrap()
            .iter()
            .any(|spec| matches!(spec, CrateOrTrybuild::Trybuild { .. }));

        // For clusters, use static linking if ANY host is non-local (conservative approach)
        let linking_mode = if !cfg!(target_os = "windows")
            && self
                .cluster_spec
                .borrow()
                .as_ref()
                .unwrap()
                .iter()
                .all(|spec| match spec {
                    CrateOrTrybuild::Crate(_, _) => true, // crates handle their own linking
                    CrateOrTrybuild::Trybuild(t) => {
                        t.host.target_type() == hydro_deploy::HostTargetType::Local
                    }
                }) {
            // See comment above for Windows exception
            LinkingMode::Dynamic
        } else {
            LinkingMode::Static
        };

        let maybe_trybuild = if has_trybuild {
            Some(create_graph_trybuild(
                graph,
                extra_stmts,
                self.name_hint.as_deref(),
                false,
                linking_mode,
            ))
        } else {
            None
        };

        let cluster_nodes = self
            .cluster_spec
            .borrow_mut()
            .take()
            .unwrap()
            .into_iter()
            .map(|spec| {
                let (service, host) = match spec {
                    CrateOrTrybuild::Crate(c, host) => (c, host),
                    CrateOrTrybuild::Trybuild(trybuild) => {
                        let (bin_name, config) = maybe_trybuild.as_ref().unwrap();
                        let host = trybuild.host.clone();
                        (
                            create_trybuild_service(
                                trybuild,
                                &config.project_dir,
                                &config.target_dir,
                                &config.features,
                                bin_name,
                                &config.linking_mode,
                            ),
                            host,
                        )
                    }
                };

                env.add_service(service, host)
            })
            .collect::<Vec<_>>();
        meta.insert(
            self.key,
            (0..(cluster_nodes.len() as u32))
                .map(TaglessMemberId::from_raw_id)
                .collect(),
        );
        *self.members.borrow_mut() = cluster_nodes
            .into_iter()
            .map(|n| DeployClusterNode { underlying: n })
            .collect();
    }

    fn update_meta(&self, meta: &Self::Meta) {
        for (cluster_id, node) in self.members.borrow().iter().enumerate() {
            node.underlying.update_meta(HydroMeta {
                clusters: meta.clone(),
                cluster_id: Some(TaglessMemberId::from_raw_id(cluster_id as u32)),
            });
        }
    }
}

#[expect(missing_docs, reason = "TODO")]
#[derive(Clone)]
pub struct DeployProcessSpec(RustCrate, Arc<dyn Host>);

impl DeployProcessSpec {
    #[expect(missing_docs, reason = "TODO")]
    pub fn new(t: RustCrate, host: Arc<dyn Host>) -> Self {
        Self(t, host)
    }
}

impl ProcessSpec<'_, HydroDeploy> for DeployProcessSpec {
    fn build(self, _key: LocationKey, _name_hint: &str) -> DeployNode {
        DeployNode {
            next_port: Rc::new(RefCell::new(0)),
            service_spec: Rc::new(RefCell::new(Some(CrateOrTrybuild::Crate(self.0, self.1)))),
            underlying: Rc::new(RefCell::new(None)),
        }
    }
}

impl ProcessSpec<'_, HydroDeploy> for TrybuildHost {
    fn build(mut self, key: LocationKey, name_hint: &str) -> DeployNode {
        self.name_hint = Some(format!("{} (process {})", name_hint, key));
        DeployNode {
            next_port: Rc::new(RefCell::new(0)),
            service_spec: Rc::new(RefCell::new(Some(CrateOrTrybuild::Trybuild(self)))),
            underlying: Rc::new(RefCell::new(None)),
        }
    }
}

#[expect(missing_docs, reason = "TODO")]
#[derive(Clone)]
pub struct DeployClusterSpec(Vec<(RustCrate, Arc<dyn Host>)>);

impl DeployClusterSpec {
    #[expect(missing_docs, reason = "TODO")]
    pub fn new(crates: Vec<(RustCrate, Arc<dyn Host>)>) -> Self {
        Self(crates)
    }
}

impl ClusterSpec<'_, HydroDeploy> for DeployClusterSpec {
    fn build(self, key: LocationKey, _name_hint: &str) -> DeployCluster {
        DeployCluster {
            key,
            next_port: Rc::new(RefCell::new(0)),
            cluster_spec: Rc::new(RefCell::new(Some(
                self.0
                    .into_iter()
                    .map(|(c, h)| CrateOrTrybuild::Crate(c, h))
                    .collect(),
            ))),
            members: Rc::new(RefCell::new(vec![])),
            name_hint: None,
        }
    }
}

impl<T: Into<TrybuildHost>, I: IntoIterator<Item = T>> ClusterSpec<'_, HydroDeploy> for I {
    fn build(self, key: LocationKey, name_hint: &str) -> DeployCluster {
        let name_hint = format!("{} (cluster {})", name_hint, key);
        DeployCluster {
            key,
            next_port: Rc::new(RefCell::new(0)),
            cluster_spec: Rc::new(RefCell::new(Some(
                self.into_iter()
                    .enumerate()
                    .map(|(idx, b)| {
                        let mut b = b.into();
                        b.name_hint = Some(name_hint.clone());
                        b.cluster_idx = Some(idx);
                        CrateOrTrybuild::Trybuild(b)
                    })
                    .collect(),
            ))),
            members: Rc::new(RefCell::new(vec![])),
            name_hint: Some(name_hint),
        }
    }
}

fn create_trybuild_service(
    trybuild: TrybuildHost,
    dir: &std::path::Path,
    target_dir: &std::path::PathBuf,
    features: &Option<Vec<String>>,
    bin_name: &str,
    linking_mode: &LinkingMode,
) -> RustCrate {
    // For dynamic linking, use the dylib-examples crate; for static, use the base crate
    let crate_dir = match linking_mode {
        LinkingMode::Dynamic => dir.join("dylib-examples"),
        LinkingMode::Static => dir.to_path_buf(),
    };

    let mut ret = RustCrate::new(&crate_dir)
        .target_dir(target_dir)
        .example(bin_name)
        .no_default_features();

    ret = ret.set_is_dylib(matches!(linking_mode, LinkingMode::Dynamic));

    if let Some(display_name) = trybuild.display_name {
        ret = ret.display_name(display_name);
    } else if let Some(name_hint) = trybuild.name_hint {
        if let Some(cluster_idx) = trybuild.cluster_idx {
            ret = ret.display_name(format!("{} / {}", name_hint, cluster_idx));
        } else {
            ret = ret.display_name(name_hint);
        }
    }

    if let Some(rustflags) = trybuild.rustflags {
        ret = ret.rustflags(rustflags);
    }

    if let Some(profile) = trybuild.profile {
        ret = ret.profile(profile);
    }

    if let Some(tracing) = trybuild.tracing {
        ret = ret.tracing(tracing);
    }

    ret = ret.features(
        vec!["hydro___feature_deploy_integration".to_string()]
            .into_iter()
            .chain(
                trybuild
                    .additional_hydro_features
                    .into_iter()
                    .map(|runtime_feature| {
                        assert!(
                            HYDRO_RUNTIME_FEATURES.iter().any(|f| f == &runtime_feature),
                            "{runtime_feature} is not a valid Hydro runtime feature"
                        );
                        format!("hydro___feature_{runtime_feature}")
                    }),
            )
            .chain(trybuild.features),
    );

    for (key, value) in trybuild.build_envs {
        ret = ret.build_env(key, value);
    }

    ret = ret.build_env("STAGELEFT_TRYBUILD_BUILD_STAGED", "1");
    ret = ret.config("build.incremental = false");

    if let Some(features) = features {
        ret = ret.features(features);
    }

    ret
}
