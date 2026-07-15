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

    // Since this example measures throughput, use optimized builds, except when running as an
    // example test where skipping custom rustflags preserves the fast shared-prebuild /
    // dynamic-linking build path.
    let rustflags: Option<&str> = if std::env::var("RUNNING_AS_EXAMPLE_TEST")
        .is_ok_and(|v| v == "1")
    {
        None
    } else if args.gcp.is_some() || args.aws {
        Some(
            "-C opt-level=3 -C codegen-units=1 -C strip=none -C debuginfo=2 -C lto=off -C link-args=--no-rosegment",
        )
    } else {
        Some("-C opt-level=3 -C codegen-units=1 -C strip=none -C debuginfo=2 -C lto=off")
    };

    let create_trybuild_host = |host: Arc<dyn Host>| {
        let tbh = TrybuildHost::new(host).features(["tokio"]);
        if let Some(rustflags) = rustflags {
            tbh.rustflags(rustflags)
        } else {
            tbh
        }
    };

    let mut builder = hydro_lang::compile::builder::FlowBuilder::new();
    let (cluster, leader) = hydro_test::cluster::compute_pi::compute_pi(&mut builder, 8192);

    // Extract the IR for graph visualization
    let built = builder.finalize();

    match built.generate_graph(&args.graph) {
        Ok(Some(_)) => return,
        Ok(None) => {}
        Err(err) => {
            eprintln!("failed to generate graph: {err}");
            std::process::exit(1);
        }
    }

    let _nodes = built
        .with_default_optimize()
        .with_process(&leader, create_trybuild_host(create_host(&mut deployment)))
        .with_cluster(
            &cluster,
            (0..8).map(|_| create_trybuild_host(create_host(&mut deployment))),
        )
        .deploy(&mut deployment);

    deployment.run_ctrl_c().await.unwrap();
}

#[test]
fn test() {
    use example_test::run_current_example;

    let mut run = run_current_example!();
    run.read_regex(
        r"\[hydro_test::cluster::compute_pi::Leader \(process \S+\)\] pi: 3\.14\d+ \(\d{8,} trials\)",
    );
}
