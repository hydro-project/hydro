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
