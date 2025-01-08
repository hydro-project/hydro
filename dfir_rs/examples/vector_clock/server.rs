use std::net::SocketAddr;

use chrono::prelude::*;
use dfir_rs::dfir_syntax;
use dfir_rs::scheduled::graph::Dfir;
use dfir_rs::util::{UdpSink, UdpStream};
use lattices::map_union::MapUnionSingletonMap;
use lattices::{Max, Merge};

use crate::protocol::{EchoMsg, VecClock};

pub(crate) async fn run_server(outbound: UdpSink, inbound: UdpStream, opts: crate::Opts) {
    println!("Server live!");

    let mut flow: Dfir = dfir_syntax! {
        // Define a shared inbound channel
        inbound_chan = source_stream_serde(inbound) -> map(Result::unwrap) -> tee();

        // Print all messages for debugging purposes
        inbound_chan[print]
            -> for_each(|(msg, addr): (EchoMsg, SocketAddr)| println!("{}: Got {:?} from {:?}", Utc::now(), msg, addr));

        // merge in the msg vc to the local vc
        inbound_chan[merge] -> map(|(msg, _addr): (EchoMsg, SocketAddr)| msg.vc) -> mergevc;
        mergevc = fold::<'static> (VecClock::default, |old: &mut VecClock, vc| {
                let my_addr = format!("{:?}", opts.addr.unwrap());
                let bump = MapUnionSingletonMap::new_from((my_addr.clone(), Max::new(old.as_reveal_mut().entry(my_addr).or_insert(Max::new(0)).into_reveal() + 1)));
                old.merge(bump);
                old.merge(vc);
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
