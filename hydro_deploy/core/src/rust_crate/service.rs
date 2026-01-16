use std::collections::HashMap;
use std::sync::{Arc, OnceLock};
use std::time::Duration;

use anyhow::{Context, Result, bail};
use async_trait::async_trait;
use futures::Future;
use hydro_deploy_integration::{InitConfig, ServerPort};
use memo_map::MemoMap;
use serde::Serialize;
use tokio::sync::{OnceCell, RwLock, mpsc};

use super::build::{BuildError, BuildOutput, BuildParams, build_crate_memoized};
use super::ports::{self, RustCratePortConfig};
use super::tracing_options::TracingOptions;
#[cfg(feature = "tracing")]
use crate::TracingResults;
use crate::progress::ProgressTracker;
use crate::{
    BaseServerStrategy, Host, LaunchedBinary, LaunchedHost, PortNetworkHint, ResourceBatch,
    ResourceResult, ServerStrategy, Service,
};

pub struct RustCrateService {
    id: usize,
    pub(super) on: Arc<dyn Host>,
    build_params: BuildParams,
    tracing: Option<TracingOptions>,
    args: Option<Vec<String>>,
    display_id: Option<String>,
    external_ports: Vec<u16>,

    meta: OnceLock<String>,

    /// Configuration for the ports this service will connect to as a client.
    pub(super) port_to_server: MemoMap<String, ports::ServerConfig>,
    /// Configuration for the ports that this service will listen on a port for.
    pub(super) port_to_bind: MemoMap<String, ServerStrategy>,

    launched_host: OnceCell<Arc<dyn LaunchedHost>>,

    /// A map of port names to config for how other services can connect to this one.
    /// Only valid after `ready` has been called, only contains ports that are configured
    /// in `server_ports`.
    pub(super) server_defns: Arc<RwLock<HashMap<String, ServerPort>>>,

    launched_binary: OnceCell<Box<dyn LaunchedBinary>>,
    started: OnceCell<()>,
}

impl RustCrateService {
    pub fn new(
        id: usize,
        on: Arc<dyn Host>,
        build_params: BuildParams,
        tracing: Option<TracingOptions>,
        args: Option<Vec<String>>,
        display_id: Option<String>,
        external_ports: Vec<u16>,
    ) -> Self {
        Self {
            id,
            on,
            build_params,
            tracing,
            args,
            display_id,
            external_ports,
            meta: OnceLock::new(),
            port_to_server: MemoMap::new(),
            port_to_bind: MemoMap::new(),
            launched_host: OnceCell::new(),
            server_defns: Arc::new(RwLock::new(HashMap::new())),
            launched_binary: OnceCell::new(),
            started: OnceCell::new(),
        }
    }

    pub fn update_meta<T: Serialize>(&self, meta: T) {
        if self.launched_binary.get().is_some() {
            panic!("Cannot update meta after binary has been launched")
        }
        self.meta
            .set(serde_json::to_string(&meta).unwrap())
            .expect("Cannot set meta twice.");
    }

    pub fn get_port(self: &Arc<Self>, name: String) -> RustCratePortConfig {
        RustCratePortConfig {
            service: Arc::downgrade(self),
            service_host: self.on.clone(),
            service_server_defns: self.server_defns.clone(),
            network_hint: PortNetworkHint::Auto,
            port: name,
            merge: false,
        }
    }

    pub fn get_port_with_hint(
        self: &Arc<Self>,
        name: String,
        network_hint: PortNetworkHint,
    ) -> RustCratePortConfig {
        RustCratePortConfig {
            service: Arc::downgrade(self),
            service_host: self.on.clone(),
            service_server_defns: self.server_defns.clone(),
            network_hint,
            port: name,
            merge: false,
        }
    }

    pub fn stdout(&self) -> mpsc::UnboundedReceiver<String> {
        self.launched_binary.get().unwrap().stdout()
    }

    pub fn stderr(&self) -> mpsc::UnboundedReceiver<String> {
        self.launched_binary.get().unwrap().stderr()
    }

    pub fn stdout_filter(&self, prefix: String) -> mpsc::UnboundedReceiver<String> {
        self.launched_binary.get().unwrap().stdout_filter(prefix)
    }

    pub fn stderr_filter(&self, prefix: String) -> mpsc::UnboundedReceiver<String> {
        self.launched_binary.get().unwrap().stderr_filter(prefix)
    }

    #[cfg(feature = "tracing")]
    pub fn tracing_results(&self) -> Option<&TracingResults> {
        self.launched_binary.get().unwrap().tracing_results()
    }

    pub fn exit_code(&self) -> Option<i32> {
        self.launched_binary.get().unwrap().exit_code()
    }

    fn build(
        &self,
    ) -> impl use<> + 'static + Future<Output = Result<&'static BuildOutput, BuildError>> {
        // Memoized, so no caching in `self` is needed.
        build_crate_memoized(self.build_params.clone())
    }
}

