//! Deployment backend for Hydro that can generate manifests that can be consumed by CDK to deploy cloud formation stacks to aws.

use std::cell::RefCell;
use std::collections::HashMap;
use std::pin::Pin;
use std::rc::Rc;

use bytes::Bytes;
use dfir_lang::graph::DfirGraph;
use futures::{Sink, Stream};
use proc_macro2::Span;
use serde::{Deserialize, Serialize};
use stageleft::QuotedWithContext;
use syn::parse_quote;
use tracing::{instrument, trace};

/// Manifest for CDK deployment - describes all processes, clusters, and their configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HydroManifest {
    /// Process definitions (single-instance services)
    pub processes: HashMap<String, ProcessManifest>,
    /// Cluster definitions (multi-instance services)
    pub clusters: HashMap<String, ClusterManifest>,
}

/// Build configuration for a Hydro binary
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuildConfig {
    /// Path to the trybuild project directory
    pub project_dir: String,
    /// Path to the target directory
    pub target_dir: String,
    /// Example/binary name to build
    pub bin_name: String,
    /// Package name containing the example (for -p flag)
    pub package_name: String,
    /// Features to enable
    pub features: Vec<String>,
}

/// Information about an exposed port
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortInfo {
    /// The port number
    pub port: u16,
}

/// Manifest entry for a single process
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessManifest {
    /// Build configuration for this process
    pub build: BuildConfig,
    /// Internal location ID used for service discovery
    pub location_key: LocationKey,
    /// Ports that need to be exposed, keyed by external port identifier
    pub ports: HashMap<String, PortInfo>,
    /// Task family name (used for ECS service discovery)
    pub task_family: String,
}

/// Manifest entry for a cluster (multiple instances of the same service)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClusterManifest {
    /// Build configuration for this cluster (same binary for all instances)
    pub build: BuildConfig,
    /// Internal location ID used for service discovery
    pub location_key: LocationKey,
    /// Ports that need to be exposed
    pub ports: Vec<u16>,
    /// Default number of instances
    pub default_count: usize,
    /// Task family prefix (instances will be named {prefix}-0, {prefix}-1, etc.)
    pub task_family_prefix: String,
}

use super::deploy_runtime_containerized_ecs::*;
use crate::compile::builder::ExternalPortId;
use crate::compile::deploy::DeployResult;
use crate::compile::deploy_provider::{
    ClusterSpec, Deploy, ExternalSpec, Node, ProcessSpec, RegisterPort,
};
use crate::compile::trybuild::generate::create_graph_trybuild;
use crate::location::dynamic::LocationId;
use crate::location::member_id::TaglessMemberId;
use crate::location::{LocationKey, MembershipEvent, NetworkHint};

/// Represents a process running in an ecs deployment
#[derive(Clone)]
pub struct EcsDeployProcess {
    id: LocationKey,
    name: String,
    next_port: Rc<RefCell<u16>>,

    exposed_ports: Rc<RefCell<HashMap<String, PortInfo>>>,

    trybuild_config:
        Rc<RefCell<Option<(String, crate::compile::trybuild::generate::TrybuildConfig)>>>,
}

impl Node for EcsDeployProcess {
    type Port = u16;
    type Meta = ();
    type InstantiateEnv = EcsDeploy;

    #[instrument(level = "trace", skip_all, ret, fields(id = %self.id, name = self.name))]
    fn next_port(&self) -> Self::Port {
        let port = {
            let mut borrow = self.next_port.borrow_mut();
            let port = *borrow;
            *borrow += 1;
            port
        };

        port
    }

    #[instrument(level = "trace", skip_all, fields(id = %self.id, name = self.name))]
    fn update_meta(&self, _meta: &Self::Meta) {}

    #[instrument(level = "trace", skip_all, fields(id = %self.id, name = self.name, ?meta, extra_stmts = extra_stmts.len()))]
    fn instantiate(
        &self,
        _env: &mut Self::InstantiateEnv,
        meta: &mut Self::Meta,
        graph: DfirGraph,
        extra_stmts: &[syn::Stmt],
        sidecars: &[syn::Expr],
    ) {
        let (bin_name, config) = create_graph_trybuild(
            graph,
            extra_stmts,
            sidecars,
            Some(&self.name),
            crate::compile::trybuild::generate::DeployMode::Containerized,
            crate::compile::trybuild::generate::LinkingMode::Static,
        );

        // Store the trybuild config for CDK export
        *self.trybuild_config.borrow_mut() = Some((bin_name, config));
    }
}

