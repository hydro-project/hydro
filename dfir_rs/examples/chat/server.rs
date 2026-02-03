use dfir_rs::dfir_syntax;
use dfir_rs::scheduled::graph::Dfir;
use dfir_rs::util::bind_udp_bytes;

use crate::protocol::{Message, MessageWithAddr};
use crate::{Opts, default_server_address};

pub(crate) async fn run_server(opts: Opts) {
    println!("Server live!");

    // If a server address & port are provided as command-line inputs, use those, else use the
    // default.
    let server_address = opts.address.unwrap_or_else(default_server_address);

    println!("Starting server on {:?}", server_address);

    let (outbound, inbound, actual_server_addr) = bind_udp_bytes(server_address).await;

    println!("Server is live! Listening on {:?}", actual_server_addr);

    let mut hf: Dfir = dfir_syntax! {
        // Define shared inbound and outbound channels
        outbound_chan = union() -> dest_sink_serde(outbound);
        inbound_chan = source_stream_serde(inbound)
            -> map(Result::unwrap)
            -> map(|(msg, addr)| MessageWithAddr::from_message(msg, addr))
            -> demux_enum::<MessageWithAddr>();
        clients = inbound_chan[ConnectRequest] -> map(|(addr,)| addr) -> tee();
        inbound_chan[ConnectResponse] -> for_each(|(addr,)| println!("Received unexpected `ConnectResponse` as server from addr {}.", addr));

        // Pipeline 1: Acknowledge client connections
        clients[0] -> map(|addr| (Message::ConnectResponse, addr)) -> [0]outbound_chan;

        // Pipeline 2: Broadcast messages to all clients
        inbound_chan[ChatMsg] -> map(|(_addr, nickname, message, ts)| Message::ChatMsg { nickname, message, ts }) -> [0]broadcast;
        clients[1] -> [1]broadcast;
        broadcast = cross_join::<'tick, 'static>() -> [1]outbound_chan;
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
