use std::net::SocketAddr;

use clap::{Parser, ValueEnum};
use client::run_client;
use hydroflow::util::{bind_udp_bytes, bind_websocket, ipv4_resolve};
use server::run_server;

mod client;
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
    let (outbound, inbound, addr) = bind_udp_bytes(addr).await.unwrap();
    println!("Listening on {:?}", addr);

    match opts.role {
        Role::Server => {
            run_server(outbound, inbound, opts).await;
        }
        _ => panic!("Unsupported!")
    }
}

#[test]
fn test() {
    use std::io::Write;

    use hydroflow::util::{run_cargo_example, wait_for_process_output};

    let (_server, _, mut server_output) =
        run_cargo_example("echoserver", "--role server --addr 127.0.0.1:2048");

    let (_client, mut client_input, mut client_output) =
        run_cargo_example("echoserver", "--role client --server-addr 127.0.0.1:2048");

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
        "UTC: Got EchoMsg \\{ payload: \"Hello\",",
    );
}
