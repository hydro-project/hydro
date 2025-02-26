use std::collections::HashMap;

use syn::Ident;

use crate::ir::*;

fn link_cycles_leaf(leaf: &mut HydroLeaf, next_stmt_id: &mut usize, sinks: &mut HashMap<Ident, usize>) {
    match leaf {
        HydroLeaf::CycleSink { ident, .. } => {
            sinks.insert(ident.clone(), *next_stmt_id);
        }
        _ => {}
    }
}

fn link_cycles_node(node: &mut HydroNode, next_stmt_id: &mut usize, sources: &mut HashMap<Ident, usize>) {
    match node {
        HydroNode::CycleSource { ident, .. } => {
            sources.insert(ident.clone(), *next_stmt_id);
        }
        _ => {}
    }
}

// Returns map from CycleSink id to CycleSource id
pub fn link_cycles(ir: &mut [HydroLeaf]) -> HashMap<usize, usize> {
    let mut sources = HashMap::new();
    let mut sinks = HashMap::new();

    traverse_dfir(ir, |leaf, next_stmt_id| {
        link_cycles_leaf(leaf, next_stmt_id, &mut sinks);
    }, |node, next_stmt_id| {
        link_cycles_node(node, next_stmt_id, &mut sources);
    });

    let mut sink_to_source = HashMap::new();
    for (sink_ident, sink_id) in sinks {
        if let Some(source_id) = sources.get(&sink_ident) {
            sink_to_source.insert(sink_id, *source_id);
        } else {
            std::panic!("No source found for CycleSink {}, Id {}", sink_ident, sink_id);
        }
    }
    sink_to_source
}
