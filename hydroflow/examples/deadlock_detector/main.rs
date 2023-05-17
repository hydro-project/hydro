use std::error::Error;
use std::fs::File;
use std::io::BufReader;
use std::path::Path;

/// This is a remedial distributed deadlock (cycle) detector
use clap::{Parser, ValueEnum};
use hydroflow::tokio;
use peer::run_detector;
use serde::Deserialize;

mod helpers;
mod peer;
mod protocol;

#[derive(Clone, ValueEnum, Debug)]
enum GraphType {
    Mermaid,
    Dot,
    Json,
}

#[derive(Parser, Debug)]
struct Opts {
    #[clap(long)]
    path: String,
    #[clap(long)]
    port: u16,
    #[clap(long)]
    addr: String,
    #[clap(value_enum, long)]
    graph: Option<GraphType>,
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

#[tokio::main]
async fn main() {
    let opts = Opts::parse();
    let path = Path::new(&opts.path);
    let peers = read_addresses_from_file(path).unwrap().peers;
    run_detector(opts, peers).await;
}
