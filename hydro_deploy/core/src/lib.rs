use std::any::Any;
use std::collections::HashMap;
use std::fmt::Debug;
use std::net::SocketAddr;
use std::sync::Arc;

use anyhow::Result;
use async_trait::async_trait;
use hydro_deploy_integration::ServerBindConfig;
use rust_crate::build::BuildOutput;
use rust_crate::tracing_options::TracingOptions;
use tokio::sync::{mpsc, oneshot};

pub mod deployment;
pub use deployment::Deployment;

pub mod progress;

pub mod localhost;
pub use localhost::LocalhostHost;

pub mod ssh;

pub mod gcp;
pub use gcp::GcpComputeEngineHost;

pub mod azure;
pub use azure::AzureHost;

pub mod aws;
pub use aws::{AwsEc2Host, AwsNetwork};

pub mod rust_crate;
pub use rust_crate::RustCrate;

pub mod custom_service;
pub use custom_service::CustomService;

pub mod terraform;

pub mod util;

#[derive(Default)]
pub struct ResourcePool {
    pub terraform: terraform::TerraformPool,
}

pub struct ResourceBatch {
    pub terraform: terraform::TerraformBatch,
}

impl ResourceBatch {
    fn new() -> ResourceBatch {
        ResourceBatch {
            terraform: terraform::TerraformBatch::default(),
        }
    }

    async fn provision(
        self,
        pool: &mut ResourcePool,
        last_result: Option<Arc<ResourceResult>>,
    ) -> Result<ResourceResult> {
        Ok(ResourceResult {
            terraform: self.terraform.provision(&mut pool.terraform).await?,
            _last_result: last_result,
        })
    }
}

#[derive(Debug)]
pub struct ResourceResult {
    pub terraform: terraform::TerraformResult,
    _last_result: Option<Arc<ResourceResult>>,
}

#[derive(Clone)]
pub struct TracingResults {
    pub folded_data: Vec<u8>,
}

#[async_trait]
pub trait LaunchedBinary: Send + Sync {
    fn stdin(&self) -> mpsc::UnboundedSender<String>;

    /// Provides a oneshot channel to handshake with the binary,
    /// with the guarantee that as long as deploy is holding on
    /// to a handle, none of the messages will also be broadcast
    /// to the user-facing [`LaunchedBinary::stdout`] channel.
    fn deploy_stdout(&self) -> oneshot::Receiver<String>;

    fn stdout(&self) -> mpsc::UnboundedReceiver<String>;
    fn stderr(&self) -> mpsc::UnboundedReceiver<String>;
    fn stdout_filter(&self, prefix: String) -> mpsc::UnboundedReceiver<String>;
    fn stderr_filter(&self, prefix: String) -> mpsc::UnboundedReceiver<String>;

    fn tracing_results(&self) -> Option<&TracingResults>;

    fn exit_code(&self) -> Option<i32>;

    /// Wait for the process to stop on its own. Returns the exit code.
    async fn wait(&mut self) -> Result<i32>;
    /// If the process is still running, force stop it. Then run post-run tasks.
    async fn stop(&mut self) -> Result<()>;
}

#[async_trait]
pub trait LaunchedHost: Send + Sync {
    /// Given a pre-selected network type, computes concrete information needed for a service
    /// to listen to network connections (such as the IP address to bind to).
    fn base_server_config(&self, strategy: &BaseServerStrategy) -> ServerBindConfig;

    fn server_config(&self, strategy: &ServerStrategy) -> ServerBindConfig {
        match strategy {
            ServerStrategy::Direct(b) => self.base_server_config(b),
            ServerStrategy::Many(b) => {
                ServerBindConfig::MultiConnection(Box::new(self.base_server_config(b)))
            }
            ServerStrategy::Demux(demux) => {
                let mut config_map = HashMap::new();
                for (key, underlying) in demux {
                    config_map.insert(*key, self.server_config(underlying));
                }

                ServerBindConfig::Demux(config_map)
            }
            ServerStrategy::Merge(merge) => {
                let mut configs = vec![];
                for underlying in merge {
                    configs.push(self.server_config(underlying));
                }

                ServerBindConfig::Merge(configs)
            }
            ServerStrategy::Tagged(underlying, id) => {
                ServerBindConfig::Tagged(Box::new(self.server_config(underlying)), *id)
            }
            ServerStrategy::Null => ServerBindConfig::Null,
        }
    }

    async fn copy_binary(&self, binary: &BuildOutput) -> Result<()>;

    async fn launch_binary(
        &self,
        id: String,
        binary: &BuildOutput,
        args: &[String],
        perf: Option<TracingOptions>,
    ) -> Result<Box<dyn LaunchedBinary>>;

