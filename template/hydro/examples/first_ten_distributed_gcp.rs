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

    let flow = hydro_lang::FlowBuilder::new();
    let p1 = flow.process();
    let p2 = flow.process();
    hydro_template::first_ten_distributed::first_ten_distributed(&p1, &p2);

    let _nodes = flow
        .with_process(
            &p1,
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
        .with_process(
            &p2,
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
        .deploy(&mut deployment);

    deployment.run_ctrl_c().await.unwrap();
}
