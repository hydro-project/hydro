use std::net::SocketAddr;

use chrono::Utc;
use clap::{Parser, ValueEnum};
use hydroflow::hydroflow_syntax;
use hydroflow::scheduled::graph::Hydroflow;
use hydroflow::util::{bind_websocket, ipv4_resolve};
use tokio_tungstenite::tungstenite::Message;

#[derive(Clone, ValueEnum, Debug)]
enum Role {
    Client,
    Server,
}

#[derive(Parser, Debug)]
struct Opts {
    #[clap(long, value_parser = ipv4_resolve)]
    addr: Option<SocketAddr>,
}

#[hydroflow::main]
async fn main() {
    // parse command line arguments
    let opts = Opts::parse();
    // if no addr was provided, we ask the OS to assign a local port by passing in "localhost:0"
    let addr = opts
        .addr
        .unwrap_or_else(|| ipv4_resolve("localhost:0").unwrap());

    // allocate `outbound` sink and `inbound` stream
    let (outbound, inbound, addr) = bind_websocket(addr).await.unwrap();
    println!("Listening on {:?}", addr);

    let mut flow: Hydroflow = hydroflow_syntax! {
        // Define a shared inbound channel
        inbound_chan = source_stream(inbound) -> map(Result::unwrap) -> tee();

        // Print all messages for debugging purposes
        inbound_chan[0]
            -> for_each(|(msg, addr): (Message, SocketAddr)| println!("{}: Got {:?} from {:?}", Utc::now(), msg, addr));

        clients = inbound_chan[1] -> map(|(_msg, addr)| addr) -> unique::<'static>();
        messages = inbound_chan[2] -> map(|(msg, _addr)| msg);

        messages -> [0]cj;
        clients -> [1]cj;
        cj = cross_join::<'tick, 'static>() -> inspect(|msg| println!("SEND {:?}", msg)) -> dest_sink(outbound);
    };

    // run the server
    flow.run_async().await;
}
