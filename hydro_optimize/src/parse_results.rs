use std::collections::HashMap;

use hydro_lang::compile::deploy::DeployResult;
use hydro_lang::compile::ir::{HydroNode, HydroRoot, traverse_dfir};
use hydro_lang::deploy::HydroDeploy;
use hydro_lang::deploy::deploy_graph::DeployCrateWrapper;
use hydro_lang::location::dynamic::LocationId;
use regex::Regex;
use tokio::sync::mpsc::UnboundedReceiver;

use crate::rewrites::{NetworkType, get_network_type};

pub fn parse_cpu_usage(measurement: String) -> f64 {
    let regex = Regex::new(r"Total (\d+\.\d+)%").unwrap();
    regex
        .captures_iter(&measurement)
        .last()
        .map(|cap| cap[1].parse::<f64>().unwrap())
        .unwrap_or(0f64)
}

/// Returns a map from (operator ID, is network receiver) to percentage of total samples.
fn parse_perf(file: String) -> HashMap<(usize, bool), f64> {
    let mut total_samples = 0f64;
    let mut unidentified_samples = 0f64;
    let mut samples_per_id = HashMap::new();
    let operator_regex = Regex::new(r"op_\d+v\d+__(.*?)__(send)?(recv)?(\d+)").unwrap();

    for line in file.lines() {
        let n_samples_index = line.rfind(' ').unwrap() + 1;
        let n_samples = &line[n_samples_index..].parse::<f64>().unwrap();

        if let Some(cap) = operator_regex.captures_iter(line).last() {
            let id = cap[4].parse::<usize>().unwrap();
            let is_network_recv = cap
                .get(3)
                .is_some_and(|direction| direction.as_str() == "recv");

            let dfir_operator_and_samples =
                samples_per_id.entry((id, is_network_recv)).or_insert(0.0);
            *dfir_operator_and_samples += n_samples;
        } else {
            unidentified_samples += n_samples;
        }
        total_samples += n_samples;
    }

    println!(
        "Out of {} samples, {} were unidentified, {}%",
        total_samples,
        unidentified_samples,
        unidentified_samples / total_samples * 100.0
    );

    samples_per_id
        .iter_mut()
        .for_each(|(_, samples)| *samples /= total_samples);
    samples_per_id
}

fn inject_perf_root(
    root: &mut HydroRoot,
    id_to_usage: &HashMap<(usize, bool), f64>,
    next_stmt_id: &mut usize,
) {
    if let Some(cpu_usage) = id_to_usage.get(&(*next_stmt_id, false)) {
        root.op_metadata_mut().cpu_usage = Some(*cpu_usage);
    }
}

fn inject_perf_node(
    node: &mut HydroNode,
    id_to_usage: &HashMap<(usize, bool), f64>,
    next_stmt_id: &mut usize,
) {
    if let Some(cpu_usage) = id_to_usage.get(&(*next_stmt_id, false)) {
        node.op_metadata_mut().cpu_usage = Some(*cpu_usage);
    }
    // If this is a Network node, separately get receiver CPU usage
    if let HydroNode::Network { metadata, .. } = node
        && let Some(cpu_usage) = id_to_usage.get(&(*next_stmt_id, true))
    {
        metadata.op.network_recv_cpu_usage = Some(*cpu_usage);
    }
}

pub fn inject_perf(ir: &mut [HydroRoot], folded_data: Vec<u8>) {
    let id_to_usage = parse_perf(String::from_utf8(folded_data).unwrap());
    traverse_dfir(
        ir,
        |leaf, next_stmt_id| {
            inject_perf_root(leaf, &id_to_usage, next_stmt_id);
        },
        |node, next_stmt_id| {
            inject_perf_node(node, &id_to_usage, next_stmt_id);
        },
    );
}

/// Returns (op_id, count)
pub fn parse_counter_usage(measurement: String) -> (usize, usize) {
    let regex = Regex::new(r"\((\d+)\): (\d+)").unwrap();
    let matches = regex.captures_iter(&measurement).last().unwrap();
    let op_id = matches[1].parse::<usize>().unwrap();
    let count = matches[2].parse::<usize>().unwrap();
    (op_id, count)
}

