#![cfg(not(target_arch = "wasm32"))]

use std::cell::RefCell;
use std::collections::hash_map::Entry::{Occupied, Vacant};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::pin::pin;
use std::rc::Rc;

use futures::{SinkExt, StreamExt};
use tokio::net::tcp::{OwnedReadHalf, OwnedWriteHalf};
use tokio::net::{TcpListener, TcpSocket, TcpStream};
use tokio::task::spawn_local;
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

/// A framed TCP `Sink` (sending).
pub type TcpFramedSink<T> = Sender<(T, SocketAddr)>;
/// A framed TCP `Stream` (receiving).
#[expect(type_alias_bounds, reason = "code readability")]
pub type TcpFramedStream<Codec: Decoder> =
    Receiver<Result<(<Codec as Decoder>::Item, SocketAddr), <Codec as Decoder>::Error>>;

/// Create a listening tcp socket, and then as new connections come in, receive their data and forward it to a queue.
pub async fn bind_tcp<T: 'static, Codec: 'static + Clone + Decoder + Encoder<T>>(
    endpoint: SocketAddr,
    codec: Codec,
) -> Result<(TcpFramedSink<T>, TcpFramedStream<Codec>, SocketAddr), std::io::Error> {
    let listener = TcpListener::bind(endpoint).await?;

    let bound_endpoint = listener.local_addr()?;

    let (tx_egress, mut rx_egress) = unsync_channel(None);
    let (tx_ingress, rx_ingress) = unsync_channel(None);

    let clients = Rc::new(RefCell::new(HashMap::new()));

    spawn_local({
        let clients = clients.clone();

        async move {
            while let Some((payload, addr)) = rx_egress.next().await {
                let client = clients.borrow_mut().remove(&addr);

                if let Some(mut sender) = client {
                    let _ = SinkExt::send(&mut sender, payload).await;
                    clients.borrow_mut().insert(addr, sender);
                }
            }
        }
    });

    spawn_local(async move {
        loop {
            let (stream, peer_addr) = if let Ok((stream, _)) = listener.accept().await {
                if let Ok(peer_addr) = stream.peer_addr() {
                    (stream, peer_addr)
                } else {
                    continue;
                }
            } else {
                continue;
            };

            let mut tx_ingress = tx_ingress.clone();

            let (send, recv) = tcp_framed(stream, codec.clone());

            // TODO: Using peer_addr here as the key is a little bit sketchy.
            // It's possible that a client could send a message, disconnect, then another client connects from the same IP address (and the same src port), and then the response could be sent to that new client.
            // This can be solved by using monotonically increasing IDs for each new client, but would break the similarity with the UDP versions of this function.
            clients.borrow_mut().insert(peer_addr, send);

            spawn_local({
                let clients = clients.clone();
                async move {
                    let mapped = recv.map(|x| Ok(x.map(|x| (x, peer_addr))));
                    let _ = tx_ingress.send_all(&mut pin!(mapped)).await;

                    clients.borrow_mut().remove(&peer_addr);
                }
            });
        }
    });

    Ok((tx_egress, rx_ingress, bound_endpoint))
}

/// The inverse of [`bind_tcp`].
///
/// When messages enqueued into the returned sender, tcp sockets will be created and connected as
/// necessary to send out the requests. As the responses come back, they will be forwarded to the
/// returned receiver.
pub fn connect_tcp<T: 'static, Codec: 'static + Clone + Decoder + Encoder<T>>(
    codec: Codec,
) -> (TcpFramedSink<T>, TcpFramedStream<Codec>) {
    let (tx_egress, mut rx_egress) = unsync_channel(None);
    let (tx_ingress, rx_ingress) = unsync_channel(None);

    spawn_local(async move {
        let mut streams = HashMap::new();

        while let Some((payload, addr)) = rx_egress.next().await {
            let stream = match streams.entry(addr) {
                Occupied(entry) => entry.into_mut(),
                Vacant(entry) => {
                    let socket = TcpSocket::new_v4().unwrap();
                    let stream = socket.connect(addr).await.unwrap();

                    let (send, recv) = tcp_framed(stream, codec.clone());

                    let mut tx_ingress = tx_ingress.clone();
                    spawn_local(async move {
                        let mapped = recv.map(|x| Ok(x.map(|x| (x, addr))));
                        let _ = tx_ingress.send_all(&mut pin!(mapped)).await;
                    });

                    entry.insert(send)
                }
            };

            let _ = stream.send(payload).await;
        }
    });

    (tx_egress, rx_ingress)
}
