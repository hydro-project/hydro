use clap::Parser;
use dfir_rs::tokio_util::codec::LinesCodec;
use hydro_deploy::Deployment;
use hydro_lang::deploy::TrybuildHost;
use hydro_lang::graph::config::GraphConfig;
use hydro_lang::location::{Location, NetworkHint};
use hydro_lang::nondet::nondet;

#[derive(Parser, Debug)]
struct Args {
    #[clap(flatten)]
    graph: GraphConfig,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    let mut deployment = Deployment::new();
    let flow = hydro_lang::compile::builder::FlowBuilder::new();

    let process = flow.process::<()>();
    let external = flow.external::<()>();

    let (port, input, membership, output_ref) =
        process.bidi_external_many_bytes::<_, _, LinesCodec>(&external, NetworkHint::Auto);

    output_ref.complete(hydro_test::external_client::chat::chat_server(
        &process,
        input,
        membership,
        nondet!(/** test */),
    ));

    // Extract the IR BEFORE the builder is consumed by deployment methods
    let built = flow.finalize();

    // Generate graph visualizations based on command line arguments
    built.generate_graph_with_config(&args.graph, None)?;

    // If we're just generating a graph file, exit early
    if args.graph.file && args.graph.graph.is_some() {
        return Ok(());
    }

    // Now use the built flow for deployment with optimization
    let nodes = built
        .with_default_optimize()
        .with_process(&process, TrybuildHost::new(deployment.Localhost()))
        .with_external(&external, deployment.Localhost())
        .deploy(&mut deployment);

    deployment.deploy().await.unwrap();

    let raw_port = nodes.raw_port(port);
    let server_port = raw_port.server_port().await;

    deployment.start().await.unwrap();

    println!("Chat server started! Connect to {:?}", server_port);
    println!("Type messages and they will be broadcast to all other connected clients.");

    tokio::signal::ctrl_c().await.unwrap();
    Ok(())
}

#[test]
fn test() {
    use example_test::run_current_example;

    let mut run = run_current_example!();
    run.read_regex(r"Chat server started");
}
