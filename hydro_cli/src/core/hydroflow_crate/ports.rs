use std::any::Any;
use std::collections::HashMap;
use std::ops::Deref;
use std::sync::{Arc, Weak};

use anyhow::Result;
use async_recursion::async_recursion;
use async_trait::async_trait;
use dyn_clone::DynClone;
use hydroflow_cli_integration::ServerPort;
use tokio::sync::RwLock;

use super::HydroflowCrate;
use crate::core::{ClientStrategy, Host, HostStrategyGetter, LaunchedHost, ServerStrategy};

pub trait HydroflowSource: Send + Sync {
    fn source_path(&self) -> SourcePath;
    fn record_server_config(&mut self, config: ServerConfig);

    fn host(&self) -> Arc<RwLock<dyn Host>>;
    fn server(&self) -> Arc<dyn HydroflowServer>;
    fn record_server_strategy(&mut self, config: ServerStrategy);

    fn wrap_reverse_server_config(&self, config: ServerConfig) -> ServerConfig {
        config
    }

    fn send_to(&mut self, sink: &mut dyn HydroflowSink) {
        let forward_res = sink.instantiate(&self.source_path());
        if let Ok(instantiated) = forward_res {
            self.record_server_config(instantiated());
        } else {
            drop(forward_res);
            let instantiated = sink
                .instantiate_reverse(&self.host(), self.server(), &|p| {
                    self.wrap_reverse_server_config(p)
                })
                .unwrap();
            self.record_server_strategy(instantiated(sink.as_any_mut()));
        }
    }
}

#[async_trait]
pub trait HydroflowServer: DynClone + Send + Sync {
    fn get_port(&self) -> ServerPort;
    async fn launched_host(&self) -> Arc<dyn LaunchedHost>;
}

pub type ReverseSinkInstantiator = Box<dyn FnOnce(&mut dyn Any) -> ServerStrategy>;

pub trait HydroflowSink: Send + Sync {
    fn as_any_mut(&mut self) -> &mut dyn Any;

    /// Instantiate the sink as the source host connecting to the sink host.
    /// Returns a thunk that can be called to perform mutations that instantiate the sink.
    fn instantiate(&self, client_path: &SourcePath) -> Result<Box<dyn FnOnce() -> ServerConfig>>;

    /// Instantiate the sink, but as the sink host connecting to the source host.
    /// Returns a thunk that can be called to perform mutations that instantiate the sink, taking a mutable reference to this sink.
    fn instantiate_reverse(
        &self,
        server_host: &Arc<RwLock<dyn Host>>,
        server_sink: Arc<dyn HydroflowServer>,
        wrap_client_port: &dyn Fn(ServerConfig) -> ServerConfig,
    ) -> Result<ReverseSinkInstantiator>;
}

pub struct TaggedSource {
    pub source: Arc<RwLock<dyn HydroflowSource>>,
    pub tag: u32,
}

impl HydroflowSource for TaggedSource {
    fn source_path(&self) -> SourcePath {
        SourcePath::Tagged(
            Box::new(self.source.try_read().unwrap().source_path()),
            self.tag,
        )
    }

    fn record_server_config(&mut self, config: ServerConfig) {
        self.source
            .try_write()
            .unwrap()
            .record_server_config(config);
    }

    fn host(&self) -> Arc<RwLock<dyn Host>> {
        self.source.try_read().unwrap().host()
    }

    fn server(&self) -> Arc<dyn HydroflowServer> {
        self.source.try_read().unwrap().server()
    }

    fn wrap_reverse_server_config(&self, config: ServerConfig) -> ServerConfig {
        ServerConfig::Tagged(Box::new(config), self.tag)
    }

    fn record_server_strategy(&mut self, config: ServerStrategy) {
        self.source
            .try_write()
            .unwrap()
            .record_server_strategy(config);
    }
}

pub struct NullSourceSink;

impl HydroflowSource for NullSourceSink {
    fn source_path(&self) -> SourcePath {
        SourcePath::Null
    }

    fn host(&self) -> Arc<RwLock<dyn Host>> {
        panic!("null source has no host")
    }

    fn server(&self) -> Arc<dyn HydroflowServer> {
        panic!("null source has no server")
    }

    fn record_server_config(&mut self, _config: ServerConfig) {}
    fn record_server_strategy(&mut self, _config: ServerStrategy) {}
}

