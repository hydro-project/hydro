use std::any::Any;
use std::collections::HashMap;
use std::fmt::Debug;
use std::ops::Deref;
use std::sync::{Arc, Weak};

use anyhow::{Result, bail};
use append_only_vec::AppendOnlyVec;
use async_recursion::async_recursion;
use dyn_clone::DynClone;
use hydro_deploy_integration::ServerPort;
use tokio::sync::RwLock;

use super::RustCrateService;
use crate::{ClientStrategy, Host, LaunchedHost, PortNetworkHint, ServerStrategy};

pub trait RustCrateSource: Send + Sync {
    fn source_path(&self) -> SourcePath;
    fn record_server_config(&self, config: ServerConfig);

    fn host(&self) -> Arc<dyn Host>;
    fn server(&self) -> Arc<dyn RustCrateServer>;
    fn record_server_strategy(&self, config: ServerStrategy);

    fn wrap_reverse_server_config(&self, config: ServerConfig) -> ServerConfig {
        config
    }

    fn send_to(&self, sink: &dyn RustCrateSink) {
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
            self.record_server_strategy(instantiated(sink));
        }
    }
}

pub trait RustCrateServer: DynClone + Debug + Send + Sync {
    fn get_port(&self) -> ServerPort;
    fn launched_host(&self) -> Arc<dyn LaunchedHost>;
}

pub type ReverseSinkInstantiator = Box<dyn FnOnce(&dyn Any) -> ServerStrategy>;

pub trait RustCrateSink: Any + Send + Sync {
    /// Instantiate the sink as the source host connecting to the sink host.
    /// Returns a thunk that can be called to perform mutations that instantiate the sink.
    fn instantiate(&self, client_path: &SourcePath) -> Result<Box<dyn FnOnce() -> ServerConfig>>;

    /// Instantiate the sink, but as the sink host connecting to the source host.
    /// Returns a thunk that can be called to perform mutations that instantiate the sink, taking a mutable reference to this sink.
    fn instantiate_reverse(
        &self,
        server_host: &Arc<dyn Host>,
        server_sink: Arc<dyn RustCrateServer>,
        wrap_client_port: &dyn Fn(ServerConfig) -> ServerConfig,
    ) -> Result<ReverseSinkInstantiator>;
}

pub struct TaggedSource {
    pub source: Arc<dyn RustCrateSource>,
    pub tag: u32,
}

impl RustCrateSource for TaggedSource {
    fn source_path(&self) -> SourcePath {
        SourcePath::Tagged(Box::new(self.source.source_path()), self.tag)
    }

    fn record_server_config(&self, config: ServerConfig) {
        self.source.record_server_config(config);
    }

    fn host(&self) -> Arc<dyn Host> {
        self.source.host()
    }

    fn server(&self) -> Arc<dyn RustCrateServer> {
        self.source.server()
    }

    fn wrap_reverse_server_config(&self, config: ServerConfig) -> ServerConfig {
        ServerConfig::Tagged(Box::new(config), self.tag)
    }

    fn record_server_strategy(&self, config: ServerStrategy) {
        self.source.record_server_strategy(config);
    }
}

pub struct NullSourceSink;

impl RustCrateSource for NullSourceSink {
    fn source_path(&self) -> SourcePath {
        SourcePath::Null
    }

    fn host(&self) -> Arc<dyn Host> {
        panic!("null source has no host")
    }

    fn server(&self) -> Arc<dyn RustCrateServer> {
        panic!("null source has no server")
    }

    fn record_server_config(&self, _config: ServerConfig) {}
    fn record_server_strategy(&self, _config: ServerStrategy) {}
}

impl RustCrateSink for NullSourceSink {
    fn instantiate(&self, _client_path: &SourcePath) -> Result<Box<dyn FnOnce() -> ServerConfig>> {
        Ok(Box::new(|| ServerConfig::Null))
    }

    fn instantiate_reverse(
        &self,
        _server_host: &Arc<dyn Host>,
        _server_sink: Arc<dyn RustCrateServer>,
        _wrap_client_port: &dyn Fn(ServerConfig) -> ServerConfig,
    ) -> Result<ReverseSinkInstantiator> {
        Ok(Box::new(|_| ServerStrategy::Null))
    }
}

pub struct DemuxSink {
    pub demux: HashMap<u32, Box<dyn RustCrateSink>>,
}

