//! An implementation of the RAFT consensus protocol for replicating a log across a
//! cluster.
//!
//! The protocol is split into two components, [`leader_election`] and
//! [`log_replication`], which [`raft_server`] runs back to back inside one dataflow tick.
//! Both take Tick-bounded inputs, so each member advances its election state and its
//! log against the same batch of inputs, in one sequential order (see the design notes
//! on [`raft`]).
//!
//! # Protocol mechanics
//!
//! Each member is a follower, a candidate, or the leader. A member whose election
//! timer fires becomes a candidate and requests votes, sending the index and term of
//! its last log entry; voters refuse candidates whose log is behind their own (RAFT
//! §5.4.1), which is what guarantees an elected leader holds every committed entry.
//!
//! The leader replicates by sending each follower `AppendEntries` messages carrying
//! new entries (empty for a pure heartbeat), the index and term of the entry
//! immediately preceding them, and the leader's commit index. The follower accepts
//! the entries iff its log contains a matching entry at that preceding position (the
//! log-matching property, RAFT §5.3), and advances its own commit index to
//! `min(leader_commit, index of last new entry)`. Acknowledgements report the highest
//! index known replicated on the follower; the leader advances its commit index once
//! a majority of members hold an entry of the current term, and steps down when any
//! reply carries a higher term than its own.

use std::collections::{HashMap, HashSet};
use std::fmt::Debug;
use std::marker::PhantomData;

use hydro_lang::forward_handle::ForwardHandle;
use hydro_lang::live_collections::stream::{ExactlyOnce, MinOrder, NoOrder, Ordering, TotalOrder};
use hydro_lang::location::cluster::{
    CLUSTER_SELF_ID, ClusterIds, Consistency, EventualConsistency,
};
use hydro_lang::location::dynamic::LocationId;
use hydro_lang::location::{Atomic, Cluster, Location, MemberId};
use hydro_lang::networking::NetworkFor;
use hydro_lang::prelude::*;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};

/// A RAFT server is either a follower, a candidate, or the leader
///
/// Public only because staged (`q!`) code must reference it by path; not stable API.
#[doc(hidden)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum RaftState {
    /// When in follower mode, a RAFT server receives `AppendEntries` RPCs and updates its log
    Follower,
    /// When in candidate mode, a RAFT server requests votes and is trying to become the leader
    Candidate,
    /// When leader, the RAFT server sends `AppendEntries` RPCs and serves client requests
    Leader,
}

/// A single entry in the replicated RAFT log.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct LogEntry<T> {
    /// The client payload carried by this entry.
    pub message: T,
    /// The leader term in which this entry was first appended (RAFT §5.3; used for the
    /// log-matching consistency check).
    pub term_received: usize,
    /// The 1-based position of this entry in the log.
    pub index: usize,
}

/// The full `AppendEntries` RPC payload (RAFT §5.3); see the module docs for the
/// acceptance and commit rules it drives.
///
/// `entries` is empty for pure heartbeats. The follower accepts the entries iff its log
/// contains an entry at `prev_log_index` with term `prev_log_term` (the log-matching
/// property), and advances its commit index to `min(leader_commit, last new entry)`.
///
/// Trait impls avoid derive bounds on `ClusterTag` (it only appears inside [`MemberId`],
/// which implements everything for any tag); serde bounds constrain only `T`.
///
/// Public only because staged (`q!`) code must reference it by path; not stable API.
#[doc(hidden)]
#[derive(Serialize, Deserialize)]
#[serde(bound(
    serialize = "T: Serialize",
    deserialize = "T: serde::de::DeserializeOwned"
))]
pub struct AppendEntriesRequest<T> {
    /// The leader's term.
    pub term: usize,
    /// Index of the log entry immediately preceding `entries`.
    pub prev_log_index: usize,
    /// Term of the entry at `prev_log_index`.
    pub prev_log_term: usize,
    /// New entries to append (empty for heartbeats).
    pub entries: Vec<LogEntry<T>>,
    /// The leader's commit index.
    pub leader_commit: usize,
}

impl<T: Clone> Clone for AppendEntriesRequest<T> {
    fn clone(&self) -> Self {
        AppendEntriesRequest {
            term: self.term,
            prev_log_index: self.prev_log_index,
            prev_log_term: self.prev_log_term,
            entries: self.entries.clone(),
            leader_commit: self.leader_commit,
        }
    }
}

impl<T: Debug> Debug for AppendEntriesRequest<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AppendEntriesRequest")
            .field("term", &self.term)
            .field("prev_log_index", &self.prev_log_index)
            .field("prev_log_term", &self.prev_log_term)
            .field("entries", &self.entries)
            .field("leader_commit", &self.leader_commit)
            .finish()
    }
}

/// Follower acknowledgement for an [`AppendEntriesRequest`] (RAFT §5.3).
///
/// The follower's identity is carried by the keyed intra-cluster channel, not the
/// payload, mirroring how vote traffic identifies senders in `leader_election`.
///
/// Public only because staged (`q!`) code must reference it by path; not stable API.
#[doc(hidden)]
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct AppendEntriesReply {
    /// The follower's current term (lets a deposed leader step down).
    pub term: usize,
    /// Whether the entries were accepted (the log-matching check passed).
    pub success: bool,
    /// The index of the highest log entry known to be replicated on the follower;
    /// meaningful when `success`, and lets the leader advance its commit index once a
    /// majority of `match_index`es reach an entry of the current term.
    pub match_index: usize,
}

/// The single wire format for all intra-cluster RAFT traffic. `raft_server` splits each
/// received batch by component: [`leader_election`] consumes the election traffic and
/// [`log_replication`] consumes the replication traffic of the same tick.
///
/// Public only because staged (`q!`) code must reference it by path; not stable API.
#[doc(hidden)]
#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(bound(
    serialize = "T: Serialize",
    deserialize = "T: serde::de::DeserializeOwned"
))]
pub enum RaftRpc<T> {
    /// Vote traffic, handled by [`leader_election`].
    Election(ElectionRpc),
    /// Log traffic, handled by [`log_replication`].
    Replication(ReplicationRpc<T>),
}

/// Vote traffic (RAFT §5.2).
///
/// Public only because staged (`q!`) code must reference it by path; not stable API.
#[doc(hidden)]
#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum ElectionRpc {
    /// Candidate -> everyone: request a vote for `term`.
    RequestVote(RequestVoteDto),
    /// Voter -> candidate: grant a vote for `term`.
    RequestVoteResponse(RequestVoteResponseDto),
}

/// Log traffic (RAFT §5.3).
///
/// Public only because staged (`q!`) code must reference it by path; not stable API.
#[doc(hidden)]
#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(bound(
    serialize = "T: Serialize",
    deserialize = "T: serde::de::DeserializeOwned"
))]
pub enum ReplicationRpc<T> {
    /// Leader -> follower: replicate entries / heartbeat.
    AppendEntries(AppendEntriesRequest<T>),
    /// Follower -> leader: acknowledge or reject an `AppendEntries`.
    AppendEntriesReply(AppendEntriesReply),
}

/// Candidate → everyone vote-request payload (RAFT §5.2).
///
/// Public only because staged (`q!`) code must reference it by path; not stable API.
#[doc(hidden)]
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct RequestVoteDto {
    /// The candidate's term.
    pub term: usize,
    /// Index of the candidate's last log entry (0 = empty log), for the RAFT §5.4.1
    /// up-to-dateness check: voters refuse candidates whose log is behind their own,
    /// which is what guarantees an elected leader holds every committed entry.
    pub last_log_index: usize,
    /// Term of the candidate's last log entry (0 = empty log).
    pub last_log_term: usize,
}

/// Voter → candidate vote-grant payload (RAFT §5.2).
///
/// Public only because staged (`q!`) code must reference it by path; not stable API.
#[doc(hidden)]
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct RequestVoteResponseDto {
    /// The term the vote is granted for.
    pub term: usize,
}

/// Configuration shared by [`raft`] and its components.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RaftConfig {
    /// The total number of members participating in the protocol. Majorities are
    /// computed as `cluster_size / 2 + 1`. Must match the deployed cluster size (in
    /// simulation tests, the value passed to `with_cluster_size`).
    pub cluster_size: usize,
}

/// A member's view of the election: its current term, and the leader of that term if
/// this member knows who it is (`Some(self)` after winning; `None` otherwise, until
/// an `AppendEntries` carrying the leader's identity arrives).
///
/// Trait impls are written manually (and serde bounds emptied) because the derives
/// would otherwise require `ClusterTag` itself to implement each trait, even though
/// the tag only appears inside [`MemberId`], which implements them for any tag.
#[derive(Serialize, Deserialize)]
#[serde(bound = "")]
pub struct LeaderView<ClusterTag> {
    /// The term this member currently believes is in effect.
    pub term: usize,
    /// The leader of `term`, if this member knows who it is.
    pub leader: Option<MemberId<ClusterTag>>,
}

impl<ClusterTag> Clone for LeaderView<ClusterTag> {
    fn clone(&self) -> Self {
        LeaderView {
            term: self.term,
            leader: self.leader.clone(),
        }
    }
}

impl<ClusterTag> Debug for LeaderView<ClusterTag> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LeaderView")
            .field("term", &self.term)
            .field("leader", &self.leader)
            .finish()
    }
}

impl<ClusterTag> PartialEq for LeaderView<ClusterTag> {
    fn eq(&self, other: &Self) -> bool {
        self.term == other.term && self.leader == other.leader
    }
}

impl<ClusterTag> Eq for LeaderView<ClusterTag> {}

/// Cluster tag for the RAFT replicas in this module's simulation tests.
///
/// Lives outside `#[cfg(test)]` because the simulator compiles staged (`q!`) code as a
/// separate crate, which must be able to resolve the tag's type path.
#[doc(hidden)]
pub struct Replica;

/// A member's election state (RAFT's `currentTerm`, `votedFor`, and role), persisted
/// across ticks by [`raft_server`].
///
/// [`leader_election`] advances it on vote traffic and election timer interrupts;
/// [`log_replication`] advances it on the terms and leaders revealed by replication
/// traffic.
///
/// Public only because staged (`q!`) code must reference it by path; not stable API.
#[doc(hidden)]
pub struct ElectionState<ClusterTag> {
    /// The latest term this member has observed (RAFT's `currentTerm`).
    pub term: usize,
    /// Who this member voted for in `term`, if anyone (itself, when a candidate).
    pub voted_for: Option<MemberId<ClusterTag>>,
    /// The member's current role.
    pub role: RaftState,
    /// The voters (including itself) that granted this member's candidacy for `term`.
    pub votes: HashSet<MemberId<ClusterTag>>,
    /// Whether an `AppendEntries` valid for the current term has been observed since
    /// the previous election timer interrupt (suppresses the next election).
    pub heartbeat_seen: bool,
    /// The leader of `term` as learned from received `AppendEntries`, if any. Not
    /// used while this member is itself the leader.
    pub known_leader: Option<MemberId<ClusterTag>>,
}

impl<ClusterTag> ElectionState<ClusterTag> {
    /// The initial state of a freshly booted member: a term-0 follower.
    pub fn new() -> Self {
        ElectionState {
            term: 0,
            voted_for: None,
            role: RaftState::Follower,
            votes: HashSet::new(),
            heartbeat_seen: false,
            known_leader: None,
        }
    }

    /// Adopts `term` if it is above the current term, which forces this member back to
    /// follower with all per-term state reset (RAFT §5.1). Returns whether `term` is
    /// the current term afterwards.
    pub fn observe_term(&mut self, term: usize) -> bool {
        if term > self.term {
            self.term = term;
            self.role = RaftState::Follower;
            self.voted_for = None;
            self.votes.clear();
            self.known_leader = None;
        }
        term == self.term
    }

    /// This member's view of the election, where `me` is this member's identity.
    pub fn view(&self, me: MemberId<ClusterTag>) -> LeaderView<ClusterTag> {
        LeaderView {
            term: self.term,
            leader: if self.role == RaftState::Leader {
                Some(me)
            } else {
                self.known_leader.clone()
            },
        }
    }
}

impl<ClusterTag> Default for ElectionState<ClusterTag> {
    fn default() -> Self {
        Self::new()
    }
}

impl<ClusterTag> Clone for ElectionState<ClusterTag> {
    fn clone(&self) -> Self {
        ElectionState {
            term: self.term,
            voted_for: self.voted_for.clone(),
            role: self.role,
            votes: self.votes.clone(),
            heartbeat_seen: self.heartbeat_seen,
            known_leader: self.known_leader.clone(),
        }
    }
}

/// A member's replication state (the log, its commit progress, and the leader's
/// per-follower bookkeeping), persisted across ticks by [`raft_server`] and advanced by
/// [`log_replication`].
///
/// Public only because staged (`q!`) code must reference it by path; not stable API.
#[doc(hidden)]
pub struct ReplicationState<T, ClusterTag> {
    /// The replicated log (1-based indexing via `LogEntry::index`).
    pub log: Vec<LogEntry<T>>,
    /// The highest log index known to be committed (0 = nothing committed).
    pub commit_index: usize,
    /// The highest log index already emitted on the `committed` output; used to emit
    /// each committed entry exactly once, in log order.
    pub emitted_index: usize,
    /// The term of the leadership that `next_index` and `match_index` describe. A
    /// leader whose term differs was just elected, so its bookkeeping is reset.
    pub leader_term: Option<usize>,
    /// Leader bookkeeping: the next log index to send to each follower.
    pub next_index: HashMap<MemberId<ClusterTag>, usize>,
    /// Leader bookkeeping: the highest log index known replicated on each follower,
    /// for the current leadership only (acks are term-filtered).
    pub match_index: HashMap<MemberId<ClusterTag>, usize>,
}

impl<T, ClusterTag> ReplicationState<T, ClusterTag> {
    /// The initial state of a freshly booted member: an empty log.
    pub fn new() -> Self {
        ReplicationState {
            log: Vec::new(),
            commit_index: 0,
            emitted_index: 0,
            leader_term: None,
            next_index: HashMap::new(),
            match_index: HashMap::new(),
        }
    }

