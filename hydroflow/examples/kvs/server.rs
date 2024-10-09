use hydroflow::hydroflow_syntax;
use hydroflow::scheduled::graph::Hydroflow;
use hydroflow::util::{UdpSink, UdpStream};

use crate::protocol::{KvsMessageWithAddr, KvsResponse};
use crate::Opts;

pub(crate) async fn run_server(outbound: UdpSink, inbound: UdpStream, opts: Opts) {
    println!("Server live!");

    let mut hf: Hydroflow = hydroflow_syntax! {
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

    #[cfg(feature = "debugging")]
    if let Some(graph) = opts.graph {
        let serde_graph = hf
            .meta_graph()
            .expect("No graph found, maybe failed to parse.");
        serde_graph.open_graph(graph, opts.write_config).unwrap();
    }
    let _ = opts;

    hf.run_async().await.unwrap();
}
