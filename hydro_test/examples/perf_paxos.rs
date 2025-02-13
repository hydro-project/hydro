use std::sync::Arc;

use hydro_deploy::gcp::GcpNetwork;
use hydro_deploy::hydroflow_crate::tracing_options::TracingOptions;
use hydro_deploy::{Deployment, Host};
use hydro_lang::deploy::{DeployCrateWrapper, TrybuildHost};
use hydro_lang::rewrites::analyze_perf::CPU_USAGE_PREFIX;
use hydro_lang::rewrites::{analyze_perf, persist_pullup};
use hydro_test::cluster::paxos::{CorePaxos, PaxosConfig};
use tokio::sync::RwLock;

type HostCreator = Box<dyn Fn(&mut Deployment) -> Arc<dyn Host>>;

#[tokio::main]
async fn main() {
    let mut deployment = Deployment::new();
    let host_arg = std::env::args().nth(1).unwrap_or_default();

    let rustflags = "-C opt-level=3 -C codegen-units=1 -C strip=none -C debuginfo=2 -C lto=off --cfg measure";
    let create_host: HostCreator = if host_arg == *"gcp" {
        let project = std::env::args().nth(2).unwrap();
        let network = Arc::new(RwLock::new(GcpNetwork::new(&project, None)));

        Box::new(move |deployment| -> Arc<dyn Host> {
            deployment
                .GcpComputeEngineHost()
                .project(&project)
                .machine_type("n2-highcpu-2")
                .image("debian-cloud/debian-11")
                .region("us-west1-a")
                .network(network.clone())
                .add()
        })
    } else {
        let localhost = deployment.Localhost();
        Box::new(move |_| -> Arc<dyn Host> { localhost.clone() })
    };

    let builder = hydro_lang::FlowBuilder::new();
    let f = 1;
    let num_clients = 1;
    let num_clients_per_node = 100; // Change based on experiment between 1, 50, 100.
    let median_latency_window_size = 1000;
    let checkpoint_frequency = 1000; // Num log entries
    let i_am_leader_send_timeout = 5; // Sec
    let i_am_leader_check_timeout = 10; // Sec
    let i_am_leader_check_timeout_delay_multiplier = 15;

    let proposers = builder.cluster();
    let acceptors = builder.cluster();

    let (clients, replicas) = hydro_test::cluster::paxos_bench::paxos_bench(
        &builder,
        num_clients_per_node,
        median_latency_window_size,
        checkpoint_frequency,
        f,
        f + 1,
        |replica_checkpoint| CorePaxos {
            proposers: proposers.clone(),
            acceptors: acceptors.clone(),
            replica_checkpoint: replica_checkpoint.broadcast_bincode(&acceptors),
            paxos_config: PaxosConfig {
                f,
                i_am_leader_send_timeout,
                i_am_leader_check_timeout,
                i_am_leader_check_timeout_delay_multiplier,
            },
        },
    );

    let frequency = 128;

    let nodes = builder
        .optimize_with(persist_pullup::persist_pullup)
        // .optimize_with(analyze_perf::analyze_perf)
        .with_cluster(
            &proposers,
            (0..f + 1)
                .map(|idx| TrybuildHost::new(create_host(&mut deployment)).rustflags(rustflags)
                .tracing(
                    TracingOptions::builder()
                        .perf_raw_outfile(format!("proposer{}.perf.data", idx))
                        .fold_outfile(format!("proposer{}.data.folded", idx))
                        .frequency(frequency)
                        .build(),
                ),
            ),
        )
        .with_cluster(
            &acceptors,
            (0..2 * f + 1)
                .map(|idx| TrybuildHost::new(create_host(&mut deployment)).rustflags(rustflags)
                .tracing(
                    TracingOptions::builder()
                        .perf_raw_outfile(format!("acceptor{}.perf.data", idx))
                        .fold_outfile(format!("acceptor{}.data.folded", idx))
                        .frequency(frequency)
                        .build(),
                ),
            ),
        )
        .with_cluster(
            &clients,
            (0..num_clients)
                .map(|idx| TrybuildHost::new(create_host(&mut deployment)).rustflags(rustflags)
                .tracing(TracingOptions::builder()
                    .perf_raw_outfile(format!("client{}.perf.data", idx))
                    .fold_outfile(format!("client{}.data.folded", idx))
                    .frequency(frequency)
                    .build(),
                ),
            ),
        )
        .with_cluster(
            &replicas,
            (0..f + 1)
                .map(|idx| TrybuildHost::new(create_host(&mut deployment)).rustflags(rustflags)
                .tracing(
                    TracingOptions::builder()
                        .perf_raw_outfile(format!("replica{}.perf.data", idx))
                        .fold_outfile(format!("replica{}.data.folded", idx))
                        .frequency(frequency)
                        .build(),
                ),
            ),
        )
        .deploy(&mut deployment);

    deployment.deploy().await.unwrap();

    let mut proposers_usage_out = vec![];
    let mut acceptors_usage_out = vec![];
    let mut clients_usage_out = vec![];
    let mut replicas_usage_out = vec![];

    for proposer in nodes.get_cluster(&proposers).members() {
        proposers_usage_out.push(proposer.stdout_filter(CPU_USAGE_PREFIX).await);
    }
    for acceptor in nodes.get_cluster(&acceptors).members() {
        acceptors_usage_out.push(acceptor.stdout_filter(CPU_USAGE_PREFIX).await);
    }
    for client in nodes.get_cluster(&clients).members() {
        clients_usage_out.push(client.stdout_filter(CPU_USAGE_PREFIX).await);
    }
    for replica in nodes.get_cluster(&replicas).members() {
        replicas_usage_out.push(replica.stdout_filter(CPU_USAGE_PREFIX).await);
    }

    deployment.start_until(async {
        std::io::stdin().read_line(&mut String::new()).unwrap();
    }).await.unwrap();

    for mut proposer_usage_out in proposers_usage_out {
        println!("Proposer {}", proposer_usage_out.recv().await.unwrap());
    }
    for mut acceptor_usage_out in acceptors_usage_out {
        println!("Acceptor {}", acceptor_usage_out.recv().await.unwrap());
    }
    for mut client_usage_out in clients_usage_out {
        println!("Client {}", client_usage_out.recv().await.unwrap());
    }
    for mut replica_usage_out in replicas_usage_out {
        println!("Replica {}", replica_usage_out.recv().await.unwrap());
    }
}
