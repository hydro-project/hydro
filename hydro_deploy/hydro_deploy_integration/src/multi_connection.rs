use std::collections::HashMap;
use std::io;
use std::ops::DerefMut;
use std::pin::Pin;
use std::task::{Context, Poll, ready};

use bytes::{Bytes, BytesMut};
use futures::{Sink, SinkExt, Stream, StreamExt};
use tokio::net::TcpListener;
#[cfg(unix)]
use tokio::net::UnixListener;
use tokio::sync::mpsc;

use crate::{AcceptedServer, BoundServer, Connected, ConnectedSourceSink, Connection, tcp_bytes};

pub struct ConnectedMultiConnection {
    source: Pin<Box<dyn Stream<Item = Result<(u64, BytesMut), io::Error>> + Send + Sync>>,
    sink: Pin<Box<dyn Sink<(u64, Bytes), Error = io::Error> + Send + Sync>>,
}

impl Connected for ConnectedMultiConnection {
    fn from_defn(pipe: Connection) -> Self {
        match pipe {
            Connection::AsServer(AcceptedServer::MultiConnection(bound_server)) => {
                let (source, sink) = MultiConnectionStream::new(*bound_server);
                ConnectedMultiConnection {
                    source: Box::pin(source),
                    sink: Box::pin(sink),
                }
            }
            _ => panic!("Cannot connect to a non-multi-connection pipe as a multi-connection"),
        }
    }
}

impl ConnectedSourceSink for ConnectedMultiConnection {
    type Output = (u64, BytesMut);
    type Stream = Pin<Box<dyn Stream<Item = Result<Self::Output, io::Error>> + Send + Sync>>;

    type Input = (u64, Bytes);
    type Sink = Pin<Box<dyn Sink<Self::Input, Error = io::Error> + Send + Sync>>;

    fn into_source_sink(self) -> (Self::Stream, Self::Sink) {
        (self.source, self.sink)
    }
}

type NewSinkSender = mpsc::UnboundedSender<(
    u64,
    Pin<Box<dyn Sink<Bytes, Error = io::Error> + Send + Sync>>,
)>;
type NewSinkReceiver = mpsc::UnboundedReceiver<(
    u64,
    Pin<Box<dyn Sink<Bytes, Error = io::Error> + Send + Sync>>,
)>;
type ConnectionMap = HashMap<u64, Pin<Box<dyn Sink<Bytes, Error = io::Error> + Send + Sync>>>;

struct MultiConnectionSource {
    #[cfg(unix)]
    unix_listener: Option<UnixListener>,
    tcp_listener: Option<TcpListener>,
    next_connection_id: u64,
    active_connections:
        HashMap<u64, Pin<Box<dyn Stream<Item = Result<BytesMut, io::Error>> + Send + Sync>>>,
    new_sink_sender: NewSinkSender,
}

struct MultiConnectionSink {
    connection_sinks: ConnectionMap,
    new_sink_receiver: NewSinkReceiver,
}

struct MultiConnectionStream;

impl MultiConnectionStream {
    fn new(bound_server: BoundServer) -> (MultiConnectionSource, MultiConnectionSink) {
        let (new_sink_sender, new_sink_receiver) = mpsc::unbounded_channel();

        let source = match bound_server {
            #[cfg(unix)]
            BoundServer::UnixSocket(listener, _) => MultiConnectionSource {
                unix_listener: Some(listener),
                tcp_listener: None,
                next_connection_id: 0,
                active_connections: HashMap::new(),
                new_sink_sender,
            },
            BoundServer::TcpPort(listener, _) => MultiConnectionSource {
                #[cfg(unix)]
                unix_listener: None,
                tcp_listener: Some(listener.into_inner()),
                next_connection_id: 0,
                active_connections: HashMap::new(),
                new_sink_sender,
            },
            _ => panic!("MultiConnection only supports UnixSocket and TcpPort"),
        };

        let sink = MultiConnectionSink {
            connection_sinks: HashMap::new(),
            new_sink_receiver,
        };

        (source, sink)
    }
}

impl Stream for MultiConnectionSource {
    type Item = Result<(u64, BytesMut), io::Error>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let me = self.deref_mut();
        // Handle Unix socket accepts
        #[cfg(unix)]
        if let Some(listener) = me.unix_listener.as_mut() {
            loop {
                match listener.poll_accept(cx) {
                    Poll::Ready(Ok((stream, _))) => {
                        use futures::{SinkExt, StreamExt};

                        use crate::unix_bytes;

                        let connection_id = me.next_connection_id;
                        me.next_connection_id += 1;

                        let framed = unix_bytes(stream);
                        let (sink, stream) = framed.split();

                        let boxed_stream: Pin<
                            Box<dyn Stream<Item = Result<BytesMut, io::Error>> + Send + Sync>,
                        > = Box::pin(stream);

                        // Buffer so that a stalled output does not prevent sending to others
                        let boxed_sink: Pin<Box<dyn Sink<Bytes, Error = io::Error> + Send + Sync>> =
                            Box::pin(sink.buffer(1024));

                        me.active_connections.insert(connection_id, boxed_stream);

                        let _ = me.new_sink_sender.send((connection_id, boxed_sink));
                    }
                    Poll::Ready(Err(e)) => {
                        if me.active_connections.is_empty() {
                            return Poll::Ready(Some(Err(e)));
                        }
                    }
                    Poll::Pending => {
                        break;
                    }
                }
            }
        }

