use crate::protocol::EchoMsg;
use chrono::prelude::*;
use hydroflow::hydroflow_syntax;
use hydroflow::lattices::ord::Max;
use hydroflow::lattices::Merge;
use hydroflow::scheduled::graph::Hydroflow;
use hydroflow::util::{UdpSink, UdpStream};
use std::net::SocketAddr;

pub(crate) async fn run_server(outbound: UdpSink, inbound: UdpStream) {
    let bot: Max<usize> = Max(0);
    println!("Server live!");

    let mut flow: Hydroflow = hydroflow_syntax! {
        // Define a shared inbound channel
        inbound_chan = source_stream_serde(inbound) -> map(Result::unwrap) -> tee();

        // Print all messages for debugging purposes
        inbound_chan[print]
            -> for_each(|(msg, addr): (EchoMsg, SocketAddr)| println!("{}: Got {:?} from {:?}", Utc::now(), msg, addr));

        // merge in the msg vc to the local vc
        inbound_chan[merge] -> map(|(msg, _addr): (EchoMsg, SocketAddr)| msg.lamport_clock) -> mergevc;
        mergevc = fold::<'static>(
            bot,
            |mut old: Max<usize>, lamport_clock: Max<usize>| {
                let bump = Max(old.0 + 1);
                old.merge(bump);
                old.merge(lamport_clock);
                old
            }
        );


        // Echo back the Echo messages, stamped with updated vc timestamp
        inbound_chan[1] -> map(|(EchoMsg {payload, ..}, addr)| (payload, addr) )
            -> [0]stamped_output;
        mergevc -> [1]stamped_output;
        stamped_output = cross_join::<'tick, 'tick>() -> map(|((payload, addr), the_clock): ((String, SocketAddr), Max<usize>)| (EchoMsg { payload, lamport_clock: the_clock }, addr))
            -> dest_sink_serde(outbound);
    };

    // run the server
    flow.run_async().await;
}
