use crate::helpers::print_graph;
use crate::protocol::Message;
use crate::{Opts, DEFAULT_SERVER_ADDRESS};
use chrono::prelude::*;
use hydroflow::hydroflow_syntax;
use hydroflow::util::{bind_udp_bytes, ipv4_resolve};
use std::net::SocketAddr;

/// Runs the client. The client is a long-running process that reads stdin, and sends messages that
/// it receives to the server. The client also prints any messages it receives to stdout.
pub(crate) async fn run_client(opts: Opts) {
    // Client listens on a port picked by the OS.
    let client_addr = ipv4_resolve("localhost:0").unwrap();

    // Use the server address that was provided in the command-line arguments, or use the default
    // if one was not provided.
    let server_addr = opts
        .address
        .unwrap_or_else(|| ipv4_resolve(DEFAULT_SERVER_ADDRESS).unwrap());

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

    // The skeletal hydroflow spec for a client.
    let mut flow = hydroflow_syntax! {

        // Whenever a serialized message is received by the application from a particular address,
        // a (serialized_payload, address_of_sender) pair is emitted by the `inbound` stream.
        //
        // `source_stream_serde` deserializes the payload into a
        // (deserialized_payload, address_of_sender) pair.
        inbound_chan = source_stream_serde(inbound)
            -> map(Result::unwrap);  // If the deserialization was unsuccessful, this line will panic.

        // Mirrors the inbound process on the outbound side.
        // `dest_sink_serde` accepts a (`Message`, `SocketAddr`) pair and serializes the `Message`
        // using `serde`, converting it to a (serialized_payload, address_of_receiver) pair.
        // `outbound` transmits the serialized_payload to the address.
        outbound_chan = dest_sink_serde(outbound);

        // Print all messages for debugging purposes.
        inbound_chan
            -> for_each(|(m, a): (Message, SocketAddr)| println!("{}: Got {:?} from {:?}", Utc::now(), m, a));

        // Consume input from stdin and send to server as Message::Echo
        source_stdin() // A stream of lines from stdin.
            -> map(|l| (Message::Echo{ payload: l.unwrap(), ts: Utc::now(), }, server_addr) )
            -> outbound_chan; // Send it to the server
    };

    // If a graph was requested to be printed, print it.
    if let Some(graph) = opts.graph {
        print_graph(&flow, graph, opts.write_config);
    }

    // Run the client. This is an async function, so we need to await it.
    flow.run_async().await;
}
