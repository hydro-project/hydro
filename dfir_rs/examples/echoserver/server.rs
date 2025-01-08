use std::net::SocketAddr;

use chrono::prelude::*;
use dfir_rs::dfir_syntax;
use dfir_rs::scheduled::graph::Dfir;
use dfir_rs::util::{UdpSink, UdpStream};

use crate::protocol::EchoMsg;

pub(crate) async fn run_server(outbound: UdpSink, inbound: UdpStream, _opts: crate::Opts) {
    println!("Server live!");

    let mut flow: Dfir = dfir_syntax! {
        // Define a shared inbound channel
        inbound_chan = source_stream_serde(inbound) -> map(Result::unwrap) -> tee();

        // Print all messages for debugging purposes
        inbound_chan[0]
            -> for_each(|(msg, addr): (EchoMsg, SocketAddr)| println!("{}: Got {:?} from {:?}", Utc::now(), msg, addr));

        // Echo back the Echo messages with updated timestamp
        inbound_chan[1]
            -> map(|(EchoMsg {payload, ..}, addr)| (EchoMsg { payload, ts: Utc::now() }, addr) ) -> dest_sink_serde(outbound);
    };

    // run the server
    flow.run_async().await;
}
