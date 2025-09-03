use std::fmt::Debug;

use hydro_lang::location::MemberId;
use hydro_lang::*;
use serde::Serialize;
use serde::de::DeserializeOwned;

use super::paxos::PaxosPayload;

pub trait PaxosLike<'a>: Sized {
    /// The nodes that receive inputs in Paxos. Usually the proposers.
    type PaxosIn: 'a;

    /// The nodes that store logs in Paxos. Usually the acceptors.
    type PaxosLog: 'a;

    /// The nodes that output the results of Paxos. Proposers in Paxos, Proxy leaders in Compartmentalized Paxos.
    type PaxosOut: 'a;
    type Ballot: Clone + Ord + Debug + Serialize + DeserializeOwned;

    fn payload_recipients(&self) -> &Cluster<'a, Self::PaxosIn>;
    fn log_stores(&self) -> &Cluster<'a, Self::PaxosLog>;

    fn get_recipient_from_ballot<L: Location<'a>>(
        ballot: Optional<Self::Ballot, L, Unbounded>,
    ) -> Optional<MemberId<Self::PaxosIn>, L, Unbounded>;

    /// # Non-Determinism
    /// During leader-reelection, the latest known leader may be stale, which may
    /// result in non-deterministic dropping of payloads.
    #[expect(clippy::type_complexity, reason = "internal paxos code // TODO")]
    fn build<P: PaxosPayload>(
        self,
        payload_generator: impl FnOnce(
            Stream<Self::Ballot, Cluster<'a, Self::PaxosIn>, Unbounded>,
        ) -> Stream<P, Cluster<'a, Self::PaxosIn>, Unbounded>,
        checkpoints: Optional<usize, Cluster<'a, Self::PaxosLog>, Unbounded>,
        nondet_leader: NonDet,
        nondet_commit: NonDet,
    ) -> Stream<(usize, Option<P>), Cluster<'a, Self::PaxosOut>, Unbounded, NoOrder>;

    /// # Non-Determinism
    /// During leader-reelection, the latest known leader may be stale, which may
    /// result in non-deterministic dropping of payloads. Also, payloads across
    /// clients will be arbitrarily interleaved as they arrive at the leader.
    #[expect(clippy::type_complexity, reason = "internal paxos code // TODO")]
    fn with_client<C: 'a, P: PaxosPayload>(
        self,
        clients: &Cluster<'a, C>,
        payloads: Stream<P, Cluster<'a, C>, Unbounded>,
        checkpoints: Optional<usize, Cluster<'a, Self::PaxosLog>, Unbounded>,
        nondet_commit: NonDet,
        nondet_order: NonDet,
    ) -> Stream<(usize, Option<P>), Cluster<'a, Self::PaxosOut>, Unbounded, NoOrder> {
        let leaders = self.payload_recipients().clone();

        self.build(
            move |new_leader_elected| {
                let cur_leader_id = Self::get_recipient_from_ballot(
                    new_leader_elected
                        .broadcast_bincode(clients, nondet!(/** TODO */))
                        .values()
                        .inspect(q!(|ballot| println!(
                            "Client notified that leader was elected: {:?}",
                            ballot
                        )))
                        .max(),
                );

                let payloads_at_proposer = {
                    let client_tick = clients.tick();
                    let payload_batch = payloads.batch(&client_tick, nondet!(/** see below */));

                    let latest_leader = cur_leader_id.snapshot(
                        &client_tick,
                        nondet!(
                            /// the risk here is that we send a batch of requests
                            /// with a stale leader ID, but because the leader ID comes from the
                            /// network there is no way to guarantee that it is up to date. This
                            /// is documented non-determinism.
                            nondet_commit
                        ),
                    );

                    let (unsent_payloads_complete, unsent_payloads) =
                        client_tick.cycle::<Stream<_, _, _, TotalOrder>>();

                    let all_payloads = unsent_payloads.chain(payload_batch);

                    unsent_payloads_complete.complete_next_tick(
                        all_payloads.clone().continue_unless(latest_leader.clone()),
                    );

                    all_payloads.cross_singleton(latest_leader).all_ticks()
                }
                .map(q!(move |(payload, leader_id)| (leader_id, payload)))
                .demux_bincode(&leaders)
                .values();

                let payloads_at_proposer = {
                    payloads_at_proposer.assume_ordering(nondet!(
                        /// documented non-determinism in interleaving of client payloads
                        nondet_order
                    ))
                };

                payloads_at_proposer
            },
            checkpoints,
            nondet!(
                /// non-deterministic leader changes may lead to sending to a stale leader, which will drop payloads
                nondet_commit
            ),
            nondet_commit,
        )
    }
}
