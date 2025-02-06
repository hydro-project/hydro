use hydro_lang::*;

use super::compartmentalized_paxos::{
    compartmentalized_paxos_core, CompartmentalizedPaxosConfig, ProxyLeader,
};
use super::paxos::{Acceptor, Ballot, PaxosPayload, Proposer};

/// Wraps the core Paxos algorithm with logic to send payloads from clients to the current
/// leader.
///
/// # Safety
/// Clients may send payloads to a stale leader if the leader changes between the time the
/// payload is sent and the time it is processed. This will result in the payload being dropped.
/// Payloads sent from multiple clients may be interleaved in a non-deterministic order.
pub unsafe fn compartmentalized_paxos_with_client<'a, C: 'a, R, P: PaxosPayload>(
    proposers: &Cluster<'a, Proposer>,
    proxy_leaders: &Cluster<'a, ProxyLeader>,
    acceptors: &Cluster<'a, Acceptor>,
    clients: &Cluster<'a, C>,
    payloads: Stream<P, Cluster<'a, C>, Unbounded>,
    replica_checkpoint: Stream<(ClusterId<R>, usize), Cluster<'a, Acceptor>, Unbounded, NoOrder>,
    paxos_config: CompartmentalizedPaxosConfig,
) -> Stream<(usize, Option<P>), Cluster<'a, ProxyLeader>, Unbounded, NoOrder> {
    unsafe {
        // SAFETY: Non-deterministic leader notifications are handled in `cur_leader_id`. We do not
        // care about the order in which key writes are processed, which is the non-determinism in
        // `sequenced_payloads`.

        compartmentalized_paxos_core(
            proposers,
            proxy_leaders,
            acceptors,
            replica_checkpoint,
            |new_leader_elected| {
                let cur_leader_id = new_leader_elected
                    .broadcast_bincode_interleaved(clients)
                    .inspect(q!(|ballot| println!(
                        "Client notified that leader was elected: {:?}",
                        ballot
                    )))
                    .max()
                    .map(q!(|ballot: Ballot| ballot.proposer_id));

                let payloads_at_proposer = {
                    // SAFETY: the risk here is that we send a batch of requests
                    // with a stale leader ID, but because the leader ID comes from the
                    // network there is no way to guarantee that it is up to date. This
                    // is documented non-determinism.

                    let client_tick = clients.tick();
                    let payload_batch = payloads.timestamped(&client_tick).tick_batch();

                    let latest_leader = cur_leader_id.timestamped(&client_tick).latest_tick();

                    let (unsent_payloads_complete, unsent_payloads) =
                        client_tick.cycle::<Stream<_, _, _, TotalOrder>>();

                    let all_payloads = unsent_payloads.chain(payload_batch);

                    unsent_payloads_complete.complete_next_tick(
                        all_payloads.clone().continue_unless(latest_leader.clone()),
                    );

                    all_payloads.cross_singleton(latest_leader).all_ticks()
                }
                // .inspect(q!(|(payload, leader_id)| println!(
                //     "Sending payload {:?} to leader {:?}",
                //     payload, leader_id
                // )))
                .map(q!(move |(payload, leader_id)| (leader_id, payload)))
                .send_bincode_interleaved(proposers);

                let payloads_at_proposer = {
                    // SAFETY: documented non-determinism in interleaving of client payloads
                    payloads_at_proposer.assume_ordering()
                };

                payloads_at_proposer
            },
            paxos_config,
        )
        .1
    }
}
