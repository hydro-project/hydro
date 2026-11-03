#![cfg(not(target_arch = "wasm32"))]

use std::collections::HashMap;
use std::collections::hash_map::Entry::{Occupied, Vacant};
use std::fmt::{Debug, Display, Formatter};
use std::net::SocketAddr;

use futures::{SinkExt, StreamExt};
use tokio::net::tcp::{OwnedReadHalf, OwnedWriteHalf};
use tokio::net::{TcpListener, TcpSocket, TcpStream};
use tokio::select;
use tokio::task::spawn_local;
use tokio_stream::{StreamMap, StreamNotifyClose};
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
    FramedRead<OwnedReadHalf, Codec>,
)
where
    Codec: Clone + Decoder,
{
    let (recv, send) = stream.into_split();
    let send = FramedWrite::new(send, codec.clone());
    let recv = FramedRead::new(recv, codec);
    (send, recv)
}

/// Helper creates a TCP `Stream` and `Sink` for `Bytes` strings where each string is
/// length-delimited.
pub fn tcp_bytes(
    stream: TcpStream,
) -> (
    FramedWrite<OwnedWriteHalf, LengthDelimitedCodec>,
    FramedRead<OwnedReadHalf, LengthDelimitedCodec>,
) {
    tcp_framed(stream, LengthDelimitedCodec::new())
}

/// Helper creates a TCP `Stream` and `Sink` for undelimited streams of `Bytes`.
pub fn tcp_bytestream(
    stream: TcpStream,
) -> (
    FramedWrite<OwnedWriteHalf, BytesCodec>,
    FramedRead<OwnedReadHalf, BytesCodec>,
) {
    tcp_framed(stream, BytesCodec::new())
}

/// Helper creates a TCP `Stream` and `Sink` for `str`ings delimited by newlines.
pub fn tcp_lines(
    stream: TcpStream,
) -> (
    FramedWrite<OwnedWriteHalf, LinesCodec>,
    FramedRead<OwnedReadHalf, LinesCodec>,
) {
    tcp_framed(stream, LinesCodec::new())
}

/// Identity of one connection accepted by a [`bind_tcp`] listener.
///
/// Each listener numbers connections independently, starting at zero. The id is not a
/// [`SocketAddr`]: after a peer disconnects, a new connection can reuse that address, and a
/// response addressed by address would be delivered to the new peer.
#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct TcpSessionId(u64);

impl Display for TcpSessionId {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        Display::fmt(&self.0, f)
    }
}

/// A framed TCP `Sink` (sending) for [`connect_tcp`], addressed by destination [`SocketAddr`].
///
/// Accepted connections from [`bind_tcp`] are addressed by [`TcpSessionId`] ([`TcpSessionSink`]).
/// UDP sinks stay address-keyed; a TCP accept session does not, because a peer address can be
/// reused.
pub type TcpFramedSink<T> = Sender<(T, SocketAddr)>;
/// A framed TCP `Stream` (receiving) for [`connect_tcp`], tagged with the remote [`SocketAddr`].
#[expect(type_alias_bounds, reason = "code readability")]
pub type TcpFramedStream<Codec: Decoder> =
    Receiver<Result<(<Codec as Decoder>::Item, SocketAddr), <Codec as Decoder>::Error>>;

/// A [`bind_tcp`] `Sink`. Each item is `(payload, session)`.
pub type TcpSessionSink<T> = Sender<(T, TcpSessionId)>;
/// A [`bind_tcp`] `Stream`. Each frame is paired with the [`TcpSessionId`] that produced it.
#[expect(type_alias_bounds, reason = "code readability")]
pub type TcpSessionStream<Codec: Decoder> =
    Receiver<Result<(<Codec as Decoder>::Item, TcpSessionId), <Codec as Decoder>::Error>>;

