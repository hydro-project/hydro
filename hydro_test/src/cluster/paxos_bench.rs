use hydro_lang::live_collections::stream::NoOrder;
use hydro_lang::location::cluster::CLUSTER_SELF_ID;
use hydro_lang::prelude::*;
use hydro_std::bench_client::{bench_client, print_bench_results};
use hydro_std::quorum::collect_quorum;

use super::kv_replica::{KvPayload, Replica, kv_replica};
use super::paxos_with_client::PaxosLike;

pub struct Client;
pub struct Aggregator;

#[expect(clippy::too_many_arguments, reason = "internal paxos code // TODO")]
pub fn paxos_bench<'a>(
    num_clients_per_node: usize,
    checkpoint_frequency: usize, // How many sequence numbers to commit before checkpointing
    f: usize, /* Maximum number of faulty nodes. A payload has been processed once f+1 replicas have processed it. */
    num_replicas: usize,
    paxos: impl PaxosLike<'a>,
    clients: &Cluster<'a, Client>,
    client_aggregator: &Process<'a, Aggregator>,
    replicas: &Cluster<'a, Replica>,
) {
    let paxos_processor = |c_to_proposers: Stream<(u32, u32), Cluster<'a, Client>, Unbounded>| {
        let payloads = c_to_proposers.map(q!(move |(key, value)| KvPayload {
            key,
            // we use our ID as part of the value and use that so the replica only notifies us
            value: (CLUSTER_SELF_ID.clone(), value)
        }));

        let acceptors = paxos.log_stores().clone();
        let (acceptor_checkpoint_complete, acceptor_checkpoint) =
            acceptors.forward_ref::<Optional<_, _, _>>();

        let sequenced_payloads = paxos.with_client(
            clients,
            payloads,
            acceptor_checkpoint,
            // TODO(shadaj): we should retry when a payload is dropped due to stale leader
            nondet!(/** benchmarking, assuming no re-election */),
            nondet!(
                /// clients 'own' certain keys, so interleaving elements from clients will not affect
                /// the order of writes to the same key
            ),
        );

        let sequenced_to_replicas = sequenced_payloads
            .broadcast(replicas, TCP.bincode(), nondet!(/** TODO */))
            .values();

        // Replicas
        let (replica_checkpoint, processed_payloads) =
            kv_replica(replicas, sequenced_to_replicas, checkpoint_frequency);

        // Get the latest checkpoint sequence per replica
        let a_checkpoint = {
            let a_checkpoint_largest_seqs = replica_checkpoint
                .broadcast(&acceptors, TCP.bincode(), nondet!(/** TODO */))
                .reduce_commutative(q!(|curr_seq, seq| {
                    if seq > *curr_seq {
                        *curr_seq = seq;
                    }
                }));

            sliced! {
                let snapshot = use(a_checkpoint_largest_seqs, nondet!(
                    /// even though we batch the checkpoint messages, because we reduce over the entire history,
                    /// the final min checkpoint is deterministic
                ));

                let a_checkpoints_quorum_reached = snapshot
                    .clone()
                    .key_count()
                    .filter_map(q!(move |num_received| if num_received == f + 1 {
                        Some(true)
                    } else {
                        None
                    }));

                // Find the smallest checkpoint seq that everyone agrees to
                snapshot
                    .entries()
                    .filter_if_some(a_checkpoints_quorum_reached)
                    .map(q!(|(_sender, seq)| seq))
                    .min()
            }
        };

        acceptor_checkpoint_complete.complete(a_checkpoint);

        let c_received_payloads = processed_payloads
            .map(q!(|payload| (
                payload.value.0,
                ((payload.key, payload.value.1), Ok(()))
            )))
            .demux(clients, TCP.bincode())
            .values();

        // we only mark a transaction as committed when all replicas have applied it
        collect_quorum::<_, _, _, ()>(c_received_payloads, f + 1, num_replicas).0
    };

    let bench_results = bench_client(
        clients,
        inc_u32_workload_generator,
        paxos_processor,
        num_clients_per_node,
        nondet!(/** bench */),
    );

    print_bench_results(bench_results, client_aggregator, clients);
}

