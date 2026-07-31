use std::sync::Arc;

use clap::{ArgAction, Parser};
use hydro_deploy::gcp::GcpNetwork;
use hydro_deploy::rust_crate::tracing_options::{
    AL2_PERF_SETUP_COMMAND, DEBIAN_PERF_SETUP_COMMAND, TracingOptions,
};
use hydro_deploy::{AwsNetwork, Deployment, Host};
use hydro_lang::deploy::TrybuildHost;
use hydro_lang::location::Location;
use hydro_lang::location::cluster::CLUSTER_SELF_ID;
use hydro_lang::nondet::nondet;
use hydro_lang::prelude::TCP;
use hydro_lang::viz::config::GraphConfig;
use hydro_test::cluster::raft::{RaftConfig, Replica, raft};
use stageleft::q;

type HostCreator = Box<dyn Fn(&mut Deployment) -> Arc<dyn Host>>;

const CLUSTER_SIZE: usize = 3;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None, group(
    clap::ArgGroup::new("cloud")
        .args(&["gcp", "aws"])
        .multiple(false)
))]
struct Args {
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
    let replicas = builder.cluster::<Replica>();

    // Each member submits a request of its own every second; only requests received
    // by the current leader are appended (the rest surface on `redirected` with a
    // leader hint, which a production caller would use to forward them).
    let requests = replicas
        .source_interval(q!(std::time::Duration::from_secs(1)))
        .map(q!(move |_| format!(
            "hello from member {}",
            CLUSTER_SELF_ID.clone().into_tagless()
        )));

    // Election timeouts are deliberately skewed per member (RAFT's randomized-timeout
    // tie-breaking): distinct periods guarantee two members' timers cannot fire
    // simultaneously forever. Heartbeats are much faster than the shortest election
    // timeout, so a live leader suppresses follower elections.
    let election_timer_interrupts = replicas.source_interval(q!(std::time::Duration::from_millis(
        500 + u64::from(CLUSTER_SELF_ID.get_raw_id()) * 130
    )));
    let heartbeat_timer_interrupts =
        replicas.source_interval(q!(std::time::Duration::from_millis(100)));

    let (committed, redirected) = raft(
        requests,
        election_timer_interrupts,
        heartbeat_timer_interrupts,
        RaftConfig {
            cluster_size: CLUSTER_SIZE,
        },
        || TCP.fail_stop().bincode(),
        nondet!(
            /// Which member leads and how concurrent requests are interleaved in the
            /// log is inherently non-deterministic; every member still prints the
            /// same committed sequence.
        ),
    );

    committed
        .end_atomic()
        .weaken_consistency()
        .for_each(q!(|entry| println!(
            "committed [term {}, index {}]: {}",
            entry.term_received, entry.index, entry.message
        )));

    redirected.for_each(q!(|(request, leader_hint)| println!(
        "redirected: {request:?} (leader hint: {leader_hint:?})"
    )));

    // Extract the IR for graph visualization
    let built = builder.finalize();

    match built.generate_graph(&args.graph) {
        Ok(Some(_)) => return,
        Ok(None) => {}
        Err(err) => {
            eprintln!("Failed to generate graph: {err}");
            std::process::exit(1);
        }
    }

    // Optimize the flow before deployment to remove marker nodes
    let optimized = built.with_default_optimize();

    let rustflags = if args.gcp.is_some() || args.aws {
        "-C opt-level=3 -C codegen-units=1 -C strip=none -C debuginfo=2 -C lto=off -C link-args=--no-rosegment"
    } else {
        "-C opt-level=3 -C codegen-units=1 -C strip=none -C debuginfo=2 -C lto=off"
    };

    let frequency = 128;
    let setup_command = if args.gcp.is_some() {
        DEBIAN_PERF_SETUP_COMMAND
    } else if args.aws {
        AL2_PERF_SETUP_COMMAND
    } else {
        ""
    };
    let create_trybuild_host = |host: Arc<dyn Host + 'static>, name: &str, i: usize| {
        let mut tbh = TrybuildHost::new(host).rustflags(rustflags);
        // Pin to core 0 on remote machines
        if args.gcp.is_some() || args.aws {
            tbh = tbh.pin_to_core(0);
        }
        if args.tracing {
            tbh = tbh.tracing(
                TracingOptions::builder()
                    .perf_raw_outfile(format!("{name}{i}.perf.data"))
                    .samply_outfile(format!("{name}{i}.profile"))
                    .fold_outfile(format!("{name}{i}.data.folded"))
                    .flamegraph_outfile(format!("{name}{i}.svg"))
                    .frequency(frequency)
                    .setup_command(setup_command)
                    .build(),
            );
        }
        tbh
    };

    let _nodes = optimized
        .with_cluster(
            &replicas,
            (0..CLUSTER_SIZE)
                .map(|i| create_trybuild_host(create_host(&mut deployment), "replicas", i)),
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
