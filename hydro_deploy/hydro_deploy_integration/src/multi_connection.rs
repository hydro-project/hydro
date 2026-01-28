use std::collections::HashMap;
use std::io;
use std::ops::DerefMut;
use std::pin::Pin;
use std::task::{Context, Poll};

use futures::{Sink, SinkExt, Stream, StreamExt};
#[cfg(unix)]
use tempfile::TempDir;
use tokio::net::TcpListener;
#[cfg(unix)]
use tokio::net::UnixListener;
use tokio::net::tcp::{OwnedReadHalf, OwnedWriteHalf};
use tokio::sync::mpsc;
use tokio_stream::wrappers::UnboundedReceiverStream;
use tokio_util::codec::{Decoder, Encoder, Framed, FramedRead, FramedWrite};

use crate::{AcceptedServer, BoundServer, Connected, Connection};

pub struct ConnectedMultiConnection<I, O, C: Decoder<Item = I> + Encoder<O>> {
    pub source: MultiConnectionSource<I, O, C>,
    pub sink: MultiConnectionSink<O, C>,
    pub membership: UnboundedReceiverStream<(u64, bool)>,
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
                let (membership_sender, membership_receiver) = mpsc::unbounded_channel();

                let source = match *bound_server {
                    #[cfg(unix)]
                    BoundServer::UnixSocket(listener, dir) => MultiConnectionSource {
                        unix_listener: Some(listener),
                        tcp_listener: None,
                        _dir_holder: Some(dir),
                        next_connection_id: 0,
                        active_connections: Vec::new(),
                        poll_cursor: 0,
                        new_sink_sender,
                        membership_sender,
                    },
                    BoundServer::TcpPort(listener, _) => MultiConnectionSource {
                        #[cfg(unix)]
                        unix_listener: None,
                        tcp_listener: Some(listener.into_inner()),
                        #[cfg(unix)]
                        _dir_holder: None,
                        next_connection_id: 0,
                        active_connections: Vec::new(),
                        poll_cursor: 0,
                        new_sink_sender,
                        membership_sender,
                    },
                    _ => panic!("MultiConnection only supports UnixSocket and TcpPort"),
                };

                let sink = MultiConnectionSink::<O, C> {
                    connection_sinks: HashMap::new(),
                    new_sink_receiver,
                };

                ConnectedMultiConnection {
                    source,
                    sink,
                    membership: UnboundedReceiverStream::new(membership_receiver),
                }
            }
            _ => panic!("Cannot connect to a non-multi-connection pipe as a multi-connection"),
        }
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
    /// Ordered list for fair polling, will never be `None` at the beginning of a poll
    active_connections: Vec<Option<(u64, DynDecodedStream<I, C>)>>,
    /// Cursor for fair round-robin polling
    poll_cursor: usize,
    new_sink_sender: mpsc::UnboundedSender<(u64, DynEncodedSink<O, C>)>,
    membership_sender: mpsc::UnboundedSender<(u64, bool)>,
}

pub struct MultiConnectionSink<O, C: Encoder<O>> {
    connection_sinks: HashMap<u64, DynEncodedSink<O, C>>,
    new_sink_receiver: mpsc::UnboundedReceiver<(u64, DynEncodedSink<O, C>)>,
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

                        me.active_connections
                            .push(Some((connection_id, boxed_stream)));

