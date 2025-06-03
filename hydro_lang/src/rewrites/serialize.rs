use serde::{Deserialize, Serialize};

use crate::{builder::RewriteIrFlowBuilder, ir::{deep_clone, HydroLeaf}, location::LocationId, rewrites::{decoupler::{self, Decoupler}, partitioner::Partitioner}, Cluster, FlowBuilder, Location};

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
pub fn replay<'a>(rewrites: &mut Rewrites, builder: RewriteIrFlowBuilder<'a>, ir: &[HydroLeaf]) -> (Vec<(Cluster<'a, ()>, usize)>, FlowBuilder<'a>){
    let mut new_clusters = vec![];

    let new_builder = builder.build_with(|builder| {
        let mut ir = deep_clone(&ir);

        // Apply decoupling/partitioning in order
        for rewrite_metadata in rewrites.iter_mut() {
            let new_cluster = builder.cluster::<()>();
            match &mut rewrite_metadata.rewrite {
                Rewrite::Decouple(decoupler) => {
                    decoupler.decoupled_location = new_cluster.id().clone();
                    decoupler::decouple(&mut ir, &decoupler);
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