impl HydroflowSink for NullSourceSink {
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }

    fn instantiate(&self, _client_path: &SourcePath) -> Result<Box<dyn FnOnce() -> ServerConfig>> {
        Ok(Box::new(|| ServerConfig::Null))
    }

    fn instantiate_reverse(
        &self,
        _server_host: &Arc<RwLock<dyn Host>>,
        _server_sink: Arc<dyn HydroflowServer>,
        _wrap_client_port: &dyn Fn(ServerConfig) -> ServerConfig,
    ) -> Result<ReverseSinkInstantiator> {
        Ok(Box::new(|_| ServerStrategy::Null))
    }
}

pub struct DemuxSink {
    pub demux: HashMap<u32, Arc<RwLock<dyn HydroflowSink>>>,
}

impl HydroflowSink for DemuxSink {
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }

    fn instantiate(&self, client_host: &SourcePath) -> Result<Box<dyn FnOnce() -> ServerConfig>> {
        let mut thunk_map = HashMap::new();
        for (key, target) in &self.demux {
            thunk_map.insert(*key, target.try_read().unwrap().instantiate(client_host)?);
        }

        Ok(Box::new(move || {
            let instantiated_map = thunk_map
                .into_iter()
                .map(|(key, thunk)| (key, thunk()))
                .collect();

            ServerConfig::Demux(instantiated_map)
        }))
    }

    fn instantiate_reverse(
        &self,
        server_host: &Arc<RwLock<dyn Host>>,
        server_sink: Arc<dyn HydroflowServer>,
        wrap_client_port: &dyn Fn(ServerConfig) -> ServerConfig,
    ) -> Result<Box<dyn FnOnce(&mut dyn Any) -> ServerStrategy>> {
        let mut thunk_map = HashMap::new();
        for (key, target) in &self.demux {
            thunk_map.insert(
                *key,
                target.try_write().unwrap().instantiate_reverse(
                    server_host,
                    server_sink.clone(),
                    // the parent wrapper selects the demux port for the parent defn, so do that first
                    &|p| ServerConfig::DemuxSelect(Box::new(wrap_client_port(p)), *key),
                )?,
            );
        }

        Ok(Box::new(move |me| {
            let me = me.downcast_mut::<DemuxSink>().unwrap();
            let instantiated_map = thunk_map
                .into_iter()
                .map(|(key, thunk)| {
                    (
                        key,
                        thunk(
                            me.demux
                                .get_mut(&key)
                                .unwrap()
                                .try_write()
                                .unwrap()
                                .as_any_mut(),
                        ),
                    )
                })
                .collect();

            ServerStrategy::Demux(instantiated_map)
        }))
    }
}

#[derive(Clone)]
pub struct HydroflowPortConfig {
    pub service: Weak<RwLock<HydroflowCrate>>,
    pub service_host: Arc<RwLock<dyn Host>>,
    pub service_server_defns: Arc<RwLock<HashMap<String, ServerPort>>>,
    pub port: String,
    pub merge: bool,
}

impl HydroflowPortConfig {
    pub fn merge(&self) -> Self {
        Self {
            service: self.service.clone(),
            service_host: self.service_host.clone(),
            service_server_defns: self.service_server_defns.clone(),
            port: self.port.clone(),
            merge: true,
        }
    }
}

impl HydroflowSource for HydroflowPortConfig {
    fn source_path(&self) -> SourcePath {
        SourcePath::Direct(
            self.service
                .upgrade()
                .unwrap()
                .try_read()
                .unwrap()
                .on
                .clone(),
        )
    }

    fn host(&self) -> Arc<RwLock<dyn Host>> {
        self.service_host.clone()
    }

    fn server(&self) -> Arc<dyn HydroflowServer> {
        let from = self.service.upgrade().unwrap();
        let from_read = from.try_read().unwrap();

        Arc::new(HydroflowPortConfig {
            service: Arc::downgrade(&from),
            service_host: from_read.on.clone(),
            service_server_defns: from_read.server_defns.clone(),
            port: self.port.clone(),
            merge: false,
        })
    }

    fn record_server_config(&mut self, config: ServerConfig) {
        let from = self.service.upgrade().unwrap();
        let mut from_write = from.try_write().unwrap();

        // TODO(shadaj): if already in this map, we want to broadcast
        assert!(!from_write.port_to_server.contains_key(&self.port));
        from_write.port_to_server.insert(self.port.clone(), config);
    }

    fn record_server_strategy(&mut self, config: ServerStrategy) {
        let from = self.service.upgrade().unwrap();
        let mut from_write = from.try_write().unwrap();

        assert!(!from_write.port_to_bind.contains_key(&self.port));
        from_write.port_to_bind.insert(self.port.clone(), config);
    }
}

