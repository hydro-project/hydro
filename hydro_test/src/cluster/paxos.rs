use std::collections::HashMap;
use std::fmt::Debug;
use std::hash::Hash;
use std::time::Duration;

use hydro_lang::live_collections::stream::{AtLeastOnce, NoOrder, TotalOrder};
use hydro_lang::location::cluster::CLUSTER_SELF_ID;
use hydro_lang::location::{Atomic, Location, MemberId};
use hydro_lang::prelude::*;
use hydro_std::quorum::{collect_quorum, collect_quorum_with_response};
use hydro_std::request_response::join_responses;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};

use super::paxos_with_client::PaxosLike;

#[derive(Serialize, Deserialize, Clone)]
pub struct Proposer {}
pub struct Acceptor {}

#[derive(Clone, Copy)]
pub struct PaxosConfig {
    /// Maximum number of faulty nodes
    pub f: usize,
    /// How often to send "I am leader" heartbeats
    pub i_am_leader_send_timeout: u64,
    /// How often to check if the leader has expired
    pub i_am_leader_check_timeout: u64,
    /// Initial delay, multiplied by proposer pid, to stagger proposers checking for timeouts
    pub i_am_leader_check_timeout_delay_multiplier: usize,
}

pub trait PaxosPayload: Serialize + DeserializeOwned + PartialEq + Eq + Clone + Debug {}
impl<T: Serialize + DeserializeOwned + PartialEq + Eq + Clone + Debug> PaxosPayload for T {}

#[derive(Serialize, Deserialize, PartialEq, Eq, Clone, Debug, Hash)]
pub struct Ballot {
    pub num: u32,
    pub proposer_id: MemberId<Proposer>,
}

impl Ord for Ballot {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.num
            .cmp(&other.num)
            .then_with(|| self.proposer_id.cmp(&other.proposer_id))
    }
}

impl PartialOrd for Ballot {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct LogValue<P> {
    pub ballot: Ballot,
    pub value: Option<P>, // might be a hole
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Clone, Debug)]
pub struct P2a<P, S> {
    pub sender: MemberId<S>,
    pub ballot: Ballot,
    pub slot: usize,
    pub value: Option<P>, // might be a re-committed hole
}

pub struct CorePaxos<'a> {
    pub proposers: Cluster<'a, Proposer>,
    pub acceptors: Cluster<'a, Acceptor>,
    pub paxos_config: PaxosConfig,
}

impl<'a> PaxosLike<'a> for CorePaxos<'a> {
    type PaxosIn = Proposer;
    type PaxosLog = Acceptor;
    type PaxosOut = Proposer;
    type Ballot = Ballot;

    fn payload_recipients(&self) -> &Cluster<'a, Self::PaxosIn> {
        &self.proposers
    }

    fn log_stores(&self) -> &Cluster<'a, Self::PaxosLog> {
        &self.acceptors
    }

    fn get_recipient_from_ballot<L: Location<'a>>(
        ballot: Optional<Self::Ballot, L, Unbounded>,
    ) -> Optional<MemberId<Self::PaxosIn>, L, Unbounded> {
        ballot.map(q!(|ballot| ballot.proposer_id))
    }

    fn build<P: PaxosPayload>(
        self,
        with_ballot: impl FnOnce(
            Stream<Ballot, Cluster<'a, Self::PaxosIn>, Unbounded>,
        ) -> Stream<P, Cluster<'a, Self::PaxosIn>, Unbounded>,
        a_checkpoint: Optional<usize, Cluster<'a, Acceptor>, Unbounded>,
        nondet_leader: NonDet,
        nondet_commit: NonDet,
    ) -> Stream<(usize, Option<P>), Cluster<'a, Self::PaxosOut>, Unbounded, NoOrder> {
        paxos_core(
            &self.proposers,
            &self.acceptors,
            a_checkpoint,
            with_ballot,
            self.paxos_config,
            nondet_leader,
            nondet_commit,
        )
        .1
    }
}

