use hydro_lang::live_collections::stream::NoOrder;
use hydro_lang::location::cluster::CLUSTER_SELF_ID;
use hydro_lang::prelude::*;
use hydro_std::bench_client::{
    BenchResult, aggregate_bench_results, bench_client, compute_throughput_latency,
};

use super::paxos_with_client::PaxosLike;
use crate::cluster::paxos_bench::inc_i32_workload_generator;

pub struct Client;
pub struct Aggregator;

#[expect(clippy::too_many_arguments, reason = "internal paxos code // TODO")]
pub fn paxos_log_bench<'a>(
    checkpoint_frequency: usize, // How many sequence numbers to commit before checkpointing
    paxos: impl PaxosLike<'a>,
    clients: &Cluster<'a, Client>,
    num_clients_per_node: Singleton<usize, Cluster<'a, Client>, Bounded>,
    client_aggregator: &Process<'a, Aggregator>,
    client_interval_millis: u64,
    aggregate_interval_millis: u64,
    print_results: impl FnOnce(BenchResult<Process<'a, Aggregator>>),
) {
    let latencies = bench_client(
        clients,
        num_clients_per_node,
        inc_i32_workload_generator,
        |input| {
            let acceptors = paxos.log_stores().clone();
            let (acceptor_checkpoint_complete, acceptor_checkpoint) =
                acceptors.forward_ref::<Optional<_, _, _>>();

            let sequenced_payloads = paxos.with_client(
                clients,
                input
                    .entries()
                    .map(q!(move |(virtual_id, payload)| {
                        // Append Client ID so replicas know who to contact later
                        (virtual_id, (CLUSTER_SELF_ID.clone(), payload))
                    }))
                    .assume_ordering_same_consistency(nondet!(/** benchmarking, order actually doesn't matter */)),
                acceptor_checkpoint,
                // TODO(shadaj): we should retry when a payload is dropped due to stale leader
                nondet!(/** benchmarking, assuming no re-election */),
                nondet!(
                    /// clients 'own' certain keys, so interleaving elements from clients will not affect
                    /// the order of writes to the same key
                ),
            );

            // Leader sends committed payloads directly to client
            let sequenced_to_clients = sequenced_payloads
                .clone()
                .map(q!(|(_seq, payload)| {
                    let (virtual_id, (client_location, value)) = payload.unwrap();
                    (client_location, (virtual_id, value))
                }))
                .demux(clients, TCP.fail_stop().bincode())
                .values()
                .into_keyed();

            // Compute checkpoints on the leader
            let nondet_log_holes = nondet!(/** Current max log sequence number dpeends on when the commit is confirmed */);
            let p_checkpoint = sliced! {
                let new_slots = use(sequenced_payloads.into_keyed().keys(), nondet_log_holes);
                let mut log_holes = use::state_null::<Stream<usize, Tick<_>, Bounded, NoOrder>>();
                let mut prev_checkpoint_slot = use::state::<Singleton<usize, Tick<_>, Bounded>>(|l| l.singleton(q!(0)));

                // Min log hole = max contiguous slot
                let max_contiguous_slot = log_holes.clone().min().unwrap_or_default();
                let new_checkpoint = max_contiguous_slot
                    .zip(prev_checkpoint_slot.clone())
                    .filter_map(q!(move |(max_contiguous, prev_checkpoint)|
                        (max_contiguous - prev_checkpoint >= checkpoint_frequency).then_some(max_contiguous)));
                prev_checkpoint_slot = new_checkpoint.clone().unwrap_or(prev_checkpoint_slot);

                // Calculate the new log holes
                let max_log_hole = log_holes.clone().max().unwrap_or_default();
                let max_new_slot = new_slots.clone().max();
                // max_new_slot+2 because we want the next hole to be the largest slot + 1. The other +1 is because the range is exclusive
                let new_potential_holes = max_log_hole
                    .zip(max_new_slot)
                    .flat_map_unordered(q!(|(max_hole, max_new_slot)| max_hole+1..max_new_slot+2));
                let new_holes = new_potential_holes.chain(log_holes.clone())
                    .filter_not_in(new_slots);
                log_holes = new_holes;

                new_checkpoint.into_stream()
            };
            let a_checkpoint = p_checkpoint
                .broadcast(&acceptors, TCP.fail_stop().bincode(), nondet!(/** Acceptor membership is static */))
                .values()
                .max();
            acceptor_checkpoint_complete.complete(a_checkpoint);

            sequenced_to_clients
        },
    )
    .entries()
    .map(q!(|(_virtual_client_id, (_output, latency))| latency));

    // Create throughput/latency graphs
    let bench_results = compute_throughput_latency(
        clients,
        latencies,
        client_interval_millis,
        nondet!(/** bench */),
    );
    let aggregate_results =
        aggregate_bench_results(bench_results, client_aggregator, aggregate_interval_millis);
    print_results(aggregate_results);
}

