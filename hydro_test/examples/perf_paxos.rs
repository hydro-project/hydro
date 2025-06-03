use std::collections::HashMap;
use std::sync::Arc;

use hydro_deploy::gcp::GcpNetwork;
use hydro_deploy::Deployment;
use hydro_lang::ir::deep_clone;
use hydro_lang::q;
use hydro_lang::rewrites::decoupler::{self, Decoupler};
use hydro_lang::Location;
use hydro_lang::rewrites::analyze_perf_and_counters::{
    analyze_cluster_results, cleanup_after_analysis, track_cluster_usage_cardinality
};
use hydro_lang::rewrites::reusable_hosts::ReusableHosts;
use hydro_lang::rewrites::{analyze_send_recv_overheads, decouple_analysis, insert_counter, link_cycles, persist_pullup, print_id};
use hydro_test::cluster::paxos::{CorePaxos, PaxosConfig};

use tokio::sync::RwLock;

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

    let mut reusable_hosts = ReusableHosts { 
        hosts: HashMap::new(),
        host_arg: host_arg,
        project: project.clone(),
        network: network.clone(),
    };

    let builder = hydro_lang::FlowBuilder::new();
    let f = 1;
    let num_clients = 3;
    let num_clients_per_node = 500; // Change based on experiment between 1, 50, 100.
    let checkpoint_frequency = 1000; // Num log entries
    let i_am_leader_send_timeout = 5; // Sec
    let i_am_leader_check_timeout = 10; // Sec
    let i_am_leader_check_timeout_delay_multiplier = 15;

    let proposers = builder.cluster();
    let acceptors = builder.cluster();
    let clients = builder.cluster();
    let client_aggregator = builder.process();
    let replicas = builder.cluster();

    hydro_test::cluster::paxos_bench::paxos_bench(
        num_clients_per_node,
        checkpoint_frequency,
        f,
        f + 1,
        CorePaxos {
            proposers: proposers.clone(),
            acceptors: acceptors.clone(),
            paxos_config: PaxosConfig {
                f,
                i_am_leader_send_timeout,
                i_am_leader_check_timeout,
                i_am_leader_check_timeout_delay_multiplier,
            },
        },
        &clients,
        &client_aggregator,
        &replicas,
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
            reusable_hosts.get_cluster_hosts(&mut deployment, "proposer", f + 1),
        )
        .with_cluster(
            &acceptors,
            reusable_hosts.get_cluster_hosts(&mut deployment, "acceptor", 2 * f + 1),
        )
        .with_cluster(
            &clients,
            reusable_hosts
                .get_cluster_hosts(&mut deployment, "client", num_clients),
        )
        .with_process(
            &client_aggregator,
            reusable_hosts.get_process_hosts(&mut deployment, "client-aggregator"),
        )
        .with_cluster(
            &replicas,
            reusable_hosts
                .get_cluster_hosts(&mut deployment, "replica", f + 1),
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

    let (bottleneck, bottleneck_num_nodes) = analyze_cluster_results(&nodes, &mut ir, &mut usage_out, &mut cardinality_out).await;
    cleanup_after_analysis(&mut ir);

    print_id::print_id(&mut ir);

    // Create a mapping from each CycleSink to its corresponding CycleSource
    let cycle_source_to_sink_input = link_cycles::cycle_source_to_sink_input(&mut ir);
    let (send_overhead, recv_overhead) =
        analyze_send_recv_overheads::analyze_send_recv_overheads(&mut ir, &bottleneck);
    let (orig_to_decoupled, decoupled_to_orig, place_on_decoupled) = decouple_analysis::decouple_analysis(
        &mut ir,
        "perf_paxos_cluster",
        &bottleneck,
        send_overhead,
        recv_overhead,
        &cycle_source_to_sink_input,
        true,
    );

    drop(nodes);

    let mut decoupled_cluster = None;

    let new_builder = rewritten_ir_builder.build_with(|builder| {
        let mut ir = deep_clone(&ir);

        decoupled_cluster = Some(builder.cluster::<()>());
        let decoupler = Decoupler {
            output_to_decoupled_machine_after: orig_to_decoupled,
            output_to_original_machine_after: decoupled_to_orig,
            place_on_decoupled_machine: place_on_decoupled,
            orig_location: bottleneck.clone(),
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

    let nodes = optimized_new_builder
        .with_cluster(
            &proposers,
            reusable_hosts.get_cluster_hosts(&mut deployment, "proposer", f + 1),
        )
        .with_cluster(
            &acceptors,
            reusable_hosts.get_cluster_hosts(&mut deployment, "acceptor", 2 * f + 1),
        )
        .with_cluster(
            &clients,
            reusable_hosts
                .get_cluster_hosts(&mut deployment, "client", num_clients),
        )
        .with_process(
            &client_aggregator,
            reusable_hosts.get_process_hosts(&mut deployment, "client-aggregator"),
        )
        .with_cluster(
            &replicas,
            reusable_hosts
                .get_cluster_hosts(&mut deployment, "replica", f + 1),
        )
        .with_cluster(
            &decoupled_cluster.unwrap(),
            reusable_hosts
                .get_cluster_hosts(&mut deployment, "decoupled", bottleneck_num_nodes),
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
