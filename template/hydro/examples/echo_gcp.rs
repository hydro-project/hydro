use hydro_deploy::Deployment;
use hydro_deploy::gcp::GcpNetwork;
use hydro_lang::location::{Location, NetworkHint};
use tokio_util::codec::LinesCodec;

#[tokio::main]
async fn main() {
    let gcp_project = std::env::args()
        .nth(1)
        .expect("Expected GCP project as first argument");

    let mut deployment = Deployment::new();
    let vpc = GcpNetwork::new(&gcp_project, None);

    let mut flow = hydro_lang::compile::builder::FlowBuilder::new();
    let process = flow.process();
    let external = flow.external::<()>();

    let (port, input, output) =
        process.bind_single_client::<_, _, LinesCodec>(&external, NetworkHint::Auto);
    output.complete(hydro_template::echo_capitalize(input));

    let nodes = flow
        .with_process(
            &process,
            deployment
                .GcpComputeEngineHost()
                .project(gcp_project.clone())
                .machine_type("e2-micro")
                .image("debian-cloud/debian-11")
                .region("us-west1-a")
                .network(vpc.clone())
                .add(),
        )
        .with_external(&external, deployment.Localhost())
        .deploy(&mut deployment);

    deployment.deploy().await.unwrap();

    let raw_port = nodes.raw_port(port);
    let server_port = raw_port.server_port().await;

    println!("Please connect a client to port {:?}", server_port);

    deployment.start_ctrl_c().await.unwrap();
}
