use std::net::SocketAddr;

use hydroflow::hydroflow_syntax;
use hydroflow::scheduled::graph::Hydroflow;
use hydroflow::util::UdpStream;

use crate::lattices::{BoundedPrefix, SealedSetOfIndexedValues};
use crate::structs::{ClientClass, LineItem, Request};

pub(crate) async fn listener_flow(
    tuple_input: UdpStream,
    bp_input: UdpStream,
    ssiv_input: UdpStream,
) -> Hydroflow {
    // Simply print what we receive.
    hydroflow_syntax! {
        source_stream_serde(tuple_input)
            -> map(Result::unwrap)
            -> for_each(|(cart, _): (((usize, ClientClass), Vec<LineItem>), SocketAddr)| println!("{:?}", cart));
        source_stream_serde(bp_input)
            -> map(Result::unwrap)
            -> for_each(|(cart, _): (((usize, ClientClass), BoundedPrefix<Request>), SocketAddr)| println!("{:?}", cart));
        source_stream_serde(ssiv_input)
            -> map(Result::unwrap)
            -> for_each(|(cart, _): (((usize, ClientClass), SealedSetOfIndexedValues<Request>), SocketAddr)| println!("{:?}", cart));
    }
}
