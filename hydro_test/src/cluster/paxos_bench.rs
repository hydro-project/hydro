use hydro_lang::*;
use hydro_std::quorum::collect_quorum;

use super::bench_client::{Client, bench_client};
use super::kv_replica::{KvPayload, Replica, kv_replica};
use super::paxos_with_client::PaxosLike;

pub fn paxos_bench<'a, Paxos: PaxosLike<'a>>(
    flow: &FlowBuilder<'a>,
    num_clients_per_node: usize,
    median_latency_window_size: usize, /* How many latencies to keep in the window for calculating the median */
    checkpoint_frequency: usize,       // How many sequence numbers to commit before checkpointing
    f: usize, /* Maximum number of faulty nodes. A payload has been processed once f+1 replicas have processed it. */
    num_replicas: usize,
    create_paxos: impl FnOnce(Stream<usize, Cluster<'a, Replica>, Unbounded>) -> Paxos,
) -> (Cluster<'a, Client>, Cluster<'a, Replica>) {
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

            let paxos = create_paxos(replica_checkpoint);

            let sequenced_payloads = unsafe {
                // SAFETY: clients "own" certain keys, so interleaving elements from clients will not affect
                // the order of writes to the same key

                // TODO(shadaj): we should retry when a payload is dropped due to stale leader
                paxos.with_client(&clients, payloads)
            };

            let sequenced_to_replicas = sequenced_payloads.broadcast_bincode_anonymous(&replicas);

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
                f + 1,
                num_replicas,
            )
            .0
            .end_atomic()
        },
        num_clients_per_node,
        median_latency_window_size,
    );

    (clients, replicas)
}

#[cfg(test)]
mod tests {
    use dfir_rs::lang::graph::WriteConfig;
    use hydro_lang::deploy::DeployRuntime;
    use stageleft::RuntimeData;

    use crate::cluster::paxos::{CorePaxos, PaxosConfig};

    #[test]
    fn paxos_ir() {
        let builder = hydro_lang::FlowBuilder::new();
        let proposers = builder.cluster();
        let acceptors = builder.cluster();

        let _ = super::paxos_bench(&builder, 1, 1, 1, 1, 2, |replica_checkpoint| CorePaxos {
            proposers: proposers.clone(),
            acceptors: acceptors.clone(),
            replica_checkpoint: replica_checkpoint.broadcast_bincode(&acceptors),
            paxos_config: PaxosConfig {
                f: 1,
                i_am_leader_send_timeout: 1,
                i_am_leader_check_timeout: 1,
                i_am_leader_check_timeout_delay_multiplier: 1,
            },
        });
        let built = builder.with_default_optimize::<DeployRuntime>();

        hydro_lang::ir::dbg_dedup_tee(|| {
            insta::assert_debug_snapshot!(built.ir());
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
