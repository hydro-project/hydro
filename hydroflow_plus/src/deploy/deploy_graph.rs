use std::cell::RefCell;
use std::collections::HashMap;
use std::future::Future;
use std::io::Error;
use std::pin::Pin;
use std::rc::Rc;
use std::sync::Arc;

use hydro_deploy::custom_service::CustomClientPort;
use hydro_deploy::hydroflow_crate::ports::{
    DemuxSink, HydroflowSink, HydroflowSource, TaggedSource,
};
use hydro_deploy::hydroflow_crate::tracing_options::TracingOptions;
use hydro_deploy::hydroflow_crate::HydroflowCrateService;
use hydro_deploy::{CustomService, Deployment, Host, HydroflowCrate};
use hydroflow::futures::StreamExt;
use hydroflow::util::deploy::{ConnectedSink, ConnectedSource};
use nameof::name_of;
use serde::de::DeserializeOwned;
use serde::Serialize;
use sha2::{Digest, Sha256};
use stageleft::{Quoted, RuntimeData};
use syn::visit_mut::VisitMut;
use tokio::sync::RwLock;
use trybuild_internals_api::path;

use super::deploy_runtime::*;
use super::trybuild::{compile_graph_trybuild, create_trybuild};
use super::{ClusterSpec, Deploy, ExternalSpec, Node, ProcessSpec, RegisterPort};
use crate::futures::SinkExt;
use crate::lang::graph::HydroflowGraph;

pub struct HydroDeploy {}

impl<'a> Deploy<'a> for HydroDeploy {
    type InstantiateEnv = Deployment;
    type CompileEnv = ();
    type Process = DeployNode;
    type Cluster = DeployCluster;
    type ExternalProcess = DeployExternal;
    type Meta = HashMap<usize, Vec<u32>>;
    type GraphId = ();
    type Port = String;
    type ExternalRawPort = CustomClientPort;

    fn allocate_process_port(process: &Self::Process) -> Self::Port {
        process.next_port()
    }

    fn allocate_cluster_port(cluster: &Self::Cluster) -> Self::Port {
        cluster.next_port()
    }

    fn allocate_external_port(external: &Self::ExternalProcess) -> Self::Port {
        external.next_port()
    }

    fn o2o_sink_source(
        _env: &(),
        _p1: &Self::Process,
        p1_port: &Self::Port,
        _p2: &Self::Process,
        p2_port: &Self::Port,
    ) -> (syn::Expr, syn::Expr) {
        let p1_port = p1_port.as_str();
        let p2_port = p2_port.as_str();
        deploy_o2o(
            RuntimeData::new("__hydroflow_plus_trybuild_cli"),
            p1_port,
            p2_port,
        )
    }

    fn o2o_connect(
        p1: &Self::Process,
        p1_port: &Self::Port,
        p2: &Self::Process,
        p2_port: &Self::Port,
    ) -> Box<dyn FnOnce()> {
        let p1 = p1.clone();
        let p1_port = p1_port.clone();
        let p2 = p2.clone();
        let p2_port = p2_port.clone();

        Box::new(move || {
            let self_underlying_borrow = p1.underlying.borrow();
            let self_underlying = self_underlying_borrow.as_ref().unwrap();
            let source_port = self_underlying
                .try_read()
                .unwrap()
                .get_port(p1_port.clone(), self_underlying);

            let other_underlying_borrow = p2.underlying.borrow();
            let other_underlying = other_underlying_borrow.as_ref().unwrap();
            let recipient_port = other_underlying
                .try_read()
                .unwrap()
                .get_port(p2_port.clone(), other_underlying);

            source_port.send_to(&recipient_port)
        })
    }

    fn o2m_sink_source(
        _env: &(),
        _p1: &Self::Process,
        p1_port: &Self::Port,
        _c2: &Self::Cluster,
        c2_port: &Self::Port,
    ) -> (syn::Expr, syn::Expr) {
        let p1_port = p1_port.as_str();
        let c2_port = c2_port.as_str();
        deploy_o2m(
            RuntimeData::new("__hydroflow_plus_trybuild_cli"),
            p1_port,
            c2_port,
        )
    }

