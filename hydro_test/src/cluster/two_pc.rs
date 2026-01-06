use std::fmt::Debug;
use std::hash::Hash;

use hydro_lang::live_collections::stream::NoOrder;
use hydro_lang::prelude::*;
use hydro_std::quorum::collect_quorum;
use serde::Serialize;
use serde::de::DeserializeOwned;

pub struct Participant {}

pub struct Coordinator {}

pub fn two_pc<'a, Payload>(
    coordinator: &Process<'a, Coordinator>,
    participants: &Cluster<'a, Participant>,
    num_participants: usize,
    payloads: Stream<Payload, Process<'a, Coordinator>, Unbounded, NoOrder>,
) -> Stream<Payload, Process<'a, Coordinator>, Unbounded, NoOrder>
where
    Payload: Serialize + DeserializeOwned + Hash + Eq + Clone + Debug + Send,
{
    // TODO: Coordinator logs
    // broadcast prepare message to participants
    let p_prepare = payloads.ir_node_named("c_prepare").broadcast(
        participants,
        TCP.bincode(),
        nondet!(/** TODO */),
    );

    // participant 1 aborts transaction 1
    // TODO: Participants log
    let c_votes = p_prepare
        .ir_node_named("p_prepare")
        .send(coordinator, TCP.bincode())
        .ir_node_named("c_votes")
        .values();

    // collect votes from participant.
    let (c_all_vote_yes, _) = collect_quorum(
        c_votes.map(q!(|kv| (kv, Ok::<(), ()>(())))),
        num_participants,
        num_participants,
    );

    // TODO: Coordinator log

    // broadcast commit transactions to participants.
    let p_commit = c_all_vote_yes.broadcast(participants, TCP.bincode(), nondet!(/** TODO */));
    // TODO: Participants log

    let c_commits = p_commit
        .ir_node_named("p_commits")
        .send(coordinator, TCP.bincode())
        .ir_node_named("c_commits")
        .values();
    let (c_all_commit, _) = collect_quorum(
        c_commits.map(q!(|kv| (kv, Ok::<(), ()>(())))),
        num_participants,
        num_participants,
    );
    // TODO: Coordinator log

    c_all_commit
}
