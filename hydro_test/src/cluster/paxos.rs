use std::collections::HashMap;
use std::fmt::Debug;
use std::hash::Hash;
use std::time::Duration;

use hydro_lang::*;
use hydro_std::quorum::{collect_quorum, collect_quorum_with_response};
use hydro_std::request_response::join_responses;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};

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

#[derive(Serialize, Deserialize, PartialEq, Eq, Copy, Clone, Debug, Hash)]
pub struct Ballot {
    pub num: u32,
    pub proposer_id: ClusterId<Proposer>,
}

impl Ord for Ballot {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.num
            .cmp(&other.num)
            .then_with(|| self.proposer_id.raw_id.cmp(&other.proposer_id.raw_id))
    }
}

impl PartialOrd for Ballot {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
struct LogValue<P> {
    ballot: Ballot,
    value: Option<P>, // might be a hole
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Clone, Debug)]
struct P2a<P> {
    ballot: Ballot,
    slot: usize,
    value: Option<P>, // might be a re-committed hole
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
/// # Safety
/// When the leader is stable, the algorithm will commit incoming payloads to the leader
/// in deterministic order. However, when the leader is changing, payloads may be
/// non-deterministically dropped. The stream of ballots is also non-deterministic because
/// leaders are elected in a non-deterministic process.
#[expect(clippy::type_complexity, reason = "internal paxos code // TODO")]
pub unsafe fn paxos_core<'a, P: PaxosPayload, R>(
    proposers: &Cluster<'a, Proposer>,
    acceptors: &Cluster<'a, Acceptor>,
    r_to_acceptors_checkpoint: Stream<
        (ClusterId<R>, usize),
        Cluster<'a, Acceptor>,
        Unbounded,
        NoOrder,
    >,
    c_to_proposers: impl FnOnce(
        Stream<Ballot, Cluster<'a, Proposer>, Unbounded>,
    ) -> Stream<P, Cluster<'a, Proposer>, Unbounded>,
    config: PaxosConfig,
) -> (
    Stream<Ballot, Cluster<'a, Proposer>, Unbounded>,
    Stream<(usize, Option<P>), Cluster<'a, Proposer>, Unbounded, NoOrder>,
) {
    let f = config.f;
    let i_am_leader_send_timeout = config.i_am_leader_send_timeout;
    let i_am_leader_check_timeout = config.i_am_leader_check_timeout;
    let i_am_leader_check_timeout_delay_multiplier =
        config.i_am_leader_check_timeout_delay_multiplier;

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

    let (p_ballot, p_is_leader, p_relevant_p1bs, a_max_ballot) = unsafe {
        // SAFETY: The primary non-determinism exposed by leader election algorithm lies in which leader
        // is elected, which affects both the ballot at each proposer and the leader flag. But using a stale ballot
        // or leader flag will only lead to failure in sequencing rather than commiting the wrong value. Because
        // ballots are non-deterministic, the acceptor max ballot is also non-deterministic, although we are
        // guaranteed that the max ballot will match the current ballot of a proposer who believes they are the leader.
        leader_election(
            proposers,
            acceptors,
            &proposer_tick,
            &acceptor_tick,
            f,
            i_am_leader_send_timeout,
            i_am_leader_check_timeout,
            i_am_leader_check_timeout_delay_multiplier,
            sequencing_max_ballot_forward_reference,
            a_log_forward_reference,
        )
    };

    let just_became_leader = p_is_leader
        .clone()
        .continue_unless(p_is_leader.clone().defer_tick());

    let c_to_proposers = c_to_proposers(
        just_became_leader
            .clone()
            .then(p_ballot.clone())
            .all_ticks()
            .drop_timestamp(),
    );

    let (p_to_replicas, a_log, sequencing_max_ballots) = unsafe {
        // SAFETY: The relevant p1bs are non-deterministic because they come from a arbitrary quorum, but because
        // we use a quorum, if we remain the leader there are no missing committed values when we combine the logs.
        // The remaining non-determinism is in when incoming payloads are batched versus the leader flag and state
        // of acceptors, which in the worst case will lead to dropped payloads as documented.
        sequence_payload(
            proposers,
            acceptors,
            &proposer_tick,
            &acceptor_tick,
            c_to_proposers,
            r_to_acceptors_checkpoint,
            p_ballot.clone(),
            p_is_leader,
            p_relevant_p1bs,
            f,
            a_max_ballot,
        )
    };

    a_log_complete_cycle.complete(unsafe {
        // SAFETY: We will always write payloads to the log before acknowledging them to the proposers,
        // which guarantees that if the leader changes the quorum overlap between sequencing and leader
        // election will include the committed value.
        a_log.latest_tick()
    });
    sequencing_max_ballot_complete_cycle.complete(sequencing_max_ballots);

    (
        // Only tell the clients once when leader election concludes
        just_became_leader
            .then(p_ballot)
            .all_ticks()
            .drop_timestamp(),
        p_to_replicas,
    )
}

