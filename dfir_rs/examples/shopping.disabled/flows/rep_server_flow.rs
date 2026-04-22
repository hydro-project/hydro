use std::net::SocketAddr;

use bytes::Bytes;
use dfir_rs::dfir_syntax;
use dfir_rs::lattices::Merge;
use dfir_rs::scheduled::graph::Dfir;
use futures::stream::SplitSink;
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
) -> Dfir<'static> {
    let (broadcast_out, broadcast_in, _) = dfir_rs::util::bind_udp_bytes(gossip_addr).await;
    let client_class = client_class_iter();
    let ssiv_merge =
        <SealedSetOfIndexedValues<Request> as Merge<SealedSetOfIndexedValues<Request>>>::merge;

    const SSIV_BOT: fn() -> SealedSetOfIndexedValues<Request> = Default::default;

    // Set up the Udp socket for proxy-server communication
    let (reqs_out, reqs_in, _) = dfir_rs::util::bind_udp_bytes(local_addr).await;

    dfir_syntax! {
        // Client Proxy
        source_iter(shopping_ssiv)
          -> map(|pair| (pair, remote_addr)) -> dest_sink_serde(reqs_out);
        // Replicated Server
        source_stream_serde(reqs_in)
          -> map(Result::unwrap)
          -> map(|((client, req), _a): ((usize, SealedSetOfIndexedValues<Request>), _)| (client, req))
          -> fold_keyed::<'static>(SSIV_BOT, |state, delta| { (ssiv_merge)(state, delta); })
          -> [0]lookup_class;
        source_iter(client_class) -> [1]lookup_class;
        lookup_class = join::<'static>()
          -> map(|(client, (li, class))| ((client, class), li) ) -> tee();
        lookup_class[clients] -> all_in;
        lookup_class[broadcast] -> [0]broadcast;
        source_iter(server_addrs) -> [1]broadcast;
        broadcast = cross_join::<'static>() -> dest_sink_serde(broadcast_out);
        source_stream_serde(broadcast_in)
          -> map(Result::unwrap)
          -> map(|(m, _a): (((usize, ClientClass), SealedSetOfIndexedValues<Request>), _)| m)
          -> all_in;
        all_in = union()
          -> fold_keyed::<'static>(SSIV_BOT, |state, delta| { (ssiv_merge)(state, delta); })
          -> unique()
          -> map(|m| (m, out_addr)) -> dest_sink_serde(out);
    }
}
