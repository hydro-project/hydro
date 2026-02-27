#![cfg(not(target_arch = "wasm32"))]

use std::collections::HashMap;
use std::collections::hash_map::Entry::{Occupied, Vacant};
use std::fmt::Debug;
use std::net::SocketAddr;

use futures::{SinkExt, StreamExt};
use tokio::net::tcp::{OwnedReadHalf, OwnedWriteHalf};
use tokio::net::{TcpListener, TcpSocket, TcpStream};
use tokio::select;
use tokio::task::spawn_local;
use tokio_stream::StreamMap;
use tokio_util::codec::{
    BytesCodec, Decoder, Encoder, FramedRead, FramedWrite, LengthDelimitedCodec, LinesCodec,
};

use super::unsync::mpsc::{Receiver, Sender};
use super::unsync_channel;

/// Helper creates a TCP `Stream` and `Sink` from the given socket, using the given `Codec` to
/// handle delineation between inputs/outputs.
pub fn tcp_framed<Codec>(
    stream: TcpStream,
    codec: Codec,
) -> (
    FramedWrite<OwnedWriteHalf, Codec>,
    futures::stream::Fuse<FramedRead<OwnedReadHalf, Codec>>,
)
where
    Codec: Clone + Decoder,
{
    let (recv, send) = stream.into_split();
    let send = FramedWrite::new(send, codec.clone());
    let recv = FramedRead::new(recv, codec);
    (send, StreamExt::fuse(recv))
}

/// Helper creates a TCP `Stream` and `Sink` for `Bytes` strings where each string is
/// length-delimited.
pub fn tcp_bytes(
    stream: TcpStream,
) -> (
    FramedWrite<OwnedWriteHalf, LengthDelimitedCodec>,
    futures::stream::Fuse<FramedRead<OwnedReadHalf, LengthDelimitedCodec>>,
) {
    tcp_framed(stream, LengthDelimitedCodec::new())
}

/// Helper creates a TCP `Stream` and `Sink` for undelimited streams of `Bytes`.
pub fn tcp_bytestream(
    stream: TcpStream,
) -> (
    FramedWrite<OwnedWriteHalf, BytesCodec>,
    futures::stream::Fuse<FramedRead<OwnedReadHalf, BytesCodec>>,
) {
    tcp_framed(stream, BytesCodec::new())
}

/// Helper creates a TCP `Stream` and `Sink` for `str`ings delimited by newlines.
pub fn tcp_lines(
    stream: TcpStream,
) -> (
    FramedWrite<OwnedWriteHalf, LinesCodec>,
    futures::stream::Fuse<FramedRead<OwnedReadHalf, LinesCodec>>,
) {
    tcp_framed(stream, LinesCodec::new())
}

/// A framed TCP `Sink` (sending).
pub type TcpFramedSink<T> = Sender<(T, SocketAddr)>;
/// A framed TCP `Stream` (receiving).
#[expect(type_alias_bounds, reason = "code readability")]
pub type TcpFramedStream<Codec: Decoder> =
    futures::stream::Fuse<Receiver<Result<(<Codec as Decoder>::Item, SocketAddr), <Codec as Decoder>::Error>>>;

