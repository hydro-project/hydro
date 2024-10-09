use std::collections::HashMap;
use std::time::Duration;

use hydroflow_plus::*;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use stageleft::*;
use tokio::time::Instant;

use super::paxos_bench::LeaderElected;

pub struct Proposer {}
pub struct Acceptor {}

pub trait PaxosPayload:
    Serialize + DeserializeOwned + PartialEq + Eq + Default + Clone + std::fmt::Debug
{
}

#[derive(Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Clone, Debug, Hash)]
pub struct Ballot {
    // Note: Important that num comes before id, since Ord is defined lexicographically
    pub num: u32,
    pub id: ClusterId<Proposer>,
}

impl LeaderElected for Ballot {
    fn leader_id(&self) -> ClusterId<Proposer> {
        self.id
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
struct P1a {
    ballot: Ballot,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
struct LogValue<P> {
    ballot: Ballot,
    value: P,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
struct P1b<P> {
    ballot: Ballot,
    max_ballot: Ballot,
    accepted: HashMap<i32, LogValue<P>>,
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Clone, Debug)]
struct P2a<P> {
    ballot: Ballot,
    slot: i32,
    value: P,
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Clone, Debug)]
struct P2b<P> {
    ballot: Ballot,
    max_ballot: Ballot,
    slot: i32,
    value: P,
}

#[expect(clippy::type_complexity, reason = "internal paxos code // TODO")]
pub fn paxos_core<'a, P: PaxosPayload, R>(
    flow: &FlowBuilder<'a>,
    r_to_acceptors_checkpoint: impl FnOnce(
        &Cluster<'a, Acceptor>,
    ) -> Stream<
        (ClusterId<R>, i32),
        Unbounded,
        NoTick,
        Cluster<'a, Acceptor>,
    >,
    c_to_proposers: impl FnOnce(
        &Cluster<'a, Proposer>,
    ) -> Stream<P, Unbounded, NoTick, Cluster<'a, Proposer>>,
    f: usize,
    i_am_leader_send_timeout: u64,
    i_am_leader_check_timeout: u64,
    i_am_leader_check_timeout_delay_multiplier: usize,
) -> (
    Cluster<'a, Proposer>,
    Cluster<'a, Acceptor>,
    Stream<Ballot, Unbounded, NoTick, Cluster<'a, Proposer>>,
    Stream<(i32, P), Unbounded, NoTick, Cluster<'a, Proposer>>,
) {
    let proposers = flow.cluster::<Proposer>();
    let acceptors = flow.cluster::<Acceptor>();

    let c_to_proposers = c_to_proposers(&proposers);

    // Proposers.
    proposers
        .source_iter(q!(["Proposers say hello"]))
        .for_each(q!(|s| println!("{}", s)));
    let p_id = proposers.self_id();

    let (p_to_proposers_i_am_leader_complete_cycle, p_to_proposers_i_am_leader) =
        proposers.cycle::<Stream<_, _, _, _>>();
    let (a_to_proposers_p1b_complete_cycle, a_to_proposers_p1b) =
        proposers.cycle::<Stream<_, _, _, _>>();
    let (a_to_proposers_p2b_complete_cycle, a_to_proposers_p2b) =
        proposers.cycle::<Stream<_, _, _, _>>();
    // a_to_proposers_p2b.clone().for_each(q!(|(_, p2b): (u32, P2b)| println!("Proposer received P2b: {:?}", p2b)));
    // p_to_proposers_i_am_leader.clone().for_each(q!(|ballot: Ballot| println!("Proposer received I am leader: {:?}", ballot)));
    // c_to_proposers.clone().for_each(q!(|payload: ClientPayload| println!("Client sent proposer payload: {:?}", payload)));

    let p_received_max_ballot = p_max_ballot(
        &proposers,
        a_to_proposers_p1b.clone(),
        a_to_proposers_p2b.clone(),
        p_to_proposers_i_am_leader.clone(),
    );
    let (p_ballot_num, p_has_largest_ballot) =
        p_ballot_calc(&proposers, p_received_max_ballot.latest_tick());

    let (p_is_leader, p_log_to_try_commit, p_max_slot, p_log_holes) = p_p1b(
        &proposers,
        a_to_proposers_p1b.inspect(q!(|(_, p1b)| println!("Proposer received P1b: {:?}", p1b))),
        p_ballot_num.clone(),
        p_has_largest_ballot,
        f,
    );

    let (p_to_proposers_i_am_leader_from_others, p_to_acceptors_p1a) = p_p1a(
        p_ballot_num.clone(),
        p_is_leader.clone(),
        &proposers,
        p_to_proposers_i_am_leader,
        &acceptors,
        i_am_leader_send_timeout,
        i_am_leader_check_timeout,
        i_am_leader_check_timeout_delay_multiplier,
    );
    p_to_proposers_i_am_leader_complete_cycle.complete(p_to_proposers_i_am_leader_from_others);

    let (p_next_slot, p_to_acceptors_p2a) = p_p2a(
        &proposers,
        p_max_slot,
        c_to_proposers,
        p_ballot_num.clone(),
        p_log_to_try_commit,
        p_log_holes,
        p_is_leader.clone(),
        &acceptors,
    );

    // Tell clients that leader election has completed and they can begin sending messages
    let p_to_clients_new_leader_elected = p_is_leader.clone()
        .continue_unless(p_next_slot)
        .cross_singleton(p_ballot_num)
        .map(q!(move |(_is_leader, ballot_num)| Ballot { num: ballot_num, id: p_id})) // Only tell the clients once when leader election concludes
        .all_ticks();
    // End tell clients that leader election has completed
    let p_to_replicas = p_p2b(&proposers, a_to_proposers_p2b, f);

    // Acceptors.
    acceptors
        .source_iter(q!(["Acceptors say hello"]))
        .for_each(q!(|s| println!("{}", s)));
    let r_to_acceptors_checkpoint = r_to_acceptors_checkpoint(&acceptors);

    // p_to_acceptors_p2a.clone().for_each(q!(|p2a: P2a| println!("Acceptor received P2a: {:?}", p2a)));
    let (a_to_proposers_p1b_new, a_to_proposers_p2b_new) = acceptor(
        p_to_acceptors_p1a.inspect(q!(|p1a| println!("Acceptor received P1a: {:?}", p1a))),
        p_to_acceptors_p2a,
        r_to_acceptors_checkpoint,
        &proposers,
        &acceptors,
        f,
    );
    a_to_proposers_p1b_complete_cycle.complete(a_to_proposers_p1b_new);
    a_to_proposers_p2b_complete_cycle.complete(a_to_proposers_p2b_new);

    (
        proposers,
        acceptors,
        p_to_clients_new_leader_elected,
        p_to_replicas,
    )
}

#[expect(clippy::type_complexity, reason = "internal paxos code // TODO")]
fn acceptor<'a, P: PaxosPayload, R>(
    p_to_acceptors_p1a: Stream<P1a, Unbounded, NoTick, Cluster<'a, Acceptor>>,
    p_to_acceptors_p2a: Stream<P2a<P>, Unbounded, NoTick, Cluster<'a, Acceptor>>,
    r_to_acceptors_checkpoint: Stream<
        (ClusterId<R>, i32),
        Unbounded,
        NoTick,
        Cluster<'a, Acceptor>,
    >,
    proposers: &Cluster<'a, Proposer>,
    acceptors: &Cluster<'a, Acceptor>,
    f: usize,
) -> (
    Stream<(ClusterId<Acceptor>, P1b<P>), Unbounded, NoTick, Cluster<'a, Proposer>>,
    Stream<(ClusterId<Acceptor>, P2b<P>), Unbounded, NoTick, Cluster<'a, Proposer>>,
) {
    // Get the latest checkpoint sequence per replica
    let a_checkpoint_largest_seqs =
        r_to_acceptors_checkpoint
            .tick_prefix()
            .reduce_keyed(q!(|curr_seq, seq| {
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
        .unwrap_or(acceptors.singleton(q!(-1)).latest_tick())
        .delta()
        .map(q!(|min_seq| (
            min_seq,
            P2a {
                // Create tuple with checkpoint number and dummy p2a
                ballot: Ballot {
                    num: 0,
                    id: ClusterId::from_raw(0)
                },
                slot: -1,
                value: Default::default()
            }
        )));
    // .inspect(q!(|(min_seq, p2a): &(i32, P2a)| println!("Acceptor new checkpoint: {:?}", min_seq)));

    let a_max_ballot = p_to_acceptors_p1a
        .clone()
        .map(q!(|p1a| p1a.ballot))
        .max()
        .unwrap_or(acceptors.singleton(q!(Ballot {
            num: 0,
            id: ClusterId::from_raw(0)
        })));
    let a_p2as_to_place_in_log = p_to_acceptors_p2a
        .clone()
        .tick_batch()
        .cross_singleton(a_max_ballot.clone().latest_tick()) // Don't consider p2as if the current ballot is higher
        .filter_map(q!(|(p2a, max_ballot)|
            if p2a.ballot >= max_ballot {
                Some((-1, p2a)) // Signal that this isn't a checkpoint with -1
            } else {
                None
            }
        ));
    let a_log = a_p2as_to_place_in_log
        .union(a_new_checkpoint.into_stream())
        .persist()
        .fold(
            q!(|| (-1, HashMap::new())),
            q!(|(prev_checkpoint, log), (new_checkpoint, p2a)| {
                if new_checkpoint != -1 {
                    // This is a checkpoint message. Delete all entries up to the checkpoint
                    for slot in *prev_checkpoint..new_checkpoint {
                        log.remove(&slot);
                    }
                    *prev_checkpoint = new_checkpoint;
                } else {
                    // This is a regular p2a message. Insert it into the log if it is not checkpointed and has a higher ballot than what was there before
                    if p2a.slot > *prev_checkpoint {
                        match log.get(&p2a.slot) {
                            None => {
                                log.insert(
                                    p2a.slot,
                                    LogValue {
                                        ballot: p2a.ballot,
                                        value: p2a.value,
                                    },
                                );
                            }
                            Some(prev_p2a) => {
                                if p2a.ballot > prev_p2a.ballot {
                                    log.insert(
                                        p2a.slot,
                                        LogValue {
                                            ballot: p2a.ballot,
                                            value: p2a.value,
                                        },
                                    );
                                }
                            }
                        };
                    }
                }
            }),
        );

    let a_to_proposers_p1b_new = p_to_acceptors_p1a
        .tick_batch()
        .cross_singleton(a_max_ballot.clone().latest_tick())
        .cross_singleton(a_log)
        .map(q!(|((p1a, max_ballot), (_prev_checkpoint, log))| (
            p1a.ballot.id,
            P1b {
                ballot: p1a.ballot,
                max_ballot,
                accepted: log
            }
        )))
        .all_ticks()
        .send_bincode(proposers);
    let a_to_proposers_p2b_new = p_to_acceptors_p2a
        .tick_batch()
        .cross_singleton(a_max_ballot.latest_tick())
        .map(q!(|(p2a, max_ballot)| (
            p2a.ballot.id,
            P2b {
                ballot: p2a.ballot,
                max_ballot,
                slot: p2a.slot,
                value: p2a.value
            }
        )))
        .all_ticks()
        .send_bincode(proposers);
    (a_to_proposers_p1b_new, a_to_proposers_p2b_new)
}

#[expect(clippy::type_complexity, reason = "internal paxos code // TODO")]
fn p_p2b<'a, P: PaxosPayload>(
    proposers: &Cluster<'a, Proposer>,
    a_to_proposers_p2b: Stream<
        (ClusterId<Acceptor>, P2b<P>),
        Unbounded,
        NoTick,
        Cluster<'a, Proposer>,
    >,
    f: usize,
) -> Stream<(i32, P), Unbounded, NoTick, Cluster<'a, Proposer>> {
    let (p_broadcasted_p2b_slots_complete_cycle, p_broadcasted_p2b_slots) = proposers.tick_cycle();
    let (p_persisted_p2bs_complete_cycle, p_persisted_p2bs) = proposers.tick_cycle();
    let p_p2b = a_to_proposers_p2b
        .clone()
        .tick_batch()
        .union(p_persisted_p2bs);
    let p_count_matching_p2bs = p_p2b
        .clone()
        .filter_map(q!(|(sender, p2b)| if p2b.ballot == p2b.max_ballot {
            // Only consider p2bs where max ballot = ballot, which means that no one preempted us
            Some((p2b.slot, (sender, p2b)))
        } else {
            None
        }))
        .fold_keyed(
            q!(|| (
                0,
                P2b {
                    ballot: Ballot {
                        num: 0,
                        id: ClusterId::from_raw(0)
                    },
                    max_ballot: Ballot {
                        num: 0,
                        id: ClusterId::from_raw(0)
                    },
                    slot: 0,
                    value: Default::default()
                }
            )),
            q!(|accum, (_sender, p2b)| {
                accum.0 += 1;
                accum.1 = p2b;
            }),
        );
    let p_p2b_quorum_reached = p_count_matching_p2bs
        .clone()
        .filter(q!(move |(_slot, (count, _p2b))| *count > f));
    let p_to_replicas = p_p2b_quorum_reached
        .clone()
        .anti_join(p_broadcasted_p2b_slots) // Only tell the replicas about committed values once
        .map(q!(|(_slot, (_count, p2b))| (p2b.slot, p2b.value)))
        .all_ticks();

    let p_p2b_all_commit_slots =
        p_count_matching_p2bs
            .clone()
            .filter_map(q!(move |(slot, (count, _p2b))| if count == 2 * f + 1 {
                Some(slot)
            } else {
                None
            }));
    // p_p2b_all_commit_slots.inspect(q!(|slot: i32| println!("Proposer slot all received: {:?}", slot)));
    let p_broadcasted_p2b_slots_new = p_p2b_quorum_reached
        .clone()
        .map(q!(|(slot, (_count, _p2b))| slot))
        .filter_not_in(p_p2b_all_commit_slots.clone());
    // p_broadcasted_p2b_slots_new.inspect(q!(|slot: i32| println!("Proposer slot broadcasted: {:?}", slot)));
    p_broadcasted_p2b_slots_complete_cycle.complete_next_tick(p_broadcasted_p2b_slots_new);
    let p_persisted_p2bs_new = p_p2b
        .clone()
        .map(q!(|(sender, p2b)| (p2b.slot, (sender, p2b))))
        .anti_join(p_p2b_all_commit_slots.clone())
        .map(q!(|(_slot, (sender, p2b))| (sender, p2b)));
    // p_persisted_p2bs_new.inspect(q!(|(sender, p2b): (u32, P2b)| println!("Proposer persisting p2b: {:?}", p2b)));
    p_persisted_p2bs_complete_cycle.complete_next_tick(p_persisted_p2bs_new);
    p_to_replicas
}

// Proposer logic to send p2as, outputting the next slot and the p2as to send to acceptors.
#[expect(
    clippy::type_complexity,
    clippy::too_many_arguments,
    reason = "internal paxos code // TODO"
)]
fn p_p2a<'a, P: PaxosPayload>(
    proposers: &Cluster<'a, Proposer>,
    p_max_slot: Optional<i32, Bounded, Tick, Cluster<'a, Proposer>>,
    c_to_proposers: Stream<P, Unbounded, NoTick, Cluster<'a, Proposer>>,
    p_ballot_num: Singleton<u32, Bounded, Tick, Cluster<'a, Proposer>>,
    p_log_to_try_commit: Stream<P2a<P>, Bounded, Tick, Cluster<'a, Proposer>>,
    p_log_holes: Stream<P2a<P>, Bounded, Tick, Cluster<'a, Proposer>>,
    p_is_leader: Optional<bool, Bounded, Tick, Cluster<'a, Proposer>>,
    acceptors: &Cluster<'a, Acceptor>,
) -> (
    Optional<i32, Bounded, Tick, Cluster<'a, Proposer>>,
    Stream<P2a<P>, Unbounded, NoTick, Cluster<'a, Acceptor>>,
) {
    let p_id = proposers.self_id();
    let (p_next_slot_complete_cycle, p_next_slot) =
        proposers.tick_cycle::<Optional<i32, _, _, _>>();
    let p_next_slot_after_reconciling_p1bs = p_max_slot
        .unwrap_or(proposers.singleton(q!(-1)).latest_tick())
        // .inspect(q!(|max_slot| println!("{} p_max_slot: {:?}", context.current_tick(), max_slot)))
        .continue_unless(p_next_slot.clone())
        .map(q!(|max_slot| max_slot + 1));

    // Send p2as
    let p_indexed_payloads = c_to_proposers
        .clone()
        .tick_batch()
        .enumerate()
        .cross_singleton(p_next_slot.clone())
        // .inspect(q!(|next| println!("{} p_indexed_payloads next slot: {}", context.current_tick(), next))))
        .cross_singleton(p_ballot_num.clone())
        // .inspect(q!(|ballot_num| println!("{} p_indexed_payloads ballot_num: {}", context.current_tick(), ballot_num))))
        .map(q!(move |(((index, payload), next_slot), ballot_num)| P2a { ballot: Ballot { num: ballot_num, id: p_id }, slot: next_slot + index as i32, value: payload }));
    // .inspect(q!(|p2a: &P2a| println!("{} p_indexed_payloads P2a: {:?}", context.current_tick(), p2a)));
    let p_to_acceptors_p2a = p_log_to_try_commit
        .union(p_log_holes)
        .continue_unless(p_next_slot.clone()) // Only resend p1b stuff once. Once it's resent, next_slot will exist.
        .union(p_indexed_payloads)
        .continue_if(p_is_leader.clone())
        .all_ticks()
        .broadcast_bincode_interleaved(acceptors);

    let p_num_payloads = c_to_proposers.clone().tick_batch().count();
    let p_exists_payloads = p_num_payloads
        .clone()
        .filter(q!(|num_payloads| *num_payloads > 0));
    let p_next_slot_after_sending_payloads = p_num_payloads
        .continue_if(p_exists_payloads.clone())
        .clone()
        .cross_singleton(p_next_slot.clone())
        .map(q!(
            |(num_payloads, next_slot)| next_slot + num_payloads as i32
        ));
    let p_next_slot_if_no_payloads = p_next_slot.clone().continue_unless(p_exists_payloads);
    let p_new_next_slot_calculated = p_next_slot_after_reconciling_p1bs
        // .inspect(q!(|slot| println!("{} p_new_next_slot_after_reconciling_p1bs: {:?}", context.current_tick(), slot)))
        .union(p_next_slot_after_sending_payloads)
        // .inspect(q!(|slot| println!("{} p_next_slot_after_sending_payloads: {:?}", context.current_tick(), slot))))
        .union(p_next_slot_if_no_payloads)
        // .inspect(q!(|slot| println!("{} p_next_slot_if_no_payloads: {:?}", context.current_tick(), slot))))
        .continue_if(p_is_leader.clone());
    let p_new_next_slot_default = p_is_leader // Default next slot to 0 if there haven't been any payloads at all
        .clone()
        .continue_unless(p_new_next_slot_calculated.clone())
        .map(q!(|_| 0));
    // .inspect(q!(|slot| println!("{} p_new_next_slot_default: {:?}", context.current_tick(), slot)));
    let p_new_next_slot = p_new_next_slot_calculated.union(p_new_next_slot_default);
    p_next_slot_complete_cycle.complete_next_tick(p_new_next_slot);
    (p_next_slot, p_to_acceptors_p2a)
}

