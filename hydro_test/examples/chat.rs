use hydro_deploy::Deployment;
use hydro_lang::deploy::TrybuildHost;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut deployment = Deployment::new();
    let builder = hydro_lang::FlowBuilder::new();
    let num_clients: u32 = 3;

    let (server, clients) = hydro_test::cluster::chat::chat_server(&builder);

    // Extract the IR BEFORE the builder is consumed by deployment methods
    let built = builder.finalize();

    // Generate graph visualizations (do this before deployment to avoid ownership issues)
    
    // Mermaid diagram
    hydro_lang::graph::mermaid::open_browser(&built)?;
    
    // ReactFlow.js visualization with type names
    hydro_lang::graph::reactflow::open_browser(&built)?;
    
    // Graphviz/DOT visualization
    hydro_lang::graph::graphviz::open_browser(&built)?;

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