    fn o2m_connect(
        p1: &Self::Process,
        p1_port: &Self::Port,
        c2: &Self::Cluster,
        c2_port: &Self::Port,
    ) -> Box<dyn FnOnce()> {
        let p1 = p1.clone();
        let p1_port = p1_port.clone();
        let c2 = c2.clone();
        let c2_port = c2_port.clone();

        Box::new(move || {
            let self_underlying_borrow = p1.underlying.borrow();
            let self_underlying = self_underlying_borrow.as_ref().unwrap();
            let source_port = self_underlying
                .try_read()
                .unwrap()
                .get_port(p1_port.clone(), self_underlying);

            let recipient_port = DemuxSink {
                demux: c2
                    .members
                    .borrow()
                    .iter()
                    .enumerate()
                    .map(|(id, c)| {
                        let n = c.underlying.try_read().unwrap();
                        (
                            id as u32,
                            Arc::new(n.get_port(c2_port.clone(), &c.underlying))
                                as Arc<dyn HydroflowSink + 'static>,
                        )
                    })
                    .collect(),
            };

            source_port.send_to(&recipient_port)
        })
    }

    fn m2o_sink_source(
        _env: &(),
        _c1: &Self::Cluster,
        c1_port: &Self::Port,
        _p2: &Self::Process,
        p2_port: &Self::Port,
    ) -> (syn::Expr, syn::Expr) {
        let c1_port = c1_port.as_str();
        let p2_port = p2_port.as_str();
        deploy_m2o(
            RuntimeData::new("__hydroflow_plus_trybuild_cli"),
            c1_port,
            p2_port,
        )
    }

    fn m2o_connect(
        c1: &Self::Cluster,
        c1_port: &Self::Port,
        p2: &Self::Process,
        p2_port: &Self::Port,
    ) -> Box<dyn FnOnce()> {
        let c1 = c1.clone();
        let c1_port = c1_port.clone();
        let p2 = p2.clone();
        let p2_port = p2_port.clone();

        Box::new(move || {
            let other_underlying_borrow = p2.underlying.borrow();
            let other_underlying = other_underlying_borrow.as_ref().unwrap();
            let recipient_port = other_underlying
                .try_read()
                .unwrap()
                .get_port(p2_port.clone(), other_underlying)
                .merge();

            for (i, node) in c1.members.borrow().iter().enumerate() {
                let source_port = node
                    .underlying
                    .try_read()
                    .unwrap()
                    .get_port(c1_port.clone(), &node.underlying);

                TaggedSource {
                    source: Arc::new(source_port),
                    tag: i as u32,
                }
                .send_to(&recipient_port);
            }
        })
    }

    fn m2m_sink_source(
        _env: &(),
        _c1: &Self::Cluster,
        c1_port: &Self::Port,
        _c2: &Self::Cluster,
        c2_port: &Self::Port,
    ) -> (syn::Expr, syn::Expr) {
        let c1_port = c1_port.as_str();
        let c2_port = c2_port.as_str();
        deploy_m2m(
            RuntimeData::new("__hydroflow_plus_trybuild_cli"),
            c1_port,
            c2_port,
        )
    }

