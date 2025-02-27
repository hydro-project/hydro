use std::sync::Arc;

use hydro_deploy::gcp::GcpNetwork;
use hydro_deploy::{Deployment, Host};
use hydro_lang::Location;
use hydro_lang::deploy::TrybuildHost;
use hydro_lang::rewrites::decoupler::Decoupler;
use hydro_lang::rewrites::{decoupler, persist_pullup, print_id};
use tokio::sync::RwLock;

type HostCreator = Box<dyn Fn(&mut Deployment) -> Arc<dyn Host>>;

struct DecoupledCluster {}

// run with no args for localhost, with `gcp <GCP PROJECT>` for GCP
#[tokio::main]
async fn main() {
    let mut deployment = Deployment::new();
    let host_arg = std::env::args().nth(1).unwrap_or_default();

    let (create_host, rustflags): (HostCreator, &'static str) = if host_arg == *"gcp" {
        let project = std::env::args().nth(2).unwrap();
        let network = Arc::new(RwLock::new(GcpNetwork::new(&project, None)));

        (
            Box::new(move |deployment| -> Arc<dyn Host> {
                deployment
                    .GcpComputeEngineHost()
                    .project(&project)
                    .machine_type("e2-micro")
                    .image("debian-cloud/debian-11")
                    .region("us-west1-a")
                    .network(network.clone())
                    .add()
            }),
            "-C opt-level=3 -C codegen-units=1 -C strip=none -C debuginfo=2 -C lto=off",
        )
    } else {
        let localhost = deployment.Localhost();
        (
            Box::new(move |_| -> Arc<dyn Host> { localhost.clone() }),
            "",
        )
    };

    let builder = hydro_lang::FlowBuilder::new();
    let (cluster, leader) = hydro_test::cluster::compute_pi::compute_pi(&builder, 8192);

    let decoupled_cluster = builder.cluster::<DecoupledCluster>();

    let decoupler = Decoupler {
        // Decouple between these operators:
        // .map(q!(|_| rand::random::<(f64, f64)>()))
        // .map(q!(|(x, y)| x * x + y * y < 1.0))
        nodes_to_decouple: vec![4],
        new_location: decoupled_cluster.id().clone(),
    };

    let _nodes = builder
        .optimize_with(persist_pullup::persist_pullup)
        .optimize_with(|leaves| decoupler::decouple(leaves, &decoupler))
        .optimize_with(print_id::print_id)
        .with_process(
            &leader,
            TrybuildHost::new(create_host(&mut deployment)).rustflags(rustflags),
        )
        .with_cluster(
            &cluster,
            (0..8).map(|_| TrybuildHost::new(create_host(&mut deployment)).rustflags(rustflags)),
        )
        .with_cluster(
            &decoupled_cluster,
            (0..8).map(|_| TrybuildHost::new(create_host(&mut deployment)).rustflags(rustflags)),
        )
        .deploy(&mut deployment);

    deployment.run_ctrl_c().await.unwrap();
}
