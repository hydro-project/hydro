//! Deployment backend for Hydro that targets Maelstrom for distributed systems testing.
//!
//! Maelstrom is a workbench for learning distributed systems by writing your own.
//! This backend compiles Hydro programs to binaries that communicate via Maelstrom's
//! stdin/stdout JSON protocol.

use std::cell::RefCell;
use std::future::Future;
use std::io::{BufRead, BufReader, Error};
use std::path::{Path, PathBuf};
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
use crate::compile::trybuild::generate::{
    ExampleBuildConfig, LinkingMode, TrybuildConfig, compile_trybuild_example,
    create_graph_trybuild,
};
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
        _env: &mut Self::InstantiateEnv,
        _p1: &Self::Process,
        _p1_port: &<Self::Process as Node>::Port,
        _p2: &Self::Process,
        _p2_port: &<Self::Process as Node>::Port,
        _name: Option<&str>,
        _networking_info: &crate::networking::NetworkingInfo,
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
        _env: &mut Self::InstantiateEnv,
        _p1: &Self::Process,
        _p1_port: &<Self::Process as Node>::Port,
        _c2: &Self::Cluster,
        _c2_port: &<Self::Cluster as Node>::Port,
        _name: Option<&str>,
        _networking_info: &crate::networking::NetworkingInfo,
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
        _env: &mut Self::InstantiateEnv,
        _c1: &Self::Cluster,
        _c1_port: &<Self::Cluster as Node>::Port,
        _p2: &Self::Process,
        _p2_port: &<Self::Process as Node>::Port,
        _name: Option<&str>,
        _networking_info: &crate::networking::NetworkingInfo,
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
        env: &mut Self::InstantiateEnv,
        _c1: &Self::Cluster,
        _c1_port: &<Self::Cluster as Node>::Port,
        _c2: &Self::Cluster,
        _c2_port: &<Self::Cluster as Node>::Port,
        _name: Option<&str>,
        networking_info: &crate::networking::NetworkingInfo,
    ) -> (syn::Expr, syn::Expr) {
        use crate::networking::{NetworkingInfo, TcpFault};
        match networking_info {
            NetworkingInfo::Tcp { fault } => match (fault, env.nemesis.as_deref()) {
                (TcpFault::Lossy | TcpFault::LossyDelayedForever, _) => {} /* lossy/delayed are always allowed */
                (_, None) => {} // no nemesis means any fault model is fine
                (TcpFault::FailStop, Some("partition")) => {
                    panic!(
                        "Maelstrom partition nemesis requires lossy networking, but fail_stop was used. \
                         Use `TCP.lossy().bincode()` or `TCP.lossy_delayed_forever().bincode()` instead of `TCP.fail_stop().bincode()`."
                    );
                }
                (TcpFault::FailStop, Some(_)) => {} // other nemeses are fine with fail_stop
            },
        }
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
        _env: &mut Self::InstantiateEnv,
        _at_location: &LocationId,
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
            LinkingMode::Dynamic,
        );

        env.bin_name = Some(bin_name);
        env.trybuild = Some(config);
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
            key,
            LocationKey::FIRST,
            "there should only be one location for a Maelstrom deployment"
        );
        MaelstromCluster {
            next_port: Rc::new(RefCell::new(0)),
            name_hint: Some(name_hint.to_owned()),
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
    /// The availability of nodes.
    pub availability: Option<String>,
    /// Nemesis to run during tests.
    pub nemesis: Option<String>,
    /// Additional maelstrom arguments.
    pub extra_args: Vec<String>,

    // Populated during deployment
    pub(crate) bin_name: Option<String>,
    pub(crate) trybuild: Option<TrybuildConfig>,
}

impl MaelstromDeployment {
    /// Create a new Maelstrom deployment with the given node count.
    pub fn new(workload: impl Into<String>) -> Self {
        Self {
            node_count: 1,
            maelstrom_path: PathBuf::from("maelstrom"),
            workload: workload.into(),
            time_limit: None,
            rate: None,
            availability: None,
            nemesis: None,
            extra_args: vec![],
            bin_name: None,
            trybuild: None,
        }
    }

    /// Set the node count.
    pub fn node_count(mut self, count: usize) -> Self {
        self.node_count = count;
        self
    }

