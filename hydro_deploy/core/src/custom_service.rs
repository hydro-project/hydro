use std::any::Any;
use std::ops::Deref;
use std::sync::{Arc, OnceLock, Weak};

use anyhow::{bail, Result};
use async_trait::async_trait;
use hydroflow_deploy_integration::{ConnectedDirect, ServerPort};
use tokio::sync::RwLock;

use super::hydroflow_crate::ports::{
    HydroflowServer, HydroflowSink, HydroflowSource, ServerConfig, SourcePath,
};
use super::{Host, LaunchedHost, ResourceBatch, ResourceResult, ServerStrategy, Service};
use crate::hydroflow_crate::ports::ReverseSinkInstantiator;

/// Represents an unknown, third-party service that is not part of the Hydroflow ecosystem.
pub struct CustomService {
    _id: usize,
    on: Arc<dyn Host>,

    /// The ports that the service wishes to expose to the public internet.
    external_ports: Vec<u16>,

    launched_host: Option<Arc<dyn LaunchedHost>>,
}

impl CustomService {
    pub fn new(id: usize, on: Arc<dyn Host>, external_ports: Vec<u16>) -> Self {
        Self {
            _id: id,
            on,
            external_ports,
            launched_host: None,
        }
    }

    pub fn declare_client(&self, self_arc: &Arc<RwLock<Self>>) -> CustomClientPort {
        CustomClientPort::new(Arc::downgrade(self_arc))
    }
}

#[async_trait]
impl Service for CustomService {
    fn collect_resources(&self, _resource_batch: &mut ResourceBatch) {
        if self.launched_host.is_some() {
            return;
        }

        let host = &self.on;

        for port in self.external_ports.iter() {
            host.request_port(&ServerStrategy::ExternalTcpPort(*port));
        }
    }

    async fn deploy(&mut self, resource_result: &Arc<ResourceResult>) -> Result<()> {
        if self.launched_host.is_some() {
            return Ok(());
        }

        let host = &self.on;
        let launched = host.provision(resource_result).await;
        self.launched_host = Some(launched);
        Ok(())
    }

    async fn ready(&mut self) -> Result<()> {
        Ok(())
    }

    async fn start(&mut self) -> Result<()> {
        Ok(())
    }

    async fn stop(&mut self) -> Result<()> {
        Ok(())
    }
}

pub struct CustomClientPort {
    pub on: Weak<RwLock<CustomService>>,
    client_port: OnceLock<ServerConfig>,
}

impl CustomClientPort {
    pub fn new(on: Weak<RwLock<CustomService>>) -> Self {
        Self {
            on,
            client_port: OnceLock::new(),
        }
    }

    pub async fn server_port(&self) -> ServerPort {
        self.client_port
            .get()
            .unwrap()
            .load_instantiated(&|p| p)
            .await
    }

    pub async fn connect(&self) -> ConnectedDirect {
        self.client_port
            .get()
            .unwrap()
            .load_instantiated(&|p| p)
            .await
            .instantiate()
            .connect::<ConnectedDirect>()
            .await
    }
}

impl HydroflowSource for CustomClientPort {
    fn source_path(&self) -> SourcePath {
        SourcePath::Direct(self.on.upgrade().unwrap().try_read().unwrap().on.clone())
    }

    fn host(&self) -> Arc<dyn Host> {
        panic!("Custom services cannot be used as the server")
    }

    fn server(&self) -> Arc<dyn HydroflowServer> {
        panic!("Custom services cannot be used as the server")
    }

    fn record_server_config(&self, config: ServerConfig) {
        self.client_port
            .set(config)
            .map_err(drop) // `ServerConfig` doesn't implement `Debug` for `.expect()`.
            .expect("Cannot call `record_server_config()` multiple times.");
    }

    fn record_server_strategy(&self, _config: ServerStrategy) {
        panic!("Custom services cannot be used as the server")
    }
}

impl HydroflowSink for CustomClientPort {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn instantiate(&self, _client_path: &SourcePath) -> Result<Box<dyn FnOnce() -> ServerConfig>> {
        bail!("Custom services cannot be used as the server")
    }

    fn instantiate_reverse(
        &self,
        server_host: &Arc<dyn Host>,
        server_sink: Arc<dyn HydroflowServer>,
        wrap_client_port: &dyn Fn(ServerConfig) -> ServerConfig,
    ) -> Result<ReverseSinkInstantiator> {
        let client = self.on.upgrade().unwrap();
        let client_read = client.try_read().unwrap();

        let server_host = server_host.clone();

        let (conn_type, bind_type) = server_host.strategy_as_server(client_read.on.deref())?;

        let client_port = wrap_client_port(ServerConfig::from_strategy(&conn_type, server_sink));

        Ok(Box::new(move |me| {
            me.downcast_ref::<CustomClientPort>()
                .unwrap()
                .record_server_config(client_port);
            bind_type(server_host.as_any())
        }))
    }
}
