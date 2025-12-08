use std::net::SocketAddr;

use bytes::Bytes;
use dfir_rs::dfir_syntax;
use dfir_rs::lattices::Merge;
use dfir_rs::scheduled::graph::Dfir;
use futures::stream::SplitSink;
use tokio_util::codec::LengthDelimitedCodec;
use tokio_util::udp::UdpFramed;

use crate::lattices::SealedSetOfIndexedValues;
use crate::structs::Request;
use crate::test_data::client_class_iter;

pub(crate) async fn server_state_flow(
    shopping_ssiv: impl Iterator<Item = (usize, SealedSetOfIndexedValues<Request>)> + 'static,
    out_addr: SocketAddr,
    out: SplitSink<UdpFramed<LengthDelimitedCodec>, (Bytes, SocketAddr)>,
    local_addr: SocketAddr,
    remote_addr: SocketAddr,
) -> Dfir<'static> {
    let client_class = client_class_iter();

    // First define some shorthand for the merge and bot of this lattice
    let ssiv_merge =
        <SealedSetOfIndexedValues<Request> as Merge<SealedSetOfIndexedValues<Request>>>::merge;
    const SSIV_BOT: fn() -> SealedSetOfIndexedValues<Request> = Default::default;

    // Set up the Udp socket for proxy-server communication
    let (reqs_out, reqs_in, _) = dfir_rs::util::bind_udp_bytes(local_addr).await;

    // This is like push_group_flow, but we split it into two processes that communicate via reqs_out and reqs_in.
    // The first process takes in shopping_ssiv requests, and forwards them via reqs_out to the second process.
    // The second process listens on reqs_in and runs the stateful logic of fold_keyed and join.
    dfir_syntax! {
        // Networked: Server-Side State
        source_iter(shopping_ssiv)
          -> map(|pair| (pair, remote_addr)) -> dest_sink_serde(reqs_out);
        source_stream_serde(reqs_in) -> map(Result::unwrap) -> map(|((client, req), _a): ((usize, SealedSetOfIndexedValues<Request>), _)| (client, req))
          -> fold_keyed::<'static>(SSIV_BOT, |state, delta| { (ssiv_merge)(state, delta); }) -> [0]lookup_class;
        source_iter(client_class) -> [1]lookup_class;
        lookup_class = join::<'static>()
          -> map(|(client, (li, class))| ((client, class), li))
          -> map(|m| (m, out_addr)) -> dest_sink_serde(out);
    }
}