/// Represents a logical cluster, which can be a variable amount of individual containers.
#[derive(Clone)]
pub struct EcsDeployCluster {
    id: LocationKey,
    name: String,
    next_port: Rc<RefCell<u16>>,

    count: usize,

    /// Stored trybuild config for CDK export
    trybuild_config:
        Rc<RefCell<Option<(String, crate::compile::trybuild::generate::TrybuildConfig)>>>,
}

impl Node for EcsDeployCluster {
    type Port = u16;
    type Meta = ();
    type InstantiateEnv = EcsDeploy;

    #[instrument(level = "trace", skip_all, ret, fields(id = %self.id, name = self.name))]
    fn next_port(&self) -> Self::Port {
        let port = {
            let mut borrow = self.next_port.borrow_mut();
            let port = *borrow;
            *borrow += 1;
            port
        };

        port
    }

    #[instrument(level = "trace", skip_all, fields(id = %self.id, name = self.name))]
    fn update_meta(&self, _meta: &Self::Meta) {}

    #[instrument(level = "trace", skip_all, fields(id = %self.id, name = self.name, extra_stmts = extra_stmts.len()))]
    fn instantiate(
        &self,
        _env: &mut Self::InstantiateEnv,
        _meta: &mut Self::Meta,
        graph: DfirGraph,
        extra_stmts: &[syn::Stmt],
        sidecars: &[syn::Expr],
    ) {
        let (bin_name, config) = create_graph_trybuild(
            graph,
            extra_stmts,
            sidecars,
            Some(&self.name),
            crate::compile::trybuild::generate::DeployMode::Containerized,
            crate::compile::trybuild::generate::LinkingMode::Static,
        );

        // Store the trybuild config for CDK export
        *self.trybuild_config.borrow_mut() = Some((bin_name, config));
    }
}

/// Represents an external process, outside the control of this deployment but still with some communication into this deployment.
#[derive(Clone, Debug)]
pub struct EcsDeployExternal {
    name: String,
    next_port: Rc<RefCell<u16>>,
}

impl Node for EcsDeployExternal {
    type Port = u16;
    type Meta = ();
    type InstantiateEnv = EcsDeploy;

    #[instrument(level = "trace", skip_all, ret, fields(name = self.name))]
    fn next_port(&self) -> Self::Port {
        let port = {
            let mut borrow = self.next_port.borrow_mut();
            let port = *borrow;
            *borrow += 1;
            port
        };

        port
    }

    #[instrument(level = "trace", skip_all, fields(name = self.name))]
    fn update_meta(&self, _meta: &Self::Meta) {}

    #[instrument(level = "trace", skip_all, fields(name = self.name, ?meta, extra_stmts = extra_stmts.len(), sidecars = sidecars.len()))]
    fn instantiate(
        &self,
        _env: &mut Self::InstantiateEnv,
        meta: &mut Self::Meta,
        graph: DfirGraph,
        extra_stmts: &[syn::Stmt],
        sidecars: &[syn::Expr],
    ) {
        trace!(name: "surface", surface = graph.surface_syntax_string());
    }
}

type DynSourceSink<Out, In, InErr> = (
    Pin<Box<dyn Stream<Item = Out>>>,
    Pin<Box<dyn Sink<In, Error = InErr>>>,
);

impl<'a> RegisterPort<'a, EcsDeploy> for EcsDeployExternal {
    #[instrument(level = "trace", skip_all, fields(name = self.name, %external_port_id, %port))]
    fn register(&self, external_port_id: ExternalPortId, port: Self::Port) {}

