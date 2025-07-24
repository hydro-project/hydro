use std::collections::HashMap;

use hydro_lang::ir::{HydroLeaf, HydroNode, traverse_dfir};
use serde::{Deserialize, Serialize};
use syn::visit_mut::{self, VisitMut};

use crate::{partition_syn_analysis::StructOrTupleIndex, rewrites::ClusterSelfIdReplace};

#[derive(Clone, Serialize, Deserialize)]
pub struct Partitioner {
    pub nodes_to_partition: HashMap<usize, StructOrTupleIndex>, /* ID of node right before a Network -> what to partition on */
    pub num_partitions: usize,
    pub partitioned_cluster_id: usize,
}

/// Don't expose partition members to the cluster
pub struct ClusterMembersReplace {
    pub num_partitions: usize,
    pub partitioned_cluster_id: usize,
}

impl VisitMut for ClusterMembersReplace {
    fn visit_expr_mut(&mut self, expr: &mut syn::Expr) {
        if let syn::Expr::Unsafe(unsafe_expr) = expr {
            for stmt in &mut unsafe_expr.block.stmts {
                if let syn::Stmt::Expr(syn::Expr::Call(call_expr), _) = stmt {
                    for arg in call_expr.args.iter_mut() {
                        if let syn::Expr::Path(path_expr) = arg {
                            for segment in path_expr.path.segments.iter_mut() {
                                let ident = segment.ident.to_string();
                                let prefix = format!(
                                    "__hydro_lang_cluster_ids_{}",
                                    self.partitioned_cluster_id
                                );
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
        visit_mut::visit_expr_mut(self, expr);
    }
}

fn replace_membership_info(node: &mut HydroNode, partitioner: &Partitioner) {
    let Partitioner {
        num_partitions,
        partitioned_cluster_id,
        ..
    } = *partitioner;

    node.visit_debug_expr(|expr| {
        let mut visitor = ClusterMembersReplace {
            num_partitions,
            partitioned_cluster_id,
        };
        visitor.visit_expr_mut(&mut expr.0);
    });
    node.visit_debug_expr(|expr| {
        let mut visitor = ClusterSelfIdReplace::Partition {
            num_partitions,
            partitioned_cluster_id,
        };
        visitor.visit_expr_mut(&mut expr.0);
    });
}

fn quoted_struct_or_tuple_index(mut tuple: syn::Expr, indices: &StructOrTupleIndex) -> syn::Expr {
    for index in indices {
        let member = if let Ok(num_index) = index.parse::<usize>() {
            syn::Member::Unnamed(syn::Index::from(num_index))
        } else {
            syn::Member::Named(syn::Ident::new(&index, proc_macro2::Span::call_site()))
        };

        let dot_token = <syn::Token![.]>::default();

        tuple = syn::Expr::Field(syn::ExprField {
            attrs: vec![],
            base: Box::new(tuple),
            dot_token,
            member,
        });
    }
    tuple
}

fn replace_sender_network(node: &mut HydroNode, partitioner: &Partitioner, next_stmt_id: usize) {
    let Partitioner {
        nodes_to_partition,
        num_partitions,
        ..
    } = partitioner;

    if let Some(indices) = nodes_to_partition.get(&next_stmt_id) {
        println!("Partitioning node {} {}", next_stmt_id, node.print_root());

        let node_content = std::mem::replace(node, HydroNode::Placeholder);
        let metadata = node_content.metadata().clone();

        let struct_or_tuple = syn::parse_quote!{ struct_or_tuple };
        let struct_or_tuple_with_fields = quoted_struct_or_tuple_index(
            struct_or_tuple,
            indices,
        );
        let f: syn::Expr = syn::parse_quote!(
            |(orig_dest, struct_or_tuple)| {
                use std::any::{Any, TypeId};
                use std::hash::{Hash, Hasher, DefaultHasher};

                // Hash the field we'll partition on
                let mut s = DefaultHasher::new();
                #struct_or_tuple_with_fields.hash(&mut s);
                let partition_val = s.finish() as u32;

                (
                    (orig_dest * #num_partitions as u32) + (partition_val % #num_partitions as u32) as u32,
                    struct_or_tuple
                )
            }
        );

        let mapped_node = HydroNode::Map {
            f: f.into(),
            input: Box::new(node_content),
            metadata,
        };

        *node = mapped_node;
    }
}

fn replace_receiver_network(node: &mut HydroNode, partitioner: &Partitioner) {
    let Partitioner {
        num_partitions,
        partitioned_cluster_id,
        ..
    } = partitioner;

    if let HydroNode::Network {
        input, metadata, ..
    } = node
    {
        if input.metadata().location_kind.root().raw_id() == *partitioned_cluster_id {
            println!("Rewriting network on receiver to remap location ids");

            let metadata = metadata.clone();
            let node_content = std::mem::replace(node, HydroNode::Placeholder);
            let f: syn::Expr = syn::parse_quote!(|(sender_id, b)| (
                ClusterId::<_>::from_raw(sender_id.raw_id / #num_partitions as u32),
                b
            ));

            let mapped_node = HydroNode::Map {
                f: f.into(),
                input: Box::new(node_content),
                metadata: metadata.clone(),
            };

            *node = mapped_node;
        }
    }
}

fn partition_node(node: &mut HydroNode, partitioner: &Partitioner, next_stmt_id: &mut usize) {
    replace_membership_info(node, partitioner);
    replace_sender_network(node, partitioner, *next_stmt_id);
    replace_receiver_network(node, partitioner);
}

/// Limitations: Can only partition sends to clusters (not processes). Can only partition sends to 1 cluster at a time. Assumes that the partitioned attribute can be casted to usize.
pub fn partition(ir: &mut [HydroLeaf], partitioner: &Partitioner) {
    traverse_dfir(
        ir,
        |_, _| {},
        |node, next_stmt_id| {
            partition_node(node, partitioner, next_stmt_id);
        },
    );
}
