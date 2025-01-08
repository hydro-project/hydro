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
    addr: Option<SocketAddr>,
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
    let addr = opts.addr.unwrap();

    match opts.role {
        Role::Client => {
            let (outbound, inbound, _) = bind_udp_bytes(addr).await;
            println!("Client is bound to {:?}", addr);
            println!("Attempting to connect to server at {:?}", opts.server_addr);
            run_client(outbound, inbound, opts.server_addr.unwrap(), opts).await;
        }
        Role::Server => {
            let (outbound, inbound, _) = bind_udp_bytes(addr).await;
            println!("Listening on {:?}", opts.addr.unwrap());
            run_server(outbound, inbound, opts).await;
        }
    }
}

#[test]
fn test() {
    use std::io::Write;

    use dfir_rs::util::{run_cargo_example, wait_for_process_output};

    let (_server, _, mut server_stdout) =
        run_cargo_example("kvs_mut", "--role server --addr 127.0.0.1:2061");

    let (_client1, mut client1_stdin, mut client1_stdout) = run_cargo_example(
        "kvs_mut",
        "--role client --addr 127.0.0.1:2062 --server-addr 127.0.0.1:2061",
    );

    let mut server_output = String::new();
    wait_for_process_output(&mut server_output, &mut server_stdout, "Server live!");

    let mut client1_output = String::new();
    wait_for_process_output(&mut client1_output, &mut client1_stdout, "Client live!");

    client1_stdin.write_all(b"PUT a,7\n").unwrap();

    let (_client2, mut client2_stdin, mut client2_stdout) = run_cargo_example(
        "kvs_mut",
        "--role client --addr 127.0.0.1:2063 --server-addr 127.0.0.1:2061",
    );

    let mut client2_output = String::new();
    wait_for_process_output(&mut client2_output, &mut client2_stdout, "Client live!");

    client2_stdin.write_all(b"GET a\n").unwrap();
    wait_for_process_output(
        &mut client2_output,
        &mut client2_stdout,
        r#"Got a Response: KvsResponse \{ key: "a", value: "7" \}"#,
    );

    client1_stdin.write_all(b"PUT a,8\n").unwrap();
    client1_stdin.write_all(b"GET a\n").unwrap();
    wait_for_process_output(
        &mut client1_output,
        &mut client1_stdout,
        r#"Got a Response: KvsResponse \{ key: "a", value: "8" \}"#,
    );
}
