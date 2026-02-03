//! Deployment backend for Hydro that targets Maelstrom for distributed systems testing.
//!
//! Maelstrom is a workbench for learning distributed systems by writing your own.
//! This backend compiles Hydro programs to binaries that communicate via Maelstrom's
//! stdin/stdout JSON protocol.

use std::cell::RefCell;
use std::future::Future;
use std::io::{BufRead, BufReader, Error};
use std::path::PathBuf;
use std::pin::Pin;
use std::process::Stdio;
use std::rc::Rc;

use bytes::{Bytes, BytesMut};
use dfir_lang::graph::DfirGraph;
use futures::{Sink, Stream};
use serde::Serialize;
use serde::de::DeserializeOwned;
use stageleft::{QuotedWithContext, RuntimeData};

use super::deploy_runtime_maelstrom::*;
use crate::compile::builder::ExternalPortId;
use crate::compile::deploy_provider::{ClusterSpec, Deploy, Node, RegisterPort};
use crate::compile::trybuild::generate::{LinkingMode, create_graph_trybuild};
use crate::location::dynamic::LocationId;
use crate::location::member_id::TaglessMemberId;
use crate::location::{LocationKey, MembershipEvent, NetworkHint};

/// Deployment backend that targets Maelstrom for distributed systems testing.
///
/// This backend compiles Hydro programs to binaries that communicate via Maelstrom's
/// stdin/stdout JSON protocol. It is restricted to programs with:
/// - Exactly one cluster (no processes)
/// - A single external input channel for client communication
pub enum MaelstromDeploy {}

impl<'a> Deploy<'a> for MaelstromDeploy {
    type Meta = ();
    type InstantiateEnv = MaelstromDeployment;

    type Process = MaelstromProcess;
    type Cluster = MaelstromCluster;
    type External = MaelstromExternal;

    fn o2o_sink_source(
        _p1: &Self::Process,
        _p1_port: &<Self::Process as Node>::Port,
        _p2: &Self::Process,
        _p2_port: &<Self::Process as Node>::Port,
    ) -> (syn::Expr, syn::Expr) {
        panic!("Maelstrom deployment does not support processes, only clusters")
    }

    fn o2o_connect(
        _p1: &Self::Process,
        _p1_port: &<Self::Process as Node>::Port,
        _p2: &Self::Process,
        _p2_port: &<Self::Process as Node>::Port,
    ) -> Box<dyn FnOnce()> {
        panic!("Maelstrom deployment does not support processes, only clusters")
    }

    fn o2m_sink_source(
        _p1: &Self::Process,
        _p1_port: &<Self::Process as Node>::Port,
        _c2: &Self::Cluster,
        _c2_port: &<Self::Cluster as Node>::Port,
    ) -> (syn::Expr, syn::Expr) {
        panic!("Maelstrom deployment does not support processes, only clusters")
    }

    fn o2m_connect(
        _p1: &Self::Process,
        _p1_port: &<Self::Process as Node>::Port,
        _c2: &Self::Cluster,
        _c2_port: &<Self::Cluster as Node>::Port,
    ) -> Box<dyn FnOnce()> {
        panic!("Maelstrom deployment does not support processes, only clusters")
    }

    fn m2o_sink_source(
        _c1: &Self::Cluster,
        _c1_port: &<Self::Cluster as Node>::Port,
        _p2: &Self::Process,
        _p2_port: &<Self::Process as Node>::Port,
    ) -> (syn::Expr, syn::Expr) {
        panic!("Maelstrom deployment does not support processes, only clusters")
    }

    fn m2o_connect(
        _c1: &Self::Cluster,
        _c1_port: &<Self::Cluster as Node>::Port,
        _p2: &Self::Process,
        _p2_port: &<Self::Process as Node>::Port,
    ) -> Box<dyn FnOnce()> {
        panic!("Maelstrom deployment does not support processes, only clusters")
    }

    fn m2m_sink_source(
        _c1: &Self::Cluster,
        _c1_port: &<Self::Cluster as Node>::Port,
        _c2: &Self::Cluster,
        _c2_port: &<Self::Cluster as Node>::Port,
    ) -> (syn::Expr, syn::Expr) {
        deploy_maelstrom_m2m(RuntimeData::new("__hydro_lang_maelstrom_meta"))
    }