/// Implements the core Paxos algorithm, which uses a cluster of propsers and acceptors
/// to sequence payloads being sent to the proposers.
///
/// Proposers that currently are the leader will work with acceptors to sequence incoming
/// payloads, but may drop payloads if they are not the lader or lose leadership during
/// the commit process.
///
/// Returns a stream of ballots, where new values are emitted when a new leader is elected,
/// and a stream of sequenced payloads with an index and optional payload (in the case of
/// holes in the log).
///
/// # Non-Determinism
/// When the leader is stable, the algorithm will commit incoming payloads to the leader
/// in deterministic order. However, when the leader is changing, payloads may be
/// non-deterministically dropped. The stream of ballots is also non-deterministic because
/// leaders are elected in a non-deterministic process.
#[expect(clippy::type_complexity, reason = "internal paxos code // TODO")]
pub fn paxos_core<'a, P: PaxosPayload>(
    proposers: &Cluster<'a, Proposer>,
    acceptors: &Cluster<'a, Acceptor>,
    a_checkpoint: Optional<usize, Cluster<'a, Acceptor>, Unbounded>,
    c_to_proposers: impl FnOnce(
        Stream<Ballot, Cluster<'a, Proposer>, Unbounded>,
    ) -> Stream<P, Cluster<'a, Proposer>, Unbounded>,
    config: PaxosConfig,
    nondet_leader: NonDet,
    nondet_commit: NonDet,
) -> (
    Stream<Ballot, Cluster<'a, Proposer>, Unbounded>,
    Stream<(usize, Option<P>), Cluster<'a, Proposer>, Unbounded, NoOrder>,
) {
    let f = config.f;

    proposers
        .source_iter(q!(["Proposers say hello"]))
        .for_each(q!(|s| println!("{}", s)));

    acceptors
        .source_iter(q!(["Acceptors say hello"]))
        .for_each(q!(|s| println!("{}", s)));

    let proposer_tick = proposers.tick();
    let acceptor_tick = acceptors.tick();

    let (sequencing_max_ballot_complete_cycle, sequencing_max_ballot_forward_reference) =
        proposers.forward_ref::<Stream<Ballot, _, _, NoOrder>>();
    let (a_log_complete_cycle, a_log_forward_reference) =
        acceptor_tick.forward_ref::<Singleton<_, _, _>>();

    let (p_ballot, p_is_leader, p_relevant_p1bs, a_max_ballot) = leader_election(
        proposers,
        acceptors,
        &proposer_tick,
        &acceptor_tick,
        f + 1,
        2 * f + 1,
        config,
        sequencing_max_ballot_forward_reference,
        a_log_forward_reference,
        nondet!(
            /// The primary non-determinism exposed by leader election algorithm lies in which leader
            /// is elected, which affects both the ballot at each proposer and the leader flag. But using a stale ballot
            /// or leader flag will only lead to failure in sequencing rather than commiting the wrong value.
            nondet_leader
        ),
        nondet!(
            /// Because ballots are non-deterministic, the acceptor max ballot is also non-deterministic, although we are
            /// guaranteed that the max ballot will match the current ballot of a proposer who believes they are the leader.
            nondet_commit
        ),
    );

    let just_became_leader = p_is_leader
        .clone()
        .filter_if_none(p_is_leader.clone().defer_tick());

    let c_to_proposers = c_to_proposers(
        just_became_leader
            .clone()
            .if_some_then(p_ballot.clone())
            .all_ticks(),
    );

    let (p_to_replicas, a_log, sequencing_max_ballots) = sequence_payload(
        proposers,
        acceptors,
        &proposer_tick,
        &acceptor_tick,
        c_to_proposers,
        a_checkpoint,
        p_ballot.clone(),
        p_is_leader,
        p_relevant_p1bs,
        f,
        a_max_ballot,
        nondet!(
            /// The relevant p1bs are non-deterministic because they come from a arbitrary quorum, but because
            /// we use a quorum, if we remain the leader there are no missing committed values when we combine the logs.
            /// The remaining non-determinism is in when incoming payloads are batched versus the leader flag and state
            /// of acceptors, which in the worst case will lead to dropped payloads as documented.
            nondet_commit
        ),
    );

    a_log_complete_cycle.complete(a_log.snapshot_atomic(nondet!(
        /// We will always write payloads to the log before acknowledging them to the proposers,
        /// which guarantees that if the leader changes the quorum overlap between sequencing and leader
        /// election will include the committed value.
    )));
    sequencing_max_ballot_complete_cycle.complete(sequencing_max_ballots);

    (
        // Only tell the clients once when leader election concludes
        just_became_leader.if_some_then(p_ballot).all_ticks(),
        p_to_replicas,
    )
}

