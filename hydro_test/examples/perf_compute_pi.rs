use std::sync::{Arc, Mutex};
use std::time::Duration;

use clap::{ArgAction, Parser};
use hydro_deploy::aws::{AwsCloudwatchLogGroup, AwsEc2IamInstanceProfile};
use hydro_deploy::gcp::GcpNetwork;
use hydro_deploy::rust_crate::tracing_options::TracingOptions;
use hydro_deploy::{AwsNetwork, Deployment, Host};
use hydro_lang::deploy::TrybuildHost;
use hydro_lang::prelude::*;
use hydro_lang::telemetry::emf;
use hydro_lang::viz::config::GraphConfig;

type HostCreator = Box<dyn Fn(&mut Deployment) -> Arc<dyn Host>>;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None, group(
    clap::ArgGroup::new("cloud")
        .args(&["gcp", "aws"])
        .multiple(false)
))]
struct PerfComputePiArgs {
    #[command(flatten)]
    graph: GraphConfig,

    /// Include CPU tracing profiling (takes a long time to download)
    #[arg(long, action = ArgAction::SetTrue)]
    tracing: bool,

    /// Use GCP for deployment (provide project name)
    #[arg(long)]
    gcp: Option<String>,

    /// Use AWS, make sure credentials are set up
    #[arg(long, action = ArgAction::SetTrue)]
    aws: bool,
}

/// Run with `--tracing` to include CPU tracing. Add `--gcp <GCP PROJECT>` for GCP, or `--aws` for AWS.
///
/// ```bash
/// cargo run -p hydro_test --example perf_compute_pi -- --tracing --gcp my-gcp-project
/// ```
///
/// ```bash
/// cargo run -p hydro_test --example perf_compute_pi -- --tracing --aws
/// ```
///
/// Once the program is running, you can **press enter** to stop the program and see the results.
/// (Pressing Ctrl+C will stop the program **without cleaning up cloud resources** nor generating the
/// flamegraphs).
#[tokio::main]
async fn main() {
    let args = PerfComputePiArgs::parse();
    let mut deployment = Deployment::new();

    let (create_host, setup_command): (HostCreator, Option<&str>) = if let Some(project) = &args.gcp
    {
        let network = GcpNetwork::new(project, None);
        let project = project.clone();

        (
            Box::new(move |deployment| -> Arc<dyn Host> {
                deployment
                    .GcpComputeEngineHost()
                    .project(&project)
                    .machine_type("n2-standard-4")
                    .image("debian-cloud/debian-11")
                    .region("us-central1-c")
                    .network(network.clone())
                    .add()
            }),
            args.tracing
                .then_some(hydro_deploy::rust_crate::tracing_options::DEBIAN_PERF_SETUP_COMMAND),
        )
    } else if args.aws {
        let region = "us-east-1";
        let network = AwsNetwork::new(region, None);
        let iam_instance_profile = Arc::new(Mutex::new(
            AwsEc2IamInstanceProfile::new(region, None).add_cloudwatch_agent_server_policy_arn(),
        ));
        let cloudwatch_log_group = Arc::new(Mutex::new(AwsCloudwatchLogGroup::new(region, None)));

        (
            Box::new(move |deployment| -> Arc<dyn Host> {
                deployment
                    .AwsEc2Host()
                    .region(region)
                    .instance_type("t3.micro")
                    .ami("ami-0e95a5e2743ec9ec9") // Amazon Linux 2
                    .network(network.clone())
                    .iam_instance_profile(iam_instance_profile.clone())
                    .cloudwatch_log_group(cloudwatch_log_group.clone())
                    .add()
            }),
            args.tracing
                .then_some(hydro_deploy::rust_crate::tracing_options::AL2_PERF_SETUP_COMMAND),
        )
    } else {
        let localhost = deployment.Localhost();
        (
            Box::new(move |_| -> Arc<dyn Host> { localhost.clone() }),
            None,
        )
    };

    #[expect(
        clippy::needless_late_init,
        reason = "Better clarity for code extracted into docs."
    )]
    let rustflags;
    if args.gcp.is_some() || args.aws {
        //[rustflags_gcp]//
        rustflags = "-C opt-level=3 -C codegen-units=1 -C strip=none -C debuginfo=2 -C lto=off -C link-args=--no-rosegment";
        //[/rustflags_gcp]//
    } else {
        //[rustflags]//
        rustflags = "-C opt-level=3 -C codegen-units=1 -C strip=none -C debuginfo=2 -C lto=off";
        //[/rustflags]//
    }

    let mut builder = FlowBuilder::new();
    let (cluster, leader) = hydro_test::cluster::compute_pi::compute_pi(&mut builder, 8192);

    let frequency = 128;

    let built = builder.finalize();

    // Generate graphs if requested
    let _ = built.generate_graph_with_config(&args.graph, None);

    // If we're just generating a graph file, exit early
    if args.graph.should_exit_after_graph_generation() {
        return;
    }

    let mut optimized = if args.tracing {
        // With tracing.
        let setup_command = setup_command.unwrap_or_default();
        built.with_default_optimize()
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
                            .setup_command(setup_command)
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
                                .setup_command(setup_command)
                                .build(),
                        )
                }),
            )
    } else {
        // No tracing.
        built
            .with_default_optimize()
            .with_process(
                &leader,
                TrybuildHost::new(create_host(&mut deployment))
                    .rustflags(rustflags)
                    .additional_hydro_features(vec!["runtime_measure".to_string()]),
            )
            .with_cluster(
                &cluster,
                (0..8).map(|_| {
                    TrybuildHost::new(create_host(&mut deployment))
                        .rustflags(rustflags)
                        .additional_hydro_features(vec!["runtime_measure".to_string()])
                }),
            )
    };

    if args.aws {
        optimized = optimized.with_sidecar_all(
            &emf::RecordMetricsSidecar::builder()
                .interval(Duration::from_secs(5))
                .build(),
        );
    }

    let _nodes = optimized.deploy(&mut deployment);

    deployment.deploy().await.unwrap();

    deployment
        .start_until(async {
            std::io::stdin().read_line(&mut String::new()).unwrap();
        })
        .await
        .unwrap();
}
