use core::panic;
use std::collections::HashMap;

use hydro_lang::ir::{HydroNode, HydroRoot, traverse_dfir};
use hydro_lang::location::LocationId;
use hydro_lang::stream::networking::{deserialize_bincode_with_type, serialize_bincode_with_type};
use serde::{Deserialize, Serialize};
use syn::visit_mut::{self, VisitMut};

use crate::partition_syn_analysis::StructOrTupleIndex;
use crate::repair::inject_id;
use crate::rewrites::{ClusterSelfIdReplace, NetworkType, get_network_type};

#[derive(Clone, Serialize, Deserialize)]
pub struct Partitioner {
    pub nodes_to_partition: HashMap<usize, StructOrTupleIndex>, /* ID of node right before a Network -> what to partition on */
    pub num_partitions: usize,
    pub location_id: usize,
    pub new_cluster_id: Option<usize>, /* If we're partitioning a process, then a new cluster will be created and we'll need to substitute the old ID with this one */
}

/// Don't expose partition members to the cluster
pub struct ClusterMembersReplace {
    pub num_partitions: usize,
    pub location_id: usize,
    pub op_id: usize,
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
                                let prefix =
                                    format!("__hydro_lang_cluster_ids_{}", self.location_id);
                                if ident.starts_with(&prefix) {
                                    let num_partitions = self.num_partitions;
                                    let expr_content =
                                        std::mem::replace(expr, syn::Expr::PLACEHOLDER);
                                    *expr = syn::parse_quote!({
                                        let all_ids = #expr_content;
                                        &all_ids[0..all_ids.len() / #num_partitions]
                                    });
                                    println!(
                                        "Partitioning: Replaced cluster members at node {}",
                                        self.op_id
                                    );
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

fn replace_membership_info(node: &mut HydroNode, partitioner: &Partitioner, op_id: usize) {
    let Partitioner {
        num_partitions,
        location_id,
        ..
    } = *partitioner;

    node.visit_debug_expr(|expr| {
        let mut visitor = ClusterMembersReplace {
            num_partitions,
            location_id,
            op_id,
        };
        visitor.visit_expr_mut(&mut expr.0);
    });
    node.visit_debug_expr(|expr| {
        let mut visitor = ClusterSelfIdReplace::Partition {
            num_partitions,
            partitioned_cluster_id: location_id,
            op_id,
        };
        visitor.visit_expr_mut(&mut expr.0);
    });
}

fn quoted_struct_or_tuple_index(mut tuple: syn::Expr, indices: &StructOrTupleIndex) -> syn::Expr {
    for index in indices {
        let member = if let Ok(num_index) = index.parse::<usize>() {
            syn::Member::Unnamed(syn::Index::from(num_index))
        } else {
            syn::Member::Named(syn::Ident::new(index, proc_macro2::Span::call_site()))
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

fn replace_sender_dest(node: &mut HydroNode, partitioner: &Partitioner, next_stmt_id: usize) {
    let Partitioner {
        nodes_to_partition,
        num_partitions,
        new_cluster_id,
        ..
    } = partitioner;

    if let Some(indices) = nodes_to_partition.get(&next_stmt_id) {
        println!("Replacing sender destination at {}", next_stmt_id);

        let node_content = std::mem::replace(node, HydroNode::Placeholder);
        let mut metadata = node_content.metadata().clone();

        let struct_or_tuple = syn::parse_quote! { struct_or_tuple };
        let struct_or_tuple_with_fields = quoted_struct_or_tuple_index(struct_or_tuple, indices);

        let f: syn::Expr = if new_cluster_id.is_some() {
            // Output type of Map now includes dest ID
            let original_output_type = *metadata.output_type.clone().unwrap().0;
            let new_output_type: syn::Type = syn::parse_quote! {
                (::hydro_lang::MemberId<()>, #original_output_type)
            };
            metadata.output_type = Some(new_output_type.into());

            // Partitioning a process into a cluster
            syn::parse_quote!(
                |struct_or_tuple| {
                    // Hash the field we'll partition on
                    let mut s = ::std::hash::DefaultHasher::new();
                    ::std::hash::Hash::hash(&#struct_or_tuple_with_fields, &mut s);
                    let partition_val = ::std::hash::Hasher::finish(&s) as u32;

                    (
                        ::hydro_lang::MemberId::<()>::from_raw((partition_val % #num_partitions as u32) as u32),
                        struct_or_tuple
                    )
                }
            )
        } else {
            // Already a cluster
            syn::parse_quote!(
                |(orig_dest, struct_or_tuple): (::hydro_lang::MemberId<_>, _)| {
                    // Hash the field we'll partition on
                    let mut s = ::std::hash::DefaultHasher::new();
                    ::std::hash::Hash::hash(&#struct_or_tuple_with_fields, &mut s);
                    let partition_val = ::std::hash::Hasher::finish(&s) as u32;

                    (
                        ::hydro_lang::MemberId::<()>::from_raw((orig_dest.raw_id * #num_partitions as u32) + (partition_val % #num_partitions as u32) as u32),
                        struct_or_tuple
                    )
                }
            )
        };

        let mapped_node = HydroNode::Map {
            f: f.into(),
            input: Box::new(node_content),
            metadata,
        };

        *node = mapped_node;
    }
}

fn replace_receiver_src_id(node: &mut HydroNode, partitioner: &Partitioner, op_id: usize) {
    let Partitioner {
        num_partitions,
        location_id,
        ..
    } = partitioner;

    if let HydroNode::Network {
        input, metadata, ..
    } = node
        && input.metadata().location_kind.root().raw_id() == *location_id
    {
        println!(
            "Rewriting network on op {} so the sender's ID is mapped from the partition to the original sender",
            op_id
        );

        let metadata = metadata.clone();
        let node_content = std::mem::replace(node, HydroNode::Placeholder);
        let f: syn::Expr = syn::parse_quote!(|(sender_id, b)| (
            ::hydro_lang::MemberId::<_>::from_raw(sender_id.raw_id / #num_partitions as u32),
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

fn replace_network_serialization(node: &mut HydroNode, partitioner: &Partitioner, op_id: usize) {
    let Partitioner { new_cluster_id, .. } = partitioner;

    if let Some(network_type) = get_network_type(node, new_cluster_id.unwrap()) {
        let HydroNode::Network {
            serialize_fn,
            deserialize_fn,
            metadata,
            ..
        } = node
        else {
            panic!("Expected a HydroNode::Network, but found {:?}", node);
        };

        let output_type = metadata.output_type.clone().unwrap().0;

        // The partitioned process (now cluster) is the sender
        // Its ID will now be in the recipient's output, so change the deserialize fn
        match network_type {
            NetworkType::Send | NetworkType::SendRecv => {
                println!(
                    "Replacing deserialize function for op {} to include partitioned cluster ID",
                    op_id
                );
                let unit_type: syn::Type = syn::parse_quote! { () }; // Assume we are a Cluster<()>
                let new_deserialize_fn =
                    deserialize_bincode_with_type(Some(&unit_type), &output_type);
                *deserialize_fn = Some(new_deserialize_fn.into());
            }
            _ => {}
        }

        // The partitioned process (now cluster) is the receiver
        // The sender will have to specify its ID, so change the serialize fn
        match network_type {
            NetworkType::Recv | NetworkType::SendRecv => {
                println!(
                    "Replacing serialize function for op {} to include partitioned cluster ID",
                    op_id
                );
                let new_serialize_fn = serialize_bincode_with_type(true, &output_type);
                *serialize_fn = Some(new_serialize_fn.into());
            }
            _ => {}
        }
    }
}

fn replace_process_location_id(
    location: &mut LocationId,
    process_location: usize,
    cluster_location: usize,
) {
    if location.root().raw_id() == process_location {
        location.swap_root(LocationId::Cluster(cluster_location));
    }
}

fn replace_process_input_persist_location_id(
    input: &mut Box<HydroNode>,
    process_location: usize,
    cluster_location: usize,
) {
    if let HydroNode::Persist {
        metadata: persist_metadata,
        ..
    } = input.as_mut()
    {
        replace_process_location_id(
            &mut persist_metadata.location_kind,
            process_location,
            cluster_location,
        );
    }
}

/// If we're partitioning a process into a cluster, we need to replace references to its location
fn replace_process_node_location(node: &mut HydroNode, partitioner: &Partitioner) {
    let Partitioner {
        location_id,
        new_cluster_id,
        ..
    } = partitioner;

    if let Some(new_id) = new_cluster_id {
        // Change any HydroNodes with a location field
        match node {
            // Update Persist's location as well (we won't see it during traversal)
            HydroNode::CrossProduct { left, right, .. } | HydroNode::Join { left, right, .. } => {
                replace_process_input_persist_location_id(left, *location_id, *new_id);
                replace_process_input_persist_location_id(right, *location_id, *new_id);
            }
            HydroNode::Difference { pos, neg, .. } | HydroNode::AntiJoin { pos, neg, .. } => {
                replace_process_input_persist_location_id(pos, *location_id, *new_id);
                replace_process_input_persist_location_id(neg, *location_id, *new_id);
            }
            HydroNode::Fold { input, .. }
            | HydroNode::FoldKeyed { input, .. }
            | HydroNode::Reduce { input, .. }
            | HydroNode::ReduceKeyed { input, .. }
            | HydroNode::Scan { input, .. } => {
                replace_process_input_persist_location_id(input, *location_id, *new_id);
            }
            _ => {}
        }

        // Modify the metadata
        replace_process_location_id(
            &mut node.metadata_mut().location_kind,
            *location_id,
            *new_id,
        );
    }
}

/// If we're partitioning a process into a cluster, we need to replace references to its location
fn replace_process_leaf_location(leaf: &mut HydroRoot, partitioner: &Partitioner) {
    let Partitioner {
        location_id,
        new_cluster_id,
        ..
    } = partitioner;

    if let Some(new_id) = new_cluster_id {
        // Modify the metadata
        if let HydroRoot::CycleSink { out_location, .. } = leaf {
            replace_process_location_id(out_location, *location_id, *new_id);
        }
    }
}

/// If we're partitioning a process into a cluster, we need to remove the default sender ID on outgoing networks
fn remove_sender_id_from_receiver(node: &mut HydroNode, partitioner: &Partitioner, op_id: usize) {
    let Partitioner { new_cluster_id, .. } = partitioner;

    let network_type = get_network_type(node, new_cluster_id.unwrap());
    match network_type {
        Some(NetworkType::Send) | Some(NetworkType::SendRecv) => {
            println!("Removing sender ID from receiver for op {}", op_id);
            let metadata = node.metadata().clone();
            let node_content = std::mem::replace(node, HydroNode::Placeholder);
            let f: syn::Expr = syn::parse_quote!(|(_sender_id, b)| b);

            let mapped_node = HydroNode::Map {
                f: f.into(),
                input: Box::new(node_content),
                metadata,
            };
            *node = mapped_node;
        }
        _ => {}
    }
}

fn partition_node(node: &mut HydroNode, partitioner: &Partitioner, next_stmt_id: &mut usize) {
    replace_membership_info(node, partitioner, *next_stmt_id);
    replace_sender_dest(node, partitioner, *next_stmt_id);

    // Change process into cluster
    if partitioner.new_cluster_id.is_some() {
        replace_process_node_location(node, partitioner);
    } else {
        replace_receiver_src_id(node, partitioner, *next_stmt_id);
    }
}

/// Limitations: Can only partition sends to clusters (not processes). Can only partition sends to 1 cluster at a time. Assumes that the partitioned attribute can be casted to usize.
pub fn partition(ir: &mut [HydroRoot], partitioner: &Partitioner) {
    traverse_dfir(
        ir,
        |_, _| {},
        |node, next_stmt_id| {
            partition_node(node, partitioner, next_stmt_id);
        },
    );

    if partitioner.new_cluster_id.is_some() {
        // Separately traverse roots since CycleSink isn't processed in traverse_dfir
        for root in ir.iter_mut() {
            replace_process_leaf_location(root, partitioner);
        }

        // DANGER: Do not depend on the ID here, since nodes would've been injected
        // Fix network only after all IDs have been replaced, since get_network_type relies on it
        traverse_dfir(
            ir,
            |_, _| {},
            |node, next_stmt_id| {
                replace_network_serialization(node, partitioner, *next_stmt_id);
                remove_sender_id_from_receiver(node, partitioner, *next_stmt_id);
            },
        );
    }

    // Fix IDs since we injected nodes
    inject_id(ir);
}
