use std::net::SocketAddr;

use chrono::prelude::*;
use dfir_rs::dfir_syntax;
use dfir_rs::lattices::{Max, Merge};
use dfir_rs::scheduled::graph::Dfir;
use dfir_rs::util::bind_udp_bytes;

use crate::Opts;
use crate::helpers::print_graph;
use crate::protocol::EchoMsg;

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

    let bot: Max<usize> = Max::new(0);

    println!("Server live!");

    let mut flow: Dfir = dfir_syntax! {
        // Define a shared inbound channel
        inbound_chan = source_stream_serde(inbound) -> map(Result::unwrap) -> tee();

        // Print all messages for debugging purposes
        inbound_chan[print]
            -> for_each(|(msg, addr): (EchoMsg, SocketAddr)| println!("{}: Got {:?} from {:?}", Utc::now(), msg, addr));

        // merge in the msg vc to the local vc
        inbound_chan[merge] -> map(|(msg, _addr): (EchoMsg, SocketAddr)| msg.lamport_clock) -> mergevc;
        mergevc = fold::<'static>(
            || bot,
            |old: &mut Max<usize>, lamport_clock: Max<usize>| {
                let bump = Max::new(old.into_reveal() + 1);
                old.merge(bump);
                old.merge(lamport_clock);
            }
        );


        // Echo back the Echo messages, stamped with updated vc timestamp
        inbound_chan[1] -> map(|(EchoMsg {payload, ..}, addr)| (payload, addr) )
            -> [0]stamped_output;
        mergevc -> [1]stamped_output;
        stamped_output = cross_join::<'tick, 'tick>() -> map(|((payload, addr), the_clock): ((String, SocketAddr), Max<usize>)| (EchoMsg { payload, lamport_clock: the_clock }, addr))
            -> dest_sink_serde(outbound);
    };

    // If a graph was requested to be printed, print it.
    if let Some(graph) = opts.graph {
        print_graph(&flow, graph, opts.write_config);
    }

    // run the server
    flow.run().await;
}
