use std::net::SocketAddr;

use clap::{Parser, ValueEnum};
use client::run_client;
use dfir_lang::graph::{WriteConfig, WriteGraphType};
use dfir_rs::util::{bind_udp_bytes, ipv4_resolve};
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
    #[clap(long)]
    graph: Option<WriteGraphType>,
    #[clap(flatten)]
    write_config: Option<WriteConfig>,
}

#[dfir_rs::main]
async fn main() {
    // parse command line arguments
    let opts = Opts::parse();
    // if no addr was provided, we ask the OS to assign a local port by passing in "localhost:0"
    let addr = opts
        .addr
        .unwrap_or_else(|| ipv4_resolve("localhost:0").unwrap());

    // allocate `outbound` sink and `inbound` stream
    let (outbound, inbound, addr) = bind_udp_bytes(addr).await;
    println!("Listening on {:?}", addr);

    match opts.role {
        Role::Server => {
            run_server(outbound, inbound, opts).await;
        }
        Role::Client => {
            run_client(outbound, inbound, opts).await;
        }
    }
}

#[test]
fn test() {
    use std::io::Write;

    use dfir_rs::util::{run_cargo_example, wait_for_process_output};

    let (_server, _, mut server_stdout) =
        run_cargo_example("lamport_clock", "--role server --addr 127.0.0.1:11052");

    let (_client1, mut client1_stdin, mut client1_stdout) = run_cargo_example(
        "lamport_clock",
        "--role client --server-addr 127.0.0.1:11052",
    );

    let (_client2, mut client2_stdin, mut client2_stdout) = run_cargo_example(
        "lamport_clock",
        "--role client --server-addr 127.0.0.1:11052",
    );

    let mut server_output = String::new();
    wait_for_process_output(&mut server_output, &mut server_stdout, "Server live!");

    let mut client1_output = String::new();
    wait_for_process_output(&mut client1_output, &mut client1_stdout, "Client live!");

    let mut client2_output = String::new();
    wait_for_process_output(&mut client2_output, &mut client2_stdout, "Client live!");

    client1_stdin.write_all(b"Hello1\n").unwrap();

    wait_for_process_output(
        &mut client1_output,
        &mut client1_stdout,
        r#"UTC: Got EchoMsg \{ payload: "Hello1", lamport_clock: Max\(1\) \} from 127.0.0.1:11052"#,
    );

    client2_stdin.write_all(b"Hello2\n").unwrap();

    wait_for_process_output(
        &mut client2_output,
        &mut client2_stdout,
        r#"UTC: Got EchoMsg \{ payload: "Hello2", lamport_clock: Max\(2\) \} from 127.0.0.1:11052"#,
    );
}
