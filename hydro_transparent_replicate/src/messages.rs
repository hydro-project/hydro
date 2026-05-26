//! Wire protocol message types for the transparent replication protocol.
//!
//! This module defines all messages exchanged between replicas and clients
//! in the primary/backup replication protocol. The protocol has two paths:
//!
//! - **Data path** (consensus-free): `Replicate`, `Ack`, `CommitNotification`
//! - **Control path** (Paxos-backed): `StateTransferRequest`, `StateTransferResponse`
//! - **Client path**: `ClientRequest`, `ClientResponse`
//!
//! All message types are generic over the command type `C` (or response type `R`)
//! to support any [`ReplicableService`](crate::r#trait::ReplicableService) implementation.

use hydro_lang::location::MemberId;
use serde::{Deserialize, Serialize};

/// Cluster marker type for transparent replication replicas.
///
/// Used as the type parameter for `Cluster<TransparentReplica>` in the Hydro
/// dataflow and for `MemberId<TransparentReplica>` in wire protocol messages.
#[derive(Serialize, Deserialize, Clone)]
pub struct TransparentReplica {}

/// Current view of the cluster membership.
///
/// A view defines which replicas are active participants in the protocol.
/// The `members` list is always sorted in ascending order and deduplicated.
/// By convention, `members[0]` is the primary for this view.
///
/// Views are sequenced by Paxos during view changes. A higher `view_num`
/// always supersedes a lower one.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct View {
    /// Monotonically increasing view number. Higher values supersede lower ones.
    pub view_num: u64,
    /// Sorted, deduplicated list of active replica IDs. `members[0]` is the primary.
    pub members: Vec<u32>,
}

impl Ord for View {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.view_num.cmp(&other.view_num)
    }
}

impl PartialOrd for View {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl View {
    /// Returns the primary replica's ID for this view.
    ///
    /// The primary is always `members[0]` by convention.
    ///
    /// # Panics
    /// Panics if `members` is empty (which violates the View invariant).
    pub fn primary(&self) -> u32 {
        self.members[0]
    }

    /// Returns `true` if the given member ID is part of this view.
    pub fn contains(&self, member_id: u32) -> bool {
        self.members.binary_search(&member_id).is_ok()
    }
}

/// Replicate message sent from the primary to all backups.
///
/// The primary assigns a sequence number to each mutating command and
/// broadcasts it to all view members. Backups validate the `view_num`,
/// store the command, and respond with an [`Ack`].
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Replicate<C> {
    /// The view in which this replication is occurring.
    pub view_num: u64,
    /// Sequence number assigned by the primary (contiguous within a view).
    pub seq: usize,
    /// The command being replicated.
    pub command: C,
    /// The primary's member ID (sender of this message).
    pub sender: MemberId<TransparentReplica>,
}

/// Acknowledgment message sent from a backup to the primary.
///
/// After a backup receives and validates a [`Replicate`] message, it sends
/// an `Ack` back to the primary. The primary commits the command once it
/// has received acks from all view members (full quorum).
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Ack {
    /// The view in which this ack is valid.
    pub view_num: u64,
    /// The sequence number being acknowledged.
    pub seq: usize,
    /// The backup's member ID (sender of this ack).
    pub sender: MemberId<TransparentReplica>,
}

/// Periodic commit notification broadcast from the primary to all replicas.
///
/// The primary periodically broadcasts its commit progress so that backups
/// can detect primary failure (if notifications stop arriving, the primary
/// may have crashed). Also carries pending ack state for failure detection.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct CommitNotification {
    /// The view in which this notification was generated.
    pub view_num: u64,
    /// Sequence numbers that have been committed since the last notification.
    pub committed_seqs: Vec<usize>,
    /// Pending (uncommitted) slots and which members have acked them.
    /// Used by the failure detector to track recently-alive members.
    pub pending_acks: Vec<(usize, Vec<u32>)>,
}

/// State transfer request sent by the new primary to surviving backups.
///
/// After a view change installs a new primary, it needs to recover the
/// committed state from a surviving backup before it can resume serving
/// requests. The new primary sends this request to get a snapshot.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct StateTransferRequest {
    /// The new view number (proves the requester is the legitimate new primary).
    pub view_num: u64,
    /// The new primary's member ID.
    pub requester: MemberId<TransparentReplica>,
}

/// State transfer response from a surviving backup to the new primary.
///
/// Contains a full snapshot of the service state plus any commands committed
/// after the snapshot was taken (the "suffix"). The new primary restores the
/// snapshot, applies the suffix, and resumes from `snapshot_seq + suffix.len() + 1`.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct StateTransferResponse<C> {
    /// Serialized service state (produced by `ReplicableService::snapshot()`).
    pub snapshot: Vec<u8>,
    /// Commands committed after the snapshot point, in sequence order.
    pub suffix: Vec<(usize, C)>,
    /// The sequence number at which the snapshot was taken.
    pub snapshot_seq: usize,
}

/// Client request wrapper for routing through the replica cluster.
///
/// Clients send commands to any replica. Non-primary replicas proxy the
/// request to the current primary. The `client_id` and `request_id` enable
/// response routing and deduplication.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ClientRequest<C> {
    /// Unique identifier for the client connection.
    pub client_id: u64,
    /// Per-client monotonically increasing request identifier.
    pub request_id: u64,
    /// The command to execute on the replicated service.
    pub command: C,
}

/// Client response wrapper returned to the originating client.
///
/// Carries the response from the replicated service back to the client,
/// tagged with the `request_id` so the client can match it to its request.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ClientResponse<R> {
    /// The request_id from the corresponding [`ClientRequest`].
    pub request_id: u64,
    /// The response produced by the service after applying the command.
    pub response: R,
}
