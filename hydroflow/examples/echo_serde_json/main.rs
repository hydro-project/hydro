use std::net::SocketAddr;

use clap::{Parser, ValueEnum};
use client::run_client;
use hydroflow::tokio;
use hydroflow::util::{bind_udp_lines, ipv4_resolve};
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

#[derive(Parser, Debug)]
struct Opts {
    #[clap(value_enum, long)]
    role: Role,
    #[clap(long, value_parser = ipv4_resolve)]
    client_addr: Option<SocketAddr>,
    #[clap(long, value_parser = ipv4_resolve)]
    server_addr: Option<SocketAddr>,
}

#[tokio::main]
async fn main() {
    // parse command line arguments
    let opts = Opts::parse();
    let server_addr = opts.server_addr.unwrap();

    // depending on the role, pass in arguments to the right function
    match opts.role {
        Role::Server => {
            // allocate `outbound` and `inbound` sockets
            let (outbound, inbound, _) = bind_udp_lines(server_addr).await;
            println!("Listening on {:?}", server_addr);
            run_server(outbound, inbound).await;
        }
        Role::Client => {
            // allocate `outbound` sink and `inbound` stream
            let client_addr = opts
                .client_addr
                .unwrap_or_else(|| ipv4_resolve("localhost:0").unwrap());
            let (outbound, inbound, client_addr) = bind_udp_lines(client_addr).await;
            println!(
                "Client is bound to {:?}, connecting to Server at {:?}",
                client_addr, server_addr
            );
            // run the client
            run_client(outbound, inbound, server_addr).await;
        }
    }
}
