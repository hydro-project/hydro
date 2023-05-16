use crate::lattices::SealedSetOfIndexedValues;
use crate::structs::{ClientClass, Request};
use crate::test_data::client_class_iter;
use hydroflow::lattices::Merge;
use hydroflow::util::ipv4_resolve;
use hydroflow::{hydroflow_syntax, scheduled::graph::Hydroflow};

use bytes::Bytes;
use futures::stream::SplitSink;
use std::net::SocketAddr;
use tokio_util::{codec::LengthDelimitedCodec, udp::UdpFramed};

pub(crate) async fn rep_server_flow(
    shopping_ssiv: impl Iterator<Item = (usize, SealedSetOfIndexedValues<Request>)> + 'static,
    out_addr: SocketAddr,
    out: SplitSink<UdpFramed<LengthDelimitedCodec>, (Bytes, SocketAddr)>,
    server_addrs: impl Iterator<Item = SocketAddr> + 'static,
) -> Hydroflow {
    let broadcast_addr = ipv4_resolve("localhost:23460").unwrap();
    let (broadcast_out, broadcast_in, _) = hydroflow::util::bind_udp_bytes(broadcast_addr).await;
    let client_class = client_class_iter();
    let ssiv_merge =
        <SealedSetOfIndexedValues<Request> as Merge<SealedSetOfIndexedValues<Request>>>::merge;

    const ssiv_bot: fn() -> SealedSetOfIndexedValues<Request> = Default::default;

    hydroflow_syntax! {
        // Replicated Server
        source_iter(shopping_ssiv) -> group_by(ssiv_bot, ssiv_merge) -> [0]lookup_class;
        source_iter(client_class) -> [1]lookup_class;
        lookup_class = join()
          -> map(|(client, (li, class))| ((client, class), li) ) -> tee();
        lookup_class[clients] -> all_in;
        lookup_class[broadcast] -> [0]broadcast;
        source_iter(server_addrs) -> [1]broadcast;
        broadcast = cross_join() -> dest_sink_serde(broadcast_out);
        source_stream_serde(broadcast_in) -> map(Result::unwrap) -> map(|(m, _a): (((usize, ClientClass), SealedSetOfIndexedValues<Request>), _)| m) -> all_in;
        all_in = merge()
          -> group_by(ssiv_bot, ssiv_merge)
          -> map(|m| (m, out_addr)) -> dest_sink_serde(out);
    }
}