    async fn forward_port(&self, addr: &SocketAddr) -> Result<SocketAddr>;
}

pub enum BaseServerStrategy {
    UnixSocket,
    InternalTcpPort(Option<u16>),
    ExternalTcpPort(
        /// The port number to bind to, which must be explicit to open the firewall.
        u16,
    ),
}

/// Types of connection that a service can receive when configured as the server.
pub enum ServerStrategy {
    Direct(BaseServerStrategy),
    Many(BaseServerStrategy),
    Demux(HashMap<u32, ServerStrategy>),
    Merge(Vec<ServerStrategy>),
    Tagged(Box<ServerStrategy>, u32),
    Null,
}

/// Like BindType, but includes metadata for determining whether a connection is possible.
pub enum ClientStrategy<'a> {
    UnixSocket(
        /// Unique identifier for the host this socket will be on.
        usize,
    ),
    InternalTcpPort(
        /// The host that this port is available on.
        &'a dyn Host,
    ),
    ForwardedTcpPort(
        /// The host that this port is available on.
        &'a dyn Host,
    ),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum HostTargetType {
    Local,
    Linux,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PortNetworkHint {
    Auto,
    TcpPort(Option<u16>),
}

pub type HostStrategyGetter = Box<dyn FnOnce(&dyn Any) -> BaseServerStrategy>;

pub trait Host: Any + Send + Sync + Debug {
    fn target_type(&self) -> HostTargetType;

    fn request_port_base(&self, bind_type: &BaseServerStrategy);

    fn request_port(&self, bind_type: &ServerStrategy) {
        match bind_type {
            ServerStrategy::Direct(base) => self.request_port_base(base),
            ServerStrategy::Many(base) => self.request_port_base(base),
            ServerStrategy::Demux(demux) => {
                for bind_type in demux.values() {
                    self.request_port(bind_type);
                }
            }
            ServerStrategy::Merge(merge) => {
                for bind_type in merge {
                    self.request_port(bind_type);
                }
            }
            ServerStrategy::Tagged(underlying, _) => {
                self.request_port(underlying);
            }
            ServerStrategy::Null => {}
        }
    }

    /// An identifier for this host, which is unique within a deployment.
    fn id(&self) -> usize;

    /// Configures the host to support copying and running a custom binary.
    fn request_custom_binary(&self);

    /// Makes requests for physical resources (servers) that this host needs to run.
    ///
    /// This should be called before `provision` is called.
    fn collect_resources(&self, resource_batch: &mut ResourceBatch);

    /// Connects to the acquired resources and prepares the host to run services.
    ///
    /// This should be called after `collect_resources` is called.
    fn provision(&self, resource_result: &Arc<ResourceResult>) -> Arc<dyn LaunchedHost>;

    fn launched(&self) -> Option<Arc<dyn LaunchedHost>>;

    /// Identifies a network type that this host can use for connections if it is the server.
    /// The host will be `None` if the connection is from the same host as the target.
    fn strategy_as_server<'a>(
        &'a self,
        connection_from: &dyn Host,
        server_tcp_port_hint: PortNetworkHint,
    ) -> Result<(ClientStrategy<'a>, HostStrategyGetter)>;

    /// Determines whether this host can connect to another host using the given strategy.
    fn can_connect_to(&self, typ: ClientStrategy) -> bool;
}

#[async_trait]
pub trait Service: Send + Sync {
    /// Makes requests for physical resources server ports that this service needs to run.
    /// This should **not** recursively call `collect_resources` on the host, since
    /// we guarantee that `collect_resources` is only called once per host.
    ///
    /// This should also perform any "free", non-blocking computations (compilations),
    /// because the `deploy` method will be called after these resources are allocated.
    fn collect_resources(&self, resource_batch: &mut ResourceBatch);

    /// Connects to the acquired resources and prepares the service to be launched.
    async fn deploy(&mut self, resource_result: &Arc<ResourceResult>) -> Result<()>;

    /// Launches the service, which should start listening for incoming network
    /// connections. The service should not start computing at this point.
    async fn ready(&mut self) -> Result<()>;

    /// Starts the service by having it connect to other services and start computations.
    async fn start(&mut self) -> Result<()>;

    /// Stops the service by having it disconnect from other services and stop computations.
    async fn stop(&mut self) -> Result<()>;
}

pub trait ServiceBuilder {
    type Service: Service + 'static;
    fn build(self, id: usize) -> Self::Service;
}

impl<S: Service + 'static, T: FnOnce(usize) -> S> ServiceBuilder for T {
    type Service = S;
    fn build(self, id: usize) -> Self::Service {
        self(id)
    }
}