#[expect(
    clippy::type_complexity,
    clippy::too_many_arguments,
    reason = "internal paxos code // TODO"
)]
unsafe fn leader_election<'a, L: Clone + Debug + Serialize + DeserializeOwned>(
    proposers: &Cluster<'a, Proposer>,
    acceptors: &Cluster<'a, Acceptor>,
    proposer_tick: &Tick<Cluster<'a, Proposer>>,
    acceptor_tick: &Tick<Cluster<'a, Acceptor>>,
    f: usize,
    i_am_leader_send_timeout: u64,
    i_am_leader_check_timeout: u64,
    i_am_leader_check_timeout_delay_multiplier: usize,
    p_received_p2b_ballots: Stream<Ballot, Cluster<'a, Proposer>, Unbounded, NoOrder>,
    a_log: Singleton<(Option<usize>, L), Tick<Cluster<'a, Acceptor>>, Bounded>,
) -> (
    Singleton<Ballot, Tick<Cluster<'a, Proposer>>, Bounded>,
    Optional<(), Tick<Cluster<'a, Proposer>>, Bounded>,
    Stream<(Option<usize>, L), Tick<Cluster<'a, Proposer>>, Bounded, NoOrder>,
    Singleton<Ballot, Tick<Cluster<'a, Acceptor>>, Bounded>,
) {
    let (p1b_fail_complete, p1b_fail) =
        proposers.forward_ref::<Stream<Ballot, _, Unbounded, NoOrder>>();
    let (p_to_proposers_i_am_leader_complete_cycle, p_to_proposers_i_am_leader_forward_ref) =
        proposers.forward_ref::<Stream<_, _, _, NoOrder>>();
    let (p_is_leader_complete_cycle, p_is_leader_forward_ref) =
        proposer_tick.forward_ref::<Optional<(), _, _>>();
    // a_to_proposers_p2b.clone().for_each(q!(|(_, p2b): (u32, P2b)| println!("Proposer received P2b: {:?}", p2b)));
    // p_to_proposers_i_am_leader.clone().for_each(q!(|ballot: Ballot| println!("Proposer received I am leader: {:?}", ballot)));
    // c_to_proposers.clone().for_each(q!(|payload: ClientPayload| println!("Client sent proposer payload: {:?}", payload)));

    let p_received_max_ballot = p1b_fail
        .union(p_received_p2b_ballots)
        .union(p_to_proposers_i_am_leader_forward_ref)
        .max()
        .unwrap_or(proposers.singleton(q!(Ballot {
            num: 0,
            proposer_id: ClusterId::from_raw(0)
        })));

    let (p_ballot, p_has_largest_ballot) = p_ballot_calc(proposer_tick, unsafe {
        // SAFETY: A stale max ballot might result in us failing to become the leader, but which proposer
        // becomes the leader is non-deterministic anyway.
        p_received_max_ballot
            .timestamped(proposer_tick)
            .latest_tick()
    });

    let (p_to_proposers_i_am_leader, p_trigger_election) = unsafe {
        // SAFETY: non-determinism in heartbeats may lead to additional leader election attempts, which
        // is propagated to the non-determinism of which leader is elected.
        p_leader_heartbeat(
            proposers,
            proposer_tick,
            p_is_leader_forward_ref,
            p_ballot.clone(),
            i_am_leader_send_timeout,
            i_am_leader_check_timeout,
            i_am_leader_check_timeout_delay_multiplier,
        )
    };

    p_to_proposers_i_am_leader_complete_cycle.complete(p_to_proposers_i_am_leader);

    let p_to_acceptors_p1a = p_trigger_election
        .then(p_ballot.clone())
        .all_ticks()
        .inspect(q!(|_| println!("Proposer leader expired, sending P1a")))
        .broadcast_bincode_interleaved(acceptors);

    let (a_max_ballot, a_to_proposers_p1b) = acceptor_p1(
        acceptor_tick,
        unsafe {
            // SAFETY: Non-deterministic batching may result in different payloads being rejected
            // by an acceptor if the payload is batched with another payload with larger ballot.
            // But as documented, payloads may be non-deterministically dropped during leader election.
            p_to_acceptors_p1a.timestamped(acceptor_tick).tick_batch()
        },
        a_log,
        proposers,
    );

    let (p_is_leader, p_accepted_values, fail_ballots) = p_p1b(
        proposer_tick,
        a_to_proposers_p1b.inspect(q!(|p1b| println!("Proposer received P1b: {:?}", p1b))),
        p_ballot.clone(),
        p_has_largest_ballot,
        f,
    );
    p_is_leader_complete_cycle.complete(p_is_leader.clone());
    p1b_fail_complete.complete(fail_ballots.drop_timestamp());

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
                    proposer_id: CLUSTER_SELF_ID,
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
        proposer_id: CLUSTER_SELF_ID
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
unsafe fn p_leader_heartbeat<'a>(
    proposers: &Cluster<'a, Proposer>,
    proposer_tick: &Tick<Cluster<'a, Proposer>>,
    p_is_leader: Optional<(), Tick<Cluster<'a, Proposer>>, Bounded>,
    p_ballot: Singleton<Ballot, Tick<Cluster<'a, Proposer>>, Bounded>,
    i_am_leader_send_timeout: u64,  // How often to heartbeat
    i_am_leader_check_timeout: u64, // How often to check if heartbeat expired
    i_am_leader_check_timeout_delay_multiplier: usize, /* Initial delay, multiplied by proposer pid, to stagger proposers checking for timeouts */
) -> (
    Stream<Ballot, Cluster<'a, Proposer>, Unbounded, NoOrder>,
    Optional<(), Tick<Cluster<'a, Proposer>>, Bounded>,
) {
    let p_to_proposers_i_am_leader = unsafe {
        // SAFETY: Delays in heartbeats may lead to leader election attempts even
        // if the leader is alive. This will result in the previous leader receiving
        // larger ballots from its peers and it will drop its leadership.
        p_is_leader
            .clone()
            .then(p_ballot)
            .latest()
            .drop_timestamp()
            .sample_every(q!(Duration::from_secs(i_am_leader_send_timeout)))
    }
    .broadcast_bincode_interleaved(proposers);

    let p_leader_expired = unsafe {
        // Delayed timeouts only affect which leader wins re-election. If the leadership flag
        // is gained after timeout correctly ignore the timeout. If the flag is lost after
        // timeout we correctly attempt to become the leader.
        p_to_proposers_i_am_leader
            .clone()
            .timeout(q!(Duration::from_secs(i_am_leader_check_timeout)))
            .timestamped(proposer_tick)
            .latest_tick()
            .continue_unless(p_is_leader)
    };

    // Add random delay depending on node ID so not everyone sends p1a at the same time
    let p_trigger_election = unsafe {
        // SAFETY: If the leader "un-expires" due to non-determinstic delay, we return
        // to a stable leader state. If the leader remains expired, non-deterministic
        // delay is propagated to the non-determinism of which leader is elected.
        p_leader_expired.continue_if(
            proposers
                .source_interval_delayed(
                    q!(Duration::from_secs(
                        (CLUSTER_SELF_ID.raw_id
                            * i_am_leader_check_timeout_delay_multiplier as u32)
                            .into()
                    )),
                    q!(Duration::from_secs(i_am_leader_check_timeout)),
                )
                .timestamped(proposer_tick)
                .tick_batch()
                .first(),
        )
    };
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
            proposer_id: ClusterId::from_raw(0)
        })));

    (
        a_max_ballot.clone(),
        p_to_acceptors_p1a
            .cross_singleton(a_max_ballot)
            .cross_singleton(a_log)
            .map(q!(|((ballot, max_ballot), log)| (
                ballot.proposer_id,
                (
                    ballot,
                    if ballot == max_ballot {
                        Ok(log)
                    } else {
                        Err(max_ballot)
                    }
                )
            )))
            .all_ticks()
            .send_bincode_interleaved(proposers),
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
    f: usize,
) -> (
    Optional<(), Tick<Cluster<'a, Proposer>>, Bounded>,
    Stream<(Option<usize>, P), Tick<Cluster<'a, Proposer>>, Bounded, NoOrder>,
    Stream<Ballot, Timestamped<Cluster<'a, Proposer>>, Unbounded, NoOrder>,
) {
    let (quorums, fails) = collect_quorum_with_response(
        a_to_proposers_p1b.timestamped(proposer_tick),
        f + 1,
        2 * f + 1,
    );

    let p_received_quorum_of_p1bs = unsafe {
        // SAFETY: All the values for a quorum will be emitted in a single batch,
        // so we will not split up the quorum.
        quorums.tick_batch()
    }
    .persist()
    .fold_keyed_commutative(
        q!(|| vec![]),
        q!(|logs, log| {
            // even though this is non-commutative, we use `flatten_unordered` later
            logs.push(log);
        }),
    )
    .max_by_key(q!(|t| t.0))
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
        .continue_if(p_has_largest_ballot.clone());

    (
        p_is_leader,
        // we used an unordered accumulator, so flattened has no order
        p_received_quorum_of_p1bs.flatten_unordered(),
        fails.map(q!(|(_, ballot)| ballot)),
    )
}

