#![cfg(not(target_arch = "wasm32"))]

use std::net::SocketAddr;

use bytes::Bytes;
use futures::stream::{SplitSink, SplitStream, StreamExt};
use tokio::net::UdpSocket;
use tokio_util::codec::length_delimited::LengthDelimitedCodec;
use tokio_util::codec::{BytesCodec, Decoder, Encoder, LinesCodec};
use tokio_util::udp::UdpFramed;

/// A framed UDP `Sink` (sending).
pub type UdpFramedSink<Codec, Item> = SplitSink<UdpFramed<Codec>, (Item, SocketAddr)>;
/// A framed UDP `Stream` (receiving).
pub type UdpFramedStream<Codec> = futures::stream::Fuse<SplitStream<UdpFramed<Codec>>>;
/// Returns a UDP `Stream`, `Sink`, and address for the given socket, using the given `Codec` to
/// handle delineation between inputs/outputs.
pub fn udp_framed<Codec, Item>(
    socket: UdpSocket,
    codec: Codec,
) -> (
    UdpFramedSink<Codec, Item>,
    UdpFramedStream<Codec>,
    SocketAddr,
)
where
    Codec: Encoder<Item> + Decoder,
{
    let framed = UdpFramed::new(socket, codec);
    let addr = framed.get_ref().local_addr().unwrap();
    let split = StreamExt::split(framed);
    (split.0, split.1.fuse(), addr)
}

/// A UDP length-delimited frame `Sink` (sending).
pub type UdpSink = UdpFramedSink<LengthDelimitedCodec, Bytes>;
/// A UDP length-delimited frame `Stream` (receiving).
pub type UdpStream = UdpFramedStream<LengthDelimitedCodec>;
/// Helper creates a UDP `Stream` and `Sink` for `Bytes` strings where each string is
/// length-delimited.
pub fn udp_bytes(socket: UdpSocket) -> (UdpSink, UdpStream, SocketAddr) {
    udp_framed(socket, LengthDelimitedCodec::new())
}

/// A UDP undelimited bytes `Sink` (sending).
pub type UdpBytesSink = UdpFramedSink<BytesCodec, Bytes>;
/// A UDP undelimited bytes `Stream` (receiving).
pub type UdpBytesStream = UdpFramedStream<BytesCodec>;
/// Helper creates a UDP `Stream` and `Sink` for undelimited streams of `Bytes`.
pub fn udp_bytestream(socket: UdpSocket) -> (UdpBytesSink, UdpBytesStream, SocketAddr) {
    udp_framed(socket, BytesCodec::new())
}

/// A UDP newline-delimited `String` `Sink` (sending).
pub type UdpLinesSink = UdpFramedSink<LinesCodec, String>;
/// A UDP newline-delimited `String` `Stream` (receivng).
pub type UdpLinesStream = UdpFramedStream<LinesCodec>;
/// Helper creates a UDP `Stream` and `Sink` for `String`s delimited by newlines.
pub fn udp_lines(
    socket: UdpSocket,
) -> (
    UdpFramedSink<LinesCodec, String>,
    UdpFramedStream<LinesCodec>,
    SocketAddr,
) {
    udp_framed(socket, LinesCodec::new())
}