#[expect(
    clippy::type_complexity,
    clippy::too_many_arguments,
    reason = "internal paxos code // TODO"
)]
pub fn leader_election<'a, L: Clone + Debug + Serialize + DeserializeOwned>(
    proposers: &Cluster<'a, Proposer>,
    acceptors: &Cluster<'a, Acceptor>,
    proposer_tick: &Tick<Cluster<'a, Proposer>>,
    acceptor_tick: &Tick<Cluster<'a, Acceptor>>,
    quorum_size: usize,
    num_quorum_participants: usize,
    paxos_config: PaxosConfig,
    p_received_p2b_ballots: Stream<Ballot, Cluster<'a, Proposer>, Unbounded, NoOrder>,
    a_log: Singleton<(Option<usize>, L), Tick<Cluster<'a, Acceptor>>, Bounded>,
    nondet_leader: NonDet,
    nondet_acceptor_ballot: NonDet,
) -> (
    Singleton<Ballot, Tick<Cluster<'a, Proposer>>, Bounded>,
    Optional<(), Tick<Cluster<'a, Proposer>>, Bounded>,
    Stream<(Option<usize>, L), Tick<Cluster<'a, Proposer>>, Bounded, NoOrder>,
    Singleton<Ballot, Tick<Cluster<'a, Acceptor>>, Bounded>,
) {
    let (p1b_fail_complete, p1b_fail) =
        proposers.forward_ref::<Stream<Ballot, _, Unbounded, NoOrder>>();
    let (p_to_proposers_i_am_leader_complete_cycle, p_to_proposers_i_am_leader_forward_ref) =
        proposers.forward_ref::<Stream<_, _, _, NoOrder, AtLeastOnce>>();
    let (p_is_leader_complete_cycle, p_is_leader_forward_ref) =
        proposer_tick.forward_ref::<Optional<(), _, _>>();
    // a_to_proposers_p2b.clone().for_each(q!(|(_, p2b): (u32, P2b)| println!("Proposer received P2b: {:?}", p2b)));
    // p_to_proposers_i_am_leader.clone().for_each(q!(|ballot: Ballot| println!("Proposer received I am leader: {:?}", ballot)));
    // c_to_proposers.clone().for_each(q!(|payload: ClientPayload| println!("Client sent proposer payload: {:?}", payload)));

    let p_received_max_ballot = p1b_fail
        .interleave(p_received_p2b_ballots)
        .interleave(p_to_proposers_i_am_leader_forward_ref)
        .max()
        .unwrap_or(proposers.singleton(q!(Ballot {
            num: 0,
            proposer_id: MemberId::from_raw_id(0)
        })));

    let (p_ballot, p_has_largest_ballot) = p_ballot_calc(
        proposer_tick,
        p_received_max_ballot.snapshot(
            proposer_tick,
            nondet!(
                /// A stale max ballot might result in us failing to become the leader, but which proposer
                /// becomes the leader is non-deterministic anyway.
                nondet_leader
            ),
        ),
    );

    let (p_to_proposers_i_am_leader, p_trigger_election) = p_leader_heartbeat(
        proposers,
        proposer_tick,
        p_is_leader_forward_ref,
        p_ballot.clone(),
        paxos_config,
        nondet!(
            /// Non-determinism in heartbeats may lead to additional leader election attempts, which
            /// is propagated to the non-determinism of which leader is elected.
            nondet_leader
        ),
    );

    p_to_proposers_i_am_leader_complete_cycle.complete(p_to_proposers_i_am_leader);

    let p_to_acceptors_p1a = p_trigger_election
        .if_some_then(p_ballot.clone())
        .all_ticks()
        .inspect(q!(|_| println!("Proposer leader expired, sending P1a")))
        .broadcast_bincode(acceptors, nondet!(/** TODO */))
        .values();

    let (a_max_ballot, a_to_proposers_p1b) = acceptor_p1(
        acceptor_tick,
        p_to_acceptors_p1a.batch(
            acceptor_tick,
            nondet!(
                /// Non-deterministic batching may result in different payloads being rejected
                /// by an acceptor if the payload is batched with another payload with larger ballot.
                /// But as documented, payloads may be non-deterministically dropped during leader election.
                nondet_acceptor_ballot
            ),
        ),
        a_log,
        proposers,
    );

    let (p_is_leader, p_accepted_values, fail_ballots) = p_p1b(
        proposer_tick,
        a_to_proposers_p1b.inspect(q!(|p1b| println!("Proposer received P1b: {:?}", p1b))),
        p_ballot.clone(),
        p_has_largest_ballot,
        quorum_size,
        num_quorum_participants,
    );
    p_is_leader_complete_cycle.complete(p_is_leader.clone());
    p1b_fail_complete.complete(fail_ballots);

    (p_ballot, p_is_leader, p_accepted_values, a_max_ballot)
}

