use bytes::BytesMut;
use clap::Parser;
use dfir_rs::tokio_util::codec::{BytesCodec, LinesCodec};
use hydro_deploy::Deployment;
use hydro_deploy::custom_service::ServerPort;
use hydro_lang::deploy::TrybuildHost;
use hydro_lang::graph::config::GraphConfig;
use hydro_lang::location::{Location, NetworkHint};

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

    let (http_port, http_input, _http_membership, http_output_ref) = process
        .bidi_external_many_bytes::<_, _, LinesCodec>(&external, NetworkHint::TcpPort(Some(4000)));

    http_output_ref.complete(
        hydro_test::external_client::http::http_static::http_serve_static(
            http_input,
            include_str!("./websocket_test_client.html"),
        )
        .into_keyed_stream(),
    );

    let (port, input, _membership, output_ref) = process
        .bidi_external_many_bytes::<_, BytesMut, BytesCodec>(
            &external,
            NetworkHint::TcpPort(Some(8080)),
        );

    output_ref
        .complete(hydro_test::external_client::websocket::chat::websocket_chat(&process, input));

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

    let http_raw_port = nodes.raw_port(http_port);
    let http_server_port = http_raw_port.server_port().await;

    let raw_port = nodes.raw_port(port);
    let server_port = raw_port.server_port().await;

    deployment.start().await.unwrap();

    let port = if let ServerPort::TcpPort(p) = server_port {
        p
    } else {
        panic!("Expected a TCP port");
    };
    println!("WebSocket echo server listening on: ws://{}", port);

    let http_port = if let ServerPort::TcpPort(p) = http_server_port {
        p
    } else {
        panic!("Expected a TCP port");
    };
    println!("Browser Demo at: http://{}", http_port);

    tokio::signal::ctrl_c().await.unwrap();
    Ok(())
}