    fn m2m_connect(
        _c1: &Self::Cluster,
        _c1_port: &<Self::Cluster as Node>::Port,
        _c2: &Self::Cluster,
        _c2_port: &<Self::Cluster as Node>::Port,
    ) -> Box<dyn FnOnce()> {
        // No runtime connection needed for Maelstrom - all routing is via stdin/stdout
        Box::new(|| {})
    }

    fn e2o_many_source(
        _extra_stmts: &mut Vec<syn::Stmt>,
        _p2: &Self::Process,
        _p2_port: &<Self::Process as Node>::Port,
        _codec_type: &syn::Type,
        _shared_handle: String,
    ) -> syn::Expr {
        panic!("Maelstrom deployment does not support processes, only clusters")
    }

    fn e2o_many_sink(_shared_handle: String) -> syn::Expr {
        panic!("Maelstrom deployment does not support processes, only clusters")
    }

    fn e2o_source(
        _extra_stmts: &mut Vec<syn::Stmt>,
        _p1: &Self::External,
        _p1_port: &<Self::External as Node>::Port,
        _p2: &Self::Process,
        _p2_port: &<Self::Process as Node>::Port,
        _codec_type: &syn::Type,
        _shared_handle: String,
    ) -> syn::Expr {
        panic!("Maelstrom deployment does not support processes, only clusters")
    }

    fn e2o_connect(
        _p1: &Self::External,
        _p1_port: &<Self::External as Node>::Port,
        _p2: &Self::Process,
        _p2_port: &<Self::Process as Node>::Port,
        _many: bool,
        _server_hint: NetworkHint,
    ) -> Box<dyn FnOnce()> {
        panic!("Maelstrom deployment does not support processes, only clusters")
    }

    fn o2e_sink(
        _p1: &Self::Process,
        _p1_port: &<Self::Process as Node>::Port,
        _p2: &Self::External,
        _p2_port: &<Self::External as Node>::Port,
        _shared_handle: String,
    ) -> syn::Expr {
        panic!("Maelstrom deployment does not support processes, only clusters")
    }

    fn cluster_ids(
        _of_cluster: LocationKey,
    ) -> impl QuotedWithContext<'a, &'a [TaglessMemberId], ()> + Clone + 'a {
        cluster_members(RuntimeData::new("__hydro_lang_maelstrom_meta"), _of_cluster)
    }

    fn cluster_self_id() -> impl QuotedWithContext<'a, TaglessMemberId, ()> + Clone + 'a {
        cluster_self_id(RuntimeData::new("__hydro_lang_maelstrom_meta"))
    }

    fn cluster_membership_stream(
        location_id: &LocationId,
    ) -> impl QuotedWithContext<'a, Box<dyn Stream<Item = (TaglessMemberId, MembershipEvent)> + Unpin>, ()>
    {
        cluster_membership_stream(location_id)
    }
}

/// A dummy process type for Maelstrom (processes are not supported).
#[derive(Clone)]
pub struct MaelstromProcess {
    _private: (),
}

impl Node for MaelstromProcess {
    type Port = String;
    type Meta = ();
    type InstantiateEnv = MaelstromDeployment;

    fn next_port(&self) -> Self::Port {
        panic!("Maelstrom deployment does not support processes")
    }

    fn update_meta(&self, _meta: &Self::Meta) {}

    fn instantiate(
        &self,
        _env: &mut Self::InstantiateEnv,
        _meta: &mut Self::Meta,
        _graph: DfirGraph,
        _extra_stmts: &[syn::Stmt],
        _sidecars: &[syn::Expr],
    ) {
        panic!("Maelstrom deployment does not support processes")
    }
}

/// Represents a cluster in Maelstrom deployment.
#[derive(Clone)]
pub struct MaelstromCluster {
    next_port: Rc<RefCell<usize>>,
    name_hint: Option<String>,
}

impl Node for MaelstromCluster {
    type Port = String;
    type Meta = ();
    type InstantiateEnv = MaelstromDeployment;

    fn next_port(&self) -> Self::Port {
        let next_port = *self.next_port.borrow();
        *self.next_port.borrow_mut() += 1;
        format!("port_{}", next_port)
    }

    fn update_meta(&self, _meta: &Self::Meta) {}