#[async_trait]
impl Service for RustCrateService {
    fn collect_resources(&self, _resource_batch: &mut ResourceBatch) {
        if self.launched_host.get().is_some() {
            return;
        }

        tokio::task::spawn(self.build());

        let host = &self.on;

        host.request_custom_binary();
        for (_, bind_type) in self.port_to_bind.iter() {
            host.request_port(bind_type);
        }

        for port in self.external_ports.iter() {
            host.request_port_base(&BaseServerStrategy::ExternalTcpPort(*port));
        }
    }

    async fn deploy(&self, resource_result: &Arc<ResourceResult>) -> Result<()> {
        self.launched_host
            .get_or_try_init::<anyhow::Error, _, _>(|| {
                ProgressTracker::with_group(
                    self.display_id
                        .clone()
                        .unwrap_or_else(|| format!("service/{}", self.id)),
                    None,
                    || async {
                        let built = self.build().await?;

                        let host = &self.on;
                        let launched = host.provision(resource_result);

                        launched.copy_binary(built).await?;
                        Ok(launched)
                    },
                )
            })
            .await?;
        Ok(())
    }

    async fn ready(&self) -> Result<()> {
        self.launched_binary
            .get_or_try_init(|| {
                ProgressTracker::with_group(
                    self.display_id
                        .clone()
                        .unwrap_or_else(|| format!("service/{}", self.id)),
                    None,
                    || async {
                        let launched_host = self.launched_host.get().unwrap();

                        let built = self.build().await?;
                        let args = self.args.as_ref().cloned().unwrap_or_default();

                        let binary = launched_host
                            .launch_binary(
                                self.display_id
                                    .clone()
                                    .unwrap_or_else(|| format!("service/{}", self.id)),
                                built,
                                &args,
                                self.tracing.clone(),
                            )
                            .await?;

                        let bind_config = self
                            .port_to_bind
                            .iter()
                            .map(|(port_name, bind_type)| {
                                (port_name.clone(), launched_host.server_config(bind_type))
                            })
                            .collect::<HashMap<_, _>>();

                        let formatted_bind_config = serde_json::to_string::<InitConfig>(&(
                            bind_config,
                            self.meta.get().map(|s| s.as_str().into()),
                        ))
                        .unwrap();

                        // request stdout before sending config so we don't miss the "ready" response
                        let stdout_receiver = binary.deploy_stdout();

                        binary.stdin().send(format!("{formatted_bind_config}\n"))?;

                        let ready_line = ProgressTracker::leaf(
                            "waiting for ready",
                            tokio::time::timeout(Duration::from_secs(60), stdout_receiver),
                        )
                        .await
                        .context("Timed out waiting for ready")?
                        .context("Program unexpectedly quit")?;
                        if let Some(line_rest) = ready_line.strip_prefix("ready: ") {
                            *self.server_defns.try_write().unwrap() =
                                serde_json::from_str(line_rest).unwrap();
                        } else {
                            bail!("expected ready");
                        }
                        Ok(binary)
                    },
                )
            })
            .await?;
        Ok(())
    }

    async fn start(&self) -> Result<()> {
        self.started
            .get_or_try_init(|| async {
                let sink_ports_futures =
                    self.port_to_server
                        .iter()
                        .map(|(port_name, outgoing)| async {
                            (&**port_name, outgoing.load_instantiated(&|p| p).await)
                        });
                let sink_ports = futures::future::join_all(sink_ports_futures)
                    .await
                    .into_iter()
                    .collect::<HashMap<_, _>>();

                let formatted_defns = serde_json::to_string(&sink_ports).unwrap();

                let stdout_receiver = self.launched_binary.get().unwrap().deploy_stdout();

                self.launched_binary
                    .get()
                    .unwrap()
                    .stdin()
                    .send(format!("start: {formatted_defns}\n"))
                    .unwrap();

                let start_ack_line = ProgressTracker::leaf(
                    self.display_id
                        .clone()
                        .unwrap_or_else(|| format!("service/{}", self.id))
                        + " / waiting for ack start",
                    tokio::time::timeout(Duration::from_secs(60), stdout_receiver),
                )
                .await??;
                if !start_ack_line.starts_with("ack start") {
                    bail!("expected ack start");
                }

                Ok(())
            })
            .await?;

        Ok(())
    }

    async fn stop(&self) -> Result<()> {
        ProgressTracker::with_group(
            self.display_id
                .clone()
                .unwrap_or_else(|| format!("service/{}", self.id)),
            None,
            || async {
                let launched_binary = self.launched_binary.get().unwrap();
                launched_binary.stdin().send("stop\n".to_string())?;

                let timeout_result = ProgressTracker::leaf(
                    "waiting for exit",
                    tokio::time::timeout(Duration::from_secs(60), launched_binary.wait()),
                )
                .await;
                match timeout_result {
                    Err(_timeout) => {} // `wait()` timed out, but stop will force quit.
                    Ok(Err(unexpected_error)) => return Err(unexpected_error), // `wait()` errored.
                    Ok(Ok(_exit_status)) => {}
                }
                launched_binary.stop().await?;

                Ok(())
            },
        )
        .await
    }
}