/// Generates an incrementing u32 for each virtual client ID, starting at 0
pub fn inc_u32_workload_generator<'a, Client>(
    _client: &Cluster<'a, Client>,
    payload_request: Stream<(u32, Option<u32>), Cluster<'a, Client>, Unbounded, NoOrder>,
) -> Stream<(u32, u32), Cluster<'a, Client>, Unbounded, NoOrder> {
    payload_request.map(q!(move |(virtual_id, payload)| {
        let value = if let Some(payload) = payload {
            payload + 1
        } else {
            0
        };
        (virtual_id, value)
    }))
}

#[cfg(test)]
mod tests {
    use dfir_lang::graph::WriteConfig;
    use hydro_deploy::Deployment;
    use hydro_lang::deploy::{DeployCrateWrapper, HydroDeploy, TrybuildHost};

    use crate::cluster::paxos::{CorePaxos, PaxosConfig};

    const PAXOS_F: usize = 1;

    #[cfg(stageleft_runtime)]
    fn create_paxos<'a>(
        proposers: &hydro_lang::location::Cluster<'a, crate::cluster::paxos::Proposer>,
        acceptors: &hydro_lang::location::Cluster<'a, crate::cluster::paxos::Acceptor>,
        clients: &hydro_lang::location::Cluster<'a, super::Client>,
        client_aggregator: &hydro_lang::location::Process<'a, super::Aggregator>,
        replicas: &hydro_lang::location::Cluster<'a, crate::cluster::kv_replica::Replica>,
    ) {
        super::paxos_bench(
            100,
            1000,
            PAXOS_F,
            PAXOS_F + 1,
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
            client_aggregator,
            replicas,
        );
    }

    #[test]
    fn paxos_ir() {
        let builder = hydro_lang::compile::builder::FlowBuilder::new();
        let proposers = builder.cluster();
        let acceptors = builder.cluster();
        let clients = builder.cluster();
        let client_aggregator = builder.process();
        let replicas = builder.cluster();

        create_paxos(
            &proposers,
            &acceptors,
            &clients,
            &client_aggregator,
            &replicas,
        );
        let mut built = builder.with_default_optimize::<HydroDeploy>();

        hydro_lang::compile::ir::dbg_dedup_tee(|| {
            hydro_build_utils::assert_debug_snapshot!(built.ir());
        });

        let preview = built.preview_compile();
        hydro_build_utils::insta::with_settings!({
            snapshot_suffix => "proposer_mermaid"
        }, {
            hydro_build_utils::assert_snapshot!(
                preview.dfir_for(&proposers).to_mermaid(&WriteConfig {
                    no_subgraphs: true,
                    no_pull_push: true,
                    no_handoffs: true,
                    op_text_no_imports: true,
                    ..WriteConfig::default()
                })
            );
        });
        hydro_build_utils::insta::with_settings!({
            snapshot_suffix => "acceptor_mermaid"
        }, {
            hydro_build_utils::assert_snapshot!(
                preview.dfir_for(&acceptors).to_mermaid(&WriteConfig {
                    no_subgraphs: true,
                    no_pull_push: true,
                    no_handoffs: true,
                    op_text_no_imports: true,
                    ..WriteConfig::default()
                })
            );
        });
    }

    #[tokio::test]
    async fn paxos_some_throughput() {
        let builder = hydro_lang::compile::builder::FlowBuilder::new();
        let proposers = builder.cluster();
        let acceptors = builder.cluster();
        let clients = builder.cluster();
        let client_aggregator = builder.process();
        let replicas = builder.cluster();

        create_paxos(
            &proposers,
            &acceptors,
            &clients,
            &client_aggregator,
            &replicas,
        );
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
            .with_cluster(
                &replicas,
                (0..PAXOS_F + 1).map(|_| TrybuildHost::new(deployment.Localhost())),
            )
            .deploy(&mut deployment);

        deployment.deploy().await.unwrap();

        let client_node = &nodes.get_process(&client_aggregator);
        let client_out = client_node.stdout_filter("Throughput:");

        deployment.start().await.unwrap();

        use std::str::FromStr;

        use regex::Regex;

        let re = Regex::new(r"Throughput: ([^ ]+) - ([^ ]+) - ([^ ]+) requests/s").unwrap();
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