#[cfg(test)]
mod tests {
    use hydro_deploy::Deployment;
    use hydro_lang::deploy::{DeployCrateWrapper, TrybuildHost};

    #[cfg(stageleft_runtime)]
    use crate::cluster::paxos::{CorePaxos, PaxosConfig};

    const PAXOS_F: usize = 1;

    #[cfg(stageleft_runtime)]
    fn create_paxos<'a>(
        proposers: &hydro_lang::location::Cluster<'a, crate::cluster::paxos::Proposer>,
        acceptors: &hydro_lang::location::Cluster<'a, crate::cluster::paxos::Acceptor>,
        clients: &hydro_lang::location::Cluster<'a, super::Client>,
        client_aggregator: &hydro_lang::location::Process<'a, super::Aggregator>,
    ) {
        use hydro_lang::location::Location;
        use hydro_std::bench_client::pretty_print_bench_results;
        use stageleft::q;

        super::paxos_log_bench(
            1000,
            CorePaxos {
                proposers: proposers.clone(),
                acceptors: acceptors.clone(),
                paxos_config: PaxosConfig {
                    f: 1,
                    i_am_leader_send_timeout: 5,
                    i_am_leader_check_timeout: 10,
                    i_am_leader_check_timeout_delay_multiplier: 15,
                },
            },
            clients,
            clients.singleton(q!(100usize)),
            client_aggregator,
            100,
            1000,
            pretty_print_bench_results,
        );
    }

    #[tokio::test]
    async fn paxos_log_some_throughput() {
        let mut builder = hydro_lang::compile::builder::FlowBuilder::new();
        let proposers = builder.cluster();
        let acceptors = builder.cluster();
        let clients = builder.cluster();
        let client_aggregator = builder.process();

        create_paxos(&proposers, &acceptors, &clients, &client_aggregator);
        let mut deployment = Deployment::new();

        let nodes = builder
            .with_cluster(
                &proposers,
                (0..PAXOS_F + 1).map(|_| TrybuildHost::new(deployment.Localhost())),
            )
            .with_cluster(
                &acceptors,
                (0..2 * PAXOS_F + 1).map(|_| TrybuildHost::new(deployment.Localhost())),
            )
            .with_cluster(&clients, vec![TrybuildHost::new(deployment.Localhost())])
            .with_process(
                &client_aggregator,
                TrybuildHost::new(deployment.Localhost()),
            )
            .deploy(&mut deployment);

        deployment.deploy().await.unwrap();

        let client_node = &nodes.get_process(&client_aggregator);
        let client_out = client_node.stdout_filter("Throughput:");

        deployment.start().await.unwrap();

        use std::str::FromStr;

        use regex::Regex;

        let re = Regex::new(r"Throughput: ([^ ]+) requests/s").unwrap();
        let mut found = 0;
        let mut client_out = client_out;
        while let Some(line) = client_out.recv().await {
            if let Some(caps) = re.captures(&line)
                && let Ok(lower) = f64::from_str(&caps[1])
                && 0.0 < lower
            {
                println!("Found throughput lower-bound: {}", lower);
                found += 1;
                if found == 2 {
                    break;
                }
            }
        }
    }
}
