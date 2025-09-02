use std::collections::HashMap;

use hydro_lang::ir::{HydroNode, HydroRoot, traverse_dfir};

fn print_id_root(root: &mut HydroRoot, next_stmt_id: &mut usize) {
    let inputs = root
        .input_metadata()
        .iter()
        .map(|m| m.op.id)
        .collect::<Vec<Option<usize>>>();
    println!(
        "{} Root {}, Inputs: {:?}",
        next_stmt_id,
        root.print_root(),
        inputs,
    );
}

fn print_id_node(node: &mut HydroNode, next_stmt_id: &mut usize) {
    let metadata = node.metadata();
    let inputs = node
        .input_metadata()
        .iter()
        .map(|m| m.op.id)
        .collect::<Vec<Option<usize>>>();
    println!(
        "{} Node {}, {:?}, Inputs: {:?}",
        next_stmt_id,
        node.print_root(),
        metadata,
        inputs,
    );
}

pub fn print_id(ir: &mut [HydroRoot]) {
    traverse_dfir(ir, print_id_root, print_id_node);
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
pub fn name_to_id_map(ir: &mut [HydroRoot]) -> HashMap<String, usize> {
    let mut mapping = HashMap::new();
    traverse_dfir(
        ir,
        |_, _| {},
        |node, next_stmt_id| name_to_id_node(node, next_stmt_id, &mut mapping),
    );
    mapping
}
