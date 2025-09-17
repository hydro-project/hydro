use std::net::SocketAddr;
use std::path::Path;

use clap::{Parser, ValueEnum};
use coordinator::run_coordinator;
use dfir_lang::graph::{WriteConfig, WriteGraphType};
use dfir_rs::util::{bind_udp_bytes, ipv4_resolve};
use serde::Deserialize;
use subordinate::run_subordinate;

mod coordinator;
mod helpers;
mod protocol;
mod subordinate;

/// This is a remedial 2PC implementation.

#[derive(Clone, ValueEnum, Debug)]
enum Role {
    Coordinator,
    Subordinate,
}

#[derive(Parser, Debug)]
struct Opts {
    #[clap(long)]
    path: String,
    #[clap(value_enum, long)]
    role: Role,
    #[clap(long, value_parser = ipv4_resolve)]
    addr: SocketAddr,
    #[clap(long)]
    graph: Option<WriteGraphType>,
    #[clap(flatten)]
    write_config: Option<WriteConfig>,
}
impl Opts {
    pub fn path(&self) -> &Path {
        Path::new(&self.path)
    }
}

#[derive(Deserialize, Debug)]
struct Addresses {
    coordinator: String,
    subordinates: Vec<String>,
}

#[dfir_rs::main]
async fn main() {
    let opts = Opts::parse();
    let addr = opts.addr;

    match opts.role {
        Role::Coordinator => {
            let (outbound, inbound, _) = bind_udp_bytes(addr).await;
            run_coordinator(outbound, inbound, opts).await;
        }
        Role::Subordinate => {
            let (outbound, inbound, _) = bind_udp_bytes(addr).await;
            run_subordinate(outbound, inbound, opts).await;
        }
    }
}

#[test]
fn test() {
    use example_test::run_current_example;

    const MEMBERS_PATH: &str = "examples/two_pc_hf/members.json";

    let mut coordinator = run_current_example!(
        format!("--path {MEMBERS_PATH} --role coordinator --addr 127.0.0.1:12346")
            .split_whitespace()
    );
    coordinator.read_string("Coordinator live!");

    let mut subordinate1 = run_current_example!(
        format!("--path {MEMBERS_PATH} --role subordinate --addr 127.0.0.1:12347")
            .split_whitespace()
    );
    subordinate1.read_string("Subordinate live!");

    let mut subordinate2 = run_current_example!(
        format!("--path {MEMBERS_PATH} --role subordinate --addr 127.0.0.1:12348")
            .split_whitespace()
    );
    subordinate2.read_string("Subordinate live!");

    let mut subordinate3 = run_current_example!(
        format!("--path {MEMBERS_PATH} --role subordinate --addr 127.0.0.1:12349")
            .split_whitespace()
    );
    subordinate3.read_string("Subordinate live!");

    coordinator.write_line("1");

    coordinator.read_string("Sending Prepare(1) to 127.0.0.1:12347");
    coordinator.read_string("Sending Prepare(1) to 127.0.0.1:12348");
    coordinator.read_string("Sending Prepare(1) to 127.0.0.1:12349");

    // Wait for either Commit or Ended response.
    coordinator.read_regex(r"Received (Commit\(1\)|Ended\(1\))");
}