// Proposer logic to calculate the next ballot number. Expects p_received_max_ballot, the largest ballot received so far. Outputs streams: ballot_num, and has_largest_ballot, which only contains a value if we have the largest ballot.
#[expect(clippy::type_complexity, reason = "internal paxos code // TODO")]
fn p_ballot_calc<'a>(
    proposer_tick: &Tick<Cluster<'a, Proposer>>,
    p_received_max_ballot: Singleton<Ballot, Tick<Cluster<'a, Proposer>>, Bounded>,
) -> (
    Singleton<Ballot, Tick<Cluster<'a, Proposer>>, Bounded>,
    Optional<(), Tick<Cluster<'a, Proposer>>, Bounded>,
) {
    let (p_ballot_num_complete_cycle, p_ballot_num) =
        proposer_tick.cycle_with_initial(proposer_tick.singleton(q!(0)));

    let p_new_ballot_num = p_received_max_ballot
        .clone()
        .zip(p_ballot_num.clone())
        .map(q!(move |(received_max_ballot, ballot_num)| {
            if received_max_ballot
                > (Ballot {
                    num: ballot_num,
                    proposer_id: CLUSTER_SELF_ID.clone(),
                })
            {
                received_max_ballot.num + 1
            } else {
                ballot_num
            }
        }));
    p_ballot_num_complete_cycle.complete_next_tick(p_new_ballot_num);

    let p_ballot = p_ballot_num.map(q!(move |num| Ballot {
        num,
        proposer_id: CLUSTER_SELF_ID.clone()
    }));

    let p_has_largest_ballot = p_received_max_ballot
        .clone()
        .zip(p_ballot.clone())
        .filter(q!(
            |(received_max_ballot, cur_ballot)| *received_max_ballot <= *cur_ballot
        ))
        .map(q!(|_| ()));

    // End stable leader election
    (p_ballot, p_has_largest_ballot)
}

#[expect(clippy::type_complexity, reason = "internal paxos code // TODO")]
fn p_leader_heartbeat<'a>(
    proposers: &Cluster<'a, Proposer>,
    proposer_tick: &Tick<Cluster<'a, Proposer>>,
    p_is_leader: Optional<(), Tick<Cluster<'a, Proposer>>, Bounded>,
    p_ballot: Singleton<Ballot, Tick<Cluster<'a, Proposer>>, Bounded>,
    paxos_config: PaxosConfig,
    nondet_reelection: NonDet,
) -> (
    Stream<Ballot, Cluster<'a, Proposer>, Unbounded, NoOrder, AtLeastOnce>,
    Optional<(), Tick<Cluster<'a, Proposer>>, Bounded>,
) {
    let i_am_leader_send_timeout = paxos_config.i_am_leader_send_timeout;
    let i_am_leader_check_timeout = paxos_config.i_am_leader_check_timeout;
    let i_am_leader_check_timeout_delay_multiplier =
        paxos_config.i_am_leader_check_timeout_delay_multiplier;

    let p_to_proposers_i_am_leader = p_is_leader
        .clone()
        .if_some_then(p_ballot)
        .latest()
        .sample_every(
            q!(Duration::from_secs(i_am_leader_send_timeout)),
            nondet!(
                /// Delays in heartbeats may lead to leader election attempts even
                /// if the leader is alive. This will result in the previous leader receiving
                /// larger ballots from its peers and it will drop its leadership.
                nondet_reelection
            ),
        )
        .broadcast_bincode(proposers, nondet!(/** TODO */))
        .values();

    let p_leader_expired = p_to_proposers_i_am_leader
        .clone()
        .timeout(
            q!(Duration::from_secs(i_am_leader_check_timeout)),
            nondet!(
                /// Delayed timeouts only affect which leader wins re-election. If the leadership flag
                /// is gained after timeout correctly ignore the timeout. If the flag is lost after
                /// timeout we correctly attempt to become the leader.
                nondet_reelection
            ),
        )
        .snapshot(proposer_tick, nondet!(/** absorbed into timeout */))
        .filter_if_none(p_is_leader);

    // Add random delay depending on node ID so not everyone sends p1a at the same time
    let p_trigger_election = p_leader_expired.filter_if_some(
        proposers
            .source_interval_delayed(
                q!(Duration::from_secs(
                    (CLUSTER_SELF_ID.get_raw_id()
                        * i_am_leader_check_timeout_delay_multiplier as u32)
                        .into()
                )),
                q!(Duration::from_secs(i_am_leader_check_timeout)),
                nondet!(
                    /// If the leader 'un-expires' due to non-determinstic delay, we return
                    /// to a stable leader state. If the leader remains expired, non-deterministic
                    /// delay is propagated to the non-determinism of which leader is elected.
                    nondet_reelection
                ),
            )
            .batch(proposer_tick, nondet!(/** absorbed into interval */))
            .first(),
    );
    (p_to_proposers_i_am_leader, p_trigger_election)
}