                        let _ = me.new_sink_sender.send((connection_id, boxed_sink));
                        let _ = me.membership_sender.send((connection_id, true));
                    }
                    Poll::Ready(Err(e)) => {
                        if !me.active_connections.iter().any(|conn| conn.is_some()) {
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

                        me.active_connections
                            .push(Some((connection_id, boxed_stream)));

                        let _ = me.new_sink_sender.send((connection_id, boxed_sink));
                        let _ = me.membership_sender.send((connection_id, true));
                    }
                    Poll::Ready(Err(e)) => {
                        if !me.active_connections.iter().any(|conn| conn.is_some()) {
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

        // Poll all active connections for data using fair round-robin cursor
        let mut out = Poll::Pending;
        let mut any_removed = false;

        if !me.active_connections.is_empty() {
            let start_cursor = me.poll_cursor;

            loop {
                let current_length = me.active_connections.len();
                let id_and_stream = &mut me.active_connections[me.poll_cursor];
                let (connection_id, stream) = id_and_stream.as_mut().unwrap();
                let connection_id = *connection_id; // Copy the ID before borrowing stream

                // Move cursor to next source for next poll
                me.poll_cursor = (me.poll_cursor + 1) % current_length;

                match stream.as_mut().poll_next(cx) {
                    Poll::Ready(Some(Ok(data))) => {
                        out = Poll::Ready(Some(Ok((connection_id, data))));
                        break;
                    }
                    Poll::Ready(Some(Err(_))) | Poll::Ready(None) => {
                        let _ = me.membership_sender.send((connection_id, false));
                        *id_and_stream = None; // Mark connection as removed
                        any_removed = true;
                    }
                    Poll::Pending => {}
                }

                // Check if we've completed a full round
                if me.poll_cursor == start_cursor {
                    break;
                }
            }
        }

        // Clean up None entries and adjust cursor
        let mut current_index = 0;
        let original_cursor = me.poll_cursor;

        if any_removed {
            me.active_connections.retain(|conn| {
                if conn.is_none() && current_index < original_cursor {
                    me.poll_cursor -= 1;
                }
                current_index += 1;
                conn.is_some()
            });
        }

        if me.poll_cursor == me.active_connections.len() {
            me.poll_cursor = 0;
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
        let mut any_pending = false;
        self.connection_sinks
            .retain(|_, sink| match sink.as_mut().poll_ready(cx) {
                Poll::Ready(Ok(())) => true,
                Poll::Ready(Err(_)) => false,
                Poll::Pending => {
                    any_pending = true;
                    true
                }
            });

        if any_pending {
            Poll::Pending
        } else {
            Poll::Ready(Ok(())) // always ready, because we drop messages if there is no sink
        }
    }

    fn start_send(mut self: Pin<&mut Self>, item: (u64, O)) -> Result<(), Self::Error> {
        if let Some(sink) = self.connection_sinks.get_mut(&item.0) {
            let _ = sink.as_mut().start_send(item.1); // TODO(shadaj): log errors when we have principled logging
        }
        // If connection doesn't exist, silently drop (connection may have closed)
        Ok(())
    }

    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        let mut any_pending = false;

        self.connection_sinks
            .retain(|_, sink| match sink.as_mut().poll_flush(cx) {
                Poll::Ready(Ok(())) => true,
                Poll::Ready(Err(_)) => false,
                Poll::Pending => {
                    any_pending = true;
                    true
                }
            });

        if any_pending {
            Poll::Pending
        } else {
            Poll::Ready(Ok(()))
        }
    }

    fn poll_close(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        let mut any_pending = false;

        self.connection_sinks.retain(|_, sink| {
            match sink.as_mut().poll_close(cx) {
                Poll::Ready(Ok(()) | Err(_)) => false, // Remove regardless of ok/err
                Poll::Pending => {
                    any_pending = true;
                    true
                }
            }
        });

        if any_pending {
            Poll::Pending
        } else {
            Poll::Ready(Ok(()))
        }
    }
}

/// TCP-only concrete type versions for use in containerized deployments
pub struct TcpMultiConnectionSource<C: Decoder> {
    /// The TCP listener accepting new connections
    pub listener: TcpListener,
    /// Counter for assigning unique connection IDs
    pub next_connection_id: u64,
    /// Active connections with their IDs and framed readers
    pub active_connections: Vec<Option<(u64, FramedRead<OwnedReadHalf, C>)>>,
    /// Cursor for fair round-robin polling
    pub poll_cursor: usize,
    /// Channel to send new sinks to the TcpMultiConnectionSink
    pub new_sink_sender: mpsc::UnboundedSender<(u64, FramedWrite<OwnedWriteHalf, C>)>,
    /// Channel to send membership events
    pub membership_sender: mpsc::UnboundedSender<(u64, bool)>,
}

impl<C: Decoder + Default + Unpin> Stream for TcpMultiConnectionSource<C>
where
    C::Error: From<io::Error>,
{
    type Item = Result<(u64, C::Item), C::Error>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let me = self.deref_mut();

        // Accept new connections
        loop {
            match me.listener.poll_accept(cx) {
                Poll::Ready(Ok((stream, _peer))) => {
                    let connection_id = me.next_connection_id;
                    me.next_connection_id += 1;

                    let (rx, tx) = stream.into_split();
                    let fr = FramedRead::new(rx, C::default());
                    let fw = FramedWrite::new(tx, C::default());

                    me.active_connections.push(Some((connection_id, fr)));
                    let _ = me.new_sink_sender.send((connection_id, fw));
                    let _ = me.membership_sender.send((connection_id, true));
                }
                Poll::Ready(Err(e)) => {
                    if !me.active_connections.iter().any(|c| c.is_some()) {
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

        // Poll all active connections for data using fair round-robin cursor
        let mut out = Poll::Pending;
        let mut any_removed = false;

        if !me.active_connections.is_empty() {
            let start_cursor = me.poll_cursor;

            loop {
                let current_length = me.active_connections.len();
                let id_and_stream = &mut me.active_connections[me.poll_cursor];
                let (connection_id, stream) = id_and_stream.as_mut().unwrap();
                let connection_id = *connection_id; // Copy the ID before borrowing stream

                // Move cursor to next source for next poll
                me.poll_cursor = (me.poll_cursor + 1) % current_length;

                match Pin::new(stream).poll_next(cx) {
                    Poll::Ready(Some(Ok(data))) => {
                        out = Poll::Ready(Some(Ok((connection_id, data))));
                        break;
                    }
                    Poll::Ready(Some(Err(_))) | Poll::Ready(None) => {
                        let _ = me.membership_sender.send((connection_id, false));
                        *id_and_stream = None; // Mark connection as removed
                        any_removed = true;
                    }
                    Poll::Pending => {}
                }

                // Check if we've completed a full round
                if me.poll_cursor == start_cursor {
                    break;
                }
            }
        }

        // Clean up None entries and adjust cursor
        let mut current_index = 0;
        let original_cursor = me.poll_cursor;

        if any_removed {
            me.active_connections.retain(|conn| {
                if conn.is_none() && current_index < original_cursor {
                    me.poll_cursor -= 1;
                }
                current_index += 1;
                conn.is_some()
            });
        }

        if me.poll_cursor == me.active_connections.len() {
            me.poll_cursor = 0;
        }

        out
    }
}

/// TCP-only multi-connection sink using concrete types (no boxing).
/// Routes (connection_id, data) to the appropriate connection.
pub struct TcpMultiConnectionSink<I, C: Encoder<I>> {
    /// Map of connection IDs to their framed writers
    pub connection_sinks: HashMap<u64, FramedWrite<OwnedWriteHalf, C>>,
    /// Channel to receive new sinks from TcpMultiConnectionSource
    pub new_sink_receiver: mpsc::UnboundedReceiver<(u64, FramedWrite<OwnedWriteHalf, C>)>,
    _marker: std::marker::PhantomData<fn(I) -> I>, /* fn(I) -> I instead of just I to keep the struct invariant over I, which keeps it Unpin. */
}

impl<I, C: Encoder<I> + Unpin> Sink<(u64, I)> for TcpMultiConnectionSink<I, C> {
    type Error = C::Error;

    fn poll_ready(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        let me = self.get_mut();
        // Receive any new sinks
        while let Poll::Ready(Some((id, sink))) = me.new_sink_receiver.poll_recv(cx) {
            me.connection_sinks.insert(id, sink);
        }
        Poll::Ready(Ok(()))
    }

    fn start_send(self: Pin<&mut Self>, item: (u64, I)) -> Result<(), Self::Error> {
        let me = self.get_mut();
        if let Some(sink) = me.connection_sinks.get_mut(&item.0) {
            let _ = Pin::new(sink).start_send(item.1);
        }
        Ok(())
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        let me = self.get_mut();
        let mut any_pending = false;

        me.connection_sinks
            .retain(|_id, sink| match Pin::new(sink).poll_flush(cx) {
                Poll::Ready(Ok(())) => true,
                Poll::Ready(Err(_)) => false,
                Poll::Pending => {
                    any_pending = true;
                    true
                }
            });

        if any_pending {
            Poll::Pending
        } else {
            Poll::Ready(Ok(()))
        }
    }

    fn poll_close(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        let me = self.get_mut();
        let mut any_pending = false;

        me.connection_sinks
            .retain(|_id, sink| match Pin::new(sink).poll_close(cx) {
                Poll::Ready(Ok(())) => false,
                Poll::Ready(Err(_)) => false,
                Poll::Pending => {
                    any_pending = true;
                    true
                }
            });

        if any_pending {
            Poll::Pending
        } else {
            Poll::Ready(Ok(()))
        }
    }
}

type TcpMultiConnectionParts<I, C> = (
    TcpMultiConnectionSource<C>,
    TcpMultiConnectionSink<I, C>,
    UnboundedReceiverStream<(u64, bool)>,
);

pub fn tcp_multi_connection<I, C>(listener: TcpListener) -> TcpMultiConnectionParts<I, C>
where
    C: Decoder + Encoder<I> + Default,
{
    let (new_sink_sender, new_sink_receiver) = mpsc::unbounded_channel();
    let (membership_sender, membership_receiver) = mpsc::unbounded_channel();

    let source = TcpMultiConnectionSource {
        listener,
        next_connection_id: 0,
        active_connections: Vec::new(),
        poll_cursor: 0,
        new_sink_sender,
        membership_sender,
    };

    let sink = TcpMultiConnectionSink {
        connection_sinks: HashMap::new(),
        new_sink_receiver,
        _marker: std::marker::PhantomData,
    };

    let membership = UnboundedReceiverStream::new(membership_receiver);

    (source, sink, membership)
}
