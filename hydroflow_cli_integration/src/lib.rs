#![feature(never_type)]

use std::collections::HashMap;
use std::marker::PhantomData;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::pin::Pin;
use std::task::{Context, Poll};
use std::time::Duration;

use bytes::{Bytes, BytesMut};
use serde::{Deserialize, Serialize};

use futures::sink::Buffer;
use futures::{ready, stream, Future, Sink, SinkExt, Stream};

use async_recursion::async_recursion;
use async_trait::async_trait;
use pin_project::pin_project;

use tokio::io;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::mpsc::UnboundedReceiver;
use tokio::task::JoinHandle;
use tokio_util::codec::{Framed, LengthDelimitedCodec};

#[cfg(unix)]
use tokio::net::{UnixListener, UnixStream};

#[cfg(not(unix))]
#[allow(dead_code)]
type UnixStream = !;

#[cfg(not(unix))]
#[allow(dead_code)]
type UnixListener = !;

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
    pub async fn bind(self) -> BoundConnection {
        match self {
            ServerBindConfig::UnixSocket => {
                #[cfg(unix)]
                {
                    let dir = tempfile::tempdir().unwrap();
                    let socket_path = dir.path().join("socket");
                    let bound = UnixListener::bind(socket_path).unwrap();
                    BoundConnection::UnixSocket(
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
                let (conn_send, conn_recv) = tokio::sync::mpsc::unbounded_channel();

                tokio::spawn(async move {
                    loop {
                        if conn_send
                            .send(listener.accept().await.map(|r| r.0))
                            .is_err()
                        {
                            break;
                        }
                    }
                });

                BoundConnection::TcpPort(conn_recv, addr)
            }
            ServerBindConfig::Demux(bindings) => {
                let mut demux = HashMap::new();
                for (key, bind) in bindings {
                    demux.insert(key, bind.bind().await);
                }
                BoundConnection::Demux(demux)
            }
            ServerBindConfig::Merge(bindings) => {
                let mut merge = Vec::new();
                for bind in bindings {
                    merge.push(bind.bind().await);
                }
                BoundConnection::Merge(merge)
            }
            ServerBindConfig::Tagged(underlying, id) => {
                BoundConnection::Tagged(Box::new(underlying.bind().await), id)
            }
            ServerBindConfig::Null => BoundConnection::Null,
        }
    }
}

#[derive(Debug)]
pub enum ServerOrBound {
    Server(ServerPort),
    Bound(BoundConnection),
}

impl ServerOrBound {
    pub async fn connect<T: Connected>(self) -> T {
        T::from_defn(self).await
    }

    pub async fn accept_tcp(&mut self) -> TcpStream {
        if let ServerOrBound::Bound(BoundConnection::TcpPort(handle, _)) = self {
            handle.recv().await.unwrap().unwrap()
        } else {
            panic!("Not a TCP port")
        }
    }
}

/// Describes how to connect to a service which is listening on some port.
#[allow(unreachable_code)]
#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum ServerPort {
    UnixSocket(PathBuf),
    TcpPort(SocketAddr),
    Demux(HashMap<u32, ServerPort>),
    Merge(Vec<ServerPort>),
    Tagged(Box<ServerPort>, u32),
    Null,
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
    async fn from_defn(pipe: ServerOrBound) -> Self;
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
pub enum BoundConnection {
    UnixSocket(JoinHandle<io::Result<UnixStream>>, tempfile::TempDir),
    TcpPort(UnboundedReceiver<io::Result<TcpStream>>, SocketAddr),
    Demux(HashMap<u32, BoundConnection>),
    Merge(Vec<BoundConnection>),
    Tagged(Box<BoundConnection>, u32),
    Null,
}

impl BoundConnection {
    pub fn sink_port(&self) -> ServerPort {
        match self {
            BoundConnection::UnixSocket(_, tempdir) => {
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
            BoundConnection::TcpPort(_, addr) => {
                ServerPort::TcpPort(SocketAddr::new(addr.ip(), addr.port()))
            }

            BoundConnection::Demux(bindings) => {
                let mut demux = HashMap::new();
                for (key, bind) in bindings {
                    demux.insert(*key, bind.sink_port());
                }
                ServerPort::Demux(demux)
            }

            BoundConnection::Merge(bindings) => {
                let mut merge = Vec::new();
                for bind in bindings {
                    merge.push(bind.sink_port());
                }
                ServerPort::Merge(merge)
            }

            BoundConnection::Tagged(underlying, id) => {
                ServerPort::Tagged(Box::new(underlying.sink_port()), *id)
            }

            BoundConnection::Null => ServerPort::Null,
        }
    }
}

#[async_recursion]
async fn accept(bound: BoundConnection) -> ConnectedBidi {
    match bound {
        BoundConnection::UnixSocket(listener, _) => {
            #[cfg(unix)]
            {
                let stream = listener.await.unwrap().unwrap();
                ConnectedBidi {
                    stream_sink: Some(Box::pin(unix_bytes(stream))),
                    source_only: None,
                    sink_only: None,
                }
            }

            #[cfg(not(unix))]
            {
                std::mem::drop(listener);
                panic!("Unix sockets are not supported on this platform")
            }
        }
        BoundConnection::TcpPort(mut listener, _) => {
            let stream = listener.recv().await.unwrap().unwrap();
            ConnectedBidi {
                stream_sink: Some(Box::pin(tcp_bytes(stream))),
                source_only: None,
                sink_only: None,
            }
        }
        BoundConnection::Merge(merge) => {
            let mut sources = vec![];
            for bound in merge {
                sources.push(accept(bound).await.into_source());
            }

            let merge_source: DynStream = Box::pin(MergeSource {
                marker: PhantomData,
                sources,
            });

            ConnectedBidi {
                stream_sink: None,
                source_only: Some(merge_source),
                sink_only: None,
            }
        }
        BoundConnection::Demux(_) => panic!("Cannot connect to a demux pipe directly"),
        BoundConnection::Tagged(_, _) => panic!("Cannot connect to a tagged pipe directly"),
        BoundConnection::Null => {
            ConnectedBidi::from_defn(ServerOrBound::Server(ServerPort::Null)).await
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

pub struct ConnectedBidi {
    stream_sink: Option<DynStreamSink>,
    source_only: Option<DynStream>,
    sink_only: Option<DynSink<Bytes>>,
}

#[async_trait]
impl Connected for ConnectedBidi {
    async fn from_defn(pipe: ServerOrBound) -> Self {
        match pipe {
            ServerOrBound::Server(ServerPort::UnixSocket(path)) => {
                #[cfg(unix)]
                {
                    let stream = UnixStream::connect(path).await.unwrap();
                    ConnectedBidi {
                        stream_sink: Some(Box::pin(unix_bytes(stream))),
                        source_only: None,
                        sink_only: None,
                    }
                }

                #[cfg(not(unix))]
                {
                    let _ = path;
                    panic!("Unix sockets are not supported on this platform");
                }
            }
            ServerOrBound::Server(ServerPort::TcpPort(addr)) => {
                let stream = async_retry(|| TcpStream::connect(addr), 10, Duration::from_secs(1))
                    .await
                    .unwrap();
                stream.set_nodelay(true).unwrap();
                ConnectedBidi {
                    stream_sink: Some(Box::pin(tcp_bytes(stream))),
                    source_only: None,
                    sink_only: None,
                }
            }
            ServerOrBound::Server(ServerPort::Merge(merge)) => {
                let sources = futures::future::join_all(merge.iter().map(|port| async {
                    ConnectedBidi::from_defn(ServerOrBound::Server(port.clone()))
                        .await
                        .into_source()
                }))
                .await;

                let merged = MergeSource {
                    marker: PhantomData,
                    sources,
                };

                ConnectedBidi {
                    stream_sink: None,
                    source_only: Some(Box::pin(merged)),
                    sink_only: None,
                }
            }
            ServerOrBound::Server(ServerPort::Demux(_)) => {
                panic!("Cannot connect to a demux pipe directly")
            }

            ServerOrBound::Server(ServerPort::Tagged(_, _)) => {
                panic!("Cannot connect to a tagged pipe directly")
            }

            ServerOrBound::Server(ServerPort::Null) => ConnectedBidi {
                stream_sink: None,
                source_only: Some(Box::pin(stream::empty())),
                sink_only: Some(Box::pin(IoErrorDrain {
                    marker: PhantomData,
                })),
            },

            ServerOrBound::Bound(bound) => accept(bound).await,
        }
    }
}

impl ConnectedSource for ConnectedBidi {
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

impl ConnectedSink for ConnectedBidi {
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
    async fn from_defn(pipe: ServerOrBound) -> Self {
        match pipe {
            ServerOrBound::Server(ServerPort::Demux(demux)) => {
                let mut connected_demux = HashMap::new();
                let keys = demux.keys().cloned().collect();
                for (id, pipe) in demux {
                    connected_demux.insert(
                        id,
                        Box::pin(
                            T::from_defn(ServerOrBound::Server(pipe))
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

            ServerOrBound::Bound(BoundConnection::Demux(demux)) => {
                let mut connected_demux = HashMap::new();
                let keys = demux.keys().cloned().collect();
                for (id, bound) in demux {
                    connected_demux.insert(
                        id,
                        Box::pin(
                            T::from_defn(ServerOrBound::Bound(bound))
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
    async fn from_defn(pipe: ServerOrBound) -> Self {
        let sources = match pipe {
            ServerOrBound::Server(ServerPort::Tagged(pipe, id)) => {
                vec![(
                    Box::pin(
                        T::from_defn(ServerOrBound::Server(*pipe))
                            .await
                            .into_source(),
                    ),
                    id,
                )]
            }

            ServerOrBound::Server(ServerPort::Merge(m)) => {
                let mut sources = Vec::new();
                for port in m {
                    if let ServerPort::Tagged(pipe, id) = port {
                        sources.push((
                            Box::pin(
                                T::from_defn(ServerOrBound::Server(*pipe))
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

            ServerOrBound::Bound(BoundConnection::Tagged(pipe, id)) => {
                vec![(
                    Box::pin(
                        T::from_defn(ServerOrBound::Bound(*pipe))
                            .await
                            .into_source(),
                    ),
                    id,
                )]
            }

            ServerOrBound::Bound(BoundConnection::Merge(m)) => {
                let mut sources = Vec::new();
                for port in m {
                    if let BoundConnection::Tagged(pipe, id) = port {
                        sources.push((
                            Box::pin(
                                T::from_defn(ServerOrBound::Bound(*pipe))
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
