//! Raft with logical configuration changes over a statically deployed Hydro cluster.
//!
//! Every process that may ever vote is present in the fixed Hydro [`Cluster`]. Raft log
//! entries select a changing subset of that universe as the logical voter configuration;
//! this module never changes Hydro cluster membership.
//!
//! Reconfiguration follows the single-server algorithm in Ongaro's dissertation, with
//! the 2015 safety correction: a newly elected leader appends a no-op and may not append
//! a configuration until it has committed an entry from its current term. Additions first
//! replicate to the prospective server as a non-voting learner. The catch-up test here is
//! deliberately the simple `matchIndex == leader last index` safety-oriented criterion;
//! the dissertation's multi-round/time-bound heuristic is an availability refinement.

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

use super::raft::{LeaderView, RaftState};

/// A canonical logical voter set. All members must belong to the fixed physical universe.
#[derive(Serialize, Deserialize)]
#[serde(bound = "")]
pub struct Configuration<ClusterTag> {
    pub voters: Vec<MemberId<ClusterTag>>,
}

impl<C> Configuration<C> {
    pub fn new(mut voters: Vec<MemberId<C>>) -> Result<Self, ReconfigurationError> {
        voters.sort();
        voters.dedup();
        if voters.is_empty() {
            return Err(ReconfigurationError::EmptyConfiguration);
        }
        Ok(Self { voters })
    }

    pub fn voters(&self) -> &[MemberId<C>] {
        &self.voters
    }

    pub fn contains(&self, member: &MemberId<C>) -> bool {
        self.voters.binary_search(member).is_ok()
    }

    pub fn majority(&self) -> usize {
        self.voters.len() / 2 + 1
    }

    pub fn with_added(&self, member: MemberId<C>) -> Result<Self, ReconfigurationError> {
        if self.contains(&member) {
            return Err(ReconfigurationError::AlreadyVoter);
        }
        let mut voters = self.voters.clone();
        voters.push(member);
        Self::new(voters)
    }

    pub fn with_removed(&self, member: &MemberId<C>) -> Result<Self, ReconfigurationError> {
        if !self.contains(member) {
            return Err(ReconfigurationError::NotVoter);
        }
        let voters = self
            .voters
            .iter()
            .filter(|candidate| *candidate != member)
            .cloned()
            .collect();
        Self::new(voters)
    }
}

impl<C> Clone for Configuration<C> {
    fn clone(&self) -> Self {
        Self {
            voters: self.voters.clone(),
        }
    }
}
impl<C> Debug for Configuration<C> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("Configuration").field(&self.voters).finish()
    }
}
impl<C> PartialEq for Configuration<C> {
    fn eq(&self, other: &Self) -> bool {
        self.voters == other.voters
    }
}
impl<C> Eq for Configuration<C> {}

/// Payloads in the internal Raft log. Only `Command` values leave the public commit port.
#[derive(Serialize, Deserialize)]
#[serde(bound(
    serialize = "T: Serialize",
    deserialize = "T: serde::de::DeserializeOwned"
))]
pub enum DynLogPayload<T, ClusterTag> {
    Command(T),
    Noop,
    Configuration(Configuration<ClusterTag>),
}

impl<T: Clone, C> Clone for DynLogPayload<T, C> {
    fn clone(&self) -> Self {
        match self {
            Self::Command(value) => Self::Command(value.clone()),
            Self::Noop => Self::Noop,
            Self::Configuration(config) => Self::Configuration(config.clone()),
        }
    }
}
impl<T: Debug, C> Debug for DynLogPayload<T, C> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Command(value) => f.debug_tuple("Command").field(value).finish(),
            Self::Noop => f.write_str("Noop"),
            Self::Configuration(config) => f.debug_tuple("Configuration").field(config).finish(),
        }
    }
}
impl<T: PartialEq, C> PartialEq for DynLogPayload<T, C> {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Command(a), Self::Command(b)) => a == b,
            (Self::Noop, Self::Noop) => true,
            (Self::Configuration(a), Self::Configuration(b)) => a == b,
            _ => false,
        }
    }
}
impl<T: Eq, C> Eq for DynLogPayload<T, C> {}

#[derive(Serialize, Deserialize)]
#[serde(bound(
    serialize = "T: Serialize",
    deserialize = "T: serde::de::DeserializeOwned"
))]
pub struct DynLogEntry<T, ClusterTag> {
    pub payload: DynLogPayload<T, ClusterTag>,
    pub term: usize,
    pub index: usize,
}
impl<T: Clone, C> Clone for DynLogEntry<T, C> {
    fn clone(&self) -> Self {
        Self {
            payload: self.payload.clone(),
            term: self.term,
            index: self.index,
        }
    }
}
impl<T: Debug, C> Debug for DynLogEntry<T, C> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DynLogEntry")
            .field("payload", &self.payload)
            .field("term", &self.term)
            .field("index", &self.index)
            .finish()
    }
}
impl<T: PartialEq, C> PartialEq for DynLogEntry<T, C> {
    fn eq(&self, other: &Self) -> bool {
        self.payload == other.payload && self.term == other.term && self.index == other.index
    }
}
impl<T: Eq, C> Eq for DynLogEntry<T, C> {}

/// An application command after it commits. Internal no-ops/configurations are omitted.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct CommittedCommand<T> {
    pub message: T,
    pub term: usize,
    pub index: usize,
}

#[derive(Serialize, Deserialize)]
#[serde(bound = "")]
pub enum ConfigurationChange<ClusterTag> {
    Add(MemberId<ClusterTag>),
    Remove(MemberId<ClusterTag>),
}
impl<C> Clone for ConfigurationChange<C> {
    fn clone(&self) -> Self {
        match self {
            Self::Add(id) => Self::Add(id.clone()),
            Self::Remove(id) => Self::Remove(id.clone()),
        }
    }
}
impl<C> Debug for ConfigurationChange<C> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Add(id) => f.debug_tuple("Add").field(id).finish(),
            Self::Remove(id) => f.debug_tuple("Remove").field(id).finish(),
        }
    }
}

#[derive(Serialize, Deserialize)]
#[serde(bound = "")]
pub struct ReconfigurationRequest<ClusterTag> {
    pub request_id: u64,
    pub change: ConfigurationChange<ClusterTag>,
}
impl<C> Clone for ReconfigurationRequest<C> {
    fn clone(&self) -> Self {
        Self {
            request_id: self.request_id,
            change: self.change.clone(),
        }
    }
}
impl<C> Debug for ReconfigurationRequest<C> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ReconfigurationRequest")
            .field("request_id", &self.request_id)
            .field("change", &self.change)
            .finish()
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub enum ReconfigurationError {
    EmptyConfiguration,
    UnknownPhysicalMember,
    AlreadyVoter,
    NotVoter,
    ChangeInProgress,
}

#[derive(Serialize, Deserialize)]
#[serde(bound = "")]
pub enum ReconfigurationResult<ClusterTag> {
    Completed {
        request_id: u64,
        configuration: Configuration<ClusterTag>,
    },
    NotLeader {
        request_id: u64,
        leader: Option<MemberId<ClusterTag>>,
    },
    Rejected {
        request_id: u64,
        error: ReconfigurationError,
    },
}
impl<C> Clone for ReconfigurationResult<C> {
    fn clone(&self) -> Self {
        match self {
            Self::Completed {
                request_id,
                configuration,
            } => Self::Completed {
                request_id: *request_id,
                configuration: configuration.clone(),
            },
            Self::NotLeader { request_id, leader } => Self::NotLeader {
                request_id: *request_id,
                leader: leader.clone(),
            },
            Self::Rejected { request_id, error } => Self::Rejected {
                request_id: *request_id,
                error: error.clone(),
            },
        }
    }
}
impl<C> Debug for ReconfigurationResult<C> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Completed {
                request_id,
                configuration,
            } => f
                .debug_struct("Completed")
                .field("request_id", request_id)
                .field("configuration", configuration)
                .finish(),
            Self::NotLeader { request_id, leader } => f
                .debug_struct("NotLeader")
                .field("request_id", request_id)
                .field("leader", leader)
                .finish(),
            Self::Rejected { request_id, error } => f
                .debug_struct("Rejected")
                .field("request_id", request_id)
                .field("error", error)
                .finish(),
        }
    }
}

