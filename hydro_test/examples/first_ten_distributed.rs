use std::sync::Arc;

use clap::{ArgAction, Parser};
use futures::SinkExt;
use hydro_deploy::aws::AwsNetwork;
use hydro_deploy::gcp::GcpNetwork;
use hydro_deploy::{Deployment, Host};
use hydro_lang::deploy::TrybuildHost;
use hydro_lang::graph::config::GraphConfig;
use tokio::sync::RwLock;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

type HostCreator = Box<dyn Fn(&mut Deployment) -> Arc<dyn Host>>;

#[derive(Parser, Debug)]
struct Args {
    /// Use GCP instead of localhost (requires project name)
    #[clap(long)]
    gcp: Option<String>,

    /// use AWS, make sure credentials are set up
    #[arg(short, long, action = ArgAction::SetTrue)]
    aws: bool,

    #[clap(flatten)]
    graph: GraphConfig,
}

// run with no args for localhost, with `--gcp <GCP PROJECT>` for GCP, with `--aws=true` for AWS.
#[tokio::main]
async fn main() {
    let subscriber = tracing_subscriber::fmt::layer().with_target(false);

    let filter_layer = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("trace"));

    tracing_subscriber::registry()
        .with(filter_layer)
        .with(subscriber)
        .init();

    let args = Args::parse();
    let mut deployment = Deployment::new();

    let (create_host, rustflags): (HostCreator, &'static str) = if let Some(project) = args.gcp {
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
    } else if args.aws {
        let network = Arc::new(RwLock::new(AwsNetwork::new("us-east-1", None)));

        (
            Box::new(move |deployment| -> Arc<dyn Host> {
                deployment
                    .AwsEc2Host()
                    .region("us-east-1")
                    .instance_type("t3.micro")
                    .ami("ami-0e95a5e2743ec9ec9") // Amazon Linux 2
                    .network(network.clone())
                    .add()
            }),
            "-C opt-level=3 -C codegen-units=1 -C strip=debuginfo -C debuginfo=0 -C lto=off",
        )
    } else {
        let localhost = deployment.Localhost();
        (
            Box::new(move |_| -> Arc<dyn Host> { localhost.clone() }),
            "",
        )
    };

    let builder = hydro_lang::compile::builder::FlowBuilder::new();
    let external = builder.external();
    let p1 = builder.process();
    let p2 = builder.process();
    let external_port =
        hydro_test::distributed::first_ten::first_ten_distributed(&external, &p1, &p2);

    // Extract the IR for graph visualization
    let built = builder.finalize();

    // Generate graph visualizations based on command line arguments
    if let Err(e) = built.generate_graph_with_config(&args.graph, None) {
        eprintln!("Error generating graph: {}", e);
    }

    // Optimize the flow before deployment to remove marker nodes
    let optimized = built.with_default_optimize();

    let nodes = optimized
        .with_process(
            &p1,
            TrybuildHost::new(create_host(&mut deployment)).rustflags(rustflags),
        )
        .with_process(
            &p2,
            TrybuildHost::new(create_host(&mut deployment)).rustflags(rustflags),
        )
        .with_external(&external, deployment.Localhost())
        .deploy(&mut deployment);

    deployment.deploy().await.unwrap();

    let mut external_port = nodes.connect_sink_bincode(external_port).await;

    deployment.start().await.unwrap();

    println!("Enter characters and press enter to send them over the network (ctrl-d to stop):");
    loop {
        let mut in_line = String::new();
        if std::io::stdin().read_line(&mut in_line).unwrap() == 0 {
            break;
        }

        external_port.send(in_line).await.unwrap();
    }

    deployment.stop().await.unwrap();
}

#[test]
fn test() {
    use example_test::run_current_example;

    let mut run = run_current_example!();
    run.read_string(
        "Enter characters and press enter to send them over the network (ctrl-d to stop):",
    );
    run.read_string("[hydro_test::distributed::first_ten::P2 (process 2)] 9");
    run.write_line("Hello World");
    run.read_string(r#"[hydro_test::distributed::first_ten::P1 (process 1)] hi: "Hello World\n"#);
}
