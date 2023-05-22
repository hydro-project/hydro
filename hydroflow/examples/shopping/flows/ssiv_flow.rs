use std::net::SocketAddr;

use bytes::Bytes;
use futures::stream::SplitSink;
use hydroflow::hydroflow_syntax;
use hydroflow::lattices::Merge;
use hydroflow::scheduled::graph::Hydroflow;
use tokio_util::codec::LengthDelimitedCodec;
use tokio_util::udp::UdpFramed;

use crate::lattices::SealedSetOfIndexedValues;
use crate::structs::Request;
use crate::test_data::client_class_iter;

pub(crate) async fn ssiv_flow(
    shopping_ssiv: impl Iterator<Item = (usize, SealedSetOfIndexedValues<Request>)> + 'static,
    out_addr: SocketAddr,
    out: SplitSink<UdpFramed<LengthDelimitedCodec>, (Bytes, SocketAddr)>,
) -> Hydroflow {
    let client_class = client_class_iter();

    // First define some shorthand for the merge and bot of this lattice
    let ssiv_merge =
        <SealedSetOfIndexedValues<Request> as Merge<SealedSetOfIndexedValues<Request>>>::merge;
    const SSIV_BOT: fn() -> SealedSetOfIndexedValues<Request> = Default::default;

    // This is the SSIV implementation for a server with interleaved requests from clients.
    // For each Request in "shopping_ssiv" we look up its "client_class" (basic or prime)
    // via a join operator, then we group by (client, class), and for each such pair
    // we grow a SSIV lattice.
    hydroflow_syntax! {
        source_iter(shopping_ssiv) -> [0]lookup_class;
        source_iter(client_class) -> [1]lookup_class;
        lookup_class = join()
          -> map(|(client, (li, class))| ((client, class), li))
          -> group_by(SSIV_BOT, ssiv_merge)
          -> map(|m| (m, out_addr)) -> dest_sink_serde(out);
    }
}
