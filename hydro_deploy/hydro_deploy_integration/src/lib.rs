use std::cell::RefCell;
use std::collections::HashMap;
use std::marker::PhantomData;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::pin::Pin;
use std::task::{Context, Poll};
use std::time::Duration;

use async_recursion::async_recursion;
use async_trait::async_trait;
use bytes::{Bytes, BytesMut};
use futures::sink::Buffer;
use futures::{Future, Sink, SinkExt, Stream, ready, stream};
use pin_project::pin_project;
use serde::{Deserialize, Serialize};
use tokio::io;
use tokio::net::{TcpListener, TcpStream};
#[cfg(unix)]
use tokio::net::{UnixListener, UnixStream};
use tokio::task::JoinHandle;
use tokio_stream::StreamExt;
use tokio_stream::wrappers::TcpListenerStream;
use tokio_util::codec::{Framed, LengthDelimitedCodec};

pub type InitConfig = (HashMap<String, ServerBindConfig>, Option<String>);

/// Contains runtime information passed by Hydro Deploy to a program,
/// describing how to connect to other services and metadata about them.
pub struct DeployPorts<T = Option<()>> {
    pub ports: RefCell<HashMap<String, Connection>>,
    pub meta: T,
}

impl<T> DeployPorts<T> {
    pub fn port(&self, name: &str) -> Connection {
        self.ports
            .try_borrow_mut()
            .unwrap()
            .remove(name)
            .unwrap_or_else(|| panic!("port {} not found", name))
    }
}

#[cfg(not(unix))]
type UnixStream = std::convert::Infallible;

#[cfg(not(unix))]
#[expect(dead_code, reason = "conditional compilation placeholder")]
type UnixListener = std::convert::Infallible;

/// Describes how to connect to a service which is listening on some port.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum ServerPort {
    UnixSocket(PathBuf),
    TcpPort(SocketAddr),
    Demux(HashMap<u32, ServerPort>),
    Merge(Vec<ServerPort>),
    Tagged(Box<ServerPort>, u32),
    Null,
}

impl ServerPort {
    pub fn connect(&self) -> ClientConnection {
        match self {
            ServerPort::UnixSocket(path) => {
                #[cfg(unix)]
                {
                    let bound = UnixStream::connect(path.clone());
                    ClientConnection::UnixSocket(tokio::spawn(bound))
                }

                #[cfg(not(unix))]
                {
                    let _ = path;
                    panic!("Unix sockets are not supported on this platform")
                }
            }
            ServerPort::TcpPort(addr) => {
                let addr_clone = *addr;
                let bound = async_retry(
                    move || TcpStream::connect(addr_clone),
                    10,
                    Duration::from_secs(1),
                );
                ClientConnection::TcpPort(tokio::spawn(bound))
            }
            ServerPort::Demux(bindings) => {
                ClientConnection::Demux(bindings.iter().map(|(k, v)| (*k, v.connect())).collect())
            }
            ServerPort::Merge(ports) => {
                ClientConnection::Merge(ports.iter().map(|p| p.connect()).collect())
            }
            ServerPort::Tagged(port, tag) => {
                ClientConnection::Tagged(Box::new(port.as_ref().connect()), *tag)
            }
            ServerPort::Null => ClientConnection::Null,
        }
    }

    pub fn instantiate(&self) -> Connection {
        Connection::AsClient(self.connect())
    }
}

