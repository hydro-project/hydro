use dfir_rs::dfir_syntax;
use dfir_rs::util::{bind_udp_bytes, ipv4_resolve};

use crate::Opts;
use crate::helpers::{parse_command, print_graph};
use crate::protocol::KvsResponse;

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
        // set up channels
        outbound_chan = dest_sink_serde(outbound);
        inbound_chan = source_stream_serde(inbound) -> map(Result::unwrap);

        // read in commands from stdin and forward to server
        source_stdin()
            -> filter_map(|line| parse_command(line.unwrap()))
            -> map(|msg| { (msg, server_addr) })
            -> outbound_chan;

        // print inbound msgs
        inbound_chan -> for_each(|(response, _addr): (KvsResponse, _)| println!("Got a Response: {:?}", response));
    };

    // If a graph was requested to be printed, print it.
    if let Some(graph) = opts.graph {
        print_graph(&flow, graph, opts.write_config);
    }

    flow.run_async().await.unwrap();
}