    #[expect(clippy::manual_async_fn, reason = "matches trait signature")]
    fn as_bytes_bidi(
        &self,
        _external_port_id: ExternalPortId,
    ) -> impl Future<
        Output = DynSourceSink<Result<bytes::BytesMut, std::io::Error>, Bytes, std::io::Error>,
    > + 'a {
        async { unimplemented!() }
    }

    #[expect(clippy::manual_async_fn, reason = "matches trait signature")]
    fn as_bincode_bidi<InT, OutT>(
        &self,
        _external_port_id: ExternalPortId,
    ) -> impl Future<Output = DynSourceSink<OutT, InT, std::io::Error>> + 'a
    where
        InT: Serialize + 'static,
        OutT: serde::de::DeserializeOwned + 'static,
    {
        async { unimplemented!() }
    }

    #[expect(clippy::manual_async_fn, reason = "matches trait signature")]
    fn as_bincode_sink<T>(
        &self,
        _external_port_id: ExternalPortId,
    ) -> impl Future<Output = Pin<Box<dyn Sink<T, Error = std::io::Error>>>> + 'a
    where
        T: Serialize + 'static,
    {
        async { unimplemented!() }
    }

    #[expect(clippy::manual_async_fn, reason = "matches trait signature")]
    fn as_bincode_source<T>(
        &self,
        _external_port_id: ExternalPortId,
    ) -> impl Future<Output = Pin<Box<dyn Stream<Item = T>>>> + 'a
    where
        T: serde::de::DeserializeOwned + 'static,
    {
        async { unimplemented!() }
    }
}

/// Represents an aws ecs deployment.
pub struct EcsDeploy;

impl Default for EcsDeploy {
    fn default() -> Self {
        Self::new()
    }
}

impl EcsDeploy {
    /// Creates a new ecs deployment.
    pub fn new() -> Self {
        Self
    }

    /// Add an internal ecs process to the deployment.
    pub fn add_ecs_process(&mut self) -> EcsDeployProcessSpec {
        EcsDeployProcessSpec
    }

    /// Add an internal ecs cluster to the deployment.
    pub fn add_ecs_cluster(&mut self, count: usize) -> EcsDeployClusterSpec {
        EcsDeployClusterSpec { count }
    }

    /// Add an external process to the deployment.
    pub fn add_external(&self, name: String) -> EcsDeployExternalSpec {
        EcsDeployExternalSpec { name }
    }

    /// Export deployment configuration for CDK consumption.
    ///
    /// This generates a manifest with build instructions that can be consumed
    /// by CDK constructs. CDK will handle building the binaries and Docker images.
    #[instrument(level = "trace", skip_all)]
    pub fn export_for_cdk(&self, nodes: &DeployResult<'_, Self>) -> HydroManifest {
        let mut manifest = HydroManifest {
            processes: HashMap::new(),
            clusters: HashMap::new(),
        };

        for (location_id, name_hint, process) in nodes.get_all_processes() {
            let raw_id = match location_id {
                LocationId::Process(id) => id,
                _ => unreachable!(),
            };
            let task_family = get_ecs_container_name(&process.name, None);
            let ports = process.exposed_ports.borrow().clone();

            let (bin_name, trybuild_config) = process
                .trybuild_config
                .borrow()
                .clone()
                .expect("trybuild_config should be set after instantiate");

            let mut features = vec!["hydro___feature_ecs_runtime".to_owned()];
            if let Some(extra_features) = trybuild_config.features {
                features.extend(extra_features);
            }

            let crate_name = trybuild_config
                .project_dir
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("unknown")
                .replace("_", "-");
            let package_name = format!("{}-hydro-trybuild", crate_name);

            manifest.processes.insert(
                name_hint.to_owned(),
                ProcessManifest {
                    build: BuildConfig {
                        project_dir: trybuild_config.project_dir.to_string_lossy().into_owned(),
                        target_dir: trybuild_config.target_dir.to_string_lossy().into_owned(),
                        bin_name,
                        package_name,
                        features,
                    },
                    location_key: raw_id,
                    ports,
                    task_family,
                },
            );
        }

        for (location_id, name_hint, cluster) in nodes.get_all_clusters() {
            let raw_id = match location_id {
                LocationId::Cluster(id) => id,
                _ => unreachable!(),
            };
            let task_family_prefix = cluster.name.clone();

            let (bin_name, trybuild_config) = cluster
                .trybuild_config
                .borrow()
                .clone()
                .expect("trybuild_config should be set after instantiate");

            let mut features = vec!["hydro___feature_ecs_runtime".to_owned()];
            if let Some(extra_features) = trybuild_config.features {
                features.extend(extra_features);
            }

            let crate_name = trybuild_config
                .project_dir
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("unknown")
                .replace("_", "-");
            let package_name = format!("{}-hydro-trybuild", crate_name);

            manifest.clusters.insert(
                name_hint.to_owned(),
                ClusterManifest {
                    build: BuildConfig {
                        project_dir: trybuild_config.project_dir.to_string_lossy().into_owned(),
                        target_dir: trybuild_config.target_dir.to_string_lossy().into_owned(),
                        bin_name,
                        package_name,
                        features,
                    },
                    location_key: raw_id,
                    ports: vec![],
                    default_count: cluster.count,
                    task_family_prefix,
                },
            );
        }

        manifest
    }
}