#[async_trait]
impl HydroflowServer for HydroflowPortConfig {
    fn get_port(&self) -> ServerPort {
        // we are in `deployment.start()`, so no one should be writing
        let server_defns = self.service_server_defns.try_read().unwrap();
        server_defns.get(&self.port).unwrap().clone()
    }

    async fn launched_host(&self) -> Arc<dyn LaunchedHost> {
        self.service_host.read().await.launched().unwrap()
    }
}

pub enum SourcePath {
    Null,
    Direct(Arc<RwLock<dyn Host>>),
    Tagged(Box<SourcePath>, u32),
}

impl SourcePath {
    fn plan<T: HydroflowServer + Clone + 'static>(
        &self,
        server: &T,
        server_host: &dyn Host,
    ) -> Result<(HostStrategyGetter, ServerConfig)> {
        match self {
            SourcePath::Direct(client_host) => {
                let client_host = client_host.try_read().unwrap();
                let (conn_type, bind_type) = server_host.strategy_as_server(client_host.deref())?;
                let base_config = ServerConfig::from_strategy(&conn_type, Arc::new(server.clone()));
                Ok((bind_type, base_config))
            }

            SourcePath::Tagged(underlying, tag) => {
                let (bind_type, base_config) = underlying.plan(server, server_host)?;
                let tag = *tag;
                let strategy_getter: HostStrategyGetter =
                    Box::new(move |host| ServerStrategy::Tagged(Box::new(bind_type(host)), tag));
                Ok((
                    strategy_getter,
                    ServerConfig::TaggedUnwrap(Box::new(base_config)),
                ))
            }

            SourcePath::Null => {
                let strategy_getter: HostStrategyGetter = Box::new(|_| ServerStrategy::Null);
                Ok((strategy_getter, ServerConfig::Null))
            }
        }
    }
}

impl HydroflowSink for HydroflowPortConfig {
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }

    fn instantiate(&self, client_path: &SourcePath) -> Result<Box<dyn FnOnce() -> ServerConfig>> {
        let server = self.service.upgrade().unwrap();
        let server_read = server.try_read().unwrap();

        let server_host_clone = server_read.on.clone();
        let server_host = server_host_clone.try_read().unwrap();

        let (bind_type, base_config) = client_path.plan(self, server_host.deref())?;

        let server = server.clone();
        let merge = self.merge;
        let port = self.port.clone();
        Ok(Box::new(move || {
            let mut server_write = server.try_write().unwrap();
            let bind_type = bind_type(server_write.on.try_write().unwrap().as_any_mut());

            if merge {
                let merge_config = server_write
                    .port_to_bind
                    .entry(port.clone())
                    .or_insert(ServerStrategy::Merge(vec![]));
                let merge_index = if let ServerStrategy::Merge(merge) = merge_config {
                    merge.push(bind_type);
                    merge.len() - 1
                } else {
                    panic!()
                };

                ServerConfig::MergeSelect(Box::new(base_config), merge_index)
            } else {
                assert!(!server_write.port_to_bind.contains_key(&port));
                server_write.port_to_bind.insert(port.clone(), bind_type);
                base_config
            }
        }))
    }

    fn instantiate_reverse(
        &self,
        server_host: &Arc<RwLock<dyn Host>>,
        server_sink: Arc<dyn HydroflowServer>,
        wrap_client_port: &dyn Fn(ServerConfig) -> ServerConfig,
    ) -> Result<Box<dyn FnOnce(&mut dyn Any) -> ServerStrategy>> {
        let client = self.service.upgrade().unwrap();
        let client_read = client.try_read().unwrap();

        let server_host_clone = server_host.clone();
        let server_host = server_host_clone.try_read().unwrap();

        let (conn_type, bind_type) =
            server_host.strategy_as_server(client_read.on.try_read().unwrap().deref())?;
        let client_port = wrap_client_port(ServerConfig::from_strategy(&conn_type, server_sink));

        let client = client.clone();
        let merge = self.merge;
        let port = self.port.clone();
        Ok(Box::new(move |_| {
            let mut client_write = client.try_write().unwrap();

            if merge {
                let merge_config = client_write
                    .port_to_server
                    .entry(port.clone())
                    .or_insert(ServerConfig::Merge(vec![]));

                if let ServerConfig::Merge(merge) = merge_config {
                    merge.push(client_port);
                } else {
                    panic!()
                };
            } else {
                assert!(!client_write.port_to_server.contains_key(&port));
                client_write
                    .port_to_server
                    .insert(port.clone(), client_port);
            };

            let mut server_host = client_write.on.try_write().unwrap();
            bind_type(server_host.as_any_mut())
        }))
    }
}