#[expect(clippy::type_complexity, reason = "internal paxos code // TODO")]
fn acceptor_p1<'a, L: Serialize + DeserializeOwned + Clone>(
    acceptor_tick: &Tick<Cluster<'a, Acceptor>>,
    p_to_acceptors_p1a: Stream<Ballot, Tick<Cluster<'a, Acceptor>>, Bounded, NoOrder>,
    a_log: Singleton<(Option<usize>, L), Tick<Cluster<'a, Acceptor>>, Bounded>,
    proposers: &Cluster<'a, Proposer>,
) -> (
    Singleton<Ballot, Tick<Cluster<'a, Acceptor>>, Bounded>,
    Stream<(Ballot, Result<(Option<usize>, L), Ballot>), Cluster<'a, Proposer>, Unbounded, NoOrder>,
) {
    let a_max_ballot = p_to_acceptors_p1a
        .clone()
        .inspect(q!(|p1a| println!("Acceptor received P1a: {:?}", p1a)))
        .persist()
        .max()
        .unwrap_or(acceptor_tick.singleton(q!(Ballot {
            num: 0,
            proposer_id: MemberId::from_raw_id(0)
        })));

    (
        a_max_ballot.clone(),
        p_to_acceptors_p1a
            .cross_singleton(a_max_ballot)
            .cross_singleton(a_log)
            .map(q!(|((ballot, max_ballot), log)| (
                ballot.proposer_id.clone(),
                (
                    ballot.clone(),
                    if ballot == max_ballot {
                        Ok(log)
                    } else {
                        Err(max_ballot)
                    }
                )
            )))
            .all_ticks()
            .demux_bincode(proposers)
            .values(),
    )
}

// Proposer logic for processing p1bs, determining if the proposer is now the leader, which uncommitted messages to commit, what the maximum slot is in the p1bs, and which no-ops to commit to fill log holes.
#[expect(clippy::type_complexity, reason = "internal paxos code // TODO")]
fn p_p1b<'a, P: Clone + Serialize + DeserializeOwned>(
    proposer_tick: &Tick<Cluster<'a, Proposer>>,
    a_to_proposers_p1b: Stream<
        (Ballot, Result<(Option<usize>, P), Ballot>),
        Cluster<'a, Proposer>,
        Unbounded,
        NoOrder,
    >,
    p_ballot: Singleton<Ballot, Tick<Cluster<'a, Proposer>>, Bounded>,
    p_has_largest_ballot: Optional<(), Tick<Cluster<'a, Proposer>>, Bounded>,
    quorum_size: usize,
    num_quorum_participants: usize,
) -> (
    Optional<(), Tick<Cluster<'a, Proposer>>, Bounded>,
    Stream<(Option<usize>, P), Tick<Cluster<'a, Proposer>>, Bounded, NoOrder>,
    Stream<Ballot, Cluster<'a, Proposer>, Unbounded, NoOrder>,
) {
    let (quorums, fails) =
        collect_quorum_with_response(a_to_proposers_p1b, quorum_size, num_quorum_participants);

    let p_received_quorum_of_p1bs = quorums
        .into_keyed()
        .assume_ordering::<TotalOrder>(nondet!(
            /// We use `flatten_unordered` later
        ))
        .fold_early_stop(
            q!(|| vec![]),
            q!(move |logs, log| {
                logs.push(log);
                logs.len() >= quorum_size
            }),
        )
        .get_max_key()
        .snapshot(
            proposer_tick,
            nondet!(
                /// If the max_by_key result is stale in this snapshot, we might be delayed in
                /// realizing that we are the leader. This does not result in any safety issues.
                /// In the meantime, ballots that acceptors rejected might be fed back into the max
                /// ballot calculation, and change `p_ballot`. By using an async snapshot, we might
                /// miss a chance to become the leader for a short period of time (until the rejected
                /// ballots are processed). This is fine, if there is a higher ballot somewhere out
                /// there, we should not be the leader anyways.
            ),
        )
        .zip(p_ballot.clone())
        .filter_map(q!(
            move |((quorum_ballot, quorum_accepted), my_ballot)| if quorum_ballot == my_ballot {
                Some(quorum_accepted)
            } else {
                None
            }
        ));

    let p_is_leader = p_received_quorum_of_p1bs
        .clone()
        .map(q!(|_| ()))
        .filter_if_some(p_has_largest_ballot.clone());

    (
        p_is_leader,
        // we used an unordered accumulator, so flattened has no order
        p_received_quorum_of_p1bs.flatten_unordered(),
        fails.map(q!(|(_, ballot)| ballot)),
    )
}

