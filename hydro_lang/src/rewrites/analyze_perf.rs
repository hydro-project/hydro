use std::collections::HashMap;

use regex::Regex;

pub use crate::internal_constants::CPU_USAGE_PREFIX;
use crate::ir::*;

pub fn parse_cpu_usage(measurement: String) -> f64 {
    let regex = Regex::new(r"Total (\d+\.\d+)%").unwrap();
    regex
        .captures_iter(&measurement)
        .last()
        .map(|cap| cap[1].parse::<f64>().unwrap())
        .unwrap_or(0f64)
}

/// Returns a map from operator ID to a map of (DFIR operator name, percentage of total samples) pairs.
/// The DFIR operator name is returned because a single Hydro operator can map to multiple DFIR operators
fn parse_perf(file: String) -> HashMap<usize, HashMap<String, f64>> {
    let mut total_samples = 0f64;
    let mut samples_per_operator = HashMap::new();
    let operator_regex = Regex::new(r"::op_\d+v\d+__(.*?)__(\d+)::").unwrap();
    let sink_feed_regex = Regex::new(r"sink_feed_flush_(\d+)").unwrap();

    for line in file.lines() {
        let n_samples_index = line.rfind(' ').unwrap() + 1;
        let n_samples = &line[n_samples_index..].parse::<f64>().unwrap();

        let mut new_samples = vec![];
        if let Some(cap) = operator_regex.captures_iter(line).last() {
            let operator_name = &cap[1];
            let id = cap[2].parse::<usize>().unwrap();
            new_samples.push((id, operator_name.to_string()));
        }
        // Note: Although we do a regex check twice per line (potentially adding samples twice), there will never be an operator and sink_feed in the same line, so it's ok
        if let Some(cap) = sink_feed_regex.captures_iter(line).last() {
            let id = cap[1].parse::<usize>().unwrap();
            new_samples.push((id, "sink_feed_flush".to_string()));
        }

        for (id, operator_name) in new_samples {
            let dfir_operator_and_samples =
                samples_per_operator.entry(id).or_insert(HashMap::new());
            let prev_samples = dfir_operator_and_samples
                .entry(operator_name)
                .or_insert(0f64);
            *prev_samples += n_samples;
        }

        total_samples += n_samples;
    }

    samples_per_operator.iter_mut().for_each(|(_, v)| {
        v.iter_mut()
            .for_each(|(_, samples)| *samples /= total_samples)
    });
    samples_per_operator
}

fn inject_perf_leaf(
    leaf: &mut HydroLeaf,
    id_to_usage: &HashMap<usize, HashMap<String, f64>>,
    next_stmt_id: &mut usize,
) {
    if let Some(dfir_operator_and_samples) = id_to_usage.get(next_stmt_id) {
        leaf.metadata_mut().cpu_usage = Some(dfir_operator_and_samples.values().sum());
    }
}

fn inject_perf_node(
    node: &mut HydroNode,
    id_to_usage: &HashMap<usize, HashMap<String, f64>>,
    next_stmt_id: &mut usize,
) {
    if let Some(dfir_operator_and_samples) = id_to_usage.get(next_stmt_id) {
        node.metadata_mut().cpu_usage = Some(dfir_operator_and_samples.values().sum());
    }
}

pub fn analyze_perf(ir: &mut [HydroLeaf], folded_data: Vec<u8>) {
    let id_to_usage = parse_perf(String::from_utf8(folded_data).unwrap());
    traverse_dfir(
        ir,
        |leaf, next_stmt_id| {
            inject_perf_leaf(leaf, &id_to_usage, next_stmt_id);
        },
        |node, next_stmt_id| {
            inject_perf_node(node, &id_to_usage, next_stmt_id);
        },
    );
}