// TODO(mingwei): this temporary code should be replaced with a properly thought out networking system.
/// Create a listening tcp socket, and then as new connections come in, receive their data and forward it to a queue.
pub async fn bind_tcp<Item, Codec>(
    endpoint: SocketAddr,
    codec: Codec,
) -> Result<(TcpFramedSink<Item>, TcpFramedStream<Codec>, SocketAddr), std::io::Error>
where
    Item: 'static,
    Codec: 'static + Clone + Decoder + Encoder<Item>,
    <Codec as Encoder<Item>>::Error: Debug,
{
    let listener = TcpListener::bind(endpoint).await?;

    let bound_endpoint = listener.local_addr()?;

    let (send_egress, mut recv_egress) = unsync_channel::<(Item, SocketAddr)>(None);
    let (send_ingres, recv_ingres) = unsync_channel(None);

    spawn_local(async move {
        let send_ingress = send_ingres;
        // Map of `addr -> peers`, to send messages to.
        let mut peers_send = HashMap::new();
        // `StreamMap` of `addr -> peers`, to receive messages from. Automatically removes streams
        // when they disconnect.
        let mut peers_recv = StreamMap::<SocketAddr, futures::stream::Fuse<FramedRead<OwnedReadHalf, Codec>>>::new();

        loop {
            // Calling methods in a loop, futures must be cancel-safe.
            select! {
                // `biased` means the cases will be prioritized in the order they are listed.
                // First we accept any new connections
                // This is not strictly neccessary, but lets us do our internal work (send outgoing
                // messages) before accepting more work (receiving more messages, accepting new
                // clients).
                biased;
                // Send outgoing messages.
                msg_send = recv_egress.next() => {
                    let Some((payload, peer_addr)) = msg_send else {
                        // `None` if the send side has been dropped (no more send messages will ever come).
                        continue;
                    };
                    let Some(stream) = peers_send.get_mut(&peer_addr) else {
                        tracing::warn!("Dropping message to non-connected peer: {}", peer_addr);
                        continue;
                    };
                    if let Err(err) = SinkExt::send(stream, payload).await {
                        tracing::error!("IO or codec error sending message to peer {}, disconnecting: {:?}", peer_addr, err);
                        peers_send.remove(&peer_addr); // `Drop` disconnects.
                    };
                }
                // Receive incoming messages.
                msg_recv = peers_recv.next(), if !peers_recv.is_empty() => {
                    // If `peers_recv` is empty then `next()` will immediately return `None` which
                    // would cause the loop to spin.
                    let Some((peer_addr, payload_result)) = msg_recv else {
                        continue; // => `peers_recv.is_empty()`.
                    };
                    if let Err(err) = send_ingress.send(payload_result.map(|payload| (payload, peer_addr))).await {
                        tracing::error!("Error passing along received message: {:?}", err);
                    }
                }
                // Accept new clients.
                new_peer = listener.accept() => {
                    let Ok((stream, _addr)) = new_peer else {
                        continue;
                    };
                    let Ok(peer_addr) = stream.peer_addr() else {
                        continue;
                    };
                    let (peer_send, peer_recv) = tcp_framed(stream, codec.clone());

                    // TODO: Using peer_addr here as the key is a little bit sketchy.
                    // It's possible that a peer could send a message, disconnect, then another peer connects from the
                    // same IP address (and the same src port), and then the response could be sent to that new client.
                    // This can be solved by using monotonically increasing IDs for each new peer, but would break the
                    // similarity with the UDP versions of this function.
                    peers_send.insert(peer_addr, peer_send);
                    peers_recv.insert(peer_addr, peer_recv);
                }
            }
        }
    });

    Ok((send_egress, recv_ingres, bound_endpoint))
}

/// The inverse of [`bind_tcp`].
///
/// When messages enqueued into the returned sender, tcp sockets will be created and connected as
/// necessary to send out the requests. As the responses come back, they will be forwarded to the
/// returned receiver.
pub fn connect_tcp<Item, Codec>(codec: Codec) -> (TcpFramedSink<Item>, TcpFramedStream<Codec>)
where
    Item: 'static,
    Codec: 'static + Clone + Decoder + Encoder<Item>,
    <Codec as Encoder<Item>>::Error: Debug,
{
    let (send_egress, mut recv_egress) = unsync_channel(None);
    let (send_ingres, recv_ingres) = unsync_channel(None);

    spawn_local(async move {
        let send_ingres = send_ingres;
        // Map of `addr -> peers`, to send messages to.
        let mut peers_send = HashMap::new();
        // `StreamMap` of `addr -> peers`, to receive messages from. Automatically removes streams
        // when they disconnect.
        let mut peers_recv = StreamMap::new();

        loop {
            // Calling methods in a loop, futures must be cancel-safe.
            select! {
                // `biased` means the cases will be prioritized in the order they are listed.
                // This is not strictly neccessary, but lets us do our internal work (send outgoing
                // messages) before accepting more work (receiving more messages).
                biased;
                // Send outgoing messages.
                msg_send = recv_egress.next() => {
                    let Some((payload, peer_addr)) = msg_send else {
                        // `None` if the send side has been dropped (no more send messages will ever come).
                        continue;
                    };

                    let stream = match peers_send.entry(peer_addr) {
                        Occupied(entry) => entry.into_mut(),
                        Vacant(entry) => {
                            let socket = TcpSocket::new_v4().unwrap();
                            let stream = socket.connect(peer_addr).await.unwrap();

                            let (peer_send, peer_recv) = tcp_framed(stream, codec.clone());

                            peers_recv.insert(peer_addr, peer_recv);
                            entry.insert(peer_send)
                        }
                    };

                    if let Err(err) = stream.send(payload).await {
                        tracing::error!("IO or codec error sending message to peer {}, disconnecting: {:?}", peer_addr, err);
                        peers_send.remove(&peer_addr); // `Drop` disconnects.
                    }
                }
                // Receive incoming messages.
                msg_recv = peers_recv.next(), if !peers_recv.is_empty() => {
                    // If `peers_recv` is empty then `next()` will immediately return `None` which
                    // would cause the loop to spin.
                    let Some((peer_addr, payload_result)) = msg_recv else {
                        continue; // => `peers_recv.is_empty()`.
                    };
                    if let Err(err) = send_ingres.send(payload_result.map(|payload| (payload, peer_addr))).await {
                        tracing::error!("Error passing along received message: {:?}", err);
                    }
                }
            }
        }
    });

    (send_egress, recv_ingres)
}