#[derive(Serialize, Deserialize)]
#[serde(bound(
    serialize = "T: Serialize",
    deserialize = "T: serde::de::DeserializeOwned"
))]
pub struct DynAppendEntries<T, C> {
    pub term: usize,
    pub leader: MemberId<C>,
    pub prev_log_index: usize,
    pub prev_log_term: usize,
    pub entries: Vec<DynLogEntry<T, C>>,
    pub leader_commit: usize,
}
impl<T: Clone, C> Clone for DynAppendEntries<T, C> {
    fn clone(&self) -> Self {
        Self {
            term: self.term,
            leader: self.leader.clone(),
            prev_log_index: self.prev_log_index,
            prev_log_term: self.prev_log_term,
            entries: self.entries.clone(),
            leader_commit: self.leader_commit,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct DynRequestVote {
    pub term: usize,
    pub last_log_index: usize,
    pub last_log_term: usize,
}
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct DynRequestVoteResponse {
    pub term: usize,
}
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct DynAppendEntriesReply {
    pub term: usize,
    pub success: bool,
    pub match_index: usize,
}

#[derive(Serialize, Deserialize)]
#[serde(bound(
    serialize = "T: Serialize",
    deserialize = "T: serde::de::DeserializeOwned"
))]
pub enum DynRaftRpc<T, C> {
    RequestVote(DynRequestVote),
    RequestVoteResponse(DynRequestVoteResponse),
    AppendEntries(DynAppendEntries<T, C>),
    AppendEntriesReply(DynAppendEntriesReply),
}
impl<T: Clone, C> Clone for DynRaftRpc<T, C> {
    fn clone(&self) -> Self {
        match self {
            Self::RequestVote(value) => Self::RequestVote(value.clone()),
            Self::RequestVoteResponse(value) => Self::RequestVoteResponse(value.clone()),
            Self::AppendEntries(value) => Self::AppendEntries(value.clone()),
            Self::AppendEntriesReply(value) => Self::AppendEntriesReply(value.clone()),
        }
    }
}

#[derive(Clone)]
#[doc(hidden)]
pub enum PendingStage {
    CatchingUp,
    AwaitingCommit { index: usize },
}
#[doc(hidden)]
pub struct PendingReconfiguration<C> {
    pub request: ReconfigurationRequest<C>,
    pub stage: PendingStage,
}
impl<C> Clone for PendingReconfiguration<C> {
    fn clone(&self) -> Self {
        Self {
            request: self.request.clone(),
            stage: self.stage.clone(),
        }
    }
}

/// Complete persistent state for one logical-membership Raft participant.
pub struct DynRaftState<T, C> {
    pub term: usize,
    pub voted_for: Option<MemberId<C>>,
    pub role: RaftState,
    pub votes: HashSet<MemberId<C>>,
    pub heartbeat_seen: bool,
    pub known_leader: Option<MemberId<C>>,
    pub log: Vec<DynLogEntry<T, C>>,
    pub commit_index: usize,
    pub emitted_index: usize,
    pub next_index: HashMap<MemberId<C>, usize>,
    pub match_index: HashMap<MemberId<C>, usize>,
    pub initial_configuration: Configuration<C>,
    pub pending: Option<PendingReconfiguration<C>>,
}

impl<T, C> DynRaftState<T, C> {
    pub fn new(initial_configuration: Configuration<C>) -> Self {
        Self {
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
            initial_configuration,
            pending: None,
        }
    }

    pub fn effective_configuration(&self) -> Configuration<C> {
        self.log
            .iter()
            .rev()
            .find_map(|entry| match &entry.payload {
                DynLogPayload::Configuration(config) => Some(config.clone()),
                _ => None,
            })
            .unwrap_or_else(|| self.initial_configuration.clone())
    }

    pub fn latest_configuration_index(&self) -> usize {
        self.log
            .iter()
            .rev()
            .find(|entry| matches!(entry.payload, DynLogPayload::Configuration(_)))
            .map_or(0, |entry| entry.index)
    }

    pub fn last_log_position(&self) -> (usize, usize) {
        self.log
            .last()
            .map_or((0, 0), |entry| (entry.term, entry.index))
    }

    pub fn committed_current_term(&self) -> bool {
        self.commit_index > 0 && self.log[self.commit_index - 1].term == self.term
    }
}

impl<T: Clone, C> Clone for DynRaftState<T, C> {
    fn clone(&self) -> Self {
        Self {
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
            initial_configuration: self.initial_configuration.clone(),
            pending: self.pending.clone(),
        }
    }
}

pub struct DynRaftStepInput<T, C> {
    pub me: MemberId<C>,
    /// Every statically deployed member, including `me`.
    pub physical_members: Vec<MemberId<C>>,
    pub election_timer_fired: bool,
    pub heartbeat_timer_fired: bool,
    pub commands: Vec<T>,
    pub reconfigurations: Vec<ReconfigurationRequest<C>>,
    pub messages: Vec<(MemberId<C>, DynRaftRpc<T, C>)>,
}

pub struct DynRaftStepOutput<T, C> {
    pub outbound: Vec<(MemberId<C>, DynRaftRpc<T, C>)>,
    pub committed_entries: Vec<DynLogEntry<T, C>>,
    pub committed_commands: Vec<CommittedCommand<T>>,
    pub redirected: Vec<(T, Option<MemberId<C>>)>,
    pub reconfiguration_results: Vec<ReconfigurationResult<C>>,
    pub view_transition: Option<LeaderView<C>>,
}

#[doc(hidden)]
#[derive(Clone, Copy)]
pub struct StepPolicy {
    pub require_current_term_commit: bool,
}
#[doc(hidden)]
pub const SAFE_POLICY: StepPolicy = StepPolicy {
    require_current_term_commit: true,
};

pub fn dyn_raft_step<T: Clone, C>(
    state: &mut DynRaftState<T, C>,
    input: DynRaftStepInput<T, C>,
) -> DynRaftStepOutput<T, C> {
    dyn_raft_step_with_policy(state, input, SAFE_POLICY)
}

/// Runs the dissertation's original, unsafe single-server-change rule.
///
/// This exists only to let simulation demonstrate the 2015 counterexample: it omits the
/// current-term commit barrier and, faithfully to the dissertation, truncates conflicting
/// log suffixes unconditionally — even below the commit index — so the violation appears
/// as divergent committed outputs rather than aborting inside the compiled simulation.
/// Normal callers must use [`dyn_raft_step`], which always enforces the barrier and keeps
/// the committed-entry truncation guard.
#[doc(hidden)]
pub fn dyn_raft_step_unpatched_for_simulation<T: Clone, C>(
    state: &mut DynRaftState<T, C>,
    input: DynRaftStepInput<T, C>,
) -> DynRaftStepOutput<T, C> {
    dyn_raft_step_with_policy(
        state,
        input,
        StepPolicy {
            require_current_term_commit: false,
        },
    )
}

