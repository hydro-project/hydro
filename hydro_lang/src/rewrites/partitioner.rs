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
    pub partitioned_cluster_id: usize,
}

// Placeholder ClusterId type
pub struct Partitioned {}

/// Replace CLUSTER_SELF_ID with the ID of the original node the partition is assigned to
#[cfg(feature = "build")]
pub struct ClusterSelfIdReplace {
    pub num_partitions: usize,
    pub partitioned_cluster_id: usize,
}

#[cfg(feature = "build")]
impl VisitMut for ClusterSelfIdReplace {
    fn visit_expr_mut(&mut self, expr: &mut syn::Expr) {
        if let syn::Expr::Path(path_expr) = expr {
            for segment in path_expr.path.segments.iter_mut() {
                let ident = segment.ident.to_string();
                let prefix = format!("__hydro_lang_cluster_self_id_{}", self.partitioned_cluster_id);
                if ident.starts_with(&prefix) {
                    let num_partitions = self.num_partitions;
                    let expr_content = std::mem::replace(expr, syn::Expr::PLACEHOLDER);
                    *expr = syn::parse_quote!({
                        #expr_content % #num_partitions as u32
                    });
                    println!("Partitioning: Replaced CLUSTER_SELF_ID");
                    return;
                }
            }
        }
        visit_mut::visit_expr_mut(self, expr);
    }
}

/// Don't expose partition members to the cluster
#[cfg(feature = "build")]
pub struct ClusterMembersReplace {
    pub num_partitions: usize,
    pub partitioned_cluster_id: usize,
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
                                    let prefix = format!("__hydro_lang_cluster_ids_{}", self.partitioned_cluster_id);
                                    if ident.starts_with(&prefix) {
                                        let num_partitions = self.num_partitions;
                                        let expr_content =
                                            std::mem::replace(expr, syn::Expr::PLACEHOLDER);
                                        *expr = syn::parse_quote!({
                                            let all_ids = #expr_content;
                                            &all_ids[0..all_ids.len() / #num_partitions]
                                        });
                                        println!("Partitioning: Replaced cluster members");
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
fn replace_membership_info(node: &mut HydroNode, num_partitions: usize, partitioned_cluster_id: usize) {
    node.visit_debug_expr(|expr| {
        let mut visitor = ClusterMembersReplace { num_partitions, partitioned_cluster_id };
        visitor.visit_expr_mut(&mut expr.0);
    });
    node.visit_debug_expr(|expr| {
        let mut visitor = ClusterSelfIdReplace { num_partitions, partitioned_cluster_id };
        visitor.visit_expr_mut(&mut expr.0);
    });
}

#[cfg(feature = "build")]
fn partition_node(node: &mut HydroNode, partitioner: &Partitioner, next_stmt_id: usize) {
    let Partitioner {
        nodes_to_partition,
        num_partitions,
        partitioned_cluster_id,
    } = partitioner;

    replace_membership_info(node, *num_partitions, *partitioned_cluster_id);

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
