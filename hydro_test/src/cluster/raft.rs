//! An implementation of the RAFT consensus protocol for replicating a log across a
//! cluster.
//!
//! The protocol logic is a pure, sequential step function over one unified per-member
//! state (see [`raft_step`]), hosted in a single dataflow tick — deliberately *not*
//! decomposed into separately-ticking election/replication components, because RAFT's
//! safety proofs interlock vote and log decisions through shared state (see the design
//! notes on [`raft`]).
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
use hydro_lang::live_collections::stream::{MinOrder, NoOrder, Ordering, TotalOrder};
use hydro_lang::location::cluster::{
    CLUSTER_SELF_ID, ClusterIds, Consistency, EventualConsistency, NoConsistency,
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
    /// When in follower mode, a RAFT server receives AppendEntries RPCs and updates its log
    Follower,
    /// When in candidate mode, a RAFT server requests votes and is trying to become the leader
    Candidate,
    /// When leader, the RAFT server sends AppendEntries RPCs and serves client requests
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
pub struct AppendEntriesRequest<T, ClusterTag> {
    /// The leader's term.
    pub term: usize,
    /// The leader's identity, so followers can redirect clients to it.
    pub leader: MemberId<ClusterTag>,
    /// Index of the log entry immediately preceding `entries`.
    pub prev_log_index: usize,
    /// Term of the entry at `prev_log_index`.
    pub prev_log_term: usize,
    /// New entries to append (empty for heartbeats).
    pub entries: Vec<LogEntry<T>>,
    /// The leader's commit index.
    pub leader_commit: usize,
}

impl<T: Clone, ClusterTag> Clone for AppendEntriesRequest<T, ClusterTag> {
    fn clone(&self) -> Self {
        AppendEntriesRequest {
            term: self.term,
            leader: self.leader.clone(),
            prev_log_index: self.prev_log_index,
            prev_log_term: self.prev_log_term,
            entries: self.entries.clone(),
            leader_commit: self.leader_commit,
        }
    }
}

impl<T: Debug, ClusterTag> Debug for AppendEntriesRequest<T, ClusterTag> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AppendEntriesRequest")
            .field("term", &self.term)
            .field("leader", &self.leader)
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

/// The single wire format for all intra-cluster RAFT traffic: vote RPCs and
/// replication RPCs travel over one channel, so every message is processed by the
/// same per-tick step against the same unified state — this is what makes the
/// vote/ack interlock atomic (see [`raft`]).
///
/// Trait impls avoid derive bounds on `ClusterTag` (it only appears inside
/// [`MemberId`], which implements everything for any tag); serde bounds constrain
/// only `T`.
///
/// Public only because staged (`q!`) code must reference it by path; not stable API.
#[doc(hidden)]
#[derive(Serialize, Deserialize)]
#[serde(bound(
    serialize = "T: Serialize",
    deserialize = "T: serde::de::DeserializeOwned"
))]
pub enum RaftRpc<T, ClusterTag> {
    /// Candidate -> everyone: request a vote for `term` (RAFT §5.2).
    RequestVote(RequestVoteDto),
    /// Voter -> candidate: grant a vote for `term`.
    RequestVoteResponse(RequestVoteResponseDto),
    /// Leader -> follower: replicate entries / heartbeat (RAFT §5.3).
    AppendEntries(AppendEntriesRequest<T, ClusterTag>),
    /// Follower -> leader: acknowledge or reject an `AppendEntries`.
    AppendEntriesReply(AppendEntriesReply),
}

impl<T: Clone, ClusterTag> Clone for RaftRpc<T, ClusterTag> {
    fn clone(&self) -> Self {
        match self {
            RaftRpc::RequestVote(dto) => RaftRpc::RequestVote(dto.clone()),
            RaftRpc::RequestVoteResponse(dto) => RaftRpc::RequestVoteResponse(dto.clone()),
            RaftRpc::AppendEntries(request) => RaftRpc::AppendEntries(request.clone()),
            RaftRpc::AppendEntriesReply(reply) => RaftRpc::AppendEntriesReply(reply.clone()),
        }
    }
}

impl<T: Debug, ClusterTag> Debug for RaftRpc<T, ClusterTag> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RaftRpc::RequestVote(dto) => f.debug_tuple("RequestVote").field(dto).finish(),
            RaftRpc::RequestVoteResponse(dto) => {
                f.debug_tuple("RequestVoteResponse").field(dto).finish()
            }
            RaftRpc::AppendEntries(request) => {
                f.debug_tuple("AppendEntries").field(request).finish()
            }
            RaftRpc::AppendEntriesReply(reply) => {
                f.debug_tuple("AppendEntriesReply").field(reply).finish()
            }
        }
    }
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

/// Configuration shared by [`raft`] and its inner server component.
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