    /// The (term, index) position of the last log entry, `(0, 0)` for an empty log —
    /// the two values compared by the §5.4.1 election restriction.
    pub fn last_log_position(&self) -> (usize, usize) {
        self.log
            .last()
            .map_or((0, 0), |entry| (entry.term_received, entry.index))
    }
}

impl<T, ClusterTag> Default for ReplicationState<T, ClusterTag> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: Clone, ClusterTag> Clone for ReplicationState<T, ClusterTag> {
    fn clone(&self) -> Self {
        ReplicationState {
            log: self.log.clone(),
            commit_index: self.commit_index,
            emitted_index: self.emitted_index,
            leader_term: self.leader_term,
            next_index: self.next_index.clone(),
            match_index: self.match_index.clone(),
        }
    }
}

/// One tick's worth of inputs to [`election_step`].
///
/// Public only because staged (`q!`) code must construct it by path; not stable API.
#[doc(hidden)]
pub struct ElectionStepInput<'m, ClusterTag> {
    /// This member's identity.
    pub me: MemberId<ClusterTag>,
    /// Every other member of the cluster (vote request broadcast targets).
    pub other_members: &'m [MemberId<ClusterTag>],
    /// Total protocol cluster size; majorities are `cluster_size / 2 + 1`.
    pub cluster_size: usize,
    /// Whether the election timer fired this tick.
    pub election_timer_fired: bool,
    /// The (term, index) position of this member's last log entry at the start of the
    /// tick, compared by the §5.4.1 election restriction and advertised in vote
    /// requests.
    pub last_log_position: (usize, usize),
    /// Vote traffic received this tick, as an unordered batch; the step sorts it
    /// canonically so its outcome depends only on the batch *multiset*.
    pub messages: Vec<(MemberId<ClusterTag>, ElectionRpc)>,
}

/// Advances one member's [`ElectionState`] by one tick: grants or refuses vote
/// requests, counts vote grants, and handles the election timer, returning the vote
/// traffic to send.
///
/// Messages are handled one at a time against the live state, after sorting the batch
/// into a canonical order, so the outcome is a function of the batch multiset
/// (required for deterministic simulation). The timer is handled after the messages.
///
/// Public (and callable outside staged code) so protocol-rule unit tests can drive
/// members directly and deterministically.
#[doc(hidden)]
pub fn election_step<ClusterTag>(
    state: &mut ElectionState<ClusterTag>,
    input: ElectionStepInput<'_, ClusterTag>,
) -> Vec<(MemberId<ClusterTag>, ElectionRpc)> {
    let ElectionStepInput {
        me,
        other_members,
        cluster_size,
        election_timer_fired,
        last_log_position,
        mut messages,
    } = input;
    let majority = cluster_size / 2 + 1;
    let mut outbound = Vec::new();

    // The leader's per-follower bookkeeping lives in `ReplicationState`;
    // `replication_step` resets it when it first runs under the new leadership.
    fn become_leader<ClusterTag>(state: &mut ElectionState<ClusterTag>) {
        state.role = RaftState::Leader;
        state.known_leader = None;
    }

    fn sort_key(rpc: &ElectionRpc) -> (u8, usize, usize, usize) {
        match rpc {
            ElectionRpc::RequestVote(dto) => (0, dto.term, dto.last_log_term, dto.last_log_index),
            ElectionRpc::RequestVoteResponse(dto) => (1, dto.term, 0, 0),
        }
    }
    messages.sort_by(|(sender_a, rpc_a), (sender_b, rpc_b)| {
        sort_key(rpc_a)
            .cmp(&sort_key(rpc_b))
            .then_with(|| sender_a.cmp(sender_b))
    });

    for (sender, message) in messages {
        match message {
            ElectionRpc::RequestVote(dto) => {
                if !state.observe_term(dto.term) {
                    // A stale-term candidacy: ignore (the candidate will observe the
                    // higher term from other traffic or retry at its next timeout).
                    continue;
                }
                // §5.4.1 election restriction: only grant if the candidate's log is
                // at least as up-to-date as ours. `last_log_position` includes every
                // entry this member acknowledged in earlier ticks.
                let candidate_up_to_date =
                    (dto.last_log_term, dto.last_log_index) >= last_log_position;
                let can_vote =
                    state.voted_for.is_none() || state.voted_for.as_ref() == Some(&sender);
                if candidate_up_to_date && can_vote {
                    state.voted_for = Some(sender.clone());
                    outbound.push((
                        sender,
                        ElectionRpc::RequestVoteResponse(RequestVoteResponseDto {
                            term: state.term,
                        }),
                    ));
                }
            }
            ElectionRpc::RequestVoteResponse(dto) => {
                if !state.observe_term(dto.term) {
                    continue;
                }
                if state.role == RaftState::Candidate {
                    state.votes.insert(sender);
                    if state.votes.len() >= majority {
                        become_leader(state);
                    }
                }
            }
        }
    }

    // Election timer interrupt. A leader runs no election timer. Otherwise: an
    // AppendEntries observed since the previous interrupt suppresses (and consumes
    // the window); if none, become a candidate for the next term (RAFT §5.2).
    if election_timer_fired && state.role != RaftState::Leader {
        if state.heartbeat_seen {
            state.heartbeat_seen = false;
        } else {
            state.term += 1;
            state.role = RaftState::Candidate;
            state.voted_for = Some(me.clone());
            state.votes.clear();
            state.votes.insert(me);
            state.known_leader = None;
            if state.votes.len() >= majority {
                // Single-member cluster: our own vote suffices.
                become_leader(state);
            } else {
                let (last_log_term, last_log_index) = last_log_position;
                for target in other_members {
                    outbound.push((
                        target.clone(),
                        ElectionRpc::RequestVote(RequestVoteDto {
                            term: state.term,
                            last_log_index,
                            last_log_term,
                        }),
                    ));
                }
            }
        }
    }

    outbound
}

/// One tick's worth of inputs to [`replication_step`].
///
/// Public only because staged (`q!`) code must construct it by path; not stable API.
#[doc(hidden)]
pub struct ReplicationStepInput<'m, T, ClusterTag> {
    /// Every other member of the cluster (heartbeat broadcast targets).
    pub other_members: &'m [MemberId<ClusterTag>],
    /// Total protocol cluster size; majorities are `cluster_size / 2 + 1`.
    pub cluster_size: usize,
    /// Whether the heartbeat timer fired this tick.
    pub heartbeat_timer_fired: bool,
    /// Client requests received this tick, in the (arbitrary but fixed) order the
    /// caller acknowledged as non-deterministic.
    pub requests: Vec<T>,
    /// Replication traffic received this tick, as an unordered batch; the step sorts
    /// it canonically so its outcome depends only on the batch *multiset*.
    pub messages: Vec<(MemberId<ClusterTag>, ReplicationRpc<T>)>,
}

/// One tick's worth of outputs from [`replication_step`].
///
/// Public only because staged (`q!`) code must destructure it by path; not stable API.
#[doc(hidden)]
pub struct ReplicationStepOutput<T, ClusterTag> {
    /// Replication traffic to send to specific members.
    pub outbound: Vec<(MemberId<ClusterTag>, ReplicationRpc<T>)>,
    /// Entries newly committed this tick, in log order (each index emitted exactly
    /// once over the member's lifetime).
    pub committed: Vec<LogEntry<T>>,
    /// Client requests received while not the leader, paired with the best-known
    /// leader hint (`None` if unknown).
    pub redirected: Vec<(T, Option<MemberId<ClusterTag>>)>,
}

/// Advances one member's [`ReplicationState`] by one tick: handles replication
/// traffic, appends or redirects client requests, advances the leader's commit index,
/// and handles the heartbeat timer, returning the traffic to send and the entries that
/// became committed.
///
/// `election` is the state [`election_step`] produced for the same tick, so a member
/// that just voted in a newer term rejects `AppendEntries` from older terms (§5.1).
/// Replication traffic carrying a higher term, or revealing the current term's leader,
/// updates `election` in turn.
///
/// Messages are handled one at a time against the live state, after sorting the batch
/// into a canonical order, so the outcome is a function of the batch multiset
/// (required for deterministic simulation).
///
/// Public (and callable outside staged code) so protocol-rule unit tests can drive
/// members directly and deterministically.
#[doc(hidden)]
pub fn replication_step<T: Clone, ClusterTag>(
    election: &mut ElectionState<ClusterTag>,
    state: &mut ReplicationState<T, ClusterTag>,
    input: ReplicationStepInput<'_, T, ClusterTag>,
) -> ReplicationStepOutput<T, ClusterTag> {
    let ReplicationStepInput {
        other_members,
        cluster_size,
        heartbeat_timer_fired,
        requests,
        mut messages,
    } = input;
    let majority = cluster_size / 2 + 1;

    let mut outbound = Vec::new();
    let mut committed = Vec::new();
    let mut redirected = Vec::new();

    // (a) A leader whose bookkeeping describes another term was just elected: every
    // follower starts at the end of the leader's log, with nothing known replicated.
    if election.role == RaftState::Leader && state.leader_term != Some(election.term) {
        state.leader_term = Some(election.term);
        state.next_index.clear();
        state.match_index.clear();
        for follower in other_members {
            state
                .next_index
                .insert(follower.clone(), state.log.len() + 1);
            state.match_index.insert(follower.clone(), 0);
        }
    }

    // (b) Process this tick's messages in a canonical order (so the step is a
    // function of the batch multiset), each one sequentially against live state.
    fn sort_key<T>(rpc: &ReplicationRpc<T>) -> (u8, usize, usize, usize) {
        match rpc {
            ReplicationRpc::AppendEntries(request) => (
                0,
                request.term,
                request.prev_log_index,
                request.entries.len(),
            ),
            ReplicationRpc::AppendEntriesReply(reply) => {
                (1, reply.term, reply.match_index, usize::from(reply.success))
            }
        }
    }
    messages.sort_by(|(sender_a, rpc_a), (sender_b, rpc_b)| {
        sort_key(rpc_a)
            .cmp(&sort_key(rpc_b))
            .then_with(|| sender_a.cmp(sender_b))
    });

    for (sender, message) in messages {
        match message {
            ReplicationRpc::AppendEntries(request) => {
                if !election.observe_term(request.term) {
                    // Stale leader: reject with our (higher) term so it steps down.
                    outbound.push((
                        sender,
                        ReplicationRpc::AppendEntriesReply(AppendEntriesReply {
                            term: election.term,
                            success: false,
                            match_index: 0,
                        }),
                    ));
                    continue;
                }
                // A current-term AppendEntries while we lead the same term would
                // mean two leaders share a term — election safety is broken.
                assert!(
                    election.role != RaftState::Leader,
                    "protocol violation: two leaders share term {}",
                    election.term,
                );
                // A live leader exists for the current term: suppress the next
                // election, depose our candidacy, learn the leader's identity.
                election.heartbeat_seen = true;
                if election.role == RaftState::Candidate {
                    election.role = RaftState::Follower;
                }
                election.known_leader = Some(sender.clone());

                // Log-matching check (RAFT §5.3).
                let log_matches = request.prev_log_index == 0
                    || (state.log.len() >= request.prev_log_index
                        && state.log[request.prev_log_index - 1].term_received
                            == request.prev_log_term);
                if !log_matches {
                    outbound.push((
                        sender,
                        ReplicationRpc::AppendEntriesReply(AppendEntriesReply {
                            term: election.term,
                            success: false,
                            match_index: 0,
                        }),
                    ));
                    continue;
                }

                // Append: truncate on conflict, skip already-present entries.
                let new_match = request.prev_log_index + request.entries.len();
                for entry in request.entries {
                    if state.log.len() >= entry.index {
                        if state.log[entry.index - 1].term_received != entry.term_received {
                            // A committed prefix can never legitimately conflict: the
                            // leader completeness property (enforced by the §5.4.1
                            // vote check in `election_step`) guarantees every leader's
                            // log contains all committed entries. Fail fast if this
                            // invariant is ever violated.
                            assert!(
                                entry.index > state.commit_index,
                                "protocol violation: asked to truncate committed \
                                 entry at index {} (commit_index {})",
                                entry.index,
                                state.commit_index,
                            );
                            state.log.truncate(entry.index - 1);
                            state.log.push(entry);
                        }
                    } else {
                        state.log.push(entry);
                    }
                }

                // Adopt the leader's commit index, capped at what this request
                // confirmed matches the leader's log.
                let commit_cap = request.leader_commit.min(new_match);
                if commit_cap > state.commit_index {
                    state.commit_index = commit_cap;
                }

                outbound.push((
                    sender,
                    ReplicationRpc::AppendEntriesReply(AppendEntriesReply {
                        term: election.term,
                        success: true,
                        match_index: new_match,
                    }),
                ));
            }
            ReplicationRpc::AppendEntriesReply(reply) => {
                if !election.observe_term(reply.term) {
                    // An ack for an earlier leadership: the follower's log may have
                    // been truncated since, so it must not count toward this term.
                    continue;
                }
                if election.role != RaftState::Leader {
                    continue;
                }
                if reply.success {
                    // Successful acknowledgements advance match/next monotonically.
                    let best = state.match_index.entry(sender.clone()).or_insert(0);
                    if reply.match_index > *best {
                        *best = reply.match_index;
                    }
                    let next = state.next_index.entry(sender).or_insert(1);
                    if reply.match_index + 1 > *next {
                        *next = reply.match_index + 1;
                    }
                } else {
                    // Rejections back off next_index to retry earlier prefixes.
                    if let Some(next) = state.next_index.get_mut(&sender) {
                        *next = (*next - 1).max(1);
                    }
                }
            }
        }
    }

    // (c) Client requests: the leader appends them in the caller-fixed order;
    // everyone else redirects them with the best-known leader hint.
    for message in requests {
        if election.role == RaftState::Leader {
            let index = state.log.len() + 1;
            state.log.push(LogEntry {
                message,
                term_received: election.term,
                index,
            });
        } else {
            redirected.push((message, election.known_leader.clone()));
        }
    }

    // (d) Advance the leader's commit index: the §5.4.2 rule — only entries of the
    // leader's own term commit by counting replicas; earlier-term entries commit
    // transitively beneath them.
    if election.role == RaftState::Leader {
        let mut candidate = state.log.len();
        while candidate > state.commit_index {
            if state.log[candidate - 1].term_received == election.term {
                // Count acks by iterating `other_members` (a fixed, deterministic
                // list) rather than the `match_index` map, whose iteration order is
                // nondeterministic.
                let acks = 1 + other_members
                    .iter()
                    .filter(|member| {
                        state
                            .match_index
                            .get(member)
                            .is_some_and(|match_index| *match_index >= candidate)
                    })
                    .count();
                if acks >= majority {
                    state.commit_index = candidate;
                    break;
                }
            }
            candidate -= 1;
        }
    }

    // (e) Heartbeat timer interrupt: the leader broadcasts AppendEntries carrying
    // each follower's outstanding suffix and the freshly advanced commit index.
    if heartbeat_timer_fired && election.role == RaftState::Leader {
        for follower in other_members {
            let next = state
                .next_index
                .get(follower)
                .copied()
                .unwrap_or(state.log.len() + 1);
            let prev_log_index = next - 1;
            let prev_log_term = if prev_log_index == 0 {
                0
            } else {
                state.log[prev_log_index - 1].term_received
            };
            outbound.push((
                follower.clone(),
                ReplicationRpc::AppendEntries(AppendEntriesRequest {
                    term: election.term,
                    prev_log_index,
                    prev_log_term,
                    entries: state.log[prev_log_index..].to_vec(),
                    leader_commit: state.commit_index,
                }),
            ));
        }
    }

    // (f) Emit newly committed entries exactly once, in log order.
    while state.emitted_index < state.commit_index {
        state.emitted_index += 1;
        committed.push(state.log[state.emitted_index - 1].clone());
    }

    ReplicationStepOutput {
        outbound,
        committed,
        redirected,
    }
}