impl<'a> Deploy<'a> for EcsDeploy {
    type InstantiateEnv = Self;
    type Process = EcsDeployProcess;
    type Cluster = EcsDeployCluster;
    type External = EcsDeployExternal;
    type Meta = ();

    #[instrument(level = "trace", skip_all, fields(p1 = p1.name, %p1_port, p2 = p2.name, p2_port))]
    fn o2o_sink_source(
        p1: &Self::Process,
        p1_port: &<Self::Process as Node>::Port,
        p2: &Self::Process,
        p2_port: &<Self::Process as Node>::Port,
    ) -> (syn::Expr, syn::Expr) {
        deploy_containerized_o2o(&p2.name, *p2_port)
    }

    #[instrument(level = "trace", skip_all, fields(p1 = p1.name, %p1_port, p2 = p2.name, p2_port))]
    fn o2o_connect(
        p1: &Self::Process,
        p1_port: &<Self::Process as Node>::Port,
        p2: &Self::Process,
        p2_port: &<Self::Process as Node>::Port,
    ) -> Box<dyn FnOnce()> {
        let serialized = format!(
            "o2o_connect {}:{p1_port:?} -> {}:{p2_port:?}",
            p1.name, p2.name
        );

        Box::new(move || {
            trace!(name: "o2o_connect thunk", %serialized);
        })
    }

    #[instrument(level = "trace", skip_all, fields(p1 = p1.name, %p1_port, c2 = c2.name, %c2_port))]
    fn o2m_sink_source(
        p1: &Self::Process,
        p1_port: &<Self::Process as Node>::Port,
        c2: &Self::Cluster,
        c2_port: &<Self::Cluster as Node>::Port,
    ) -> (syn::Expr, syn::Expr) {
        deploy_containerized_o2m(*c2_port)
    }

    #[instrument(level = "trace", skip_all, fields(p1 = p1.name, %p1_port, c2 = c2.name, %c2_port))]
    fn o2m_connect(
        p1: &Self::Process,
        p1_port: &<Self::Process as Node>::Port,
        c2: &Self::Cluster,
        c2_port: &<Self::Cluster as Node>::Port,
    ) -> Box<dyn FnOnce()> {
        let serialized = format!(
            "o2m_connect {}:{p1_port:?} -> {}:{c2_port:?}",
            p1.name, c2.name
        );

        Box::new(move || {
            trace!(name: "o2m_connect thunk", %serialized);
        })
    }

    #[instrument(level = "trace", skip_all, fields(c1 = c1.name, %c1_port, p2 = p2.name, %p2_port))]
    fn m2o_sink_source(
        c1: &Self::Cluster,
        c1_port: &<Self::Cluster as Node>::Port,
        p2: &Self::Process,
        p2_port: &<Self::Process as Node>::Port,
    ) -> (syn::Expr, syn::Expr) {
        deploy_containerized_m2o(*p2_port, &p2.name)
    }

    #[instrument(level = "trace", skip_all, fields(c1 = c1.name, %c1_port, p2 = p2.name, %p2_port))]
    fn m2o_connect(
        c1: &Self::Cluster,
        c1_port: &<Self::Cluster as Node>::Port,
        p2: &Self::Process,
        p2_port: &<Self::Process as Node>::Port,
    ) -> Box<dyn FnOnce()> {
        let serialized = format!(
            "o2m_connect {}:{c1_port:?} -> {}:{p2_port:?}",
            c1.name, p2.name
        );

        Box::new(move || {
            trace!(name: "m2o_connect thunk", %serialized);
        })
    }

    #[instrument(level = "trace", skip_all, fields(c1 = c1.name, %c1_port, c2 = c2.name, %c2_port))]
    fn m2m_sink_source(
        c1: &Self::Cluster,
        c1_port: &<Self::Cluster as Node>::Port,
        c2: &Self::Cluster,
        c2_port: &<Self::Cluster as Node>::Port,
    ) -> (syn::Expr, syn::Expr) {
        deploy_containerized_m2m(*c2_port)
    }