#[expect(clippy::type_complexity, reason = "internal paxos code // TODO")]
pub fn recommit_after_leader_election<'a, P: PaxosPayload>(
    accepted_logs: Stream<
        (Option<usize>, HashMap<usize, LogValue<P>>),
        Tick<Cluster<'a, Proposer>>,
        Bounded,
        NoOrder,
    >,
    p_ballot: Singleton<Ballot, Tick<Cluster<'a, Proposer>>, Bounded>,
    f: usize,
) -> (
    Stream<((usize, Ballot), Option<P>), Tick<Cluster<'a, Proposer>>, Bounded, NoOrder>,
    Optional<usize, Tick<Cluster<'a, Proposer>>, Bounded>,
) {
    let p_p1b_max_checkpoint = accepted_logs
        .clone()
        .filter_map(q!(|(checkpoint, _log)| checkpoint))
        .max()
        .into_singleton();
    let p_p1b_highest_entries_and_count = accepted_logs
        .map(q!(|(_checkpoint, log)| log))
        .flatten_unordered() // Convert HashMap log back to stream
        .into_keyed()
        .fold_commutative::<(usize, Option<LogValue<P>>), _, _>(q!(|| (0, None)), q!(|curr_entry, new_entry| {
            if let Some(curr_entry_payload) = &mut curr_entry.1 {
                let same_values = new_entry.value == curr_entry_payload.value;
                let higher_ballot = new_entry.ballot > curr_entry_payload.ballot;
                // Increment count if the values are the same
                if same_values {
                    curr_entry.0 += 1;
                }
                // Replace the ballot with the largest one
                if higher_ballot {
                    curr_entry_payload.ballot = new_entry.ballot;
                    // Replace the value with the one from the largest ballot, if necessary
                    if !same_values {
                        curr_entry.0 = 1;
                        curr_entry_payload.value = new_entry.value;
                    }
                }
            } else {
                *curr_entry = (1, Some(new_entry));
            }
        }))
        .map(q!(|(count, entry)| (count, entry.unwrap())));
    let p_log_to_try_commit = p_p1b_highest_entries_and_count
        .clone()
        .entries()
        .cross_singleton(p_ballot.clone())
        .cross_singleton(p_p1b_max_checkpoint.clone())
        .filter_map(q!(move |(((slot, (count, entry)), ballot), checkpoint)| {
            if count > f {
                return None;
            } else if let Some(checkpoint) = checkpoint
                && slot <= checkpoint
            {
                return None;
            }
            Some(((slot, ballot), entry.value))
        }));
    let p_max_slot = p_p1b_highest_entries_and_count.clone().keys().max();
    let p_proposed_slots = p_p1b_highest_entries_and_count.clone().keys();
    let p_log_holes = p_max_slot
        .clone()
        .zip(p_p1b_max_checkpoint)
        .flat_map_ordered(q!(|(max_slot, checkpoint)| {
            if let Some(checkpoint) = checkpoint {
                (checkpoint + 1)..max_slot
            } else {
                0..max_slot
            }
        }))
        .filter_not_in(p_proposed_slots)
        .cross_singleton(p_ballot.clone())
        .map(q!(move |(slot, ballot)| ((slot, ballot), None)));

    (p_log_to_try_commit.chain(p_log_holes), p_max_slot)
}

