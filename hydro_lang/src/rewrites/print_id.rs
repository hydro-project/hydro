use crate::ir::*;

fn print_id_leaf(leaf: &mut HydroLeaf, next_stmt_id: usize) {
    println!("{} Hydro leaf {}", next_stmt_id, leaf.print_root(),);
}

fn print_id_node(node: &mut HydroNode, next_stmt_id: usize) {
    println!("{} Hydro node {}", next_stmt_id, node.print_root(),);
}

pub fn print_id(ir: &mut [HydroLeaf]) {
    traverse_dfir(ir, print_id_leaf, print_id_node);
}
