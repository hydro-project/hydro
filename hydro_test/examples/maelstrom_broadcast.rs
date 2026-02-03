//! Maelstrom broadcast server example.
//!
//! To run with Maelstrom:
//! ```bash
//! nix shell nixpkgs#graphviz nixpkgs#gnuplot -c cargo run -p hydro_test --features maelstrom --example maelstrom_broadcast -- --maelstrom-path $HOME/Downloads/maelstrom/maelstrom
//! ```

use clap::Parser;
use hydro_lang::deploy::maelstrom::deploy_maelstrom::{MaelstromClusterSpec, MaelstromDeployment};
use hydro_lang::deploy::maelstrom::maelstrom_bidi_clients;
use hydro_lang::nondet::nondet;

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

    output_handle.complete(
        hydro_test::maelstrom::broadcast::broadcast_server(input)
            .assume_ordering(nondet!(/** TODO */)),
    );

    let mut deployment = MaelstromDeployment::new(args.node_count)
        .maelstrom_path(&args.maelstrom_path)
        .workload("broadcast")
        .time_limit(args.time_limit);

    let _nodes = flow
        .finalize()
        .with_default_optimize()
        .with_cluster(&cluster, MaelstromClusterSpec::new(args.node_count))
        .deploy(&mut deployment);

    if args.build_only {
        let binary_path = deployment.build()?;
        println!("Built binary at: {}", binary_path.display());
        println!();
        println!("To run with Maelstrom:");
        println!(
            "  {} test -w broadcast --bin {} --node-count {} --time-limit {}",
            args.maelstrom_path,
            binary_path.display(),
            args.node_count,
            args.time_limit
        );
    } else {
        let status = deployment.run()?;
        if !status.success() {
            std::process::exit(status.code().unwrap_or(1));
        }
    }

    Ok(())
}