/// RAFT leader election (§5.2): runs [`election_step`] on one tick's vote traffic and
/// election timer interrupts.
///
/// `state` is the member's election state at the start of the tick; the returned
/// singleton is that state advanced, alongside the vote traffic to send.
/// `last_log_position` must be the position of the member's last log entry at the
/// start of the tick: the §5.4.1 vote check then covers every entry the member
/// acknowledged in earlier ticks.
#[expect(
    clippy::type_complexity,
    reason = "Tick-bounded live collections spell out their full types"
)]
pub fn leader_election<'a, ClusterTag: 'a>(
    state: Singleton<ElectionState<ClusterTag>, Tick<Cluster<'a, ClusterTag>>, Bounded>,
    messages: Stream<
        (MemberId<ClusterTag>, ElectionRpc),
        Tick<Cluster<'a, ClusterTag>>,
        Bounded,
        NoOrder,
    >,
    election_timer_fired: Singleton<bool, Tick<Cluster<'a, ClusterTag>>, Bounded>,
    last_log_position: Singleton<(usize, usize), Tick<Cluster<'a, ClusterTag>>, Bounded>,
    other_members: Singleton<Vec<MemberId<ClusterTag>>, Tick<Cluster<'a, ClusterTag>>, Bounded>,
    config: RaftConfig,
) -> (
    Singleton<ElectionState<ClusterTag>, Tick<Cluster<'a, ClusterTag>>, Bounded>,
    Stream<(MemberId<ClusterTag>, ElectionRpc), Tick<Cluster<'a, ClusterTag>>, Bounded>,
) {
    let cluster_size = config.cluster_size;

    let message_vec = messages.fold(
        q!(|| Vec::new()),
        q!(
            |messages, message| {
                messages.push(message);
            },
            commutative = manual_proof!(
                /** election_step sorts the accumulated batch into a canonical order
                before applying it, so its result depends only on the batch multiset,
                never on arrival order */
            )
        ),
    );

    // The step advances `state` in place through a mutable reference handle. Handle
    // accesses run in code order, so the step sees every earlier write to `state` and
    // every later reader sees the step's writes. The step runs on the one-element
    // stream of the tick's message batch, whose output is the outbound traffic.
    let state_mut = state.by_mut();
    let fired_ref = election_timer_fired.by_ref();
    let position_ref = last_log_position.by_ref();
    let members_ref = other_members.by_ref();
    let outbound = message_vec
        .into_stream()
        .flat_map_ordered(q!(move |messages| {
            crate::cluster::raft::election_step(
                &mut *state_mut,
                ElectionStepInput {
                    me: CLUSTER_SELF_ID.clone(),
                    other_members: members_ref,
                    // Written without field shorthand: stageleft renames captured
                    // free variables, and shorthand hides the use site from it.
                    cluster_size: { cluster_size },
                    election_timer_fired: *fired_ref,
                    last_log_position: *position_ref,
                    messages,
                },
            )
        }));

    (state, outbound)
}

/// The Tick-bounded live collections produced by [`log_replication`].
pub struct LogReplicationOutputs<'a, T, ClusterTag> {
    /// The member's election state after this tick's replication traffic.
    pub election_state:
        Singleton<ElectionState<ClusterTag>, Tick<Cluster<'a, ClusterTag>>, Bounded>,
    /// The member's replication state after this tick.
    pub state: Singleton<ReplicationState<T, ClusterTag>, Tick<Cluster<'a, ClusterTag>>, Bounded>,
    /// Replication traffic to send.
    pub outbound:
        Stream<(MemberId<ClusterTag>, ReplicationRpc<T>), Tick<Cluster<'a, ClusterTag>>, Bounded>,
    /// Entries newly committed this tick, in log order.
    pub committed: Stream<LogEntry<T>, Tick<Cluster<'a, ClusterTag>>, Bounded>,
    /// Client requests received while not the leader, paired with the best-known
    /// leader hint (`None` when the member does not know who leads).
    pub redirected:
        Stream<(T, Option<MemberId<ClusterTag>>), Tick<Cluster<'a, ClusterTag>>, Bounded>,
}

/// RAFT log replication (§5.3): runs [`replication_step`] on one tick's replication
/// traffic, client requests, and heartbeat timer interrupts.
///
/// `election_state` must be the state [`leader_election`] returned for the same tick,
/// so replication runs under the term this tick's votes adopted. `state` is the
/// member's replication state at the start of the tick. The returned election state
/// carries the terms and leaders revealed by replication traffic; it is the state the
/// next tick's [`leader_election`] must start from.
pub fn log_replication<'a, T, ClusterTag>(
    election_state: Singleton<ElectionState<ClusterTag>, Tick<Cluster<'a, ClusterTag>>, Bounded>,
    state: Singleton<ReplicationState<T, ClusterTag>, Tick<Cluster<'a, ClusterTag>>, Bounded>,
    messages: Stream<
        (MemberId<ClusterTag>, ReplicationRpc<T>),
        Tick<Cluster<'a, ClusterTag>>,
        Bounded,
        NoOrder,
    >,
    requests: Stream<T, Tick<Cluster<'a, ClusterTag>>, Bounded, TotalOrder>,
    heartbeat_timer_fired: Singleton<bool, Tick<Cluster<'a, ClusterTag>>, Bounded>,
    other_members: Singleton<Vec<MemberId<ClusterTag>>, Tick<Cluster<'a, ClusterTag>>, Bounded>,
    config: RaftConfig,
) -> LogReplicationOutputs<'a, T, ClusterTag>
where
    T: Clone + 'a,
    ClusterTag: 'a,
{
    let cluster_size = config.cluster_size;
    let tick = messages.location().clone();

    let message_vec = messages.fold(
        q!(|| Vec::new()),
        q!(
            |messages, message| {
                messages.push(message);
            },
            commutative = manual_proof!(
                /** replication_step sorts the accumulated batch into a canonical
                order before applying it, so its result depends only on the batch
                multiset, never on arrival order */
            )
        ),
    );
    // Requests in the order fixed by the caller (TotalOrder fold).
    let request_vec = requests.fold(
        q!(|| Vec::new()),
        q!(|requests, request| {
            requests.push(request);
        }),
    );

    let committed: Stream<LogEntry<T>, _, Bounded> = tick.source_iter(q!(Vec::new()));
    let redirected: Stream<(T, Option<MemberId<ClusterTag>>), _, Bounded> =
        tick.source_iter(q!(Vec::new()));

    // The step advances both states in place and fills `committed` and `redirected`
    // through mutable reference handles, ordered as in `leader_election`.
    let election_mut = election_state.by_mut();
    let state_mut = state.by_mut();
    let fired_ref = heartbeat_timer_fired.by_ref();
    let members_ref = other_members.by_ref();
    let committed_mut = committed.by_mut();
    let redirected_mut = redirected.by_mut();
    let outbound = message_vec
        .zip(request_vec)
        .into_stream()
        .flat_map_ordered(q!(move |(messages, requests)| {
            let output = crate::cluster::raft::replication_step(
                &mut *election_mut,
                &mut *state_mut,
                ReplicationStepInput {
                    other_members: members_ref,
                    // Written without field shorthand: stageleft renames captured
                    // free variables, and shorthand hides the use site from it.
                    cluster_size: { cluster_size },
                    heartbeat_timer_fired: *fired_ref,
                    requests,
                    messages,
                },
            );
            committed_mut.extend(output.committed);
            redirected_mut.extend(output.redirected);
            output.outbound
        }));

    LogReplicationOutputs {
        election_state,
        state,
        outbound,
        committed,
        redirected,
    }
}

/// The streams produced by [`raft_server`].
pub struct RaftOutputs<'a, T, ClusterTag> {
    /// The committed log entries, emitted on every member in log order. This stream
    /// is eventually consistent: all members observe the same sequence, though at
    /// different times. It is `Atomic` so that user actions taken on the commit path
    /// are completed before the protocol treats the entries as committed.
    pub committed: Stream<
        LogEntry<T>,
        Atomic<Cluster<'a, ClusterTag, u32, EventualConsistency>>,
        Unbounded,
        TotalOrder,
    >,
    /// Requests that arrived at a member that was not the leader, paired with the
    /// best-known leader hint for the client to retry against (`None` when the
    /// member does not know who leads).
    pub redirected:
        Stream<(T, Option<MemberId<ClusterTag>>), Cluster<'a, ClusterTag>, Unbounded, TotalOrder>,
    /// Each member's view transitions (term and known leader), emitted whenever the
    /// view changes. Useful for tests and observability; may be left unobserved.
    pub leader_views: Stream<LeaderView<ClusterTag>, Cluster<'a, ClusterTag>>,
}

