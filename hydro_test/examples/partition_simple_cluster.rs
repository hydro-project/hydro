use std::collections::HashMap;
use std::sync::Arc;

use hydro_deploy::gcp::GcpNetwork;
use hydro_deploy::{Deployment, Host};
use hydro_lang::Location;
use hydro_lang::deploy::TrybuildHost;
use hydro_lang::rewrites::persist_pullup;
use hydro_optimize::partitioner::{self, Partitioner};
use tokio::sync::RwLock;

type HostCreator = Box<dyn Fn(&mut Deployment) -> Arc<dyn Host>>;

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
    let (process, cluster) = hydro_test::cluster::simple_cluster::simple_cluster(&builder);

    let num_original_nodes = 2;
    let partitioner = Partitioner {
        nodes_to_partition: HashMap::from([(5, vec!["1".to_string()])]),
        num_partitions: 3,
        partitioned_cluster_id: cluster.id().raw_id(),
    };

    let _nodes = builder
        .optimize_with(persist_pullup::persist_pullup)
        .optimize_with(|leaves| partitioner::partition(leaves, &partitioner))
        .with_process(
            &process,
            TrybuildHost::new(create_host(&mut deployment)).rustflags(rustflags),
        )
        .with_cluster(
            &cluster,
            (0..num_original_nodes * partitioner.num_partitions)
                .map(|_| TrybuildHost::new(create_host(&mut deployment)).rustflags(rustflags)),
        )
        .deploy(&mut deployment);
    deployment.run_ctrl_c().await.unwrap();
}
