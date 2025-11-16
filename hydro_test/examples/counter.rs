use clap::Parser;
use hydro_deploy::Deployment;
use hydro_lang::prelude::*;
use hydro_lang::viz::config::GraphConfig;

#[derive(Parser, Debug)]
struct Args {
    #[clap(flatten)]
    graph: GraphConfig,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    let mut deployment = Deployment::new();
    let flow = FlowBuilder::new();

    let process = flow.process::<()>();
    let tick = process.tick();

    let counter = hydro_test::counter::counter(&tick);
    // Create the counter and print using a Hydro inspect inside the dataflow.
    // Use for_each as a terminal sink to keep the stream live.
    tick.spin_batch(q!(1))
        .cross_singleton(counter)
        .all_ticks()
        .for_each(q!(|(_, count)| {
            println!("Counter tick: {}", count);
        }));

    // Extract the IR BEFORE the builder is consumed by deployment methods
    let built = flow.finalize();

    // Generate graph visualizations based on command line arguments
    built.generate_graph_with_config(&args.graph, None)?;

    // If we're just generating a graph file, exit early
    if args.graph.should_exit_after_graph_generation() {
        return Ok(());
    }

    // Now use the built flow for deployment with optimization
    let _nodes = built
        .with_process(&process, deployment.Localhost())
        .deploy(&mut deployment);

    deployment.deploy().await.unwrap();
    deployment.start().await.unwrap();

    println!("Counter started! Press Ctrl+C to stop.");

    tokio::signal::ctrl_c().await.unwrap();
    Ok(())
}
