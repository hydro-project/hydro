use std::collections::HashMap;

use syn::Ident;

use crate::ir::*;

fn link_cycles_leaf(
    leaf: &mut HydroLeaf,
    sink_inputs: &mut HashMap<Ident, usize>,
) {
    if let HydroLeaf::CycleSink { ident, input, .. } = leaf {
        sink_inputs.insert(ident.clone(), input.metadata().id.unwrap());
    }
}

fn link_cycles_node(
    node: &mut HydroNode,
    sources: &mut HashMap<Ident, usize>,
) {
    if let HydroNode::CycleSource { ident, metadata, .. } = node {
        sources.insert(ident.clone(), metadata.id.unwrap());
    }
}

// Returns map from CycleSource id to the input IDs of the corresponding CycleSink's input
// Assumes that metadtata.id is set for all nodes
pub fn cycle_source_to_sink_input(ir: &mut [HydroLeaf]) -> HashMap<usize, usize> {
    let mut sources = HashMap::new();
    let mut sink_inputs = HashMap::new();

    // Can't use traverse_dfir since that skips CycleSink
    transform_bottom_up(
        ir,
        &mut |leaf| {
            link_cycles_leaf(leaf, &mut sink_inputs);
        },
        &mut |node| {
            link_cycles_node(node, &mut sources);
        },
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