#[doc(hidden)]
pub fn dyn_raft_step_with_policy<T: Clone, C>(
    state: &mut DynRaftState<T, C>,
    input: DynRaftStepInput<T, C>,
    policy: StepPolicy,
) -> DynRaftStepOutput<T, C> {
    let DynRaftStepInput {
        me,
        mut physical_members,
        election_timer_fired,
        heartbeat_timer_fired,
        commands,
        reconfigurations,
        mut messages,
    } = input;
    physical_members.sort();
    physical_members.dedup();
    assert!(
        physical_members.binary_search(&me).is_ok(),
        "self absent from physical universe"
    );

    let old_view = LeaderView {
        term: state.term,
        leader: if state.role == RaftState::Leader {
            Some(me.clone())
        } else {
            state.known_leader.clone()
        },
    };
    let mut outbound = Vec::new();
    let mut committed_entries = Vec::new();
    let mut committed_commands = Vec::new();
    let mut redirected = Vec::new();
    let mut reconfiguration_results = Vec::new();

    fn observe_term<T, C>(state: &mut DynRaftState<T, C>, term: usize) -> bool {
        if term > state.term {
            state.term = term;
            state.role = RaftState::Follower;
            state.voted_for = None;
            state.votes.clear();
            state.known_leader = None;
        }
        term == state.term
    }

    fn append<T, C>(state: &mut DynRaftState<T, C>, payload: DynLogPayload<T, C>) -> usize {
        let index = state.log.len() + 1;
        state.log.push(DynLogEntry {
            payload,
            term: state.term,
            index,
        });
        index
    }

    fn become_leader<T, C>(
        state: &mut DynRaftState<T, C>,
        me: &MemberId<C>,
        physical_members: &[MemberId<C>],
    ) {
        state.role = RaftState::Leader;
        state.known_leader = None;
        state.next_index.clear();
        state.match_index.clear();
        for member in physical_members {
            if member != me {
                state.next_index.insert(member.clone(), state.log.len() + 1);
                state.match_index.insert(member.clone(), 0);
            }
        }
        // The no-op both commits inherited entries transitively and is the 2015
        // reconfiguration barrier once a majority stores it.
        append(state, DynLogPayload::Noop);
    }

    fn sort_key<T, C>(rpc: &DynRaftRpc<T, C>) -> (u8, usize, usize, usize) {
        match rpc {
            DynRaftRpc::RequestVote(value) => {
                (0, value.term, value.last_log_term, value.last_log_index)
            }
            DynRaftRpc::RequestVoteResponse(value) => (1, value.term, 0, 0),
            DynRaftRpc::AppendEntries(value) => {
                (2, value.term, value.prev_log_index, value.entries.len())
            }
            DynRaftRpc::AppendEntriesReply(value) => {
                (3, value.term, value.match_index, usize::from(value.success))
            }
        }
    }
    messages.sort_by(|(a_sender, a), (b_sender, b)| {
        sort_key(a)
            .cmp(&sort_key(b))
            .then_with(|| a_sender.cmp(b_sender))
    });

    let was_leader = state.role == RaftState::Leader;
    for (sender, message) in messages {
        match message {
            DynRaftRpc::RequestVote(request) => {
                if !observe_term(state, request.term) {
                    continue;
                }
                let up_to_date =
                    (request.last_log_term, request.last_log_index) >= state.last_log_position();
                let can_vote =
                    state.voted_for.is_none() || state.voted_for.as_ref() == Some(&sender);
                // Recipients do not consult local configuration when processing RPCs.
                if up_to_date && can_vote {
                    state.voted_for = Some(sender.clone());
                    outbound.push((
                        sender,
                        DynRaftRpc::RequestVoteResponse(DynRequestVoteResponse {
                            term: state.term,
                        }),
                    ));
                }
            }
            DynRaftRpc::RequestVoteResponse(response) => {
                if !observe_term(state, response.term) || state.role != RaftState::Candidate {
                    continue;
                }
                let configuration = state.effective_configuration();
                if configuration.contains(&sender) {
                    state.votes.insert(sender);
                }
                if state.votes.len() >= configuration.majority() {
                    become_leader(state, &me, &physical_members);
                }
            }
            DynRaftRpc::AppendEntries(request) => {
                if !observe_term(state, request.term) {
                    outbound.push((
                        sender,
                        DynRaftRpc::AppendEntriesReply(DynAppendEntriesReply {
                            term: state.term,
                            success: false,
                            match_index: 0,
                        }),
                    ));
                    continue;
                }
                assert!(
                    state.role != RaftState::Leader,
                    "two leaders in term {}",
                    state.term
                );
                state.heartbeat_seen = true;
                state.role = RaftState::Follower;
                state.known_leader = Some(request.leader);
                let matches = request.prev_log_index == 0
                    || (state.log.len() >= request.prev_log_index
                        && state.log[request.prev_log_index - 1].term == request.prev_log_term);
                if !matches {
                    outbound.push((
                        sender,
                        DynRaftRpc::AppendEntriesReply(DynAppendEntriesReply {
                            term: state.term,
                            success: false,
                            match_index: 0,
                        }),
                    ));
                    continue;
                }
                let new_match = request.prev_log_index + request.entries.len();
                for entry in request.entries {
                    if state.log.len() >= entry.index {
                        if state.log[entry.index - 1].term != entry.term {
                            // The dissertation algorithm truncates unconditionally. Keep
                            // that behavior verbatim under the unpatched policy so the
                            // 2015 violation surfaces as divergent committed outputs that
                            // a simulation oracle can observe; under the safe policy this
                            // guard asserts the situation is unreachable (a panic here
                            // inside the compiled simulation aborts the whole process).
                            if policy.require_current_term_commit {
                                assert!(
                                    entry.index > state.commit_index,
                                    "protocol violation: truncate committed entry {} (commit {})",
                                    entry.index,
                                    state.commit_index
                                );
                            }
                            state.log.truncate(entry.index - 1);
                            state.log.push(entry);
                        }
                    } else {
                        state.log.push(entry);
                    }
                }
                state.commit_index = state.commit_index.max(request.leader_commit.min(new_match));
                outbound.push((
                    sender,
                    DynRaftRpc::AppendEntriesReply(DynAppendEntriesReply {
                        term: state.term,
                        success: true,
                        match_index: new_match,
                    }),
                ));
            }
            DynRaftRpc::AppendEntriesReply(reply) => {
                if !observe_term(state, reply.term) || state.role != RaftState::Leader {
                    continue;
                }
                if reply.success {
                    let matched = state.match_index.entry(sender.clone()).or_insert(0);
                    *matched = (*matched).max(reply.match_index);
                    let next = state.next_index.entry(sender).or_insert(1);
                    *next = (*next).max(reply.match_index + 1);
                } else if let Some(next) = state.next_index.get_mut(&sender) {
                    *next = (*next - 1).max(1);
                }
            }
        }
    }

    if was_leader && state.role != RaftState::Leader {
        if let Some(pending) = state.pending.take() {
            reconfiguration_results.push(ReconfigurationResult::NotLeader {
                request_id: pending.request.request_id,
                leader: state.known_leader.clone(),
            });
        }
    }

    for command in commands {
        if state.role == RaftState::Leader {
            append(state, DynLogPayload::Command(command));
        } else {
            redirected.push((command, state.known_leader.clone()));
        }
    }

    for request in reconfigurations {
        if state.role != RaftState::Leader {
            reconfiguration_results.push(ReconfigurationResult::NotLeader {
                request_id: request.request_id,
                leader: state.known_leader.clone(),
            });
            continue;
        }
        if state.pending.is_some() {
            reconfiguration_results.push(ReconfigurationResult::Rejected {
                request_id: request.request_id,
                error: ReconfigurationError::ChangeInProgress,
            });
            continue;
        }
        let target = match &request.change {
            ConfigurationChange::Add(member) | ConfigurationChange::Remove(member) => member,
        };
        if physical_members.binary_search(target).is_err() {
            reconfiguration_results.push(ReconfigurationResult::Rejected {
                request_id: request.request_id,
                error: ReconfigurationError::UnknownPhysicalMember,
            });
            continue;
        }
        let config = state.effective_configuration();
        let validation = match &request.change {
            ConfigurationChange::Add(member) if config.contains(member) => {
                Some(ReconfigurationError::AlreadyVoter)
            }
            ConfigurationChange::Remove(member) if !config.contains(member) => {
                Some(ReconfigurationError::NotVoter)
            }
            ConfigurationChange::Remove(member) if config.voters.len() == 1 => {
                Some(ReconfigurationError::EmptyConfiguration)
            }
            _ => None,
        };
        if let Some(error) = validation {
            reconfiguration_results.push(ReconfigurationResult::Rejected {
                request_id: request.request_id,
                error,
            });
            continue;
        }
        let stage = match request.change {
            ConfigurationChange::Add(_) => PendingStage::CatchingUp,
            ConfigurationChange::Remove(_) => PendingStage::CatchingUp,
        };
        state.pending = Some(PendingReconfiguration { request, stage });
    }

    if election_timer_fired && state.role != RaftState::Leader {
        let configuration = state.effective_configuration();
        if state.heartbeat_seen {
            state.heartbeat_seen = false;
        } else if configuration.contains(&me) {
            state.term += 1;
            state.role = RaftState::Candidate;
            state.voted_for = Some(me.clone());
            state.votes.clear();
            state.votes.insert(me.clone());
            state.known_leader = None;
            if state.votes.len() >= configuration.majority() {
                become_leader(state, &me, &physical_members);
            } else {
                let (last_log_term, last_log_index) = state.last_log_position();
                for target in configuration.voters() {
                    if target != &me {
                        outbound.push((
                            target.clone(),
                            DynRaftRpc::RequestVote(DynRequestVote {
                                term: state.term,
                                last_log_index,
                                last_log_term,
                            }),
                        ));
                    }
                }
            }
        }
    }

    // Advance commit according to the leader's latest local configuration.
    if state.role == RaftState::Leader {
        let configuration = state.effective_configuration();
        let mut candidate = state.log.len();
        while candidate > state.commit_index {
            if state.log[candidate - 1].term == state.term {
                let acknowledgements = configuration
                    .voters()
                    .iter()
                    .filter(|member| {
                        if *member == &me {
                            true
                        } else {
                            state
                                .match_index
                                .get(*member)
                                .is_some_and(|matched| *matched >= candidate)
                        }
                    })
                    .count();
                if acknowledgements >= configuration.majority() {
                    state.commit_index = candidate;
                    break;
                }
            }
            candidate -= 1;
        }
    }

    // Progress the leader-owned AddServer/RemoveServer operation. The prior config
    // must be committed, and the patched algorithm also requires a current-term commit.
    if state.role == RaftState::Leader {
        let ready_for_configuration = state.latest_configuration_index() <= state.commit_index
            && (!policy.require_current_term_commit || state.committed_current_term());
        let append_config = state.pending.as_ref().and_then(|pending| {
            if !matches!(pending.stage, PendingStage::CatchingUp) || !ready_for_configuration {
                return None;
            }
            let current = state.effective_configuration();
            match &pending.request.change {
                ConfigurationChange::Add(member) => {
                    let caught_up = state
                        .match_index
                        .get(member)
                        .is_some_and(|matched| *matched >= state.log.len());
                    caught_up.then(|| current.with_added(member.clone()))
                }
                ConfigurationChange::Remove(member) => Some(current.with_removed(member)),
            }
        });
        if let Some(result) = append_config {
            match result {
                Ok(configuration) => {
                    let index = append(state, DynLogPayload::Configuration(configuration));
                    if let Some(pending) = &mut state.pending {
                        pending.stage = PendingStage::AwaitingCommit { index };
                    }
                }
                Err(error) => {
                    let pending = state.pending.take().expect("pending operation exists");
                    reconfiguration_results.push(ReconfigurationResult::Rejected {
                        request_id: pending.request.request_id,
                        error,
                    });
                }
            }
        }
        let completed = state
            .pending
            .as_ref()
            .and_then(|pending| match pending.stage {
                PendingStage::AwaitingCommit { index } if state.commit_index >= index => {
                    Some((pending.request.request_id, state.effective_configuration()))
                }
                _ => None,
            });
        if let Some((request_id, configuration)) = completed {
            state.pending = None;
            reconfiguration_results.push(ReconfigurationResult::Completed {
                request_id,
                configuration,
            });
            if !state.effective_configuration().contains(&me) {
                state.role = RaftState::Follower;
                state.known_leader = None;
            }
        }
    }

    if heartbeat_timer_fired && state.role == RaftState::Leader {
        let mut targets = state.effective_configuration().voters().to_vec();
        if let Some(PendingReconfiguration {
            request:
                ReconfigurationRequest {
                    change: ConfigurationChange::Add(member),
                    ..
                },
            stage: PendingStage::CatchingUp,
        }) = &state.pending
        {
            targets.push(member.clone());
        }
        targets.sort();
        targets.dedup();
        for follower in targets {
            if follower == me {
                continue;
            }
            let next = state.next_index.get(&follower).copied().unwrap_or(1);
            let prev_log_index = next - 1;
            let prev_log_term = if prev_log_index == 0 {
                0
            } else {
                state.log[prev_log_index - 1].term
            };
            outbound.push((
                follower,
                DynRaftRpc::AppendEntries(DynAppendEntries {
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

    while state.emitted_index < state.commit_index {
        state.emitted_index += 1;
        let entry = state.log[state.emitted_index - 1].clone();
        if let DynLogPayload::Command(message) = &entry.payload {
            committed_commands.push(CommittedCommand {
                message: message.clone(),
                term: entry.term,
                index: entry.index,
            });
        }
        committed_entries.push(entry);
    }

    let new_view = LeaderView {
        term: state.term,
        leader: if state.role == RaftState::Leader {
            Some(me)
        } else {
            state.known_leader.clone()
        },
    };
    let view_transition = (new_view != old_view).then_some(new_view);
    DynRaftStepOutput {
        outbound,
        committed_entries,
        committed_commands,
        redirected,
        reconfiguration_results,
        view_transition,
    }
}

/// Public wrapper configuration. The initial voters are the first
/// `initial_voter_count` members in canonical `ClusterIds` order.
#[derive(Clone, Copy, Debug)]
pub struct DynRaftConfig {
    pub initial_voter_count: usize,
}

pub struct DynRaftOutputs<'a, T, C> {
    pub committed: Stream<
        CommittedCommand<T>,
        Atomic<Cluster<'a, C, EventualConsistency>>,
        Unbounded,
        TotalOrder,
    >,
    pub redirected:
        Stream<(T, Option<MemberId<C>>), Cluster<'a, C, NoConsistency>, Unbounded, TotalOrder>,
    pub reconfiguration_results:
        Stream<ReconfigurationResult<C>, Cluster<'a, C, NoConsistency>, Unbounded, TotalOrder>,
    pub leader_views: Stream<LeaderView<C>, Cluster<'a, C>>,
}

/// Hosts [`dyn_raft_step`] in one Hydro tick per member. Physical cluster membership is
/// read only as a fixed address universe; logical configurations live solely in the log.
pub fn dyn_raft_server<'a, T, C, O, RO, Net>(
    cluster: &Cluster<'a, C>,
    commands: Stream<T, Cluster<'a, C>, Unbounded, O>,
    reconfigurations: Stream<ReconfigurationRequest<C>, Cluster<'a, C>, Unbounded, RO>,
    election_timer_interrupts: Stream<(), Cluster<'a, C>>,
    heartbeat_timer_interrupts: Stream<(), Cluster<'a, C>>,
    config: DynRaftConfig,
    net: Net,
    nondet_raft: NonDet,
) -> DynRaftOutputs<'a, T, C>
where
    T: Clone + Serialize + DeserializeOwned + 'a,
    C: 'a,
    O: Ordering,
    RO: Ordering,
    Net: NetworkFor<DynRaftRpc<T, C>>,
    NoOrder: MinOrder<Net::OrderingGuarantee, Min = NoOrder>,
{
    let commands = commands.assume_ordering::<TotalOrder>(nondet!(
        /** Concurrent commands have an arbitrary but fixed log order. */
        nondet_raft
    ));
    let reconfigurations = reconfigurations.assume_ordering::<TotalOrder>(nondet!(
        /** Concurrent administrative requests have an arbitrary but fixed order. */
        nondet_raft
    ));
    #[expect(clippy::type_complexity, reason = "protocol feedback channel")]
    let (traffic_handle, traffic): (
        ForwardHandle<
            'a,
            Stream<(MemberId<C>, DynRaftRpc<T, C>), Cluster<'a, C>, Unbounded, NoOrder>,
        >,
        Stream<(MemberId<C>, DynRaftRpc<T, C>), Cluster<'a, C>, Unbounded, NoOrder>,
    ) = cluster.forward_ref();

    let LocationId::Cluster(cluster_key) = Location::id(cluster) else {
        unreachable!("dyn_raft_server runs on a cluster")
    };
    let cluster_members = ClusterIds {
        key: cluster_key,
        _phantom: PhantomData,
    };
    let cluster_members_for_state = cluster_members.clone();
    let initial_voter_count = config.initial_voter_count;

    #[expect(clippy::type_complexity, reason = "explicit protocol outputs")]
    let (outbound, committed, redirected, results, views): (
        Stream<(MemberId<C>, DynRaftRpc<T, C>), Cluster<'a, C>>,
        Stream<CommittedCommand<T>, Cluster<'a, C>>,
        Stream<(T, Option<MemberId<C>>), Cluster<'a, C>>,
        Stream<ReconfigurationResult<C>, Cluster<'a, C>>,
        Stream<LeaderView<C>, Cluster<'a, C>>,
    ) = sliced! {
        let command_batch = use::batch(commands, nondet!(/** batching changes latency only */ nondet_raft));
        let reconfiguration_batch = use::batch(reconfigurations, nondet!(/** batching changes latency only */ nondet_raft));
        let election_batch = use::batch(election_timer_interrupts, nondet!(/** election timing selects leaders */ nondet_raft));
        let heartbeat_batch = use::batch(heartbeat_timer_interrupts, nondet!(/** heartbeat timing changes latency */ nondet_raft));
        let traffic_batch = use::batch(traffic, nondet!(/** messages are canonicalized in dyn_raft_step */ nondet_raft));
        let mut state = use::state(|l| l.singleton(q!(DynRaftState::new(Configuration::new(cluster_members_for_state
                .iter()
                .take(initial_voter_count)
                .map(|id| MemberId::from_tagless(id.clone()))
                .collect()).expect("initial voter count must be non-zero and at most the physical cluster size")))));
        let tick = command_batch.location().clone();
        let commands = command_batch.fold(q!(|| Vec::new()), q!(|out, value| out.push(value)));
        let reconfigurations = reconfiguration_batch.fold(q!(|| Vec::new()), q!(|out, value| out.push(value)));
        let election = election_batch.count().map(q!(|count| count > 0));
        let heartbeat = heartbeat_batch.count().map(q!(|count| count > 0));
        let messages = traffic_batch.fold(
            q!(|| Vec::new()),
            q!(|out, value| out.push(value), commutative = manual_proof!(/** step canonicalizes the multiset */)),
        );
        let physical_members = tick.singleton(q!(cluster_members
            .iter()
            .map(|id| MemberId::from_tagless(id.clone()))
            .collect::<Vec<_>>()));

        let state_ref = state.by_mut();
        let commands_ref = commands.by_ref();
        let reconfigurations_ref = reconfigurations.by_ref();
        let election_ref = election.by_ref();
        let heartbeat_ref = heartbeat.by_ref();
        let messages_ref = messages.by_ref();
        let physical_members_ref = physical_members.by_ref();
        let committed: Stream<CommittedCommand<T>, _, Bounded> = tick.source_iter(q!(Vec::new()));
        let committed_ref = committed.by_mut();
        let redirected: Stream<(T, Option<MemberId<C>>), _, Bounded> = tick.source_iter(q!(Vec::new()));
        let redirected_ref = redirected.by_mut();
        let results: Stream<ReconfigurationResult<C>, _, Bounded> = tick.source_iter(q!(Vec::new()));
        let results_ref = results.by_mut();
        let views: Stream<LeaderView<C>, _, Bounded> = tick.source_iter(q!(Vec::new()));
        let views_ref = views.by_mut();
        let outbound = tick.singleton(q!(() )).into_stream().flat_map_ordered(q!(move |_| {
            let output = crate::cluster::dyn_raft::dyn_raft_step(
                &mut *state_ref,
                DynRaftStepInput {
                    me: CLUSTER_SELF_ID.clone(),
                    physical_members: physical_members_ref.clone(),
                    election_timer_fired: *election_ref,
                    heartbeat_timer_fired: *heartbeat_ref,
                    commands: commands_ref.clone(),
                    reconfigurations: reconfigurations_ref.clone(),
                    messages: messages_ref.clone(),
                },
            );
            for value in output.committed_commands { committed_ref.push(value); }
            for value in output.redirected { redirected_ref.push(value); }
            for value in output.reconfiguration_results { results_ref.push(value); }
            if let Some(value) = output.view_transition { views_ref.push(value); }
            output.outbound
        }));
        (outbound, committed, redirected, results, views)
    };

    traffic_handle.complete(outbound.into_keyed().demux(cluster, net).entries());
    DynRaftOutputs {
        committed: committed
            .assert_has_consistency_of::<Cluster<'a, C, EventualConsistency>>(manual_proof!(
                /** patched Raft configuration rules preserve one committed log */
            ))
            .atomic(),
        redirected,
        reconfiguration_results: results,
        leader_views: views,
    }
}

#[expect(clippy::type_complexity, reason = "public guarantees are explicit")]
pub fn dyn_raft<'a, T, C, Con, O, RO, Net>(
    commands: Stream<T, Cluster<'a, C, Con>, Unbounded, O>,
    reconfigurations: Stream<ReconfigurationRequest<C>, Cluster<'a, C>, Unbounded, RO>,
    election_timer_interrupts: Stream<(), Cluster<'a, C>>,
    heartbeat_timer_interrupts: Stream<(), Cluster<'a, C>>,
    config: DynRaftConfig,
    net: impl Fn() -> Net,
    nondet_order: NonDet,
) -> DynRaftOutputs<'a, T, C>
where
    T: Clone + Serialize + DeserializeOwned + 'a,
    C: 'a,
    Con: Consistency,
    O: Ordering,
    RO: Ordering,
    Net: NetworkFor<DynRaftRpc<T, C>>,
    NoOrder: MinOrder<Net::OrderingGuarantee, Min = NoOrder>,
{
    let cluster = commands.location().drop_consistency();
    dyn_raft_server(
        &cluster,
        commands.weaken_consistency(),
        reconfigurations,
        election_timer_interrupts,
        heartbeat_timer_interrupts,
        config,
        net(),
        nondet!(/** elections, batching, and replication timing are nondeterministic */ nondet_order),
    )
}

/// Simulation-only outputs for the *unpatched* dissertation server.
///
/// Unlike [`DynRaftOutputs`], this exposes every committed log **entry** (with its
/// physical index and payload), not just application commands. A runtime safety
/// oracle needs the physical index/payload of *all* committed entries — including
/// `Noop` and `Configuration` entries — to detect the 2015 counterexample, in which
/// two members commit different payloads at the same physical log index.
#[doc(hidden)]
pub struct DynRaftUnpatchedOutputs<'a, T, C> {
    /// Every committed entry, tagged per member by `sim_cluster_output`.
    pub committed_entries: Stream<DynLogEntry<T, C>, Cluster<'a, C>, Unbounded, TotalOrder>,
    pub reconfiguration_results:
        Stream<ReconfigurationResult<C>, Cluster<'a, C, NoConsistency>, Unbounded, TotalOrder>,
    pub leader_views: Stream<LeaderView<C>, Cluster<'a, C>>,
}

