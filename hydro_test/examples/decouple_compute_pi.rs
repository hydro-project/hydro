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
use hydro_lang::Location;
use hydro_lang::deploy::TrybuildHost;
use hydro_lang::graph::config::GraphConfig;
use hydro_lang::rewrites::persist_pullup;
use hydro_optimize::debug;
use hydro_optimize::decoupler::{self, Decoupler};
use tokio::sync::RwLock;

type HostCreator = Box<dyn Fn(&mut Deployment) -> Arc<dyn Host>>;

struct DecoupledCluster {}

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

    let builder = hydro_lang::FlowBuilder::new();
    let (cluster, leader) = hydro_test::cluster::compute_pi::compute_pi(&builder, 8192);

    let decoupled_cluster = builder.cluster::<DecoupledCluster>();

    let decoupler = Decoupler {
        // Decouple between these operators:
        // .map(q!(|_| rand::random::<(f64, f64)>()))
        // .map(q!(|(x, y)| x * x + y * y < 1.0))
        output_to_decoupled_machine_after: vec![4],
        output_to_original_machine_after: vec![],
        place_on_decoupled_machine: vec![],
        orig_location: cluster.id().clone(),
        decoupled_location: decoupled_cluster.id().clone(),
    };

    // Extract the IR BEFORE optimization
    let built = builder.finalize();

    // Generate graphs if requested
    let _ = built.generate_graph_with_config(&args.graph, None);

    let _nodes = built
        .optimize_with(persist_pullup::persist_pullup)
        .optimize_with(|roots| decoupler::decouple(roots, &decoupler))
        .optimize_with(debug::print_id)
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
