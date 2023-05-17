use std::net::SocketAddr;

use chrono::prelude::*;
use hydroflow::hydroflow_syntax;
use hydroflow::util::{UdpLinesSink, UdpLinesStream};

use crate::helpers::{deserialize_json, serialize_json};
use crate::protocol::EchoMsg;

pub(crate) async fn run_client(
    outbound: UdpLinesSink,
    inbound: UdpLinesStream,
    server_addr: SocketAddr,
) {
    println!("Attempting to connect to server at {:?}", server_addr);
    println!("Client live!");

    let mut flow = hydroflow_syntax! {
        // take stdin and send to server as an Echo::Message
        source_stdin() -> map(|l| (EchoMsg{ payload: l.unwrap(), ts: Utc::now(), }, server_addr) )
            -> map(|(msg, addr)| (serialize_json(msg), addr))
            -> dest_sink(outbound);

        // receive and print messages
        source_stream(inbound) -> map(deserialize_json)
            -> for_each(|(m, _a): (EchoMsg, SocketAddr) | println!("{:?}", m));
    };

    flow.run_async().await.unwrap();
}
