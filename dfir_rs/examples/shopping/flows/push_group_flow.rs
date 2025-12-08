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

pub(crate) async fn push_group_flow(
    shopping_ssiv: impl Iterator<Item = (usize, SealedSetOfIndexedValues<Request>)> + 'static,
    out_addr: SocketAddr,
    out: SplitSink<UdpFramed<LengthDelimitedCodec>, (Bytes, SocketAddr)>,
) -> Dfir<'static> {
    let client_class = client_class_iter();

    // First define some shorthand for the merge and bot of this lattice
    let ssiv_merge =
        <SealedSetOfIndexedValues<Request> as Merge<SealedSetOfIndexedValues<Request>>>::merge;
    const SSIV_BOT: fn() -> SealedSetOfIndexedValues<Request> = Default::default;

    // This is the SSIV implementation for a server with interleaved requests from clients,
    // after pushing down the fold_keyed through the join to the left source.
    // For each Request in "shopping_ssiv" we group by client, and for each client
    // we grow a SSIV lattice. Then for each SSIV lattice we look up its client_class
    // (basic or prime) via a join operator, and generate the output.
    dfir_syntax! {
        // push fold_keyed through join
        source_iter(shopping_ssiv) -> fold_keyed::<'static>(SSIV_BOT, |state, delta| { (ssiv_merge)(state, delta); }) -> [0]lookup_class;
        source_iter(client_class) -> [1]lookup_class;
        lookup_class = join::<'static>()
          -> map(|(client, (li, class))| ((client, class), li))
          -> map(|m| (m, out_addr)) -> dest_sink_serde(out);
    }
}
