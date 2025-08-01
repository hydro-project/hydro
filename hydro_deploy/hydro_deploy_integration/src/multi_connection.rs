use std::collections::HashMap;
use std::io;
use std::marker::PhantomData;
use std::ops::DerefMut;
use std::pin::Pin;
use std::task::{Context, Poll, ready};

use futures::{Sink, SinkExt, Stream, StreamExt};
#[cfg(unix)]
use tempfile::TempDir;
use tokio::net::TcpListener;
#[cfg(unix)]
use tokio::net::UnixListener;
use tokio::sync::mpsc;
use tokio_util::codec::{Decoder, Encoder, Framed};

use crate::{AcceptedServer, BoundServer, Connected, ConnectedSourceSink, Connection};

pub struct ConnectedMultiConnection<I, O, C: Decoder<Item = I> + Encoder<O>> {
    source: MultiConnectionSource<I, O, C>,
    sink: MultiConnectionSink<O, C>,
}

impl<
    I: 'static,
    O: Send + Sync + 'static,
    C: Decoder<Item = I> + Encoder<O> + Send + Sync + Default + 'static,
> Connected for ConnectedMultiConnection<I, O, C>
{
    fn from_defn(pipe: Connection) -> Self {
        match pipe {
            Connection::AsServer(AcceptedServer::MultiConnection(bound_server)) => {
                let (new_sink_sender, new_sink_receiver) = mpsc::unbounded_channel();

                let source = match *bound_server {
                    #[cfg(unix)]
                    BoundServer::UnixSocket(listener, dir) => MultiConnectionSource {
                        unix_listener: Some(listener),
                        tcp_listener: None,
                        _dir_holder: Some(dir),
                        next_connection_id: 0,
                        active_connections: HashMap::new(),
                        new_sink_sender,
                        _phantom: Default::default(),
                    },
                    BoundServer::TcpPort(listener, _) => MultiConnectionSource {
                        #[cfg(unix)]
                        unix_listener: None,
                        tcp_listener: Some(listener.into_inner()),
                        #[cfg(unix)]
                        _dir_holder: None,
                        next_connection_id: 0,
                        active_connections: HashMap::new(),
                        new_sink_sender,
                        _phantom: Default::default(),
                    },
                    _ => panic!("MultiConnection only supports UnixSocket and TcpPort"),
                };

                let sink = MultiConnectionSink::<O, C> {
                    connection_sinks: HashMap::new(),
                    new_sink_receiver,
                    _phantom: Default::default(),
                };

                ConnectedMultiConnection { source, sink }
            }
            _ => panic!("Cannot connect to a non-multi-connection pipe as a multi-connection"),
        }
    }
}

impl<
    I: Send,
    O: Send + Sync + 'static,
    C: Decoder<Item = I> + Encoder<O> + Send + Sync + Default + 'static,
> ConnectedSourceSink for ConnectedMultiConnection<I, O, C>
{
    type Output = (u64, I);
    type OutError = <C as Decoder>::Error;
    type Stream = MultiConnectionSource<I, O, C>;

    type Input = (u64, O);
    type InError = <C as Encoder<O>>::Error;
    type Sink = MultiConnectionSink<O, C>;

    fn into_source_sink(self) -> (Self::Stream, Self::Sink) {
        (self.source, self.sink)
    }
}

type DynDecodedStream<I, C> =
    Pin<Box<dyn Stream<Item = Result<I, <C as Decoder>::Error>> + Send + Sync>>;
type DynEncodedSink<O, C> = Pin<Box<dyn Sink<O, Error = <C as Encoder<O>>::Error> + Send + Sync>>;

pub struct MultiConnectionSource<I, O, C: Decoder<Item = I> + Encoder<O>> {
    #[cfg(unix)]
    unix_listener: Option<UnixListener>,
    tcp_listener: Option<TcpListener>,
    #[cfg(unix)]
    _dir_holder: Option<TempDir>, // keeps the folder containing the socket alive
    next_connection_id: u64,
    active_connections: HashMap<u64, DynDecodedStream<I, C>>,
    new_sink_sender: mpsc::UnboundedSender<(u64, DynEncodedSink<O, C>)>,
    _phantom: PhantomData<(Box<O>, Box<C>)>,
}

pub struct MultiConnectionSink<O, C: Encoder<O>> {
    connection_sinks: HashMap<u64, DynEncodedSink<O, C>>,
    new_sink_receiver: mpsc::UnboundedReceiver<(u64, DynEncodedSink<O, C>)>,
    _phantom: PhantomData<(Box<O>, Box<C>)>,
}

impl<
    I,
    O: Send + Sync + 'static,
    C: Decoder<Item = I> + Encoder<O> + Send + Sync + Default + 'static,
> Stream for MultiConnectionSource<I, O, C>
{
    type Item = Result<(u64, I), <C as Decoder>::Error>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let me = self.deref_mut();
        // Handle Unix socket accepts
        #[cfg(unix)]
        if let Some(listener) = me.unix_listener.as_mut() {
            loop {
                match listener.poll_accept(cx) {
                    Poll::Ready(Ok((stream, _))) => {
                        use futures::{SinkExt, StreamExt};
                        use tokio_util::codec::Framed;

                        let connection_id = me.next_connection_id;
                        me.next_connection_id += 1;

                        let framed = Framed::new(stream, C::default());
                        let (sink, stream) = framed.split();

                        let boxed_stream: Pin<
                            Box<dyn Stream<Item = Result<I, <C as Decoder>::Error>> + Send + Sync>,
                        > = Box::pin(stream);

                        // Buffer so that a stalled output does not prevent sending to others
                        let boxed_sink: Pin<
                            Box<dyn Sink<O, Error = <C as Encoder<O>>::Error> + Send + Sync>,
                        > = Box::pin(sink.buffer(1024));

                        me.active_connections.insert(connection_id, boxed_stream);

                        let _ = me.new_sink_sender.send((connection_id, boxed_sink));
                    }
                    Poll::Ready(Err(e)) => {
                        if me.active_connections.is_empty() {
                            return Poll::Ready(Some(Err(e.into())));
                        } else {
                            break;
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

                        let framed = Framed::new(stream, C::default());
                        let (sink, stream) = framed.split();

                        let boxed_stream: Pin<
                            Box<dyn Stream<Item = Result<I, <C as Decoder>::Error>> + Send + Sync>,
                        > = Box::pin(stream);

                        // Buffer so that a stalled output does not prevent sending to others
                        let boxed_sink: Pin<
                            Box<dyn Sink<O, Error = <C as Encoder<O>>::Error> + Send + Sync>,
                        > = Box::pin(sink.buffer(1024));

                        me.active_connections.insert(connection_id, boxed_stream);

                        let _ = me.new_sink_sender.send((connection_id, boxed_sink));
                    }
                    Poll::Ready(Err(e)) => {
                        if me.active_connections.is_empty() {
                            return Poll::Ready(Some(Err(e.into())));
                        } else {
                            break;
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
                    break; // TODO(shadaj): this is unfair
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

impl<O, C: Encoder<O>> Sink<(u64, O)> for MultiConnectionSink<O, C> {
    type Error = <C as Encoder<O>>::Error;

    fn poll_ready(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
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
                        )
                        .into()));
                    } else {
                        break;
                    }
                }
                Poll::Pending => {
                    break;
                }
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

    fn start_send(mut self: Pin<&mut Self>, item: (u64, O)) -> Result<(), Self::Error> {
        if let Some(sink) = self.connection_sinks.get_mut(&item.0) {
            let _ = sink.as_mut().start_send(item.1); // TODO(shadaj): log errors when we have principled logging
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
