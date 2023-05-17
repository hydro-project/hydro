use std::net::SocketAddr;

use hydroflow::hydroflow_syntax;
use hydroflow::util::{UdpSink, UdpStream};

use crate::helpers::parse_command;
use crate::protocol::KVSMessage;
use crate::GraphType;

pub(crate) async fn run_client(
    outbound: UdpSink,
    inbound: UdpStream,
    server_addr: SocketAddr,
    graph: Option<GraphType>,
) {
    println!("Client live!");

    let mut hf = hydroflow_syntax! {
        // set up channels
        outbound_chan = dest_sink_serde(outbound);
        inbound_chan = source_stream_serde(inbound) -> map(Result::unwrap) -> map(|(m, _a)| m) -> demux(|m, var_args!(resps, errs)| match m {
            KVSMessage::Response {..} => resps.give(m),
            _ => errs.give(m),
        });
        inbound_chan[errs] -> for_each(|m| println!("Received unexpected message type: {:?}", m));

        // read in commands from stdin and forward to server
        source_stdin()
            -> filter_map(|line| parse_command(line.unwrap()))
            -> map(|msg| { (msg, server_addr) })
            -> outbound_chan;

        // print inbound msgs
        inbound_chan[resps] -> for_each(|m| println!("Got a Response: {:?}", m));
    };

    if let Some(graph) = graph {
        let serde_graph = hf
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
    hf.run_async().await.unwrap();
}
