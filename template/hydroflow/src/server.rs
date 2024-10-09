use crate::helpers::print_graph;
use crate::protocol::Message;
use crate::DEFAULT_SERVER_ADDRESS;
use chrono::prelude::*;
use hydroflow::hydroflow_syntax;
use hydroflow::scheduled::graph::Hydroflow;
use hydroflow::util::{bind_udp_bytes, ipv4_resolve};
use std::net::SocketAddr;

/// Runs the server. The server is a long-running process that listens for messages and echoes
/// them back the client.
pub(crate) async fn run_server(opts: crate::Opts) {
    // If a server address & port are provided as command-line inputs, use those, else use the
    // default.
    let server_address = opts
        .address
        .unwrap_or_else(|| ipv4_resolve(DEFAULT_SERVER_ADDRESS).unwrap());

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

    // The skeletal hydroflow spec for a server.
    let mut flow: Hydroflow = hydroflow_syntax! {

        // Whenever a serialized message is received by the application from a particular address,
        // a (serialized_payload, address_of_sender) pair is emitted by the `inbound` stream.
        //
        // `source_stream_serde` deserializes the payload into a
        // (deserialized_payload, address_of_sender) pair.
        inbound_chan = source_stream_serde(inbound) // `source_stream_serde` deserializes the payload
            -> map(Result::unwrap); // If the deserialization was unsuccessful, this line will panic.

        // Mirrors the inbound process on the outbound side.
        // `dest_sink_serde` accepts a (`Message`, `SocketAddr`) pair and serializes the `Message`
        // using `serde`, converting it to a (serialized_payload, address_of_receiver) pair.
        // `outbound` transmits the serialized_payload to the address.
        outbound_chan = union() -> dest_sink_serde(outbound);

        // Demux and destructure the inbound messages into separate streams
        inbound_demuxed = inbound_chan
            -> inspect(|(m, a): &(Message, SocketAddr)| println!("{}: Got {:?} from {:?}", Utc::now(), m, a)) // For debugging purposes.
            -> demux(|(msg, addr), var_args!(echo, heartbeat, errs)|
                match msg {
                        Message::Echo {payload, ..} => echo.give((payload, addr)),
                        Message::Heartbeat => heartbeat.give(addr),
                        _ => errs.give((msg, addr)),
                    }
                );

        // Echo a response back to the sender of the echo request.
        inbound_demuxed[echo]
            -> map(|(payload, sender_addr)| (Message::Echo { payload, ts: Utc::now() }, sender_addr) )
            -> [0]outbound_chan;

        // Respond to Heartbeat messages
        inbound_demuxed[heartbeat]
            -> map(|addr| (Message::HeartbeatAck, addr))
            -> [1]outbound_chan;

        // Print unexpected messages
        inbound_demuxed[errs]
            -> for_each(|(msg, addr)| println!("Received unexpected message type: {:?} from {:?}", msg, addr));

    };

    // If a graph was requested to be printed, print it.
    if let Some(graph) = opts.graph {
        print_graph(&flow, graph, opts.write_config);
    }

    // Run the server. This is an async function, so we need to await it.
    flow.run_async().await;
}
