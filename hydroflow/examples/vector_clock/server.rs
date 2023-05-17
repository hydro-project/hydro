use std::net::SocketAddr;

use chrono::prelude::*;
use hydroflow::hydroflow_syntax;
use hydroflow::scheduled::graph::Hydroflow;
use hydroflow::util::{UdpSink, UdpStream};
use lattices::map_union::MapUnionSingletonMap;
use lattices::ord::Max;
use lattices::Merge;

use crate::protocol::{EchoMsg, VecClock};

pub(crate) async fn run_server(outbound: UdpSink, inbound: UdpStream, opts: crate::Opts) {
    println!("Server live!");

    let mut flow: Hydroflow = hydroflow_syntax! {
        // Define a shared inbound channel
        inbound_chan = source_stream_serde(inbound) -> map(Result::unwrap) -> tee();

        // Print all messages for debugging purposes
        inbound_chan[print]
            -> for_each(|(msg, addr): (EchoMsg, SocketAddr)| println!("{}: Got {:?} from {:?}", Utc::now(), msg, addr));

        // merge in the msg vc to the local vc
        inbound_chan[merge] -> map(|(msg, _addr): (EchoMsg, SocketAddr)| msg.vc) -> mergevc;
        mergevc = fold::<'static> (VecClock::default(), |mut old, vc| {
                let my_addr = format!("{:?}", opts.addr.unwrap());
                let bump = MapUnionSingletonMap::new_from((my_addr.clone(), Max::new(old.0[&my_addr].0 + 1)));
                old.merge(bump);
                old.merge(vc);
                old
            }
        );


        // Echo back the Echo messages, stamped with updated vc timestamp
        inbound_chan[1] -> map(|(EchoMsg {payload, ..}, addr)| (payload, addr) )
            -> [0]stamped_output;
        mergevc -> [1]stamped_output;
        stamped_output = cross_join::<'tick, 'tick>() -> map(|((payload, addr), vc)| (EchoMsg { payload, vc }, addr))
            -> dest_sink_serde(outbound);
    };

    // run the server
    flow.run_async().await;
}
