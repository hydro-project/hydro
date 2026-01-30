use std::net::SocketAddr;

use dfir_rs::dfir_syntax;
use dfir_rs::scheduled::graph::Dfir;
use dfir_rs::util::{UdpSink, UdpStream};

use crate::helpers::decide;
use crate::protocol::Msg;
use crate::{Addresses, Opts};

pub(crate) async fn run_subordinate(outbound: UdpSink, inbound: UdpStream, opts: Opts) {
    println!("Subordinate live!");

    let path = opts.path();
    let mut df: Dfir = dfir_syntax! {
        // Outbound address
        server_addr = source_json::<Addresses>(path)
            -> map(|json| json.coordinator)
            -> map(|s| s.parse::<SocketAddr>().unwrap())
            -> inspect(|coordinator| println!("Coordinator: {}", coordinator));
        server_addr_join = cross_join::<'tick, 'static>();
        server_addr -> [1]server_addr_join;

        // set up channels
        outbound_chan = union() -> [0]server_addr_join -> tee();
        outbound_chan[0] -> dest_sink_serde(outbound);
        inbound_chan = source_stream_serde(inbound) -> map(Result::unwrap) -> map(|(m, _a)| m) -> tee();
        msgs = inbound_chan[0] -> demux_enum::<Msg>();

        msgs[AckP2] -> errs;
        msgs[Ended] -> errs;
        errs = union() -> for_each(|m| println!("Received unexpected message type: {:?}", m));

        // we log all messages (in this prototype we just print)
        inbound_chan[1] -> for_each(|m| println!("Received {:?}", m));
        outbound_chan[1] -> for_each(|m| println!("Sending {:?}", m));


        // handle p1 message: choose vote and respond
        // in this prototype we choose randomly whether to abort via decide()
        report_chan = msgs[Prepare] -> map(|(xid,)| {
            if decide(80) { Msg::Commit(xid) } else { Msg::Abort(xid) }
        });
        report_chan -> [0]outbound_chan;

        // handle p2 message: acknowledge
        msgs[Abort] -> p2_response;
        msgs[Commit] -> p2_response;
        p2_response = union() -> map(|(xid,)| Msg::AckP2(xid)) -> [1]outbound_chan;

        // handle end message: acknowledge (and print)
        msgs[End] -> map(|(xid,)| Msg::Ended(xid)) -> [2]outbound_chan;
    };

    #[cfg(feature = "debugging")]
    if let Some(graph) = opts.graph {
        let serde_graph = df
            .meta_graph()
            .expect("No graph found, maybe failed to parse.");
        serde_graph.open_graph(graph, opts.write_config).unwrap();
    }

    let None = df.run().await;
}