/// RAFT on every cluster member: [`leader_election`] followed by [`log_replication`]
/// inside a single tick that batches every input the protocol reacts to (client
/// requests, both timers, and all intra-cluster traffic over one channel).
///
/// See [`raft`] for the input/timer contracts and why both components share a tick.
/// Each member's state feeds its own next tick through `sliced!` state; the only loop
/// between members — outbound messages reaching other members' inputs — passes through
/// the network demux, an asynchronous boundary.
pub fn raft_server<'a, T, ClusterTag, O, Net>(
    cluster: &Cluster<'a, ClusterTag>,
    requests: Stream<T, Cluster<'a, ClusterTag>, Unbounded, O>,
    election_timer_interrupts: Stream<(), Cluster<'a, ClusterTag>>,
    heartbeat_timer_interrupts: Stream<(), Cluster<'a, ClusterTag>>,
    config: RaftConfig,
    net: Net,
    nondet_raft: NonDet,
) -> RaftOutputs<'a, T, ClusterTag>
where
    T: Clone + Serialize + DeserializeOwned + 'a,
    ClusterTag: 'a,
    O: Ordering,
    Net: NetworkFor<RaftRpc<T>, RetriesGuarantee = ExactlyOnce>,
    NoOrder: MinOrder<Net::OrderingGuarantee, Min = NoOrder>,
{
    // Fix an arbitrary total order for incoming requests up front (a no-op when the
    // input is already `TotalOrder`): the leader must place concurrent requests at
    // *some* log indexes, and which order it picks is exactly the non-determinism
    // acknowledged by `nondet_raft`.
    let requests = requests.assume_ordering::<TotalOrder>(nondet!(
        /// The arrival order of concurrent client requests at the leader determines
        /// their log order, which is inherently non-deterministic.
        nondet_raft
    ));

    // All intra-cluster traffic flows over a single channel. The forward_ref closes
    // the loop: the sliced! block below both consumes received messages and produces
    // the outbound messages that feed the channel (through the network — an async
    // boundary, so this is not an in-tick cycle).
    #[expect(clippy::type_complexity, reason = "forward_ref requires the full type")]
    let (traffic_handle, traffic): (
        ForwardHandle<
            'a,
            Stream<(MemberId<ClusterTag>, RaftRpc<T>), Cluster<'a, ClusterTag>, Unbounded, NoOrder>,
        >,
        Stream<(MemberId<ClusterTag>, RaftRpc<T>), Cluster<'a, ClusterTag>, Unbounded, NoOrder>,
    ) = cluster.forward_ref();

    // The cluster membership list, resolved on each member at runtime (identical
    // everywhere, since it comes from deploy/sim metadata).
    let LocationId::Cluster(cluster_key) = Location::id(cluster) else {
        unreachable!("raft_server always runs on a cluster")
    };
    let cluster_members = ClusterIds::<u32> {
        key: cluster_key,
        _phantom: PhantomData,
    };

    #[expect(
        clippy::type_complexity,
        reason = "the sliced! outputs are annotated with their full stream types"
    )]
    let (outbound_messages, committed, redirected, view_transitions): (
        Stream<(MemberId<ClusterTag>, RaftRpc<T>), Cluster<'a, ClusterTag>>,
        Stream<LogEntry<T>, Cluster<'a, ClusterTag>>,
        Stream<(T, Option<MemberId<ClusterTag>>), Cluster<'a, ClusterTag>>,
        Stream<LeaderView<ClusterTag>, Cluster<'a, ClusterTag>>,
    ) = sliced! {
        let request_batch = use::batch(requests, nondet!(
            /// Which requests are batched together only affects which log indexes the
            /// leader assigns them, folded into the arbitrary order fixed above.
            nondet_raft
        ));
        let election_batch = use::batch(election_timer_interrupts, nondet!(
            /// When timer interrupts are processed relative to messages only affects
            /// which elections are attempted, which folds into the non-determinism of
            /// which member wins an election.
            nondet_raft
        ));
        let heartbeat_batch = use::batch(heartbeat_timer_interrupts, nondet!(
            /// Heartbeat timing only affects when replication progress is made, not
            /// the committed sequence.
            nondet_raft
        ));
        let traffic_batch = use::batch(traffic, nondet!(
            /// Message delivery interleavings shift which member wins elections and
            /// when entries replicate and commit, but never the committed sequence
            /// itself: both components process each tick's batch against the
            /// member's full state, one after the other.
            nondet_raft
        ));

        let mut election_state = use::state(|l| l.singleton(q!(ElectionState::new())));
        let mut replication_state = use::state(|l| l.singleton(q!(ReplicationState::new())));

        let tick = request_batch.location().clone();
        let election_fired = election_batch.count().map(q!(|n| n > 0));
        let heartbeat_fired = heartbeat_batch.count().map(q!(|n| n > 0));

        // The other members of the cluster, the broadcast targets.
        let other_members = tick.singleton(q!(
            cluster_members
                .iter()
                .map(|id| MemberId::from_tagless(id.clone()))
                .filter(|member| *member != CLUSTER_SELF_ID)
                .collect::<Vec<_>>()
        ));

        // Route each received message to the component that handles it.
        let (election_traffic, replication_traffic) =
            traffic_batch.partition(q!(|(_, rpc)| matches!(rpc, RaftRpc::Election(_))));
        let election_messages = election_traffic.filter_map(q!(|(sender, rpc)| match rpc {
            RaftRpc::Election(rpc) => Some((sender, rpc)),
            RaftRpc::Replication(_) => None,
        }));
        let replication_messages = replication_traffic.filter_map(q!(|(sender, rpc)| match rpc {
            RaftRpc::Replication(rpc) => Some((sender, rpc)),
            RaftRpc::Election(_) => None,
        }));

        // Start-of-tick reads. Reference handle accesses run in code order, so these
        // read the state before either component advances it.
        let election_start = election_state.by_ref();
        let replication_start = replication_state.by_ref();
        let view_before = tick
            .singleton(q!(()))
            .into_stream()
            .map(q!(move |_| election_start.view(CLUSTER_SELF_ID.clone())));
        // A one-element stream, so `first` always holds its value.
        let last_log_position = tick
            .singleton(q!(()))
            .into_stream()
            .map(q!(move |_| replication_start.last_log_position()))
            .first()
            .unwrap_or_default();

        let (elected_state, election_outbound) = leader_election(
            election_state,
            election_messages,
            election_fired,
            last_log_position,
            other_members.clone(),
            config,
        );
        let replication = log_replication(
            elected_state,
            replication_state,
            replication_messages,
            request_batch,
            heartbeat_fired,
            other_members,
            config,
        );
        election_state = replication.election_state;
        replication_state = replication.state;

        // Report a view transition iff the view changed this tick.
        let election_end = election_state.by_ref();
        let view_transitions = view_before.filter_map(q!(move |before| {
            let after = election_end.view(CLUSTER_SELF_ID.clone());
            (after != before).then_some(after)
        }));

        let outbound = election_outbound
            .map(q!(|(target, rpc)| (target, RaftRpc::Election(rpc))))
            .chain(
                replication
                    .outbound
                    .map(q!(|(target, rpc)| (target, RaftRpc::Replication(rpc)))),
            );

        (outbound, replication.committed, replication.redirected, view_transitions)
    };

    traffic_handle.complete(
        outbound_messages
            .into_keyed()
            // The channel's fault model is chosen by the caller: sim tests use
            // `TCP.fail_stop()` (the simulator cannot explore lossy channels without
            // giving up liveness assertions), while real deployments that face
            // partitions pass `TCP.lossy_delayed_forever()` — RAFT tolerates the
            // loss: lost AppendEntries (or replies) are re-sent on the next
            // heartbeat, and lost vote traffic is retried at the next election
            // timeout.
            .demux(cluster, net)
            .entries(),
    );

    RaftOutputs {
        // The RAFT protocol itself is what justifies the consistency cast: every
        // member emits the same totally-ordered committed sequence, eventually.
        committed: committed
            .assert_has_consistency_of::<Cluster<'a, ClusterTag, u32, EventualConsistency>>(
                manual_proof!(
                    /** RAFT's election restriction, log-matching, and majority-commit
                    rules guarantee every member emits the same committed sequence,
                    eventually */
                ),
            )
            .atomic(),
        redirected,
        leader_views: view_transitions,
    }
}

