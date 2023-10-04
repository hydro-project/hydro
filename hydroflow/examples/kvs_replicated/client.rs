use std::net::SocketAddr;

use hydroflow::hydroflow_syntax;
use hydroflow::util::{UdpSink, UdpStream};

use crate::helpers::parse_command;
use crate::protocol::KvsMessage;
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
        inbound_chan = source_stream_serde(inbound) -> map(Result::unwrap);

        // read in commands from stdin and forward to server
        source_stdin()
            -> filter_map(|line| parse_command(line.unwrap()))
            -> map(|msg| { (msg, server_addr) })
            -> outbound_chan;

        // print inbound msgs
        inbound_chan -> for_each(|(response, _addr): (KvsMessage, _)| println!("Got a Response: {:?}", response));
    };

    if let Some(graph) = graph {
        let serde_graph = hf
            .meta_graph()
            .expect("No graph found, maybe failed to parse.");
        match graph {
            GraphType::Mermaid => {
                serde_graph.open_mermaid().unwrap();
            }
            GraphType::Dot => {
                serde_graph.open_dot().unwrap();
            }
            GraphType::Json => {
                unimplemented!();
            }
        }
    }
    hf.run_async().await.unwrap();
}