// Proposer logic for processing p1bs, determining if the proposer is now the leader, which uncommitted messages to commit, what the maximum slot is in the p1bs, and which no-ops to commit to fill log holes.
#[expect(clippy::type_complexity, reason = "internal paxos code // TODO")]
fn p_p1b<'a, P: PaxosPayload>(
    proposers: &Cluster<'a, Proposer>,
    a_to_proposers_p1b: Stream<
        (ClusterId<Acceptor>, P1b<P>),
        Unbounded,
        NoTick,
        Cluster<'a, Proposer>,
    >,
    p_ballot_num: Singleton<u32, Bounded, Tick, Cluster<'a, Proposer>>,
    p_has_largest_ballot: Optional<(Ballot, u32), Bounded, Tick, Cluster<'a, Proposer>>,
    f: usize,
) -> (
    Optional<bool, Bounded, Tick, Cluster<'a, Proposer>>,
    Stream<P2a<P>, Bounded, Tick, Cluster<'a, Proposer>>,
    Optional<i32, Bounded, Tick, Cluster<'a, Proposer>>,
    Stream<P2a<P>, Bounded, Tick, Cluster<'a, Proposer>>,
) {
    let p_id = proposers.self_id();
    let p_relevant_p1bs = a_to_proposers_p1b
        .clone()
        .tick_prefix()
        .cross_singleton(p_ballot_num.clone())
        .filter(q!(move |((_sender, p1b), ballot_num)| p1b.ballot
            == Ballot {
                num: *ballot_num,
                id: p_id
            }));
    let p_received_quorum_of_p1bs = p_relevant_p1bs
        .clone()
        .map(q!(|((sender, _p1b), _ballot_num)| { sender }))
        .unique()
        .count()
        .filter_map(q!(move |num_received| if num_received > f {
            Some(true)
        } else {
            None
        }));
    let p_is_leader = p_received_quorum_of_p1bs.continue_if(p_has_largest_ballot.clone());

    let p_p1b_highest_entries_and_count = p_relevant_p1bs
        .clone()
        .flat_map(q!(|((_, p1b), _)| p1b.accepted.into_iter())) // Convert HashMap log back to stream
        .fold_keyed(q!(|| (0, LogValue { ballot: Ballot { num: 0, id: ClusterId::from_raw(0) }, value: Default::default() })), q!(|curr_entry, new_entry| {
            let same_values = new_entry.value == curr_entry.1.value;
            let higher_ballot = new_entry.ballot > curr_entry.1.ballot;
            // Increment count if the values are the same
            if same_values {
                curr_entry.0 += 1;
            }
            // Replace the ballot with the largest one
            if higher_ballot {
                curr_entry.1.ballot = new_entry.ballot;
                // Replace the value with the one from the largest ballot, if necessary
                if !same_values {
                    curr_entry.0 = 1;
                    curr_entry.1.value = new_entry.value;
                }
            }
        }));
    let p_log_to_try_commit = p_p1b_highest_entries_and_count
        .clone()
        .cross_singleton(p_ballot_num.clone())
        .filter_map(q!(
            move |((slot, (count, entry)), ballot_num)| if count <= f as u32 {
                Some(P2a {
                    ballot: Ballot {
                        num: ballot_num,
                        id: p_id,
                    },
                    slot,
                    value: entry.value,
                })
            } else {
                None
            }
        ));
    let p_max_slot = p_p1b_highest_entries_and_count
        .clone()
        .map(q!(|(slot, _)| slot))
        .max();
    let p_proposed_slots = p_p1b_highest_entries_and_count
        .clone()
        .map(q!(|(slot, _)| slot));
    let p_log_holes = p_max_slot
        .clone()
        .flat_map(q!(|max_slot| 0..max_slot))
        .filter_not_in(p_proposed_slots)
        .cross_singleton(p_ballot_num.clone())
        .map(q!(move |(slot, ballot_num)| P2a {
            ballot: Ballot {
                num: ballot_num,
                id: p_id
            },
            slot,
            value: Default::default()
        }));
    (p_is_leader, p_log_to_try_commit, p_max_slot, p_log_holes)
}

