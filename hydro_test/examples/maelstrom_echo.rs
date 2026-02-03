//! Maelstrom echo server example.
//!
//! This example demonstrates how to use the Maelstrom deployment backend
//! to run a simple echo server that can be tested with Maelstrom.
//!
//! To run with Maelstrom:
//! ```bash
//! nix shell nixpkgs#graphviz nixpkgs#gnuplot -c cargo run -p hydro_test --features maelstrom --example maelstrom_echo -- --maelstrom-path $HOME/Downloads/maelstrom/maelstrom --node-count 1 --time-limit 10
//! ```

use clap::Parser;
use hydro_lang::deploy::maelstrom::deploy_maelstrom::{MaelstromClusterSpec, MaelstromDeployment};
use hydro_lang::deploy::maelstrom::maelstrom_bidi_clients;

#[derive(Parser, Debug)]
struct Args {
    /// Only build the binary, don't run Maelstrom
    #[arg(long)]
    build_only: bool,

    /// Path to the maelstrom binary
    #[arg(long)]
    maelstrom_path: String,

    /// Number of nodes to run
    #[arg(long)]
    node_count: usize,

    /// Time limit in seconds
    #[arg(long)]
    time_limit: u64,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    let mut flow = hydro_lang::compile::builder::FlowBuilder::new();
    let cluster = flow.cluster::<()>();

    // Set up bidirectional client communication
    let (input, output_handle) = maelstrom_bidi_clients(&cluster);

    // Connect the echo server
    output_handle.complete(hydro_test::maelstrom::echo::echo_server(input));

    let mut deployment = MaelstromDeployment::new(args.node_count)
        .maelstrom_path(&args.maelstrom_path)
        .workload("echo")
        .time_limit(args.time_limit);

    let _nodes = flow
        .with_cluster(&cluster, MaelstromClusterSpec)
        .deploy(&mut deployment);

    if args.build_only {
        let binary_path = deployment.build()?;
        println!("Built binary at: {}", binary_path.display());
        println!();
        println!("To run with Maelstrom:");
        println!(
            "  {} test -w echo --bin {} --node-count {} --time-limit {}",
            args.maelstrom_path,
            binary_path.display(),
            args.node_count,
            args.time_limit
        );
    } else {
        deployment.run()?;
    }

    Ok(())
}