#[derive(Debug)]
pub enum ClientConnection {
    UnixSocket(JoinHandle<io::Result<UnixStream>>),
    TcpPort(JoinHandle<io::Result<TcpStream>>),
    Demux(HashMap<u32, ClientConnection>),
    Merge(Vec<ClientConnection>),
    Tagged(Box<ClientConnection>, u32),
    Null,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum ServerBindConfig {
    UnixSocket,
    TcpPort(
        /// The host the port should be bound on.
        String,
    ),
    Demux(HashMap<u32, ServerBindConfig>),
    Merge(Vec<ServerBindConfig>),
    Tagged(Box<ServerBindConfig>, u32),
    Null,
}

impl ServerBindConfig {
    #[async_recursion]
    pub async fn bind(self) -> BoundServer {
        match self {
            ServerBindConfig::UnixSocket => {
                #[cfg(unix)]
                {
                    let dir = tempfile::tempdir().unwrap();
                    let socket_path = dir.path().join("socket");
                    let bound = UnixListener::bind(socket_path).unwrap();
                    BoundServer::UnixSocket(
                        tokio::spawn(async move { Ok(bound.accept().await?.0) }),
                        dir,
                    )
                }

                #[cfg(not(unix))]
                {
                    panic!("Unix sockets are not supported on this platform")
                }
            }
            ServerBindConfig::TcpPort(host) => {
                let listener = TcpListener::bind((host, 0)).await.unwrap();
                let addr = listener.local_addr().unwrap();
                BoundServer::TcpPort(TcpListenerStream::new(listener), addr)
            }
            ServerBindConfig::Demux(bindings) => {
                let mut demux = HashMap::new();
                for (key, bind) in bindings {
                    demux.insert(key, bind.bind().await);
                }
                BoundServer::Demux(demux)
            }
            ServerBindConfig::Merge(bindings) => {
                let mut merge = Vec::new();
                for bind in bindings {
                    merge.push(bind.bind().await);
                }
                BoundServer::Merge(merge)
            }
            ServerBindConfig::Tagged(underlying, id) => {
                BoundServer::Tagged(Box::new(underlying.bind().await), id)
            }
            ServerBindConfig::Null => BoundServer::Null,
        }
    }
}

#[derive(Debug)]
pub enum Connection {
    AsClient(ClientConnection),
    AsServer(BoundServer),
}

impl Connection {
    pub async fn connect<T: Connected>(self) -> T {
        T::from_defn(self).await
    }

    pub fn connect_local_blocking<T: Connected>(self) -> T {
        let handle = tokio::runtime::Handle::current();
        let _guard = handle.enter();
        futures::executor::block_on(T::from_defn(self))
    }

    pub async fn accept_tcp(&mut self) -> TcpStream {
        if let Connection::AsServer(BoundServer::TcpPort(handle, _)) = self {
            handle.next().await.unwrap().unwrap()
        } else {
            panic!("Not a TCP port")
        }
    }
}

pub type DynStream = Pin<Box<dyn Stream<Item = Result<BytesMut, io::Error>> + Send + Sync>>;

pub type DynSink<Input> = Pin<Box<dyn Sink<Input, Error = io::Error> + Send + Sync>>;

pub trait StreamSink:
    Stream<Item = Result<BytesMut, io::Error>> + Sink<Bytes, Error = io::Error>
{
}
impl<T: Stream<Item = Result<BytesMut, io::Error>> + Sink<Bytes, Error = io::Error>> StreamSink
    for T
{
}

pub type DynStreamSink = Pin<Box<dyn StreamSink + Send + Sync>>;

#[async_trait]
pub trait Connected: Send {
    async fn from_defn(pipe: Connection) -> Self;
}

pub trait ConnectedSink {
    type Input: Send;
    type Sink: Sink<Self::Input, Error = io::Error> + Send + Sync;

    fn into_sink(self) -> Self::Sink;
}

pub trait ConnectedSource {
    type Output: Send;
    type Stream: Stream<Item = Result<Self::Output, io::Error>> + Send + Sync;
    fn into_source(self) -> Self::Stream;
}

#[derive(Debug)]
pub enum BoundServer {
    UnixSocket(JoinHandle<io::Result<UnixStream>>, tempfile::TempDir),
    TcpPort(TcpListenerStream, SocketAddr),
    Demux(HashMap<u32, BoundServer>),
    Merge(Vec<BoundServer>),
    Tagged(Box<BoundServer>, u32),
    Null,
}

impl BoundServer {
    pub fn server_port(&self) -> ServerPort {
        match self {
            BoundServer::UnixSocket(_, tempdir) => {
                #[cfg(unix)]
                {
                    ServerPort::UnixSocket(tempdir.path().join("socket"))
                }

                #[cfg(not(unix))]
                {
                    let _ = tempdir;
                    panic!("Unix sockets are not supported on this platform")
                }
            }
            BoundServer::TcpPort(_, addr) => {
                ServerPort::TcpPort(SocketAddr::new(addr.ip(), addr.port()))
            }

            BoundServer::Demux(bindings) => {
                let mut demux = HashMap::new();
                for (key, bind) in bindings {
                    demux.insert(*key, bind.server_port());
                }
                ServerPort::Demux(demux)
            }

            BoundServer::Merge(bindings) => {
                let mut merge = Vec::new();
                for bind in bindings {
                    merge.push(bind.server_port());
                }
                ServerPort::Merge(merge)
            }

            BoundServer::Tagged(underlying, id) => {
                ServerPort::Tagged(Box::new(underlying.server_port()), *id)
            }

            BoundServer::Null => ServerPort::Null,
        }
    }
}

#[async_recursion]
async fn accept(bound: BoundServer) -> ConnectedDirect {
    match bound {
        BoundServer::UnixSocket(listener, _) => {
            #[cfg(unix)]
            {
                let stream = listener.await.unwrap().unwrap();
                ConnectedDirect {
                    stream_sink: Some(Box::pin(unix_bytes(stream))),
                    source_only: None,
                    sink_only: None,
                }
            }

            #[cfg(not(unix))]
            {
                drop(listener);
                panic!("Unix sockets are not supported on this platform")
            }
        }
        BoundServer::TcpPort(mut listener, _) => {
            let stream = listener.next().await.unwrap().unwrap();
            ConnectedDirect {
                stream_sink: Some(Box::pin(tcp_bytes(stream))),
                source_only: None,
                sink_only: None,
            }
        }
        BoundServer::Merge(merge) => {
            let mut sources = vec![];
            for bound in merge {
                sources.push(accept(bound).await.into_source());
            }

            let merge_source: DynStream = Box::pin(MergeSource {
                marker: PhantomData,
                sources,
            });

            ConnectedDirect {
                stream_sink: None,
                source_only: Some(merge_source),
                sink_only: None,
            }
        }
        BoundServer::Demux(_) => panic!("Cannot connect to a demux pipe directly"),
        BoundServer::Tagged(_, _) => panic!("Cannot connect to a tagged pipe directly"),
        BoundServer::Null => {
            ConnectedDirect::from_defn(Connection::AsClient(ClientConnection::Null)).await
        }
    }
}

fn tcp_bytes(stream: TcpStream) -> impl StreamSink {
    Framed::new(stream, LengthDelimitedCodec::new())
}

#[cfg(unix)]
fn unix_bytes(stream: UnixStream) -> impl StreamSink {
    Framed::new(stream, LengthDelimitedCodec::new())
}

struct IoErrorDrain<T> {
    marker: PhantomData<T>,
}

impl<T> Sink<T> for IoErrorDrain<T> {
    type Error = io::Error;

