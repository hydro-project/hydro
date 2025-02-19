#[cfg(feature = "build")]
use std::collections::HashMap;

#[cfg(feature = "build")]
use syn::visit_mut::{self, VisitMut};

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
    pub num_partitions: usize,
}

// Placeholder ClusterId type
pub struct Partitioned {}

/// Don't expose partition members to the cluster
#[cfg(feature = "build")]
pub struct ClusterMembersReplace {
    pub num_partitions: usize,
}

#[cfg(feature = "build")]
impl VisitMut for ClusterMembersReplace {
    fn visit_expr_mut(&mut self, expr: &mut syn::Expr) {
        if let syn::Expr::Unsafe(unsafe_expr) = expr {
            for stmt in &mut unsafe_expr.block.stmts {
                if let syn::Stmt::Expr(stmt_expr, _) = stmt {
                    if let syn::Expr::Call(call_expr) = stmt_expr {
                        for arg in call_expr.args.iter_mut() {
                            if let syn::Expr::Path(path_expr) = arg {
                                for segment in path_expr.path.segments.iter_mut() {
                                    let ident = segment.ident.to_string();
                                    if ident.starts_with("__hydro_lang_cluster_ids_") {
                                        let num_partitions = self.num_partitions;
                                        let expr_content = std::mem::replace(expr, syn::Expr::PLACEHOLDER);
                                        *expr = syn::parse_quote! { 
                                            &#expr_content[0..#expr_content.len() / #num_partitions]
                                        };
                                        // Don't need to visit children
                                        return;
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        visit_mut::visit_expr_mut(self, expr);
    }
}

#[cfg(feature = "build")]
fn partition_node(node: &mut HydroNode, partitioner: &Partitioner, next_stmt_id: usize) {
    let Partitioner {
        nodes_to_partition,
        num_partitions,
    } = partitioner;

    node.visit_debug_expr(|expr| {
        let mut visitor = ClusterMembersReplace {
            num_partitions: *num_partitions,
        };
        visitor.visit_expr_mut(&mut expr.0);
    });

    if let Some(partition_attr) = nodes_to_partition.get(&next_stmt_id) {
        println!("Partitioning node {} {}", next_stmt_id, node.print_root());

        let node_content = std::mem::replace(node, HydroNode::Placeholder);
        let metadata = node_content.metadata().clone();

        let f: syn::Expr = match partition_attr {
            PartitionAttribute::All() => {
                syn::parse_quote!(|(orig_dest, item)| {
                    let orig_dest_id = orig_dest.raw_id;
                    let new_dest_id = (orig_dest_id * #num_partitions as u32) + (item as usize % #num_partitions) as u32;
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
                    let new_dest_id = (orig_dest_id * #num_partitions as u32) + (tuple.#tuple_index_ident as usize % #num_partitions) as u32;
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
