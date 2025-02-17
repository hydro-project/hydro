use std::collections::HashMap;
use std::sync::Arc;

use hydro_deploy::gcp::GcpNetwork;
use hydro_deploy::hydroflow_crate::tracing_options::TracingOptions;
use hydro_deploy::{Deployment, Host};
use hydro_lang::deploy::{DeployCrateWrapper, TrybuildHost};
use hydro_lang::ir::deep_clone;
use hydro_lang::rewrites::analyze_perf::{analyze_perf, parse_cpu_usage, CPU_USAGE_PREFIX};
use hydro_lang::rewrites::persist_pullup;
use hydro_test::cluster::paxos::{CorePaxos, PaxosConfig};
use tokio::sync::RwLock;

type HostCreator = Box<dyn Fn(&mut Deployment) -> Arc<dyn Host>>;

fn cluster_specs(
    host_arg: &str,
    deployment: &mut Deployment,
    cluster_name: &str,
    num_nodes: usize,
) -> Vec<TrybuildHost> {
    let create_host: HostCreator = if host_arg == "gcp" {
        let project = std::env::args().nth(2).unwrap();
        let network = Arc::new(RwLock::new(GcpNetwork::new(&project, None)));

        Box::new(move |deployment| -> Arc<dyn Host> {
            let startup_script = "sudo sh -c 'apt update && apt install -y linux-perf binutils && echo -1 > /proc/sys/kernel/perf_event_paranoid && echo 0 > /proc/sys/kernel/kptr_restrict'";
            deployment
                .GcpComputeEngineHost()
                .project(&project)
                .machine_type("n2-highcpu-2")
                .image("debian-cloud/debian-11")
                .region("us-west1-a")
                .network(network.clone())
                .startup_script(startup_script)
                .add()
        })
    } else {
        let localhost = deployment.Localhost();
        Box::new(move |_| -> Arc<dyn Host> { localhost.clone() })
    };

    let rustflags =
        "-C opt-level=3 -C codegen-units=1 -C strip=none -C debuginfo=2 -C lto=off";

    (0..num_nodes)
        .map(|idx| {
            TrybuildHost::new(create_host(deployment))
                .additional_hydro_features(vec!["runtime_measure".to_string()])
                .rustflags(rustflags)
                .tracing(
                    TracingOptions::builder()
                        .perf_raw_outfile(format!("{}{}.perf.data", cluster_name, idx))
                        .fold_outfile(format!("{}{}.data.folded", cluster_name, idx))
                        .frequency(128)
                        .build(),
                )
        })
        .collect()
}

#[tokio::main]
async fn main() {
    let mut deployment = Deployment::new();
    let host_arg = std::env::args().nth(1).unwrap_or_default();

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

    let optimized = builder.optimize_with(persist_pullup::persist_pullup);
    let mut ir = deep_clone(optimized.ir());
    let nodes = optimized
        .with_cluster(
            &proposers,
            cluster_specs(&host_arg, &mut deployment, "proposer", f + 1),
        )
        .with_cluster(
            &acceptors,
            cluster_specs(&host_arg, &mut deployment, "acceptor", 2 * f + 1),
        )
        .with_cluster(
            &clients,
            cluster_specs(&host_arg, &mut deployment, "client", num_clients),
        )
        .with_cluster(
            &replicas,
            cluster_specs(&host_arg, &mut deployment, "replica", f + 1),
        )
        .deploy(&mut deployment);

    deployment.deploy().await.unwrap();

    // Get stdout for each process to capture their CPU usage later
    let mut usage_out = HashMap::new();
    for (id, name, cluster) in nodes.get_all_clusters() {
        for (idx, node) in cluster.members().iter().enumerate() {
            let out = node.stdout_filter(CPU_USAGE_PREFIX).await;
            usage_out.insert((id.clone(), name.clone(), idx), out);
        }
    }

    deployment
        .start_until(async {
            std::io::stdin().read_line(&mut String::new()).unwrap();
        })
        .await
        .unwrap();

    // Re-analyze the IR using perf data from each node
    for (id, name, cluster) in nodes.get_all_clusters() {
        // Iterate through nodes' usages and keep the max usage one
        let mut max_usage = None;
        for (idx, _) in cluster.members().iter().enumerate() {
            let measurement = usage_out
                .get_mut(&(id.clone(), name.clone(), idx))
                .unwrap()
                .recv()
                .await
                .unwrap();
            println!("{} {} {}", &name, idx, measurement);
            let usage = parse_cpu_usage(measurement);
            if let Some((prev_usage, _)) = max_usage {
                if usage > prev_usage {
                    max_usage = Some((usage, idx));
                }
            } else {
                max_usage = Some((usage, idx));
            }
        }

        if let Some((usage, idx)) = max_usage {
            if let Some(perf_results) = cluster.members().get(idx).unwrap().tracing_results().await
            {
                println!("{}: {}", &name, usage);
                analyze_perf(&mut ir, perf_results.folded_data);
            }
        }
    }
}
