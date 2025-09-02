use hydro_lang::builder::RewriteIrFlowBuilder;
use hydro_lang::ir::{HydroIrMetadata, HydroNode, HydroRoot, deep_clone};
use hydro_lang::location::LocationId;
use hydro_lang::{Cluster, FlowBuilder, Location};
use serde::{Deserialize, Serialize};
use syn::visit_mut::{self, VisitMut};

use crate::decoupler::{self, Decoupler};
use crate::partitioner::Partitioner;

#[derive(Clone, Serialize, Deserialize)]
pub enum Rewrite {
    Decouple(Decoupler),
    Partition(Partitioner),
}

#[derive(Clone, Serialize, Deserialize)]
pub struct RewriteMetadata {
    pub node: LocationId,
    pub num_nodes: usize,
    pub rewrite: Rewrite,
}

pub type Rewrites = Vec<RewriteMetadata>;

/// Replays the rewrites in order.
/// Returns Vec(Cluster, number of nodes) for each created cluster and a new FlowBuilder
pub fn replay<'a>(
    rewrites: &mut Rewrites,
    builder: RewriteIrFlowBuilder<'a>,
    ir: &[HydroRoot],
) -> (Vec<(Cluster<'a, ()>, usize)>, FlowBuilder<'a>) {
    let mut new_clusters = vec![];

    let new_builder = builder.build_with(|builder| {
        let mut ir = deep_clone(ir);

        // Apply decoupling/partitioning in order
        for rewrite_metadata in rewrites.iter_mut() {
            let new_cluster = builder.cluster::<()>();
            match &mut rewrite_metadata.rewrite {
                Rewrite::Decouple(decoupler) => {
                    decoupler.decoupled_location = new_cluster.id().clone();
                    decoupler::decouple(&mut ir, decoupler);
                }
                Rewrite::Partition(_partitioner) => {
                    panic!("Partitioning is not yet replayable");
                }
            }
            new_clusters.push((new_cluster, rewrite_metadata.num_nodes));
        }

        ir
    });

    (new_clusters, new_builder)
}

/// Replace CLUSTER_SELF_ID with the ID of the original node the partition is assigned to
#[derive(Copy, Clone)]
pub enum ClusterSelfIdReplace {
    Decouple {
        orig_cluster_id: usize,
        decoupled_cluster_id: usize,
    },
    Partition {
        num_partitions: usize,
        partitioned_cluster_id: usize,
        op_id: usize,
    },
}

impl VisitMut for ClusterSelfIdReplace {
    fn visit_expr_mut(&mut self, expr: &mut syn::Expr) {
        if let syn::Expr::Path(path_expr) = expr {
            for segment in path_expr.path.segments.iter_mut() {
                let ident = segment.ident.to_string();

                match self {
                    ClusterSelfIdReplace::Decouple {
                        orig_cluster_id,
                        decoupled_cluster_id,
                    } => {
                        let prefix = format!("__hydro_lang_cluster_self_id_{}", orig_cluster_id);
                        if ident.starts_with(&prefix) {
                            segment.ident = syn::Ident::new(
                                &format!("__hydro_lang_cluster_self_id_{}", decoupled_cluster_id),
                                segment.ident.span(),
                            );
                            println!("Decoupling: Replaced CLUSTER_SELF_ID");
                            return;
                        }
                    }
                    ClusterSelfIdReplace::Partition {
                        num_partitions,
                        partitioned_cluster_id,
                        op_id,
                    } => {
                        let prefix =
                            format!("__hydro_lang_cluster_self_id_{}", partitioned_cluster_id);
                        if ident.starts_with(&prefix) {
                            let expr_content = std::mem::replace(expr, syn::Expr::PLACEHOLDER);
                            *expr = syn::parse_quote!({
                                #expr_content / #num_partitions as u32
                            });
                            println!("Partitioning: Replaced CLUSTER_SELF_ID for node {}", op_id);
                            return;
                        }
                    }
                }
            }
        }
        visit_mut::visit_expr_mut(self, expr);
    }
}

pub fn relevant_inputs(
    input_metadatas: Vec<&HydroIrMetadata>,
    cluster_to_decouple: &LocationId,
) -> Vec<usize> {
    input_metadatas
        .iter()
        .filter_map(|input_metadata| {
            if cluster_to_decouple == input_metadata.location_kind.root() {
                Some(input_metadata.op.id.unwrap())
            } else {
                None
            }
        })
        .collect()
}

#[derive(Clone, PartialEq, Eq)]
pub enum NetworkType {
    Recv,
    Send,
    SendRecv,
}

pub fn get_network_type(node: &HydroNode, location: usize) -> Option<NetworkType> {
    let mut is_to_us = false;
    let mut is_from_us = false;

    if let HydroNode::Network { input, .. } = node {
        if input.metadata().location_kind.root().raw_id() == location {
            is_from_us = true;
        }
        if node.metadata().location_kind.root().raw_id() == location {
            is_to_us = true;
        }

        return if is_from_us && is_to_us {
            Some(NetworkType::SendRecv)
        } else if is_from_us {
            Some(NetworkType::Send)
        } else if is_to_us {
            Some(NetworkType::Recv)
        } else {
            None
        };
    }
    None
}
