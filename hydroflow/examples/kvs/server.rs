use hydroflow::hydroflow_syntax;
use hydroflow::scheduled::graph::Hydroflow;
use hydroflow::util::{UdpSink, UdpStream};

use crate::protocol::KVSMessage;
use crate::GraphType;

pub(crate) async fn run_server(outbound: UdpSink, inbound: UdpStream, graph: Option<GraphType>) {
    println!("Server live!");

    let mut df: Hydroflow = hydroflow_syntax! {
        // NW channels
        outbound_chan = merge() -> dest_sink_serde(outbound);
        inbound_chan = source_stream_serde(inbound)
            -> map(Result::unwrap)
            -> demux(|(m, a), var_args!(puts, gets, errs)| match m {
                    KVSMessage::Put {..} => puts.give((m, a)),
                    KVSMessage::Get {..} => gets.give((m, a)),
                    _ => errs.give((m, a)),
            });
        puts = inbound_chan[puts] -> tee();
        gets = inbound_chan[gets] -> tee();
        inbound_chan[errs] -> for_each(|(m, a)| println!("Received unexpected message type {:?} from {:?}", m, a));

        puts[0] -> for_each(|(m, a)| println!("Got a Put {:?} from {:?}", m, a));
        gets[0] -> for_each(|(m, a)| println!("Got a Get {:?} from {:?}", m, a));

        parsed_puts = puts[1] -> filter_map(|(m, a)| {
            match m {
                KVSMessage::Put{key, value} => Some((key, value, a)),
                _ => None }
            }) -> tee();
        parsed_gets = gets[1] -> filter_map(|(m, a)| {
            match m {
                KVSMessage::Get{key} => Some((key, a)),
                _ => None }
            });

        // ack puts
        parsed_puts[0] -> map(| (key, value, client) |
                                (KVSMessage::Response{key, value}, client))
            -> [0]outbound_chan;

        // join PUTs and GETs by key
        lookup = join()->tee();
        parsed_puts[1] -> map(|(key, value, _)| (key, value)) -> [0]lookup;
        parsed_gets -> [1]lookup;
        lookup[0] -> for_each(|t| println!("Found a match: {:?}", t));

        // send lookup responses back to the client address from the GET
        lookup[1] -> map(|(key, (value, client))| (KVSMessage::Response{key, value}, client)) -> [1]outbound_chan;
    };

    if let Some(graph) = graph {
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
