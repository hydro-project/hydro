use std::collections::HashMap;
use std::str;
use std::sync::Arc;

use hydro_deploy::gcp::GcpNetwork;
use hydro_deploy::rust_crate::tracing_options::{DEBIAN_PERF_SETUP_COMMAND, TracingOptions};
use hydro_deploy::{Deployment, Host};
use hydro_lang::deploy::{DeployCrateWrapper, TrybuildHost};
use hydro_lang::ir::deep_clone;
use hydro_lang::q;
use hydro_lang::rewrites::analyze_counter::COUNTER_PREFIX;
use hydro_lang::rewrites::analyze_perf::CPU_USAGE_PREFIX;
use hydro_lang::rewrites::analyze_perf_and_counters::analyze_results;
use hydro_lang::rewrites::{insert_counter, persist_pullup};
use hydro_test::cluster::paxos::{CorePaxos, PaxosConfig};
use tokio::sync::RwLock;

type HostCreator = Box<dyn Fn(&mut Deployment, &str) -> Arc<dyn Host>>;

fn cluster_specs(
    host_arg: &str,
    project: Option<String>,
    network: Option<Arc<RwLock<GcpNetwork>>>,
    deployment: &mut Deployment,
    cluster_name: &str,
    num_nodes: usize,
) -> Vec<TrybuildHost> {
    let create_host: HostCreator = if host_arg == "gcp" {
        Box::new(move |deployment, cluster_name| -> Arc<dyn Host> {
            deployment
                .GcpComputeEngineHost()
                .project(project.as_ref().unwrap())
                .machine_type("n2-highcpu-2")
                .image("debian-cloud/debian-11")
                .region("us-west1-a")
                .network(network.as_ref().unwrap().clone())
                .display_name(cluster_name)
                .add()
        })
    } else {
        let localhost = deployment.Localhost();
        Box::new(move |_, _| -> Arc<dyn Host> { localhost.clone() })
    };

    let rustflags = if host_arg == "gcp" {
        "-C opt-level=3 -C codegen-units=1 -C strip=none -C debuginfo=2 -C lto=off -C link-args=--no-rosegment"
    } else {
        "-C opt-level=3 -C codegen-units=1 -C strip=none -C debuginfo=2 -C lto=off"
    };

    (0..num_nodes)
        .map(|idx| {
            TrybuildHost::new(create_host(deployment, &format!("{}{}", cluster_name, idx)))
                .additional_hydro_features(vec!["runtime_measure".to_string()])
                .rustflags(rustflags)
                .tracing(
                    TracingOptions::builder()
                        .perf_raw_outfile(format!("{}{}.perf.data", cluster_name, idx))
                        .fold_outfile(format!("{}{}.data.folded", cluster_name, idx))
                        .frequency(128)
                        .setup_command(DEBIAN_PERF_SETUP_COMMAND)
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
    let checkpoint_frequency = 1000; // Num log entries
    let i_am_leader_send_timeout = 5; // Sec
    let i_am_leader_check_timeout = 10; // Sec
    let i_am_leader_check_timeout_delay_multiplier = 15;

    let proposers = builder.cluster();
    let acceptors = builder.cluster();
    let clients = builder.cluster();
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
        &replicas,
    );

    let counter_output_duration = q!(std::time::Duration::from_secs(1));

    let optimized = builder
        .optimize_with(persist_pullup::persist_pullup)
        .optimize_with(|leaf| {
            insert_counter::insert_counter(leaf, counter_output_duration);
        });
    let mut ir = deep_clone(optimized.ir());

    let project = if host_arg == "gcp" {
        std::env::args().nth(2)
    } else {
        None
    };

    let network = project
        .as_ref()
        .map(|project| Arc::new(RwLock::new(GcpNetwork::new(project, None))));

    let nodes = optimized
        .with_cluster(
            &proposers,
            cluster_specs(
                &host_arg,
                project.clone(),
                network.clone(),
                &mut deployment,
                "proposer",
                f + 1,
            ),
        )
        .with_cluster(
            &acceptors,
            cluster_specs(
                &host_arg,
                project.clone(),
                network.clone(),
                &mut deployment,
                "acceptor",
                2 * f + 1,
            ),
        )
        .with_cluster(
            &clients,
            cluster_specs(
                &host_arg,
                project.clone(),
                network.clone(),
                &mut deployment,
                "client",
                num_clients,
            ),
        )
        .with_cluster(
            &replicas,
            cluster_specs(
                &host_arg,
                project.clone(),
                network.clone(),
                &mut deployment,
                "replica",
                f + 1,
            ),
        )
        .deploy(&mut deployment);

    deployment.deploy().await.unwrap();

    // Get stdout for each process to capture their CPU usage and cardinality later
    let mut usage_out = HashMap::new();
    let mut cardinality_out = HashMap::new();
    for (id, name, cluster) in nodes.get_all_clusters() {
        for (idx, node) in cluster.members().iter().enumerate() {
            let out = node.stdout_filter(CPU_USAGE_PREFIX).await;
            usage_out.insert((id.clone(), name.clone(), idx), out);

            let out = node.stdout_filter(COUNTER_PREFIX).await;
            cardinality_out.insert((id.clone(), name.clone(), idx), out);
        }
    }

    deployment
        .start_until(async {
            std::io::stdin().read_line(&mut String::new()).unwrap();
        })
        .await
        .unwrap();

    analyze_results(nodes, &mut ir, &mut usage_out, &mut cardinality_out).await;
    hydro_lang::ir::dbg_dedup_tee(|| {
        println!("{:#?}", ir);
    });
}