    fn instantiate(
        &self,
        env: &mut Self::InstantiateEnv,
        _meta: &mut Self::Meta,
        graph: DfirGraph,
        extra_stmts: &[syn::Stmt],
        sidecars: &[syn::Expr],
    ) {
        let (bin_name, config) = create_graph_trybuild(
            graph,
            extra_stmts,
            sidecars,
            self.name_hint.as_deref(),
            crate::compile::trybuild::generate::DeployMode::Maelstrom,
            LinkingMode::Static,
        );

        env.bin_name = Some(bin_name);
        env.project_dir = Some(config.project_dir);
        env.target_dir = Some(config.target_dir);
        env.features = config.features;
    }
}

/// Represents an external client in Maelstrom deployment.
#[derive(Clone)]
pub enum MaelstromExternal {}

impl Node for MaelstromExternal {
    type Port = String;
    type Meta = ();
    type InstantiateEnv = MaelstromDeployment;

    fn next_port(&self) -> Self::Port {
        unreachable!()
    }

    fn update_meta(&self, _meta: &Self::Meta) {}

    fn instantiate(
        &self,
        _env: &mut Self::InstantiateEnv,
        _meta: &mut Self::Meta,
        _graph: DfirGraph,
        _extra_stmts: &[syn::Stmt],
        _sidecars: &[syn::Expr],
    ) {
        unreachable!()
    }
}

impl<'a> RegisterPort<'a, MaelstromDeploy> for MaelstromExternal {
    fn register(&self, _external_port_id: ExternalPortId, _port: Self::Port) {
        unreachable!()
    }

    #[expect(clippy::manual_async_fn, reason = "false positive, involves lifetimes")]
    fn as_bytes_bidi(
        &self,
        _external_port_id: ExternalPortId,
    ) -> impl Future<
        Output = (
            Pin<Box<dyn Stream<Item = Result<BytesMut, Error>>>>,
            Pin<Box<dyn Sink<Bytes, Error = Error>>>,
        ),
    > + 'a {
        async move { unreachable!() }
    }

    #[expect(clippy::manual_async_fn, reason = "false positive, involves lifetimes")]
    fn as_bincode_bidi<InT, OutT>(
        &self,
        _external_port_id: ExternalPortId,
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
        async move { unreachable!() }
    }

    #[expect(clippy::manual_async_fn, reason = "false positive, involves lifetimes")]
    fn as_bincode_sink<T: Serialize + 'static>(
        &self,
        _external_port_id: ExternalPortId,
    ) -> impl Future<Output = Pin<Box<dyn Sink<T, Error = Error>>>> + 'a {
        async move { unreachable!() }
    }

    #[expect(clippy::manual_async_fn, reason = "false positive, involves lifetimes")]
    fn as_bincode_source<T: DeserializeOwned + 'static>(
        &self,
        _external_port_id: ExternalPortId,
    ) -> impl Future<Output = Pin<Box<dyn Stream<Item = T>>>> + 'a {
        async move { unreachable!() }
    }
}

/// Specification for building a Maelstrom cluster.
#[derive(Clone)]
pub struct MaelstromClusterSpec;

impl<'a> ClusterSpec<'a, MaelstromDeploy> for MaelstromClusterSpec {
    fn build(self, key: LocationKey, name_hint: &str) -> MaelstromCluster {
        assert_eq!(
            key.to_string(),
            "loc1v1",
            "there should only be one location for a Maelstrom deployment"
        );
        MaelstromCluster {
            next_port: Rc::new(RefCell::new(0)),
            name_hint: Some(name_hint.to_string()),
        }
    }
}

/// The Maelstrom deployment environment.
///
/// This holds configuration for the Maelstrom run and accumulates
/// compilation artifacts during deployment.
pub struct MaelstromDeployment {
    /// Number of nodes in the cluster.
    pub node_count: usize,
    /// Path to the maelstrom binary.
    pub maelstrom_path: PathBuf,
    /// Workload to run (e.g., "echo", "broadcast", "g-counter").
    pub workload: String,
    /// Time limit in seconds.
    pub time_limit: Option<u64>,
    /// Rate of requests per second.
    pub rate: Option<u64>,
    /// Additional maelstrom arguments.
    pub extra_args: Vec<String>,

    // Populated during deployment
    pub(crate) bin_name: Option<String>,
    pub(crate) project_dir: Option<PathBuf>,
    pub(crate) target_dir: Option<PathBuf>,
    pub(crate) features: Option<Vec<String>>,
}

