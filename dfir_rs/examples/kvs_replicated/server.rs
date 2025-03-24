use dfir_rs::dfir_syntax;
use dfir_rs::scheduled::graph::Dfir;
use dfir_rs::util::bind_udp_bytes;

use crate::Opts;
use crate::helpers::print_graph;
use crate::protocol::{KvsMessage, KvsMessageWithAddr};

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

    let peer_server = opts.peer_address;

    println!(
        "Server is live! Listening on {:?} and talking to peer server {:?}",
        actual_server_addr, peer_server
    );

    let mut flow: Dfir = dfir_syntax! {
        // Setup network channels.
        network_send = union() -> dest_sink_serde(outbound);
        network_recv = source_stream_serde(inbound)
            -> map(Result::unwrap)
            -> inspect(|(msg, addr)| println!("Message received {:?} from {:?}", msg, addr))
            -> map(|(msg, addr)| KvsMessageWithAddr::from_message(msg, addr))
            -> demux_enum::<KvsMessageWithAddr>();
        network_recv[ServerResponse] -> for_each(|(key, value, addr)| eprintln!("Unexpected server response {:?}->{:?} from {:?}", key, value, addr));
        peers = network_recv[PeerJoin] -> map(|(peer_addr,)| peer_addr) -> tee();
        network_recv[ClientPut] -> writes;
        network_recv[PeerGossip] -> writes;
        writes = union() -> tee();
        gets = network_recv[ClientGet];

        // Join PUTs and GETs by key
        writes -> map(|(key, value, _addr)| (key, value)) -> writes_store;
        writes_store = persist::<'static>() -> tee();
        writes_store -> [0]lookup;
        gets -> [1]lookup;
        lookup = join();

        // Send GET responses back to the client address.
        lookup[1]
            -> inspect(|tup| println!("Found a match: {:?}", tup))
            -> map(|(key, (value, client_addr))| (KvsMessage::ServerResponse { key, value }, client_addr))
            -> network_send;

        // Join as a peer if peer_server is set.
        source_iter(peer_server) -> map(|peer_addr| (KvsMessage::PeerJoin, peer_addr)) -> network_send;

        // Peers: When a new peer joins, send them all data.
        writes_store -> [0]peer_join;
        peers -> [1]peer_join;
        peer_join = cross_join()
            -> map(|((key, value), peer_addr)| (KvsMessage::PeerGossip { key, value }, peer_addr))
            -> network_send;

        // Outbound gossip. Send updates to peers.
        peers -> peer_store;
        source_iter(peer_server) -> peer_store;
        peer_store = union() -> persist::<'static>();
        writes -> [0]outbound_gossip;
        peer_store -> [1]outbound_gossip;
        outbound_gossip = cross_join()
            // Don't send gossip back to the sender.
            -> filter(|((_key, _value, writer_addr), peer_addr)| writer_addr != peer_addr)
            -> map(|((key, value, _writer_addr), peer_addr)| (KvsMessage::PeerGossip { key, value }, peer_addr))
            -> network_send;
    };

    // If a graph was requested to be printed, print it.
    if let Some(graph) = opts.graph {
        print_graph(&flow, graph, opts.write_config);
    }

    flow.run_async().await.unwrap();
}
