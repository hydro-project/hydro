use std::net::SocketAddr;

use bytes::Bytes;
use dfir_rs::dfir_syntax;
use dfir_rs::lattices::Merge;
use dfir_rs::scheduled::graph::Dfir;
use futures::stream::SplitSink;
use tokio_util::codec::LengthDelimitedCodec;
use tokio_util::udp::UdpFramed;

use crate::lattices::BoundedPrefix;
use crate::structs::Request;
use crate::test_data::client_class_iter;

pub(crate) async fn bp_flow(
    shopping_bp: impl Iterator<Item = (usize, BoundedPrefix<Request>)> + 'static,
    out_addr: SocketAddr,
    out: SplitSink<UdpFramed<LengthDelimitedCodec>, (Bytes, SocketAddr)>,
) -> Dfir<'static> {
    let client_class = client_class_iter();

    // First define some shorthand for the merge and bot of this lattice
    let bp_merge = <BoundedPrefix<Request> as Merge<BoundedPrefix<Request>>>::merge;
    const BP_BOT: fn() -> BoundedPrefix<Request> = Default::default;

    // This is the BP case for a server with interleaved requests from clients.
    // For each Request in "shopping_bp" we look up its "client_class" (basic or prime)
    // via a join operator, then we group by (client, class), and for each such pair
    // we grow a BP lattice.
    dfir_syntax! {
        // BP, two customer classes
        source_iter(shopping_bp) -> [0]lookup_class;
        source_iter(client_class) -> [1]lookup_class;
        lookup_class = join::<'static>()
          -> map(|(client, (li, class))| ((client, class), li))
          -> fold_keyed::<'static>(BP_BOT, |state, delta| { (bp_merge)(state, delta); })
          -> map(|m| (m, out_addr)) -> dest_sink_serde(out);
    }
}