impl MaelstromDeployment {
    /// Create a new Maelstrom deployment with the given node count.
    pub fn new(node_count: usize) -> Self {
        Self {
            node_count,
            maelstrom_path: PathBuf::from("maelstrom"),
            workload: "echo".to_string(),
            time_limit: None,
            rate: None,
            extra_args: vec![],
            bin_name: None,
            project_dir: None,
            target_dir: None,
            features: None,
        }
    }

    /// Set the path to the maelstrom binary.
    pub fn maelstrom_path(mut self, path: impl Into<PathBuf>) -> Self {
        self.maelstrom_path = path.into();
        self
    }

    /// Set the workload to run.
    pub fn workload(mut self, workload: impl Into<String>) -> Self {
        self.workload = workload.into();
        self
    }

    /// Set the time limit in seconds.
    pub fn time_limit(mut self, seconds: u64) -> Self {
        self.time_limit = Some(seconds);
        self
    }

    /// Set the request rate per second.
    pub fn rate(mut self, rate: u64) -> Self {
        self.rate = Some(rate);
        self
    }

    /// Add extra arguments to pass to maelstrom.
    pub fn extra_args(mut self, args: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.extra_args.extend(args.into_iter().map(Into::into));
        self
    }

    /// Build the compiled binary in dev mode.
    /// Returns the path to the compiled binary.
    pub fn build(&self) -> Result<PathBuf, Error> {
        let bin_name = self
            .bin_name
            .as_ref()
            .expect("No binary name set - did you call deploy?");
        let project_dir = self.project_dir.as_ref().expect("No project dir set");
        let target_dir = self.target_dir.as_ref().expect("No target dir set");

        let mut cmd = std::process::Command::new("cargo");
        cmd.arg("build")
            .arg("--example")
            .arg(bin_name)
            .arg("--no-default-features")
            .current_dir(project_dir)
            .env("CARGO_TARGET_DIR", target_dir)
            .env("STAGELEFT_TRYBUILD_BUILD_STAGED", "1");

        // Always include maelstrom_runtime feature for runtime support
        let mut all_features = vec!["hydro___feature_maelstrom_runtime".to_string()];
        if let Some(features) = &self.features {
            all_features.extend(features.iter().cloned());
        }
        if !all_features.is_empty() {
            cmd.arg("--features").arg(all_features.join(","));
        }

        let status = cmd.status()?;
        if !status.success() {
            return Err(Error::other(format!(
                "cargo build failed with status: {}",
                status
            )));
        }

        Ok(target_dir.join("debug").join("examples").join(bin_name))
    }

    /// Run Maelstrom with the compiled binary, return Ok(()) if all checks pass.
    ///
    /// This will block until Maelstrom completes.
    pub fn run(&self) -> Result<(), Error> {
        let binary_path = self.build()?;

        let mut cmd = std::process::Command::new(&self.maelstrom_path);
        cmd.arg("test")
            .arg("-w")
            .arg(&self.workload)
            .arg("--bin")
            .arg(&binary_path)
            .arg("--node-count")
            .arg(self.node_count.to_string())
            .stdout(Stdio::piped());

        if let Some(time_limit) = self.time_limit {
            cmd.arg("--time-limit").arg(time_limit.to_string());
        }

        if let Some(rate) = self.rate {
            cmd.arg("--rate").arg(rate.to_string());
        }

        for arg in &self.extra_args {
            cmd.arg(arg);
        }

        let spawned = cmd.spawn()?;

        for line in BufReader::new(spawned.stdout.unwrap()).lines() {
            let line = line?;
            eprintln!("{}", &line);

            if line.starts_with("Analysis invalid!") {
                return Err(Error::other("Analysis was invalid"));
            } else if line.starts_with("Errors occurred during analysis, but no anomalies found.") {
                return Ok(());
            } else if line.starts_with("Everything looks good!") {
                return Ok(());
            }
        }

        Err(Error::other("Maelstrom produced an unexpected result"))
    }

    /// Get the path to the compiled binary (after building).
    pub fn binary_path(&self) -> Option<PathBuf> {
        let bin_name = self.bin_name.as_ref()?;
        let target_dir = self.target_dir.as_ref()?;
        Some(target_dir.join("debug").join("examples").join(bin_name))
    }
}
