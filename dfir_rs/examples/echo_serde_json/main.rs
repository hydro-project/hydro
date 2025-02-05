use std::net::SocketAddr;

use clap::{Parser, ValueEnum};
use client::run_client;
use dfir_rs::util::{bind_udp_lines, ipv4_resolve};
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

#[dfir_rs::main]
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

#[test]
fn test() {
    use std::io::Write;

    use dfir_rs::util::{run_cargo_example, wait_for_process_output};

    let (_server, _, mut server_output) = run_cargo_example(
        "echo_serde_json",
        "--role server --server-addr 127.0.0.1:2049",
    );

    let (_client, mut client_input, mut client_output) = run_cargo_example(
        "echo_serde_json",
        "--role client --server-addr 127.0.0.1:2049",
    );

    let mut server_output_so_far = String::new();
    let mut client_output_so_far = String::new();

    wait_for_process_output(
        &mut server_output_so_far,
        &mut server_output,
        "Server live!\n",
    );
    wait_for_process_output(
        &mut client_output_so_far,
        &mut client_output,
        "Client live!\n",
    );

    client_input.write_all(b"Hello\n").unwrap();

    wait_for_process_output(
        &mut client_output_so_far,
        &mut client_output,
        "EchoMsg \\{ payload: \"Hello\", ts: .* \\}",
    );
}