impl RustCrateSink for DemuxSink {
    fn instantiate(&self, client_host: &SourcePath) -> Result<Box<dyn FnOnce() -> ServerConfig>> {
        let mut thunk_map = HashMap::new();
        for (key, target) in &self.demux {
            thunk_map.insert(*key, target.instantiate(client_host)?);
        }

        Ok(Box::new(move || {
            let instantiated_map = thunk_map
                .into_iter()
                .map(|(key, thunk)| (key, (thunk)()))
                .collect();

            ServerConfig::Demux(instantiated_map)
        }))
    }

    fn instantiate_reverse(
        &self,
        server_host: &Arc<dyn Host>,
        server_sink: Arc<dyn RustCrateServer>,
        wrap_client_port: &dyn Fn(ServerConfig) -> ServerConfig,
    ) -> Result<ReverseSinkInstantiator> {
        let mut thunk_map = HashMap::new();
        for (key, target) in &self.demux {
            thunk_map.insert(
                *key,
                target.instantiate_reverse(
                    server_host,
                    server_sink.clone(),
                    // the parent wrapper selects the demux port for the parent defn, so do that first
                    &|p| ServerConfig::DemuxSelect(Box::new(wrap_client_port(p)), *key),
                )?,
            );
        }

        Ok(Box::new(move |me| {
            let me = me.downcast_ref::<DemuxSink>().unwrap();
            let instantiated_map = thunk_map
                .into_iter()
                .map(|(key, thunk)| (key, (thunk)(me.demux.get(&key).unwrap())))
                .collect();

            ServerStrategy::Demux(instantiated_map)
        }))
    }
}

#[derive(Clone, Debug)]
pub struct RustCratePortConfig {
    pub service: Weak<RustCrateService>,
    pub service_host: Arc<dyn Host>,
    pub service_server_defns: Arc<RwLock<HashMap<String, ServerPort>>>,
    pub network_hint: PortNetworkHint,
    pub port: String,
    pub merge: bool,
}

impl RustCratePortConfig {
    pub fn merge(&self) -> Self {
        Self {
            service: self.service.clone(),
            service_host: self.service_host.clone(),
            service_server_defns: self.service_server_defns.clone(),
            network_hint: self.network_hint,
            port: self.port.clone(),
            merge: true,
        }
    }
}

impl RustCrateSource for RustCratePortConfig {
    fn source_path(&self) -> SourcePath {
        SourcePath::Direct(self.service.upgrade().unwrap().on.clone())
    }

    fn host(&self) -> Arc<dyn Host> {
        self.service_host.clone()
    }

    fn server(&self) -> Arc<dyn RustCrateServer> {
        let from = self.service.upgrade().unwrap();

        Arc::new(RustCratePortConfig {
            service: Arc::downgrade(&from),
            service_host: from.on.clone(),
            service_server_defns: from.server_defns.clone(),
            network_hint: self.network_hint,
            port: self.port.clone(),
            merge: false,
        })
    }

    fn record_server_config(&self, config: ServerConfig) {
        let from = self.service.upgrade().unwrap();
        // TODO(shadaj): if already in this map, we want to broadcast
        assert!(
            from.port_to_server.insert(self.port.clone(), config),
            "The port configuration is incorrect, for example, are you using a ConnectedDirect instead of a ConnectedDemux?"
        );
    }

    fn record_server_strategy(&self, config: ServerStrategy) {
        let from = self.service.upgrade().unwrap();
        assert!(
            from.port_to_bind.insert(self.port.clone(), config),
            "port already set!"
        );
    }
}

impl RustCrateServer for RustCratePortConfig {
    fn get_port(&self) -> ServerPort {
        // we are in `deployment.start()`, so no one should be writing
        let server_defns = self.service_server_defns.try_read().unwrap();
        server_defns.get(&self.port).unwrap().clone()
    }

    fn launched_host(&self) -> Arc<dyn LaunchedHost> {
        self.service_host.launched().unwrap()
    }
}

pub enum SourcePath {
    Null,
    Direct(Arc<dyn Host>),
    Many(Arc<dyn Host>),
    Tagged(Box<SourcePath>, u32),
}

