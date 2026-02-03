use dfir_rs::dfir_syntax;
use dfir_rs::scheduled::graph::Dfir;
use dfir_rs::util::bind_udp_bytes;

use crate::Opts;
use crate::helpers::print_graph;
use crate::protocol::{KvsMessageWithAddr, KvsResponse};

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

    let mut flow: Dfir = dfir_syntax! {
        // Setup network channels.
        network_send = dest_sink_serde(outbound);
        network_recv = source_stream_serde(inbound)
            -> map(Result::unwrap)
            -> inspect(|(msg, addr)| println!("Message received {:?} from {:?}", msg, addr))
            -> map(|(msg, addr)| KvsMessageWithAddr::from_message(msg, addr))
            -> demux_enum::<KvsMessageWithAddr>();
        puts = network_recv[Put];
        gets = network_recv[Get];

        /* DIFFERENCE HERE: SEE README.md */
        // Join PUTs and GETs by key, persisting the PUTs.
        puts -> map(|(key, value, _addr)| (key, value)) -> [0]lookup;
        gets -> [1]lookup;
        lookup = join::<'static, 'tick>();

        // Send GET responses back to the client address.
        lookup
            -> inspect(|tup| println!("Found a match: {:?}", tup))
            -> map(|(key, (value, client_addr))| (KvsResponse { key, value }, client_addr))
            -> network_send;
    };

    // If a graph was requested to be printed, print it.
    if let Some(graph) = opts.graph {
        print_graph(&flow, graph, opts.write_config);
    }

    let None = flow.run().await;
}
