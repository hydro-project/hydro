use crate::ir::*;

fn print_id_leaf(leaf: &mut HydroLeaf, next_stmt_id: &mut usize) {
    let metadata = leaf.metadata();
    println!(
        "{} Leaf {}, Cardinality {:?}, Usage {:?}",
        next_stmt_id,
        leaf.print_root(),
        metadata.cardinality,
        metadata.cpu_usage
    );
}

fn print_id_node(node: &mut HydroNode, next_stmt_id: &mut usize) {
    let metadata = node.metadata();
    println!(
        "{} Node {}, Cardinality {:?}, Usage {:?}",
        next_stmt_id,
        node.print_root(),
        metadata.cardinality,
        metadata.cpu_usage
    );
}

pub fn print_id(ir: &mut [HydroLeaf]) {
    traverse_dfir(ir, print_id_leaf, print_id_node);
}