    fn poll_ready(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn start_send(self: Pin<&mut Self>, _item: T) -> Result<(), Self::Error> {
        Ok(())
    }

    fn poll_flush(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn poll_close(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }
}

async fn async_retry<T, E, F: Future<Output = Result<T, E>>>(
    thunk: impl Fn() -> F,
    count: usize,
    delay: Duration,
) -> Result<T, E> {
    for _ in 1..count {
        let result = thunk().await;
        if result.is_ok() {
            return result;
        } else {
            tokio::time::sleep(delay).await;
        }
    }

    thunk().await
}

pub struct ConnectedDirect {
    stream_sink: Option<DynStreamSink>,
    source_only: Option<DynStream>,
    sink_only: Option<DynSink<Bytes>>,
}

#[async_trait]
impl Connected for ConnectedDirect {
    async fn from_defn(pipe: Connection) -> Self {
        match pipe {
            Connection::AsClient(ClientConnection::UnixSocket(stream)) => {
                #[cfg(unix)]
                {
                    let stream = stream.await.unwrap().unwrap();
                    ConnectedDirect {
                        stream_sink: Some(Box::pin(unix_bytes(stream))),
                        source_only: None,
                        sink_only: None,
                    }
                }

                #[cfg(not(unix))]
                {
                    drop(stream);
                    panic!("Unix sockets are not supported on this platform");
                }
            }
            Connection::AsClient(ClientConnection::TcpPort(stream)) => {
                let stream = stream.await.unwrap().unwrap();
                stream.set_nodelay(true).unwrap();
                ConnectedDirect {
                    stream_sink: Some(Box::pin(tcp_bytes(stream))),
                    source_only: None,
                    sink_only: None,
                }
            }
            Connection::AsClient(ClientConnection::Merge(merge)) => {
                let sources = futures::future::join_all(merge.into_iter().map(|port| async {
                    ConnectedDirect::from_defn(Connection::AsClient(port))
                        .await
                        .into_source()
                }))
                .await;

                let merged = MergeSource {
                    marker: PhantomData,
                    sources,
                };

                ConnectedDirect {
                    stream_sink: None,
                    source_only: Some(Box::pin(merged)),
                    sink_only: None,
                }
            }
            Connection::AsClient(ClientConnection::Demux(_)) => {
                panic!("Cannot connect to a demux pipe directly")
            }

            Connection::AsClient(ClientConnection::Tagged(_, _)) => {
                panic!("Cannot connect to a tagged pipe directly")
            }

            Connection::AsClient(ClientConnection::Null) => ConnectedDirect {
                stream_sink: None,
                source_only: Some(Box::pin(stream::empty())),
                sink_only: Some(Box::pin(IoErrorDrain {
                    marker: PhantomData,
                })),
            },

            Connection::AsServer(bound) => accept(bound).await,
        }
    }
}

impl ConnectedSource for ConnectedDirect {
    type Output = BytesMut;
    type Stream = DynStream;

    fn into_source(mut self) -> DynStream {
        if let Some(s) = self.stream_sink.take() {
            Box::pin(s)
        } else {
            self.source_only.take().unwrap()
        }
    }
}

impl ConnectedSink for ConnectedDirect {
    type Input = Bytes;
    type Sink = DynSink<Bytes>;

    fn into_sink(mut self) -> DynSink<Self::Input> {
        if let Some(s) = self.stream_sink.take() {
            Box::pin(s)
        } else {
            self.sink_only.take().unwrap()
        }
    }
}

pub type BufferedDrain<S, I> = DemuxDrain<I, Buffer<S, I>>;

pub struct ConnectedDemux<T: ConnectedSink>
where
    <T as ConnectedSink>::Input: Sync,
{
    pub keys: Vec<u32>,
    sink: Option<BufferedDrain<T::Sink, T::Input>>,
}

#[pin_project]
pub struct DemuxDrain<T, S: Sink<T, Error = io::Error> + Send + Sync + ?Sized> {
    marker: PhantomData<T>,
    #[pin]
    sinks: HashMap<u32, Pin<Box<S>>>,
}

impl<T, S: Sink<T, Error = io::Error> + Send + Sync> Sink<(u32, T)> for DemuxDrain<T, S> {
    type Error = io::Error;

    fn poll_ready(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        for sink in self.project().sinks.values_mut() {
            ready!(Sink::poll_ready(sink.as_mut(), _cx))?;
        }

        Poll::Ready(Ok(()))
    }

    fn start_send(self: Pin<&mut Self>, item: (u32, T)) -> Result<(), Self::Error> {
        Sink::start_send(
            self.project()
                .sinks
                .get_mut()
                .get_mut(&item.0)
                .unwrap_or_else(|| panic!("No sink in this demux for key {}", item.0))
                .as_mut(),
            item.1,
        )
    }

    fn poll_flush(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        for sink in self.project().sinks.values_mut() {
            ready!(Sink::poll_flush(sink.as_mut(), _cx))?;
        }

        Poll::Ready(Ok(()))
    }

    fn poll_close(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        for sink in self.project().sinks.values_mut() {
            ready!(Sink::poll_close(sink.as_mut(), _cx))?;
        }

        Poll::Ready(Ok(()))
    }
}

#[async_trait]
impl<T: Connected + ConnectedSink> Connected for ConnectedDemux<T>
where
    <T as ConnectedSink>::Input: 'static + Sync,
{
    async fn from_defn(pipe: Connection) -> Self {
        match pipe {
            Connection::AsClient(ClientConnection::Demux(demux)) => {
                let mut connected_demux = HashMap::new();
                let keys = demux.keys().cloned().collect();
                for (id, pipe) in demux {
                    connected_demux.insert(
                        id,
                        Box::pin(
                            T::from_defn(Connection::AsClient(pipe))
                                .await
                                .into_sink()
                                .buffer(1024),
                        ),
                    );
                }

                let demuxer = DemuxDrain {
                    marker: PhantomData,
                    sinks: connected_demux,
                };

                ConnectedDemux {
                    keys,
                    sink: Some(demuxer),
                }
            }

            Connection::AsServer(BoundServer::Demux(demux)) => {
                let mut connected_demux = HashMap::new();
                let keys = demux.keys().cloned().collect();
                for (id, bound) in demux {
                    connected_demux.insert(
                        id,
                        Box::pin(
                            T::from_defn(Connection::AsServer(bound))
                                .await
                                .into_sink()
                                .buffer(1024),
                        ),
                    );
                }

                let demuxer = DemuxDrain {
                    marker: PhantomData,
                    sinks: connected_demux,
                };

                ConnectedDemux {
                    keys,
                    sink: Some(demuxer),
                }
            }
            _ => panic!("Cannot connect to a non-demux pipe as a demux"),
        }
    }
}

impl<T: ConnectedSink> ConnectedSink for ConnectedDemux<T>
where
    <T as ConnectedSink>::Input: 'static + Sync,
{
    type Input = (u32, T::Input);
    type Sink = BufferedDrain<T::Sink, T::Input>;

    fn into_sink(mut self) -> Self::Sink {
        self.sink.take().unwrap()
    }
}

pub struct MergeSource<T: Unpin, S: Stream<Item = T> + Send + Sync + ?Sized> {
    marker: PhantomData<T>,
    sources: Vec<Pin<Box<S>>>,
}

impl<T: Unpin, S: Stream<Item = T> + Send + Sync + ?Sized> Stream for MergeSource<T, S> {
    type Item = T;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let sources = &mut self.get_mut().sources;
        let mut next = None;

        let mut i = 0;
        while i < sources.len() {
            match sources[i].as_mut().poll_next(cx) {
                Poll::Ready(Some(v)) => {
                    next = Some(v);
                    break;
                }
                Poll::Ready(None) => {
                    // this happens infrequently, so OK to be O(n)
                    sources.remove(i);
                }
                Poll::Pending => {
                    i += 1;
                }
            }
        }

        if sources.is_empty() {
            Poll::Ready(None)
        } else if next.is_none() {
            Poll::Pending
        } else {
            Poll::Ready(next)
        }
    }
}

pub struct TaggedSource<T: Unpin, S: Stream<Item = Result<T, io::Error>> + Send + Sync + ?Sized> {
    marker: PhantomData<T>,
    id: u32,
    source: Pin<Box<S>>,
}

impl<T: Unpin, S: Stream<Item = Result<T, io::Error>> + Send + Sync + ?Sized> Stream
    for TaggedSource<T, S>
{
    type Item = Result<(u32, T), io::Error>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let id = self.as_ref().id;
        let source = &mut self.get_mut().source;
        match source.as_mut().poll_next(cx) {
            Poll::Ready(Some(v)) => Poll::Ready(Some(v.map(|d| (id, d)))),
            Poll::Ready(None) => Poll::Ready(None),
            Poll::Pending => Poll::Pending,
        }
    }
}

type MergedMux<T> = MergeSource<
    Result<(u32, <T as ConnectedSource>::Output), io::Error>,
    TaggedSource<<T as ConnectedSource>::Output, <T as ConnectedSource>::Stream>,
>;

pub struct ConnectedTagged<T: ConnectedSource>
where
    <T as ConnectedSource>::Output: 'static + Sync + Unpin,
{
    source: MergedMux<T>,
}

#[async_trait]
impl<T: Connected + ConnectedSource> Connected for ConnectedTagged<T>
where
    <T as ConnectedSource>::Output: 'static + Sync + Unpin,
{
    async fn from_defn(pipe: Connection) -> Self {
        let sources = match pipe {
            Connection::AsClient(ClientConnection::Tagged(pipe, id)) => {
                vec![(
                    Box::pin(
                        T::from_defn(Connection::AsClient(*pipe))
                            .await
                            .into_source(),
                    ),
                    id,
                )]
            }

            Connection::AsClient(ClientConnection::Merge(m)) => {
                let mut sources = Vec::new();
                for port in m {
                    if let ClientConnection::Tagged(pipe, id) = port {
                        sources.push((
                            Box::pin(
                                T::from_defn(Connection::AsClient(*pipe))
                                    .await
                                    .into_source(),
                            ),
                            id,
                        ));
                    } else {
                        panic!("Merge port must be tagged");
                    }
                }

                sources
            }

            Connection::AsServer(BoundServer::Tagged(pipe, id)) => {
                vec![(
                    Box::pin(
                        T::from_defn(Connection::AsServer(*pipe))
                            .await
                            .into_source(),
                    ),
                    id,
                )]
            }

            Connection::AsServer(BoundServer::Merge(m)) => {
                let mut sources = Vec::new();
                for port in m {
                    if let BoundServer::Tagged(pipe, id) = port {
                        sources.push((
                            Box::pin(
                                T::from_defn(Connection::AsServer(*pipe))
                                    .await
                                    .into_source(),
                            ),
                            id,
                        ));
                    } else {
                        panic!("Merge port must be tagged");
                    }
                }

                sources
            }

            _ => panic!("Cannot connect to a non-tagged pipe as a tagged"),
        };

        let mut connected_mux = Vec::new();
        for (pipe, id) in sources {
            connected_mux.push(Box::pin(TaggedSource {
                marker: PhantomData,
                id,
                source: pipe,
            }));
        }

        let muxer = MergeSource {
            marker: PhantomData,
            sources: connected_mux,
        };

        ConnectedTagged { source: muxer }
    }
}

impl<T: ConnectedSource> ConnectedSource for ConnectedTagged<T>
where
    <T as ConnectedSource>::Output: 'static + Sync + Unpin,
{
    type Output = (u32, T::Output);
    type Stream = MergeSource<Result<Self::Output, io::Error>, TaggedSource<T::Output, T::Stream>>;

    fn into_source(self) -> Self::Stream {
        self.source
    }
}