/// The complete per-member RAFT state — election *and* replication — persisted
/// across ticks by the unified server component inside [`raft`].
///
/// Keeping `term` / `voted_for` / `log` in one struct, mutated by one sequential
/// step function ([`raft_step`]), is what makes RAFT's safety argument hold: the
/// decision to grant a vote and the decision to acknowledge log entries are
/// read-modify-writes against the *same* state, so no schedule can interleave them
/// against stale views of each other. (A previous design split this state across
/// two dataflow components with asynchronous feedback, which was a real,
/// simulator-reproducible safety bug: committed entries could be truncated.)
///
/// Public only because staged (`q!`) code must reference it by path; not stable API.
#[doc(hidden)]
pub struct RaftServerState<T, ClusterTag> {
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
    /// The replicated log (1-based indexing via `LogEntry::index`).
    pub log: Vec<LogEntry<T>>,
    /// The highest log index known to be committed (0 = nothing committed).
    pub commit_index: usize,
    /// The highest log index already emitted on the `committed` output; used to emit
    /// each committed entry exactly once, in log order.
    pub emitted_index: usize,
    /// Leader bookkeeping: the next log index to send to each follower. Reset on
    /// every leadership acquisition.
    pub next_index: HashMap<MemberId<ClusterTag>, usize>,
    /// Leader bookkeeping: the highest log index known replicated on each follower,
    /// for the current leadership only (acks are term-filtered). Reset on every
    /// leadership acquisition.
    pub match_index: HashMap<MemberId<ClusterTag>, usize>,
}

