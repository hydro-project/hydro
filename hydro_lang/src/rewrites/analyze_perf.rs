use regex::Regex;
use std::collections::HashMap;

use crate::ir::*;

/// Returns a map from operator ID to a map of (DFIR operator name, percentage of total samples) pairs.
/// The DFIR operator name is returned because a single Hydro operator can map to multiple DFIR operators
fn parse_perf(file_path: &str) -> HashMap<usize, HashMap<String, f64>> {
    let file = std::fs::read_to_string(file_path).unwrap();

    let mut total_samples = 0f64;
    let mut samples_per_operator = HashMap::new();
    let operator_regex = Regex::new(r"::op_\d+v\d+__(.*?)__(\d+)::").unwrap();

    for line in file.lines() {
        let n_samples_index = line.rfind(' ').unwrap() + 1;
        let n_samples = &line[n_samples_index..].parse::<f64>().unwrap();

        for cap in operator_regex.captures_iter(line) {
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

// fn combine_perf_outputs(folded_filepaths: Vec<&str>) {
//     let mut id_to_usage = HashMap::new();
//     for filepath in folded_filepaths {
//         let mut id_to_usage_for_file = parse_perf(filepath);

//     }
// }

fn analyze_perf_node(
    node: &mut HydroNode,
    id_to_usage: &mut HashMap<usize, HashMap<String, f64>>,
    next_stmt_id: &mut usize,
) {
    let my_id = next_stmt_id;
    if let Some(dfir_operator_and_samples) = id_to_usage.get(my_id) {
        for (dfir_operator, samples) in dfir_operator_and_samples {
            println!("Hydro node {:?}: {} {:.02}%", node, dfir_operator, samples * 100f64);
        }
    }
    else {
        println!("No samples for operator with ID: {}. Not necessarily an error, could be because it barely executed.", my_id);
    }
}

pub fn analyze_perf(ir: Vec<HydroLeaf>) -> Vec<HydroLeaf> {
    let mut seen_tees = Default::default();
    let mut id_to_usage = parse_perf("../leader.data.folded");
    let mut next_stmt_id = 0;
    ir.into_iter()
        .map(|l| {
            l.transform_children(
                |n, s, c| n.transform_bottom_up(analyze_perf_node, s, &mut id_to_usage, c),
                &mut seen_tees,
                &mut next_stmt_id,
            )
        })
        .collect()
}