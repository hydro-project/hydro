#[cfg(feature = "build")]
use proc_macro2::Span;

#[cfg(feature = "build")]
use crate::ir::*;
#[cfg(feature = "build")]
use crate::location::LocationId;
#[cfg(feature = "build")]
use crate::stream::{deserialize_bincode_with_type, serialize_bincode_with_type};

#[cfg(feature = "build")]
pub struct Partitioner {
    pub network_nodes_to_partition: Vec<usize>,
    pub num_partitions: usize,
    // TODO: partitioning function, specify a "field" of the original output
}

#[cfg(feature = "build")]
fn partition_node(node: &mut HydroNode, partitioner: &Partitioner, next_stmt_id: usize) {
    let metadata = node.metadata().clone();
    if partitioner.network_nodes_to_partition.contains(&next_stmt_id) {
        println!("Partitioning node {} {}", next_stmt_id, node.print_root());

        let output_debug_type = metadata.output_type.clone().unwrap();

    }
}

#[cfg(feature = "build")]
pub fn partition(ir: &mut [HydroLeaf], partitioner: &Partitioner) {
    traverse_dfir(
        ir,
        |_, _| {},
        |node, next_stmt_id| {
            partition_node(node, partitioner, next_stmt_id);
        },
    );
}
