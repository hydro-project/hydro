use hydroflow::hydroflow_syntax;
use hydroflow::scheduled::graph::Hydroflow;
use hydroflow::util::{bind_udp_bytes, bind_websocket};

use crate::protocol::{Message, MessageWithAddr};
use crate::{default_server_address, Opts};

pub(crate) async fn run_server(opts: Opts) {
    println!("Server live!");

    // If a server address & port are provided as command-line inputs, use those, else use the
    // default.
    let server_address = opts.address.unwrap_or_else(default_server_address);

    println!("Starting server on {:?}", server_address);

    let (outbound, inbound, actual_server_addr) = bind_websocket(server_address).await.unwrap();

    println!("Server is live! Listening on {:?}", actual_server_addr);

    let mut hf: Hydroflow = hydroflow_syntax! {
        // Define shared inbound and outbound channels
        inbound_chan = source_stream(inbound)
            -> map(Result::unwrap)
            -> tee();

        inbound_chan -> map(|(msg, addr)| addr) -> [1]broadcast;
        inbound_chan -> map(|(msg, addr)| msg) -> [0]broadcast;

        broadcast = cross_join::<'tick, 'static>() -> dest_sink(outbound);
    };

    #[cfg(feature = "debugging")]
    if let Some(graph) = opts.graph {
        let serde_graph = hf
            .meta_graph()
            .expect("No graph found, maybe failed to parse.");
        serde_graph.open_graph(graph, opts.write_config).unwrap();
    }

    hf.run_async().await.unwrap();
}
