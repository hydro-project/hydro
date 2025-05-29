use std::net::SocketAddr;

use chrono::prelude::*;
use dfir_rs::dfir_syntax;
use dfir_rs::scheduled::graph::Dfir;
use dfir_rs::util::bind_udp_lines;

use crate::Opts;
use crate::helpers::{deserialize_json, print_graph, serialize_json};
use crate::protocol::EchoMsg;

/// Runs the server. The server is a long-running process that listens for messages and echoes
/// them back the client.
pub(crate) async fn run_server(opts: Opts) {
    // If a server address & port are provided as command-line inputs, use those, else use the
    // default.
    let server_address = opts.address;

    println!("Starting server on {:?}", server_address);

    // Bind a server-side socket to requested address and port. If "0" was provided as the port, the
    // OS will allocate a port and the actual port used will be available in `actual_server_addr`.
    //
    // `outbound` is a `UdpSink`, we use it to send messages. `inbound` is `UdpStream`, we use it
    // to receive messages.
    //
    // This is an async function, so we need to await it.
    let (outbound, inbound, actual_server_addr) = bind_udp_lines(server_address).await;

    println!("Server is live! Listening on {:?}", actual_server_addr);

    // The skeletal DFIR spec for a server.
    let mut flow: Dfir = dfir_syntax! {
        // Inbound channel sharing
        inbound_chan = source_stream(inbound) -> map(deserialize_json) -> tee();

        // Logic
        inbound_chan[0] -> for_each(|(m, a): (EchoMsg, SocketAddr)| println!("Got {:?} from {:?}", m, a));
        inbound_chan[1] -> map(|(EchoMsg { payload, .. }, addr)| (EchoMsg { payload, ts: Utc::now() }, addr))
            -> map(|(m, a)| (serialize_json(m), a))
            -> dest_sink(outbound);
    };

    // If a graph was requested to be printed, print it.
    if let Some(graph) = opts.graph {
        print_graph(&flow, graph, opts.write_config);
    }

    // Run the server. This is an async function, so we need to await it.
    flow.run_async().await;
}