impl SourcePath {
    #[expect(
        clippy::type_complexity,
        reason = "internals (dyn Fn to defer instantiation)"
    )]
    fn plan<T: RustCrateServer + Clone + 'static>(
        &self,
        server: &T,
        server_host: &dyn Host,
        network_hint: PortNetworkHint,
    ) -> Result<(Box<dyn FnOnce(&dyn Any) -> ServerStrategy>, ServerConfig)> {
        match self {
            SourcePath::Direct(client_host) => {
                let (conn_type, bind_type) =
                    server_host.strategy_as_server(client_host.deref(), network_hint)?;
                let base_config = ServerConfig::from_strategy(&conn_type, Arc::new(server.clone()));
                Ok((
                    Box::new(|host| ServerStrategy::Direct(bind_type(host))),
                    base_config,
                ))
            }

            SourcePath::Many(client_host) => {
                let (conn_type, bind_type) =
                    server_host.strategy_as_server(client_host.deref(), network_hint)?;
                let base_config = ServerConfig::from_strategy(&conn_type, Arc::new(server.clone()));
                Ok((
                    Box::new(|host| ServerStrategy::Many(bind_type(host))),
                    base_config,
                ))
            }

            SourcePath::Tagged(underlying, tag) => {
                let (bind_type, base_config) =
                    underlying.plan(server, server_host, network_hint)?;
                let tag = *tag;
                Ok((
                    Box::new(move |host| ServerStrategy::Tagged(Box::new(bind_type(host)), tag)),
                    ServerConfig::TaggedUnwrap(Box::new(base_config)),
                ))
            }

            SourcePath::Null => Ok((Box::new(|_| ServerStrategy::Null), ServerConfig::Null)),
        }
    }
}

impl RustCrateSink for RustCratePortConfig {
    fn instantiate(&self, client_path: &SourcePath) -> Result<Box<dyn FnOnce() -> ServerConfig>> {
        let server = self.service.upgrade().unwrap();

        let server_host = server.on.clone();

        let (bind_type, base_config) =
            client_path.plan(self, server_host.deref(), self.network_hint)?;

        let server = server.clone();
        let merge = self.merge;
        let port = self.port.clone();
        Ok(Box::new(move || {
            let bind_type = (bind_type)(&*server.on);

            if merge {
                let merge_config = server
                    .port_to_bind
                    .get_or_insert_owned(port, || ServerStrategy::Merge(Default::default()));
                let ServerStrategy::Merge(merge) = merge_config else {
                    panic!("Expected a merge connection definition")
                };
                merge.push(bind_type);
                ServerConfig::MergeSelect(Box::new(base_config), merge.len() - 1)
            } else {
                assert!(
                    server.port_to_bind.insert(port.clone(), bind_type),
                    "port already set!"
                );
                base_config
            }
        }))
    }

    fn instantiate_reverse(
        &self,
        server_host: &Arc<dyn Host>,
        server_sink: Arc<dyn RustCrateServer>,
        wrap_client_port: &dyn Fn(ServerConfig) -> ServerConfig,
    ) -> Result<ReverseSinkInstantiator> {
        if !matches!(self.network_hint, PortNetworkHint::Auto) {
            bail!("Trying to form collection where I am the client, but I have server hint")
        }

        let client = self.service.upgrade().unwrap();

        let server_host = server_host.clone();

        let (conn_type, bind_type) =
            server_host.strategy_as_server(&*client.on, PortNetworkHint::Auto)?;
        let client_port = wrap_client_port(ServerConfig::from_strategy(&conn_type, server_sink));

        let client = client.clone();
        let merge = self.merge;
        let port = self.port.clone();
        Ok(Box::new(move |_| {
            if merge {
                let merge_config = client
                    .port_to_server
                    .get_or_insert_owned(port, || ServerConfig::Merge(Default::default()));
                let ServerConfig::Merge(merge) = merge_config else {
                    panic!()
                };
                merge.push(client_port);
            } else {
                assert!(
                    client.port_to_server.insert(port.clone(), client_port),
                    "port already set!"
                );
            };

            ServerStrategy::Direct((bind_type)(&*client.on))
        }))
    }
}

#[derive(Clone, Debug)]
pub enum ServerConfig {
    Direct(Arc<dyn RustCrateServer>),
    Forwarded(Arc<dyn RustCrateServer>),
    /// A demux that will be used at runtime to listen to many connections.
    Demux(HashMap<u32, ServerConfig>),
    /// The other side of a demux, with a port to extract the appropriate connection.
    DemuxSelect(Box<ServerConfig>, u32),
    /// A merge that will be used at runtime to combine many connections.
    /// AppendOnlyVec has a quite large inline array, so we box it.
    Merge(Box<AppendOnlyVec<ServerConfig>>),
    /// The other side of a merge, with a port to extract the appropriate connection.
    MergeSelect(Box<ServerConfig>, usize),
    Tagged(Box<ServerConfig>, u32),
    TaggedUnwrap(Box<ServerConfig>),
    Null,
}

impl ServerConfig {
    pub fn from_strategy(
        strategy: &ClientStrategy,
        server: Arc<dyn RustCrateServer>,
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
                forward_connection(&selected, server.launched_host().as_ref()).await
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
                for conn in merge.iter() {
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
