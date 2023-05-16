use crate::lattices::SealedSetOfIndexedValues;
use crate::structs::Request;
use crate::test_data::client_class_iter;
use hydroflow::lattices::Merge;
use hydroflow::{hydroflow_syntax, scheduled::graph::Hydroflow};

use bytes::Bytes;
use futures::stream::SplitSink;
use std::net::SocketAddr;
use tokio_util::{codec::LengthDelimitedCodec, udp::UdpFramed};

pub(crate) async fn client_state_flow(
    shopping_ssiv: impl Iterator<Item = (usize, SealedSetOfIndexedValues<Request>)> + 'static,
    out_addr: SocketAddr,
    out: SplitSink<UdpFramed<LengthDelimitedCodec>, (Bytes, SocketAddr)>,
    server_addr: SocketAddr,
) -> Hydroflow {
    let client_class = client_class_iter();

    // First define some shorthand for the merge and bot of this lattice
    let ssiv_merge =
        <SealedSetOfIndexedValues<Request> as Merge<SealedSetOfIndexedValues<Request>>>::merge;
    const ssiv_bot: fn() -> SealedSetOfIndexedValues<Request> = Default::default;

    // Set up the Udp socket for proxy-server communication
    let (carts_out, carts_in, _) = hydroflow::util::bind_udp_bytes(server_addr).await;

    // This is like server_state_flow, but we split it into two transducers at a different spot.
    // Here, the first transducer takes in shopping_ssiv requests and runs a stateful group_by, persisting all the shopping requests in ssiv's.
    // The second transducer listens on reqs_in and runs the lookup join.
    hydroflow_syntax! {
        source_iter(shopping_ssiv)
          -> group_by(ssiv_bot, ssiv_merge)
          -> map(|pair| (pair, server_addr)) -> dest_sink_serde(carts_out);
        source_stream_serde(carts_in) -> map(Result::unwrap) -> map(|((client, cart), _a): ((usize, SealedSetOfIndexedValues<Request>), _)| (client, cart))
          -> [0]lookup_class;
        source_iter(client_class) -> [1]lookup_class;
        lookup_class = join()
          -> map(|(client, (li, class))| ((client, class), li))
          -> map(|m| (m, out_addr)) -> dest_sink_serde(out);
    }
}
