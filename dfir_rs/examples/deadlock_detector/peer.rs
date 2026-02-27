use std::convert::identity;
use std::net::SocketAddr;

use dfir_rs::dfir_syntax;
use dfir_rs::scheduled::graph::Dfir;
use futures::StreamExt;
use tokio::io::AsyncBufReadExt;
use tokio::net::UdpSocket;
use tokio_stream::wrappers::LinesStream;

use crate::Opts;
use crate::helpers::{deserialize_msg, format_cycle, gen_bool, parse_edge, serialize_msg};
use crate::protocol::{Message, SimplePath};

pub(crate) async fn run_detector(opts: Opts, peer_list: Vec<String>) {
    // setup message send/recv ports
    let server_socket = UdpSocket::bind(("127.0.0.1", opts.port)).await.unwrap();
    let (outbound, inbound, _) = dfir_rs::util::udp_lines(server_socket);

    // We provide a command line for users to type waits-for edges (u32,u32).
    let reader = tokio::io::BufReader::new(tokio::io::stdin());
    let stdin_lines = LinesStream::new(reader.lines()).fuse();

    let mut hf: Dfir = dfir_syntax! {
        // fetch peers from file, convert ip:port to a SocketAddr, and tee
        peers = source_iter(peer_list)
            -> map(|s| s.parse::<SocketAddr>().unwrap())
            -> tee();

        // set up channels
        outbound_chan = map(|(m,a)| (serialize_msg(m), a)) -> dest_sink(outbound);
        inbound_chan = source_stream(inbound)
            -> filter(|msg| {
                // For some reason Windows generates connection reset errors on UDP sockets, even though UDP has no sessions.
                // This code filters them out.
                // `Os { code: 10054, kind: ConnectionReset, message: "An existing connection was forcibly closed by the remote host."`
                // https://stackoverflow.com/questions/10332630/connection-reset-on-receiving-packet-in-udp-server
                // TODO(mingwei): Clean this up, figure out how to configure windows UDP sockets correctly.
                if let Err(tokio_util::codec::LinesCodecError::Io(io_err)) = msg {
                    io_err.kind() != std::io::ErrorKind::ConnectionReset
                } else {
                    true
                }
            })
            -> map(deserialize_msg::<Message>);

        // setup gossip channel to all peers. gen_bool chooses True with the odds passed in.
        gossip_join = cross_join::<'tick>()
            -> filter(|_| gen_bool(0.8)) -> outbound_chan;
        gossip = map(identity) -> persist::<'static>() -> [0]gossip_join;
        peers[1] -> persist::<'static>() -> [1]gossip_join;
        peers[2] -> for_each(|s| println!("Peer: {:?}", s));

        // prompt for input
        source_iter([()]) -> for_each(|_s| println!("Type in an edge as a tuple of two integers (x,y): "));
        // read in edges from stdin
        new_edges = source_stream(stdin_lines)
            -> filter_map(|line| {
                parse_edge(line.unwrap())});

        // persist an edges set
        edges = union() -> tee();
        edges[0] -> defer_tick() -> [1]edges;

        // add new edges locally
        new_edges -> [0]edges;

        // gossip all edges
        edges[1] -> fold::<'static>(Message::new, |m: &mut Message, edge| {
            m.edges.insert(edge);
        }) -> gossip;


        // Form local transitive closure (from scratch) and check for cycles.
        // This is datalog TC, extended with a SimplePath vector being accumulated along the way.
        // Rule 1: add inbound edges to paths
        // paths(from, to, [from, to] :- edges(from, to))
        inbound_chan[4] -> flat_map(|m| m.edges) -> [2]edges;
        paths = union();
        edges[2] -> map(|(from, to)| (from, to, SimplePath::new(vec![from, to]))) -> [0]paths;

        // Rule 2: form new_paths from the join of acyclic paths and edges
        // new_paths(from, to, path.append(to)) :- paths(from, mid, path), edges(mid, to), paths.cycle() == false
        new_paths = join::<'static>() -> map(|(_mid, ((from, mut path), to))| {
            path.push(to);
            (from, to, path)
         }) -> tee();
        paths -> filter_map(|(from, to, path)| {
            if path.cycle() {None} // don't extend self-loops
            else {Some((to, (from, path)))}
        }) -> [0]new_paths;
        edges[3] -> map(|(from, to)| (from, to)) -> [1]new_paths;
        // stdio(from, to, path) :- new_paths(from, to, path)
        new_paths[0]
            -> filter_map(|(from, to, path): (u32, u32, SimplePath<u32>)| if from == to {Some(path.canonical())} else {None})
            -> unique::<'static>()
            -> for_each(|path: Vec<u32>| {
                    println!("path found: {}", format_cycle(path));
               });
        // paths :- new_paths
        new_paths[1] -> [1]paths;

    };

    #[cfg(feature = "debugging")]
    if let Some(graph) = opts.graph {
        let serde_graph = hf
            .meta_graph()
            .expect("No graph found, maybe failed to parse.");
        serde_graph.open_graph(graph, opts.write_config).unwrap();
    }

    let None = hf.run().await;
}