#[derive(Clone)]
pub enum ServerConfig {
    Direct(Arc<dyn HydroflowServer>),
    Forwarded(Arc<dyn HydroflowServer>),
    /// A demux that will be used at runtime to listen to many connections.
    Demux(HashMap<u32, ServerConfig>),
    /// The other side of a demux, with a port to extract the appropriate connection.
    DemuxSelect(Box<ServerConfig>, u32),
    /// A merge that will be used at runtime to combine many connections.
    Merge(Vec<ServerConfig>),
    /// The other side of a merge, with a port to extract the appropriate connection.
    MergeSelect(Box<ServerConfig>, usize),
    Tagged(Box<ServerConfig>, u32),
    TaggedUnwrap(Box<ServerConfig>),
    Null,
}

impl ServerConfig {
    pub fn from_strategy(
        strategy: &ClientStrategy,
        server: Arc<dyn HydroflowServer>,
    ) -> ServerConfig {
        match strategy {
            ClientStrategy::UnixSocket(_) | ClientStrategy::InternalTcpPort(_) => {
                ServerConfig::Direct(server)
            }
            ClientStrategy::ForwardedTcpPort(_) => ServerConfig::Forwarded(server),
        }
    }
}

#[async_recursion]
async fn forward_connection(conn: &ServerPort, target: &dyn LaunchedHost) -> ServerPort {
    match conn {
        ServerPort::UnixSocket(_) => panic!("Expected a TCP port to be forwarded"),
        ServerPort::TcpPort(addr) => ServerPort::TcpPort(target.forward_port(addr).await.unwrap()),
        ServerPort::Demux(demux) => {
            let mut forwarded_map = HashMap::new();
            for (key, conn) in demux {
                forwarded_map.insert(*key, forward_connection(conn, target).await);
            }
            ServerPort::Demux(forwarded_map)
        }
        ServerPort::Merge(merge) => {
            let mut forwarded_vec = Vec::new();
            for conn in merge {
                forwarded_vec.push(forward_connection(conn, target).await);
            }
            ServerPort::Merge(forwarded_vec)
        }
        ServerPort::Tagged(underlying, id) => {
            ServerPort::Tagged(Box::new(forward_connection(underlying, target).await), *id)
        }
        ServerPort::Null => ServerPort::Null,
    }
}

impl ServerConfig {
    #[async_recursion]
    pub async fn load_instantiated(
        &self,
        select: &(dyn Fn(ServerPort) -> ServerPort + Send + Sync),
    ) -> ServerPort {
        match self {
            ServerConfig::Direct(server) => select(server.get_port()),

            ServerConfig::Forwarded(server) => {
                let selected = select(server.get_port());
                forward_connection(&selected, server.launched_host().await.as_ref()).await
            }

            ServerConfig::Demux(demux) => {
                let mut demux_map = HashMap::new();
                for (key, conn) in demux {
                    demux_map.insert(*key, conn.load_instantiated(select).await);
                }
                ServerPort::Demux(demux_map)
            }

            ServerConfig::DemuxSelect(underlying, key) => {
                let key = *key;
                underlying
                    .load_instantiated(
                        &(move |p| {
                            if let ServerPort::Demux(mut mapping) = p {
                                select(mapping.remove(&key).unwrap())
                            } else {
                                panic!("Expected a demux connection definition")
                            }
                        }),
                    )
                    .await
            }

            ServerConfig::Merge(merge) => {
                let mut merge_vec = Vec::new();
                for conn in merge {
                    merge_vec.push(conn.load_instantiated(select).await);
                }
                ServerPort::Merge(merge_vec)
            }

            ServerConfig::MergeSelect(underlying, key) => {
                let key = *key;
                underlying
                    .load_instantiated(
                        &(move |p| {
                            if let ServerPort::Merge(mut mapping) = p {
                                select(mapping.remove(key))
                            } else {
                                panic!("Expected a merge connection definition")
                            }
                        }),
                    )
                    .await
            }

            ServerConfig::Tagged(underlying, id) => {
                ServerPort::Tagged(Box::new(underlying.load_instantiated(select).await), *id)
            }

            ServerConfig::TaggedUnwrap(underlying) => {
                let loaded = underlying.load_instantiated(select).await;
                dbg!(&loaded);
                if let ServerPort::Tagged(underlying, _) = loaded {
                    *underlying
                } else {
                    panic!("Expected a tagged connection definition")
                }
            }

            ServerConfig::Null => ServerPort::Null,
        }
    }
}
