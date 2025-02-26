use crate::ir::*;

fn remove_counter_node(node: &mut HydroNode, _next_stmt_id: &mut usize) {
    if let HydroNode::Counter { input, .. } = node {
        *node = std::mem::replace(input, HydroNode::Placeholder);
    }
}

pub fn remove_counter(ir: &mut [HydroLeaf]) {
    traverse_dfir(ir, |_, _| {}, remove_counter_node);
}