    /// Set the path to the maelstrom binary.
    pub fn maelstrom_path(mut self, path: impl Into<PathBuf>) -> Self {
        self.maelstrom_path = path.into();
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

    /// Set the availability for the test.
    pub fn availability(mut self, availability: impl Into<String>) -> Self {
        self.availability = Some(availability.into());
        self
    }

    /// Set the nemesis for the test.
    pub fn nemesis(mut self, nemesis: impl Into<String>) -> Self {
        self.nemesis = Some(nemesis.into());
        self
    }

    /// Add extra arguments to pass to maelstrom.
    pub fn extra_args(mut self, args: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.extra_args.extend(args.into_iter().map(Into::into));
        self
    }

    /// Build the compiled binary in dev mode.
    /// Returns the path to the compiled binary.
    ///
    /// This shares the same parallel-compilation machinery as the simulator: the
    /// program is linked dynamically against a prebuilt dylib of its dependencies,
    /// so repeated and concurrent builds only need to recompile the generated
    /// example itself.
    pub fn build(&self) -> Result<PathBuf, Error> {
        let bin_name = self
            .bin_name
            .as_ref()
            .expect("No binary name set - did you call deploy?");
        let trybuild = self
            .trybuild
            .as_ref()
            .expect("No trybuild config set - did you call deploy?");

        let out = compile_trybuild_example(ExampleBuildConfig {
            trybuild: trybuild.clone(),
            bin_name: bin_name.clone(),
            runtime_feature: "hydro___feature_maelstrom_runtime",
            // Maelstrom builds the generated example directly as an executable.
            example_name: bin_name.clone(),
            crate_type: None,
            set_trybuild_lib_name: false,
            allow_fuzz: false,
        })
        .map_err(|()| Error::other("Maelstrom binary compilation failed"))?;

        // Persist the built executable so it survives past the temporary build guards.
        out.keep().map_err(|e| Error::other(e.to_string()))
    }

    /// Run Maelstrom with the compiled binary, return Ok(()) if all checks pass.
    ///
    /// This will block until Maelstrom completes.
    pub fn run(self) -> Result<(), Error> {
        let binary_path = self.build()?;

        // Warm up the binary before handing it to Maelstrom. On macOS, the
        // first execution of a freshly written binary triggers a Gatekeeper /
        // XProtect (`syspolicyd`) scan that can take several seconds on loaded
        // CI machines. Maelstrom only waits 10 seconds for each node to answer
        // the `init` RPC, so a cold first exec (multiplied across concurrently
        // launched nodes) can cause spurious node-startup timeouts. The warmup
        // invocation sees EOF on stdin and exits immediately, priming the
        // system's first-exec caches for the real run.
        std::process::Command::new(&binary_path)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()?
            .wait()?;

        // Use a unique working directory per run to avoid conflicts with concurrent tests.
        let run_dir = tempfile::tempdir().map_err(Error::other)?;

        let mut cmd = std::process::Command::new(&self.maelstrom_path);
        cmd.arg("test")
            .arg("-w")
            .arg(&self.workload)
            .arg("--bin")
            .arg(&binary_path)
            .arg("--node-count")
            .arg(self.node_count.to_string())
            .current_dir(run_dir.path())
            .stdout(Stdio::piped());

        if let Some(time_limit) = self.time_limit {
            cmd.arg("--time-limit").arg(time_limit.to_string());
        }

        if let Some(rate) = self.rate {
            cmd.arg("--rate").arg(rate.to_string());
        }

        if let Some(availability) = self.availability {
            cmd.arg("--availability").arg(availability);
        }

        if let Some(nemesis) = self.nemesis {
            cmd.arg("--nemesis").arg(nemesis);
        }

        for arg in &self.extra_args {
            cmd.arg(arg);
        }

        let spawned = cmd.spawn()?;

        for line in BufReader::new(spawned.stdout.unwrap()).lines() {
            let line = line?;
            eprintln!("{}", &line);

            if line.starts_with("Analysis invalid!") {
                let path = run_dir.keep();
                dump_node_logs(&path);
                return Err(Error::other(format!(
                    "Analysis was invalid. Maelstrom store at: {}",
                    path.display()
                )));
            } else if line.starts_with("Errors occurred during analysis, but no anomalies found.")
                || line.starts_with("Everything looks good!")
            {
                return Ok(());
            }
        }

        let path = run_dir.keep();
        dump_node_logs(&path);
        Err(Error::other(format!(
            "Maelstrom produced an unexpected result. Store at: {}",
            path.display()
        )))
    }

    /// Get the path to the compiled binary, building it if necessary.
    pub fn binary_path(&self) -> Option<PathBuf> {
        self.build().ok()
    }
}

/// Print the per-node logs from a Maelstrom run directory to stderr.
///
/// Maelstrom truncates node stderr in its own error messages (keeping only the
/// tail), so when a node crashes the actual panic message is often cut off.
/// The full logs live in `store/<workload>/<timestamp>/node-logs/*.log` under
/// the run directory; dump them so failures are debuggable in CI, where the
/// preserved store directory is not otherwise accessible.
fn dump_node_logs(run_dir: &Path) {
    fn collect(dir: &Path, out: &mut Vec<PathBuf>) {
        let Ok(entries) = std::fs::read_dir(dir) else {
            return;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            // Skip symlinks (e.g. `store/latest`) to avoid duplicates.
            if path.is_symlink() || !path.is_dir() {
                continue;
            }
            if path.file_name().is_some_and(|name| name == "node-logs") {
                if let Ok(logs) = std::fs::read_dir(&path) {
                    out.extend(logs.flatten().map(|e| e.path()));
                }
            } else {
                collect(&path, out);
            }
        }
    }

    let mut log_files = Vec::new();
    collect(&run_dir.join("store"), &mut log_files);
    log_files.sort();

    for log in log_files {
        eprintln!("==== Maelstrom node log: {} ====", log.display());
        match std::fs::read_to_string(&log) {
            Ok(contents) => eprint!("{}", contents),
            Err(e) => eprintln!("(failed to read log: {})", e),
        }
        eprintln!("==== end of node log: {} ====", log.display());
    }
}