// Proposer logic to calculate the largest ballot received so far.
#[expect(clippy::type_complexity, reason = "internal paxos code // TODO")]
fn p_max_ballot<'a, P: PaxosPayload>(
    proposers: &Cluster<'a, Proposer>,
    a_to_proposers_p1b: Stream<
        (ClusterId<Acceptor>, P1b<P>),
        Unbounded,
        NoTick,
        Cluster<'a, Proposer>,
    >,
    a_to_proposers_p2b: Stream<
        (ClusterId<Acceptor>, P2b<P>),
        Unbounded,
        NoTick,
        Cluster<'a, Proposer>,
    >,
    p_to_proposers_i_am_leader: Stream<Ballot, Unbounded, NoTick, Cluster<'a, Proposer>>,
) -> Singleton<Ballot, Unbounded, NoTick, Cluster<'a, Proposer>> {
    let p_received_p1b_ballots = a_to_proposers_p1b
        .clone()
        .map(q!(|(_, p1b)| p1b.max_ballot));
    let p_received_p2b_ballots = a_to_proposers_p2b
        .clone()
        .map(q!(|(_, p2b)| p2b.max_ballot));
    p_received_p1b_ballots
        .union(p_received_p2b_ballots)
        .union(p_to_proposers_i_am_leader)
        .max()
        .unwrap_or(proposers.singleton(q!(Ballot {
            num: 0,
            id: ClusterId::from_raw(0)
        })))
}

