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

pub(crate) async fn client_state_flow(
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
    let (carts_out, carts_in, _) = dfir_rs::util::bind_udp_bytes(local_addr).await;

    // This is like server_state_flow, but we split it into two processes at a different spot.
    // Here, the first process takes in shopping_ssiv requests and runs a stateful fold_keyed, persisting all the shopping requests in ssiv's.
    // The second process listens on reqs_in and runs the lookup join.
    dfir_syntax! {
        source_iter(shopping_ssiv)
          -> fold_keyed::<'static>(SSIV_BOT, |state, delta| { (ssiv_merge)(state, delta); })
          -> map(|pair| (pair, remote_addr)) -> dest_sink_serde(carts_out);
        source_stream_serde(carts_in) -> map(Result::unwrap) -> map(|((client, cart), _a): ((usize, SealedSetOfIndexedValues<Request>), _)| (client, cart))
          -> [0]lookup_class;
        source_iter(client_class) -> [1]lookup_class;
        lookup_class = join::<'static>()
          -> map(|(client, (li, class))| ((client, class), li))
          -> map(|m| (m, out_addr)) -> dest_sink_serde(out);
    }
}
