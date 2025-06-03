use std::collections::HashMap;

use tokio::sync::mpsc::UnboundedReceiver;

use super::populate_metadata::inject_id;
use super::remove_counter::remove_counter;
use crate::builder::deploy::DeployResult;
use crate::deploy::deploy_graph::DeployCrateWrapper;
use crate::deploy::HydroDeploy;
use crate::ir::HydroLeaf;
use crate::location::LocationId;
use crate::rewrites::populate_metadata::{
    inject_count, inject_perf, parse_counter_usage, parse_cpu_usage,
};
use crate::internal_constants::{
    CPU_USAGE_PREFIX, COUNTER_PREFIX,
};

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
    for (id, name, process) in nodes.get_all_processes() {
        let (process_usage_out, process_cardinality_out) =
            track_process_usage_cardinality(process).await;
        usage_out.insert((id.clone(), name.clone(), 0), process_usage_out);
        cardinality_out.insert((id.clone(), name.clone(), 0), process_cardinality_out);
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
) -> (LocationId, usize) {
    let mut max_usage_cluster_id = None;
    let mut max_usage_cluster_size = 0;
    let mut max_usage_overall = 0f64;

    for (id, name, cluster) in nodes.get_all_clusters() {
        println!("Analyzing cluster {:?}: {}", id, name);
        
        // Iterate through nodes' usages and keep the max usage one
        let mut max_usage = None;
        for (idx, _) in cluster.members().iter().enumerate() {
            let usage =
                get_usage(usage_out.get_mut(&(id.clone(), name.clone(), idx)).unwrap()).await;
            println!("Node {} usage: {}", idx, usage);
            if let Some((prev_usage, _)) = max_usage {
                if usage > prev_usage {
                    max_usage = Some((usage, idx));
                }
            } else {
                max_usage = Some((usage, idx));
            }
        }

        if let Some((usage, idx)) = max_usage {
            // Modify IR with perf & cardinality numbers
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

            // Update cluster with max usage
            if max_usage_overall < usage {
                max_usage_cluster_id = Some(id.clone());
                max_usage_cluster_size = cluster.members().len();
                max_usage_overall = usage;
                println!("The bottleneck is {}", name);
            }
        }
    }

    (max_usage_cluster_id.unwrap(), max_usage_cluster_size)
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