        // Handle TCP socket accepts
        if let Some(listener) = me.tcp_listener.as_mut() {
            loop {
                match listener.poll_accept(cx) {
                    Poll::Ready(Ok((stream, _))) => {
                        let connection_id = me.next_connection_id;
                        me.next_connection_id += 1;

                        let framed = tcp_bytes(stream);
                        let (sink, stream) = framed.split();

                        let boxed_stream: Pin<
                            Box<dyn Stream<Item = Result<BytesMut, io::Error>> + Send + Sync>,
                        > = Box::pin(stream);
                        let boxed_sink: Pin<Box<dyn Sink<Bytes, Error = io::Error> + Send + Sync>> =
                            Box::pin(sink.buffer(1024));

                        me.active_connections.insert(connection_id, boxed_stream);

                        let _ = me.new_sink_sender.send((connection_id, boxed_sink));
                    }
                    Poll::Ready(Err(e)) => {
                        if me.active_connections.is_empty() {
                            return Poll::Ready(Some(Err(e)));
                        }
                    }
                    Poll::Pending => {
                        break;
                    }
                }
            }
        }

        // Poll all active connections for data
        let mut connections_to_remove = Vec::new();
        let mut out = Poll::Pending;
        for (&connection_id, stream) in self.active_connections.iter_mut() {
            match stream.as_mut().poll_next(cx) {
                Poll::Ready(Some(Ok(data))) => {
                    out = Poll::Ready(Some(Ok((connection_id, data))));
                    break;
                }
                Poll::Ready(Some(Err(_))) | Poll::Ready(None) => {
                    connections_to_remove.push(connection_id);
                }
                Poll::Pending => {}
            }
        }

        for connection_id in connections_to_remove {
            self.active_connections.remove(&connection_id);
        }

        out
    }
}

impl Sink<(u64, Bytes)> for MultiConnectionSink {
    type Error = io::Error;

    fn poll_ready(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        // Poll for new sinks
        loop {
            match self.new_sink_receiver.poll_recv(cx) {
                Poll::Ready(Some((connection_id, sink))) => {
                    self.connection_sinks.insert(connection_id, sink);
                }
                Poll::Ready(None) => {
                    if self.connection_sinks.is_empty() {
                        return Poll::Ready(Err(io::Error::new(
                            io::ErrorKind::BrokenPipe,
                            "No additional sinks are available (was the stream dropped)?",
                        )));
                    } else {
                        break;
                    }
                }
                Poll::Pending => {}
            }
        }

        // Check if all sinks are ready, removing any that are closed
        let mut closed_connections = Vec::new();
        for (&connection_id, sink) in self.connection_sinks.iter_mut() {
            match ready!(sink.as_mut().poll_ready(cx)) {
                Ok(()) => {}
                Err(_) => {
                    closed_connections.push(connection_id);
                }
            }
        }

        for connection_id in closed_connections {
            self.connection_sinks.remove(&connection_id);
        }

        Poll::Ready(Ok(())) // always ready, because we drop messages if there is no sink
    }

    fn start_send(mut self: Pin<&mut Self>, item: (u64, Bytes)) -> Result<(), Self::Error> {
        if let Some(sink) = self.connection_sinks.get_mut(&item.0) {
            let _ = sink.as_mut().start_send(item.1); // silently ignore send errors
        }
        // If connection doesn't exist, silently drop (connection may have closed)
        Ok(())
    }

    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        let mut closed_connections = Vec::new();
        let mut any_pending = false;

        for (&connection_id, sink) in self.connection_sinks.iter_mut() {
            match sink.as_mut().poll_flush(cx) {
                Poll::Ready(Ok(())) => {}
                Poll::Ready(Err(_)) => {
                    closed_connections.push(connection_id);
                }
                Poll::Pending => {
                    any_pending = true;
                }
            }
        }

        for connection_id in closed_connections {
            self.connection_sinks.remove(&connection_id);
        }

        if any_pending {
            Poll::Pending
        } else {
            Poll::Ready(Ok(()))
        }
    }

    fn poll_close(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        let mut closed_connections = Vec::new();
        let mut any_pending = false;

        for (&connection_id, sink) in self.connection_sinks.iter_mut() {
            match sink.as_mut().poll_close(cx) {
                Poll::Ready(Ok(())) => {
                    closed_connections.push(connection_id);
                }
                Poll::Ready(Err(_)) => {
                    closed_connections.push(connection_id);
                }
                Poll::Pending => {
                    any_pending = true;
                }
            }
        }

        for connection_id in closed_connections {
            self.connection_sinks.remove(&connection_id);
        }

        if any_pending {
            Poll::Pending
        } else {
            Poll::Ready(Ok(()))
        }
    }
}
