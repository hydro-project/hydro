use std::net::SocketAddr;

use bytes::Bytes;
use futures::stream::SplitSink;
use hydroflow::hydroflow_syntax;
use hydroflow::lattices::Merge;
use hydroflow::scheduled::graph::Hydroflow;
use tokio_util::codec::LengthDelimitedCodec;
use tokio_util::udp::UdpFramed;

use crate::lattices::SealedSetOfIndexedValues;
use crate::structs::{ClientClass, Request};
use crate::test_data::client_class_iter;

pub(crate) async fn rep_server_flow(
    shopping_ssiv: impl Iterator<Item = (usize, SealedSetOfIndexedValues<Request>)> + 'static,
    out_addr: SocketAddr,
    out: SplitSink<UdpFramed<LengthDelimitedCodec>, (Bytes, SocketAddr)>,
    local_addr: SocketAddr,
    remote_addr: SocketAddr,
    gossip_addr: SocketAddr,
    server_addrs: impl Iterator<Item = SocketAddr> + 'static,
) -> Hydroflow {
    let (broadcast_out, broadcast_in, _) = hydroflow::util::bind_udp_bytes(gossip_addr).await;
    let client_class = client_class_iter();
    let ssiv_merge =
        <SealedSetOfIndexedValues<Request> as Merge<SealedSetOfIndexedValues<Request>>>::merge;

    const SSIV_BOT: fn() -> SealedSetOfIndexedValues<Request> = Default::default;

    // Set up the Udp socket for proxy-server communication
    let (reqs_out, reqs_in, _) = hydroflow::util::bind_udp_bytes(local_addr).await;

    hydroflow_syntax! {
        // Client Proxy
        source_iter(shopping_ssiv)
          -> map(|pair| (pair, remote_addr)) -> dest_sink_serde(reqs_out);
        // Replicated Server
        source_stream_serde(reqs_in)
          -> map(Result::unwrap)
          -> map(|((client, req), _a): ((usize, SealedSetOfIndexedValues<Request>), _)| (client, req))
          -> group_by(SSIV_BOT, ssiv_merge)
          -> [0]lookup_class;
        source_iter(client_class) -> [1]lookup_class;
        lookup_class = join()
          -> map(|(client, (li, class))| ((client, class), li) ) -> tee();
        lookup_class[clients] -> all_in;
        lookup_class[broadcast] -> [0]broadcast;
        source_iter(server_addrs) -> [1]broadcast;
        broadcast = cross_join() -> dest_sink_serde(broadcast_out);
        source_stream_serde(broadcast_in)
          -> map(Result::unwrap)
          -> map(|(m, _a): (((usize, ClientClass), SealedSetOfIndexedValues<Request>), _)| m)
          -> all_in;
        all_in = merge()
          -> group_by(SSIV_BOT, ssiv_merge)
          -> unique()
          -> map(|m| (m, out_addr)) -> dest_sink_serde(out);
    }
}
