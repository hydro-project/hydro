use crate::lattices::SealedSetOfIndexedValues;
use crate::structs::Request;
use crate::test_data::client_class_iter;
use hydroflow::lattices::Merge;
use hydroflow::{hydroflow_syntax, scheduled::graph::Hydroflow};

use bytes::Bytes;
use futures::stream::SplitSink;
use std::net::SocketAddr;
use tokio_util::{codec::LengthDelimitedCodec, udp::UdpFramed};

pub(crate) async fn server_state_flow(
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
    let (reqs_out, reqs_in, _) = hydroflow::util::bind_udp_bytes(server_addr).await;

    // This is like push_group_flow, but we split it into two transducers that communicate via reqs_out and reqs_in.
    // The first transducer takes in shopping_ssiv requests, and forwards them via reqs_out to the second transducer.
    // The second transducer listens on reqs_in and runs the stateful logic of group_by and join.
    hydroflow_syntax! {
        // Networked: Server-Side State
        source_iter(shopping_ssiv)
          -> map(|pair| (pair, server_addr)) -> dest_sink_serde(reqs_out);
        source_stream_serde(reqs_in) -> map(Result::unwrap) -> map(|((client, req), _a): ((usize, SealedSetOfIndexedValues<Request>), _)| (client, req))
          -> group_by(ssiv_bot, ssiv_merge) -> [0]lookup_class;
        source_iter(client_class) -> [1]lookup_class;
        lookup_class = join()
          -> map(|(client, (li, class))| ((client, class), li))
          -> map(|m| (m, out_addr)) -> dest_sink_serde(out);
    }
}
