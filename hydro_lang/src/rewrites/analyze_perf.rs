use regex::Regex;
use std::collections::HashMap;

use crate::ir::*;

pub const CPU_USAGE_PREFIX: &str = "CPU:";

/// Returns a map from operator ID to a map of (DFIR operator name, percentage of total samples) pairs.
/// The DFIR operator name is returned because a single Hydro operator can map to multiple DFIR operators
fn parse_perf(file: String) -> HashMap<usize, HashMap<String, f64>> {
    let mut total_samples = 0f64;
    let mut samples_per_operator = HashMap::new();
    let operator_regex = Regex::new(r"::op_\d+v\d+__(.*?)__(\d+)::").unwrap();

    for line in file.lines() {
        let n_samples_index = line.rfind(' ').unwrap() + 1;
        let n_samples = &line[n_samples_index..].parse::<f64>().unwrap();

        if let Some(cap) = operator_regex.captures_iter(line).last() {
            let operator_name = &cap[1];
            let id = cap[2].parse::<usize>().unwrap();
            let dfir_operator_and_samples = samples_per_operator.entry(id).or_insert(HashMap::new());
            let prev_samples = dfir_operator_and_samples.entry(operator_name.to_string()).or_insert(0f64);
            *prev_samples += n_samples;
        }

        total_samples += n_samples;
    }

    samples_per_operator
        .iter_mut()
        .for_each(|(_, v)| v.iter_mut().for_each(|(_, samples)| *samples /= total_samples));
    samples_per_operator
}

fn analyze_perf_leaf(
    leaf: &mut HydroLeaf,
    id_to_usage: &HashMap<usize, HashMap<String, f64>>,
    next_stmt_id: usize,
) {
    if let Some(dfir_operator_and_samples) = id_to_usage.get(&next_stmt_id) {
        for (dfir_operator, samples) in dfir_operator_and_samples {
            println!("{} Hydro leaf {}: {} {:.02}%", next_stmt_id, leaf.print_root(), dfir_operator, samples * 100f64);
        }
    }
}

fn analyze_perf_node(
    node: &mut HydroNode,
    id_to_usage: &HashMap<usize, HashMap<String, f64>>,
    next_stmt_id: usize,
) {
    if let Some(dfir_operator_and_samples) = id_to_usage.get(&next_stmt_id) {
        for (dfir_operator, samples) in dfir_operator_and_samples {
            println!("{} Hydro node {}: {} {:.02}%", next_stmt_id, node.print_root(), dfir_operator, samples * 100f64);
        }
    }
}

// #[cfg(feature = "build")]
// #[stageleft::runtime]
// pub fn analyze_perf(ir: &mut Vec<HydroLeaf>) {
//     let id_to_usage = parse_perf(std::fs::read_to_string("proposer0.data.folded").unwrap());
//     traverse_dfir(ir, |leaf, next_stmt_id| {
//         analyze_perf_leaf(leaf, &id_to_usage, next_stmt_id);
//     }, |node, next_stmt_id| {
//         analyze_perf_node(node, &id_to_usage, next_stmt_id);
//     });
// }

#[cfg(feature = "build")]
#[stageleft::runtime]
pub fn analyze_perf(ir: &mut Vec<HydroLeaf>, folded_data: Vec<u8>) {
    let id_to_usage = parse_perf(String::from_utf8(folded_data).unwrap());
    traverse_dfir(ir, |leaf, next_stmt_id| {
        analyze_perf_leaf(leaf, &id_to_usage, next_stmt_id);
    }, |node, next_stmt_id| {
        analyze_perf_node(node, &id_to_usage, next_stmt_id);
    });
}