fn inject_count_node(
    node: &mut HydroNode,
    next_stmt_id: &mut usize,
    op_to_count: &HashMap<usize, usize>,
) {
    if let Some(count) = op_to_count.get(next_stmt_id) {
        let metadata = node.metadata_mut();
        metadata.cardinality = Some(*count);
    } else {
        match node {
            HydroNode::Tee { inner ,metadata, .. } => {
                metadata.cardinality = inner.0.borrow().metadata().cardinality;
            }
            | HydroNode::Map { input, metadata, .. } // Equal to parent cardinality
            | HydroNode::DeferTick { input, metadata, .. } // Equal to parent cardinality
            | HydroNode::Enumerate { input, metadata, .. }
            | HydroNode::Inspect { input, metadata, .. }
            | HydroNode::Sort { input, metadata, .. }
            | HydroNode::Counter { input, metadata, .. }
            => {
                metadata.cardinality = input.metadata().cardinality;
            }
            _ => {}
        }
    }
}

pub fn inject_count(ir: &mut [HydroRoot], op_to_count: &HashMap<usize, usize>) {
    traverse_dfir(
        ir,
        |_, _| {},
        |node, next_stmt_id| {
            inject_count_node(node, next_stmt_id, op_to_count);
        },
    );
}

pub async fn analyze_process_results(
    process: &impl DeployCrateWrapper,
    ir: &mut [HydroRoot],
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
    nodes: &DeployResult<'_, HydroDeploy>,
    ir: &mut [HydroRoot],
    usage_out: &mut HashMap<(LocationId, String, usize), UnboundedReceiver<String>>,
    cardinality_out: &mut HashMap<(LocationId, String, usize), UnboundedReceiver<String>>,
    exclude_from_decoupling: Vec<String>,
) -> (LocationId, String, usize) {
    let mut max_usage_cluster_id = None;
    let mut max_usage_cluster_size = 0;
    let mut max_usage_cluster_name = String::new();
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
            if max_usage_overall < usage && !exclude_from_decoupling.contains(&name) {
                max_usage_cluster_id = Some(id.clone());
                max_usage_cluster_name = name.clone();
                max_usage_cluster_size = cluster.members().len();
                max_usage_overall = usage;
                println!("The bottleneck is {}", name);
            }
        }
    }

    (
        max_usage_cluster_id.unwrap(),
        max_usage_cluster_name,
        max_usage_cluster_size,
    )
}

pub async fn get_usage(usage_out: &mut UnboundedReceiver<String>) -> f64 {
    let measurement = usage_out.recv().await.unwrap();
    parse_cpu_usage(measurement)
}

fn analyze_overheads_node(
    node: &mut HydroNode,
    _next_stmt_id: &mut usize,
    max_send_overhead: &mut f64,
    max_recv_overhead: &mut f64,
    location: &LocationId,
) {
    let metadata = node.metadata();
    let network_type = get_network_type(node, location.root().raw_id());
    match network_type {
        Some(NetworkType::Send) | Some(NetworkType::SendRecv) => {
            if let Some(cpu_usage) = metadata.op.cpu_usage {
                // Use cardinality from the network's input, not the network itself.
                // Reason: Cardinality is measured at ONE recipient, but the sender may be sending to MANY machines.
                if let Some(cardinality) = node.input_metadata().first().unwrap().cardinality {
                    let overhead = cpu_usage / cardinality as f64;

                    println!("New send overhead: {}", overhead);
                    if overhead > *max_send_overhead {
                        *max_send_overhead = overhead;
                    }
                }
            }
        }
        _ => {}
    }
    match network_type {
        Some(NetworkType::Recv) | Some(NetworkType::SendRecv) => {
            if let Some(cardinality) = metadata.cardinality
                && let Some(cpu_usage) = metadata.op.network_recv_cpu_usage
            {
                let overhead = cpu_usage / cardinality as f64;

                println!("New receive overhead: {}", overhead);
                if overhead > *max_recv_overhead {
                    *max_recv_overhead = overhead;
                }
            }
        }
        _ => {}
    }
}

// Track the max of each so we decouple conservatively
pub fn analyze_send_recv_overheads(ir: &mut [HydroRoot], location: &LocationId) -> (f64, f64) {
    let mut max_send_overhead = 0.0;
    let mut max_recv_overhead = 0.0;

    traverse_dfir(
        ir,
        |_, _| {},
        |node, next_stmt_id| {
            analyze_overheads_node(
                node,
                next_stmt_id,
                &mut max_send_overhead,
                &mut max_recv_overhead,
                location,
            );
        },
    );

    if max_send_overhead == 0.0 {
        println!("Warning: No send overhead found.");
    }
    if max_recv_overhead == 0.0 {
        println!("Warning: No receive overhead found.");
    }

    (max_send_overhead, max_recv_overhead)
}
