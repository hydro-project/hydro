use crate::ir::*;

fn print_id_leaf(leaf: &mut HydroLeaf, next_stmt_id: &mut usize) {
    let metadata = leaf.metadata();
    println!(
        "{} Leaf {}, {:?}",
        next_stmt_id,
        leaf.print_root(),
        metadata,
    );
}

fn print_id_node(node: &mut HydroNode, next_stmt_id: &mut usize) {
    let metadata = node.metadata();
    println!(
        "{} Node {}, {:?}",
        next_stmt_id,
        node.print_root(),
        metadata,
    );
}

pub fn print_id(ir: &mut [HydroLeaf]) {
    traverse_dfir(ir, print_id_leaf, print_id_node);
}
