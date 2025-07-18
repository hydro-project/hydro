use std::fmt::Debug;
use std::hash::Hash;

use hydro_lang::*;
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
    let p_prepare = payloads.broadcast_bincode(participants);

    // participant 1 aborts transaction 1
    // TODO: Participants log
    let c_votes = p_prepare.send_bincode_anonymous(coordinator);

    // collect votes from participant.
    let coordinator_tick = coordinator.tick();
    let (c_all_vote_yes, _) = collect_quorum(
        c_votes
            .map(q!(|kv| (kv, Ok::<(), ()>(()))))
            .atomic(&coordinator_tick),
        num_participants,
        num_participants,
    );

    // TODO: Coordinator log

    // broadcast commit transactions to participants.
    let p_commit = c_all_vote_yes.broadcast_bincode(participants);
    // TODO: Participants log

    let c_commits = p_commit.send_bincode_anonymous(coordinator);
    let (c_all_commit, _) = collect_quorum(
        c_commits
            .map(q!(|kv| (kv, Ok::<(), ()>(()))))
            .atomic(&coordinator_tick),
        num_participants,
        num_participants,
    );
    // TODO: Coordinator log

    c_all_commit.end_atomic()
}
