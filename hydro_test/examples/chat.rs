use clap::Parser;
use hydro_deploy::Deployment;
use hydro_lang::deploy::TrybuildHost;
use hydro_test::graph_util::GraphConfig;

#[derive(Parser, Debug)]
struct Args {
    #[clap(flatten)]
    graph: GraphConfig,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    let mut deployment = Deployment::new();
    let builder = hydro_lang::FlowBuilder::new();
    let num_clients: u32 = 3;

    let (server, clients) = hydro_test::cluster::chat::chat_server(&builder);

    // Extract the IR BEFORE the builder is consumed by deployment methods
    let built = builder.finalize();

    // Generate graph visualizations based on command line arguments
    args.graph.generate_graph(&built)?;

    // Generate both short and long label files for comparison if no specific graph type was requested
    if args.graph.graph.is_none() {
        println!("No graph type specified. Generating comparison files...");

        // Generate files with current settings
        args.graph.generate_all_files(&built, "chat")?;

        // Also generate the opposite label style for comparison
        let opposite_config = GraphConfig {
            graph: None,
            no_metadata: args.graph.no_metadata,
            no_location_groups: args.graph.no_location_groups,
            no_tee_ids: args.graph.no_tee_ids,
            long_labels: !args.graph.long_labels,
        };
        opposite_config.generate_all_files(&built, "chat")?;

        println!("\nFiles generated! Compare short vs long labels:");
        println!("  - chat_short_labels.mmd vs chat_long_labels.mmd");
        println!("  - chat_short_labels.dot vs chat_long_labels.dot");
        println!("  - chat_short_labels.json vs chat_long_labels.json");
    }

    // Now use the built flow for deployment with optimization
    let _nodes = built
        .with_default_optimize()
        .with_process(&server, TrybuildHost::new(deployment.Localhost()))
        .with_cluster(
            &clients,
            (0..num_clients).map(|_| TrybuildHost::new(deployment.Localhost())),
        )
        .deploy(&mut deployment);

    deployment.deploy().await.unwrap();
    deployment.start().await.unwrap();

    tokio::signal::ctrl_c().await.unwrap();
    Ok(())
}