// Proposer logic to calculate the next ballot number. Expects p_received_max_ballot, the largest ballot received so far. Outputs streams: ballot_num, and has_largest_ballot, which only contains a value if we have the largest ballot.
#[expect(clippy::type_complexity, reason = "internal paxos code // TODO")]
fn p_ballot_calc<'a>(
    proposers: &Cluster<'a, Proposer>,
    p_received_max_ballot: Singleton<Ballot, Bounded, Tick, Cluster<'a, Proposer>>,
) -> (
    Singleton<u32, Bounded, Tick, Cluster<'a, Proposer>>,
    Optional<(Ballot, u32), Bounded, Tick, Cluster<'a, Proposer>>,
) {
    let p_id = proposers.self_id();
    let (p_ballot_num_complete_cycle, p_ballot_num) =
        proposers.tick_cycle_with_initial(proposers.singleton(q!(0)).latest_tick());

    let p_new_ballot_num = p_received_max_ballot
        .clone()
        .cross_singleton(p_ballot_num.clone())
        .map(q!(move |(received_max_ballot, ballot_num)| {
            if received_max_ballot
                > (Ballot {
                    num: ballot_num,
                    id: p_id,
                })
            {
                received_max_ballot.num + 1
            } else {
                ballot_num
            }
        }));
    p_ballot_num_complete_cycle.complete_next_tick(p_new_ballot_num);

    let p_has_largest_ballot = p_received_max_ballot
        .clone()
        .cross_singleton(p_ballot_num.clone())
        .filter(q!(
            move |(received_max_ballot, ballot_num)| *received_max_ballot
                <= Ballot {
                    num: *ballot_num,
                    id: p_id
                }
        ));

    // End stable leader election
    (p_ballot_num, p_has_largest_ballot)
}