#[expect(clippy::type_complexity, reason = "internal paxos code // TODO")]
fn recommit_after_leader_election<'a, P: PaxosPayload>(
    accepted_logs: Stream<
        (Option<usize>, HashMap<usize, LogValue<P>>),
        Tick<Cluster<'a, Proposer>>,
        Bounded,
        NoOrder,
    >,
    p_ballot: Singleton<Ballot, Tick<Cluster<'a, Proposer>>, Bounded>,
    f: usize,
) -> (
    Stream<P2a<P>, Tick<Cluster<'a, Proposer>>, Bounded, NoOrder>,
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
        .fold_keyed_commutative::<(usize, Option<LogValue<P>>), _, _>(q!(|| (0, None)), q!(|curr_entry, new_entry| {
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
        .map(q!(|(slot, (count, entry))| (slot, (count, entry.unwrap()))));
    let p_log_to_try_commit = p_p1b_highest_entries_and_count
        .clone()
        .cross_singleton(p_ballot.clone())
        .cross_singleton(p_p1b_max_checkpoint.clone())
        .filter_map(q!(move |(((slot, (count, entry)), ballot), checkpoint)| {
            if count > f {
                return None;
            } else if let Some(checkpoint) = checkpoint {
                if slot <= checkpoint {
                    return None;
                }
            }
            Some(P2a {
                ballot,
                slot,
                value: entry.value,
            })
        }));
    let p_max_slot = p_p1b_highest_entries_and_count
        .clone()
        .map(q!(|(slot, _)| slot))
        .max();
    let p_proposed_slots = p_p1b_highest_entries_and_count
        .clone()
        .map(q!(|(slot, _)| slot));
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
        .map(q!(|(slot, ballot)| P2a {
            ballot,
            slot,
            value: None
        }));

    (p_log_to_try_commit.chain(p_log_holes), p_max_slot)
}

#[expect(
    clippy::type_complexity,
    clippy::too_many_arguments,
    reason = "internal paxos code // TODO"
)]
unsafe fn sequence_payload<'a, P: PaxosPayload, R>(
    proposers: &Cluster<'a, Proposer>,
    acceptors: &Cluster<'a, Acceptor>,
    proposer_tick: &Tick<Cluster<'a, Proposer>>,
    acceptor_tick: &Tick<Cluster<'a, Acceptor>>,
    c_to_proposers: Stream<P, Cluster<'a, Proposer>, Unbounded>,
    r_to_acceptors_checkpoint: Stream<
        (ClusterId<R>, usize),
        Cluster<'a, Acceptor>,
        Unbounded,
        NoOrder,
    >,

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
) -> (
    Stream<(usize, Option<P>), Cluster<'a, Proposer>, Unbounded, NoOrder>,
    Singleton<
        (Option<usize>, HashMap<usize, LogValue<P>>),
        Timestamped<Cluster<'a, Acceptor>>,
        Unbounded,
    >,
    Stream<Ballot, Cluster<'a, Proposer>, Unbounded, NoOrder>,
) {
    let (p_log_to_recommit, p_max_slot) =
        recommit_after_leader_election(p_relevant_p1bs, p_ballot.clone(), f);

    let indexed_payloads = index_payloads(proposer_tick, p_max_slot, unsafe {
        // SAFETY: We batch payloads so that we can compute the correct slot based on
        // base slot. In the case of a leader re-election, the base slot is updated which
        // affects the computed payload slots. This non-determinism can lead to non-determinism
        // in which payloads are committed when the leader is changing, which is documented at
        // the function level.
        c_to_proposers
            .timestamped(proposer_tick)
            .tick_batch()
            .continue_if(p_is_leader.clone())
    });

    let payloads_to_send = indexed_payloads
        .cross_singleton(p_ballot.clone())
        .map(q!(|((slot, payload), ballot)| (
            (slot, ballot),
            Some(payload)
        )))
        .chain(p_log_to_recommit.map(q!(|p2a| ((p2a.slot, p2a.ballot), p2a.value))))
        .continue_if(p_is_leader)
        .all_ticks();

    let (a_log, a_to_proposers_p2b) = acceptor_p2(
        acceptor_tick,
        a_max_ballot.clone(),
        payloads_to_send
            .clone()
            .map(q!(|((slot, ballot), value)| P2a {
                ballot,
                slot,
                value
            }))
            .broadcast_bincode_interleaved(acceptors),
        r_to_acceptors_checkpoint,
        proposers,
        f,
    );

    // TOOD: only persist if we are the leader
    let (quorums, fails) = collect_quorum(
        a_to_proposers_p2b.timestamped(proposer_tick),
        f + 1,
        2 * f + 1,
    );

    let p_to_replicas = join_responses(proposer_tick, quorums.map(q!(|k| (k, ()))), unsafe {
        // SAFETY: The metadata will always be generated before we get a quorum
        // because `payloads_to_send` is used to send the payloads to acceptors.
        payloads_to_send.tick_batch()
    });

    (
        p_to_replicas
            .map(q!(|((slot, _ballot), (value, _))| (slot, value)))
            .drop_timestamp(),
        a_log,
        fails.map(q!(|(_, ballot)| ballot)).drop_timestamp(),
    )
}

