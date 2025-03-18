use hydro_deploy::Deployment;
use hydro_lang::ir::deep_clone;
use hydro_lang::rewrites::analyze_perf_and_counters::{
    analyze_cluster_results, analyze_process_results, cleanup_after_analysis, get_usage,
    perf_cluster_specs, perf_process_specs, track_cluster_usage_cardinality,
    track_process_usage_cardinality,
};
use hydro_lang::rewrites::{
    analyze_send_recv_overheads, decouple_analysis, insert_counter, link_cycles, persist_pullup,
};
use hydro_lang::{q, Location};

// run with no args for localhost, with `gcp <GCP PROJECT>` for GCP
#[tokio::main]
async fn main() {
    let mut deployment = Deployment::new();
    let host_arg = std::env::args().nth(1).unwrap_or_default();
    let project = if host_arg == "gcp" {
        std::env::args().nth(2).unwrap()
    } else {
        String::new()
    };
    let network = Arc::new(RwLock::new(GcpNetwork::new(&project, None)));

    let builder = hydro_lang::FlowBuilder::new();
    let (cluster, leader) = hydro_test::cluster::compute_pi::compute_pi(&builder, 8192);

    let counter_output_duration = q!(std::time::Duration::from_secs(1));

    let optimized = builder
        .optimize_with(persist_pullup::persist_pullup)
        .optimize_with(|ir| insert_counter::insert_counter(ir, counter_output_duration));
    let mut ir = deep_clone(optimized.ir());
    let nodes = optimized
        .with_process(
            &leader,
            perf_process_specs(&host_arg, project.clone(), network.clone(), &mut deployment, "leader"),
        )
        .with_cluster(
            &cluster,
            perf_cluster_specs(&host_arg, project.clone(), network.clone(), &mut deployment, "cluster", 8),
        )
        .deploy(&mut deployment);

    deployment.deploy().await.unwrap();

    let leader_process = nodes.get_process(&leader);
    let (mut leader_usage_out, mut leader_cardinality_out) =
        track_process_usage_cardinality(leader_process).await;
    let (mut usage_out, mut cardinality_out) = track_cluster_usage_cardinality(&nodes).await;

    deployment
        .start_until(async {
            std::io::stdin().read_line(&mut String::new()).unwrap();
        })
        .await
        .unwrap();

    analyze_process_results(
        leader_process,
        &mut ir,
        get_usage(&mut leader_usage_out).await,
        &mut leader_cardinality_out,
    )
    .await;
    analyze_cluster_results(&nodes, &mut ir, &mut usage_out, &mut cardinality_out).await;
    cleanup_after_analysis(&mut ir);

    hydro_lang::ir::dbg_dedup_tee(|| {
        println!("{:#?}", ir);
    });

    // Create a mapping from each CycleSink to its corresponding CycleSource
    let cycle_source_to_sink_inputs = link_cycles::cycle_source_to_sink_input(&mut ir);
    let (send_overhead, recv_overhead) =
        analyze_send_recv_overheads::analyze_send_recv_overheads(&mut ir, &cluster.id());
    decouple_analysis::decouple_analysis(
        &mut ir,
        "perf_compute_pi_cluster",
        &cluster.id(),
        send_overhead,
        recv_overhead,
        &cycle_source_to_sink_inputs,
    );
}
