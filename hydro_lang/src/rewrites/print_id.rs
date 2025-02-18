#[cfg(feature = "build")]
use crate::ir::*;

#[cfg(feature = "build")]
fn print_id_leaf(leaf: &mut HydroLeaf, next_stmt_id: usize) {
    println!("{} Hydro leaf {}", next_stmt_id, leaf.print_root(),);
}

#[cfg(feature = "build")]
fn print_id_node(node: &mut HydroNode, next_stmt_id: usize) {
    println!("{} Hydro node {}", next_stmt_id, node.print_root(),);
}

#[cfg(feature = "build")]
pub fn print_id(ir: &mut [HydroLeaf]) {
    traverse_dfir(ir, print_id_leaf, print_id_node);
}
