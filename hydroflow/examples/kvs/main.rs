use std::net::SocketAddr;

use clap::{Parser, ValueEnum};
use client::run_client;
use hydroflow::tokio;
use hydroflow::util::{bind_udp_bytes, ipv4_resolve};
use server::run_server;

mod client;
mod helpers;
mod protocol;
mod server;

#[derive(Clone, ValueEnum, Debug)]
enum Role {
    Client,
    Server,
}
#[derive(Clone, ValueEnum, Debug)]
enum GraphType {
    Mermaid,
    Dot,
    Json,
}

#[derive(Parser, Debug)]
struct Opts {
    #[clap(value_enum, long)]
    role: Role,
    #[clap(long, value_parser = ipv4_resolve)]
    addr: Option<SocketAddr>,
    #[clap(long, value_parser = ipv4_resolve)]
    server_addr: Option<SocketAddr>,
    #[clap(value_enum, long)]
    graph: Option<GraphType>,
}

#[tokio::main]
async fn main() {
    let opts = Opts::parse();
    let addr = opts.addr.unwrap();

    match opts.role {
        Role::Client => {
            let (outbound, inbound, _) = bind_udp_bytes(addr).await;
            println!("Client is bound to {:?}", addr);
            println!("Attempting to connect to server at {:?}", opts.server_addr);
            run_client(
                outbound,
                inbound,
                opts.server_addr.unwrap(),
                opts.graph.clone(),
            )
            .await;
        }
        Role::Server => {
            let (outbound, inbound, _) = bind_udp_bytes(addr).await;
            println!("Listening on {:?}", opts.addr.unwrap());
            run_server(outbound, inbound, opts.graph.clone()).await;
        }
    }
}
