use std::collections::HashMap;

use hydro_lang::ir::{HydroLeaf, HydroNode, traverse_dfir};

fn print_id_leaf(leaf: &mut HydroLeaf, next_stmt_id: &mut usize) {
    let metadata = leaf.metadata();
    let inputs = leaf
        .input_metadata()
        .iter()
        .map(|m| m.id)
        .collect::<Vec<Option<usize>>>();
    println!(
        "{} Leaf {}, {:?}, Inputs: {:?}",
        next_stmt_id,
        leaf.print_root(),
        metadata,
        inputs,
    );
}

fn print_id_node(node: &mut HydroNode, next_stmt_id: &mut usize) {
    let metadata = node.metadata();
    let inputs = node
        .input_metadata()
        .iter()
        .map(|m| m.id)
        .collect::<Vec<Option<usize>>>();
    println!(
        "{} Node {}, {:?}, Inputs: {:?}",
        next_stmt_id,
        node.print_root(),
        metadata,
        inputs,
    );
}

pub fn print_id(ir: &mut [HydroLeaf]) {
    traverse_dfir(ir, print_id_leaf, print_id_node);
}

fn name_to_id_node(
    node: &mut HydroNode,
    next_stmt_id: &mut usize,
    mapping: &mut HashMap<String, usize>,
) {
    if let Some(name) = &node.metadata().tag {
        mapping.insert(name.clone(), *next_stmt_id);
    }
}

// Create a mapping from named IR nodes (from `ir_node_named`) to their IDs
pub fn name_to_id_map(ir: &mut [HydroLeaf]) -> HashMap<String, usize> {
    let mut mapping = HashMap::new();
    traverse_dfir(
        ir,
        |_, _| {},
        |node, next_stmt_id| name_to_id_node(node, next_stmt_id, &mut mapping),
    );
    mapping
}
