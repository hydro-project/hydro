use hydro_lang::*;
use hydro_std::quorum::collect_quorum;

use super::bench_client::{bench_client, Client};
use super::kv_replica::{kv_replica, KvPayload, Replica};
use super::paxos::{Acceptor, PaxosConfig, Proposer};
use super::paxos_with_client::paxos_with_client;

pub fn paxos_bench<'a>(
    flow: &FlowBuilder<'a>,
    num_clients_per_node: usize,
    median_latency_window_size: usize, /* How many latencies to keep in the window for calculating the median */
    checkpoint_frequency: usize,       // How many sequence numbers to commit before checkpointing
    paxos_config: PaxosConfig,
) -> (
    Cluster<'a, Proposer>,
    Cluster<'a, Acceptor>,
    Cluster<'a, Client>,
    Cluster<'a, Replica>,
) {
    let proposers = flow.cluster::<Proposer>();
    let acceptors = flow.cluster::<Acceptor>();
    let clients = flow.cluster::<Client>();
    let replicas = flow.cluster::<Replica>();

    bench_client(
        &clients,
        |c_to_proposers| {
            let payloads = c_to_proposers.map(q!(move |(key, value)| KvPayload {
                key,
                // we use our ID as part of the value and use that so the replica only notifies us
                value: (CLUSTER_SELF_ID, value)
            }));

            let (replica_checkpoint_complete, replica_checkpoint) =
                replicas.forward_ref::<Stream<_, _, _>>();

            let sequenced_payloads = unsafe {
                // SAFETY: clients "own" certain keys, so interleaving elements from clients will not affect
                // the order of writes to the same key

                // TODO(shadaj): we should retry when a payload is dropped due to stale leader
                paxos_with_client(
                    &proposers,
                    &acceptors,
                    &clients,
                    payloads,
                    replica_checkpoint.broadcast_bincode(&acceptors),
                    paxos_config,
                )
            };

            let sequenced_to_replicas = sequenced_payloads.broadcast_bincode_interleaved(&replicas);

            // Replicas
            let (replica_checkpoint, processed_payloads) =
                kv_replica(&replicas, sequenced_to_replicas, checkpoint_frequency);

            replica_checkpoint_complete.complete(replica_checkpoint);

            let c_received_payloads = processed_payloads
                .map(q!(|payload| (
                    payload.value.0,
                    ((payload.key, payload.value.1), Ok(()))
                )))
                .send_bincode_anonymous(&clients);

            // we only mark a transaction as committed when all replicas have applied it
            collect_quorum::<_, _, _, ()>(
                c_received_payloads.atomic(&clients.tick()),
                paxos_config.f + 1,
                paxos_config.f + 1,
            )
            .0
            .end_atomic()
        },
        num_clients_per_node,
        median_latency_window_size,
    );

    (proposers, acceptors, clients, replicas)
}

#[cfg(test)]
mod tests {
    use hydro_lang::deploy::DeployRuntime;
    use stageleft::RuntimeData;

    use crate::cluster::paxos::PaxosConfig;

    #[test]
    fn paxos_ir() {
        let builder = hydro_lang::FlowBuilder::new();
        let _ = super::paxos_bench(
            &builder,
            1,
            1,
            1,
            PaxosConfig {
                f: 1,
                i_am_leader_send_timeout: 1,
                i_am_leader_check_timeout: 1,
                i_am_leader_check_timeout_delay_multiplier: 1,
            },
        );
        let built = builder.with_default_optimize::<DeployRuntime>();

        hydro_lang::ir::dbg_dedup_tee(|| {
            insta::assert_debug_snapshot!(built.ir());
        });

        let _ = built.compile(&RuntimeData::new("FAKE"));
    }
}
