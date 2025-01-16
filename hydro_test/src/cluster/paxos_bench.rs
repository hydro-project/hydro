use hydro_lang::*;
use hydro_std::quorum::collect_quorum;

use super::bench_client::{bench_client, Client};
use super::kv_replica::{kv_replica, KvPayload, Replica};
use super::paxos::{Acceptor, Proposer};
use super::paxos_with_client::paxos_with_client;

#[expect(clippy::too_many_arguments, reason = "internal paxos code // TODO")]
pub fn paxos_bench<'a>(
    flow: &FlowBuilder<'a>,
    f: usize,
    num_clients_per_node: usize,
    median_latency_window_size: usize, /* How many latencies to keep in the window for calculating the median */
    checkpoint_frequency: usize,       // How many sequence numbers to commit before checkpointing
    i_am_leader_send_timeout: u64,     // How often to heartbeat
    i_am_leader_check_timeout: u64,    // How often to check if heartbeat expired
    i_am_leader_check_timeout_delay_multiplier: usize, /* Initial delay, multiplied by proposer pid, to stagger proposers checking for timeouts */
) -> (
    Process<'a, ()>,
    Cluster<'a, Proposer>,
    Cluster<'a, Acceptor>,
    Cluster<'a, Client>,
    Cluster<'a, Replica>,
) {
    let sequencer = flow.process::<()>();
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

            // Sequencing
            // for a non-fault-tolerant version, we could use this instead of Paxos
            // let sequenced_to_replicas = unsafe {
            //     payloads
            //         .send_bincode_interleaved(&sequencer)
            //         .assume_ordering()
            // }
            // .map(q!(|kv| Some(kv)))
            // .enumerate()
            // .broadcast_bincode(&replicas);

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
                    f,
                    i_am_leader_send_timeout,
                    i_am_leader_check_timeout,
                    i_am_leader_check_timeout_delay_multiplier,
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
                .send_bincode_interleaved(&clients);

            // we only mark a transaction as committed when all replicas have applied it
            collect_quorum::<_, _, _, ()>(
                c_received_payloads.timestamped(&clients.tick()),
                f + 1,
                f + 1,
            )
            .0
            .drop_timestamp()
        },
        num_clients_per_node,
        median_latency_window_size,
    );

    (sequencer, proposers, acceptors, clients, replicas)
}

#[cfg(test)]
mod tests {
    use hydro_lang::deploy::DeployRuntime;
    use stageleft::RuntimeData;

    #[test]
    fn paxos_ir() {
        let builder = hydro_lang::FlowBuilder::new();
        let _ = super::paxos_bench(&builder, 1, 1, 1, 1, 1, 1, 1);
        let built = builder.with_default_optimize::<DeployRuntime>();

        hydro_lang::ir::dbg_dedup_tee(|| {
            insta::assert_debug_snapshot!(built.ir());
        });

        let _ = built.compile(&RuntimeData::new("FAKE"));
    }
}
