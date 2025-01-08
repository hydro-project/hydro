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
    use std::io::Write;

    use dfir_rs::util::{run_cargo_example, wait_for_process_output};

    let (_peer1, mut peer1_stdin, mut peer1_stdout) = run_cargo_example(
        "deadlock_detector",
        &format!(
            "--path {}/examples/deadlock_detector/peers.json --addr 127.0.0.1 --port 12346",
            env!("CARGO_MANIFEST_DIR")
        ),
    );

    let (_peer2, mut peer2_stdin, mut peer2_stdout) = run_cargo_example(
        "deadlock_detector",
        &format!(
            "--path {}/examples/deadlock_detector/peers.json --addr 127.0.0.1 --port 12347",
            env!("CARGO_MANIFEST_DIR")
        ),
    );

    let (_peer3, mut peer3_stdin, mut peer3_stdout) = run_cargo_example(
        "deadlock_detector",
        &format!(
            "--path {}/examples/deadlock_detector/peers.json --addr 127.0.0.1 --port 12348",
            env!("CARGO_MANIFEST_DIR")
        ),
    );

    let mut peer3_output = String::new();
    wait_for_process_output(
        &mut peer3_output,
        &mut peer3_stdout,
        "Type in an edge as a tuple of two integers \\(x,y\\):",
    );
    let mut peer1_output = String::new();
    wait_for_process_output(
        &mut peer1_output,
        &mut peer1_stdout,
        "Type in an edge as a tuple of two integers \\(x,y\\):",
    );
    let mut peer2_output = String::new();
    wait_for_process_output(
        &mut peer2_output,
        &mut peer2_stdout,
        "Type in an edge as a tuple of two integers \\(x,y\\):",
    );

    peer1_stdin.write_all(b"(1, 2)\n").unwrap();
    peer2_stdin.write_all(b"(2, 3)\n").unwrap();
    peer3_stdin.write_all(b"(3, 1)\n").unwrap();

    wait_for_process_output(
        &mut peer1_output,
        &mut peer1_stdout,
        "path found: 1 -> 2 -> 3 -> 1",
    );

    wait_for_process_output(
        &mut peer2_output,
        &mut peer2_stdout,
        "path found: 1 -> 2 -> 3 -> 1",
    );

    wait_for_process_output(
        &mut peer3_output,
        &mut peer3_stdout,
        "path found: 1 -> 2 -> 3 -> 1",
    );
}
