use std::net::SocketAddr;

use chrono::prelude::*;
use dfir_rs::dfir_syntax;
use dfir_rs::util::{bind_udp_bytes, ipv4_resolve};
use lattices::map_union::MapUnionSingletonMap;
use lattices::{Max, Merge};

use crate::Opts;
use crate::helpers::print_graph;
use crate::protocol::{EchoMsg, VecClock};

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

    let mut flow = dfir_syntax! {
        // Define shared inbound and outbound channels
        inbound_chan = source_stream_serde(inbound) -> map(Result::unwrap) -> tee();
        outbound_chan = // union() ->  // commented out since we only use this once in the client template
            dest_sink_serde(outbound);

        // Print all messages for debugging purposes
        inbound_chan[print]
            -> for_each(|(m, a): (EchoMsg, SocketAddr)| println!("{}: Got {:?} from {:?}", Utc::now(), m, a));

        // given the inbound packet, bump the local clock and merge this in
        inbound_chan[merge] -> map(|(msg, _sender): (EchoMsg, SocketAddr)| msg.vc) -> [net]mergevc;
        mergevc = union() -> fold::<'static> (VecClock::default, |old: &mut VecClock, vc| {
                    let my_addr = format!("{:?}", allocated_client_addr);
                    let bump = MapUnionSingletonMap::new_from((my_addr.clone(), Max::new(old.as_reveal_mut().entry(my_addr).or_insert(Max::new(0)).into_reveal() + 1)));
                    old.merge(bump);
                    old.merge(vc);
            }
        );

        // for each input from stdin, bump the local vc and send it to the server with the (post-bump) local vc
        input = source_stdin() -> map(|l| l.unwrap()) -> tee();
        input[tick] -> map(|_| VecClock::default()) -> [input]mergevc;

        // stamp each input with the latest local vc (as of this tick!)
        input[send] -> [0]stamped_output;
        mergevc[useful] -> [1]stamped_output;
        stamped_output = cross_join::<'tick, 'tick>() -> map(|(l, the_vc)| (EchoMsg { payload: l, vc: the_vc }, server_addr));

        // and send to server
        stamped_output[send] -> outbound_chan;
    };

    // If a graph was requested to be printed, print it.
    if let Some(graph) = opts.graph {
        print_graph(&flow, graph, opts.write_config);
    }

    flow.run().await.unwrap();
}
