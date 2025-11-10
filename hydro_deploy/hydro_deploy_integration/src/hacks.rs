use std::marker::PhantomData;
use std::pin::Pin;

use bytes::{Bytes, BytesMut};
use futures::sink::Buffer;
use futures::{Sink, SinkExt, Stream, StreamExt};
use sinktools::demux_map::DemuxMap;

use crate::{MergeSource, TaggedSource};

pub struct MapAdapterTypeHinter<MemberId: Unpin> {
    // #[pin]
    pub sink: DemuxMap<
        u32,
        Pin<Box<Buffer<Pin<Box<dyn Sink<Bytes, Error = std::io::Error> + Send + Sync>>, Bytes>>>,
    >,
    pub mapper: Box<dyn Fn(MemberId) -> u32>,
    pub _phantom: PhantomData<MemberId>,
}
// }

impl<MemberId: Unpin> Sink<(MemberId, Bytes)> for MapAdapterTypeHinter<MemberId> {
    type Error = std::io::Error;

    fn poll_ready(
        mut self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        self.sink.poll_ready_unpin(cx)
    }

    fn start_send(
        mut self: Pin<&mut Self>,
        (member_id, payload): (MemberId, Bytes),
    ) -> Result<(), Self::Error> {
        let mapped = (self.mapper)(member_id);
        self.sink.start_send_unpin((mapped, payload))
    }

    fn poll_flush(
        mut self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        self.sink.poll_flush_unpin(cx)
    }

    fn poll_close(
        mut self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        self.sink.poll_close_unpin(cx)
    }
}

// pub fn makeit<MemberId>(
//     si: DemuxMap<
//         u32,
//         Pin<Box<Buffer<Pin<Box<dyn Sink<Bytes, Error = std::io::Error> + Send + Sync>>, Bytes>>>,
//     >,
// ) -> MapAdapterTypeHinter<Tag, F: Fn(MemberId) -> u32 + Unpin> {
//     MapAdapterTypeHinter {
//         sink: si,
//         _phantom: Default::default(),
//     }
// }

pub struct SourceAdapterTypeHinter<MemberId: Unpin> {
    pub stream: MergeSource<
        Result<(u32, BytesMut), std::io::Error>,
        TaggedSource<
            u32,
            BytesMut,
            Pin<Box<dyn Stream<Item = Result<BytesMut, std::io::Error>> + Send + Sync>>,
        >,
    >,
    pub mapper: Box<dyn Fn(u32) -> MemberId>,
    pub _phantom: PhantomData<MemberId>,
}

impl<MemberId: Unpin> Stream for SourceAdapterTypeHinter<MemberId> {
    type Item = Result<(MemberId, BytesMut), std::io::Error>;

    fn poll_next(
        self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Option<Self::Item>> {
        let s = self.get_mut();

        s.stream
            .poll_next_unpin(cx)
            .map_ok(|(k, v)| ((s.mapper)(k), v))
    }
}

// Map<MergeSource<Result<(u32, BytesMut), Error>, TaggedSource<u32, BytesMut, Pin<Box<dyn Stream<Item = Result<BytesMut, Error>> + Send + Sync>>>>, impl FnMut(Result<(u32, BytesMut), Error>) -> Result<(MemberId<()>, BytesMut), Error>>
