use std::{cell::RefCell, collections::HashMap};

use regex::Regex;

use crate::{ir::*, location::LocationId};
pub use crate::runtime_support::resource_measurement::{COUNTER_PREFIX, CPU_USAGE_PREFIX};

fn inject_id_leaf(leaf: &mut HydroLeaf, next_stmt_id: &mut usize) {
    let metadata = leaf.metadata_mut();
    metadata.id = Some(*next_stmt_id);
}

fn inject_id_node(node: &mut HydroNode, next_stmt_id: &mut usize) {
    let metadata = node.metadata_mut();
    metadata.id = Some(*next_stmt_id);
}

pub fn inject_id(ir: &mut [HydroLeaf]) {
    traverse_dfir(ir, inject_id_leaf, inject_id_node);
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

pub fn inject_count(ir: &mut [HydroLeaf], op_to_count: &HashMap<usize, usize>) {
    traverse_dfir(
        ir,
        |_, _| {},
        |node, next_stmt_id| {
            inject_count_node(node, next_stmt_id, op_to_count);
        },
    );
}

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
            let is_network_recv = cap.get(3).is_some_and(|direction| direction.as_str() == "recv");

            let dfir_operator_and_samples =
                samples_per_id.entry((id, is_network_recv)).or_insert(0.0);
            *dfir_operator_and_samples += n_samples;
        }
        else {
            unidentified_samples += n_samples;
        }
        total_samples += n_samples;
    }

    println!("Out of {} samples, {} were unidentified, {}%", total_samples, unidentified_samples, unidentified_samples / total_samples * 100.0);

    samples_per_id.iter_mut().for_each(|(_, samples)| {
        *samples /= total_samples
    });
    samples_per_id
}

fn inject_perf_leaf(
    leaf: &mut HydroLeaf,
    id_to_usage: &HashMap<(usize, bool), f64>,
    next_stmt_id: &mut usize,
) {
    if let Some(cpu_usage) = id_to_usage.get(&(*next_stmt_id, false)) {
        leaf.metadata_mut().cpu_usage = Some(*cpu_usage);
    }
}

fn inject_perf_node(
    node: &mut HydroNode,
    id_to_usage: &HashMap<(usize, bool), f64>,
    next_stmt_id: &mut usize,
) {
    if let Some(cpu_usage) = id_to_usage.get(&(*next_stmt_id, false)) {
        node.metadata_mut().cpu_usage = Some(*cpu_usage);
    }
    // If this is a Network node, separately get receiver CPU usage
    if let HydroNode::Network { metadata, .. } = node {
        if let Some(cpu_usage) = id_to_usage.get(&(*next_stmt_id, true)) {
            metadata.network_recv_cpu_usage = Some(*cpu_usage);
        }
    }
}

pub fn inject_perf(ir: &mut [HydroLeaf], folded_data: Vec<u8>) {
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


fn inject_location_leaf(leaf: &mut HydroLeaf, id_to_location: &RefCell<HashMap<usize, LocationId>>, missing_location: &RefCell<bool>) {
    let inputs = leaf.input_metadata();
    let input_metadata = inputs.first().unwrap();
    let input_id = input_metadata.id.unwrap();

    if let Some(location) = id_to_location.borrow().get(&input_metadata.id.unwrap()) {
        let metadata = leaf.metadata_mut();
        metadata.location_kind.swap_root(location.clone());

        if let HydroLeaf::CycleSink { location_kind, .. } = leaf {
            *location_kind = location.clone();
            println!("Cycle sink with input {} has location {:?}", input_id, location.clone());
        }
    }
    else {
        println!("Missing location for leaf: {:?}", leaf.print_root());
        *missing_location.borrow_mut() = true;
    }
}

fn inject_location_node(node: &mut HydroNode, id_to_location: &RefCell<HashMap<usize, LocationId>>, missing_location: &RefCell<bool>, cycle_source_to_sink_input: &HashMap<usize, usize>) {
    if let Some(op_id) = node.metadata().id {
        let inputs = match node {
            HydroNode::Source { location_kind, .. }
            | HydroNode::Network { to_location: location_kind, .. } => {
                // Get location sources from the nodes must have it be correct: Source and Network
                id_to_location.borrow_mut().insert(op_id, location_kind.clone());
                return;
            }
            HydroNode::Tee { inner, .. } => {
                vec![inner.0.borrow().metadata().id.unwrap()]
            }
            HydroNode::CycleSource { .. } => {
                vec![*cycle_source_to_sink_input.get(&op_id).unwrap()]
            }
            _ => {
                node.input_metadata().iter().map(|input_metadata| input_metadata.id.unwrap()).collect()
            }
        };

        // Otherwise, get it from (either) input
        let metadata = node.metadata_mut();
        for input in inputs {
            let location = id_to_location.borrow().get(&input).cloned();
            if let Some(location) = location {
                metadata.location_kind.swap_root(location.clone());
                id_to_location.borrow_mut().insert(op_id, location.clone());

                match node {
                    // Update Persist's location as well (we won't see it during traversal)
                    HydroNode::Fold { input, .. } | HydroNode::FoldKeyed { input, .. } | HydroNode::Reduce { input, .. } | HydroNode::ReduceKeyed { input, ..} => {
                        if let HydroNode::Persist { metadata: persist_metadata, .. } = input.as_mut() {
                            persist_metadata.location_kind.swap_root(location);
                        }
                    }
                    // CycleSource also stores the location outside of its metadata, so update it as well
                    HydroNode::CycleSource { location_kind, .. } => {
                        location_kind.swap_root(location);
                    }
                    _ => {}
                }
                return;
            }
        }

        // If the location was not set, let the recursive function know
        println!("Missing location for node: {:?}", node.print_root());
        *missing_location.borrow_mut() = true;
    }
}

pub fn inject_location(ir: &mut [HydroLeaf], cycle_source_to_sink_input: &HashMap<usize, usize>) {
    let id_to_location = RefCell::new(HashMap::new());

    loop {
        println!("Attempting to inject location, looping until fixpoint...");
        let missing_location = RefCell::new(false);

        transform_bottom_up(ir, &mut |leaf| {
            inject_location_leaf(leaf, &id_to_location, &missing_location);
        }, &mut |node| {
            inject_location_node(node, &id_to_location, &missing_location, cycle_source_to_sink_input);
        });

        if !missing_location.borrow().clone() {
            println!("Locations injected!");
            break;
        }
    }
}