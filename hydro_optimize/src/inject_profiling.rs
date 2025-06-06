use std::{collections::HashMap, time::Duration};

use stageleft::Quoted;

use hydro_lang::ir::{traverse_dfir, HydroNode};

use tokio::sync::mpsc::UnboundedReceiver;

use hydro_lang::builder::deploy::DeployResult;
use hydro_lang::deploy::deploy_graph::DeployCrateWrapper;
use hydro_lang::deploy::HydroDeploy;
use hydro_lang::ir::HydroLeaf;
use hydro_lang::location::LocationId;

fn insert_counter_node(node: &mut HydroNode, next_stmt_id: &mut usize, duration: syn::Expr) {
    match node {
        HydroNode::Placeholder
        | HydroNode::Unpersist { .. }
        | HydroNode::Counter { .. } => {
            std::panic!("Unexpected {:?} found in insert_counter_node", node.print_root());
        }
        HydroNode::Source { metadata, .. }
        | HydroNode::CycleSource { metadata, .. }
        | HydroNode::Persist { metadata, .. }
        | HydroNode::Delta { metadata, .. }
        | HydroNode::Chain { metadata, .. } // Can technically be derived by summing parent cardinalities
        | HydroNode::CrossSingleton { metadata, .. }
        | HydroNode::CrossProduct { metadata, .. } // Can technically be derived by multiplying parent cardinalities
        | HydroNode::Join { metadata, .. }
        | HydroNode::ResolveFutures { metadata, .. }
        | HydroNode::ResolveFuturesOrdered { metadata, .. }
        | HydroNode::Difference { metadata, .. }
        | HydroNode::AntiJoin { metadata, .. }
        | HydroNode::FlatMap { metadata, .. }
        | HydroNode::Filter { metadata, .. }
        | HydroNode::FilterMap { metadata, .. }
        | HydroNode::Unique { metadata, .. }
        | HydroNode::Fold { metadata, .. } // Output 1 value per tick
        | HydroNode::Reduce { metadata, .. } // Output 1 value per tick
        | HydroNode::FoldKeyed { metadata, .. }
        | HydroNode::ReduceKeyed { metadata, .. }
        | HydroNode::Network { metadata, .. }
         => {
            let metadata = metadata.clone();
            let node_content = std::mem::replace(node, HydroNode::Placeholder);

            let counter = HydroNode::Counter {
                tag: next_stmt_id.to_string(),
                duration: duration.into(),
                input: Box::new(node_content),
                metadata: metadata.clone(),
            };

            // when we emit this IR, the counter will bump the stmt id, so simulate that here
            *next_stmt_id += 1;

            *node = counter;
        }
        HydroNode::Tee { .. } // Do nothing, we will count the parent of the Tee
        | HydroNode::Map { .. } // Equal to parent cardinality
        | HydroNode::DeferTick { .. } // Equal to parent cardinality
        | HydroNode::Enumerate { .. }
        | HydroNode::Inspect { .. }
        | HydroNode::Sort { .. }
         => {}
    }
}

pub fn insert_counter(ir: &mut [HydroLeaf], duration: impl Quoted<'static, Duration>) {
    let duration = duration.splice_typed();
    traverse_dfir(
        ir,
        |_, _| {},
        |node, next_stmt_id| {
            insert_counter_node(node, next_stmt_id, duration.clone());
        },
    );
}

use hydro_lang::internal_constants::{
    CPU_USAGE_PREFIX, COUNTER_PREFIX,
};

async fn track_process_usage_cardinality(
    process: &impl DeployCrateWrapper,
) -> (UnboundedReceiver<String>, UnboundedReceiver<String>) {
    (
        process.stdout_filter(CPU_USAGE_PREFIX).await,
        process.stdout_filter(COUNTER_PREFIX).await,
    )
}

pub(crate) async fn track_cluster_usage_cardinality(
    nodes: &DeployResult<'_, HydroDeploy>,
) -> (
    HashMap<(LocationId, String, usize), UnboundedReceiver<String>>,
    HashMap<(LocationId, String, usize), UnboundedReceiver<String>>,
) {
    let mut usage_out = HashMap::new();
    let mut cardinality_out = HashMap::new();
    for (id, name, cluster) in nodes.get_all_clusters() {
        for (idx, node) in cluster.members().iter().enumerate() {
            let (node_usage_out, node_cardinality_out) =
                track_process_usage_cardinality(node).await;
            usage_out.insert((id.clone(), name.clone(), idx), node_usage_out);
            cardinality_out.insert((id.clone(), name.clone(), idx), node_cardinality_out);
        }
    }
    for (id, name, process) in nodes.get_all_processes() {
        let (process_usage_out, process_cardinality_out) =
            track_process_usage_cardinality(process).await;
        usage_out.insert((id.clone(), name.clone(), 0), process_usage_out);
        cardinality_out.insert((id.clone(), name.clone(), 0), process_cardinality_out);
    }
    (usage_out, cardinality_out)
}