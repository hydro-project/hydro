use std::collections::HashMap;
use std::sync::Arc;

use hydro_deploy::gcp::GcpNetwork;
use hydro_deploy::rust_crate::tracing_options::TracingOptions;
use hydro_deploy::{Deployment, Host};
use tokio::sync::mpsc::UnboundedReceiver;
use tokio::sync::RwLock;

use super::populate_metadata::inject_id;
use super::remove_counter::remove_counter;
use crate::builder::deploy::DeployResult;
use crate::deploy::deploy_graph::DeployCrateWrapper;
use crate::deploy::{HydroDeploy, TrybuildHost};
use crate::ir::HydroLeaf;
use crate::location::LocationId;
use crate::rewrites::populate_metadata::{
    inject_count, inject_perf, parse_counter_usage, parse_cpu_usage, COUNTER_PREFIX,
    CPU_USAGE_PREFIX,
};

type HostCreator = Box<dyn Fn(&mut Deployment) -> Arc<dyn Host>>;

pub fn perf_process_specs(
    host_arg: &str,
    project: String,
    network: Arc<RwLock<GcpNetwork>>,
    deployment: &mut Deployment,
    process_name: &str,
) -> TrybuildHost {
    perf_cluster_specs(host_arg, project, network.clone(), deployment, process_name, 1)
        .into_iter().next()
        .unwrap()
}

pub fn perf_cluster_specs(
    host_arg: &str,
    project: String,
    network: Arc<RwLock<GcpNetwork>>,
    deployment: &mut Deployment,
    cluster_name: &str,
    num_nodes: usize,
) -> Vec<TrybuildHost> {
    let create_host: HostCreator = if host_arg == "gcp" {
        Box::new(move |deployment| -> Arc<dyn Host> {
            let startup_script = "sudo sh -c 'apt update && apt install -y linux-perf binutils && echo -1 > /proc/sys/kernel/perf_event_paranoid && echo 0 > /proc/sys/kernel/kptr_restrict'";
            deployment
                .GcpComputeEngineHost()
                .project(&project)
                .machine_type("n2-standard-4")
                .image("debian-cloud/debian-11")
                .region("us-central1-c")
                .network(network.clone())
                .startup_script(startup_script)
                .add()
        })
    } else {
        let localhost = deployment.Localhost();
        Box::new(move |_| -> Arc<dyn Host> { localhost.clone() })
    };

    let rustflags = "-C opt-level=3 -C codegen-units=1 -C strip=none -C debuginfo=2 -C lto=off";

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

pub async fn track_process_usage_cardinality(
    process: &impl DeployCrateWrapper,
) -> (UnboundedReceiver<String>, UnboundedReceiver<String>) {
    (
        process.stdout_filter(CPU_USAGE_PREFIX).await,
        process.stdout_filter(COUNTER_PREFIX).await,
    )
}

pub async fn track_cluster_usage_cardinality(
    nodes: &DeployResult<'static, HydroDeploy>,
) -> (
    HashMap<(LocationId, String, usize), UnboundedReceiver<String>>,
    HashMap<(LocationId, String, usize), UnboundedReceiver<String>>,
) {
    let mut usage_out = HashMap::new();
    let mut cardinality_out = HashMap::new();
    for (id, name, cluster) in nodes.get_all_clusters() {
        for (idx, node) in cluster.members().iter().enumerate() {
            let (node_usage_out, node_cardinality_out) =
                track_process_usage_cardinality(node).await;
            usage_out.insert((id.clone(), name.clone(), idx), node_usage_out);
            cardinality_out.insert((id.clone(), name.clone(), idx), node_cardinality_out);
        }
    }
    (usage_out, cardinality_out)
}

pub async fn analyze_process_results(
    process: &impl DeployCrateWrapper,
    ir: &mut [HydroLeaf],
    _node_usage: f64,
    node_cardinality: &mut UnboundedReceiver<String>,
) {
    // TODO: Integrate CPU usage into perf usage stats (so we also consider idle time)
    if let Some(perf_results) = process.tracing_results().await {
        // Inject perf usages into metadata
        inject_perf(ir, perf_results.folded_data);

        // Get cardinality data. Allow later values to overwrite earlier ones
        let mut op_to_counter = HashMap::new();
        while let Some(measurement) = node_cardinality.recv().await {
            let (op_id, count) = parse_counter_usage(measurement);
            op_to_counter.insert(op_id, count);
        }
        inject_count(ir, &op_to_counter);
    }
}

pub async fn analyze_cluster_results(
    nodes: &DeployResult<'static, HydroDeploy>,
    ir: &mut [HydroLeaf],
    usage_out: &mut HashMap<(LocationId, String, usize), UnboundedReceiver<String>>,
    cardinality_out: &mut HashMap<(LocationId, String, usize), UnboundedReceiver<String>>,
) {
    for (id, name, cluster) in nodes.get_all_clusters() {
        println!("Analyzing cluster {:?}: {}", id, name);
        
        // Iterate through nodes' usages and keep the max usage one
        let mut max_usage = None;
        for (idx, _) in cluster.members().iter().enumerate() {
            let usage =
                get_usage(usage_out.get_mut(&(id.clone(), name.clone(), idx)).unwrap()).await;
            if let Some((prev_usage, _)) = max_usage {
                if usage > prev_usage {
                    max_usage = Some((usage, idx));
                }
            } else {
                max_usage = Some((usage, idx));
            }
        }

        if let Some((usage, idx)) = max_usage {
            let node_cardinality = cardinality_out
                .get_mut(&(id.clone(), name.clone(), idx))
                .unwrap();
            analyze_process_results(
                cluster.members().get(idx).unwrap(),
                ir,
                usage,
                node_cardinality,
            )
            .await;
        }
    }
}

pub async fn get_usage(usage_out: &mut UnboundedReceiver<String>) -> f64 {
    let measurement = usage_out.recv().await.unwrap();
    parse_cpu_usage(measurement)
}

pub fn cleanup_after_analysis(ir: &mut [HydroLeaf]) {
    // Remove HydroNode::Counter (since we don't want to consider decoupling those)
    remove_counter(ir);
    // Inject new next_stmt_id into metadata (old ones are invalid after removing the counter)
    inject_id(ir);
}
