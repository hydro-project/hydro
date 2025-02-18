#[cfg(feature = "build")]
use std::collections::HashMap;

#[cfg(feature = "build")]
use crate::ir::*;

/// Fields that could be used for partitioning
#[cfg(feature = "build")]
pub enum PartitionAttribute {
    All(),
    TupleIndex(usize),
}

#[cfg(feature = "build")]
pub struct Partitioner {
    pub nodes_to_partition: HashMap<usize, PartitionAttribute>, /* ID of node right before a Network -> what to partition on */
    pub num_original_nodes: usize, // Number of nodes at destination process before partitioning
    pub num_partitions: usize,
}

// Placeholder ClusterId type
pub struct Partitioned {}

#[cfg(feature = "build")]
fn partition_node(node: &mut HydroNode, partitioner: &Partitioner, next_stmt_id: usize) {
    let Partitioner {
        nodes_to_partition,
        num_original_nodes,
        num_partitions,
        ..
    } = partitioner;

    if let Some(partition_attr) = nodes_to_partition.get(&next_stmt_id) {
        println!("Partitioning node {} {}", next_stmt_id, node.print_root());

        let node_content = std::mem::replace(node, HydroNode::Placeholder);
        let metadata = node_content.metadata().clone();

        let f: syn::Expr = match partition_attr {
            PartitionAttribute::All() => {
                syn::parse_quote!(|(orig_dest, item)| {
                    let orig_dest_id = orig_dest.raw_id;
                    let new_dest_id = orig_dest_id + (#num_original_nodes * (item as usize % #num_partitions)) as u32;
                    (
                        ClusterId::<hydro_lang::rewrites::partitioner::Partitioned>::from_raw(new_dest_id),
                        item.clone()
                    )
                })
            }
            PartitionAttribute::TupleIndex(tuple_index) => {
                let tuple_index_ident = syn::Index::from(*tuple_index);
                syn::parse_quote!(|(orig_dest, tuple)| {
                    let orig_dest_id = orig_dest.raw_id;
                    let new_dest_id = orig_dest_id + (#num_original_nodes * (tuple.#tuple_index_ident as usize % #num_partitions)) as u32;
                    (
                        ClusterId::<hydro_lang::rewrites::partitioner::Partitioned>::from_raw(new_dest_id),
                        tuple.clone()
                    )
                })
            }
        };

        let mapped_node = HydroNode::Map {
            f: f.into(),
            input: Box::new(node_content),
            metadata,
        };

        *node = mapped_node;
    }
}

/// Limitations: Can only partition sends to clusters (not processes). Can only partition sends to 1 cluster at a time. Assumes that the partitioned attribute can be casted to usize.
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