/// Hosts the **unpatched** single-server reconfiguration rule
/// ([`dyn_raft_step_unpatched_for_simulation`]) so that Hydro's simulator can *discover*
/// Ongaro's 2015 membership-change counterexample on its own — choosing the batching,
/// message, election, and (optionally) crash schedules — rather than replaying a
/// hand-scripted execution.
///
/// This is wired identically to the production [`dyn_raft_server`], with two deliberate
/// differences:
///
/// 1. It routes through [`dyn_raft_step_unpatched_for_simulation`], which omits the 2015
///    current-term commit barrier. The production path in [`dyn_raft_server`] is
///    unchanged and always enforces the barrier via [`dyn_raft_step`].
/// 2. It exposes committed *entries* (physical index + payload) so a test can install an
///    explicit safety oracle. Because this path is intentionally unsafe, it does not
///    attach any `assert_has_consistency_of` guarantee.
///
/// Physical Hydro cluster membership stays static; logical Raft configurations remain
/// subsets of that fixed universe, exactly as in the production server.
#[doc(hidden)]
pub fn dyn_raft_server_unpatched_for_simulation<'a, T, C, O, RO, Net>(
    cluster: &Cluster<'a, C>,
    commands: Stream<T, Cluster<'a, C>, Unbounded, O>,
    reconfigurations: Stream<ReconfigurationRequest<C>, Cluster<'a, C>, Unbounded, RO>,
    election_timer_interrupts: Stream<(), Cluster<'a, C>>,
    heartbeat_timer_interrupts: Stream<(), Cluster<'a, C>>,
    config: DynRaftConfig,
    net: Net,
    nondet_raft: NonDet,
) -> DynRaftUnpatchedOutputs<'a, T, C>
where
    T: Clone + Serialize + DeserializeOwned + 'a,
    C: 'a,
    O: Ordering,
    RO: Ordering,
    Net: NetworkFor<DynRaftRpc<T, C>>,
    NoOrder: MinOrder<Net::OrderingGuarantee, Min = NoOrder>,
{
    let commands = commands.assume_ordering::<TotalOrder>(nondet!(
        /** Concurrent commands have an arbitrary but fixed log order. */
        nondet_raft
    ));
    let reconfigurations = reconfigurations.assume_ordering::<TotalOrder>(nondet!(
        /** Concurrent administrative requests have an arbitrary but fixed order. */
        nondet_raft
    ));
    #[expect(clippy::type_complexity, reason = "protocol feedback channel")]
    let (traffic_handle, traffic): (
        ForwardHandle<
            'a,
            Stream<(MemberId<C>, DynRaftRpc<T, C>), Cluster<'a, C>, Unbounded, NoOrder>,
        >,
        Stream<(MemberId<C>, DynRaftRpc<T, C>), Cluster<'a, C>, Unbounded, NoOrder>,
    ) = cluster.forward_ref();

    let LocationId::Cluster(cluster_key) = Location::id(cluster) else {
        unreachable!("dyn_raft_server_unpatched_for_simulation runs on a cluster")
    };
    let cluster_members = ClusterIds {
        key: cluster_key,
        _phantom: PhantomData,
    };
    let cluster_members_for_state = cluster_members.clone();
    let initial_voter_count = config.initial_voter_count;

    #[expect(clippy::type_complexity, reason = "explicit protocol outputs")]
    let (outbound, committed_entries, results, views): (
        Stream<(MemberId<C>, DynRaftRpc<T, C>), Cluster<'a, C>>,
        Stream<DynLogEntry<T, C>, Cluster<'a, C>>,
        Stream<ReconfigurationResult<C>, Cluster<'a, C>>,
        Stream<LeaderView<C>, Cluster<'a, C>>,
    ) = sliced! {
        let command_batch = use::batch(commands, nondet!(/** batching changes latency only */ nondet_raft));
        let reconfiguration_batch = use::batch(reconfigurations, nondet!(/** batching changes latency only */ nondet_raft));
        let election_batch = use::batch(election_timer_interrupts, nondet!(/** election timing selects leaders */ nondet_raft));
        let heartbeat_batch = use::batch(heartbeat_timer_interrupts, nondet!(/** heartbeat timing changes latency */ nondet_raft));
        let traffic_batch = use::batch(traffic, nondet!(/** messages are canonicalized in the step function */ nondet_raft));
        let mut state = use::state(|l| l.singleton(q!(DynRaftState::new(Configuration::new(cluster_members_for_state
                .iter()
                .take(initial_voter_count)
                .map(|id| MemberId::from_tagless(id.clone()))
                .collect()).expect("initial voter count must be non-zero and at most the physical cluster size")))));
        let tick = command_batch.location().clone();
        let commands = command_batch.fold(q!(|| Vec::new()), q!(|out, value| out.push(value)));
        let reconfigurations = reconfiguration_batch.fold(q!(|| Vec::new()), q!(|out, value| out.push(value)));
        let election = election_batch.count().map(q!(|count| count > 0));
        let heartbeat = heartbeat_batch.count().map(q!(|count| count > 0));
        let messages = traffic_batch.fold(
            q!(|| Vec::new()),
            q!(|out, value| out.push(value), commutative = manual_proof!(/** step canonicalizes the multiset */)),
        );
        let physical_members = tick.singleton(q!(cluster_members
            .iter()
            .map(|id| MemberId::from_tagless(id.clone()))
            .collect::<Vec<_>>()));

        let state_ref = state.by_mut();
        let commands_ref = commands.by_ref();
        let reconfigurations_ref = reconfigurations.by_ref();
        let election_ref = election.by_ref();
        let heartbeat_ref = heartbeat.by_ref();
        let messages_ref = messages.by_ref();
        let physical_members_ref = physical_members.by_ref();
        let committed: Stream<DynLogEntry<T, C>, _, Bounded> = tick.source_iter(q!(Vec::new()));
        let committed_ref = committed.by_mut();
        let results: Stream<ReconfigurationResult<C>, _, Bounded> = tick.source_iter(q!(Vec::new()));
        let results_ref = results.by_mut();
        let views: Stream<LeaderView<C>, _, Bounded> = tick.source_iter(q!(Vec::new()));
        let views_ref = views.by_mut();
        let outbound = tick.singleton(q!(() )).into_stream().flat_map_ordered(q!(move |_| {
            let output = crate::cluster::dyn_raft::dyn_raft_step_unpatched_for_simulation(
                &mut *state_ref,
                DynRaftStepInput {
                    me: CLUSTER_SELF_ID.clone(),
                    physical_members: physical_members_ref.clone(),
                    election_timer_fired: *election_ref,
                    heartbeat_timer_fired: *heartbeat_ref,
                    commands: commands_ref.clone(),
                    reconfigurations: reconfigurations_ref.clone(),
                    messages: messages_ref.clone(),
                },
            );
            for value in output.committed_entries { committed_ref.push(value); }
            for value in output.reconfiguration_results { results_ref.push(value); }
            if let Some(value) = output.view_transition { views_ref.push(value); }
            output.outbound
        }));
        (outbound, committed, results, views)
    };

    traffic_handle.complete(outbound.into_keyed().demux(cluster, net).entries());
    DynRaftUnpatchedOutputs {
        committed_entries,
        reconfiguration_results: results,
        leader_views: views,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    pub struct Replica;

    fn id(raw: u32) -> MemberId<Replica> {
        MemberId::from_raw_id(raw)
    }

    struct StepCluster {
        states: Vec<DynRaftState<String, Replica>>,
        inboxes: Vec<Vec<(MemberId<Replica>, DynRaftRpc<String, Replica>)>>,
        committed: Vec<Vec<DynLogEntry<String, Replica>>>,
        results: Vec<Vec<ReconfigurationResult<Replica>>>,
        policy: StepPolicy,
    }

    impl StepCluster {
        fn new(size: usize, initial: &[u32]) -> Self {
            let configuration =
                Configuration::new(initial.iter().copied().map(id).collect()).unwrap();
            Self {
                states: (0..size)
                    .map(|_| DynRaftState::new(configuration.clone()))
                    .collect(),
                inboxes: vec![Vec::new(); size],
                committed: vec![Vec::new(); size],
                results: vec![Vec::new(); size],
                policy: SAFE_POLICY,
            }
        }

        fn step(
            &mut self,
            member: usize,
            election: bool,
            heartbeat: bool,
            commands: &[&str],
            reconfigurations: Vec<ReconfigurationRequest<Replica>>,
        ) {
            let messages = std::mem::take(&mut self.inboxes[member]);
            let physical_size = self.states.len();
            let output = dyn_raft_step_with_policy(
                &mut self.states[member],
                DynRaftStepInput {
                    me: id(member as u32),
                    physical_members: (0..physical_size).map(|raw| id(raw as u32)).collect(),
                    election_timer_fired: election,
                    heartbeat_timer_fired: heartbeat,
                    commands: commands.iter().map(|value| (*value).to_owned()).collect(),
                    reconfigurations,
                    messages,
                },
                self.policy,
            );
            for (target, rpc) in output.outbound {
                self.inboxes[target.get_raw_id() as usize].push((id(member as u32), rpc));
            }
            self.committed[member].extend(output.committed_entries);
            self.results[member].extend(output.reconfiguration_results);
        }

        fn quiet(&mut self) {
            loop {
                let mut progress = false;
                for member in 0..self.states.len() {
                    if !self.inboxes[member].is_empty() {
                        progress = true;
                        self.step(member, false, false, &[], vec![]);
                    }
                }
                if !progress {
                    break;
                }
            }
        }

        fn elect(&mut self, member: usize) {
            self.step(member, true, false, &[], vec![]);
            self.quiet();
            assert_eq!(self.states[member].role, RaftState::Leader);
        }

        fn heartbeat(&mut self, leader: usize) {
            self.step(leader, false, true, &[], vec![]);
            self.quiet();
        }

        fn check_commit_safety(&self) {
            let mut by_index: HashMap<usize, &DynLogPayload<String, Replica>> = HashMap::new();
            for history in &self.committed {
                for entry in history {
                    if let Some(previous) = by_index.insert(entry.index, &entry.payload) {
                        assert_eq!(
                            previous, &entry.payload,
                            "different commits at index {}",
                            entry.index
                        );
                    }
                }
            }
        }
    }

    fn add(request_id: u64, member: u32) -> ReconfigurationRequest<Replica> {
        ReconfigurationRequest {
            request_id,
            change: ConfigurationChange::Add(id(member)),
        }
    }
    fn remove(request_id: u64, member: u32) -> ReconfigurationRequest<Replica> {
        ReconfigurationRequest {
            request_id,
            change: ConfigurationChange::Remove(id(member)),
        }
    }

    #[test]
    fn election_noop_then_command_commits() {
        let mut cluster = StepCluster::new(3, &[0, 1, 2]);
        cluster.elect(0);
        assert!(matches!(
            cluster.states[0].log[0].payload,
            DynLogPayload::Noop
        ));
        cluster.heartbeat(0);
        cluster.step(0, false, false, &["x"], vec![]);
        cluster.heartbeat(0);
        cluster.heartbeat(0);
        cluster.check_commit_safety();
        assert!(
            cluster
                .committed
                .iter()
                .all(|history| history.iter().any(|entry| {
                    matches!(&entry.payload, DynLogPayload::Command(value) if value == "x")
                }))
        );
    }

    #[test]
    fn add_is_caught_up_as_nonvoter_before_configuration_commits() {
        let mut cluster = StepCluster::new(4, &[0, 1, 2]);
        cluster.elect(0);
        cluster.heartbeat(0); // replicate+commit leader no-op
        cluster.step(0, false, false, &[], vec![add(7, 3)]);
        assert!(!cluster.states[0].effective_configuration().contains(&id(3)));
        cluster.heartbeat(0); // learner receives existing log
        cluster.step(0, false, false, &[], vec![]); // process learner ack and append config
        assert!(cluster.states[0].effective_configuration().contains(&id(3)));
        cluster.heartbeat(0);
        cluster.step(0, false, false, &[], vec![]); // process acks/commit
        assert!(cluster.results[0].iter().any(|result| matches!(
            result,
            ReconfigurationResult::Completed { request_id: 7, .. }
        )));
        cluster.check_commit_safety();
    }

    #[test]
    fn removal_is_leader_managed_and_committed() {
        let mut cluster = StepCluster::new(4, &[0, 1, 2, 3]);
        cluster.elect(0);
        cluster.heartbeat(0);
        cluster.step(0, false, false, &[], vec![remove(9, 3)]);
        assert!(!cluster.states[0].effective_configuration().contains(&id(3)));
        cluster.heartbeat(0);
        cluster.step(0, false, false, &[], vec![]);
        assert!(cluster.results[0].iter().any(|result| matches!(
            result,
            ReconfigurationResult::Completed { request_id: 9, .. }
        )));
        cluster.check_commit_safety();
    }

    #[test]
    fn patched_leader_waits_for_current_term_commit_before_competing_change() {
        let mut cluster = StepCluster::new(4, &[0, 1, 2, 3]);
        // Install an uncommitted term-1 removal on S0 only.
        cluster.states[0].term = 1;
        cluster.states[0].role = RaftState::Leader;
        cluster.states[0].log.push(DynLogEntry {
            payload: DynLogPayload::Configuration(
                Configuration::new(vec![id(0), id(1), id(2)]).unwrap(),
            ),
            term: 1,
            index: 1,
        });
        // S1 wins term 2 without seeing it. Its no-op is not committed yet.
        cluster.states[1].term = 2;
        cluster.states[1].role = RaftState::Leader;
        cluster.states[1].log.push(DynLogEntry {
            payload: DynLogPayload::Noop,
            term: 2,
            index: 1,
        });
        cluster.step(1, false, false, &[], vec![remove(2, 2)]);
        assert!(cluster.states[1].pending.is_some());
        assert_eq!(
            cluster.states[1].log.len(),
            1,
            "configuration must remain queued"
        );
        cluster.heartbeat(1);
        cluster.step(1, false, false, &[], vec![]);
        assert!(matches!(
            cluster.states[1].log.last().unwrap().payload,
            DynLogPayload::Configuration(_)
        ));
    }

    #[test]
    #[should_panic(expected = "different commits at index")]
    fn unsafe_policy_reproduces_ongaro_two_removes_counterexample() {
        let mut cluster = StepCluster::new(4, &[0, 1, 2, 3]);
        cluster.policy = StepPolicy {
            require_current_term_commit: false,
        };

        // Term 1: S0 appends D={S0,S1,S2}, but D reaches nobody else.
        cluster.elect(0);
        cluster.step(0, false, false, &[], vec![remove(1, 3)]);
        assert_eq!(cluster.states[0].latest_configuration_index(), 2);

        // Term 2: S1 obtains the old configuration's votes from S1,S2,S3.
        cluster.step(1, true, false, &[], vec![]);
        let request_vote_for_s0 = std::mem::take(&mut cluster.inboxes[0]);
        cluster.step(2, false, false, &[], vec![]);
        cluster.step(3, false, false, &[], vec![]);
        cluster.step(1, false, false, &[], vec![]);
        assert_eq!(cluster.states[1].role, RaftState::Leader);

        // Before its term-2 no-op commits, the buggy policy appends competing
        // E={S0,S1,S3}. Replicate only to S3: under E, S1+S3 is a majority.
        cluster.step(1, false, false, &[], vec![remove(2, 2)]);
        cluster.step(1, false, true, &[], vec![]);
        cluster.inboxes[0].clear();
        cluster.inboxes[2].clear();
        cluster.step(3, false, false, &[], vec![]);
        cluster.step(1, false, false, &[], vec![]);
        assert_eq!(cluster.states[1].commit_index, 2);
        cluster.step(1, false, true, &[], vec![]);
        cluster.inboxes[0].clear();
        cluster.inboxes[2].clear();
        cluster.step(3, false, false, &[], vec![]);
        assert_eq!(cluster.states[3].commit_index, 2);

        // S0 observes term 2 but never its log, then wins term 3 with S2 under D.
        cluster.inboxes[0] = request_vote_for_s0;
        cluster.step(0, false, false, &[], vec![]);
        cluster.inboxes[1].clear();
        cluster.step(0, true, false, &[], vec![]);
        cluster.inboxes[1].clear();
        cluster.step(2, false, false, &[], vec![]);
        cluster.step(0, false, false, &[], vec![]);
        assert_eq!(cluster.states[0].role, RaftState::Leader);
        assert_eq!(cluster.states[0].term, 3);

        // Back nextIndex down against S1 until S0 sends its conflicting suffix. The
        // dissertation algorithm silently truncates S1's already-committed term-2
        // entry; the divergence then surfaces on the committed outputs.
        for _ in 0..4 {
            cluster.step(0, false, true, &[], vec![]);
            cluster.inboxes[2].clear();
            cluster.step(1, false, false, &[], vec![]);
            cluster.step(0, false, false, &[], vec![]);
        }
        cluster.check_commit_safety();
        panic!("counterexample failed to produce divergent committed logs");
    }

    #[test]
    fn unsafe_dissertation_policy_exposes_competing_configuration_shape() {
        let mut cluster = StepCluster::new(4, &[0, 1, 2, 3]);
        cluster.policy = StepPolicy {
            require_current_term_commit: false,
        };
        cluster.states[1].term = 2;
        cluster.states[1].role = RaftState::Leader;
        cluster.states[1].log.push(DynLogEntry {
            payload: DynLogPayload::Noop,
            term: 2,
            index: 1,
        });
        cluster.step(1, false, false, &[], vec![remove(2, 2)]);
        assert!(matches!(
            cluster.states[1].log.last().unwrap().payload,
            DynLogPayload::Configuration(_)
        ));
        assert_eq!(cluster.states[1].commit_index, 0);
    }

    /// Membership churn under concurrent administration, with a learner joining: four
    /// physical members; three start as voters, one (member 3) starts outside the voting
    /// configuration as a non-voting learner. Two operators, contacting two different
    /// nodes, concurrently ask to add the learner and remove a voter while election and
    /// heartbeat timers fire. Whatever schedule occurs, the committed log must never
    /// fork: no two members may commit different payloads at the same log index.
    ///
    /// The test fixes only the workload:
    ///
    /// 1. Elect a leader (one election tick to member 0, one quiescence).
    /// 2. Two concurrent operators: one join (`Add(3)`) sent once to member 0, one leave
    ///    (`Remove(1)`) sent once to member 1.
    /// 3. A modest, symmetric batch of election and heartbeat ticks, sent to every
    ///    member with no `.await` in between — the same shape `raft.rs`'s own
    ///    `fully_concurrent_run_never_forks_the_committed_log` test uses (2 election
    ///    waves, 6 heartbeat waves).
    ///
    /// Everything else — who leads which term, whether the join or the leave (or
    /// neither, or both) lands, and every message/batch ordering decision — is the
    /// simulator's.
    ///
    /// **Status (post-run note):** no violation found so far (tens of millions of
    /// coverage-guided executions across 8 parallel seeds); reported as inconclusive,
    /// not as safety — see `docs/dyn_raft_simulator_discovery.md`. `#[ignore]`d until a
    /// conclusive result exists.
    #[test]
    #[ignore = "exploratory: run under `cargo sim` (see doc comment); result not yet conclusive"]
    fn simulator_explores_concurrent_join_and_leave() {
        use hydro_lang::location::MemberId;

        const N: usize = 4;
        const INITIAL_VOTERS: usize = 3;
        const ELECTION_WAVES: usize = 2;
        const HEARTBEAT_PUMPS: usize = 6;

        let mut flow = FlowBuilder::new();
        let cluster = flow.cluster::<Replica>();

        let (election_send, election_timer_interrupts) = cluster.sim_input();
        let (heartbeat_send, heartbeat_timer_interrupts) = cluster.sim_input();
        let (_command_send, commands) = cluster.sim_input::<String, TotalOrder, _>();
        let (reconfig_send, reconfigurations) =
            cluster.sim_input::<ReconfigurationRequest<Replica>, TotalOrder, _>();

        let outputs = dyn_raft_server_unpatched_for_simulation(
            &cluster,
            commands,
            reconfigurations,
            election_timer_interrupts,
            heartbeat_timer_interrupts,
            DynRaftConfig {
                initial_voter_count: INITIAL_VOTERS,
            },
            TCP.fail_stop().bincode(),
            nondet!(
                /** elections, batching, and replication timing are nondeterministic; the
                committed log must not fork regardless — and, on this deliberately
                unpatched path, the simulator is expected to explore whether it can. */
            ),
        );

        let committed_recv = outputs.committed_entries.sim_cluster_output();
        let _results_recv = outputs.reconfiguration_results.sim_cluster_output();
        let _views_recv = outputs.leader_views.sim_cluster_output();

        flow.sim()
            .with_cluster_size(&cluster, N)
            .fuzz(async || {
                // Step 1: elect a leader.
                election_send.send(0, ());
                hydro_lang::sim::quiesce().await;

                // Step 2: two operators concurrently submit one join and one leave, each
                // contacting a different node.
                reconfig_send.send(
                    0,
                    ReconfigurationRequest {
                        request_id: 1,
                        change: ConfigurationChange::Add(MemberId::from_raw_id(3)),
                    },
                );
                reconfig_send.send(
                    1,
                    ReconfigurationRequest {
                        request_id: 2,
                        change: ConfigurationChange::Remove(MemberId::from_raw_id(1)),
                    },
                );

                // Step 3: a modest, symmetric batch of further ticks (same shape as
                // raft.rs's fully_concurrent_run_never_forks_the_committed_log), with no
                // `.await` in between — the simulator owns the entire schedule from here.
                for _ in 0..ELECTION_WAVES {
                    for member in 0..N as u32 {
                        election_send.send(member, ());
                    }
                }
                for _ in 0..HEARTBEAT_PUMPS {
                    for member in 0..N as u32 {
                        heartbeat_send.send(member, ());
                    }
                }

                // Single final quiescence, then the same fork oracle as the other tests.
                hydro_lang::sim::quiesce().await;

                let mut committed_at: HashMap<usize, DynLogPayload<String, Replica>> =
                    HashMap::new();
                for member in 0..N as u32 {
                    for entry in committed_recv.collect::<Vec<_>>(member).await {
                        if let Some(previous) = committed_at.get(&entry.index) {
                            assert_eq!(
                                previous, &entry.payload,
                                "committed log forked: physical index {} committed two \
                                 different payloads",
                                entry.index
                        );
                    } else {
                        committed_at.insert(entry.index, entry.payload);
                    }
                }
            }
        });
    }

    /// Membership churn under concurrent administration: two operators, contacting two
    /// different nodes, concurrently ask to remove two different members while election
    /// and heartbeat timers fire. Whatever schedule occurs — whichever requests are
    /// accepted, rejected, or retried, and whoever leads — the committed log must never
    /// fork: no two members may commit different payloads at the same log index.
    ///
    /// The test fixes only the workload (steps 1–3 below); the simulator owns every
    /// batching, delivery, and leadership decision.
    ///
    /// **Result (post-discovery note):** coverage-guided fuzzing (`cargo sim`) found
    /// schedules violating this property; the checked-in reproducer under `sim-failures/`
    /// replays one deterministically, hence `#[should_panic]`. Analysis in
    /// `docs/dyn_raft_simulator_discovery.md`.
    #[test]
    #[should_panic(expected = "committed log forked")]
    fn simulator_explores_concurrent_double_leave() {
        use hydro_lang::location::MemberId;

        const N: usize = 4;
        const ELECTION_WAVES: usize = 2;
        const HEARTBEAT_PUMPS: usize = 6;

        let mut flow = FlowBuilder::new();
        let cluster = flow.cluster::<Replica>();

        let (election_send, election_timer_interrupts) = cluster.sim_input();
        let (heartbeat_send, heartbeat_timer_interrupts) = cluster.sim_input();
        let (_command_send, commands) = cluster.sim_input::<String, TotalOrder, _>();
        let (reconfig_send, reconfigurations) =
            cluster.sim_input::<ReconfigurationRequest<Replica>, TotalOrder, _>();

        let outputs = dyn_raft_server_unpatched_for_simulation(
            &cluster,
            commands,
            reconfigurations,
            election_timer_interrupts,
            heartbeat_timer_interrupts,
            DynRaftConfig {
                initial_voter_count: N,
            },
            TCP.fail_stop().bincode(),
            nondet!(
                /** elections, batching, and replication timing are nondeterministic; the
                committed log must not fork regardless — and, on this deliberately
                unpatched path, the simulator is expected to explore whether it can. */
            ),
        );

        let committed_recv = outputs.committed_entries.sim_cluster_output();
        let _results_recv = outputs.reconfiguration_results.sim_cluster_output();
        let _views_recv = outputs.leader_views.sim_cluster_output();

        flow.sim()
            .with_cluster_size(&cluster, N)
            .fuzz(async || {
                // Step 1: elect a leader.
                election_send.send(0, ());
                hydro_lang::sim::quiesce().await;

                // Step 2: two operators concurrently submit removals of two different
                // members, each contacting a different node.
                reconfig_send.send(
                    0,
                    ReconfigurationRequest {
                        request_id: 1,
                        change: ConfigurationChange::Remove(MemberId::from_raw_id(1)),
                    },
                );
                reconfig_send.send(
                    1,
                    ReconfigurationRequest {
                        request_id: 2,
                        change: ConfigurationChange::Remove(MemberId::from_raw_id(2)),
                    },
                );

                // Step 3: a modest, symmetric batch of further ticks (same shape as
                // raft.rs's fully_concurrent_run_never_forks_the_committed_log), with no
                // `.await` in between — the simulator owns the entire schedule from here.
                for _ in 0..ELECTION_WAVES {
                    for member in 0..N as u32 {
                        election_send.send(member, ());
                    }
                }
                for _ in 0..HEARTBEAT_PUMPS {
                    for member in 0..N as u32 {
                        heartbeat_send.send(member, ());
                    }
                }

                // Single final quiescence, then the same fork oracle as the other tests.
                hydro_lang::sim::quiesce().await;

                let mut committed_at: HashMap<usize, DynLogPayload<String, Replica>> =
                    HashMap::new();
                for member in 0..N as u32 {
                    for entry in committed_recv.collect::<Vec<_>>(member).await {
                        if let Some(previous) = committed_at.get(&entry.index) {
                            assert_eq!(
                                previous, &entry.payload,
                                "committed log forked: physical index {} committed two \
                                 different payloads",
                                entry.index
                            );
                        } else {
                            committed_at.insert(entry.index, entry.payload);
                        }
                    }
                }
            });
    }
}
