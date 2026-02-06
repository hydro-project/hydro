use std::sync::Arc;

use clap::{ArgAction, Parser};
use hydro_deploy::gcp::GcpNetwork;
use hydro_deploy::{AwsNetwork, Deployment, Host};
use hydro_lang::deploy::TrybuildHost;
use hydro_lang::viz::config::GraphConfig;
use hydro_test::cluster::paxos::{CorePaxos, PaxosConfig};

type HostCreator = Box<dyn Fn(&mut Deployment) -> Arc<dyn Host>>;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None, group(
    clap::ArgGroup::new("cloud")
        .args(&["gcp", "aws"])
        .multiple(false)
))]
struct Args {
    #[command(flatten)]
    graph: GraphConfig,

    /// Use GCP for deployment (provide project name)
    #[arg(long)]
    gcp: Option<String>,

    /// Use AWS, make sure credentials are set up
    #[arg(long, action = ArgAction::SetTrue)]
    aws: bool,
}

#[tokio::main]
async fn main() {
    let args = Args::parse();
    let mut deployment = Deployment::new();

    let create_host: HostCreator = if let Some(project) = &args.gcp {
        let network = GcpNetwork::new(project, None);
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
    } else if args.aws {
        let region = "us-east-1";
        let network = AwsNetwork::new(region, None);

        Box::new(move |deployment| -> Arc<dyn Host> {
            deployment
                .AwsEc2Host()
                .region(region)
                .instance_type("t3.micro")
                .ami("ami-0e95a5e2743ec9ec9") // Amazon Linux 2
                .network(network.clone())
                .add()
        })
    } else {
        let localhost = deployment.Localhost();
        Box::new(move |_| -> Arc<dyn Host> { localhost.clone() })
    };

    let mut builder = hydro_lang::compile::builder::FlowBuilder::new();
    let f = 1;
    let num_clients = 3;
    let num_clients_per_node = 100; // Change based on experiment between 1, 50, 100.
    let checkpoint_frequency = 1000; // Num log entries
    let i_am_leader_send_timeout = 5; // Sec
    let i_am_leader_check_timeout = 10; // Sec
    let i_am_leader_check_timeout_delay_multiplier = 15;
    let print_result_frequency = 1000; // Millis

    let proposers = builder.cluster();
    let acceptors = builder.cluster();
    let clients = builder.cluster();
    let client_aggregator = builder.process();
    let replicas = builder.cluster();

    hydro_test::cluster::paxos_bench::paxos_bench(
        num_clients_per_node,
        checkpoint_frequency,
        f,
        f + 1,
        CorePaxos {
            proposers: proposers.clone(),
            acceptors: acceptors.clone(),
            paxos_config: PaxosConfig {
                f,
                i_am_leader_send_timeout,
                i_am_leader_check_timeout,
                i_am_leader_check_timeout_delay_multiplier,
            },
        },
        &clients,
        &client_aggregator,
        &replicas,
        print_result_frequency / 10,
        print_result_frequency,
        hydro_std::bench_client::pretty_print_bench_results,
    );

    // Extract the IR for graph visualization
    let built = builder.finalize();

    // Generate graph visualizations based on command line arguments
    if let Err(e) = built.generate_graph_with_config(&args.graph, None) {
        eprintln!("Error generating graph: {}", e);
    }

    // If we're just generating a graph file, exit early
    if args.graph.should_exit_after_graph_generation() {
        return;
    }

    // Optimize the flow before deployment to remove marker nodes
    let optimized = built.with_default_optimize();

    let rustflags = if args.gcp.is_some() || args.aws {
        "-C opt-level=3 -C codegen-units=1 -C strip=none -C debuginfo=2 -C lto=off -C link-args=--no-rosegment"
    } else {
        "-C opt-level=3 -C codegen-units=1 -C strip=none -C debuginfo=2 -C lto=off"
    };

    let _nodes = optimized
        .with_cluster(
            &proposers,
            (0..f + 1)
                .map(|_| TrybuildHost::new(create_host(&mut deployment)).rustflags(rustflags)),
        )
        .with_cluster(
            &acceptors,
            (0..2 * f + 1)
                .map(|_| TrybuildHost::new(create_host(&mut deployment)).rustflags(rustflags)),
        )
        .with_cluster(
            &clients,
            (0..num_clients)
                .map(|_| TrybuildHost::new(create_host(&mut deployment)).rustflags(rustflags)),
        )
        .with_process(
            &client_aggregator,
            TrybuildHost::new(create_host(&mut deployment)).rustflags(rustflags),
        )
        .with_cluster(
            &replicas,
            (0..f + 1)
                .map(|_| TrybuildHost::new(create_host(&mut deployment)).rustflags(rustflags)),
        )
        .deploy(&mut deployment);

    deployment.deploy().await.unwrap();

    deployment.start().await.unwrap();

    tokio::signal::ctrl_c().await.unwrap();
}
