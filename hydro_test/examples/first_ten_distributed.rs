use std::sync::Arc;

use clap::Parser;
use futures::SinkExt;
use hydro_deploy::gcp::GcpNetwork;
use hydro_deploy::{Deployment, Host};
use hydro_lang::deploy::TrybuildHost;
use hydro_lang::graph_util::GraphConfig;
use tokio::sync::RwLock;

type HostCreator = Box<dyn Fn(&mut Deployment) -> Arc<dyn Host>>;

#[derive(Parser, Debug)]
struct Args {
    /// Use GCP instead of localhost (requires project name)
    #[clap(long)]
    gcp: Option<String>,

    #[clap(flatten)]
    graph: GraphConfig,
}

// run with no args for localhost, with `--gcp <GCP PROJECT>` for GCP
#[tokio::main]
async fn main() {
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
    } else {
        let localhost = deployment.Localhost();
        (
            Box::new(move |_| -> Arc<dyn Host> { localhost.clone() }),
            "",
        )
    };

    let builder = hydro_lang::FlowBuilder::new();
    let external = builder.external_process();
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
