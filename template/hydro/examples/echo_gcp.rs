use std::sync::Arc;

use hydro_deploy::Deployment;
use hydro_deploy::gcp::GcpNetwork;
use hydro_lang::deploy::TrybuildHost;
use tokio::sync::RwLock;

static RELEASE_RUSTFLAGS: &str =
    "-C opt-level=3 -C codegen-units=1 -C strip=none -C debuginfo=2 -C lto=off";

#[tokio::main]
async fn main() {
    let gcp_project = std::env::args()
        .nth(1)
        .expect("Expected GCP project as first argument");

    let mut deployment = Deployment::new();
    let vpc = Arc::new(RwLock::new(GcpNetwork::new(&gcp_project, None)));

    let flow = hydro_lang::compile::builder::FlowBuilder::new();
    let process = flow.process();
    let external = flow.external::<()>();

    let (in_port, input) = process.source_external_bincode(&external);
    hydro_template::echo_capitalize(input)
        .for_each(q!(|s| {
            println!("Echoed capitalized: {}", s);
        }));

    let _nodes = flow
        .with_process(
            &process,
            TrybuildHost::new(
                deployment
                    .GcpComputeEngineHost()
                    .project(gcp_project.clone())
                    .machine_type("e2-micro")
                    .image("debian-cloud/debian-11")
                    .region("us-west1-a")
                    .network(vpc.clone())
                    .add(),
            )
            .rustflags(RELEASE_RUSTFLAGS),
        )
        .with_external(&external, deployment.Localhost())
        .deploy(&mut deployment);

    deployment.deploy().await.unwrap();

    let mut external_in = nodes.connect(in_port).await;

    deployment.start().await.unwrap();

    for line in std::io::stdin().lines() {
        let msg = msg.unwrap();
        if msg.is_empty() {
            break;
        }
        external_in.send(msg).await.unwrap();
    }
    external_in.send("hello".to_string()).await.unwrap();
    external_in.send("world".to_string()).await.unwrap();
}
