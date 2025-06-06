use std::{collections::HashMap, sync::Arc};

use hydro_deploy::{gcp::GcpNetwork, rust_crate::tracing_options::{TracingOptions, DEBIAN_PERF_SETUP_COMMAND}, Deployment, Host};
use stageleft::q;
use tokio::sync::RwLock;

use hydro_lang::{builder::RewriteIrFlowBuilder, deploy::TrybuildHost, ir::deep_clone, rewrites::persist_pullup::persist_pullup, FlowBuilder};

use hydro_lang::ir::HydroLeaf;
use hydro_lang::location::LocationId;

use crate::{decouple_analysis::decouple_analysis, decoupler::Decoupler, inject_profiling::{insert_counter, track_cluster_usage_cardinality}, parse_results::{analyze_cluster_results, analyze_send_recv_overheads}, repair::{cycle_source_to_sink_input, inject_id, remove_counter}};

pub struct ReusableHosts {
    pub hosts: HashMap<String, Arc<dyn Host>>, // Key = display_name
    pub host_arg: String,
    pub project: String,
    pub network: Arc<RwLock<GcpNetwork>>,
}

impl ReusableHosts {
    // NOTE: Creating hosts with the same display_name in the same deployment will result in undefined behavior.
    fn lazy_create_host(&mut self, deployment: &mut Deployment, display_name: String) -> Arc<dyn Host> {
        self.hosts.entry(display_name.clone())
            .or_insert_with(|| {
                if self.host_arg == "gcp" {
                    deployment
                        .GcpComputeEngineHost()
                        .project(&self.project)
                        .machine_type("n2-standard-4")
                        .image("debian-cloud/debian-12")
                        .region("us-central1-c")
                        .network(self.network.clone())
                        .display_name(display_name)
                        .add()
                } else {
                    deployment.Localhost()
                }
            })
            .clone()
    }

    fn create_trybuild_host(&mut self, deployment: &mut Deployment, display_name: String) -> TrybuildHost {
        let rustflags = if self.host_arg == "gcp" {
            "-C opt-level=3 -C codegen-units=1 -C strip=none -C debuginfo=2 -C lto=off -C link-args=--no-rosegment"
        } else {
            "-C opt-level=3 -C codegen-units=1 -C strip=none -C debuginfo=2 -C lto=off"
        };
        TrybuildHost::new(self.lazy_create_host(deployment, display_name.clone()))
            .additional_hydro_features(vec!["runtime_measure".to_string()])
            .rustflags(rustflags)
            .tracing(
                TracingOptions::builder()
                    .perf_raw_outfile(format!("{}.perf.data", display_name.clone()))
                    .fold_outfile(format!("{}.data.folded", display_name))
                    .frequency(128)
                    .setup_command(DEBIAN_PERF_SETUP_COMMAND)
                    .build(),
            )
    }

    pub fn get_cluster_hosts(&mut self, deployment: &mut Deployment, cluster_name: String, num_hosts: usize) -> Vec<TrybuildHost> {
        (0..num_hosts).map(|i| {
            self.create_trybuild_host(deployment, format!("{}{}", cluster_name, i))
        }).collect()
    }

    pub fn get_process_hosts(&mut self, deployment: &mut Deployment, display_name: String) -> TrybuildHost {
        self.create_trybuild_host(deployment, display_name)
    }
}

/// TODO: Return type should be changed to also include Partitioner
pub async fn deploy_and_analyze<'a>(reusable_hosts: &mut ReusableHosts, deployment: &mut Deployment, builder: FlowBuilder<'a>, clusters: &Vec<(usize, String, usize)>, processes: &Vec<(usize, String)>) -> (RewriteIrFlowBuilder<'a>, Vec<HydroLeaf>, Decoupler, usize) {
    let counter_output_duration = q!(std::time::Duration::from_secs(1));
    
    // Rewrite with counter tracking
    let rewritten_ir_builder = builder.rewritten_ir_builder();
    let optimized = builder
        .optimize_with(persist_pullup)
        .optimize_with(|leaf| {
            insert_counter(leaf, counter_output_duration);
        });
    let mut ir = deep_clone(optimized.ir());

    // Insert all clusters & processes
    let mut deployable = optimized.into_deploy();
    for (cluster_id, name, num_hosts) in clusters {
        deployable = deployable.with_cluster_id_name(*cluster_id, name.clone(), reusable_hosts.get_cluster_hosts(deployment, name.clone(), *num_hosts));
    }
    for (process_id, name) in processes {
        deployable = deployable.with_process_id_name(*process_id, name.clone(), reusable_hosts.get_process_hosts(deployment, name.clone()));
    }
    let nodes = deployable.deploy(deployment);
    deployment.deploy().await.unwrap();

    let (mut usage_out, mut cardinality_out) = track_cluster_usage_cardinality(&nodes).await;

    // Wait for user to input a newline
    deployment
        .start_until(async {
            std::io::stdin().read_line(&mut String::new()).unwrap();
        })
        .await
        .unwrap();
        
    let (bottleneck, bottleneck_num_nodes) = analyze_cluster_results(&nodes, &mut ir, &mut usage_out, &mut cardinality_out).await;
    // Remove HydroNode::Counter (since we don't want to consider decoupling those)
    remove_counter(&mut ir);
    // Inject new next_stmt_id into metadata (old ones are invalid after removing the counter)
    inject_id(&mut ir);

    // print_id(&mut ir);

    // Create a mapping from each CycleSink to its corresponding CycleSource
    let cycle_source_to_sink_input = cycle_source_to_sink_input(&mut ir);
    let (send_overhead, recv_overhead) = analyze_send_recv_overheads(&mut ir, &bottleneck);
    let (orig_to_decoupled, decoupled_to_orig, place_on_decoupled) = decouple_analysis(
        &mut ir,
        "decouple",
        &bottleneck,
        send_overhead,
        recv_overhead,
        &cycle_source_to_sink_input,
        true,
    );

    // TODO: Save decoupling decision to file

    (rewritten_ir_builder, ir, Decoupler {
        output_to_decoupled_machine_after: orig_to_decoupled,
        output_to_original_machine_after: decoupled_to_orig,
        place_on_decoupled_machine: place_on_decoupled,
        orig_location: bottleneck.clone(),
        decoupled_location: LocationId::Process(0), // Placeholder, must replace
    }, bottleneck_num_nodes)
}