// Proposer logic to send "I am leader" messages periodically to other proposers, or send p1a to acceptors if other leaders expired.
#[expect(
    clippy::too_many_arguments,
    clippy::type_complexity,
    reason = "internal paxos code // TODO"
)]
fn p_p1a<'a>(
    p_ballot_num: Singleton<u32, Bounded, Tick, Cluster<'a, Proposer>>,
    p_is_leader: Optional<bool, Bounded, Tick, Cluster<'a, Proposer>>,
    proposers: &Cluster<'a, Proposer>,
    p_to_proposers_i_am_leader: Stream<Ballot, Unbounded, NoTick, Cluster<'a, Proposer>>,
    acceptors: &Cluster<'a, Acceptor>,
    i_am_leader_send_timeout: u64,  // How often to heartbeat
    i_am_leader_check_timeout: u64, // How often to check if heartbeat expired
    i_am_leader_check_timeout_delay_multiplier: usize, /* Initial delay, multiplied by proposer pid, to stagger proposers checking for timeouts */
) -> (
    Stream<Ballot, Unbounded, NoTick, Cluster<'a, Proposer>>,
    Stream<P1a, Unbounded, NoTick, Cluster<'a, Acceptor>>,
) {
    let p_id = proposers.self_id();
    let p_to_proposers_i_am_leader_new = p_ballot_num
        .clone()
        .continue_if(
            proposers
                .source_interval(q!(Duration::from_secs(i_am_leader_send_timeout)))
                .latest_tick(),
        )
        .continue_if(p_is_leader.clone())
        .map(q!(move |ballot_num| Ballot {
            num: ballot_num,
            id: p_id
        }))
        .all_ticks()
        .broadcast_bincode_interleaved(proposers);

    let p_latest_received_i_am_leader = p_to_proposers_i_am_leader.clone().fold(
        q!(|| None),
        q!(|latest, _| {
            // Note: May want to check received ballot against our own?
            *latest = Some(Instant::now());
        }),
    );
    // Add random delay depending on node ID so not everyone sends p1a at the same time
    let p_leader_expired = proposers.source_interval_delayed(q!(Duration::from_secs((p_id.raw_id * i_am_leader_check_timeout_delay_multiplier as u32).into())), q!(Duration::from_secs(i_am_leader_check_timeout)))
        .cross_singleton(p_latest_received_i_am_leader.clone())
        .latest_tick()
        // .inspect(q!(|v| println!("Proposer checking if leader expired")))
        // .continue_if(p_is_leader.clone().count().filter(q!(|c| *c == 0)).inspect(q!(|c| println!("Proposer is_leader count: {}", c))))
        .continue_unless(p_is_leader)
        .filter(q!(move |(_, latest_received_i_am_leader)| {
            if let Some(latest_received_i_am_leader) = latest_received_i_am_leader {
                (Instant::now().duration_since(*latest_received_i_am_leader)) > Duration::from_secs(i_am_leader_check_timeout)
            } else {
                true
            }
        }));
    p_leader_expired
        .clone()
        .all_ticks()
        .for_each(q!(|_| println!("Proposer leader expired")));

    let p_to_acceptors_p1a = p_ballot_num
        .continue_if(p_leader_expired)
        .map(q!(move |ballot_num| P1a {
            ballot: Ballot {
                num: ballot_num,
                id: p_id
            }
        }))
        .all_ticks()
        .broadcast_bincode_interleaved(acceptors);
    (p_to_proposers_i_am_leader_new, p_to_acceptors_p1a)
}
