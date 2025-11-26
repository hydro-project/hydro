use std::net::SocketAddr;

use clap::{Parser, ValueEnum};
use client::run_client;
use dfir_lang::graph::{WriteConfig, WriteGraphType};
use dfir_rs::util::ipv4_resolve;
use server::run_server;

use crate::randomized_gossiping_server::run_gossiping_server;

mod client;
mod protocol;
mod randomized_gossiping_server;
mod server;

#[derive(Clone, Copy, ValueEnum, Debug, Eq, PartialEq)]
enum Role {
    Client,
    Server,

    // These roles are only used by the randomized-gossip variant of the chat example.
    GossipingServer1,
    GossipingServer2,
    GossipingServer3,
    GossipingServer4,
    GossipingServer5,
}

pub fn default_server_address() -> SocketAddr {
    ipv4_resolve("localhost:54321").unwrap()
}

#[derive(Parser, Debug)]
struct Opts {
    #[clap(long)]
    name: String,
    #[clap(value_enum, long)]
    role: Role,
    #[clap(long, value_parser = ipv4_resolve)]
    address: Option<SocketAddr>,
    #[clap(long)]
    graph: Option<WriteGraphType>,
    #[clap(flatten)]
    write_config: Option<WriteConfig>,
}

#[dfir_rs::main]
async fn main() {
    let opts = Opts::parse();

    match opts.role {
        Role::Client => {
            run_client(opts).await;
        }
        Role::Server => {
            run_server(opts).await;
        }
        Role::GossipingServer1
        | Role::GossipingServer2
        | Role::GossipingServer3
        | Role::GossipingServer4
        | Role::GossipingServer5 => run_gossiping_server(opts).await,
    }
}

#[test]
fn test() {
    use example_test::run_current_example;

    let mut server = run_current_example!("--role server --name server --address 127.0.0.1:11247");
    server.read_regex("Server live!");

    let mut client1 =
        run_current_example!("--role client --name client1 --address 127.0.0.1:11247");
    let mut client2 =
        run_current_example!("--role client --name client2 --address 127.0.0.1:11247");

    client1.read_regex("Client is live!");
    client2.read_regex("Client is live!");

    // wait 100ms so we don't drop a packet
    let hundo_millis = web_time::Duration::from_millis(100);
    std::thread::sleep(hundo_millis);

    client1.write_line("Hello");
    client2.read_regex(".*, .* client1: Hello");
}

#[test]
fn test_gossip() {
    use example_test::run_current_example;

    let mut server1 =
        run_current_example!("--role gossiping-server1 --name server --address 127.0.0.1:11248");
    server1.read_regex("Server is live!");

    let mut server2 =
        run_current_example!("--role gossiping-server2 --name server --address 127.0.0.1:11249");
    server2.read_regex("Server is live!");

    let mut client1 =
        run_current_example!("--role client --name client1 --address 127.0.0.1:11248");
    let mut client2 =
        run_current_example!("--role client --name client2 --address 127.0.0.1:11249");

    client1.read_string("Client is live!");
    client2.read_string("Client is live!");

    // wait 100ms so we don't drop a packet
    let hundo_millis = web_time::Duration::from_millis(100);
    std::thread::sleep(hundo_millis);

    // Since gossiping has a small probability of a message not being received (maybe more so with
    // 2 servers), we define success as any one of these messages reaching.
    for _ in 1..=50 {
        client1.write_line("Hello");
    }

    client2.read_regex(".*, .* client1: Hello");
}