#[derive(Clone)]
enum CheckpointOrP2a<P> {
    Checkpoint(usize),
    P2a(P2a<P>),
}

// Proposer logic to send p2as, outputting the next slot and the p2as to send to acceptors.
fn index_payloads<'a, P: PaxosPayload>(
    proposer_tick: &Tick<Cluster<'a, Proposer>>,
    p_max_slot: Optional<usize, Tick<Cluster<'a, Proposer>>, Bounded>,
    c_to_proposers: Stream<P, Tick<Cluster<'a, Proposer>>, Bounded>,
) -> Stream<(usize, P), Tick<Cluster<'a, Proposer>>, Bounded> {
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
fn acceptor_p2<'a, P: PaxosPayload, R>(
    acceptor_tick: &Tick<Cluster<'a, Acceptor>>,
    a_max_ballot: Singleton<Ballot, Tick<Cluster<'a, Acceptor>>, Bounded>,
    p_to_acceptors_p2a: Stream<P2a<P>, Cluster<'a, Acceptor>, Unbounded, NoOrder>,
    r_to_acceptors_checkpoint: Stream<
        (ClusterId<R>, usize),
        Cluster<'a, Acceptor>,
        Unbounded,
        NoOrder,
    >,
    proposers: &Cluster<'a, Proposer>,
    f: usize,
) -> (
    Singleton<
        (Option<usize>, HashMap<usize, LogValue<P>>),
        Timestamped<Cluster<'a, Acceptor>>,
        Unbounded,
    >,
    Stream<((usize, Ballot), Result<(), Ballot>), Cluster<'a, Proposer>, Unbounded, NoOrder>,
) {
    let p_to_acceptors_p2a_batch = unsafe {
        // SAFETY: we use batches to ensure that the log is updated before sending
        // a confirmation to the proposer. Because we use `persist()` on these
        // messages before folding into the log, non-deterministic batch boundaries
        // will not affect the eventual log state.
        p_to_acceptors_p2a.timestamped(acceptor_tick).tick_batch()
    };

    // Get the latest checkpoint sequence per replica
    let a_checkpoint_largest_seqs = unsafe {
        // SAFETY: if a checkpoint is delayed, its effect is that the log may contain slots
        // that do not need to be saved (because the data is at all replicas). This affects
        // the logs that will be collected during a leader re-election, but eventually the
        // same checkpoint will arrive at acceptors and those slots will be eventually deleted.
        r_to_acceptors_checkpoint
            .timestamped(acceptor_tick)
            .tick_batch()
    }
    .persist()
    .reduce_keyed_commutative(q!(|curr_seq, seq| {
        if seq > *curr_seq {
            *curr_seq = seq;
        }
    }));
    let a_checkpoints_quorum_reached = a_checkpoint_largest_seqs.clone().count().filter_map(q!(
        move |num_received| if num_received == f + 1 {
            Some(true)
        } else {
            None
        }
    ));
    // Find the smallest checkpoint seq that everyone agrees to, track whenever it changes
    let a_new_checkpoint = a_checkpoint_largest_seqs
        .continue_if(a_checkpoints_quorum_reached)
        .map(q!(|(_sender, seq)| seq))
        .min()
        .delta()
        .map(q!(|min_seq| CheckpointOrP2a::Checkpoint(min_seq)));
    // .inspect(q!(|(min_seq, p2a): &(i32, P2a)| println!("Acceptor new checkpoint: {:?}", min_seq)));

    let a_p2as_to_place_in_log = p_to_acceptors_p2a_batch
        .clone()
        .cross_singleton(a_max_ballot.clone()) // Don't consider p2as if the current ballot is higher
        .filter_map(q!(|(p2a, max_ballot)|
            if p2a.ballot >= max_ballot {
                Some(CheckpointOrP2a::P2a(p2a))
            } else {
                None
            }
        ));
    let a_log = a_p2as_to_place_in_log
        .chain(a_new_checkpoint.into_stream())
        .all_ticks()
        .fold_commutative(
            q!(|| (None, HashMap::new())),
            q!(|(prev_checkpoint, log), checkpoint_or_p2a| {
                match checkpoint_or_p2a {
                    CheckpointOrP2a::Checkpoint(new_checkpoint) => {
                        if prev_checkpoint
                            .map(|prev| new_checkpoint > prev)
                            .unwrap_or(true)
                        {
                            for slot in (prev_checkpoint.unwrap_or(0))..new_checkpoint {
                                log.remove(&slot);
                            }

                            *prev_checkpoint = Some(new_checkpoint);
                        }
                    }
                    CheckpointOrP2a::P2a(p2a) => {
                        // This is a regular p2a message. Insert it into the log if it is not checkpointed and has a higher ballot than what was there before
                        if prev_checkpoint.map(|prev| p2a.slot > prev).unwrap_or(true)
                            && log
                                .get(&p2a.slot)
                                .map(|prev_p2a: &LogValue<_>| p2a.ballot > prev_p2a.ballot)
                                .unwrap_or(true)
                        {
                            log.insert(
                                p2a.slot,
                                LogValue {
                                    ballot: p2a.ballot,
                                    value: p2a.value,
                                },
                            );
                        }
                    }
                }
            }),
        );

    let a_to_proposers_p2b = p_to_acceptors_p2a_batch
        .cross_singleton(a_max_ballot)
        .map(q!(|(p2a, max_ballot)| (
            p2a.ballot.proposer_id,
            (
                (p2a.slot, p2a.ballot),
                if p2a.ballot == max_ballot {
                    Ok(())
                } else {
                    Err(max_ballot)
                }
            )
        )))
        .all_ticks()
        .send_bincode_interleaved(proposers);
    (a_log, a_to_proposers_p2b)
}