    #[instrument(level = "trace", skip_all, fields(c1 = c1.name, %c1_port, c2 = c2.name, %c2_port))]
    fn m2m_connect(
        c1: &Self::Cluster,
        c1_port: &<Self::Cluster as Node>::Port,
        c2: &Self::Cluster,
        c2_port: &<Self::Cluster as Node>::Port,
    ) -> Box<dyn FnOnce()> {
        let serialized = format!(
            "m2m_connect {}:{c1_port:?} -> {}:{c2_port:?}",
            c1.name, c2.name
        );

        Box::new(move || {
            trace!(name: "m2m_connect thunk", %serialized);
        })
    }

    #[instrument(level = "trace", skip_all, fields(p2 = p2.name, %p2_port, %shared_handle, extra_stmts = extra_stmts.len()))]
    fn e2o_many_source(
        extra_stmts: &mut Vec<syn::Stmt>,
        p2: &Self::Process,
        p2_port: &<Self::Process as Node>::Port,
        codec_type: &syn::Type,
        shared_handle: String,
    ) -> syn::Expr {
        p2.exposed_ports
            .borrow_mut()
            .insert(shared_handle.clone(), PortInfo { port: *p2_port });

        let socket_ident = syn::Ident::new(
            &format!("__hydro_deploy_many_{}_socket", &shared_handle),
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

        let bind_addr = format!("0.0.0.0:{}", p2_port);

        extra_stmts.push(syn::parse_quote! {
            let #socket_ident = tokio::net::TcpListener::bind(#bind_addr).await.unwrap();
        });

        let root = crate::staging_util::get_this_crate();

        extra_stmts.push(syn::parse_quote! {
            let (#source_ident, #sink_ident, #membership_ident) = #root::runtime_support::hydro_deploy_integration::multi_connection::tcp_multi_connection::<_, #codec_type>(#socket_ident);
        });

        parse_quote!(#source_ident)
    }

    #[instrument(level = "trace", skip_all, fields(%shared_handle))]
    fn e2o_many_sink(shared_handle: String) -> syn::Expr {
        let sink_ident = syn::Ident::new(
            &format!("__hydro_deploy_many_{}_sink", &shared_handle),
            Span::call_site(),
        );
        parse_quote!(#sink_ident)
    }

    #[instrument(level = "trace", skip_all, fields(p1 = p1.name, %p1_port, p2 = p2.name, %p2_port, ?codec_type, %shared_handle))]
    fn e2o_source(
        extra_stmts: &mut Vec<syn::Stmt>,
        p1: &Self::External,
        p1_port: &<Self::External as Node>::Port,
        p2: &Self::Process,
        p2_port: &<Self::Process as Node>::Port,
        codec_type: &syn::Type,
        shared_handle: String,
    ) -> syn::Expr {
        // Record the port for manifest export
        p2.exposed_ports
            .borrow_mut()
            .insert(shared_handle.clone(), PortInfo { port: *p2_port });

        let source_ident = syn::Ident::new(
            &format!("__hydro_deploy_{}_source", &shared_handle),
            Span::call_site(),
        );

        let bind_addr = format!("0.0.0.0:{}", p2_port);

        // Always use LazySinkSource for external connections - it creates both sink and source
        // which is needed for bidirectional connections (unpaired: false)
        let socket_ident = syn::Ident::new(
            &format!("__hydro_deploy_{}_socket", &shared_handle),
            Span::call_site(),
        );

        let sink_ident = syn::Ident::new(
            &format!("__hydro_deploy_{}_sink", &shared_handle),
            Span::call_site(),
        );

        extra_stmts.push(syn::parse_quote! {
            let #socket_ident = tokio::net::TcpListener::bind(#bind_addr).await.unwrap();
        });

        let create_expr = deploy_containerized_external_sink_source_ident(bind_addr, socket_ident);

        extra_stmts.push(syn::parse_quote! {
            let (#sink_ident, #source_ident) = (#create_expr).split();
        });

        parse_quote!(#source_ident)
    }

    #[instrument(level = "trace", skip_all, fields(p1 = p1.name, %p1_port, p2 = p2.name, %p2_port, ?many, ?server_hint))]
    fn e2o_connect(
        p1: &Self::External,
        p1_port: &<Self::External as Node>::Port,
        p2: &Self::Process,
        p2_port: &<Self::Process as Node>::Port,
        many: bool,
        server_hint: NetworkHint,
    ) -> Box<dyn FnOnce()> {
        let serialized = format!(
            "e2o_connect {}:{p1_port:?} -> {}:{p2_port:?}",
            p1.name, p2.name
        );

        Box::new(move || {
            trace!(name: "e2o_connect thunk", %serialized);
        })
    }

    #[instrument(level = "trace", skip_all, fields(p1 = p1.name, %p1_port, p2 = p2.name, %p2_port, %shared_handle))]
    fn o2e_sink(
        p1: &Self::Process,
        p1_port: &<Self::Process as Node>::Port,
        p2: &Self::External,
        p2_port: &<Self::External as Node>::Port,
        shared_handle: String,
    ) -> syn::Expr {
        let sink_ident = syn::Ident::new(
            &format!("__hydro_deploy_{}_sink", &shared_handle),
            Span::call_site(),
        );
        parse_quote!(#sink_ident)
    }

    #[instrument(level = "trace", skip_all, fields(%of_cluster))]
    fn cluster_ids(
        of_cluster: LocationKey,
    ) -> impl QuotedWithContext<'a, &'a [TaglessMemberId], ()> + Clone + 'a {
        cluster_ids()
    }

    #[instrument(level = "trace", skip_all)]
    fn cluster_self_id() -> impl QuotedWithContext<'a, TaglessMemberId, ()> + Clone + 'a {
        cluster_self_id()
    }

    #[instrument(level = "trace", skip_all, fields(?location_id))]
    fn cluster_membership_stream(
        location_id: &LocationId,
    ) -> impl QuotedWithContext<'a, Box<dyn Stream<Item = (TaglessMemberId, MembershipEvent)> + Unpin>, ()>
    {
        cluster_membership_stream(location_id)
    }
}

#[instrument(level = "trace", skip_all, ret, fields(%name_hint, %location))]
fn get_ecs_image_name(name_hint: &str, location: LocationKey) -> String {
    let name_hint = name_hint
        .split("::")
        .last()
        .unwrap()
        .to_ascii_lowercase()
        .replace(".", "-")
        .replace("_", "-")
        .replace("::", "-");

    format!("hy-{name_hint}-{location}")
}

#[instrument(level = "trace", skip_all, ret, fields(%image_name, ?instance))]
fn get_ecs_container_name(image_name: &str, instance: Option<usize>) -> String {
    if let Some(instance) = instance {
        format!("{image_name}-{instance}")
    } else {
        image_name.to_owned()
    }
}
/// Represents a Process running in an ecs deployment
#[derive(Clone)]
pub struct EcsDeployProcessSpec;

impl<'a> ProcessSpec<'a, EcsDeploy> for EcsDeployProcessSpec {
    #[instrument(level = "trace", skip_all, fields(%id, %name_hint))]
    fn build(self, id: LocationKey, name_hint: &'_ str) -> <EcsDeploy as Deploy<'a>>::Process {
        EcsDeployProcess {
            id,
            name: get_ecs_image_name(name_hint, id),
            next_port: Rc::new(RefCell::new(1000)),
            exposed_ports: Rc::new(RefCell::new(HashMap::new())),
            trybuild_config: Rc::new(RefCell::new(None)),
        }
    }
}

/// Represents a Cluster running across `count` ecs tasks.
#[derive(Clone)]
pub struct EcsDeployClusterSpec {
    count: usize,
}

impl<'a> ClusterSpec<'a, EcsDeploy> for EcsDeployClusterSpec {
    #[instrument(level = "trace", skip_all, fields(%id, %name_hint))]
    fn build(self, id: LocationKey, name_hint: &str) -> <EcsDeploy as Deploy<'a>>::Cluster {
        EcsDeployCluster {
            id,
            name: get_ecs_image_name(name_hint, id),
            next_port: Rc::new(RefCell::new(1000)),
            count: self.count,
            trybuild_config: Rc::new(RefCell::new(None)),
        }
    }
}

/// Represents an external process outside of the management of hydro deploy.
pub struct EcsDeployExternalSpec {
    name: String,
}

impl<'a> ExternalSpec<'a, EcsDeploy> for EcsDeployExternalSpec {
    #[instrument(level = "trace", skip_all, fields(%id, %name_hint))]
    fn build(self, id: LocationKey, name_hint: &str) -> <EcsDeploy as Deploy<'a>>::External {
        EcsDeployExternal {
            name: self.name,
            next_port: Rc::new(RefCell::new(10000)),
        }
    }
}
