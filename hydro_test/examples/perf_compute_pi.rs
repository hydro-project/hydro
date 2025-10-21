use std::sync::Arc;

use clap::Parser;
use hydro_deploy::gcp::GcpNetwork;
use hydro_deploy::rust_crate::tracing_options::TracingOptions;
use hydro_deploy::{Deployment, Host};
use hydro_lang::deploy::TrybuildHost;
use hydro_lang::prelude::*;
use hydro_lang::viz::config::GraphConfig;
use tokio::sync::RwLock;

type HostCreator = Box<dyn Fn(&mut Deployment) -> Arc<dyn Host>>;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct PerfComputePiArgs {
    #[command(flatten)]
    graph: GraphConfig,

    /// Use GCP for deployment (provide project name)
    #[arg(long)]
    gcp: Option<String>,
}

/// Run with no args for localhost, with `gcp <GCP PROJECT>` for GCP
///
/// ```bash
/// cargo run -p hydro_test --example perf_compute_pi -- gcp my-gcp-project
/// ```
///
/// Once the program is running, you can **press enter** to stop the program and see the results.
/// (Pressing Ctrl+C will stop the program **without cleaning up cloud resources** nor generating the
/// flamegraphs).
#[tokio::main]
async fn main() {
    let args = PerfComputePiArgs::parse();
    let mut deployment = Deployment::new();

    let create_host: HostCreator = if let Some(project) = &args.gcp {
        let network = Arc::new(RwLock::new(GcpNetwork::new(project, None)));
        let project = project.clone();

        Box::new(move |deployment| -> Arc<dyn Host> {
            deployment
                .GcpComputeEngineHost()
                .project(&project)
                .machine_type("n2-standard-4")
                .image("debian-cloud/debian-11")
                .region("us-central1-c")
                .network(network.clone())
                .add()
        })
    } else {
        let localhost = deployment.Localhost();
        Box::new(move |_| -> Arc<dyn Host> { localhost.clone() })
    };

    #[expect(
        clippy::needless_late_init,
        reason = "Better clarity for code extracted into docs."
    )]
    let rustflags;
    if args.gcp.is_some() {
        //[rustflags_gcp]//
        rustflags = "-C opt-level=3 -C codegen-units=1 -C strip=none -C debuginfo=2 -C lto=off -C link-args=--no-rosegment";
        //[/rustflags_gcp]//
    } else {
        //[rustflags]//
        rustflags = "-C opt-level=3 -C codegen-units=1 -C strip=none -C debuginfo=2 -C lto=off";
        //[/rustflags]//
    }

    let builder = FlowBuilder::new();
    let (cluster, leader) = hydro_test::cluster::compute_pi::compute_pi(&builder, 8192);

    let frequency = 128;

    let built = builder.finalize();

    // Generate graphs if requested
    let _ = built.generate_graph_with_config(&args.graph, None);

    // If we're just generating a graph file, exit early
    if args.graph.file && args.graph.graph.is_some() {
        return;
    }

    let optimized = built.with_default_optimize();

    let _nodes = optimized
        //[trybuildhost]//
        .with_process(
            &leader,
            TrybuildHost::new(create_host(&mut deployment))
                .rustflags(rustflags)
                .additional_hydro_features(vec!["runtime_measure".to_string()])
                // ...
                //[/trybuildhost]//
                //[tracing]//
                .tracing(
                    TracingOptions::builder()
                        .perf_raw_outfile("leader.perf.data")
                        .samply_outfile("leader.profile")
                        .fold_outfile("leader.data.folded")
                        .flamegraph_outfile("leader.svg")
                        .frequency(frequency)
                        .setup_command(hydro_deploy::rust_crate::tracing_options::DEBIAN_PERF_SETUP_COMMAND)
                        .build(),
                ),
                //[/tracing]//
        )
        .with_cluster(
            &cluster,
            (0..8).map(|idx| {
                TrybuildHost::new(create_host(&mut deployment))
                    .rustflags(rustflags)
                    .additional_hydro_features(vec!["runtime_measure".to_string()])
                    .tracing(
                        TracingOptions::builder()
                            .perf_raw_outfile(format!("cluster{}.perf.data", idx))
                            .samply_outfile(format!("cluster{}.profile", idx))
                            .fold_outfile(format!("cluster{}.data.folded", idx))
                            .flamegraph_outfile(format!("cluster{}.svg", idx))
                            .frequency(frequency)
                            .setup_command(hydro_deploy::rust_crate::tracing_options::DEBIAN_PERF_SETUP_COMMAND)
                            .build(),
                    )
            }),
        )
        .deploy(&mut deployment);

    deployment.deploy().await.unwrap();

    deployment
        .start_until(async {
            std::io::stdin().read_line(&mut String::new()).unwrap();
        })
        .await
        .unwrap();
}
