use hydro_deploy::Deployment;
use hydro_lang::ir::deep_clone;
use hydro_lang::q;
use hydro_lang::rewrites::decoupler::{self, Decoupler};
use hydro_lang::Location;
use hydro_lang::rewrites::analyze_perf_and_counters::{
    analyze_cluster_results, cleanup_after_analysis, perf_cluster_specs, track_cluster_usage_cardinality,
};
use hydro_lang::rewrites::{analyze_send_recv_overheads, decouple_analysis, link_cycles, insert_counter, persist_pullup, print_id};
use hydro_test::cluster::paxos::{CorePaxos, PaxosConfig};

#[tokio::main]
async fn main() {
    let mut deployment = Deployment::new();
    let host_arg = std::env::args().nth(1).unwrap_or_default();

    let builder = hydro_lang::FlowBuilder::new();
    let f = 1;
    let num_clients = 1;
    let num_clients_per_node = 100; // Change based on experiment between 1, 50, 100.
    let median_latency_window_size = 1000;
    let checkpoint_frequency = 1000; // Num log entries
    let i_am_leader_send_timeout = 5; // Sec
    let i_am_leader_check_timeout = 10; // Sec
    let i_am_leader_check_timeout_delay_multiplier = 15;

    let proposers = builder.cluster();
    let acceptors = builder.cluster();

    let (clients, replicas) = hydro_test::cluster::paxos_bench::paxos_bench(
        &builder,
        num_clients_per_node,
        median_latency_window_size,
        checkpoint_frequency,
        f,
        f + 1,
        |replica_checkpoint| CorePaxos {
            proposers: proposers.clone(),
            acceptors: acceptors.clone(),
            replica_checkpoint: replica_checkpoint.broadcast_bincode(&acceptors),
            paxos_config: PaxosConfig {
                f,
                i_am_leader_send_timeout,
                i_am_leader_check_timeout,
                i_am_leader_check_timeout_delay_multiplier,
            },
        },
    );

    let counter_output_duration = q!(std::time::Duration::from_secs(1));

    let rewritten_ir_builder = builder.rewritten_ir_builder();
    let optimized = builder
        .optimize_with(persist_pullup::persist_pullup)
        .optimize_with(|leaf| {
            insert_counter::insert_counter(leaf, counter_output_duration);
        });
    let mut ir = deep_clone(optimized.ir());
    let nodes = optimized
        .with_cluster(
            &proposers,
            perf_cluster_specs(&host_arg, &mut deployment, "proposer", f + 1),
        )
        .with_cluster(
            &acceptors,
            perf_cluster_specs(&host_arg, &mut deployment, "acceptor", 2 * f + 1),
        )
        .with_cluster(
            &clients,
            perf_cluster_specs(&host_arg, &mut deployment, "client", num_clients),
        )
        .with_cluster(
            &replicas,
            perf_cluster_specs(&host_arg, &mut deployment, "replica", f + 1),
        )
        .deploy(&mut deployment);
    
    deployment.deploy().await.unwrap();

    let (mut usage_out, mut cardinality_out) = track_cluster_usage_cardinality(&nodes).await;

    deployment
        .start_until(async {
            std::io::stdin().read_line(&mut String::new()).unwrap();
        })
        .await
        .unwrap();

    analyze_cluster_results(&nodes, &mut ir, &mut usage_out, &mut cardinality_out).await;
    cleanup_after_analysis(&mut ir);

    print_id::print_id(&mut ir);

    // Create a mapping from each CycleSink to its corresponding CycleSource
    let cycle_source_to_sink_input = link_cycles::cycle_source_to_sink_input(&mut ir);
    let (send_overhead, recv_overhead) =
        analyze_send_recv_overheads::analyze_send_recv_overheads(&mut ir, &proposers.id());
    let (orig_to_decoupled, decoupled_to_orig) = decouple_analysis::decouple_analysis(
        &mut ir,
        "perf_paxos_cluster",
        &proposers.id(),
        send_overhead,
        recv_overhead,
        &cycle_source_to_sink_input,
    );

    drop(nodes);

    let mut decoupled_cluster = None;

    let new_builder = rewritten_ir_builder.build_with(|builder| {
        let mut ir = deep_clone(&ir);

        decoupled_cluster = Some(builder.cluster::<()>());
        let decoupler = Decoupler {
            output_to_decoupled_machine_after: orig_to_decoupled,
            output_to_original_machine_after: decoupled_to_orig,
            orig_location: proposers.id().clone(),
            decoupled_location: decoupled_cluster.clone().unwrap().id().clone(),
        };
        decoupler::decouple(&mut ir, &decoupler);

        ir
    });
    let optimized_new_builder = new_builder
        .optimize_with(persist_pullup::persist_pullup)
        .optimize_with(|leaf| {
            insert_counter::insert_counter(leaf, counter_output_duration);
        });

    let mut nodes = optimized_new_builder
        .with_cluster(
            &proposers,
            perf_cluster_specs(&host_arg, &mut deployment, "proposer", f + 1),
        )
        .with_cluster(
            &acceptors,
            perf_cluster_specs(&host_arg, &mut deployment, "acceptor", 2 * f + 1),
        )
        .with_cluster(
            &clients,
            perf_cluster_specs(&host_arg, &mut deployment, "client", num_clients),
        )
        .with_cluster(
            &replicas,
            perf_cluster_specs(&host_arg, &mut deployment, "replica", f + 1),
        )
        .with_cluster(
            &decoupled_cluster.unwrap(),
            perf_cluster_specs(&host_arg, &mut deployment, "decoupled", f + 1),
        )
        .deploy(&mut deployment);

    deployment.deploy().await.unwrap();

    let (mut usage_out, mut cardinality_out) = track_cluster_usage_cardinality(&nodes).await;

    deployment
        .start_until(async {
            std::io::stdin().read_line(&mut String::new()).unwrap();
        })
        .await
        .unwrap();
}
