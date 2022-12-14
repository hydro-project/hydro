use clap::{Parser, ValueEnum};
use client::run_client;
use hydroflow::tokio;
use hydroflow::util::{bind_udp_bytes, ipv4_resolve};
use server::run_server;
use std::net::SocketAddr;

mod client;
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
    #[clap(long)]
    name: String,
    #[clap(value_enum, long)]
    role: Role,
    #[clap(long, value_parser = ipv4_resolve)]
    client_addr: Option<SocketAddr>,
    #[clap(long, value_parser = ipv4_resolve)]
    server_addr: SocketAddr,
    #[clap(value_enum, long)]
    graph: Option<GraphType>,
}

#[tokio::main]
async fn main() {
    let opts = Opts::parse();
    let server_addr = opts.server_addr;

    match opts.role {
        Role::Client => {
            let client_addr = opts.client_addr.unwrap();
            println!(
                "Client is bound to {:?}, connecting to Server at {:?}",
                client_addr, server_addr
            );
            let (outbound, inbound) = bind_udp_bytes(client_addr).await;
            run_client(
                outbound,
                inbound,
                server_addr,
                opts.name.clone(),
                opts.graph.clone(),
            )
            .await;
        }
        Role::Server => {
            println!("Listening on {:?}", server_addr);
            let (outbound, inbound) = bind_udp_bytes(server_addr).await;

            run_server(outbound, inbound, opts.graph.clone()).await;
        }
    }
}
