use std::net::SocketAddr;

use chrono::prelude::*;
use hydroflow::hydroflow_syntax;
use hydroflow::util::{UdpSink, UdpStream};
use lattices::map_union::MapUnionSingletonMap;
use lattices::ord::Max;
use lattices::Merge;

use crate::protocol::{EchoMsg, VecClock};
use crate::{GraphType, Opts};

pub(crate) async fn run_client(
    outbound: UdpSink,
    inbound: UdpStream,
    opts: Opts,
    addr: SocketAddr,
) {
    // server_addr is required for client
    let server_addr = opts.server_addr.expect("Client requires a server address");

    println!("Client live!");

    let mut flow = hydroflow_syntax! {
        // Define shared inbound and outbound channels
        inbound_chan = source_stream_serde(inbound) -> map(Result::unwrap) -> tee();
        outbound_chan = // merge() ->  // commented out since we only use this once in the client template
            dest_sink_serde(outbound);

        // Print all messages for debugging purposes
        inbound_chan[print]
            -> for_each(|(m, a): (EchoMsg, SocketAddr)| println!("{}: Got {:?} from {:?}", Utc::now(), m, a));

        // given the inbound packet, bump the local clock and merge this in
        inbound_chan[merge] -> map(|(msg, _sender): (EchoMsg, SocketAddr)| msg.vc) -> [net]mergevc;
        mergevc = merge() -> fold::<'static> (VecClock::default(), |mut old, vc| {
                    let my_addr = format!("{:?}", addr);
                    let bump = MapUnionSingletonMap::new_from((my_addr.clone(), Max::new(old.0[&my_addr].0 + 1)));
                    old.merge(bump);
                    old.merge(vc);
                    old
            }
        );

        // for each input from stdin, bump the local vc and send it to the server with the (post-bump) local vc
        input = source_stdin() -> map(|l| l.unwrap()) -> tee();
        input[tick] -> map(|_| VecClock::default()) -> [input]mergevc;

        // stamp each input with the latest local vc (as of this tick!)
        input[send] -> [0]stamped_output;
        mergevc[useful] -> [1]stamped_output;
        stamped_output = cross_join::<'tick, 'tick>() -> map(|(l, the_vc)| (EchoMsg { payload: l, vc: the_vc }, server_addr));

        // and send to server
        stamped_output[send] -> outbound_chan;
    };

    if let Some(graph) = opts.graph {
        let serde_graph = flow
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

    flow.run_async().await.unwrap();
}
