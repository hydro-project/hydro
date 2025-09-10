use std::net::SocketAddr;

use chrono::prelude::*;
use dfir_rs::dfir_syntax;
use dfir_rs::scheduled::graph::Dfir;
use dfir_rs::util::bind_udp_bytes;

use crate::Opts;
use crate::helpers::print_graph;
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
    let (outbound, inbound, actual_server_addr) = bind_udp_bytes(server_address).await;

    println!("Server is live! Listening on {:?}", actual_server_addr);

    // The skeletal DFIR spec for a server.
    let mut flow: Dfir = dfir_syntax! {

        // Whenever a serialized message is received by the application from a particular address,
        // a (serialized_payload, address_of_sender) pair is emitted by the `inbound` stream.
        //
        // `source_stream_serde` deserializes the payload into a
        // (deserialized_payload, address_of_sender) pair.
        inbound_chan = source_stream_serde(inbound) // `source_stream_serde` deserializes the payload
            -> map(Result::unwrap); // If the deserialization was unsuccessful, this line will panic.

        // Mirrors the inbound process on the outbound side.
        // `dest_sink_serde` accepts a (`EchoMsg`, `SocketAddr`) pair and serializes the `EchoMsg`
        // using `serde`, converting it to a (serialized_payload, address_of_receiver) pair.
        // `outbound` transmits the serialized_payload to the address.
        outbound_chan = dest_sink_serde(outbound);

        // Handle the inbound messages.
        inbound_chan
            -> inspect(|(m, a): &(EchoMsg, SocketAddr)| println!("{}: Got {:?} from {:?}", Utc::now(), m, a)) // For debugging purposes.
            -> map(|(EchoMsg { payload, ts: _ }, sender_addr)| (EchoMsg { payload, ts: Utc::now() }, sender_addr) )
            -> [0]outbound_chan;
    };

    // If a graph was requested to be printed, print it.
    if let Some(graph) = opts.graph {
        print_graph(&flow, graph, opts.write_config);
    }

    // Run the server. This is an async function, so we need to await it.
    flow.run().await;
}
