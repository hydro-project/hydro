use std::collections::HashMap;
use std::time::Duration;

use hydro_deploy::Deployment;
use hydro_lang::FlowBuilder;
use hydro_lang::builder::RewriteIrFlowBuilder;
use hydro_lang::builder::deploy::DeployResult;
use hydro_lang::deploy::HydroDeploy;
use hydro_lang::deploy::deploy_graph::DeployCrateWrapper;
use hydro_lang::internal_constants::{COUNTER_PREFIX, CPU_USAGE_PREFIX};
use hydro_lang::ir::{HydroLeaf, HydroNode, deep_clone, traverse_dfir};
use hydro_lang::location::LocationId;
use hydro_lang::rewrites::persist_pullup::persist_pullup;
use tokio::sync::mpsc::UnboundedReceiver;

use crate::decouple_analysis::decouple_analysis;
use crate::decoupler::Decoupler;
use crate::deploy::ReusableHosts;
use crate::parse_results::{analyze_cluster_results, analyze_send_recv_overheads};
use crate::repair::{cycle_source_to_sink_input, inject_id, remove_counter};

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
        | HydroNode::Scan { metadata, .. }
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

fn insert_counter(ir: &mut [HydroLeaf], duration: syn::Expr) {
    traverse_dfir(
        ir,
        |_, _| {},
        |node, next_stmt_id| {
            insert_counter_node(node, next_stmt_id, duration.clone());
        },
    );
}

async fn track_process_usage_cardinality(
    process: &impl DeployCrateWrapper,
) -> (UnboundedReceiver<String>, UnboundedReceiver<String>) {
    (
        process.stdout_filter(CPU_USAGE_PREFIX).await,
        process.stdout_filter(COUNTER_PREFIX).await,
    )
}

async fn track_cluster_usage_cardinality(
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

/// TODO: Return type should be changed to also include Partitioner
pub async fn deploy_and_analyze<'a>(
    reusable_hosts: &mut ReusableHosts,
    deployment: &mut Deployment,
    builder: FlowBuilder<'a>,
    clusters: &Vec<(usize, String, usize)>,
    processes: &Vec<(usize, String)>,
    exclude_from_decoupling: Vec<String>,
    num_seconds: Option<usize>,
) -> (
    RewriteIrFlowBuilder<'a>,
    Vec<HydroLeaf>,
    Decoupler,
    String,
    usize,
) {
    let counter_output_duration = syn::parse_quote!(std::time::Duration::from_secs(1));

    // Rewrite with counter tracking
    let rewritten_ir_builder = builder.rewritten_ir_builder();
    let optimized = builder.optimize_with(persist_pullup).optimize_with(|leaf| {
        insert_counter(leaf, counter_output_duration);
    });
    let mut ir = deep_clone(optimized.ir());

    // Insert all clusters & processes
    let mut deployable = optimized.into_deploy();
    for (cluster_id, name, num_hosts) in clusters {
        deployable = deployable.with_cluster_id_name(
            *cluster_id,
            name.clone(),
            reusable_hosts.get_cluster_hosts(deployment, name.clone(), *num_hosts),
        );
    }
    for (process_id, name) in processes {
        deployable = deployable.with_process_id_name(
            *process_id,
            name.clone(),
            reusable_hosts.get_process_hosts(deployment, name.clone()),
        );
    }
    let nodes = deployable.deploy(deployment);
    deployment.deploy().await.unwrap();

    let (mut usage_out, mut cardinality_out) = track_cluster_usage_cardinality(&nodes).await;

    // Wait for user to input a newline
    deployment
        .start_until(async {
            if let Some(seconds) = num_seconds {
                // Wait for some number of seconds
                tokio::time::sleep(Duration::from_secs(seconds as u64)).await;
            } else {
                // Wait for a new line
                std::io::stdin().read_line(&mut String::new()).unwrap();
            }
        })
        .await
        .unwrap();

    let (bottleneck, bottleneck_name, bottleneck_num_nodes) = analyze_cluster_results(
        &nodes,
        &mut ir,
        &mut usage_out,
        &mut cardinality_out,
        exclude_from_decoupling,
    )
    .await;
    // Remove HydroNode::Counter (since we don't want to consider decoupling those)
    remove_counter(&mut ir);
    // Inject new next_stmt_id into metadata (old ones are invalid after removing the counter)
    inject_id(&mut ir);

    // Create a mapping from each CycleSink to its corresponding CycleSource
    let cycle_source_to_sink_input = cycle_source_to_sink_input(&mut ir);
    let (send_overhead, recv_overhead) = analyze_send_recv_overheads(&mut ir, &bottleneck);
    let (orig_to_decoupled, decoupled_to_orig, place_on_decoupled) = decouple_analysis(
        &mut ir,
        &bottleneck,
        send_overhead,
        recv_overhead,
        &cycle_source_to_sink_input,
    );

    // TODO: Save decoupling decision to file

    (
        rewritten_ir_builder,
        ir,
        Decoupler {
            output_to_decoupled_machine_after: orig_to_decoupled,
            output_to_original_machine_after: decoupled_to_orig,
            place_on_decoupled_machine: place_on_decoupled,
            orig_location: bottleneck.clone(),
            decoupled_location: LocationId::Process(0), // Placeholder, must replace
        },
        bottleneck_name,
        bottleneck_num_nodes,
    )
}
