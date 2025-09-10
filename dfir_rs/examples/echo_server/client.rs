use std::net::SocketAddr;

use chrono::prelude::*;
use dfir_rs::dfir_syntax;
use dfir_rs::util::{bind_udp_bytes, ipv4_resolve};

use crate::Opts;
use crate::helpers::print_graph;
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
    let (outbound, inbound, allocated_client_addr) = bind_udp_bytes(client_addr).await;

    println!(
        "Client is live! Listening on {:?} and talking to server on {:?}",
        allocated_client_addr, server_addr
    );

    // The skeletal DFIR spec for a client.
    let mut flow = dfir_syntax! {

        // Whenever a serialized message is received by the application from a particular address,
        // a (serialized_payload, address_of_sender) pair is emitted by the `inbound` stream.
        //
        // `source_stream_serde` deserializes the payload into a
        // (deserialized_payload, address_of_sender) pair.
        inbound_chan = source_stream_serde(inbound)
            -> map(Result::unwrap);  // If the deserialization was unsuccessful, this line will panic.

        // Mirrors the inbound process on the outbound side.
        // `dest_sink_serde` accepts a (`EchoMsg`, `SocketAddr`) pair and serializes the `EchoMsg`
        // using `serde`, converting it to a (serialized_payload, address_of_receiver) pair.
        // `outbound` transmits the serialized_payload to the address.
        outbound_chan = dest_sink_serde(outbound);

        // Print all messages for debugging purposes.
        inbound_chan
            -> for_each(|(m, a): (EchoMsg, SocketAddr)| println!("{}: Got {:?} from {:?}", Utc::now(), m, a));

        // Consume input from stdin and send to server as an `EchoMsg`.
        source_stdin() // A stream of lines from stdin.
            -> map(|l| (EchoMsg { payload: l.unwrap(), ts: Utc::now(), }, server_addr) )
            -> outbound_chan; // Send it to the server
    };

    // If a graph was requested to be printed, print it.
    if let Some(graph) = opts.graph {
        print_graph(&flow, graph, opts.write_config);
    }

    // Run the client. This is an async function, so we need to await it.
    flow.run().await;
}
