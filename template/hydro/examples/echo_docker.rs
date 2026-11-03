use bytes::{Bytes, BytesMut};
use hydro_lang::deploy::{DockerDeploy, DockerNetwork};
use hydro_lang::location::{Location, NetworkHint};
use stageleft::q;
use tokio_util::codec::LengthDelimitedCodec;

// `DockerDeploy` numbers process ports from 1000. The reply path reserves
// 1000, and the `bind_single_client` listener is the next port, which is
// published on the host.
const CONTAINER_PORT: u16 = 1001;

#[tokio::main]
async fn main() {
    let network = DockerNetwork::new("hydro_echo".to_owned());
    let mut deployment = DockerDeploy::new(network);

    let mut flow = hydro_lang::compile::builder::FlowBuilder::new();
    let process = flow.process();
    let external = flow.external::<()>();

    // Docker external sockets are length-delimited frames. `NetworkHint::Auto`
    // is the only hint this deployment accepts.
    let (_port, input, output) =
        process.bind_single_client::<_, Bytes, LengthDelimitedCodec>(&external, NetworkHint::Auto);
    let lines = input.map(q!(|msg: BytesMut| String::from_utf8(msg.to_vec()).unwrap()));
    output.complete(
        hydro_template::echo_capitalize(lines)
            .map(q!(|line: String| Bytes::from(line.into_bytes()))),
    );

    let nodes = flow
        .with_process(&process, deployment.add_localhost_docker(None, Vec::new()))
        .with_external(&external, deployment.add_external("external".to_owned()))
        .deploy(&mut deployment);

    deployment.provision(&nodes).await.unwrap();
    deployment.start(&nodes).await.unwrap();

    let (host, port) = nodes
        .get_process(&process)
        .get_tcp_endpoint(CONTAINER_PORT)
        .await;
    println!("Launched Echo Server! Send a length-delimited UTF-8 frame to {host}:{port}.");

    tokio::signal::ctrl_c().await.unwrap();

    deployment.stop(&nodes).await.unwrap();
    deployment.cleanup(&nodes).await.unwrap();
}
