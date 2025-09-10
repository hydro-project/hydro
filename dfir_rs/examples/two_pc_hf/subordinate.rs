use std::net::SocketAddr;

use dfir_rs::dfir_syntax;
use dfir_rs::scheduled::graph::Dfir;
use dfir_rs::util::{UdpSink, UdpStream};

use crate::helpers::decide;
use crate::protocol::{CoordMsg, MsgType, SubordResponse};
use crate::{Addresses, Opts};

pub(crate) async fn run_subordinate(outbound: UdpSink, inbound: UdpStream, opts: Opts) {
    println!("Subordinate live!");

    let path = opts.path();
    let mut df: Dfir = dfir_syntax! {
        // Outbound address
        server_addr = source_json(path)
            -> map(|json: Addresses| json.coordinator)
            -> map(|s| s.parse::<SocketAddr>().unwrap())
            -> inspect(|coordinator| println!("Coordinator: {}", coordinator));
        server_addr_join = cross_join::<'tick, 'static>();
        server_addr -> [1]server_addr_join;

        // set up channels
        outbound_chan = union() -> [0]server_addr_join -> tee();
        outbound_chan[0] -> dest_sink_serde(outbound);
        inbound_chan = source_stream_serde(inbound) -> map(Result::unwrap) -> map(|(m, _a)| m) -> tee();
        msgs = inbound_chan[0] ->  demux(|m:CoordMsg, var_args!(prepares, p2, ends, errs)| match m.mtype {
            MsgType::Prepare => prepares.give(m),
            MsgType::Abort => p2.give(m),
            MsgType::Commit => p2.give(m),
            MsgType::End => ends.give(m),
            _ => errs.give(m),
        });
        msgs[errs] -> for_each(|m| println!("Received unexpected message type: {:?}", m));

        // we log all messages (in this prototype we just print)
        inbound_chan[1] -> for_each(|m| println!("Received {:?}", m));
        outbound_chan[1] -> for_each(|m| println!("Sending {:?}", m));


        // handle p1 message: choose vote and respond
        // in this prototype we choose randomly whether to abort via decide()
        report_chan = msgs[prepares] -> map(|m: CoordMsg| SubordResponse {
            xid: m.xid,
            mtype: if decide(80) { MsgType::Commit } else { MsgType::Abort }
        });
        report_chan -> [0]outbound_chan;

        // handle p2 message: acknowledge (and print)
        p2_response = map(|(m, t)| SubordResponse {
            xid: m.xid,
            mtype: t,
        }) -> [1]outbound_chan;
        msgs[p2] -> map(|m:CoordMsg| (m, MsgType::AckP2)) -> p2_response;

        // handle end message: acknowledge (and print)
        msgs[ends] -> map(|m:CoordMsg| SubordResponse {
            xid: m.xid,
            mtype: MsgType::Ended,
        }) -> [2]outbound_chan;
    };

    #[cfg(feature = "debugging")]
    if let Some(graph) = opts.graph {
        let serde_graph = df
            .meta_graph()
            .expect("No graph found, maybe failed to parse.");
        serde_graph.open_graph(graph, opts.write_config).unwrap();
    }

    df.run().await.unwrap();
}
