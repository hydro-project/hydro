use std::time::Duration;

use stageleft::Quoted;

use crate::ir::*;

fn insert_counter_node(node: &mut HydroNode, next_stmt_id: usize, duration: syn::Expr) {
    match node {
        HydroNode::Placeholder
        | HydroNode::Unpersist { .. }
        | HydroNode::Counter { .. } => {
            std::panic!("Unexpected {:?} found in insert_counter_node", node.print_root());
        }
        HydroNode::Source { metadata, .. }
        | HydroNode::CycleSource { metadata, .. }
        | HydroNode::Persist { metadata, .. }
        | HydroNode::Delta { metadata, .. }
        | HydroNode::Chain { metadata, .. } // Can technically be derived by summing parent cardinalities
        | HydroNode::CrossProduct { metadata, .. } // Can technically be derived by multiplying parent cardinalities
        | HydroNode::Join { metadata, .. }
        | HydroNode::Difference { metadata, .. }
        | HydroNode::AntiJoin { metadata, .. }
        | HydroNode::FlatMap { metadata, .. }
        | HydroNode::Filter { metadata, .. }
        | HydroNode::FilterMap { metadata, .. }
        | HydroNode::Unique { metadata, .. }
        | HydroNode::FoldKeyed { metadata, .. }
        | HydroNode::ReduceKeyed { metadata, .. }
        | HydroNode::Network { metadata, .. }
         => {
            let metadata = metadata.clone();
            let node_content = std::mem::replace(node, HydroNode::Placeholder);

            let counter = HydroNode::Counter {
                tag: next_stmt_id.to_string(),
                duration: duration.into(),
                input: Box::new(node_content),
                metadata: metadata.clone(),
            };

            *node = counter;
        }
        HydroNode::Tee { .. } // Do nothing, we will count the parent of the Tee
        | HydroNode::CrossSingleton { .. } // Singletons only ever include 1 value
        | HydroNode::Map { .. } // Equal to parent cardinality
        | HydroNode::DeferTick { .. } // Equal to parent cardinality
        | HydroNode::Enumerate { .. }
        | HydroNode::Inspect { .. }
        | HydroNode::Sort { .. }
        | HydroNode::Fold { .. } // Output 1 value
        | HydroNode::Reduce { .. } // Output 1 value
         => {}
    }
}

pub fn insert_counter(ir: &mut [HydroLeaf], duration: impl Quoted<'static, Duration>) {
    let duration = duration.splice_typed();
    traverse_dfir(
        ir,
        |_, _| {},
        |node, next_stmt_id| {
            insert_counter_node(node, next_stmt_id, duration.clone());
        },
    );
}
