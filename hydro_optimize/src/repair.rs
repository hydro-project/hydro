use std::cell::RefCell;
use std::collections::HashMap;

use hydro_lang::compile::ir::{HydroNode, HydroRoot, transform_bottom_up, traverse_dfir};
use hydro_lang::location::dynamic::LocationId;
use syn::Ident;

fn inject_id_root(root: &mut HydroRoot, next_stmt_id: &mut usize) {
    let metadata = root.op_metadata_mut();
    metadata.id = Some(*next_stmt_id);
}

fn inject_id_node(node: &mut HydroNode, next_stmt_id: &mut usize) {
    let metadata = node.op_metadata_mut();
    metadata.id = Some(*next_stmt_id);
}

pub fn inject_id(ir: &mut [HydroRoot]) {
    traverse_dfir(ir, inject_id_root, inject_id_node);
}

fn link_cycles_root(root: &mut HydroRoot, sink_inputs: &mut HashMap<Ident, usize>) {
    if let HydroRoot::CycleSink { ident, input, .. } = root {
        sink_inputs.insert(ident.clone(), input.op_metadata().id.unwrap());
    }
}

fn link_cycles_node(node: &mut HydroNode, sources: &mut HashMap<Ident, usize>) {
    if let HydroNode::CycleSource {
        ident, metadata, ..
    } = node
    {
        sources.insert(ident.clone(), metadata.op.id.unwrap());
    }
}

// Returns map from CycleSource id to the input IDs of the corresponding CycleSink's input
// Assumes that metadtata.id is set for all nodes
pub fn cycle_source_to_sink_input(ir: &mut [HydroRoot]) -> HashMap<usize, usize> {
    let mut sources = HashMap::new();
    let mut sink_inputs = HashMap::new();

    // Can't use traverse_dfir since that skips CycleSink
    transform_bottom_up(
        ir,
        &mut |leaf| {
            link_cycles_root(leaf, &mut sink_inputs);
        },
        &mut |node| {
            link_cycles_node(node, &mut sources);
        },
        false,
    );

    let mut source_to_sink_input = HashMap::new();
    for (sink_ident, sink_input_id) in sink_inputs {
        if let Some(source_id) = sources.get(&sink_ident) {
            source_to_sink_input.insert(*source_id, sink_input_id);
        } else {
            std::panic!(
                "No source found for CycleSink {}, Input Id {}",
                sink_ident,
                sink_input_id
            );
        }
    }
    println!("Source to sink input: {:?}", source_to_sink_input);
    source_to_sink_input
}

fn inject_location_root(
    root: &mut HydroRoot,
    id_to_location: &RefCell<HashMap<usize, LocationId>>,
    missing_location: &RefCell<bool>,
) {
    let inputs = root.input_metadata();
    let input_metadata = inputs.first().unwrap();

    if let Some(location) = id_to_location.borrow().get(&input_metadata.op.id.unwrap()) {
        if let HydroRoot::CycleSink { out_location, .. } = root {
            out_location.swap_root(location.root().clone());
        }
    } else {
        println!("Missing location for root: {:?}", root.print_root());
        *missing_location.borrow_mut() = true;
    }
}

fn inject_location_input_persist(input: &mut Box<HydroNode>, new_location: LocationId) {
    if let HydroNode::Persist {
        metadata: persist_metadata,
        ..
    } = input.as_mut()
    {
        persist_metadata.location_kind.swap_root(new_location);
    }
}

fn inject_location_node(
    node: &mut HydroNode,
    id_to_location: &RefCell<HashMap<usize, LocationId>>,
    missing_location: &RefCell<bool>,
    cycle_source_to_sink_input: &HashMap<usize, usize>,
) {
    if let Some(op_id) = node.op_metadata().id {
        let inputs = match node {
            HydroNode::Source { metadata, .. }
            | HydroNode::ExternalInput { metadata, .. }
            | HydroNode::Network { metadata, .. } => {
                // Get location sources from the nodes must have it be correct: Source and Network
                id_to_location
                    .borrow_mut()
                    .insert(op_id, metadata.location_kind.clone());
                return;
            }
            HydroNode::Tee { inner, .. } => {
                vec![inner.0.borrow().op_metadata().id.unwrap()]
            }
            HydroNode::CycleSource { .. } => {
                vec![*cycle_source_to_sink_input.get(&op_id).unwrap()]
            }
            _ => node
                .input_metadata()
                .iter()
                .map(|input_metadata| input_metadata.op.id.unwrap())
                .collect(),
        };

        // Otherwise, get it from (either) input
        let metadata = node.metadata_mut();
        for input in inputs {
            let location = id_to_location.borrow().get(&input).cloned();
            if let Some(location) = location {
                metadata.location_kind.swap_root(location.root().clone());
                id_to_location
                    .borrow_mut()
                    .insert(op_id, metadata.location_kind.clone());

                match node {
                    // Update Persist's location as well (we won't see it during traversal)
                    HydroNode::CrossProduct { left, right, .. }
                    | HydroNode::Join { left, right, .. } => {
                        inject_location_input_persist(left, location.root().clone());
                        inject_location_input_persist(right, location.root().clone());
                    }
                    HydroNode::Difference { pos, neg, .. }
                    | HydroNode::AntiJoin { pos, neg, .. } => {
                        inject_location_input_persist(pos, location.root().clone());
                        inject_location_input_persist(neg, location.root().clone());
                    }
                    HydroNode::Fold { input, .. }
                    | HydroNode::FoldKeyed { input, .. }
                    | HydroNode::Reduce { input, .. }
                    | HydroNode::ReduceKeyed { input, .. }
                    | HydroNode::Scan { input, .. } => {
                        inject_location_input_persist(input, location.root().clone());
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

pub fn inject_location(ir: &mut [HydroRoot], cycle_source_to_sink_input: &HashMap<usize, usize>) {
    let id_to_location = RefCell::new(HashMap::new());

    loop {
        println!("Attempting to inject location, looping until fixpoint...");
        let missing_location = RefCell::new(false);

        transform_bottom_up(
            ir,
            &mut |leaf| {
                inject_location_root(leaf, &id_to_location, &missing_location);
            },
            &mut |node| {
                inject_location_node(
                    node,
                    &id_to_location,
                    &missing_location,
                    cycle_source_to_sink_input,
                );
            },
            false,
        );

        if !*missing_location.borrow() {
            println!("Locations injected!");

            // Check well-formedness here
            transform_bottom_up(ir, &mut |_| {}, &mut |_| {}, true);
            break;
        }
    }
}

fn remove_counter_node(node: &mut HydroNode, _next_stmt_id: &mut usize) {
    if let HydroNode::Counter { input, .. } = node {
        *node = std::mem::replace(input, HydroNode::Placeholder);
    }
}

pub fn remove_counter(ir: &mut [HydroRoot]) {
    traverse_dfir(ir, |_, _| {}, remove_counter_node);
}
