use std::collections::HashMap;
use std::sync::Arc;

use clap::Parser;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[command(flatten)]
    graph: GraphConfig,

    /// Use GCP for deployment (provide project name)
    #[arg(long)]
    gcp: Option<String>,
}
use hydro_deploy::gcp::GcpNetwork;
use hydro_deploy::{Deployment, Host};
use hydro_lang::builder::rewrites::persist_pullup;
use hydro_lang::deploy::TrybuildHost;
use hydro_lang::graph::config::GraphConfig;
use hydro_lang::location::Location;
use hydro_optimize::partitioner::{self, Partitioner};
use tokio::sync::RwLock;

type HostCreator = Box<dyn Fn(&mut Deployment) -> Arc<dyn Host>>;

// run with no args for localhost, with `--gcp <GCP PROJECT>` for GCP
#[tokio::main]
async fn main() {
    let args = Args::parse();
    let mut deployment = Deployment::new();

    let (create_host, rustflags): (HostCreator, &'static str) = if let Some(project) = &args.gcp {
        let network = Arc::new(RwLock::new(GcpNetwork::new(project, None)));
        let project = project.clone();

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

    let builder = hydro_lang::builder::FlowBuilder::new();
    let (process, cluster) = hydro_test::cluster::simple_cluster::simple_cluster(&builder);

    let num_original_nodes = 2;
    let partitioner = Partitioner {
        nodes_to_partition: HashMap::from([(5, vec!["1".to_string()])]),
        num_partitions: 3,
        location_id: cluster.id().raw_id(),
        new_cluster_id: None,
    };

    // Extract the IR BEFORE optimization
    let built = builder.finalize();

    // Generate graphs if requested
    let _ = built.generate_graph_with_config(&args.graph, None);

    let _nodes = built
        .optimize_with(persist_pullup::persist_pullup)
        .optimize_with(|roots| partitioner::partition(roots, &partitioner))
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
