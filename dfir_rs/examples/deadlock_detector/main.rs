use std::error::Error;
use std::fs::File;
use std::io::BufReader;
use std::path::Path;

/// This is a remedial distributed deadlock (cycle) detector
use clap::Parser;
use dfir_rs::lang::graph::{WriteConfig, WriteGraphType};
use peer::run_detector;
use serde::Deserialize;

mod helpers;
mod peer;
mod protocol;

#[derive(Parser, Debug)]
struct Opts {
    #[clap(long)]
    path: String,
    #[clap(long)]
    port: u16,
    #[clap(long)]
    addr: String,
    #[clap(long)]
    graph: Option<WriteGraphType>,
    #[clap(flatten)]
    write_config: Option<WriteConfig>,
}

#[derive(Deserialize, Debug)]
struct Addresses {
    peers: Vec<String>,
}

fn read_addresses_from_file(path: impl AsRef<Path>) -> Result<Addresses, Box<dyn Error>> {
    // Open the file in read-only mode with buffer.
    let file = File::open(path)?;
    let reader = BufReader::new(file);

    // Read the JSON contents of the file as an instance of `peers`.
    let u = serde_json::from_reader(reader)?;

    // Return the addresses.
    Ok(u)
}

#[dfir_rs::main]
async fn main() {
    let opts = Opts::parse();
    let path = Path::new(&opts.path);
    let peers = read_addresses_from_file(path).unwrap().peers;
    run_detector(opts, peers).await;
}

#[test]
fn test() {
    use example_test::run_current_example;

    let mut peer1 = run_current_example!(
        format!(
            "--path {}/examples/deadlock_detector/peers.json --addr 127.0.0.1 --port 12346",
            env!("CARGO_MANIFEST_DIR")
        )
        .split_whitespace()
    );
    let mut peer2 = run_current_example!(
        format!(
            "--path {}/examples/deadlock_detector/peers.json --addr 127.0.0.1 --port 12347",
            env!("CARGO_MANIFEST_DIR")
        )
        .split_whitespace()
    );
    let mut peer3 = run_current_example!(
        format!(
            "--path {}/examples/deadlock_detector/peers.json --addr 127.0.0.1 --port 12348",
            env!("CARGO_MANIFEST_DIR")
        )
        .split_whitespace()
    );

    peer1.read_string("Type in an edge as a tuple of two integers (x,y):");
    peer2.read_string("Type in an edge as a tuple of two integers (x,y):");
    peer3.read_string("Type in an edge as a tuple of two integers (x,y):");

    peer1.write_line("(1, 2)");
    peer2.write_line("(2, 3)");
    peer3.write_line("(3, 1)");

    peer1.read_string("path found: 1 -> 2 -> 3 -> 1");
    peer2.read_string("path found: 1 -> 2 -> 3 -> 1");
    peer3.read_string("path found: 1 -> 2 -> 3 -> 1");
}
