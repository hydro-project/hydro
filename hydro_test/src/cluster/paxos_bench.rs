use hydro_lang::*;
use hydro_std::bench_client::{bench_client, print_bench_results};
use hydro_std::quorum::collect_quorum;

use super::kv_replica::{KvPayload, Replica, kv_replica};
use super::paxos_with_client::PaxosLike;

pub struct Client;
pub struct Aggregator;

pub fn paxos_bench<'a>(
    num_clients_per_node: usize,
    num_client_machines: usize,
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
            value: (CLUSTER_SELF_ID, value)
        }));

        let acceptors = paxos.log_stores().clone();
        let (acceptor_checkpoint_complete, acceptor_checkpoint) =
            acceptors.forward_ref::<Optional<_, _, _>>();

        let sequenced_payloads = unsafe {
            // SAFETY: clients "own" certain keys, so interleaving elements from clients will not affect
            // the order of writes to the same key

            // TODO(shadaj): we should retry when a payload is dropped due to stale leader
            paxos.with_client(clients, payloads, acceptor_checkpoint)
        };

        let sequenced_to_replicas = sequenced_payloads.broadcast_bincode_anonymous(replicas);

        // Replicas
        let (replica_checkpoint, processed_payloads) =
            kv_replica(replicas, sequenced_to_replicas, checkpoint_frequency);

        // Get the latest checkpoint sequence per replica
        let checkpoint_tick = acceptors.tick();
        let a_checkpoint = unsafe {
            // SAFETY: even though we batch the checkpoint messages, because we reduce over the entire history,
            // the final min checkpoint is deterministic
            // TODO(shadaj): once we can reduce keyed over unbounded streams, this should be safe

            let a_checkpoint_largest_seqs = replica_checkpoint
                .broadcast_bincode(&acceptors)
                .tick_batch(&checkpoint_tick)
                .persist()
                .reduce_keyed_commutative(q!(|curr_seq, seq| {
                    if seq > *curr_seq {
                        *curr_seq = seq;
                    }
                }));

            let a_checkpoints_quorum_reached = a_checkpoint_largest_seqs
                .clone()
                .count()
                .filter_map(q!(move |num_received| if num_received == f + 1 {
                    Some(true)
                } else {
                    None
                }));

            // Find the smallest checkpoint seq that everyone agrees to
            a_checkpoint_largest_seqs
                .continue_if(a_checkpoints_quorum_reached)
                .map(q!(|(_sender, seq)| seq))
                .min()
                .latest()
        };

        acceptor_checkpoint_complete.complete(a_checkpoint);

        let c_received_payloads = processed_payloads
            .map(q!(|payload| (
                payload.value.0,
                ((payload.key, payload.value.1), Ok(()))
            )))
            .send_bincode_anonymous(clients);

        // we only mark a transaction as committed when all replicas have applied it
        collect_quorum::<_, _, _, ()>(
            c_received_payloads.atomic(&clients.tick()),
            f + 1,
            num_replicas,
        )
        .0
        .end_atomic()
    };

    let bench_results = unsafe { bench_client(clients, paxos_processor, num_clients_per_node) };

    print_bench_results(bench_results, client_aggregator, num_client_machines);
}

#[cfg(test)]
mod tests {
    use dfir_lang::graph::WriteConfig;
    use hydro_lang::deploy::DeployRuntime;

    use crate::cluster::paxos::{CorePaxos, PaxosConfig};

    #[test]
    fn paxos_ir() {
        let builder = hydro_lang::FlowBuilder::new();
        let proposers = builder.cluster();
        let acceptors = builder.cluster();
        let clients = builder.cluster();
        let client_aggregator = builder.process();
        let replicas = builder.cluster();

        super::paxos_bench(
            1,
            1,
            1,
            1,
            2,
            CorePaxos {
                proposers: proposers.clone(),
                acceptors: acceptors.clone(),
                paxos_config: PaxosConfig {
                    f: 1,
                    i_am_leader_send_timeout: 1,
                    i_am_leader_check_timeout: 1,
                    i_am_leader_check_timeout_delay_multiplier: 1,
                },
            },
            &clients,
            &client_aggregator,
            &replicas,
        );
        let built = builder.with_default_optimize::<DeployRuntime>();

        hydro_lang::ir::dbg_dedup_tee(|| {
            insta::assert_debug_snapshot!(built.into_ir());
        });

        let preview = built.preview_compile();
        insta::with_settings!({snapshot_suffix => "proposer_mermaid"}, {
            insta::assert_snapshot!(
                preview.dfir_for(&proposers).to_mermaid(&WriteConfig {
                    no_subgraphs: true,
                    no_varnames: false,
                    no_pull_push: true,
                    no_handoffs: true,
                    no_references: false,
                    op_short_text: false,
                    op_text_no_imports: true,
                })
            );
        });

        let _ = built.compile(&RuntimeData::new("FAKE"));
    }
}
