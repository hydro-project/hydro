use std::collections::HashMap;

use tokio::sync::mpsc::UnboundedReceiver;

use crate::builder::deploy::DeployResult;
use crate::deploy::deploy_graph::DeployCrateWrapper;
use crate::deploy::HydroDeploy;
use crate::ir::HydroLeaf;
use crate::location::LocationId;
use crate::rewrites::analyze_counter::{inject_count, parse_counter_usage};
use crate::rewrites::analyze_perf::{analyze_perf, parse_cpu_usage};

pub async fn analyze_results(
    nodes: DeployResult<'static, HydroDeploy>,
    ir: &mut [HydroLeaf],
    usage_out: &mut HashMap<(LocationId, String, usize), UnboundedReceiver<String>>,
    cardinality_out: &mut HashMap<(LocationId, String, usize), UnboundedReceiver<String>>,
) {
    for (id, name, cluster) in nodes.get_all_clusters() {
        // Iterate through nodes' usages and keep the max usage one
        let mut max_usage = None;
        for (idx, _) in cluster.members().iter().enumerate() {
            let measurement = usage_out
                .get_mut(&(id.clone(), name.clone(), idx))
                .unwrap()
                .recv()
                .await
                .unwrap();
            let usage = parse_cpu_usage(measurement);
            if let Some((prev_usage, _)) = max_usage {
                if usage > prev_usage {
                    max_usage = Some((usage, idx));
                }
            } else {
                max_usage = Some((usage, idx));
            }
        }

        if let Some((usage, idx)) = max_usage {
            if let Some(perf_results) = cluster.members().get(idx).unwrap().tracing_results().await
            {
                println!("{}: {}", &name, usage);

                // Inject perf usages into metadata
                analyze_perf(ir, perf_results.folded_data);

                // Get cardinality data. Allow later values to overwrite earlier ones
                let node_cardinality = cardinality_out
                    .get_mut(&(id.clone(), name.clone(), idx))
                    .unwrap();
                let mut op_to_counter = HashMap::new();
                while let Some(measurement) = node_cardinality.recv().await {
                    let (op_id, count) = parse_counter_usage(measurement);
                    op_to_counter.insert(op_id, count);
                }

                inject_count(ir, &op_to_counter);
            }
        }
    }
}