impl<T, ClusterTag> RaftServerState<T, ClusterTag> {
    /// The initial state of a freshly booted member: term 0 follower with an empty
    /// log.
    pub fn new() -> Self {
        RaftServerState {
            term: 0,
            voted_for: None,
            role: RaftState::Follower,
            votes: HashSet::new(),
            heartbeat_seen: false,
            known_leader: None,
            log: Vec::new(),
            commit_index: 0,
            emitted_index: 0,
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

impl<T, ClusterTag> Default for RaftServerState<T, ClusterTag> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: Clone, ClusterTag> Clone for RaftServerState<T, ClusterTag> {
    fn clone(&self) -> Self {
        RaftServerState {
            term: self.term,
            voted_for: self.voted_for.clone(),
            role: self.role,
            votes: self.votes.clone(),
            heartbeat_seen: self.heartbeat_seen,
            known_leader: self.known_leader.clone(),
            log: self.log.clone(),
            commit_index: self.commit_index,
            emitted_index: self.emitted_index,
            next_index: self.next_index.clone(),
            match_index: self.match_index.clone(),
        }
    }
}

/// One tick's worth of inputs to [`raft_step`]: everything the member observed since
/// the previous step, plus its static identity/configuration.
///
/// Public only because staged (`q!`) code must construct it by path; not stable API.
#[doc(hidden)]
pub struct RaftStepInput<T, ClusterTag> {
    /// This member's identity.
    pub me: MemberId<ClusterTag>,
    /// Every other member of the cluster (vote and heartbeat broadcast targets).
    pub other_members: Vec<MemberId<ClusterTag>>,
    /// Total protocol cluster size; majorities are `cluster_size / 2 + 1`.
    pub cluster_size: usize,
    /// Whether the election timer fired this tick.
    pub election_timer_fired: bool,
    /// Whether the heartbeat timer fired this tick.
    pub heartbeat_timer_fired: bool,
    /// Client requests received this tick, in the (arbitrary but fixed) order the
    /// caller acknowledged as non-deterministic.
    pub requests: Vec<T>,
    /// Intra-cluster messages received this tick, as an unordered batch; the step
    /// sorts them canonically so its outcome depends only on the batch *multiset*.
    pub messages: Vec<(MemberId<ClusterTag>, RaftRpc<T, ClusterTag>)>,
}

/// One tick's worth of outputs from [`raft_step`].
///
/// Public only because staged (`q!`) code must destructure it by path; not stable API.
#[doc(hidden)]
pub struct RaftStepOutput<T, ClusterTag> {
    /// Messages to send to specific members.
    pub outbound: Vec<(MemberId<ClusterTag>, RaftRpc<T, ClusterTag>)>,
    /// Entries newly committed this tick, in log order (each index emitted exactly
    /// once over the member's lifetime).
    pub committed: Vec<LogEntry<T>>,
    /// Client requests received while not the leader, paired with the best-known
    /// leader hint (`None` if unknown).
    pub redirected: Vec<(T, Option<MemberId<ClusterTag>>)>,
    /// The member's view after this tick, if it changed during the tick.
    pub view_transition: Option<LeaderView<ClusterTag>>,
}

/// Advances one member's [`RaftServerState`] by one tick: processes the received
/// messages, client requests, and timer events, returning the messages to send and
/// the entries that became committed.
///
/// This is deliberately a *pure, sequential* function (plain Rust, no staging): every
/// message is handled one at a time against the live state, so the protocol's
/// interlock invariants hold by construction —
///
/// * a member that acknowledged entries in term `T` compares any later vote request
///   against the log that *includes* those entries (§5.4.1 reads the real log);
/// * a member that voted in term `T'` has `term = T'` when the next `AppendEntries`
///   arrives, so it rejects any leader of an older term (§5.1's term check reads the
///   real term).
///
/// Message processing is order-insensitive at the batch level: the batch is sorted
/// into a canonical order first, so the outcome is a function of the batch multiset
/// (required for deterministic simulation).
///
/// Public (and callable outside staged code) so protocol-rule unit tests can drive
/// members directly and deterministically.
#[doc(hidden)]
pub fn raft_step<T: Clone, ClusterTag>(
    state: &mut RaftServerState<T, ClusterTag>,
    input: RaftStepInput<T, ClusterTag>,
) -> RaftStepOutput<T, ClusterTag> {
    let RaftStepInput {
        me,
        other_members,
        cluster_size,
        election_timer_fired,
        heartbeat_timer_fired,
        requests,
        mut messages,
    } = input;
    let majority = cluster_size / 2 + 1;

    let mut outbound = Vec::new();
    let mut committed = Vec::new();
    let mut redirected = Vec::new();

    let old_view = LeaderView {
        term: state.term,
        leader: if state.role == RaftState::Leader {
            Some(me.clone())
        } else {
            state.known_leader.clone()
        },
    };

    // A term above ours forces us back to follower with all per-term state reset
    // (RAFT §5.1). Returns whether the message's term is current afterwards.
    fn observe_term<T, ClusterTag>(
        state: &mut RaftServerState<T, ClusterTag>,
        term: usize,
    ) -> bool {
        if term > state.term {
            state.term = term;
            state.role = RaftState::Follower;
            state.voted_for = None;
            state.votes.clear();
            state.known_leader = None;
        }
        term == state.term
    }

    fn become_leader<T, ClusterTag>(
        state: &mut RaftServerState<T, ClusterTag>,
        other_members: &[MemberId<ClusterTag>],
    ) {
        state.role = RaftState::Leader;
        state.known_leader = None;
        state.next_index.clear();
        state.match_index.clear();
        for follower in other_members {
            state
                .next_index
                .insert(follower.clone(), state.log.len() + 1);
            state.match_index.insert(follower.clone(), 0);
        }
    }

    // (a) Process this tick's messages in a canonical order (so the step is a
    // function of the batch multiset), each one sequentially against live state.
    fn sort_key<T, ClusterTag>(rpc: &RaftRpc<T, ClusterTag>) -> (u8, usize, usize, usize) {
        match rpc {
            RaftRpc::RequestVote(dto) => (0, dto.term, dto.last_log_term, dto.last_log_index),
            RaftRpc::RequestVoteResponse(dto) => (1, dto.term, 0, 0),
            RaftRpc::AppendEntries(request) => (
                2,
                request.term,
                request.prev_log_index,
                request.entries.len(),
            ),
            RaftRpc::AppendEntriesReply(reply) => {
                (3, reply.term, reply.match_index, usize::from(reply.success))
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
            RaftRpc::RequestVote(dto) => {
                if !observe_term(state, dto.term) {
                    // A stale-term candidacy: ignore (the candidate will observe the
                    // higher term from other traffic or retry at its next timeout).
                    continue;
                }
                // §5.4.1 election restriction: only grant if the candidate's log is
                // at least as up-to-date as ours — this reads the *live* log, which
                // is exactly the interlock the split-component design lacked.
                let candidate_up_to_date =
                    (dto.last_log_term, dto.last_log_index) >= state.last_log_position();
                let can_vote =
                    state.voted_for.is_none() || state.voted_for.as_ref() == Some(&sender);
                if candidate_up_to_date && can_vote {
                    state.voted_for = Some(sender.clone());
                    outbound.push((
                        sender,
                        RaftRpc::RequestVoteResponse(RequestVoteResponseDto { term: state.term }),
                    ));
                }
            }
            RaftRpc::RequestVoteResponse(dto) => {
                if !observe_term(state, dto.term) {
                    continue;
                }
                if state.role == RaftState::Candidate {
                    state.votes.insert(sender);
                    if state.votes.len() >= majority {
                        become_leader(state, &other_members);
                    }
                }
            }
            RaftRpc::AppendEntries(request) => {
                if !observe_term(state, request.term) {
                    // Stale leader: reject with our (higher) term so it steps down.
                    outbound.push((
                        sender,
                        RaftRpc::AppendEntriesReply(AppendEntriesReply {
                            term: state.term,
                            success: false,
                            match_index: 0,
                        }),
                    ));
                    continue;
                }
                // A current-term AppendEntries while we lead the same term would
                // mean two leaders share a term — election safety is broken.
                assert!(
                    state.role != RaftState::Leader,
                    "protocol violation: two leaders share term {}",
                    state.term,
                );
                // A live leader exists for the current term: suppress the next
                // election, depose our candidacy, learn the leader's identity.
                state.heartbeat_seen = true;
                if state.role == RaftState::Candidate {
                    state.role = RaftState::Follower;
                }
                state.known_leader = Some(request.leader.clone());

                // Log-matching check (RAFT §5.3).
                let log_matches = request.prev_log_index == 0
                    || (state.log.len() >= request.prev_log_index
                        && state.log[request.prev_log_index - 1].term_received
                            == request.prev_log_term);
                if !log_matches {
                    outbound.push((
                        sender,
                        RaftRpc::AppendEntriesReply(AppendEntriesReply {
                            term: state.term,
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
                            // vote check above) guarantees every leader's log
                            // contains all committed entries. Fail fast if this
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
                    RaftRpc::AppendEntriesReply(AppendEntriesReply {
                        term: state.term,
                        success: true,
                        match_index: new_match,
                    }),
                ));
            }
            RaftRpc::AppendEntriesReply(reply) => {
                if !observe_term(state, reply.term) {
                    // An ack for an earlier leadership: the follower's log may have
                    // been truncated since, so it must not count toward this term.
                    continue;
                }
                if state.role != RaftState::Leader {
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

    // (b) Client requests: the leader appends them in the caller-fixed order;
    // everyone else redirects them with the best-known leader hint.
    for message in requests {
        if state.role == RaftState::Leader {
            let index = state.log.len() + 1;
            state.log.push(LogEntry {
                message,
                term_received: state.term,
                index,
            });
        } else {
            redirected.push((message, state.known_leader.clone()));
        }
    }

    // (c) Election timer interrupt. A leader runs no election timer. Otherwise: an
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
            state.votes.insert(me.clone());
            state.known_leader = None;
            if state.votes.len() >= majority {
                // Single-member cluster: our own vote suffices.
                become_leader(state, &other_members);
            } else {
                let (last_log_term, last_log_index) = state.last_log_position();
                for target in &other_members {
                    outbound.push((
                        target.clone(),
                        RaftRpc::RequestVote(RequestVoteDto {
                            term: state.term,
                            last_log_index,
                            last_log_term,
                        }),
                    ));
                }
            }
        }
    }

    // (d) Advance the leader's commit index: the §5.4.2 rule — only entries of the
    // leader's own term commit by counting replicas; earlier-term entries commit
    // transitively beneath them.
    if state.role == RaftState::Leader {
        let mut candidate = state.log.len();
        while candidate > state.commit_index {
            if state.log[candidate - 1].term_received == state.term {
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
    if heartbeat_timer_fired && state.role == RaftState::Leader {
        for follower in &other_members {
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
                RaftRpc::AppendEntries(AppendEntriesRequest {
                    term: state.term,
                    leader: me.clone(),
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

    // (g) Report a view transition iff the view changed this tick.
    let new_view = LeaderView {
        term: state.term,
        leader: if state.role == RaftState::Leader {
            Some(me)
        } else {
            state.known_leader.clone()
        },
    };
    let view_transition = (new_view != old_view).then_some(new_view);

    RaftStepOutput {
        outbound,
        committed,
        redirected,
        view_transition,
    }
}

/// The streams produced by the unified RAFT server component.
pub struct RaftOutputs<'a, T, ClusterTag> {
    /// The committed log entries, emitted on every member in log order. This stream
    /// is eventually consistent: all members observe the same sequence, though at
    /// different times. It is `Atomic` so that user actions taken on the commit path
    /// are completed before the protocol treats the entries as committed.
    pub committed: Stream<
        LogEntry<T>,
        Atomic<Cluster<'a, ClusterTag, EventualConsistency>>,
        Unbounded,
        TotalOrder,
    >,
    /// Requests that arrived at a member that was not the leader, paired with the
    /// best-known leader hint for the client to retry against (`None` when the
    /// member does not know who leads).
    pub redirected: Stream<
        (T, Option<MemberId<ClusterTag>>),
        Cluster<'a, ClusterTag, NoConsistency>,
        Unbounded,
        TotalOrder,
    >,
    /// Each member's view transitions (term and known leader), emitted whenever the
    /// view changes. Useful for tests and observability; may be left unobserved.
    pub leader_views: Stream<LeaderView<ClusterTag>, Cluster<'a, ClusterTag>>,
}

/// The unified RAFT server: one state machine per member, advanced by [`raft_step`]
/// inside a single tick that consumes every input the protocol reacts to (client
/// requests, both timers, and all intra-cluster traffic over one channel).
///
/// See [`raft`] for the input/timer contracts. The only feedback loop — a member's
/// outbound messages reaching other members' inputs — passes through the network
/// demux, which is an asynchronous boundary, so no in-tick cycle exists and no
/// tick-deferral plumbing is needed.
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
    Net: NetworkFor<RaftRpc<T, ClusterTag>>,
    NoOrder: MinOrder<Net::OrderingGuarantee, Min = NoOrder>,
{
    let cluster_size = config.cluster_size;

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
            Stream<
                (MemberId<ClusterTag>, RaftRpc<T, ClusterTag>),
                Cluster<'a, ClusterTag>,
                Unbounded,
                NoOrder,
            >,
        >,
        Stream<
            (MemberId<ClusterTag>, RaftRpc<T, ClusterTag>),
            Cluster<'a, ClusterTag>,
            Unbounded,
            NoOrder,
        >,
    ) = cluster.forward_ref();

    // The cluster membership list, resolved on each member at runtime (identical
    // everywhere, since it comes from deploy/sim metadata).
    let LocationId::Cluster(cluster_key) = Location::id(cluster) else {
        unreachable!("raft_server always runs on a cluster")
    };
    let cluster_members = ClusterIds {
        key: cluster_key,
        _phantom: PhantomData,
    };

    #[expect(
        clippy::type_complexity,
        reason = "the sliced! outputs are annotated with their full stream types"
    )]
    let (outbound_messages, committed, redirected, view_transitions): (
        Stream<(MemberId<ClusterTag>, RaftRpc<T, ClusterTag>), Cluster<'a, ClusterTag>>,
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
            /// itself: every message is processed atomically against the member's
            /// full state.
            nondet_raft
        ));

        let mut server_state = use::state(|l| l.singleton(q!(RaftServerState::new())));

        let tick = request_batch.location().clone();
        let election_fired = election_batch.count().map(q!(|n| n > 0));
        let heartbeat_fired = heartbeat_batch.count().map(q!(|n| n > 0));

        // Requests in the order fixed by assume_ordering (TotalOrder fold).
        let request_vec = request_batch.fold(
            q!(|| Vec::new()),
            q!(|requests, request| {
                requests.push(request);
            }),
        );

        // Received messages as an unordered batch; raft_step sorts them into a
        // canonical order, so downstream results depend only on the batch multiset.
        let message_vec = traffic_batch.fold(
            q!(|| Vec::new()),
            q!(
                |messages, message| {
                    messages.push(message);
                },
                commutative = manual_proof!(
                    /** the accumulated batch is sorted into a canonical order by
                    raft_step before being applied, so downstream results depend only
                    on the batch multiset, never on arrival order */
                )
            ),
        );

        // The other members of the cluster, the broadcast targets.
        let other_members = tick.singleton(q!(
            cluster_members
                .iter()
                .map(|id| MemberId::from_tagless(id.clone()))
                .filter(|member| *member != CLUSTER_SELF_ID)
                .collect::<Vec<_>>()
        ));

        // Reference handles for this tick's aggregates, the persistent state, and the
        // side-channel outputs: reads via `by_ref` (`&T`), writes via `by_mut`
        // (`&mut T` on the state, `&mut Vec<T>` on streams). The step is hosted on a
        // one-element-per-tick trigger stream whose output is the outbound message
        // stream itself — which also guarantees the step is never dead-code pruned.
        let state_ref = server_state.by_mut();
        let election_fired_ref = election_fired.by_ref();
        let heartbeat_fired_ref = heartbeat_fired.by_ref();
        let request_vec_ref = request_vec.by_ref();
        let message_vec_ref = message_vec.by_ref();
        let other_members_ref = other_members.by_ref();

        let committed: Stream<LogEntry<T>, _, Bounded> = tick.source_iter(q!(Vec::new()));
        let committed_ref = committed.by_mut();
        let redirected: Stream<(T, Option<MemberId<ClusterTag>>), _, Bounded> =
            tick.source_iter(q!(Vec::new()));
        let redirected_ref = redirected.by_mut();
        let view_transitions: Stream<LeaderView<ClusterTag>, _, Bounded> =
            tick.source_iter(q!(Vec::new()));
        let view_transitions_ref = view_transitions.by_mut();

        // The entire protocol step: hand this tick's aggregates to the pure
        // sequential state machine and distribute its outputs.
        let outbound = tick
            .singleton(q!(()))
            .into_stream()
            .flat_map_ordered(q!(move |_| {
                let output = crate::cluster::raft::raft_step(
                    &mut *state_ref,
                    RaftStepInput {
                        me: CLUSTER_SELF_ID.clone(),
                        other_members: other_members_ref.clone(),
                        // Written without field shorthand: stageleft renames captured
                        // free variables, and shorthand hides the use site from it.
                        cluster_size: { cluster_size },
                        election_timer_fired: *election_fired_ref,
                        heartbeat_timer_fired: *heartbeat_fired_ref,
                        requests: request_vec_ref.clone(),
                        messages: message_vec_ref.clone(),
                    },
                );
                for entry in output.committed {
                    committed_ref.push(entry);
                }
                for redirect in output.redirected {
                    redirected_ref.push(redirect);
                }
                if let Some(view) = output.view_transition {
                    view_transitions_ref.push(view);
                }
                output.outbound
            }));

        (outbound, committed, redirected, view_transitions)
    };

    traffic_handle.complete(outbound_messages
            .into_keyed()
            // The channel's fault model is chosen by the caller: sim tests use
            // `TCP.fail_stop()` (the simulator cannot explore lossy channels without
            // giving up liveness assertions), while real deployments that face
            // partitions pass `TCP.lossy_delayed_forever()` — RAFT tolerates the
            // loss: lost AppendEntries (or replies) are re-sent on the next
            // heartbeat, and lost vote traffic is retried at the next election
            // timeout.
            .demux(cluster, net)
            .entries());

    RaftOutputs {
        // The RAFT protocol itself is what justifies the consistency cast: every
        // member emits the same totally-ordered committed sequence, eventually.
        committed: committed
            .assert_has_consistency_of::<Cluster<'a, ClusterTag, EventualConsistency>>(
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
/// # Design: one state machine, one tick
///
/// Each member runs a single sequential state machine ([`raft_step`]) over the
/// complete RAFT state ([`RaftServerState`]) inside one tick, fed by one intra-cluster
/// channel. This is a deliberate safety property, not a stylistic choice: RAFT's
/// proofs interlock vote decisions and log/ack decisions through shared state
/// (`currentTerm`, `votedFor`, the log), and any design that splits that state across
/// asynchronously-connected components re-introduces windows where one half acts on a
/// stale view of the other. (An earlier two-component design here did exactly that,
/// and the simulator reliably reproduced committed-entry truncation; see the
/// regression tests `concurrent_elections_never_fork_the_committed_log` and
/// `fully_concurrent_run_never_forks_the_committed_log`.)
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
    requests: Stream<T, Cluster<'a, ClusterTag, Con>, Unbounded, O>,
    election_timer_interrupts: Stream<(), Cluster<'a, ClusterTag>>,
    heartbeat_timer_interrupts: Stream<(), Cluster<'a, ClusterTag>>,
    config: RaftConfig,
    net: impl Fn() -> Net,
    nondet_order: NonDet,
) -> (
    Stream<
        LogEntry<T>,
        Atomic<Cluster<'a, ClusterTag, EventualConsistency>>,
        Unbounded,
        TotalOrder,
    >,
    Stream<
        (T, Option<MemberId<ClusterTag>>),
        Cluster<'a, ClusterTag, NoConsistency>,
        Unbounded,
        TotalOrder,
    >,
)
where
    T: Clone + Serialize + DeserializeOwned + 'a,
    O: Ordering,
    Con: Consistency,
    ClusterTag: 'a,
    Net: NetworkFor<RaftRpc<T, ClusterTag>>,
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
        AppendEntriesRequest, LeaderView, LogEntry, RaftConfig, RaftRpc, RaftServerState,
        RaftState, RaftStepInput, Replica, RequestVoteDto, raft, raft_server, raft_step,
    };

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
        let cluster = flow.cluster::<Replica>();

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
        let cluster = flow.cluster::<Replica>();

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

    /// Drives N deterministic [`RaftServerState`] machines connected by an in-memory
    /// "network" of per-member inboxes, calling [`raft_step`] directly. Protocol-rule
    /// tests run on this instead of the simulator: they need precise, deterministic
    /// control over delivery and timers (e.g. "this entry is never replicated"),
    /// while schedule exploration is covered by the simulation tests below.
    ///
    /// Members `deployed..cluster_size` model crashed members: they exist in the
    /// protocol configuration (majorities count them) but never take a step, so
    /// messages to them pile up unanswered.
    struct StepCluster {
        states: Vec<RaftServerState<String, Replica>>,
        inboxes: Vec<Vec<(MemberId<Replica>, RaftRpc<String, Replica>)>>,
        committed: Vec<Vec<LogEntry<String>>>,
        redirected: Vec<Vec<(String, Option<MemberId<Replica>>)>>,
        cluster_size: usize,
        deployed: usize,
    }

    impl StepCluster {
        fn new(deployed: usize, cluster_size: usize) -> Self {
            StepCluster {
                states: (0..cluster_size).map(|_| RaftServerState::new()).collect(),
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
                RaftStepInput {
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
            while self.states[member].role == RaftState::Follower {
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
                self.states[member].role == RaftState::Leader,
                "member {} failed to win its election (role {:?}, term {})",
                member,
                self.states[member].role,
                self.states[member].term
            );
            self.states[member].term
        }

        /// Installs `member` as the leader of `term` directly, modeling figure 8's
        /// crash-and-reelect histories without simulating the message loss that
        /// produces them (exactly what the pre-unification sim tests did by
        /// injecting `LeaderView`s). The state produced is a protocol-reachable
        /// leadership: term advanced, vote self-recorded, follower bookkeeping reset.
        fn crown(&mut self, member: usize, term: usize) {
            assert!(term > self.states[member].term, "terms only move forward");
            let log_len = self.states[member].log.len();
            let state = &mut self.states[member];
            state.term = term;
            state.role = RaftState::Leader;
            state.voted_for = Some(Self::id(member));
            state.votes.clear();
            state.known_leader = None;
            state.next_index.clear();
            state.match_index.clear();
            for target in 0..self.cluster_size {
                if target != member {
                    state.next_index.insert(Self::id(target), log_len + 1);
                    state.match_index.insert(Self::id(target), 0);
                }
            }
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

    /// The interlock that motivated unifying the protocol state into one step (the
    /// cross-tick race fixed by this design; see [`raft`]'s docs): a member's vote
    /// decisions and its log/ack decisions must be read-modify-writes against the
    /// same state, in both directions, even when the triggering messages arrive in
    /// the same batch.
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
            RaftRpc::AppendEntries(AppendEntriesRequest {
                term: 1,
                leader: leader_0.clone(),
                prev_log_index: prev,
                prev_log_term: if prev == 0 { 0 } else { 1 },
                entries,
                leader_commit: 0,
            })
        };

        // Ack-then-vote: member 1 acknowledges entry 1 from the term-1 leader, and
        // *in the same batch* receives a term-2 vote request from a candidate whose
        // log is empty. Whatever the processing order, the two decisions must never
        // combine into "acked the entry AND endorsed a candidate missing it": the
        // canonical order processes the vote first (empty-vs-empty log: granted,
        // term becomes 2), which forces the term-1 append to be rejected.
        let mut state: RaftServerState<String, Replica> = RaftServerState::new();
        let output = raft_step(
            &mut state,
            RaftStepInput {
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
                        RaftRpc::RequestVote(RequestVoteDto {
                            term: 2,
                            last_log_index: 0,
                            last_log_term: 0,
                        }),
                    ),
                ],
            },
        );
        assert_eq!(state.term, 2, "the vote request's term must be adopted");
        assert_eq!(
            state.voted_for,
            Some(candidate_2.clone()),
            "the empty-log candidate is legitimately granted against an empty log"
        );
        assert!(
            state.log.is_empty(),
            "having endorsed a term-2 candidate, the term-1 append must be rejected"
        );
        let acked = output.outbound.iter().any(|(_, message)| {
            matches!(
                message,
                RaftRpc::AppendEntriesReply(reply) if reply.success
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
            RaftStepInput {
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
            state.log.is_empty(),
            "a stale-term append must stay rejected"
        );
        assert!(matches!(
            &output.outbound[..],
            [(target, RaftRpc::AppendEntriesReply(reply))]
                if *target == leader_0 && !reply.success && reply.term == 2
        ));

        // Ack-then-vote, later ticks: a member whose log holds an acknowledged entry
        // refuses a later candidate that lacks it (§5.4.1 against the live log).
        let mut state: RaftServerState<String, Replica> = RaftServerState::new();
        raft_step(
            &mut state,
            RaftStepInput {
                me: StepCluster::id(1),
                other_members: vec![StepCluster::id(0), StepCluster::id(2)],
                cluster_size: 3,
                election_timer_fired: false,
                heartbeat_timer_fired: false,
                requests: vec![],
                messages: vec![(leader_0.clone(), append(vec![entry(1)], 0))],
            },
        );
        assert_eq!(
            state.log.len(),
            1,
            "the term-1 append is accepted this time"
        );
        raft_step(
            &mut state,
            RaftStepInput {
                me: StepCluster::id(1),
                other_members: vec![StepCluster::id(0), StepCluster::id(2)],
                cluster_size: 3,
                election_timer_fired: false,
                heartbeat_timer_fired: false,
                requests: vec![],
                messages: vec![(
                    candidate_2,
                    RaftRpc::RequestVote(RequestVoteDto {
                        term: 2,
                        last_log_index: 0,
                        last_log_term: 0,
                    }),
                )],
            },
        );
        assert_eq!(
            state.term, 2,
            "the higher term is adopted even when refusing"
        );
        assert_eq!(
            state.voted_for, None,
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
        let mut state: RaftServerState<String, Replica> = RaftServerState::new();
        // Install member 0 as leader of term 1.
        state.term = 1;
        state.role = RaftState::Leader;
        state.voted_for = Some(StepCluster::id(0));
        state.next_index.insert(StepCluster::id(1), 1);
        state.match_index.insert(StepCluster::id(1), 0);

        let output = raft_step(
            &mut state,
            RaftStepInput {
                me: StepCluster::id(0),
                other_members: vec![StepCluster::id(1), StepCluster::id(2)],
                cluster_size: 3,
                election_timer_fired: false,
                heartbeat_timer_fired: false,
                requests: vec![],
                messages: vec![(
                    StepCluster::id(1),
                    RaftRpc::AppendEntriesReply(super::AppendEntriesReply {
                        term: 3,
                        success: false,
                        match_index: 0,
                    }),
                )],
            },
        );
        assert_eq!(state.term, 3, "the leader must adopt the higher term");
        assert_eq!(
            state.role,
            RaftState::Follower,
            "the leader must step down to follower on a higher-term reply"
        );
        assert_eq!(
            state.voted_for, None,
            "voted_for must be cleared when stepping down"
        );
        assert!(
            state.votes.is_empty(),
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
        cluster.states[0].term = 1;
        cluster.states[0].log = (1..=3)
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
            cluster.states[1].role == RaftState::Candidate,
            "the stale-logged member 1 must stall as a candidate, got {:?}",
            cluster.states[1].role
        );
        assert_eq!(
            cluster.states[0].voted_for, None,
            "member 0 must not vote for a candidate whose log is behind its own"
        );

        // Phase 2: member 0 campaigns with the up-to-date log and must win. (Its
        // term already advanced past member 1's candidacy via the vote request.)
        let term = cluster.elect(0);
        assert!(
            term > cluster.states[1].term || cluster.states[1].term == term,
            "the election settles a single term"
        );
        assert_eq!(
            cluster.states[1].voted_for,
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
    /// commit entirely on its own. (Leadership is installed via `crown`, exactly as
    /// the pre-unification test installed it via an injected view: a 1-of-3 member
    /// cannot win a real election, which is the point.)
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
            cluster.states[0].voted_for, None,
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
    /// installed with `crown` (as the pre-unification test did with injected views)
    /// while replication runs the real protocol:
    /// * Term 1: member 0 appends "old" but never heartbeats it (figure 8(a)).
    /// * Term 2: member 0, re-crowned, replicates "old" everywhere — match_index
    ///   reaches 1 on every member, beyond a majority, yet nothing may commit:
    ///   "old" is a term-1 entry under a term-2 leader. This is the trap that catches
    ///   a dropped §5.4.2 restriction (figure 8(c)).
    /// * Term 3: member 2 appends "conflict" at index 2, unreplicated (figure 8's S5;
    ///   the common prefix keeps later empty heartbeats from disturbing it).
    /// * Term 4: member 0, re-crowned, appends "winner" at index 2, unreplicated —
    ///   the doomed entry.
    /// * Term 5: member 2 is crowned; its log is authoritative: the next_index
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
            (1..3).all(|member| cluster.states[member].log.len() == 1),
            "\"old\" must be replicated to every member"
        );
        assert!(
            (1..3).all(|member| { cluster.states[0].match_index[&StepCluster::id(member)] == 1 }),
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
        let cluster = flow.cluster::<Replica>();

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

        // The committed stream carries a consistency assertion (see log_replication);
        // the simulator cannot validate those yet and asks tests to skip them.
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

    /// **Regression net for a fixed cross-tick race.** Before the protocol state was
    /// unified into one tick, this test reliably reproduced a real safety bug (6/6
    /// fuzz runs, usually within the first few iterations): `leader_election` and
    /// `log_replication` kept their halves of the state (`term`/`voted_for` vs.
    /// `log`/acks) in separate ticks connected by deferred feedback streams, so a
    /// voter could grant a vote using a log summary lagging its just-acked entries,
    /// or ack an old-term leader after voting in a newer term. Either window let an
    /// entry commit via acks from members that simultaneously helped elect a leader
    /// missing that entry — the new leader then truncated a committed entry
    /// (surfacing as the truncation-guard panic, `protocol violation: asked to
    /// truncate committed entry`). Notably no message loss was needed: plain
    /// `fail_stop` channels and interleaving alone sufficed.
    ///
    /// The fix makes the vote/ack interlock atomic by construction: one state
    /// machine ([`raft_step`]) over one [`RaftServerState`] in one tick (see
    /// [`raft`]'s design notes). This test keeps exploring the schedules that used
    /// to break it.
    ///
    /// Detection is safety-only, in two layers:
    ///
    /// 1. Every member's committed history must stay pairwise prefix-consistent —
    ///    members may lag, but may never emit a *different* entry at the same
    ///    position.
    /// 2. [`raft_step`] asserts a member is never asked to truncate at or below
    ///    its own commit index; the simulator surfaces that panic as a failure.
    ///
    /// The choreography deliberately overlaps replication and elections without
    /// intermediate quiescence: each round primes a challenger's heartbeat-
    /// suppression flag, then in one un-quiesced burst sends a fresh request to
    /// member 0, pumps its heartbeat timer (pushing AppendEntries with the new
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
        let cluster = flow.cluster::<Replica>();

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

        // The committed stream carries a consistency assertion (see log_replication);
        // the simulator cannot validate those yet and asks tests to skip them.
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

    /// **Regression net for the fixed cross-tick race** (same underlying bug as
    /// [`concurrent_elections_never_fork_the_committed_log`], where the history is
    /// documented; before the fix this reproduced it in 2 of 3 runs).
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
    /// 3. [`raft_step`]'s truncation guard never fires (surfaces as a panic /
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
        let cluster = flow.cluster::<Replica>();

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

        // The committed stream carries a consistency assertion (see log_replication);
        // the simulator cannot validate those yet and asks tests to skip them.
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
            )));

        let election_timer_interrupts = replicas.source_interval(q!(
            std::time::Duration::from_millis(500 + u64::from(CLUSTER_SELF_ID.get_raw_id()) * 130)
        ));
        let heartbeat_timer_interrupts =
            replicas.source_interval(q!(std::time::Duration::from_millis(100)));

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
        let replicas = builder.cluster::<Replica>();
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
