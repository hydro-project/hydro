use std::net::SocketAddr;

use chrono::prelude::*;
use dfir_rs::dfir_syntax;
use dfir_rs::util::{bind_udp_lines, ipv4_resolve};

use crate::Opts;
use crate::helpers::{deserialize_json, print_graph, serialize_json};
use crate::protocol::EchoMsg;

/// Runs the client. The client is a long-running process that reads stdin, and sends messages that
/// it receives to the server. The client also prints any messages it receives to stdout.
pub(crate) async fn run_client(opts: Opts) {
    // Client listens on a port picked by the OS.
    let client_addr = ipv4_resolve("localhost:0").unwrap();

    // Use the server address that was provided in the command-line arguments, or use the default
    // if one was not provided.
    let server_addr = opts.address;
    assert_ne!(
        0,
        server_addr.port(),
        "Client cannot connect to server port 0."
    );

    // Bind a client-side socket to the requested address and port. The OS will allocate a port and
    // the actual port used will be available in `actual_client_addr`.
    //
    // `outbound` is a `UdpSink`, we use it to send messages. `inbound` is `UdpStream`, we use it
    // to receive messages.
    //
    // bind_udp_bytes is an async function, so we need to await it.
    let (outbound, inbound, allocated_client_addr) = bind_udp_lines(client_addr).await;

    println!(
        "Client is live! Listening on {:?} and talking to server on {:?}",
        allocated_client_addr, server_addr
    );

    // The DFIR spec for the client.
    let mut flow = dfir_syntax! {
        // take stdin and send to server as an Echo::Message
        source_stdin() -> map(|l| (EchoMsg{ payload: l.unwrap(), ts: Utc::now(), }, server_addr) )
            -> map(|(msg, addr)| (serialize_json(msg), addr))
            -> dest_sink(outbound);

        // receive and print messages
        source_stream(inbound) -> map(deserialize_json)
            -> for_each(|(m, _a): (EchoMsg, SocketAddr) | println!("{:?}", m));
    };

    // If a graph was requested to be printed, print it.
    if let Some(graph) = opts.graph {
        print_graph(&flow, graph, opts.write_config);
    }

    // Run the client. This is an async function, so we need to await it.
    flow.run_async().await;
}
