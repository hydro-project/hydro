use clap::Parser;
use dfir_rs::tokio_util::codec::LinesCodec;
use hydro_deploy::Deployment;
use hydro_deploy::custom_service::ServerPort;
use hydro_lang::Location;
use hydro_lang::deploy::TrybuildHost;
use hydro_lang::graph::config::GraphConfig;
use hydro_lang::location::NetworkHint;

#[derive(Parser, Debug)]
struct Args {
    #[clap(flatten)]
    graph: GraphConfig,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    let mut deployment = Deployment::new();
    let flow = hydro_lang::FlowBuilder::new();

    let process = flow.process::<()>();
    let external = flow.external::<()>();

    let (port, input, _membership, output_ref) = process
        .bidi_external_many_bytes::<_, _, LinesCodec>(&external, NetworkHint::TcpPort(Some(4001)));

    output_ref
        .complete(hydro_test::external_client::http_counter::http_counter_server(input, &process));

    // Extract the IR BEFORE the builder is consumed by deployment methods
    let built = flow.finalize();

    // Generate graph visualizations based on command line arguments
    built.generate_graph_with_config(&args.graph, None)?;

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

    let port = if let ServerPort::TcpPort(p) = server_port {
        p
    } else {
        panic!("Expected a TCP port");
    };
    println!("HTTP counter server listening on: http://{:?}", port);

    tokio::signal::ctrl_c().await.unwrap();
    Ok(())
}