/// Given a non-consistent stream of events on a Cluster, `raft` will merge them to
/// produce an eventually consistent stream containing the union of the events from the
/// `requests` stream across all members of the Cluster.
///
/// The function returns the RAFT log stream, *and* a stream of requests that need to be
/// redirected to the leader (paired with the best-known leader hint, `None` while the
/// receiving member does not know who leads — e.g. before the leader's first heartbeat
/// reaches it; callers wanting a usable hint can hold these themselves).
/// The log stream carries full [`LogEntry`]s — the `term` and `index` metadata lets
/// consumers deduplicate across leader changes. It is `Atomic` so that user actions
/// taken on the commit path complete before the protocol treats entries as committed.
///
/// # Design: two components, one tick
///
/// RAFT's safety proofs interlock vote decisions and log/ack decisions through shared
/// state (`currentTerm`, `votedFor`, the log): a voter must compare candidates against
/// a log that includes every entry it acknowledged (§5.4.1), and a member that voted
/// in a term must reject `AppendEntries` from older terms (§5.1). Each member
/// therefore runs [`leader_election`] and then [`log_replication`] on the same tick's
/// batch, threading the state through both:
///
/// * [`leader_election`] reads the log position the previous tick's replication left;
/// * [`log_replication`] runs under the term [`leader_election`] just adopted;
/// * the election state [`log_replication`] returns starts the next tick's
///   [`leader_election`].
///
/// Every member's state transitions thus happen in one sequential order. Running the
/// components in separate ticks connected by deferred feedback breaks the interlock:
/// each component then acts on a stale view of the other, and an entry can commit
/// through acks from members that simultaneously elect a leader missing it. The
/// regression tests `concurrent_elections_never_fork_the_committed_log` and
/// `fully_concurrent_run_never_forks_the_committed_log` explore those schedules.
///
/// # Timers
///
/// The two timer streams are inputs so production controls the periods and tests
/// control time itself:
/// * `election_timer_interrupts`: fire per member; production must randomize the
///   period per member (RAFT's tie-breaking).
/// * `heartbeat_timer_interrupts`: fire per member with a period much shorter than the
///   election timeout, so a live leader's heartbeats arrive between election
///   interrupts and suppress them.
/// * `net`: builds the fault model for the intra-cluster channel. Simulation tests
///   pass `|| TCP.fail_stop().bincode()`; deployments facing partitions pass
///   `|| TCP.lossy_delayed_forever().bincode()`, which RAFT is designed to
///   tolerate.
#[expect(
    clippy::type_complexity,
    reason = "the return type spells out the exact consistency/ordering guarantees"
)]
pub fn raft<'a, T, ClusterTag, Con, O, Net>(
    requests: Stream<T, Cluster<'a, ClusterTag, u32, Con>, Unbounded, O>,
    election_timer_interrupts: Stream<(), Cluster<'a, ClusterTag>>,
    heartbeat_timer_interrupts: Stream<(), Cluster<'a, ClusterTag>>,
    config: RaftConfig,
    net: impl Fn() -> Net,
    nondet_order: NonDet,
) -> (
    Stream<
        LogEntry<T>,
        Atomic<Cluster<'a, ClusterTag, u32, EventualConsistency>>,
        Unbounded,
        TotalOrder,
    >,
    Stream<(T, Option<MemberId<ClusterTag>>), Cluster<'a, ClusterTag>, Unbounded, TotalOrder>,
)
where
    T: Clone + Serialize + DeserializeOwned + 'a,
    O: Ordering,
    Con: Consistency,
    ClusterTag: 'a,
    Net: NetworkFor<RaftRpc<T>, RetriesGuarantee = ExactlyOnce>,
    NoOrder: MinOrder<Net::OrderingGuarantee, Min = NoOrder>,
{
    // The server runs on the consistency-less view of the cluster: `Cluster`'s
    // default consistency parameter is `NoConsistency`, so `drop_consistency` is the
    // honest (and identity-shaped) conversion from the caller's `Con`-tagged handle.
    let cluster = requests.location().drop_consistency();

    let outputs = raft_server(
        &cluster,
        requests.weaken_consistency(),
        election_timer_interrupts,
        heartbeat_timer_interrupts,
        config,
        net(),
        nondet!(
            /// Which member leads, how concurrent requests are interleaved in the
            /// log, and when entries replicate fold into the caller's acknowledged
            /// non-determinism; the committed sequence itself is consistent.
            nondet_order
        ),
    );

    (outputs.committed, outputs.redirected)
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use hydro_lang::live_collections::stream::{ExactlyOnce, TotalOrder};
    use hydro_lang::location::MemberId;
    use hydro_lang::prelude::*;

    use super::{
        AppendEntriesReply, AppendEntriesRequest, ElectionRpc, ElectionState, ElectionStepInput,
        LeaderView, LogEntry, RaftConfig, RaftRpc, RaftState, Replica, ReplicationRpc,
        ReplicationState, ReplicationStepInput, RequestVoteDto, election_step, raft, raft_server,
        replication_step,
    };

    /// One member's complete protocol state, as `raft_server` persists it across ticks.
    struct MemberState {
        election: ElectionState<Replica>,
        replication: ReplicationState<String, Replica>,
    }

    impl MemberState {
        fn new() -> Self {
            MemberState {
                election: ElectionState::new(),
                replication: ReplicationState::new(),
            }
        }
    }

    /// One tick's worth of inputs to [`raft_step`].
    struct StepInput {
        me: MemberId<Replica>,
        other_members: Vec<MemberId<Replica>>,
        cluster_size: usize,
        election_timer_fired: bool,
        heartbeat_timer_fired: bool,
        requests: Vec<String>,
        messages: Vec<(MemberId<Replica>, RaftRpc<String>)>,
    }

    /// One tick's worth of outputs from [`raft_step`].
    struct StepOutput {
        outbound: Vec<(MemberId<Replica>, RaftRpc<String>)>,
        committed: Vec<LogEntry<String>>,
        redirected: Vec<(String, Option<MemberId<Replica>>)>,
    }

    /// Advances one member by one tick the way `raft_server` composes the components:
    /// [`election_step`] against the start-of-tick log position, then
    /// [`replication_step`] against the election state it produced.
    fn raft_step(state: &mut MemberState, input: StepInput) -> StepOutput {
        let mut election_messages = Vec::new();
        let mut replication_messages = Vec::new();
        for (sender, rpc) in input.messages {
            match rpc {
                RaftRpc::Election(rpc) => election_messages.push((sender, rpc)),
                RaftRpc::Replication(rpc) => replication_messages.push((sender, rpc)),
            }
        }
        let election_outbound = election_step(
            &mut state.election,
            ElectionStepInput {
                me: input.me,
                other_members: &input.other_members,
                cluster_size: input.cluster_size,
                election_timer_fired: input.election_timer_fired,
                last_log_position: state.replication.last_log_position(),
                messages: election_messages,
            },
        );
        let replication = replication_step(
            &mut state.election,
            &mut state.replication,
            ReplicationStepInput {
                other_members: &input.other_members,
                cluster_size: input.cluster_size,
                heartbeat_timer_fired: input.heartbeat_timer_fired,
                requests: input.requests,
                messages: replication_messages,
            },
        );
        StepOutput {
            outbound: election_outbound
                .into_iter()
                .map(|(target, rpc)| (target, RaftRpc::Election(rpc)))
                .chain(
                    replication
                        .outbound
                        .into_iter()
                        .map(|(target, rpc)| (target, RaftRpc::Replication(rpc))),
                )
                .collect(),
            committed: replication.committed,
            redirected: replication.redirected,
        }
    }

    const CLUSTER_SIZE: usize = 4;

    /// An even-sized cluster (4 members) where two members' election timers fire
    /// simultaneously. In every explored execution (fuzzed simulation; run under
    /// `cargo sim` for coverage-guided exploration, or `cargo test` for randomized
    /// iterations) we require:
    ///
    /// 1. **Exactly one leader per claimed term**: no term may ever be claimed by two
    ///    members' leader views. This catches bugs like a voter granting multiple votes
    ///    in the same term, or a quorum of `n / 2` instead of `n / 2 + 1` — with 4
    ///    members, a 2-2 split vote reaches a bogus quorum of 2 on *both* sides,
    ///    electing two leaders for the same term.
    /// 2. **Some leader is eventually elected**: a term with two simultaneous candidates
    ///    may legitimately elect *nobody* (the two non-candidate voters split 1-1 and
    ///    both candidates stall below the majority of 3), so after the first round
    ///    settles, the test re-fires only member 0's timer — reproducing RAFT's
    ///    randomized-timeout tie-breaking. That retry is uncontested, so every execution
    ///    must settle with member 0 as leader. This rejects trivial implementations that
    ///    satisfy (1) by never electing anyone.
    /// 3. Members whose timers never fire (2 and 3) never claim leadership.
    #[test]
    fn even_cluster_simultaneous_candidates_exactly_one_leader_per_term() {
        let mut flow = FlowBuilder::new();
        let cluster = flow.cluster::<Replica, u32>();

        let (interrupt_send, election_timer_interrupts) = cluster.sim_input();
        // The heartbeat timer never fires and no requests arrive: every election
        // interrupt triggers a real candidacy (nothing suppresses it), and all logs
        // stay empty so the §5.4.1 check is trivially satisfied for all candidates.
        let (_heartbeat_send, heartbeat_timer_interrupts) = cluster.sim_input();
        let (_request_send, requests) = cluster.sim_input::<String, TotalOrder, ExactlyOnce>();
        let outputs = raft_server(
            &cluster,
            requests,
            election_timer_interrupts,
            heartbeat_timer_interrupts,
            RaftConfig {
                cluster_size: CLUSTER_SIZE,
            },
            TCP.fail_stop().bincode(),
            nondet!(/** which member wins an election is inherently non-deterministic */),
        );

        // The transitions stream is a complete, exactly-once, in-order history of each
        // member's view — directly observable, unlike a Singleton whose snapshot-based
        // observation may skip or repeat intermediate views.
        let view_recv = outputs.leader_views.sim_cluster_output();

        // The committed stream carries a consistency assertion (see raft_server); the
        // simulator cannot validate those yet and asks tests to skip them.
        flow.sim()
            .skip_consistency_assertions()
            .with_cluster_size(&cluster, CLUSTER_SIZE)
            .fuzz(async || {
                let mut transitions: Vec<Vec<LeaderView<Replica>>> = vec![Vec::new(); CLUSTER_SIZE];

                // Round 1: members 0 and 1 become candidates simultaneously.
                interrupt_send.send(0, ());
                interrupt_send.send(1, ());

                // Phase barrier: let all vote traffic settle so the collected histories
                // are complete before the retry is fired.
                hydro_lang::sim::quiesce().await;
                for member in 0..CLUSTER_SIZE as u32 {
                    transitions[member as usize].extend(view_recv.collect::<Vec<_>>(member).await);
                }

                // Round 2: only member 0's timer fires (staggered retry). If round 1
                // ended in a split vote, member 0 must now win a fresh term outright.
                interrupt_send.send(0, ());

                hydro_lang::sim::quiesce().await;
                for member in 0..CLUSTER_SIZE as u32 {
                    transitions[member as usize].extend(view_recv.collect::<Vec<_>>(member).await);
                }

                // (1) Exactly one leader per claimed term: walk every transition in
                // which a member names *itself* leader, recording term -> member and
                // asserting the term was unclaimed. This also catches a member
                // re-claiming the same term twice, which the transitions contract
                // forbids. (3) Only the members whose timers fired may claim at all.
                let mut leader_by_term: HashMap<usize, usize> = HashMap::new();
                for (member, member_transitions) in transitions.iter().enumerate() {
                    let me = MemberId::<Replica>::from_raw_id(member as u32);
                    for view in member_transitions {
                        if view.leader.as_ref() == Some(&me) {
                            assert!(
                                member <= 1,
                                "member {} claimed leadership of term {} without its timer firing",
                                member,
                                view.term
                            );
                            if let Some(previous) = leader_by_term.insert(view.term, member) {
                                panic!(
                                    "term {} was claimed by both member {} and member {}",
                                    view.term, previous, member
                                );
                            }
                        }
                    }
                }

                // (2) The uncontested retry guarantees member 0 settles as leader in
                // every execution (either it was already the leader, or it wins a term
                // higher than any other member's).
                let final_view = transitions[0]
                    .last()
                    .expect("member 0's interrupt must cause at least one view transition")
                    .clone();
                assert_eq!(
                    final_view.leader,
                    Some(MemberId::<Replica>::from_raw_id(0)),
                    "member 0 should be the settled leader after the uncontested retry"
                );
                assert!(
                    final_view.term >= 1,
                    "a won election must have bumped the term"
                );
            });
    }

    /// Heartbeats carry the leader's identity, so a round of heartbeats must converge
    /// every member's view onto the same leader — here driven end-to-end through the
    /// real mechanism (the leader's heartbeat timer broadcasting `AppendEntries`):
    ///
    /// 1. Member 0's election timer fires uncontested and it wins term 1. Before any
    ///    heartbeat, the voters know term 1 exists (vote requests carry it) but not
    ///    who leads it.
    /// 2. One heartbeat-timer round: each follower must learn the leader in *exactly
    ///    one* transition (term untouched — a heartbeat for the current term must not
    ///    bump it), the leader's own view must not change, and afterwards all four
    ///    members hold the identical view `(1, Some(member 0))`.
    /// 3. Deposition: member 1's election timer fires once — suppressed, because a
    ///    heartbeat was observed since the last interrupt (no view may change) — and
    ///    then again, winning term 2. After one of member 1's heartbeat rounds, every
    ///    member (including the deposed member 0) must converge to
    ///    `(2, Some(member 1))`. (With real vote traffic the term adoption and the
    ///    leader learning arrive as separate transitions, so unlike phase 2 this
    ///    asserts convergence rather than single-transition granularity.)
    ///
    /// The existing invariant also holds across all histories: no term is ever
    /// self-claimed by two members.
    #[test]
    fn heartbeats_converge_leader_views() {
        let mut flow = FlowBuilder::new();
        let cluster = flow.cluster::<Replica, u32>();

        let (interrupt_send, election_timer_interrupts) = cluster.sim_input();
        let (heartbeat_interrupt_send, heartbeat_timer_interrupts) = cluster.sim_input();
        let (_request_send, requests) = cluster.sim_input::<String, TotalOrder, ExactlyOnce>();
        let outputs = raft_server(
            &cluster,
            requests,
            election_timer_interrupts,
            heartbeat_timer_interrupts,
            RaftConfig {
                cluster_size: CLUSTER_SIZE,
            },
            TCP.fail_stop().bincode(),
            nondet!(/** which member wins an election is inherently non-deterministic */),
        );

        let view_recv = outputs.leader_views.sim_cluster_output();

        // The committed stream carries a consistency assertion (see raft_server); the
        // simulator cannot validate those yet and asks tests to skip them.
        flow.sim()
            .skip_consistency_assertions()
            .with_cluster_size(&cluster, CLUSTER_SIZE)
            .fuzz(async || {
                let member_0 = MemberId::<Replica>::from_raw_id(0);
                let member_1 = MemberId::<Replica>::from_raw_id(1);
                let mut histories: Vec<Vec<LeaderView<Replica>>> = vec![Vec::new(); CLUSTER_SIZE];

                // Phase 1: an uncontested election. The `quiesce` phase barriers ensure
                // all vote traffic settles before any heartbeat is sent.
                interrupt_send.send(0, ());

                hydro_lang::sim::quiesce().await;
                for member in 0..CLUSTER_SIZE as u32 {
                    histories[member as usize].extend(view_recv.collect::<Vec<_>>(member).await);
                }
                assert_eq!(
                    histories[0].last(),
                    Some(&LeaderView {
                        term: 1,
                        leader: Some(member_0.clone()),
                    }),
                    "the uncontested candidate must win term 1"
                );
                for (member, history) in histories.iter().enumerate().skip(1) {
                    assert_eq!(
                        history.last(),
                        Some(&LeaderView {
                            term: 1,
                            leader: None,
                        }),
                        "member {} should know term 1 but not its leader before any heartbeat",
                        member
                    );
                }

                // Phase 2: one real heartbeat round from the leader.
                heartbeat_interrupt_send.send(0, ());

                hydro_lang::sim::quiesce().await;
                for member in 0..CLUSTER_SIZE as u32 {
                    let new_transitions: Vec<LeaderView<Replica>> =
                        view_recv.collect(member).await;
                    if member == 0 {
                        assert!(
                            new_transitions.is_empty(),
                            "the leader's view must not change while it stays leader"
                        );
                    } else {
                        assert_eq!(
                            new_transitions,
                            vec![LeaderView {
                                term: 1,
                                leader: Some(member_0.clone()),
                            }],
                            "member {} must learn the leader from the heartbeat in exactly one transition",
                            member
                        );
                    }
                    histories[member as usize].extend(new_transitions);
                }
                for (member, history) in histories.iter().enumerate() {
                    assert_eq!(
                        history.last(),
                        Some(&LeaderView {
                            term: 1,
                            leader: Some(member_0.clone()),
                        }),
                        "after a round of heartbeats, member {} must share the leader view",
                        member
                    );
                }

                // Phase 3a: member 1's first interrupt is suppressed — it observed a
                // heartbeat since its last interrupt, so no candidacy may start and
                // no member's view may change.
                interrupt_send.send(1, ());
                hydro_lang::sim::quiesce().await;
                for member in 0..CLUSTER_SIZE as u32 {
                    let new_transitions: Vec<LeaderView<Replica>> =
                        view_recv.collect(member).await;
                    assert!(
                        new_transitions.is_empty(),
                        "member {}'s view changed after a suppressed election interrupt",
                        member
                    );
                }

                // Phase 3b: the second interrupt finds no fresh heartbeat and starts
                // a real candidacy; member 1 wins term 2 (all logs are empty, so
                // every voter grants), deposing member 0.
                interrupt_send.send(1, ());
                hydro_lang::sim::quiesce().await;
                for member in 0..CLUSTER_SIZE as u32 {
                    histories[member as usize].extend(view_recv.collect::<Vec<_>>(member).await);
                }
                assert_eq!(
                    histories[1].last(),
                    Some(&LeaderView {
                        term: 2,
                        leader: Some(member_1.clone()),
                    }),
                    "member 1 must win term 2 uncontested"
                );

                // One heartbeat round converges everyone — including the deposed
                // member 0 — onto the new leader.
                heartbeat_interrupt_send.send(1, ());
                hydro_lang::sim::quiesce().await;
                for member in 0..CLUSTER_SIZE as u32 {
                    histories[member as usize].extend(view_recv.collect::<Vec<_>>(member).await);
                }
                for (member, history) in histories.iter().enumerate() {
                    assert_eq!(
                        history.last(),
                        Some(&LeaderView {
                            term: 2,
                            leader: Some(member_1.clone()),
                        }),
                        "member {} must converge on the term-2 leader",
                        member
                    );
                }

                // Invariant: no term is ever self-claimed by two members.
                let mut leader_by_term: HashMap<usize, usize> = HashMap::new();
                for (member, member_history) in histories.iter().enumerate() {
                    let me = MemberId::<Replica>::from_raw_id(member as u32);
                    for view in member_history {
                        if view.leader.as_ref() == Some(&me)
                            && let Some(previous) = leader_by_term.insert(view.term, member)
                        {
                            panic!(
                                "term {} was claimed by both member {} and member {}",
                                view.term, previous, member
                            );
                        }
                    }
                }
            });
    }

    /// Drives N deterministic [`MemberState`] machines connected by an in-memory
    /// "network" of per-member inboxes, calling [`raft_step`] directly. Protocol-rule
    /// tests run on this instead of the simulator: they need precise, deterministic
    /// control over delivery and timers (e.g. "this entry is never replicated"),
    /// while schedule exploration is covered by the simulation tests below.
    ///
    /// Members `deployed..cluster_size` model crashed members: they exist in the
    /// protocol configuration (majorities count them) but never take a step, so
    /// messages to them pile up unanswered.
    struct StepCluster {
        states: Vec<MemberState>,
        inboxes: Vec<Vec<(MemberId<Replica>, RaftRpc<String>)>>,
        committed: Vec<Vec<LogEntry<String>>>,
        redirected: Vec<Vec<(String, Option<MemberId<Replica>>)>>,
        cluster_size: usize,
        deployed: usize,
    }

    impl StepCluster {
        fn new(deployed: usize, cluster_size: usize) -> Self {
            StepCluster {
                states: (0..cluster_size).map(|_| MemberState::new()).collect(),
                inboxes: vec![Vec::new(); cluster_size],
                committed: vec![Vec::new(); cluster_size],
                redirected: vec![Vec::new(); cluster_size],
                cluster_size,
                deployed,
            }
        }

        fn id(member: usize) -> MemberId<Replica> {
            MemberId::from_raw_id(member as u32)
        }

        /// Advances `member` by one tick, consuming its inbox plus the given timer
        /// events and client requests, and routing its outbound messages.
        fn step(&mut self, member: usize, election: bool, heartbeat: bool, requests: &[&str]) {
            let messages = std::mem::take(&mut self.inboxes[member]);
            let output = raft_step(
                &mut self.states[member],
                StepInput {
                    me: Self::id(member),
                    other_members: (0..self.cluster_size)
                        .filter(|target| *target != member)
                        .map(Self::id)
                        .collect(),
                    cluster_size: self.cluster_size,
                    election_timer_fired: election,
                    heartbeat_timer_fired: heartbeat,
                    requests: requests
                        .iter()
                        .map(|request| (*request).to_owned())
                        .collect(),
                    messages,
                },
            );
            for (target, message) in output.outbound {
                self.inboxes[target.get_raw_id() as usize].push((Self::id(member), message));
            }
            self.committed[member].extend(output.committed);
            self.redirected[member].extend(output.redirected);
        }

        /// Steps deployed members (no timers) until no inbox holds a message —
        /// i.e. all in-flight traffic, including replies to replies, has settled.
        fn deliver_until_quiet(&mut self) {
            loop {
                let mut progressed = false;
                for member in 0..self.deployed {
                    if !self.inboxes[member].is_empty() {
                        progressed = true;
                        self.step(member, false, false, &[]);
                    }
                }
                if !progressed {
                    return;
                }
            }
        }

        /// Runs a real election at `member`: fires its election timer (twice if the
        /// first interrupt is consumed by heartbeat suppression), settles the vote
        /// traffic, and asserts the member won. Returns the won term.
        fn elect(&mut self, member: usize) -> usize {
            // At most 2 interrupts: the first may be consumed by heartbeat
            // suppression; the second must start a candidacy.
            let mut attempts = 0;
            while self.states[member].election.role == RaftState::Follower {
                assert!(
                    attempts < 2,
                    "member {}'s election timer never started a candidacy \
                     after {} interrupts",
                    member,
                    attempts
                );
                self.step(member, true, false, &[]);
                attempts += 1;
            }
            self.deliver_until_quiet();
            assert!(
                self.states[member].election.role == RaftState::Leader,
                "member {} failed to win its election (role {:?}, term {})",
                member,
                self.states[member].election.role,
                self.states[member].election.term
            );
            self.states[member].election.term
        }

        /// Installs `member` as the leader of `term` directly, modeling figure 8's
        /// crash-and-reelect histories without simulating the message loss that
        /// produces them. The state produced is a protocol-reachable leadership: term
        /// advanced, vote self-recorded; the member's next step resets its follower
        /// bookkeeping, as for any new leadership.
        fn crown(&mut self, member: usize, term: usize) {
            let state = &mut self.states[member].election;
            assert!(term > state.term, "terms only move forward");
            state.term = term;
            state.role = RaftState::Leader;
            state.voted_for = Some(Self::id(member));
            state.votes.clear();
            state.known_leader = None;
        }

        /// Sends `payload` to `member` and asserts it was appended as leader rather
        /// than redirected.
        fn submit(&mut self, member: usize, payload: &str) {
            let redirects_before = self.redirected[member].len();
            self.step(member, false, false, &[payload]);
            assert_eq!(
                self.redirected[member].len(),
                redirects_before,
                "member {} redirected \"{}\" instead of appending it as leader",
                member,
                payload
            );
        }

        /// One settled heartbeat round: the leader broadcasts, and all resulting
        /// traffic (appends, acks, and the leader's reactions) quiesces.
        fn heartbeat_round(&mut self, leader: usize) {
            self.step(leader, false, true, &[]);
            self.deliver_until_quiet();
        }

        fn assert_committed(&self, expected: &[(&str, usize)]) {
            let expected: Vec<LogEntry<String>> = expected
                .iter()
                .enumerate()
                .map(|(position, (message, term))| LogEntry {
                    message: (*message).to_owned(),
                    term_received: *term,
                    index: position + 1,
                })
                .collect();
            for member in 0..self.deployed {
                assert_eq!(
                    self.committed[member], expected,
                    "member {}'s committed history diverges",
                    member
                );
            }
        }

        fn assert_nothing_redirected(&self) {
            for member in 0..self.deployed {
                assert!(
                    self.redirected[member].is_empty(),
                    "member {} redirected requests: {:?}",
                    member,
                    self.redirected[member]
                );
            }
        }
    }

    /// The interlock that requires both components to run in one tick (see [`raft`]'s
    /// design notes): a member's vote decisions and its log/ack decisions must be
    /// read-modify-writes against the same state, in both directions, even when the
    /// triggering messages arrive in the same batch.
    ///
    /// * **Ack-then-vote**: a member that has acknowledged an entry compares later
    ///   vote requests against the log *including* that entry, so a candidate
    ///   missing it is refused (§5.4.1 reads the live log).
    /// * **Vote-then-ack**: a member that has voted in a higher term rejects an
    ///   old-term leader's `AppendEntries` outright, so its ack can never help
    ///   commit an entry behind the new leader's back (§5.1 reads the live term).
    #[test]
    fn vote_and_ack_decisions_interlock() {
        let leader_0 = StepCluster::id(0);
        let candidate_2 = StepCluster::id(2);
        let entry = |index: usize| LogEntry {
            message: format!("entry-{}", index),
            term_received: 1,
            index,
        };
        let append = |entries: Vec<LogEntry<String>>, prev: usize| {
            RaftRpc::Replication(ReplicationRpc::AppendEntries(AppendEntriesRequest {
                term: 1,
                prev_log_index: prev,
                prev_log_term: usize::from(prev != 0),
                entries,
                leader_commit: 0,
            }))
        };

        // Ack-then-vote: member 1 acknowledges entry 1 from the term-1 leader, and
        // *in the same batch* receives a term-2 vote request from a candidate whose
        // log is empty. Whatever the processing order, the two decisions must never
        // combine into "acked the entry AND endorsed a candidate missing it": the
        // election step runs first (empty-vs-empty log: granted, term becomes 2), which
        // forces the replication step to reject the term-1 append.
        let mut state = MemberState::new();
        let output = raft_step(
            &mut state,
            StepInput {
                me: StepCluster::id(1),
                other_members: vec![StepCluster::id(0), StepCluster::id(2)],
                cluster_size: 3,
                election_timer_fired: false,
                heartbeat_timer_fired: false,
                requests: vec![],
                messages: vec![
                    (leader_0.clone(), append(vec![entry(1)], 0)),
                    (
                        candidate_2.clone(),
                        RaftRpc::Election(ElectionRpc::RequestVote(RequestVoteDto {
                            term: 2,
                            last_log_index: 0,
                            last_log_term: 0,
                        })),
                    ),
                ],
            },
        );
        assert_eq!(
            state.election.term, 2,
            "the vote request's term must be adopted"
        );
        assert_eq!(
            state.election.voted_for,
            Some(candidate_2.clone()),
            "the empty-log candidate is legitimately granted against an empty log"
        );
        assert!(
            state.replication.log.is_empty(),
            "having endorsed a term-2 candidate, the term-1 append must be rejected"
        );
        let acked = output.outbound.iter().any(|(_, message)| {
            matches!(
                message,
                RaftRpc::Replication(ReplicationRpc::AppendEntriesReply(reply)) if reply.success
            )
        });
        assert!(
            !acked,
            "an ack and a vote for a candidate missing the acked entry must never \
             both happen"
        );

        // Vote-then-ack, later ticks: the vote persists, so an old-term append in a
        // *subsequent* tick is still rejected (the term was adopted atomically with
        // the grant — there is no window where the log half still believes term 1).
        let output = raft_step(
            &mut state,
            StepInput {
                me: StepCluster::id(1),
                other_members: vec![StepCluster::id(0), StepCluster::id(2)],
                cluster_size: 3,
                election_timer_fired: false,
                heartbeat_timer_fired: false,
                requests: vec![],
                messages: vec![(leader_0.clone(), append(vec![entry(1)], 0))],
            },
        );
        assert!(
            state.replication.log.is_empty(),
            "a stale-term append must stay rejected"
        );
        assert!(matches!(
            &output.outbound[..],
            [(target, RaftRpc::Replication(ReplicationRpc::AppendEntriesReply(reply)))]
                if *target == leader_0 && !reply.success && reply.term == 2
        ));

        // Ack-then-vote, later ticks: a member whose log holds an acknowledged entry
        // refuses a later candidate that lacks it (§5.4.1 against the live log).
        let mut state = MemberState::new();
        raft_step(
            &mut state,
            StepInput {
                me: StepCluster::id(1),
                other_members: vec![StepCluster::id(0), StepCluster::id(2)],
                cluster_size: 3,
                election_timer_fired: false,
                heartbeat_timer_fired: false,
                requests: vec![],
                messages: vec![(leader_0, append(vec![entry(1)], 0))],
            },
        );
        assert_eq!(
            state.replication.log.len(),
            1,
            "the term-1 append is accepted this time"
        );
        raft_step(
            &mut state,
            StepInput {
                me: StepCluster::id(1),
                other_members: vec![StepCluster::id(0), StepCluster::id(2)],
                cluster_size: 3,
                election_timer_fired: false,
                heartbeat_timer_fired: false,
                requests: vec![],
                messages: vec![(
                    candidate_2,
                    RaftRpc::Election(ElectionRpc::RequestVote(RequestVoteDto {
                        term: 2,
                        last_log_index: 0,
                        last_log_term: 0,
                    })),
                )],
            },
        );
        assert_eq!(
            state.election.term, 2,
            "the higher term is adopted even when refusing"
        );
        assert_eq!(
            state.election.voted_for, None,
            "a candidate whose log lacks the acknowledged entry must be refused"
        );
    }

    /// A leader that receives an `AppendEntriesReply` carrying a term higher than
    /// its own must step down to follower immediately (RAFT §5.1): it is no longer
    /// the legitimate leader. This covers the deposition path via reply traffic
    /// (the complementary path — deposition via a higher-term `RequestVote` — is
    /// exercised by `vote_and_ack_decisions_interlock`).
    #[test]
    fn leader_steps_down_on_higher_term_reply() {
        let mut state = MemberState::new();
        // Install member 0 as leader of term 1.
        state.election.term = 1;
        state.election.role = RaftState::Leader;
        state.election.voted_for = Some(StepCluster::id(0));

        let output = raft_step(
            &mut state,
            StepInput {
                me: StepCluster::id(0),
                other_members: vec![StepCluster::id(1), StepCluster::id(2)],
                cluster_size: 3,
                election_timer_fired: false,
                heartbeat_timer_fired: false,
                requests: vec![],
                messages: vec![(
                    StepCluster::id(1),
                    RaftRpc::Replication(ReplicationRpc::AppendEntriesReply(AppendEntriesReply {
                        term: 3,
                        success: false,
                        match_index: 0,
                    })),
                )],
            },
        );
        assert_eq!(
            state.election.term, 3,
            "the leader must adopt the higher term"
        );
        assert_eq!(
            state.election.role,
            RaftState::Follower,
            "the leader must step down to follower on a higher-term reply"
        );
        assert_eq!(
            state.election.voted_for, None,
            "voted_for must be cleared when stepping down"
        );
        assert!(
            state.election.votes.is_empty(),
            "votes must be cleared when stepping down"
        );
        // No outbound messages expected (a deposed leader just stops leading).
        assert!(
            output.outbound.is_empty(),
            "stepping down should not produce outbound messages"
        );
    }

    /// The RAFT §5.4.1 election restriction: voters refuse candidates whose log is
    /// less up-to-date than their own. This property is what guarantees an elected
    /// leader holds every committed entry — without it, a member that sat out a
    /// partition inflating its term can win an election with a stale log and
    /// truncate committed entries (observed as a real linearizability violation
    /// under simulated partitions before this check existed).
    ///
    /// Setup: a two-member cluster where member 0's log position is (term 1,
    /// index 3) and member 1's log is empty. Majorities need both votes, so:
    ///
    /// 1. Member 1 campaigns with an empty log: member 0 must refuse (casting no
    ///    vote at all), and member 1 must never claim leadership.
    /// 2. Member 0 campaigns with the up-to-date log: member 1 must grant (its own
    ///    log is empty), and member 0 must win.
    #[test]
    fn stale_log_candidate_is_refused() {
        let mut cluster = StepCluster::new(2, 2);
        cluster.states[0].election.term = 1;
        cluster.states[0].replication.log = (1..=3)
            .map(|index| LogEntry {
                message: format!("entry-{}", index),
                term_received: 1,
                index,
            })
            .collect();

        // Phase 1: member 1 campaigns with an empty log; member 0 must refuse.
        cluster.step(1, true, false, &[]);
        cluster.deliver_until_quiet();
        assert!(
            cluster.states[1].election.role == RaftState::Candidate,
            "the stale-logged member 1 must stall as a candidate, got {:?}",
            cluster.states[1].election.role
        );
        assert_eq!(
            cluster.states[0].election.voted_for, None,
            "member 0 must not vote for a candidate whose log is behind its own"
        );

        // Phase 2: member 0 campaigns with the up-to-date log and must win. (Its
        // term already advanced past member 1's candidacy via the vote request.)
        let term = cluster.elect(0);
        assert!(
            term > cluster.states[1].election.term || cluster.states[1].election.term == term,
            "the election settles a single term"
        );
        assert_eq!(
            cluster.states[1].election.voted_for,
            Some(StepCluster::id(0)),
            "member 1 must grant the up-to-date candidate"
        );
    }

    /// The log replication happy path, end to end on the deterministic step harness:
    /// member 0 wins a real election, appends a request, and two settled heartbeat
    /// rounds replicate it, commit it at the leader (majority acks), and propagate
    /// the commit index to the followers. Every member commits exactly that entry,
    /// in log order, and nothing is redirected.
    #[test]
    fn leader_replicates_and_commits_requests() {
        let mut cluster = StepCluster::new(3, 3);
        let term = cluster.elect(0);
        assert_eq!(term, 1, "an uncontested first election settles term 1");

        cluster.submit(0, "alpha");
        // Round 1 replicates the entry and returns acks (the leader's commit index
        // advances while processing them); round 2 propagates the commit index.
        cluster.heartbeat_round(0);
        cluster.heartbeat_round(0);

        cluster.assert_committed(&[("alpha", 1)]);
        cluster.assert_nothing_redirected();
    }

    /// The redirect path: a request lands on a member that is not the leader. It
    /// must not be committed anywhere and must come back on `redirected` immediately
    /// — with hint `None` while no leader is known, and with the leader's identity
    /// once heartbeats have taught it.
    #[test]
    fn non_leader_redirects_requests() {
        // Before any election: no hint exists.
        let mut cluster = StepCluster::new(3, 3);
        cluster.step(1, false, false, &["beta"]);
        assert_eq!(
            cluster.redirected[1],
            vec![("beta".to_owned(), None)],
            "a request on a non-leader must be redirected with the best leader hint"
        );

        // After an election plus one heartbeat round: the hint names the leader.
        cluster.elect(0);
        cluster.heartbeat_round(0);
        cluster.step(2, false, false, &["gamma"]);
        assert_eq!(
            cluster.redirected[2],
            vec![("gamma".to_owned(), Some(StepCluster::id(0)))],
            "after heartbeats, redirects must carry the learned leader"
        );

        for member in 0..3 {
            assert!(
                cluster.committed[member].is_empty(),
                "nothing may commit without a leader replicating"
            );
        }
    }

    /// Quorum safety: the config describes a 3-member cluster, but only the leader is
    /// deployed — the two "crashed" followers can never acknowledge, so no entry may
    /// ever commit (majority is 2, and the leader's own vote is only 1). This is the
    /// test that catches an off-by-one quorum of `n / 2`, which would let the leader
    /// commit entirely on its own. (Leadership is installed via `crown`: a 1-of-3
    /// member cannot win a real election, which is the point.)
    #[test]
    fn leader_without_quorum_commits_nothing() {
        let mut cluster = StepCluster::new(1, 3);
        cluster.crown(0, 1);

        cluster.submit(0, "lonely");
        for _ in 0..3 {
            cluster.heartbeat_round(0);
            assert!(
                cluster.committed[0].is_empty(),
                "an entry committed without a quorum of acknowledgements"
            );
        }
        cluster.assert_nothing_redirected();
    }

    /// Divergent-log reconciliation (RAFT §5.3, figure 7): member 0 leads term 1 and
    /// commits "x" everywhere, then accepts "z" into its log *without replicating it*
    /// (its heartbeat timer never fires again). Member 1 — whose log ends at the
    /// committed "x" — becomes leader of term 2 through a real election (en route,
    /// member 0 refuses it under §5.4.1 since its own log is longer, but the other
    /// two members' grants suffice) and replicates "y". Member 0 must truncate its
    /// conflicting uncommitted "z" (same index, older term) and every member must
    /// commit exactly [x, y]; "z" must never appear on `committed`.
    ///
    /// This is the test that catches a skipped log-matching / conflict-truncation
    /// check, which would leave member 0 committing "z" where others commit "y".
    #[test]
    fn new_leader_overwrites_conflicting_uncommitted_entries() {
        let mut cluster = StepCluster::new(3, 3);
        assert_eq!(cluster.elect(0), 1);
        cluster.submit(0, "x");
        cluster.heartbeat_round(0);
        cluster.heartbeat_round(0);
        cluster.assert_committed(&[("x", 1)]);

        // "z" enters member 0's log but is never replicated.
        cluster.submit(0, "z");

        // Member 1 wins term 2 (member 0 refuses — its log is ahead — but members 1
        // and 2 form the majority) and replicates "y" over "z".
        assert_eq!(cluster.elect(1), 2);
        assert_eq!(
            cluster.states[0].election.voted_for, None,
            "member 0 must refuse the candidate whose log lacks its extra entry"
        );
        cluster.submit(1, "y");
        cluster.heartbeat_round(1);
        cluster.heartbeat_round(1);

        cluster.assert_committed(&[("x", 1), ("y", 2)]);
        cluster.assert_nothing_redirected();
        assert!(
            cluster
                .committed
                .iter()
                .flatten()
                .all(|entry| entry.message != "z"),
            "the truncated uncommitted entry must never surface on committed"
        );
    }

    /// Committing entries from previous terms (RAFT §5.4.2, figure 8): a leader must
    /// never count replicas to commit an entry from an earlier term — even one
    /// replicated to *every* member — because a competitor holding a conflicting
    /// entry could still be elected and overwrite uncommitted history. Previous-term
    /// entries may only commit transitively, beneath a committed entry of the
    /// leader's own term.
    ///
    /// The figure-8 histories arise from crashes and message loss, so leaderships are
    /// installed with `crown` while replication runs the real protocol:
    /// * Term 1: member 0 appends "old" but never heartbeats it (figure 8(a)).
    /// * Term 2: member 0, re-crowned, replicates "old" everywhere — `match_index`
    ///   reaches 1 on every member, beyond a majority, yet nothing may commit:
    ///   "old" is a term-1 entry under a term-2 leader. This is the trap that catches
    ///   a dropped §5.4.2 restriction (figure 8(c)).
    /// * Term 3: member 2 appends "conflict" at index 2, unreplicated (figure 8's S5;
    ///   the common prefix keeps later empty heartbeats from disturbing it).
    /// * Term 4: member 0, re-crowned, appends "winner" at index 2, unreplicated —
    ///   the doomed entry.
    /// * Term 5: member 2 is crowned; its log is authoritative: the `next_index`
    ///   walk-back truncates "winner" in favor of "conflict", and a fresh
    ///   current-term entry "new" commits by counting — carrying "old" and
    ///   "conflict" with it transitively (figure 8(d)/(e)).
    #[test]
    fn previous_term_entries_commit_only_transitively() {
        let mut cluster = StepCluster::new(3, 3);

        // Term 1: "old" enters member 0's log, never replicated.
        cluster.crown(0, 1);
        cluster.submit(0, "old");
        assert!(
            cluster.committed.iter().all(|entries| entries.is_empty()),
            "\"old\" must not commit unreplicated"
        );

        // Term 2: member 0 re-crowned; heartbeat rounds replicate "old" everywhere
        // and process every member's acknowledgement at the leader.
        cluster.crown(0, 2);
        cluster.heartbeat_round(0);
        cluster.heartbeat_round(0);
        assert!(
            (1..3).all(|member| cluster.states[member].replication.log.len() == 1),
            "\"old\" must be replicated to every member"
        );
        assert!(
            (1..3).all(|member| {
                cluster.states[0].replication.match_index[&StepCluster::id(member)] == 1
            }),
            "every member's acknowledgement of \"old\" must have been processed"
        );

        // THE §5.4.2 TRAP: a majority's acks for "old" sit in match_index, but it is
        // a term-1 entry under a term-2 leader. A leader that counts replicas for
        // previous-term entries commits (and emits) it right here.
        assert!(
            cluster.committed.iter().all(|entries| entries.is_empty()),
            "a previous-term entry must never commit by counting replicas \
             (RAFT §5.4.2), got {:?}",
            cluster.committed
        );

        // Term 3: member 2 appends "conflict" at index 2, unreplicated.
        cluster.crown(2, 3);
        cluster.submit(2, "conflict");

        // Term 4: member 0 appends "winner" at index 2, unreplicated — the doomed
        // entry.
        cluster.crown(0, 4);
        cluster.submit(0, "winner");

        // Term 5: member 2's log is authoritative; "new" commits by counting and
        // drags "old" and "conflict" across the commit line transitively. Rounds:
        // walk-back + delivery + commit + propagation each need a settled round.
        cluster.crown(2, 5);
        cluster.submit(2, "new");
        for _ in 0..4 {
            cluster.heartbeat_round(2);
        }

        cluster.assert_committed(&[("old", 1), ("conflict", 3), ("new", 5)]);
        cluster.assert_nothing_redirected();
        assert!(
            cluster
                .committed
                .iter()
                .flatten()
                .all(|entry| entry.message != "winner"),
            "the overwritten \"winner\" must never surface on committed"
        );
    }

    /// The composed protocol end to end through [`raft`]'s public API. The test
    /// drives only `raft`'s real inputs (two timer streams and client requests)
    /// and observes only its real outputs (`committed` and `redirected`):
    ///
    /// 1. Member 0 wins term 1 through a *real* election (timer interrupt, vote
    ///    RPCs, majority count) — no injected views exist at this layer.
    /// 2. A request sent to member 0 is committed on every member as
    ///    `(index 1, term 1)`, proving election → replication handoff.
    /// 3. A request sent to follower member 1 is redirected with hint
    ///    `Some(member 0)` — the identity learned from the leader's real
    ///    `AppendEntries` heartbeats.
    /// 4. Member 1's election timer fires after heartbeats were observed: it must be
    ///    suppressed (heartbeat-since-last-interrupt), so a further request commits
    ///    on every member still at term 1 — the leader was not deposed.
    #[test]
    fn composed_raft_elects_replicates_and_suppresses() {
        const N: usize = 3;
        const MAX_ROUNDS: usize = 16;
        let mut flow = FlowBuilder::new();
        let cluster = flow.cluster::<Replica, u32>();

        let (election_interrupt_send, election_timer_interrupts) = cluster.sim_input();
        let (heartbeat_interrupt_send, heartbeat_timer_interrupts) = cluster.sim_input();
        let (request_send, requests) = cluster.sim_input::<String, _, _>();

        let (committed, redirected) = raft(
            requests,
            election_timer_interrupts,
            heartbeat_timer_interrupts,
            RaftConfig { cluster_size: N },
            || TCP.fail_stop().bincode(),
            nondet!(
                /** which member leads and how concurrent requests are ordered is
                non-deterministic; the committed sequence must not be */
            ),
        );

        let committed_recv = committed.end_atomic().sim_cluster_output();
        let redirected_recv = redirected.sim_cluster_output();

        // The committed stream carries a consistency assertion (see raft_server); the
        // simulator cannot validate those yet and asks tests to skip them.
        flow.sim()
            .skip_consistency_assertions()
            .with_cluster_size(&cluster, N)
            .fuzz(async || {
                let member_0 = MemberId::<Replica>::from_raw_id(0);
                let mut committed: Vec<Vec<LogEntry<String>>> = vec![Vec::new(); N];

                // Pumps member 0's heartbeat timer in quiescence-separated rounds
                // until every member's history holds `at_least` entries (see
                // ReplicationHarness::pump_until_committed for the bound rationale).
                let pump_until_committed =
                    async |committed: &mut Vec<Vec<LogEntry<String>>>, at_least: usize| {
                        for _ in 0..MAX_ROUNDS {
                            heartbeat_interrupt_send.send(0, ());
                            hydro_lang::sim::quiesce().await;
                            for member in 0..N as u32 {
                                committed[member as usize]
                                    .extend(committed_recv.collect::<Vec<_>>(member).await);
                            }
                            if committed.iter().all(|entries| entries.len() >= at_least) {
                                return;
                            }
                        }
                        panic!(
                            "not every member committed {} entries within {} heartbeat \
                             rounds; per-member counts: {:?}",
                            at_least,
                            MAX_ROUNDS,
                            committed
                                .iter()
                                .map(|entries| entries.len())
                                .collect::<Vec<_>>()
                        );
                    };

                // Phase 1: member 0 wins term 1 through a real election. No heartbeat
                // has ever been observed, so the interrupt cannot be suppressed; the
                // `quiesce` barrier settles all vote traffic before anything else.
                election_interrupt_send.send(0, ());
                hydro_lang::sim::quiesce().await;
                let during_election: Vec<LogEntry<String>> = committed_recv.collect(0).await;
                assert!(
                    during_election.is_empty(),
                    "nothing may commit during an election with no requests"
                );

                // Phase 2: end-to-end commit through the composition. If member 0 had
                // not won, the request would be redirected and the pump would panic.
                request_send.send(0, "first".to_owned());
                pump_until_committed(&mut committed, 1).await;
                for (member, entries) in committed.iter().enumerate() {
                    assert_eq!(
                        *entries,
                        vec![LogEntry {
                            message: "first".to_owned(),
                            term_received: 1,
                            index: 1,
                        }],
                        "member {} must commit the leader's request at (index 1, term 1)",
                        member
                    );
                }
                let leader_redirects: Vec<(String, Option<MemberId<Replica>>)> =
                    redirected_recv.collect(0).await;
                assert!(
                    leader_redirects.is_empty(),
                    "the elected leader must not redirect requests sent to it"
                );

                // Phase 3: the phase-2 heartbeats carried member 0's identity through
                // the forward_ref loop into member 1's LeaderView; a misdirected
                // request must come back with that learned hint.
                request_send.send(1, "misdirected".to_owned());
                hydro_lang::sim::quiesce().await;
                let follower_redirects: Vec<(String, Option<MemberId<Replica>>)> =
                    redirected_recv.collect(1).await;
                assert_eq!(
                    follower_redirects,
                    vec![("misdirected".to_owned(), Some(member_0.clone()))],
                    "the follower must redirect with the leader hint learned from heartbeats"
                );

                // Phase 4: member 1 observed heartbeats since its (nonexistent)
                // previous election interrupt, so its timeout must be suppressed:
                // no candidacy, no term bump, member 0 stays leader...
                election_interrupt_send.send(1, ());
                hydro_lang::sim::quiesce().await;
                let after_suppression: Vec<LogEntry<String>> = committed_recv.collect(1).await;
                assert!(
                    after_suppression.is_empty(),
                    "a suppressed election must not disturb the committed log"
                );

                // ...which is proven by committing another request still at term 1.
                request_send.send(0, "second".to_owned());
                pump_until_committed(&mut committed, 2).await;
                for (member, entries) in committed.iter().enumerate() {
                    assert_eq!(
                        entries[1],
                        LogEntry {
                            message: "second".to_owned(),
                            term_received: 1,
                            index: 2,
                        },
                        "member {}: the suppressed election must not have bumped the \
                         term or deposed the leader",
                        member
                    );
                    assert_eq!(
                        entries.len(),
                        2,
                        "member {} committed extra entries",
                        member
                    );
                }
            });
    }

    /// **Regression net for the cross-tick vote/ack race.** When `leader_election` and
    /// `log_replication` run in separate ticks connected by deferred feedback streams,
    /// this test reliably reproduces a real safety bug (6/6 fuzz runs, usually within
    /// the first few iterations): a voter can grant a vote using a log summary lagging
    /// its just-acked entries, or ack an old-term leader after voting in a newer term.
    /// Either window lets an entry commit via acks from members that simultaneously
    /// help elect a leader missing that entry — the new leader then truncates a
    /// committed entry (surfacing as the truncation-guard panic, `protocol violation:
    /// asked to truncate committed entry`). No message loss is needed: plain
    /// `fail_stop` channels and interleaving alone suffice.
    ///
    /// Running both components in one tick, with the state threaded through them,
    /// closes both windows (see [`raft`]'s design notes). This test keeps exploring
    /// the schedules that open them.
    ///
    /// Detection is safety-only, in two layers:
    ///
    /// 1. Every member's committed history must stay pairwise prefix-consistent —
    ///    members may lag, but may never emit a *different* entry at the same
    ///    position.
    /// 2. [`replication_step`] asserts a member is never asked to truncate at or
    ///    below its own commit index; the simulator surfaces that panic as a failure.
    ///
    /// The choreography deliberately overlaps replication and elections without
    /// intermediate quiescence: each round primes a challenger's heartbeat-
    /// suppression flag, then in one un-quiesced burst sends a fresh request to
    /// member 0, pumps its heartbeat timer (pushing `AppendEntries` with the new
    /// entry), and fires the challenger's election timer (a concurrent candidacy).
    /// The fuzzer explores the delivery interleavings. No liveness is asserted in
    /// the racy rounds — a stalled round is a legal outcome.
    #[test]
    fn concurrent_elections_never_fork_the_committed_log() {
        const N: usize = 3;
        const MAX_ROUNDS: usize = 16;
        const RACY_ROUNDS: usize = 6;

        fn assert_prefix_consistent(histories: &[Vec<LogEntry<String>>]) {
            for a in 0..histories.len() {
                for b in (a + 1)..histories.len() {
                    for (position, (entry_a, entry_b)) in
                        histories[a].iter().zip(&histories[b]).enumerate()
                    {
                        assert_eq!(
                            entry_a, entry_b,
                            "committed logs forked: members {} and {} disagree at \
                             committed position {}",
                            a, b, position
                        );
                    }
                }
            }
        }

        let mut flow = FlowBuilder::new();
        let cluster = flow.cluster::<Replica, u32>();

        let (election_interrupt_send, election_timer_interrupts) = cluster.sim_input();
        let (heartbeat_interrupt_send, heartbeat_timer_interrupts) = cluster.sim_input();
        let (request_send, requests) = cluster.sim_input::<String, _, _>();

        let (committed, redirected) = raft(
            requests,
            election_timer_interrupts,
            heartbeat_timer_interrupts,
            RaftConfig { cluster_size: N },
            || TCP.fail_stop().bincode(),
            nondet!(
                /** which member leads and how concurrent requests are ordered is
                non-deterministic; the committed sequence must not be */
            ),
        );

        let committed_recv = committed.end_atomic().sim_cluster_output();
        let redirected_recv = redirected.sim_cluster_output();

        // The committed stream carries a consistency assertion (see raft_server); the
        // simulator cannot validate those yet and asks tests to skip them.
        flow.sim()
            .skip_consistency_assertions()
            .with_cluster_size(&cluster, N)
            .fuzz(async || {
                let mut committed: Vec<Vec<LogEntry<String>>> = vec![Vec::new(); N];

                // Quiesce (phase barrier), fold each member's newly committed entries
                // into its history, drain redirects, and check the fork invariant.
                let collect_and_check = async |committed: &mut Vec<Vec<LogEntry<String>>>| {
                    hydro_lang::sim::quiesce().await;
                    for member in 0..N as u32 {
                        committed[member as usize]
                            .extend(committed_recv.collect::<Vec<_>>(member).await);
                        let _: Vec<(String, Option<MemberId<Replica>>)> =
                            redirected_recv.collect(member).await;
                    }
                    assert_prefix_consistent(committed);
                };

                // Phase 1: member 0 wins term 1 uncontested and commits a seed entry,
                // so later (possibly stale) challengers have something to be behind.
                // The `quiesce` barrier (in collect_and_check) after the interrupt lets
                // the election settle
                // (vote traffic and view propagation) before the request is sent —
                // otherwise the request could race ahead of member 0's leadership and
                // be redirected into the void. This phase is deliberately race-free,
                // so a liveness bound is safe (same rationale as
                // composed_raft_elects_replicates_and_suppresses).
                election_interrupt_send.send(0, ());
                collect_and_check(&mut committed).await;
                request_send.send(0, "seed".to_owned());
                let mut pumps = 0;
                while committed.iter().any(|entries| entries.is_empty()) {
                    assert!(
                        pumps < MAX_ROUNDS,
                        "seed entry did not commit within {} uncontested heartbeat \
                         rounds; per-member counts: {:?}",
                        MAX_ROUNDS,
                        committed
                            .iter()
                            .map(|entries| entries.len())
                            .collect::<Vec<_>>()
                    );
                    pumps += 1;
                    heartbeat_interrupt_send.send(0, ());
                    collect_and_check(&mut committed).await;
                }

                // Phase 2: racy rounds. Everything inside a burst is sent without
                // quiescing in between, so replication of a fresh entry and a
                // challenger's candidacy overlap freely.
                for round in 0..RACY_ROUNDS {
                    let challenger = 1 + (round % (N - 1)) as u32;

                    // Prime: consume the challenger's heartbeat-suppression flag so
                    // the burst's second interrupt can actually start a candidacy
                    // (unless a racing heartbeat re-arms it — also a valid schedule).
                    election_interrupt_send.send(challenger, ());

                    // Burst: fresh entry + replication pumps + concurrent candidacy.
                    request_send.send(0, format!("racy-{}", round));
                    request_send.send(challenger, format!("challenger-{}", round));
                    heartbeat_interrupt_send.send(0, ());
                    election_interrupt_send.send(challenger, ());
                    heartbeat_interrupt_send.send(0, ());
                    collect_and_check(&mut committed).await;

                    // Settle: pump every member's heartbeat timer (only an acting
                    // leader — whoever that now is — emits AppendEntries), letting
                    // any successful election finish replicating so the next round
                    // starts from a coherent, but unspecified, leadership.
                    for _ in 0..3 {
                        for member in 0..N as u32 {
                            heartbeat_interrupt_send.send(member, ());
                        }
                        collect_and_check(&mut committed).await;
                    }
                }
            });
    }

    /// **Regression net for the cross-tick vote/ack race** (the bug documented on
    /// [`concurrent_elections_never_fork_the_committed_log`]; with the components in
    /// separate ticks this test reproduces it in 2 of 3 runs).
    ///
    /// The maximal version of the no-quiescence idea: *every* input for the entire
    /// run — election timer interrupts, client requests, and heartbeat pumps, for
    /// every member — is sent up front with **no intermediate quiescence at all**.
    /// The simulator's quiescing `collect` is used exactly once, at the end, to
    /// drain outputs; by then it cannot constrain how any inputs interleaved. The
    /// fuzzer therefore owns the complete schedule: which timer fires between which
    /// message deliveries, how batches form on each member's tick, everything.
    /// For deeper exploration, run it under coverage-guided fuzzing:
    /// `cargo sim -- fully_concurrent_run` (a found failure is saved as a
    /// minimized reproducer under `src/cluster/sim-failures/`, which plain
    /// `cargo test` then replays deterministically).
    ///
    /// The price of zero barriers is that nothing intermediate can be asserted — no
    /// "member X won term Y", and no liveness at all (a schedule where elections
    /// perpetually contend and nothing ever commits is legal). What remains is the
    /// protocol's core safety contract, checked at the end:
    ///
    /// 1. each member's committed stream has contiguous log indexes from 1 (each
    ///    position emitted exactly once, in order);
    /// 2. all members' committed histories are pairwise prefix-consistent (lagging
    ///    is fine, forking is not);
    /// 3. [`replication_step`]'s truncation guard never fires (surfaces as a panic /
    ///    SIGABRT if a committed entry is overwritten).
    ///
    /// Compared to the targeted test above, this explores a much larger schedule
    /// space with no phase structure biasing it toward the historical window — the
    /// stronger long-term net, while the targeted test remains the faster, more
    /// diagnostic one.
    #[test]
    fn fully_concurrent_run_never_forks_the_committed_log() {
        const N: usize = 3;
        const ELECTIONS_PER_MEMBER: usize = 2;
        const HEARTBEAT_PUMPS_PER_MEMBER: usize = 6;

        let mut flow = FlowBuilder::new();
        let cluster = flow.cluster::<Replica, u32>();

        let (election_interrupt_send, election_timer_interrupts) = cluster.sim_input();
        let (heartbeat_interrupt_send, heartbeat_timer_interrupts) = cluster.sim_input();
        let (request_send, requests) = cluster.sim_input::<String, _, _>();

        let (committed, redirected) = raft(
            requests,
            election_timer_interrupts,
            heartbeat_timer_interrupts,
            RaftConfig { cluster_size: N },
            || TCP.fail_stop().bincode(),
            nondet!(
                /** which member leads and how concurrent requests are ordered is
                non-deterministic; the committed sequence must not be */
            ),
        );

        let committed_recv = committed.end_atomic().sim_cluster_output();
        let redirected_recv = redirected.sim_cluster_output();

        // The committed stream carries a consistency assertion (see raft_server); the
        // simulator cannot validate those yet and asks tests to skip them.
        flow.sim()
            .skip_consistency_assertions()
            .with_cluster_size(&cluster, N)
            .fuzz(async || {
                // Fire everything up front. No `.await` anywhere in this block, so
                // all inputs are concurrently outstanding and the fuzzer alone
                // decides every interleaving. Sends are woven round-robin across
                // members and input kinds only to avoid biasing the schedule via
                // enqueue order.
                for wave in 0..ELECTIONS_PER_MEMBER {
                    for member in 0..N as u32 {
                        election_interrupt_send.send(member, ());
                        request_send.send(member, format!("wave-{}-member-{}", wave, member));
                    }
                }
                for _ in 0..HEARTBEAT_PUMPS_PER_MEMBER {
                    for member in 0..N as u32 {
                        heartbeat_interrupt_send.send(member, ());
                    }
                }

                // The single, final quiescence: drain both outputs.
                let mut committed: Vec<Vec<LogEntry<String>>> = vec![Vec::new(); N];
                for member in 0..N as u32 {
                    committed[member as usize]
                        .extend(committed_recv.collect::<Vec<_>>(member).await);
                    let _: Vec<(String, Option<MemberId<Replica>>)> =
                        redirected_recv.collect(member).await;
                }

                // Safety only. (1) Per-member: contiguous indexes from 1, in order.
                for (member, history) in committed.iter().enumerate() {
                    for (position, entry) in history.iter().enumerate() {
                        assert_eq!(
                            entry.index,
                            position + 1,
                            "member {} emitted committed entries out of order or \
                             with gaps: {:?}",
                            member,
                            history.iter().map(|e| e.index).collect::<Vec<_>>()
                        );
                    }
                }
                // (2) Pairwise: histories may lag but never fork.
                for a in 0..N {
                    for b in (a + 1)..N {
                        for (position, (entry_a, entry_b)) in
                            committed[a].iter().zip(&committed[b]).enumerate()
                        {
                            assert_eq!(
                                entry_a, entry_b,
                                "committed logs forked: members {} and {} disagree \
                                 at committed position {}",
                                a, b, position
                            );
                        }
                    }
                }
            });
    }

    /// Deploy-shaped wiring mirroring `examples/raft.rs`: periodic per-member
    /// requests, per-member skewed election timers, fast heartbeats, and TCP
    /// networking, with committed entries and redirected requests printed.
    fn create_raft(replicas: &Cluster<'_, Replica>) {
        use hydro_lang::location::Location;
        use hydro_lang::location::cluster::CLUSTER_SELF_ID;

        let requests = replicas
            .source_interval(q!(std::time::Duration::from_secs(1)))
            .map(q!(move |_| format!(
                "hello from member {}",
                CLUSTER_SELF_ID.clone().into_tagless()
            )))
            .assume_retries::<ExactlyOnce>(nondet!(
                /// The interval timer's `()`s are `AtLeastOnce`; this wiring treats each
                /// fire as one distinct request, a choice the workload is allowed to make.
            ));

        let election_timer_interrupts = replicas
            .source_interval(q!(std::time::Duration::from_millis(
                500 + u64::from(CLUSTER_SELF_ID.get_raw_id()) * 130
            )))
            .assume_retries::<ExactlyOnce>(nondet!(
                /// `source_interval`'s fires are `AtLeastOnce`; `raft` consumes its timers
                /// `ExactlyOnce` and only tests whether a tick saw a fire, so a repeated
                /// fire is just another fire.
            ));
        let heartbeat_timer_interrupts = replicas
            .source_interval(q!(std::time::Duration::from_millis(100)))
            .assume_retries::<ExactlyOnce>(nondet!(
                /// `source_interval`'s fires are `AtLeastOnce`; `raft` consumes its timers
                /// `ExactlyOnce` and only tests whether a tick saw a fire, so a repeated
                /// fire is just another fire.
            ));

        let (committed, redirected) = raft(
            requests,
            election_timer_interrupts,
            heartbeat_timer_interrupts,
            RaftConfig { cluster_size: 3 },
            || TCP.fail_stop().bincode(),
            nondet!(
                /// Which member leads and how concurrent requests interleave in the
                /// log is inherently non-deterministic; every member still prints
                /// the same committed sequence.
            ),
        );

        committed
            .end_atomic()
            .weaken_consistency()
            .for_each(q!(|entry| println!(
                "committed [term {}, index {}]: {}",
                entry.term_received, entry.index, entry.message
            )));

        redirected.for_each(q!(|(request, leader_hint)| println!(
            "redirected: {request:?} (leader hint: {leader_hint:?})"
        )));
    }

    /// Pins the Hydro IR (and the per-member DFIR graph) generated for the
    /// deploy-shaped raft wiring, so optimizer or staging regressions surface as
    /// snapshot diffs — matching the `paxos_ir` / `two_pc_ir` convention.
    #[test]
    fn raft_ir() {
        use dfir_lang::graph::WriteConfig;
        use hydro_lang::deploy::HydroDeploy;

        let mut builder = FlowBuilder::new();
        let replicas = builder.cluster::<Replica, u32>();
        create_raft(&replicas);
        let mut built = builder.with_default_optimize::<HydroDeploy>();

        hydro_lang::compile::ir::dbg_dedup_tee(|| {
            hydro_build_utils::assert_debug_snapshot!(built.ir());
        });

        let preview = built.preview_compile();
        hydro_build_utils::insta::with_settings!({
            snapshot_suffix => "replica_mermaid"
        }, {
            hydro_build_utils::assert_snapshot!(
                preview.dfir_for(&replicas).unwrap().to_mermaid(&WriteConfig {
                    no_subgraphs: true,
                    no_pull_push: true,
                    no_handoffs: true,
                    op_text_no_imports: true,
                    ..WriteConfig::default()
                })
            );
        });
    }
}
