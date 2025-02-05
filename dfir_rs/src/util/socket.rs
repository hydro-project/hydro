use tokio::net::unix::{OwnedReadHalf, OwnedWriteHalf};
use tokio::net::UnixStream;
use tokio_util::codec::{
    BytesCodec, Decoder, FramedRead, FramedWrite, LengthDelimitedCodec, LinesCodec,
};

/// Helper creates a Unix `Stream` and `Sink` from the given socket, using the given `Codec` to
/// handle delineation between inputs/outputs.
pub fn unix_framed<Codec>(
    stream: UnixStream,
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

/// Helper creates a Unix `Stream` and `Sink` for `Bytes` strings where each string is
/// length-delimited.
pub fn unix_bytes(
    stream: UnixStream,
) -> (
    FramedWrite<OwnedWriteHalf, LengthDelimitedCodec>,
    FramedRead<OwnedReadHalf, LengthDelimitedCodec>,
) {
    unix_framed(stream, LengthDelimitedCodec::new())
}

/// Helper creates a Unix `Stream` and `Sink` for undelimited streams of `Bytes`.
pub fn unix_bytestream(
    stream: UnixStream,
) -> (
    FramedWrite<OwnedWriteHalf, BytesCodec>,
    FramedRead<OwnedReadHalf, BytesCodec>,
) {
    unix_framed(stream, BytesCodec::new())
}

/// Helper creates a Unix `Stream` and `Sink` for `str`ings delimited by newlines.
pub fn unix_lines(
    stream: UnixStream,
) -> (
    FramedWrite<OwnedWriteHalf, LinesCodec>,
    FramedRead<OwnedReadHalf, LinesCodec>,
) {
    unix_framed(stream, LinesCodec::new())
}
