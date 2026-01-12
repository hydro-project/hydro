use std::sync::Arc;

use clap::{ArgAction, Parser};
use hydro_deploy::gcp::GcpNetwork;
use hydro_deploy::{AwsNetwork, Deployment, Host};
use hydro_lang::deploy::TrybuildHost;
use hydro_lang::viz::config::GraphConfig;

type HostCreator = Box<dyn Fn(&mut Deployment) -> Arc<dyn Host>>;

#[derive(Parser, Debug)]
#[command(group(
    clap::ArgGroup::new("cloud")
        .args(&["gcp", "aws"])
        .multiple(false)
))]
struct Args {
    /// Use GCP instead of localhost (requires project name)
    #[clap(long)]
    gcp: Option<String>,

    /// Use AWS, make sure credentials are set up
    #[arg(long, action = ArgAction::SetTrue)]
    aws: bool,

    #[clap(flatten)]
    graph: GraphConfig,
}

// run with no args for localhost, with `--gcp <GCP PROJECT>` for GCP, with `--aws` for AWS
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
                .machine_type("e2-micro")
                .image("debian-cloud/debian-11")
                .region("us-west1-a")
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

    let rustflags = if args.gcp.is_some() || args.aws {
        "-C opt-level=3 -C codegen-units=1 -C strip=none -C debuginfo=2 -C lto=off -C link-args=--no-rosegment"
    } else {
        "-C opt-level=3 -C codegen-units=1 -C strip=none -C debuginfo=2 -C lto=off"
    };

    let builder = hydro_lang::compile::builder::FlowBuilder::new();
    let (process, cluster) = hydro_test::cluster::simple_cluster::simple_cluster(&builder);

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

    let _nodes = built
        .with_default_optimize()
        .with_process(
            &process,
            TrybuildHost::new(create_host(&mut deployment)).rustflags(rustflags),
        )
        .with_cluster(
            &cluster,
            (0..2).map(|_| TrybuildHost::new(create_host(&mut deployment)).rustflags(rustflags)),
        )
        .deploy(&mut deployment);
    deployment.run_ctrl_c().await.unwrap();
}
