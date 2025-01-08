use std::net::SocketAddr;

use clap::{Parser, ValueEnum};
use client::run_client;
use dfir_rs::lang::graph::{WriteConfig, WriteGraphType};
use dfir_rs::util::{bind_udp_bytes, ipv4_resolve};
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
    addr: SocketAddr,
    #[clap(long, value_parser = ipv4_resolve)]
    server_addr: Option<SocketAddr>,
    #[clap(long)]
    graph: Option<WriteGraphType>,
    #[clap(flatten)]
    write_config: Option<WriteConfig>,
}

#[dfir_rs::main]
async fn main() {
    let opts = Opts::parse();
    let addr = opts.addr;

    match opts.role {
        Role::Client => {
            let (outbound, inbound, _) = bind_udp_bytes(addr).await;
            println!("Client is bound to {:?}", addr);
            println!("Attempting to connect to server at {:?}", opts.server_addr);
            run_client(outbound, inbound, opts).await;
        }
        Role::Server => {
            let (outbound, inbound, _) = bind_udp_bytes(addr).await;
            println!("Listening on {:?}", addr);
            run_server(outbound, inbound, opts).await;
        }
    }
}

#[test]
fn test() {
    use std::io::Write;

    use dfir_rs::util::{run_cargo_example, wait_for_process_output};

    let (_server_1, _, mut server_1_stdout) =
        run_cargo_example("kvs_replicated", "--role server --addr 127.0.0.1:2051");

    let (_client_1, mut client_1_stdin, mut client_1_stdout) = run_cargo_example(
        "kvs_replicated",
        "--role client --addr 127.0.0.1:2052 --server-addr 127.0.0.1:2051",
    );

    let mut server_1_output = String::new();
    wait_for_process_output(&mut server_1_output, &mut server_1_stdout, "Server live!");

    let mut client_1_output = String::new();
    wait_for_process_output(&mut client_1_output, &mut client_1_stdout, "Client live!");

    client_1_stdin.write_all(b"PUT a,7\n").unwrap();

    let (_server_2, _, mut server_2_stdout) = run_cargo_example(
        "kvs_replicated",
        "--role server --addr 127.0.0.1:2053 --server-addr 127.0.0.1:2051",
    );

    let (_client_2, mut client_2_stdin, mut client_2_stdout) = run_cargo_example(
        "kvs_replicated",
        "--role client --addr 127.0.0.1:2054 --server-addr 127.0.0.1:2053",
    );

    let mut server_2_output = String::new();
    wait_for_process_output(&mut server_2_output, &mut server_2_stdout, "Server live!");
    wait_for_process_output(
        &mut server_2_output,
        &mut server_2_stdout,
        r#"Message received PeerGossip \{ key: "a", value: "7" \} from 127\.0\.0\.1:2051"#,
    );

    let mut client_2_output = String::new();
    wait_for_process_output(&mut client_2_output, &mut client_2_stdout, "Client live!");

    client_2_stdin.write_all(b"GET a\n").unwrap();
    wait_for_process_output(
        &mut client_2_output,
        &mut client_2_stdout,
        r#"Got a Response: ServerResponse \{ key: "a", value: "7" \}"#,
    );
}
