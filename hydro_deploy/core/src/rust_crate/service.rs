use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use anyhow::{Context, Result, bail};
use async_trait::async_trait;
use futures::Future;
use hydro_deploy_integration::{InitConfig, ServerPort};
use serde::Serialize;
use tokio::sync::{RwLock, mpsc};

use super::build::{BuildError, BuildOutput, BuildParams, build_crate_memoized};
use super::ports::{self, RustCratePortConfig, RustCrateSink, SourcePath};
use super::tracing_options::TracingOptions;
use crate::progress::ProgressTracker;
use crate::{
    Host, LaunchedBinary, LaunchedHost, ResourceBatch, ResourceResult, ServerStrategy, Service,
    TracingResults,
};

pub struct RustCrateService {
    id: usize,
    pub(super) on: Arc<dyn Host>,
    build_params: BuildParams,
    tracing: Option<TracingOptions>,
    args: Option<Vec<String>>,
    display_id: Option<String>,
    external_ports: Vec<u16>,

    meta: Option<String>,

    /// Configuration for the ports this service will connect to as a client.
    pub(super) port_to_server: HashMap<String, ports::ServerConfig>,

    /// Configuration for the ports that this service will listen on a port for.
    pub(super) port_to_bind: HashMap<String, ServerStrategy>,

    launched_host: Option<Arc<dyn LaunchedHost>>,

    /// A map of port names to config for how other services can connect to this one.
    /// Only valid after `ready` has been called, only contains ports that are configured
    /// in `server_ports`.
    pub(super) server_defns: Arc<RwLock<HashMap<String, ServerPort>>>,

    launched_binary: Option<Box<dyn LaunchedBinary>>,
    started: bool,
}

impl RustCrateService {
    #[expect(clippy::too_many_arguments, reason = "internal code")]
    pub fn new(
        id: usize,
        src: PathBuf,
        on: Arc<dyn Host>,
        bin: Option<String>,
        example: Option<String>,
        profile: Option<String>,
        rustflags: Option<String>,
        target_dir: Option<PathBuf>,
        build_env: Vec<(String, String)>,
        no_default_features: bool,
        tracing: Option<TracingOptions>,
        features: Option<Vec<String>>,
        config: Option<String>,
        args: Option<Vec<String>>,
        display_id: Option<String>,
        external_ports: Vec<u16>,
    ) -> Self {
        let target_type = on.target_type();

        let build_params = BuildParams::new(
            src,
            bin,
            example,
            profile,
            rustflags,
            target_dir,
            build_env,
            no_default_features,
            target_type,
            features,
            config,
        );

        Self {
            id,
            on,
            build_params,
            tracing,
            args,
            display_id,
            external_ports,
            meta: None,
            port_to_server: HashMap::new(),
            port_to_bind: HashMap::new(),
            launched_host: None,
            server_defns: Arc::new(RwLock::new(HashMap::new())),
            launched_binary: None,
            started: false,
        }
    }

    pub fn update_meta<T: Serialize>(&mut self, meta: T) {
        if self.launched_binary.is_some() {
            panic!("Cannot update meta after binary has been launched")
        }

        self.meta = Some(serde_json::to_string(&meta).unwrap());
    }

    pub fn get_port(
        &self,
        name: String,
        self_arc: &Arc<RwLock<RustCrateService>>,
    ) -> RustCratePortConfig {
        RustCratePortConfig {
            service: Arc::downgrade(self_arc),
            service_host: self.on.clone(),
            service_server_defns: self.server_defns.clone(),
            port: name,
            merge: false,
        }
    }

    pub fn add_connection(
        &mut self,
        self_arc: &Arc<RwLock<RustCrateService>>,
        my_port: String,
        sink: &dyn RustCrateSink,
    ) -> Result<()> {
        let forward_res = sink.instantiate(&SourcePath::Direct(self.on.clone()));
        if let Ok(instantiated) = forward_res {
            // TODO(shadaj): if already in this map, we want to broadcast
            assert!(!self.port_to_server.contains_key(&my_port));
            self.port_to_server.insert(my_port, instantiated());
            Ok(())
        } else {
            drop(forward_res);
            let instantiated = sink.instantiate_reverse(
                &self.on,
                Arc::new(RustCratePortConfig {
                    service: Arc::downgrade(self_arc),
                    service_host: self.on.clone(),
                    service_server_defns: self.server_defns.clone(),
                    port: my_port.clone(),
                    merge: false,
                }),
                &|p| p,
            )?;

            assert!(!self.port_to_bind.contains_key(&my_port));
            self.port_to_bind.insert(my_port, (instantiated)(sink));

            Ok(())
        }
    }