#[expect(
    clippy::type_complexity,
    clippy::too_many_arguments,
    reason = "internal paxos code // TODO"
)]
fn sequence_payload<'a, P: PaxosPayload>(
    proposers: &Cluster<'a, Proposer>,
    acceptors: &Cluster<'a, Acceptor>,
    proposer_tick: &Tick<Cluster<'a, Proposer>>,
    acceptor_tick: &Tick<Cluster<'a, Acceptor>>,
    c_to_proposers: Stream<P, Cluster<'a, Proposer>, Unbounded>,
    a_checkpoint: Optional<usize, Cluster<'a, Acceptor>, Unbounded>,

    p_ballot: Singleton<Ballot, Tick<Cluster<'a, Proposer>>, Bounded>,
    p_is_leader: Optional<(), Tick<Cluster<'a, Proposer>>, Bounded>,

    p_relevant_p1bs: Stream<
        (Option<usize>, HashMap<usize, LogValue<P>>),
        Tick<Cluster<'a, Proposer>>,
        Bounded,
        NoOrder,
    >,
    f: usize,

    a_max_ballot: Singleton<Ballot, Tick<Cluster<'a, Acceptor>>, Bounded>,

    nondet_commit: NonDet,
) -> (
    Stream<(usize, Option<P>), Cluster<'a, Proposer>, Unbounded, NoOrder>,
    Singleton<
        (Option<usize>, HashMap<usize, LogValue<P>>),
        Atomic<Cluster<'a, Acceptor>>,
        Unbounded,
    >,
    Stream<Ballot, Cluster<'a, Proposer>, Unbounded, NoOrder>,
) {
    let (p_log_to_recommit, p_max_slot) =
        recommit_after_leader_election(p_relevant_p1bs, p_ballot.clone(), f);

    let indexed_payloads = index_payloads(
        proposer_tick,
        p_max_slot,
        c_to_proposers
            .batch(
                proposer_tick,
                nondet!(
                    /// We batch payloads so that we can compute the correct slot based on
                    /// base slot. In the case of a leader re-election, the base slot is updated which
                    /// affects the computed payload slots. This non-determinism can lead to non-determinism
                    /// in which payloads are committed when the leader is changing, which is documented at
                    /// the function level
                    nondet_commit
                ),
            )
            .filter_if_some(p_is_leader.clone()),
    );

    let payloads_to_send = indexed_payloads
        .cross_singleton(p_ballot.clone())
        .map(q!(|((slot, payload), ballot)| (
            (slot, ballot),
            Some(payload)
        )))
        .chain(p_log_to_recommit)
        .filter_if_some(p_is_leader)
        .all_ticks_atomic();

    let (a_log, a_to_proposers_p2b) = acceptor_p2(
        acceptor_tick,
        a_max_ballot.clone(),
        payloads_to_send
            .clone()
            .end_atomic()
            .map(q!(move |((slot, ballot), value)| P2a {
                sender: CLUSTER_SELF_ID.clone(),
                ballot,
                slot,
                value
            }))
            .broadcast_bincode(acceptors, nondet!(/** TODO */))
            .values(),
        a_checkpoint,
        proposers,
    );

    // TOOD: only persist if we are the leader
    let (quorums, fails) = collect_quorum(a_to_proposers_p2b, f + 1, 2 * f + 1);

    let p_to_replicas = join_responses(
        quorums.map(q!(|k| (k, ()))),
        payloads_to_send.batch_atomic(nondet!(
            /// The metadata will always be generated before we get a quorum
            /// because our batch of `payloads_to_send` is at least after
            /// what we sent to the acceptors.
        )),
    );

    (
        p_to_replicas.map(q!(|((slot, _ballot), (value, _))| (slot, value))),
        a_log,
        fails.map(q!(|(_, ballot)| ballot)),
    )
}

// Proposer logic to send p2as, outputting the next slot and the p2as to send to acceptors.
pub fn index_payloads<'a, L: Location<'a>, P: PaxosPayload>(
    proposer_tick: &Tick<L>,
    p_max_slot: Optional<usize, Tick<L>, Bounded>,
    c_to_proposers: Stream<P, Tick<L>, Bounded>,
) -> Stream<(usize, P), Tick<L>, Bounded> {
    let (p_next_slot_complete_cycle, p_next_slot) =
        proposer_tick.cycle_with_initial::<Singleton<usize, _, _>>(proposer_tick.singleton(q!(0)));
    let p_next_slot_after_reconciling_p1bs = p_max_slot.map(q!(|max_slot| max_slot + 1));

    let base_slot = p_next_slot_after_reconciling_p1bs.unwrap_or(p_next_slot);

    let p_indexed_payloads = c_to_proposers
        .enumerate()
        .cross_singleton(base_slot.clone())
        .map(q!(|((index, payload), base_slot)| (
            base_slot + index,
            payload
        )));

    let p_num_payloads = p_indexed_payloads.clone().count();
    let p_next_slot_after_sending_payloads =
        p_num_payloads
            .clone()
            .zip(base_slot)
            .map(q!(|(num_payloads, base_slot)| base_slot + num_payloads));

    p_next_slot_complete_cycle.complete_next_tick(p_next_slot_after_sending_payloads);
    p_indexed_payloads
}