    fn m2m_connect(
        c1: &Self::Cluster,
        c1_port: &Self::Port,
        c2: &Self::Cluster,
        c2_port: &Self::Port,
    ) -> Box<dyn FnOnce()> {
        let c1 = c1.clone();
        let c1_port = c1_port.clone();
        let c2 = c2.clone();
        let c2_port = c2_port.clone();

        Box::new(move || {
            for (i, sender) in c1.members.borrow().iter().enumerate() {
                let source_port = sender
                    .underlying
                    .try_read()
                    .unwrap()
                    .get_port(c1_port.clone(), &sender.underlying);

                let recipient_port = DemuxSink {
                    demux: c2
                        .members
                        .borrow()
                        .iter()
                        .enumerate()
                        .map(|(id, c)| {
                            let n = c.underlying.try_read().unwrap();
                            (
                                id as u32,
                                Arc::new(n.get_port(c2_port.clone(), &c.underlying).merge())
                                    as Arc<dyn HydroflowSink + 'static>,
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

    fn e2o_source(
        _compile_env: &Self::CompileEnv,
        _p1: &Self::ExternalProcess,
        p1_port: &Self::Port,
        _p2: &Self::Process,
        p2_port: &Self::Port,
    ) -> syn::Expr {
        let p1_port = p1_port.as_str();
        let p2_port = p2_port.as_str();
        deploy_e2o(
            RuntimeData::new("__hydroflow_plus_trybuild_cli"),
            p1_port,
            p2_port,
        )
    }

    fn e2o_connect(
        p1: &Self::ExternalProcess,
        p1_port: &Self::Port,
        p2: &Self::Process,
        p2_port: &Self::Port,
    ) -> Box<dyn FnOnce()> {
        let p1 = p1.clone();
        let p1_port = p1_port.clone();
        let p2 = p2.clone();
        let p2_port = p2_port.clone();

        Box::new(move || {
            let self_underlying_borrow = p1.underlying.borrow();
            let self_underlying = self_underlying_borrow.as_ref().unwrap();
            let source_port = self_underlying
                .try_read()
                .unwrap()
                .declare_client(self_underlying);

            let other_underlying_borrow = p2.underlying.borrow();
            let other_underlying = other_underlying_borrow.as_ref().unwrap();
            let recipient_port = other_underlying
                .try_read()
                .unwrap()
                .get_port(p2_port.clone(), other_underlying);

            source_port.send_to(&recipient_port);

            p1.client_ports
                .borrow_mut()
                .insert(p1_port.clone(), source_port);
        })
    }

    fn o2e_sink(
        _compile_env: &Self::CompileEnv,
        _p1: &Self::Process,
        p1_port: &Self::Port,
        _p2: &Self::ExternalProcess,
        p2_port: &Self::Port,
    ) -> syn::Expr {
        let p1_port = p1_port.as_str();
        let p2_port = p2_port.as_str();
        deploy_o2e(
            RuntimeData::new("__hydroflow_plus_trybuild_cli"),
            p1_port,
            p2_port,
        )
    }

    fn o2e_connect(
        p1: &Self::Process,
        p1_port: &Self::Port,
        p2: &Self::ExternalProcess,
        p2_port: &Self::Port,
    ) -> Box<dyn FnOnce()> {
        let p1 = p1.clone();
        let p1_port = p1_port.clone();
        let p2 = p2.clone();
        let p2_port = p2_port.clone();

        Box::new(move || {
            let self_underlying_borrow = p1.underlying.borrow();
            let self_underlying = self_underlying_borrow.as_ref().unwrap();
            let source_port = self_underlying
                .try_read()
                .unwrap()
                .get_port(p1_port.clone(), self_underlying);

            let other_underlying_borrow = p2.underlying.borrow();
            let other_underlying = other_underlying_borrow.as_ref().unwrap();
            let recipient_port = other_underlying
                .try_read()
                .unwrap()
                .declare_client(other_underlying);

            source_port.send_to(&recipient_port);

            p2.client_ports
                .borrow_mut()
                .insert(p2_port.clone(), recipient_port);
        })
    }

    fn cluster_ids(
        _env: &Self::CompileEnv,
        of_cluster: usize,
    ) -> impl Quoted<'a, &'a Vec<u32>> + Copy + 'a {
        cluster_members(
            RuntimeData::new("__hydroflow_plus_trybuild_cli"),
            of_cluster,
        )
    }

    fn cluster_self_id(_env: &Self::CompileEnv) -> impl Quoted<'a, u32> + Copy + 'a {
        cluster_self_id(RuntimeData::new("__hydroflow_plus_trybuild_cli"))
    }
}

pub trait DeployCrateWrapper {
    fn underlying(&self) -> Arc<RwLock<HydroflowCrateService>>;

    #[expect(async_fn_in_trait, reason = "no auto trait bounds needed")]
    async fn stdout(&self) -> tokio::sync::mpsc::UnboundedReceiver<String> {
        self.underlying().read().await.stdout()
    }

    #[expect(async_fn_in_trait, reason = "no auto trait bounds needed")]
    async fn stderr(&self) -> tokio::sync::mpsc::UnboundedReceiver<String> {
        self.underlying().read().await.stderr()
    }
}

#[derive(Clone)]
pub struct TrybuildHost {
    pub host: Arc<dyn Host>,
    pub display_name: Option<String>,
    pub rustflags: Option<String>,
    pub tracing: Option<TracingOptions>,
    pub name_hint: Option<String>,
    pub cluster_idx: Option<usize>,
}

impl TrybuildHost {
    pub fn new(host: Arc<dyn Host>) -> Self {
        Self {
            host,
            display_name: None,
            rustflags: None,
            tracing: None,
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

    pub fn tracing(self, tracing: TracingOptions) -> Self {
        if self.tracing.is_some() {
            panic!("{} already set", name_of!(tracing in Self));
        }

        Self {
            tracing: Some(tracing),
            ..self
        }
    }
}

impl From<Arc<dyn Host>> for TrybuildHost {
    fn from(h: Arc<dyn Host>) -> Self {
        Self {
            host: h,
            display_name: None,
            rustflags: None,
            tracing: None,
            name_hint: None,
            cluster_idx: None,
        }
    }
}

#[derive(Clone)]
pub struct DeployExternal {
    next_port: Rc<RefCell<usize>>,
    host: Arc<dyn Host>,
    underlying: Rc<RefCell<Option<Arc<RwLock<CustomService>>>>>,
    client_ports: Rc<RefCell<HashMap<String, CustomClientPort>>>,
    allocated_ports: Rc<RefCell<HashMap<usize, String>>>,
}

impl DeployExternal {
    pub fn take_port(&self, key: usize) -> CustomClientPort {
        self.client_ports
            .borrow_mut()
            .remove(self.allocated_ports.borrow().get(&key).unwrap())
            .unwrap()
    }
}

impl<'a> RegisterPort<'a, HydroDeploy> for DeployExternal {
    fn register(&self, key: usize, port: <HydroDeploy as Deploy>::Port) {
        self.allocated_ports.borrow_mut().insert(key, port);
    }

    fn raw_port(&self, key: usize) -> <HydroDeploy as Deploy>::ExternalRawPort {
        self.client_ports
            .borrow_mut()
            .remove(self.allocated_ports.borrow().get(&key).unwrap())
            .unwrap()
    }

    fn as_bytes_sink(
        &self,
        key: usize,
    ) -> impl Future<Output = Pin<Box<dyn crate::futures::Sink<crate::bytes::Bytes, Error = Error>>>> + 'a
    {
        let port = self.raw_port(key);
        async move {
            let sink = port.connect().await.into_sink();
            sink as Pin<Box<dyn crate::futures::Sink<crate::bytes::Bytes, Error = Error>>>
        }
    }

    fn as_bincode_sink<T: Serialize + 'static>(
        &self,
        key: usize,
    ) -> impl Future<Output = Pin<Box<dyn crate::futures::Sink<T, Error = Error>>>> + 'a {
        let port = self.raw_port(key);
        async move {
            let sink = port.connect().await.into_sink();
            Box::pin(sink.with(|item| async move { Ok(bincode::serialize(&item).unwrap().into()) }))
                as Pin<Box<dyn crate::futures::Sink<T, Error = Error>>>
        }
    }

    fn as_bytes_source(
        &self,
        key: usize,
    ) -> impl Future<Output = Pin<Box<dyn crate::futures::Stream<Item = crate::bytes::Bytes>>>> + 'a
    {
        let port = self.raw_port(key);
        async move {
            let source = port.connect().await.into_source();
            Box::pin(source.map(|r| r.unwrap().freeze()))
                as Pin<Box<dyn crate::futures::Stream<Item = crate::bytes::Bytes>>>
        }
    }

    fn as_bincode_source<T: DeserializeOwned + 'static>(
        &self,
        key: usize,
    ) -> impl Future<Output = Pin<Box<dyn crate::futures::Stream<Item = T>>>> + 'a {
        let port = self.raw_port(key);
        async move {
            let source = port.connect().await.into_source();
            Box::pin(source.map(|item| bincode::deserialize(&item.unwrap()).unwrap()))
                as Pin<Box<dyn crate::futures::Stream<Item = T>>>
        }
    }
}

impl Node for DeployExternal {
    type Port = String;
    type Meta = HashMap<usize, Vec<u32>>;
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
        _graph: HydroflowGraph,
        _extra_stmts: Vec<syn::Stmt>,
    ) {
        let service = env.CustomService(self.host.clone(), vec![]);
        *self.underlying.borrow_mut() = Some(service);
    }

    fn update_meta(&mut self, _meta: &Self::Meta) {}
}

impl<'a> ExternalSpec<'a, HydroDeploy> for Arc<dyn Host> {
    fn build(self, _id: usize, _name_hint: &str) -> DeployExternal {
        DeployExternal {
            next_port: Rc::new(RefCell::new(0)),
            host: self,
            underlying: Rc::new(RefCell::new(None)),
            allocated_ports: Rc::new(RefCell::new(HashMap::new())),
            client_ports: Rc::new(RefCell::new(HashMap::new())),
        }
    }
}

pub enum CrateOrTrybuild {
    Crate(HydroflowCrate),
    Trybuild(TrybuildHost),
}

#[derive(Clone)]
pub struct DeployNode {
    id: usize,
    next_port: Rc<RefCell<usize>>,
    service_spec: Rc<RefCell<Option<CrateOrTrybuild>>>,
    underlying: Rc<RefCell<Option<Arc<RwLock<HydroflowCrateService>>>>>,
}

impl DeployCrateWrapper for DeployNode {
    fn underlying(&self) -> Arc<RwLock<HydroflowCrateService>> {
        self.underlying.borrow().as_ref().unwrap().clone()
    }
}

impl Node for DeployNode {
    type Port = String;
    type Meta = HashMap<usize, Vec<u32>>;
    type InstantiateEnv = Deployment;

    fn next_port(&self) -> String {
        let next_port = *self.next_port.borrow();
        *self.next_port.borrow_mut() += 1;

        format!("port_{}", next_port)
    }

    fn update_meta(&mut self, meta: &Self::Meta) {
        let underlying_node = self.underlying.borrow();
        let mut n = underlying_node.as_ref().unwrap().try_write().unwrap();
        n.update_meta(HydroflowPlusMeta {
            clusters: meta.clone(),
            cluster_id: None,
            subgraph_id: self.id,
        });
    }

    fn instantiate(
        &self,
        env: &mut Self::InstantiateEnv,
        _meta: &mut Self::Meta,
        graph: HydroflowGraph,
        extra_stmts: Vec<syn::Stmt>,
    ) {
        let service = match self.service_spec.borrow_mut().take().unwrap() {
            CrateOrTrybuild::Crate(c) => c,
            CrateOrTrybuild::Trybuild(trybuild) => {
                let (bin_name, (dir, target_dir, features)) =
                    create_graph_trybuild(graph, extra_stmts, &trybuild.name_hint);
                create_trybuild_service(trybuild, &dir, &target_dir, &features, &bin_name)
            }
        };

        *self.underlying.borrow_mut() = Some(env.add_service(service));
    }
}

#[derive(Clone)]
pub struct DeployClusterNode {
    underlying: Arc<RwLock<HydroflowCrateService>>,
}

impl DeployCrateWrapper for DeployClusterNode {
    fn underlying(&self) -> Arc<RwLock<HydroflowCrateService>> {
        self.underlying.clone()
    }
}

#[derive(Clone)]
pub struct DeployCluster {
    id: usize,
    next_port: Rc<RefCell<usize>>,
    cluster_spec: Rc<RefCell<Option<Vec<CrateOrTrybuild>>>>,
    members: Rc<RefCell<Vec<DeployClusterNode>>>,
    name_hint: Option<String>,
}

impl DeployCluster {
    pub fn members(&self) -> Vec<DeployClusterNode> {
        self.members.borrow().clone()
    }
}

impl Node for DeployCluster {
    type Port = String;
    type Meta = HashMap<usize, Vec<u32>>;
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
        graph: HydroflowGraph,
        extra_stmts: Vec<syn::Stmt>,
    ) {
        let has_trybuild = self
            .cluster_spec
            .borrow()
            .as_ref()
            .unwrap()
            .iter()
            .any(|spec| matches!(spec, CrateOrTrybuild::Trybuild { .. }));

        let maybe_trybuild = if has_trybuild {
            Some(create_graph_trybuild(graph, extra_stmts, &self.name_hint))
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
                let service = match spec {
                    CrateOrTrybuild::Crate(c) => c,
                    CrateOrTrybuild::Trybuild(trybuild) => {
                        let (bin_name, (dir, target_dir, features)) =
                            maybe_trybuild.as_ref().unwrap();
                        create_trybuild_service(trybuild, dir, target_dir, features, bin_name)
                    }
                };

                env.add_service(service)
            })
            .collect::<Vec<_>>();
        meta.insert(self.id, (0..(cluster_nodes.len() as u32)).collect());
        *self.members.borrow_mut() = cluster_nodes
            .into_iter()
            .map(|n| DeployClusterNode { underlying: n })
            .collect();
    }

    fn update_meta(&mut self, meta: &Self::Meta) {
        for (cluster_id, node) in self.members.borrow().iter().enumerate() {
            let mut n = node.underlying.try_write().unwrap();
            n.update_meta(HydroflowPlusMeta {
                clusters: meta.clone(),
                cluster_id: Some(cluster_id as u32),
                subgraph_id: self.id,
            });
        }
    }
}

#[derive(Clone)]
pub struct DeployProcessSpec(HydroflowCrate);

impl DeployProcessSpec {
    pub fn new(t: HydroflowCrate) -> Self {
        Self(t)
    }
}

impl<'a> ProcessSpec<'a, HydroDeploy> for DeployProcessSpec {
    fn build(self, id: usize, _name_hint: &str) -> DeployNode {
        DeployNode {
            id,
            next_port: Rc::new(RefCell::new(0)),
            service_spec: Rc::new(RefCell::new(Some(CrateOrTrybuild::Crate(self.0)))),
            underlying: Rc::new(RefCell::new(None)),
        }
    }
}

impl<'a> ProcessSpec<'a, HydroDeploy> for TrybuildHost {
    fn build(mut self, id: usize, name_hint: &str) -> DeployNode {
        self.name_hint = Some(format!("{} (process {id})", name_hint));
        DeployNode {
            id,
            next_port: Rc::new(RefCell::new(0)),
            service_spec: Rc::new(RefCell::new(Some(CrateOrTrybuild::Trybuild(self)))),
            underlying: Rc::new(RefCell::new(None)),
        }
    }
}

#[derive(Clone)]
pub struct DeployClusterSpec(Vec<HydroflowCrate>);

impl DeployClusterSpec {
    pub fn new(crates: Vec<HydroflowCrate>) -> Self {
        Self(crates)
    }
}

impl<'a> ClusterSpec<'a, HydroDeploy> for DeployClusterSpec {
    fn build(self, id: usize, _name_hint: &str) -> DeployCluster {
        DeployCluster {
            id,
            next_port: Rc::new(RefCell::new(0)),
            cluster_spec: Rc::new(RefCell::new(Some(
                self.0.into_iter().map(CrateOrTrybuild::Crate).collect(),
            ))),
            members: Rc::new(RefCell::new(vec![])),
            name_hint: None,
        }
    }
}

impl<'a> ClusterSpec<'a, HydroDeploy> for Vec<TrybuildHost> {
    fn build(self, id: usize, name_hint: &str) -> DeployCluster {
        let name_hint = format!("{} (cluster {id})", name_hint);
        DeployCluster {
            id,
            next_port: Rc::new(RefCell::new(0)),
            cluster_spec: Rc::new(RefCell::new(Some(
                self.into_iter()
                    .enumerate()
                    .map(|(idx, mut b)| {
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

fn clean_name_hint(name_hint: &str) -> String {
    name_hint
        .replace("::", "__")
        .replace(" ", "_")
        .replace(",", "_")
        .replace("<", "_")
        .replace(">", "")
        .replace("(", "")
        .replace(")", "")
}

// TODO(shadaj): has to be public due to stageleft limitations
#[doc(hidden)]
pub struct ReplaceCrateNameWithStaged {
    pub crate_name: String,
}

impl VisitMut for ReplaceCrateNameWithStaged {
    fn visit_type_path_mut(&mut self, i: &mut syn::TypePath) {
        if let Some(first) = i.path.segments.first() {
            if first.ident == self.crate_name {
                let tail = i.path.segments.iter().skip(1).collect::<Vec<_>>();
                *i = syn::parse_quote!(crate::__staged #(::#tail)*);
            }
        }

        syn::visit_mut::visit_type_path_mut(self, i);
    }
}

// TODO(shadaj): has to be public due to stageleft limitations
#[doc(hidden)]
pub struct ReplaceCrateWithOrig {
    pub crate_name: String,
}

impl VisitMut for ReplaceCrateWithOrig {
    fn visit_item_use_mut(&mut self, i: &mut syn::ItemUse) {
        if let syn::UseTree::Path(p) = &mut i.tree {
            if p.ident == "crate" {
                p.ident = syn::Ident::new(&self.crate_name, p.ident.span());
                i.leading_colon = Some(Default::default());
            }
        }

        syn::visit_mut::visit_item_use_mut(self, i);
    }
}

fn create_graph_trybuild(
    graph: HydroflowGraph,
    extra_stmts: Vec<syn::Stmt>,
    name_hint: &Option<String>,
) -> (
    String,
    (std::path::PathBuf, std::path::PathBuf, Option<Vec<String>>),
) {
    let source_dir = trybuild_internals_api::cargo::manifest_dir().unwrap();
    let source_manifest = trybuild_internals_api::dependencies::get_manifest(&source_dir).unwrap();
    let crate_name = &source_manifest.package.name.to_string().replace("-", "_");

    let is_test = super::trybuild::IS_TEST.load(std::sync::atomic::Ordering::Relaxed);

    let mut generated_code = compile_graph_trybuild(graph, extra_stmts);

    ReplaceCrateNameWithStaged {
        crate_name: crate_name.clone(),
    }
    .visit_file_mut(&mut generated_code);

    let mut inlined_staged = stageleft_tool::gen_staged_trybuild(
        &path!(source_dir / "src" / "lib.rs"),
        crate_name.clone(),
        is_test,
    );

    ReplaceCrateWithOrig {
        crate_name: crate_name.clone(),
    }
    .visit_file_mut(&mut inlined_staged);

    let source = prettyplease::unparse(&syn::parse_quote! {
        #generated_code

        #[allow(
            unused,
            ambiguous_glob_reexports,
            clippy::suspicious_else_formatting,
            unexpected_cfgs,
            reason = "generated code"
        )]
        pub mod __staged {
            #inlined_staged
        }
    });

    let mut hasher = Sha256::new();
    hasher.update(&source);
    let hash = format!("{:X}", hasher.finalize())
        .chars()
        .take(8)
        .collect::<String>();

    let bin_name = if let Some(name_hint) = &name_hint {
        format!("{}_{}", clean_name_hint(name_hint), &hash)
    } else {
        hash
    };

    let trybuild_created = create_trybuild(&source, &bin_name, is_test).unwrap();
    (bin_name, trybuild_created)
}

fn create_trybuild_service(
    trybuild: TrybuildHost,
    dir: &std::path::PathBuf,
    target_dir: &std::path::PathBuf,
    features: &Option<Vec<String>>,
    bin_name: &str,
) -> HydroflowCrate {
    let mut ret = HydroflowCrate::new(dir, trybuild.host)
        .target_dir(target_dir)
        .bin(bin_name)
        .no_default_features();

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

    if let Some(tracing) = trybuild.tracing {
        ret = ret.tracing(tracing);
    }

    if let Some(features) = features {
        ret = ret.features(features.clone());
    }

    ret
}