    pub fn stdout(&self) -> mpsc::UnboundedReceiver<String> {
        self.launched_binary.as_ref().unwrap().stdout()
    }

    pub fn stderr(&self) -> mpsc::UnboundedReceiver<String> {
        self.launched_binary.as_ref().unwrap().stderr()
    }

    pub fn stdout_filter(&self, prefix: String) -> mpsc::UnboundedReceiver<String> {
        self.launched_binary.as_ref().unwrap().stdout_filter(prefix)
    }

    pub fn stderr_filter(&self, prefix: String) -> mpsc::UnboundedReceiver<String> {
        self.launched_binary.as_ref().unwrap().stderr_filter(prefix)
    }

    pub fn tracing_results(&self) -> Option<&TracingResults> {
        self.launched_binary.as_ref().unwrap().tracing_results()
    }

    pub fn exit_code(&self) -> Option<i32> {
        self.launched_binary.as_ref().unwrap().exit_code()
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
        if self.launched_host.is_some() {
            return;
        }

        tokio::task::spawn(self.build());

        let host = &self.on;

        host.request_custom_binary();
        for (_, bind_type) in self.port_to_bind.iter() {
            host.request_port(bind_type);
        }

        for port in self.external_ports.iter() {
            host.request_port(&ServerStrategy::ExternalTcpPort(*port));
        }
    }

    async fn deploy(&mut self, resource_result: &Arc<ResourceResult>) -> Result<()> {
        if self.launched_host.is_some() {
            return Ok(());
        }

        ProgressTracker::with_group(
            self.display_id
                .clone()
                .unwrap_or_else(|| format!("service/{}", self.id)),
            None,
            || async {
                let built = ProgressTracker::leaf("build", self.build()).await?;

                let host = &self.on;
                let launched = host.provision(resource_result);

                launched.copy_binary(built).await?;

                self.launched_host = Some(launched);
                Ok(())
            },
        )
        .await
    }

    async fn ready(&mut self) -> Result<()> {
        if self.launched_binary.is_some() {
            return Ok(());
        }

        ProgressTracker::with_group(
            self.display_id
                .clone()
                .unwrap_or_else(|| format!("service/{}", self.id)),
            None,
            || async {
                let launched_host = self.launched_host.as_ref().unwrap();

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

                let mut bind_config = HashMap::new();
                for (port_name, bind_type) in self.port_to_bind.iter() {
                    bind_config.insert(port_name.clone(), launched_host.server_config(bind_type));
                }

                let formatted_bind_config =
                    serde_json::to_string::<InitConfig>(&(bind_config, self.meta.clone())).unwrap();

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

                self.launched_binary = Some(binary);

                Ok(())
            },
        )
        .await
    }

    async fn start(&mut self) -> Result<()> {
        if self.started {
            return Ok(());
        }

        let mut sink_ports = HashMap::new();
        for (port_name, outgoing) in self.port_to_server.drain() {
            sink_ports.insert(port_name.clone(), outgoing.load_instantiated(&|p| p).await);
        }

        let formatted_defns = serde_json::to_string(&sink_ports).unwrap();

        let stdout_receiver = self.launched_binary.as_ref().unwrap().deploy_stdout();

        self.launched_binary
            .as_ref()
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

        self.started = true;
        Ok(())
    }

    async fn stop(&mut self) -> Result<()> {
        ProgressTracker::with_group(
            self.display_id
                .clone()
                .unwrap_or_else(|| format!("service/{}", self.id)),
            None,
            || async {
                let launched_binary = self.launched_binary.as_mut().unwrap();
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
