use std::net::SocketAddr;

use dfir_rs::dfir_syntax;
use dfir_rs::scheduled::graph::Dfir;
use dfir_rs::util::UdpStream;

use crate::lattices::{BoundedPrefix, SealedSetOfIndexedValues};
use crate::structs::{ClientClass, LineItem, Request};

pub(crate) async fn listener_flow(
    tuple_input: UdpStream,
    bp_input: UdpStream,
    ssiv_input: UdpStream,
) -> Dfir<'static> {
    // Simply print what we receive.
    dfir_syntax! {
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
