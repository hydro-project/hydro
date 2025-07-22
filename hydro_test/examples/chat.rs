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

    // Example 1: Default behavior (short labels)
    println!("Opening browsers with SHORT labels (default)...");
    hydro_lang::graph::mermaid::open_browser(&built)?;
    hydro_lang::graph::reactflow::open_browser(&built)?;
    hydro_lang::graph::graphviz::open_browser(&built)?;

    // Example 2: Create custom config for LONG labels
    let config_long_labels = hydro_lang::graph::render::HydroWriteConfig {
        show_metadata: false,
        show_location_groups: true,
        include_tee_ids: true,
        use_short_labels: false, // Set to false for long/full node names
        process_id_name: built.process_id_name().clone(),
        cluster_id_name: built.cluster_id_name().clone(),
        external_id_name: built.external_id_name().clone(),
    };

    // Generate files with long labels for comparison
    println!("Generating files with LONG labels...");

    // Mermaid with long labels
    let mermaid_long =
        hydro_lang::graph::render::render_hydro_ir_mermaid(built.ir(), &config_long_labels);
    std::fs::write("chat_long_labels.mmd", mermaid_long)?;
    println!("Generated: chat_long_labels.mmd");

    // Graphviz with long labels
    let graphviz_long =
        hydro_lang::graph::render::render_hydro_ir_dot(built.ir(), &config_long_labels);
    std::fs::write("chat_long_labels.dot", graphviz_long)?;
    println!("Generated: chat_long_labels.dot");

    // ReactFlow with long labels
    let reactflow_long =
        hydro_lang::graph::render::render_hydro_ir_reactflow(built.ir(), &config_long_labels);
    std::fs::write("chat_long_labels.json", reactflow_long)?;
    println!("Generated: chat_long_labels.json");

    // Compare: also generate short label versions
    let config_short_labels = hydro_lang::graph::render::HydroWriteConfig {
        show_metadata: false,
        show_location_groups: true,
        include_tee_ids: true,
        use_short_labels: true, // Short labels (default)
        process_id_name: built.process_id_name().clone(),
        cluster_id_name: built.cluster_id_name().clone(),
        external_id_name: built.external_id_name().clone(),
    };

    println!("Generating files with SHORT labels for comparison...");
    let mermaid_short =
        hydro_lang::graph::render::render_hydro_ir_mermaid(built.ir(), &config_short_labels);
    std::fs::write("chat_short_labels.mmd", mermaid_short)?;
    println!("Generated: chat_short_labels.mmd");

    println!("\nFiles generated! Compare:");
    println!("  - chat_short_labels.mmd vs chat_long_labels.mmd");
    println!("  - chat_long_labels.dot (Graphviz)");
    println!("  - chat_long_labels.json (ReactFlow)");

    // Graphviz/DOT visualization
    // hydro_lang::graph::graphviz::open_browser(&built)?;

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
