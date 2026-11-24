use hydro_lang::live_collections::stream::{ExactlyOnce, NoOrder, TotalOrder};
use hydro_lang::location::cluster::CLUSTER_SELF_ID;
use hydro_lang::location::{Location, MemberId};
use hydro_lang::prelude::*;
use hydro_std::bench_client::{
    BenchResult, aggregate_bench_results, bench_client, compute_throughput_latency,
};
use hydro_std::quorum::collect_quorum;

use super::paxos_bench::inc_i32_workload_generator;
use super::raft::{RaftConfig, Replica, raft_server};

pub struct Client;
pub struct Aggregator;

/// A client request as it travels through the Raft log: the client that issued it, the
/// virtual client within that machine, and the payload.
type Request = (MemberId<Client>, u32, i32);

/// Benchmarks [`raft_server`] with closed-loop [`bench_client`]s.
///
/// Clients send each request to the leader that a majority of replicas name in their
/// latest leader views, holding requests while there is none. Every member's election
/// timer fires at startup, so a member can win an early term that the others skip, and
/// a leader can be deposed while some replicas still name it. Requests sent to such a
/// leader are lost, and `bench_client` never retries, so their virtual clients stall for
/// the rest of the run. Clients therefore use a leader only while a majority's latest
/// views name it, which rules out both; requests already in flight to a leader when it is
/// deposed are still lost.
///
/// A request completes when the first replica commits it, which is the leader, since
/// followers learn the commit index from a later `AppendEntries`. A replica that receives
/// a request while not leading forwards it to its leader hint, and drops it when it has
/// none.
///
/// * `election_timeout_millis`: member `i` uses `election_timeout_millis + 130 * i`, so
///   that members' election timers do not stay aligned.
/// * `heartbeat_interval_millis`: the leader only sends `AppendEntries` on a heartbeat, so
///   this interval bounds commit latency from below.
#[expect(clippy::too_many_arguments, reason = "benchmark configuration")]
pub fn raft_bench<'a>(
    replicas: &Cluster<'a, Replica>,
    config: RaftConfig,
    election_timeout_millis: u64,
    heartbeat_interval_millis: u64,
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
            let (leader_views_complete, leader_views) = clients.forward_ref::<Stream<
                (MemberId<Replica>, (usize, Option<MemberId<Replica>>)),
                Cluster<'a, Client>,
                Unbounded,
                NoOrder,
            >>();
            let majority = config.cluster_size / 2 + 1;
            let latest_leader = leader_views
                .fold(
                    q!(|| std::collections::BTreeMap::new()),
                    q!(
                        |latest, (reporter, view)| {
                            let current = latest.entry(reporter).or_insert_with(|| view.clone());
                            if view > *current {
                                *current = view;
                            }
                        },
                        commutative = manual_proof!(
                            /** A replica's views increase in `(term, leader)` order: its
                            term never decreases, and within a term it learns at most one
                            leader (`None < Some`). The per-replica maximum is therefore
                            its latest view whatever order the views arrive in. */
                        )
                    ),
                )
                .filter_map(q!(move |latest| {
                    let mut followers = std::collections::BTreeMap::new();
                    for (term, leader) in latest.values() {
                        if let Some(leader) = leader {
                            *followers.entry((*term, leader.clone())).or_insert(0usize) += 1;
                        }
                    }
                    followers
                        .into_iter()
                        .find(|(_, count)| *count >= majority)
                        .map(|((_term, leader), _)| leader)
                }));

            let requests_at_replicas = sliced! {
                let mut unsent = use::state_null::<Stream<_, _, _, TotalOrder>>();
                let new_requests = use::batch(
                    input.entries().assume_ordering::<TotalOrder>(nondet!(
                        /// virtual clients are independent, so the order in which a
                        /// client machine sends their requests is arbitrary
                    )),
                    nondet!(/** batching only groups requests sent to the same leader */),
                );
                let leader = use::snapshot(latest_leader, nondet!(
                    /// the leader ID comes from the network and may be stale; a stale
                    /// leader forwards the request to its own leader hint
                ));

                let all_requests = unsent.chain(new_requests);
                unsent = all_requests.clone().filter_if(leader.clone().is_none());
                all_requests.cross_singleton(leader)
            }
            .map(q!(move |((virtual_id, payload), leader)| (
                leader,
                (CLUSTER_SELF_ID.clone(), virtual_id, payload)
            )))
            .demux(replicas, TCP.fail_stop().bincode())
            .values();

            let (forwarded_complete, forwarded) =
                replicas.forward_ref::<Stream<Request, Cluster<'a, Replica>, Unbounded, NoOrder>>();
            let election_timer_interrupts = replicas
                .source_interval(q!(std::time::Duration::from_millis(
                    election_timeout_millis + u64::from(CLUSTER_SELF_ID.get_raw_id()) * 130
                )))
                .assume_retries::<ExactlyOnce>(nondet!(
                    /// a repeated election timer fire is just another fire
                ));
            let heartbeat_timer_interrupts = replicas
                .source_interval(q!(std::time::Duration::from_millis(
                    heartbeat_interval_millis
                )))
                .assume_retries::<ExactlyOnce>(nondet!(
                    /// a repeated heartbeat timer fire is just another fire
                ));

            let outputs = raft_server(
                replicas,
                requests_at_replicas
                    .weaken_ordering::<NoOrder>()
                    .merge_unordered(forwarded),
                election_timer_interrupts,
                heartbeat_timer_interrupts,
                config,
                TCP.fail_stop().bincode(),
                nondet!(/** benchmarking: the committed order of requests is irrelevant */),
            );

            forwarded_complete.complete(
                outputs
                    .redirected
                    .filter_map(q!(|(request, hint)| hint.map(|leader| (leader, request))))
                    .demux(replicas, TCP.fail_stop().bincode())
                    .values()
                    .weaken_ordering::<NoOrder>(),
            );

            leader_views_complete.complete(
                outputs
                    .leader_views
                    .map(q!(|view| (view.term, view.leader)))
                    .broadcast(
                        clients,
                        TCP.fail_stop().bincode(),
                        nondet!(
                            /// clients only need some recent leader
                        ),
                    )
                    .entries()
                    .weaken_ordering::<NoOrder>(),
            );

            let responses = outputs
                .committed
                .end_atomic()
                .weaken_consistency()
                .map(q!(|entry| {
                    let (client, virtual_id, payload) = entry.message;
                    (client, ((virtual_id, payload), Ok::<(), ()>(())))
                }))
                .demux(clients, TCP.fail_stop().bincode())
                .values();

            // Every replica commits every entry; the first to report it is the leader.
            collect_quorum(responses, 1, config.cluster_size)
                .0
                .into_keyed()
        },
    )
    .entries()
    .map(q!(|(_virtual_client_id, (_output, latency))| latency));

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
    use crate::cluster::bench_test_util;

    const CLUSTER_SIZE: usize = 3;

    #[cfg(stageleft_runtime)]
    fn create_raft_bench<'a>(
        replicas: &hydro_lang::location::Cluster<'a, crate::cluster::raft::Replica>,
        clients: &hydro_lang::location::Cluster<'a, super::Client>,
        client_aggregator: &hydro_lang::location::Process<'a, super::Aggregator>,
    ) {
        use hydro_lang::location::Location;
        use hydro_std::bench_client::pretty_print_bench_results;
        use stageleft::q;

        super::raft_bench(
            replicas,
            crate::cluster::raft::RaftConfig {
                cluster_size: CLUSTER_SIZE,
            },
            500,
            10,
            clients,
            clients.singleton(q!(100usize)),
            client_aggregator,
            100,
            1000,
            pretty_print_bench_results,
        );
    }

    #[tokio::test]
    async fn raft_some_throughput() {
        let mut builder = hydro_lang::compile::builder::FlowBuilder::new();
        let replicas = builder.cluster();
        let clients = builder.cluster();
        let client_aggregator = builder.process();

        create_raft_bench(&replicas, &clients, &client_aggregator);
        let mut deployment = Deployment::new();

        let nodes = builder
            .with_cluster(
                &replicas,
                (0..CLUSTER_SIZE).map(|_| TrybuildHost::new(deployment.Localhost())),
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

        bench_test_util::assert_positive_throughput(
            client_out,
            2,
            bench_test_util::FIRST_THROUGHPUT_TIMEOUT,
        )
        .await;
    }
}