// TODO(mingwei): this temporary code should be replaced with a properly thought out networking system.
/// Create a listening TCP socket, then forward framed data from each accepted connection.
///
/// Each accepted connection is assigned a new [`TcpSessionId`]. Inbound frames are tagged with
/// that id, and outbound `(payload, id)` pairs are written to the matching connection. An id is
/// never reused on this listener, so a response cannot be delivered to a later peer that reused
/// the same address. Outbound [`connect_tcp`] and UDP helpers stay [`SocketAddr`]-keyed.
///
/// The returned [`SocketAddr`] is the local address bound (the input port may be `0`).
pub async fn bind_tcp<Item, Codec>(
    endpoint: SocketAddr,
    codec: Codec,
) -> Result<(TcpSessionSink<Item>, TcpSessionStream<Codec>, SocketAddr), std::io::Error>
where
    Item: 'static,
    Codec: 'static + Clone + Decoder + Encoder<Item>,
    <Codec as Encoder<Item>>::Error: Debug,
{
    let listener = TcpListener::bind(endpoint).await?;

    let bound_endpoint = listener.local_addr()?;

    let (send_egress, mut recv_egress) = unsync_channel::<(Item, TcpSessionId)>(None);
    let (send_ingres, recv_ingres) = unsync_channel(None);

    spawn_local(async move {
        let send_ingress = send_ingres;
        // Map of `session id -> peers`, to send messages to.
        let mut peers_send = HashMap::new();
        // `StreamMap` of `session id -> peers`, to receive messages from. `StreamNotifyClose`
        // reports disconnects so the write half can be dropped with the read half. `StreamMap`
        // then removes the finished stream.
        let mut peers_recv =
            StreamMap::<TcpSessionId, StreamNotifyClose<FramedRead<OwnedReadHalf, Codec>>>::new();
        // Per-listener generation. Not a peer address: that address can be reused after disconnect.
        let mut next_session_id = 0u64;

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
                    let Some((payload, session_id)) = msg_send else {
                        // `None` if the send side has been dropped (no more send messages will ever come).
                        continue;
                    };
                    let Some(stream) = peers_send.get_mut(&session_id) else {
                        tracing::warn!("Dropping message to non-connected session: {}", session_id);
                        continue;
                    };
                    if let Err(err) = SinkExt::send(stream, payload).await {
                        tracing::error!("IO or codec error sending message to session {}, disconnecting: {:?}", session_id, err);
                        peers_send.remove(&session_id); // `Drop` disconnects.
                    };
                }
                // Receive incoming messages.
                msg_recv = peers_recv.next(), if !peers_recv.is_empty() => {
                    // If `peers_recv` is empty then `next()` will immediately return `None` which
                    // would cause the loop to spin.
                    let Some((session_id, payload_result)) = msg_recv else {
                        continue; // => `peers_recv.is_empty()`.
                    };
                    // `None` means the read half closed. Drop the write half so this id cannot
                    // still target a socket after the peer is gone.
                    let Some(payload_result) = payload_result else {
                        peers_send.remove(&session_id);
                        continue;
                    };
                    if let Err(err) = send_ingress.send(payload_result.map(|payload| (payload, session_id))).await {
                        tracing::error!("Error passing along received message: {:?}", err);
                    }
                }
                // Accept new clients.
                new_peer = listener.accept() => {
                    let Ok((stream, _addr)) = new_peer else {
                        continue;
                    };
                    let (peer_send, peer_recv) = tcp_framed(stream, codec.clone());

                    let session_id = TcpSessionId(next_session_id);
                    next_session_id += 1;
                    peers_send.insert(session_id, peer_send);
                    peers_recv.insert(session_id, StreamNotifyClose::new(peer_recv));
                }
            }
        }
    });

    Ok((send_egress, recv_ingres, bound_endpoint))
}

/// Open outbound TCP connections on demand.
///
/// This is the client-side counterpart of [`bind_tcp`], but messages are addressed by destination
/// [`SocketAddr`], not [`TcpSessionId`]. When a message is enqueued for an address, a socket is
/// created and connected if needed, then reused for later messages to that address. Responses are
/// forwarded to the returned receiver tagged with that address.
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

#[cfg(test)]
mod test {
    use std::net::SocketAddr;
    use std::time::Duration;

    use futures::{Stream, StreamExt};
    use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
    use tokio::net::TcpSocket;
    use tokio_util::codec::LinesCodec;

    use super::bind_tcp;

    /// Bind `addr` with `SO_REUSEADDR` and zero linger so a closed connection's source port can be
    /// reused immediately.
    fn reusable_socket(addr: SocketAddr) -> TcpSocket {
        let socket = TcpSocket::new_v4().expect("create tcp socket");
        socket.set_reuseaddr(true).expect("SO_REUSEADDR");
        socket
            .set_linger(Some(Duration::from_secs(0)))
            .expect("SO_LINGER");
        socket.bind(addr).expect("bind client socket");
        socket
    }

    async fn next_payload<S, T, A, E>(rx: &mut S) -> (T, A)
    where
        S: Stream<Item = Result<(T, A), E>> + Unpin,
        E: std::fmt::Debug,
    {
        loop {
            match rx.next().await.expect("server stream closed") {
                Ok(frame) => return frame,
                // A reset connection can surface as a codec error before the next payload.
                Err(err) => tracing::debug!("ignoring tcp frame error: {err:?}"),
            }
        }
    }

    #[crate::test]
    async fn reused_peer_addr_gets_distinct_tcp_sessions() {
        let (tx, mut rx, server_addr) = bind_tcp("127.0.0.1:0".parse().unwrap(), LinesCodec::new())
            .await
            .unwrap();

        let mut client_1 = reusable_socket("127.0.0.1:0".parse().unwrap())
            .connect(server_addr)
            .await
            .unwrap();
        let client_addr = client_1.local_addr().unwrap();
        client_1.write_all(b"one\n").await.unwrap();
        client_1.flush().await.unwrap();

        let (msg, id_1) = tokio::time::timeout(Duration::from_secs(2), next_payload(&mut rx))
            .await
            .expect("timed out waiting for the first connection");
        assert_eq!(msg, "one");

        // Close the first connection, then accept a second one from the same socket address.
        drop(client_1);

        let mut client_2 = reusable_socket(client_addr)
            .connect(server_addr)
            .await
            .unwrap();
        assert_eq!(
            client_2.local_addr().unwrap(),
            client_addr,
            "test did not reuse the peer address"
        );
        client_2.write_all(b"two\n").await.unwrap();
        client_2.flush().await.unwrap();

        let (msg, id_2) = tokio::time::timeout(Duration::from_secs(2), next_payload(&mut rx))
            .await
            .expect("timed out waiting for the second connection");
        assert_eq!(msg, "two");
        assert_ne!(
            id_1, id_2,
            "two connections from {client_addr} must not share a session id"
        );

        tx.send(("for-first".to_owned(), id_1)).await.unwrap();
        tx.send(("for-second".to_owned(), id_2)).await.unwrap();

        let mut lines = BufReader::new(&mut client_2);
        let mut response = String::new();
        tokio::time::timeout(Duration::from_secs(2), lines.read_line(&mut response))
            .await
            .expect("timed out waiting for a response on the second connection")
            .unwrap();
        assert_eq!(
            response, "for-second\n",
            "response addressed to the first session was delivered to the second connection"
        );
    }
}