#[expect(clippy::type_complexity, reason = "internal paxos code // TODO")]
pub fn acceptor_p2<'a, P: PaxosPayload, S: Clone>(
    acceptor_tick: &Tick<Cluster<'a, Acceptor>>,
    a_max_ballot: Singleton<Ballot, Tick<Cluster<'a, Acceptor>>, Bounded>,
    p_to_acceptors_p2a: Stream<P2a<P, S>, Cluster<'a, Acceptor>, Unbounded, NoOrder>,
    a_checkpoint: Optional<usize, Cluster<'a, Acceptor>, Unbounded>,
    proposers: &Cluster<'a, S>,
) -> (
    Singleton<
        (Option<usize>, HashMap<usize, LogValue<P>>),
        Atomic<Cluster<'a, Acceptor>>,
        Unbounded,
    >,
    Stream<((usize, Ballot), Result<(), Ballot>), Cluster<'a, S>, Unbounded, NoOrder>,
) {
    let p_to_acceptors_p2a_batch = p_to_acceptors_p2a.batch(
        acceptor_tick,
        nondet!(
            /// We use batches to ensure that the log is updated before sending
            /// a confirmation to the proposer. Because we use `persist()` on these
            /// messages before folding into the log, non-deterministic batch boundaries
            /// will not affect the eventual log state.
        ),
    );

    let a_checkpoint = a_checkpoint.snapshot(
        acceptor_tick,
        nondet!(
            /// We can arbitrarily snapshot the checkpoint sequence number,
            /// since a delayed garbage collection does not affect correctness.
        ),
    );
    // .inspect(q!(|min_seq| println!("Acceptor new checkpoint: {:?}", min_seq)));

    let a_p2as_to_place_in_log = p_to_acceptors_p2a_batch
        .clone()
        .cross_singleton(a_max_ballot.clone()) // Don't consider p2as if the current ballot is higher
        .filter_map(q!(|(p2a, max_ballot)|
            if p2a.ballot >= max_ballot {
                Some((p2a.slot, LogValue {
                    ballot: p2a.ballot,
                    value: p2a.value,
                }))
            } else {
                None
            }
        ));
    let a_log = a_p2as_to_place_in_log
        .all_ticks_atomic()
        .into_keyed()
        .reduce_watermark_commutative(
            a_checkpoint.clone(),
            q!(|prev_entry, entry| {
                // Insert p2a into the log if it has a higher ballot than what was there before
                if entry.ballot > prev_entry.ballot {
                    *prev_entry = LogValue {
                        ballot: entry.ballot,
                        value: entry.value,
                    };
                }
            }),
        );
    let a_log_snapshot = a_log
        .snapshot_atomic(nondet!(
            /// We need to know the current state of the log for p1b
            /// TODO(shadaj): this isn't a justification for correctness
        ))
        .entries()
        .fold_commutative(
            q!(|| HashMap::new()),
            q!(|map, (slot, entry)| {
                map.insert(slot, entry);
            }),
        );

    let a_to_proposers_p2b = p_to_acceptors_p2a_batch
        .cross_singleton(a_max_ballot)
        .map(q!(|(p2a, max_ballot)| (
            p2a.sender,
            (
                (p2a.slot, p2a.ballot.clone()),
                if p2a.ballot == max_ballot {
                    Ok(())
                } else {
                    Err(max_ballot)
                }
            )
        )))
        .all_ticks()
        .demux_bincode(proposers)
        .values();

    (
        a_checkpoint
            .into_singleton()
            .zip(a_log_snapshot)
            .latest_atomic(),
        a_to_proposers_p2b,
    )
}

#[cfg(test)]
mod tests {
    use hydro_lang::prelude::*;

    use super::index_payloads;

    #[test]
    fn proposer_indexes_payloads() {
        let flow = FlowBuilder::new();
        let node = flow.process::<()>();
        let tick = node.tick();

        let (in_send, input_payloads) = node.sim_input();
        let indexed = index_payloads(
            &tick,
            tick.none(),
            input_payloads.batch(&tick, nondet!(/** test */)),
        );

        let out_recv = indexed.all_ticks().sim_output();

        flow.sim().exhaustive(async || {
            in_send.send(1);
            in_send.send(2);
            in_send.send(3);
            in_send.send(4);

            out_recv
                .assert_yields_only([(0, 1), (1, 2), (2, 3), (3, 4)])
                .await;
        });
    }

    #[test]
    fn proposer_indexes_payloads_jumps_on_new_max() {
        let flow = FlowBuilder::new();
        let node = flow.process::<()>();
        let tick = node.tick();

        let (in_send, input_payloads) = node.sim_input();
        let release_new_base = node
            .source_iter(q!([123]))
            .batch(&tick, nondet!(/** test */))
            .first();
        let indexed = index_payloads(
            &tick,
            release_new_base,
            input_payloads.batch(&tick, nondet!(/** test */)),
        );

        let out_recv = indexed.all_ticks().sim_output();

        flow.sim().exhaustive(async || {
            in_send.send(1);
            in_send.send(2);
            in_send.send(3);
            in_send.send(4);

            let mut next_expected = 0;
            for i in 1..=4 {
                let (next_slot, v) = out_recv.next().await.unwrap();
                assert_eq!(v, i);

                if next_expected < 123 {
                    assert!(next_slot == next_expected || next_slot == 124);
                } else {
                    assert!(next_slot == next_expected);
                }

                next_expected = next_slot + 1;
            }
        });
    }
}
