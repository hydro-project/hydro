use hydroflow::hydroflow_syntax;
use hydroflow::scheduled::graph::Hydroflow;
use hydroflow::util::{UdpSink, UdpStream};

use crate::protocol::Message;
use crate::{GraphType, Opts};

pub(crate) async fn run_server(outbound: UdpSink, inbound: UdpStream, opts: Opts) {
    println!("Server live!");

    let mut df: Hydroflow = hydroflow_syntax! {
        // Define shared inbound and outbound channels
        outbound_chan = merge() -> dest_sink_serde(outbound);
        inbound_chan = source_stream_serde(inbound)
            -> map(Result::unwrap)
            ->  demux(|(msg, addr), var_args!(clients, msgs, errs)|
                    match msg {
                        Message::ConnectRequest => clients.give(addr),
                        Message::ChatMsg {..} => msgs.give(msg),
                        _ => errs.give(msg),
                    }
                );
        clients = inbound_chan[clients] -> tee();
        inbound_chan[errs] -> for_each(|m| println!("Received unexpected message type: {:?}", m));

        // Pipeline 1: Acknowledge client connections
        clients[0] -> map(|addr| (Message::ConnectResponse, addr)) -> [0]outbound_chan;

        // Pipeline 2: Broadcast messages to all clients
        broadcast = cross_join() -> [1]outbound_chan;
        inbound_chan[msgs] -> [0]broadcast;
        clients[1] -> [1]broadcast;
    };

    if let Some(graph) = opts.graph {
        let serde_graph = df
            .meta_graph()
            .expect("No graph found, maybe failed to parse.");
        match graph {
            GraphType::Mermaid => {
                println!("{}", serde_graph.to_mermaid());
            }
            GraphType::Dot => {
                println!("{}", serde_graph.to_dot())
            }
            GraphType::Json => {
                unimplemented!();
                // println!("{}", serde_graph.to_json())
            }
        }
    }

    df.run_async().await.unwrap();
}
