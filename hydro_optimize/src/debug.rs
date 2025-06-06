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
