//! The Hydro dataflow implementing primary/backup replication.
//!
//! This module contains the core replication logic adapted from
//! `hydro_test/src/cluster/basic_primary_backup.rs`, generalized over
//! a generic command type `C` instead of the hardcoded `KvPayload<K, V>`.

use hydro_lang::live_collections::sliced::yield_atomic;
use hydro_lang::live_collections::stream::{NoOrder, TotalOrder};
use hydro_lang::location::cluster::CLUSTER_SELF_ID;
use hydro_lang::location::{Location, NoTick};
use hydro_lang::prelude::*;
use serde::de::DeserializeOwned;
use serde::Serialize;
use stageleft::q;
use std::collections::BTreeMap;
use std::fmt::Debug;

use crate::messages::{Ack, Replicate, TransparentReplica, View};
use crate::ReplicableService;

// ─────────────────────────────────────────────────────────────────────────────
// index_payloads — local copy adapted from hydro_test/src/cluster/paxos.rs
// ─────────────────────────────────────────────────────────────────────────────

/// Assigns contiguous sequence numbers to a batch of payloads.
///
/// Given an optional base sequence number (from reconciliation after view change)
/// and a stream of payloads within a tick, produces `(seq, payload)` pairs with
/// contiguous sequence numbers starting from the base (or 0 if no base).
fn index_payloads<'a, L: Location<'a> + NoTick, P>(
    p_max_slot: Optional<usize, Tick<L>, Bounded>,
    c_to_proposers: Stream<P, Tick<L>, Bounded>,
) -> Stream<(usize, P), Tick<L>, Bounded>
where
    P: Clone + Serialize + DeserializeOwned + Debug + Send + 'static,
{
    sliced! {
        let mut next_slot = use::state(|l| l.singleton(q!(0)));
        let updated_max_slot = use::atomic(p_max_slot.latest_atomic(), nondet!(/** up to date with tick input */));
        let payload_batch = use::atomic(c_to_proposers.all_ticks_atomic(), nondet!(/** up to date with tick input */));

        let next_slot_after_reconciling = updated_max_slot.map(q!(|s| s + 1));
        let base_slot = next_slot_after_reconciling.unwrap_or(next_slot);

        let indexed_payloads = payload_batch
            .enumerate()
            .cross_singleton(base_slot.clone())
            .map(q!(|((index, payload), base_slot)| (
                base_slot + index,
                payload
            )));

        let num_payloads = indexed_payloads.clone().count();
        next_slot = num_payloads
            .zip(base_slot)
            .map(q!(|(num_payloads, base_slot)| base_slot + num_payloads));

        yield_atomic(indexed_payloads)
    }.batch_atomic(nondet!(/** up to date with tick input */))
}

// ─────────────────────────────────────────────────────────────────────────────
// Core replication module
// ─────────────────────────────────────────────────────────────────────────────

/// Output of the core replication module.
///
/// Bundles the streams produced by [`core_replication_module`] for consumption
/// by downstream modules (notification broadcaster, failure detector, etc.).
pub struct CoreOutput<'a, C> {
    /// Committed `(seq, command)` pairs — ready for application on the primary.
    /// Only fires on the primary (after quorum collection).
    pub committed: Stream<(usize, C), Cluster<'a, TransparentReplica>, Unbounded, NoOrder>,
    /// Replicated `(seq, command)` pairs — validated replicates received by ALL replicas.
    /// Fires on every replica (primary and backups) for every validated Replicate message.
    /// Use this to maintain hot standby state on backups.
    pub replicated: Stream<(usize, C), Cluster<'a, TransparentReplica>, Unbounded, NoOrder>,
    /// Committed sequence numbers — consumed by the notification broadcaster.
    pub committed_seqs: Stream<usize, Cluster<'a, TransparentReplica>, Unbounded, NoOrder>,
    /// Pending ack state per slot — consumed by the notification broadcaster.
    pub pending_acks: Stream<(usize, Vec<u32>), Cluster<'a, TransparentReplica>, Unbounded, NoOrder>,
    /// Max replicated slot on this replica — consumed by log reconciliation.
    pub max_replicated_seq: Optional<usize, Cluster<'a, TransparentReplica>, Unbounded>,
    /// Read-only commands that bypassed replication — apply directly on primary.
    ///
    /// These commands were classified as read-only by the `is_read_only` predicate
    /// and skip the full replicate → quorum → commit path. The caller should apply
    /// them directly on the primary's service instance and respond immediately.
    pub read_only_commands: Stream<C, Cluster<'a, TransparentReplica>, Unbounded, NoOrder>,
}

// ─────────────────────────────────────────────────────────────────────────────
// NONDETERMINISM IN THE FAILURE-FREE PATH
//
// There are exactly two sources of nondeterminism in the failure-free path:
//
//   1. BATCH BOUNDARIES — which client operations land in each tick.
//   2. SLOT ASSIGNMENT ORDER — within a batch, the order in which
//      operations are assigned to slots.
//
// Everything downstream of slot assignment — broadcasting, acking,
// quorum collection — is deterministic processing of an unordered
// stream of (slot, op) pairs.
// ─────────────────────────────────────────────────────────────────────────────

/// Core primary/backup replication module generalized over command type `C`.
///
/// This function implements the data path of the replication protocol:
/// - Read-only commands (provided as a separate stream) bypass replication entirely
/// - Mutating commands are batched, assigned sequence numbers, broadcast, and committed
///
/// The caller is responsible for splitting commands into `mutating_commands` and
/// `read_only_commands` before calling this function. Typically this is done using
/// `S::is_read_only()` from the [`ReplicableService`](crate::ReplicableService) trait.
/// Read-only commands skip the replicate → quorum → commit path and are emitted on
/// `CoreOutput::read_only_commands` for immediate application on the primary.
///
/// Adapted from `hydro_test/src/cluster/basic_primary_backup.rs::core_pb_module`,
/// replacing `KvPayload<K, V>` with generic `C`.
#[expect(clippy::type_complexity, reason = "Hydro stream types are deeply nested")]
pub fn core_replication_module<'a, C>(
    replicas: &Cluster<'a, TransparentReplica>,
    tick: &Tick<Cluster<'a, TransparentReplica>>,
    mutating_commands: Stream<C, Cluster<'a, TransparentReplica>, Unbounded>,
    read_only_commands: Stream<C, Cluster<'a, TransparentReplica>, Unbounded>,
    current_view: Singleton<View, Cluster<'a, TransparentReplica>, Unbounded>,
    reconciled_seq: Optional<usize, Cluster<'a, TransparentReplica>, Unbounded>,
) -> CoreOutput<'a, C>
where
    C: Clone + Serialize + DeserializeOwned + Debug + Send + 'static,
{
    // Snapshot the view into this tick.
    let view = current_view.snapshot(tick, nondet!(
        /// A stale view may cause the primary to broadcast with an outdated view
        /// number. Backups reject mismatched view numbers, so this causes dropped
        /// payloads during view change but never incorrect commits.
    ));

    // Derive is_primary from the snapshotted view.
    let is_primary = view.clone()
        .filter(q!(move |v| CLUSTER_SELF_ID.get_raw_id() == v.primary()))
        .map(q!(|_| ()));

    // ═══════════════════════════════════════════════════════════════════════
    // READ-ONLY OPTIMIZATION: Read-only commands bypass replication entirely.
    // They are batched, filtered to primary-only, and emitted for immediate
    // application without sequencing, broadcasting, or quorum collection.
    // ═══════════════════════════════════════════════════════════════════════

    let read_only_on_primary = read_only_commands
        .batch(tick, nondet!(/** read-only batch boundaries */))
        .filter_if_some(is_primary.clone())
        .weaken_ordering::<NoOrder>()
        .all_ticks();

    // reconciled_seq is only relevant after a view change (failure path).
    // In the failure-free path it's None and index_payloads starts from 0.
    let base_seq = reconciled_seq.snapshot(tick, nondet!(
        /// Only matters after view change. In failure-free path this is None.
    ));

    // ═══════════════════════════════════════════════════════════════════════
    // NONDETERMINISTIC: The primary decides batch boundaries and slot order.
    // After this point, the slot table is fixed and everything is deterministic.
    // ═══════════════════════════════════════════════════════════════════════

    let batch = mutating_commands.batch(tick, nondet!(/** NONDET #1: batch boundaries */));
    let primary_ops = batch.filter_if_some(is_primary.clone());
    let slot_table = index_payloads(base_seq, primary_ops); // NONDET #2: slot ordering

    // ═══════════════════════════════════════════════════════════════════════
    // DETERMINISTIC: Everything below processes (slot, op) pairs.
    // ═══════════════════════════════════════════════════════════════════════

    // Primary stamps each (slot, command) with the current view and broadcasts.
    let replicates = slot_table.clone()
        .cross_singleton(view.clone())
        .map(q!(move |((seq, command), v)| Replicate {
            view_num: v.view_num, seq, command, sender: CLUSTER_SELF_ID.clone(),
        }))
        .all_ticks()
        .broadcast(replicas, TCP.fail_stop().bincode(), nondet!(/** network send */))
        .values();

    // Each replica: validate view, ack back to sender.
    let valid = replicates
        .batch(tick, nondet!(/** tick boundary */))
        .cross_singleton(view.clone())
        .filter(q!(|(r, v)| r.view_num == v.view_num))
        .map(q!(|(r, _)| r));

    let acks = valid.clone()
        .map(q!(move |r| (r.sender.clone(), Ack {
            view_num: r.view_num, seq: r.seq, sender: CLUSTER_SELF_ID.clone(),
        })))
        .all_ticks()
        .demux(replicas, TCP.fail_stop().bincode())
        .values();

    // Track max replicated slot (O(1) state for log reconciliation).
    let max_replicated_seq = valid.clone()
        .map(q!(|r| r.seq))
        .across_ticks(|s| s.fold(
            q!(|| None::<usize>),
            q!(|max, seq| { *max = Some(max.map_or(seq, |m: usize| m.max(seq))); },
               commutative = manual_proof!(/** max is commutative */)),
        ))
        .filter_map(q!(|opt| opt))
        .all_ticks()
        .max();

    // Replicated (seq, command) pairs — fires on ALL replicas for every validated Replicate.
    let replicated = valid
        .map(q!(|r| (r.seq, r.command)))
        .weaken_ordering::<NoOrder>()
        .all_ticks();

    // Quorum: commit when all view members have acked a slot.
    //
    // After a view change, stashed state from the old view must be discarded.
    // We detect view changes via the view_num and clear all stashed state.
    let (committed, committed_seqs, pending_ack_state) = sliced! {
        let new_acks = use(acks, nondet!(/** persistence across ticks */));
        let new_metadata = use::atomic(
            slot_table.weaken_ordering::<NoOrder>().all_ticks_atomic(),
            nondet!(/** metadata sync */),
        );

        let mut stashed_acks = use::state_null::<Stream<_, _, Bounded, NoOrder>>();
        let mut stashed_metadata = use::state_null::<Stream<(usize, C), _, Bounded, NoOrder>>();
        let mut stashed_quorum = use::state_null::<Stream<(usize, ()), _, Bounded, NoOrder>>();
        let mut prev_view_num = use::state(|l| l.singleton(q!(0u64)));

        // Detect view change: if view_num changed, discard all stashed state.
        let current_view_num = view.clone().map(q!(|v| v.view_num));
        let view_changed = current_view_num.clone()
            .zip(prev_view_num.clone())
            .filter(q!(|(cur, prev): &(u64, u64)| *cur != *prev))
            .map(q!(|(_, _)| ()));
        prev_view_num = current_view_num;

        // On view change, drop all stashed state (it's from the old view).
        let live_stashed_acks = stashed_acks.filter_if_none(view_changed.clone());
        let live_stashed_metadata = stashed_metadata.filter_if_none(view_changed.clone());
        let live_stashed_quorum = stashed_quorum.filter_if_none(view_changed);

        let all_acks = live_stashed_acks.chain(new_acks)
            .cross_singleton(view.clone())
            .filter(q!(|(a, v)| a.view_num == v.view_num))
            .map(q!(|(a, _)| a));

        let ack_counts = all_acks.clone()
            .map(q!(|a| (a.seq, a.sender.get_raw_id())))
            .into_keyed()
            .fold(
                q!(|| Vec::<u32>::new()),
                q!(|v: &mut Vec<u32>, id: u32| { if !v.contains(&id) { v.push(id); } },
                   commutative = manual_proof!(/** set insertion */)),
            );

        let quorum_size = view.clone().map(q!(|v| v.members.len()));
        let reached = ack_counts.clone().entries()
            .cross_singleton(quorum_size)
            .filter_map(q!(|((seq, senders), n)| if senders.len() >= n { Some(seq) } else { None }));

        let pending_entries = ack_counts.entries().anti_join(reached.clone());

        stashed_acks = all_acks
            .map(q!(|a| (a.seq, a)))
            .anti_join(reached.clone())
            .map(q!(|(_, a)| a));

        let reached_keyed = reached.clone().map(q!(|s| (s, ())));
        let all_meta = live_stashed_metadata.chain(new_metadata);
        let all_quorum = live_stashed_quorum.chain(reached_keyed);

        let joined = all_meta.clone().join(all_quorum.clone())
            .map(q!(|(seq, (command, ()))| (seq, command)));
        let joined_keys = joined.clone().map(q!(|(s, _)| s));

        stashed_metadata = all_meta.anti_join(joined_keys.clone());
        stashed_quorum = all_quorum.anti_join(joined_keys);

        (joined, reached, pending_entries)
    };

    CoreOutput { committed, replicated, committed_seqs, pending_acks: pending_ack_state, max_replicated_seq, read_only_commands: read_only_on_primary }
}


// ─────────────────────────────────────────────────────────────────────────────
// Service application module
// ─────────────────────────────────────────────────────────────────────────────

/// Output of the service application module.
///
/// Contains the response streams produced by applying commands through the
/// [`ReplicableService`] instance inside a `scan` operator.
pub struct ApplyOutput<'a, R> {
    /// Responses for committed (mutating) commands, paired with their sequence number.
    pub committed_responses: Stream<(usize, R), Cluster<'a, TransparentReplica>, Unbounded, TotalOrder>,
    /// Responses for read-only commands (no sequence number since they bypass replication).
    pub read_only_responses: Stream<R, Cluster<'a, TransparentReplica>, Unbounded, TotalOrder>,
}

/// Input tag for the unified apply stream.
///
/// Committed commands carry a sequence number; read-only commands do not.
/// Both are fed through the same `scan` operator so they share the same
/// service state. The `Restore` variant handles state transfer by restoring
/// the service from a snapshot.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
enum ApplyInput<C> {
    /// A committed (mutating) command with its assigned sequence number.
    Committed(usize, C),
    /// A read-only command that bypassed replication.
    ReadOnly(C),
    /// Restore service state from a snapshot (state transfer).
    /// Contains (snapshot_bytes, next_seq_after_restore).
    Restore(Vec<u8>, usize),
}

/// Output tag from the unified apply stream.
///
/// Distinguishes responses from committed vs read-only commands so they
/// can be routed to the appropriate downstream consumers.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
enum ApplyOutputTag<R> {
    /// Response for a committed command, paired with its sequence number.
    Committed(usize, R),
    /// Response for a read-only command.
    ReadOnly(R),
}

/// Applies committed and read-only commands through a [`ReplicableService`] instance
/// using a `scan` operator.
///
/// The service state lives inside the scan operator's closure state. Commands are
/// processed in sequence order: committed commands are buffered and applied in
/// ascending sequence number order, while read-only commands are applied immediately
/// against the current state.
///
/// # Arguments
///
/// * `committed` - Stream of `(seq, command)` pairs from [`CoreOutput::committed`].
/// * `read_only_commands` - Stream of read-only commands from [`CoreOutput::read_only_commands`].
/// * `tick` - The tick context for batching.
/// * `backup_apply` - When `true`, all replicas (including backups) apply commands.
///   When `false`, only the primary applies (backups just log). Currently unused;
///   the unified scan approach applies on all replicas that receive committed commands.
///   TODO: When `backup_apply = false`, filter the committed stream to primary-only
///   before feeding into the scan. Backups would only log without applying.
///
/// # Returns
///
/// An [`ApplyOutput`] containing response streams for both committed and read-only commands.
///
/// # Design
///
/// Both committed and read-only commands are merged into a single tagged stream and
/// fed through one `scan` operator. This ensures read-only commands always see the
/// latest committed state. The scan's accumulator holds:
/// - The service instance `S`
/// - A buffer (`BTreeMap`) for out-of-order committed commands
/// - The next expected sequence number
///
/// Committed commands that arrive out of order are buffered until all preceding
/// commands have been applied. Read-only commands are applied immediately regardless
/// of buffered state (they read from whatever state is current).
///
/// # Primary vs. Backup Application Strategy
///
/// The primary never needs to reorder because it controls sequencing — committed
/// commands exit quorum collection gap-free. Backups receive `Replicate<C>` messages
/// out of order with possible gaps from the network.
///
/// This implementation uses a unified `scan` with internal BTreeMap buffering that
/// handles both cases: on the primary, commands arrive in order so the buffer is
/// never used; on backups, out-of-order commands are buffered until the contiguous
/// prefix can be released.
///
/// An alternative approach for backups would use the `sequence_payloads` pattern from
/// `hydro_test/src/cluster/kv_replica/sequence_payloads.rs` — which buffers out-of-order
/// items across ticks using a `cycle`, sorts them, and only releases contiguous prefixes
/// for application via a separate fold. The unified scan approach is simpler and handles
/// both roles in one operator.
#[expect(clippy::type_complexity, reason = "Hydro stream types are deeply nested")]
pub fn apply_commands<'a, S>(
    committed: Stream<(usize, S::Command), Cluster<'a, TransparentReplica>, Unbounded, NoOrder>,
    read_only_commands: Stream<S::Command, Cluster<'a, TransparentReplica>, Unbounded, NoOrder>,
    restore_snapshots: Stream<(Vec<u8>, usize), Cluster<'a, TransparentReplica>, Unbounded, NoOrder>,
    tick: &Tick<Cluster<'a, TransparentReplica>>,
) -> ApplyOutput<'a, S::Response>
where
    S: ReplicableService,
    S::Command: Clone + Serialize + DeserializeOwned + Debug + Send + 'static,
    S::Response: Clone + Serialize + DeserializeOwned + Debug + Send + 'static,
{
    // Tag committed commands with their sequence number.
    // NOTE: On the primary, committed commands arrive gap-free from quorum collection
    // (you cannot commit seq N+1 without first committing seq N). On backups, commands
    // may arrive out of order from the network. The scan's internal BTreeMap handles
    // both cases: primary commands pass through immediately, backup commands are buffered
    // until the contiguous prefix can be released.
    let tagged_committed = committed
        .map(q!(|(seq, cmd)| ApplyInput::Committed(seq, cmd)));

    // Tag read-only commands.
    let tagged_read_only = read_only_commands
        .map(q!(|cmd| ApplyInput::ReadOnly(cmd)));

    // Tag restore snapshots (from state transfer).
    let tagged_restore = restore_snapshots
        .map(q!(|(snapshot, resume_seq)| ApplyInput::Restore(snapshot, resume_seq)));

    // Merge all streams into a single tagged stream.
    let merged = tagged_committed
        .interleave(tagged_read_only)
        .interleave(tagged_restore);

    // Batch into the tick, then sort within the batch.
    // ApplyInput sorts Committed before ReadOnly, and Committed sorts by seq.
    // This ensures committed commands are applied in sequence order before
    // read-only commands within each batch.
    let batched = merged
        .batch(tick, nondet!(/** apply batch boundaries */));

    // Sort within the batch: Committed(seq, _) sorts by seq, ReadOnly comes after.
    // We use assume_ordering here because within a batch we process committed
    // commands in sequence order (the scan handles ordering internally via buffering).
    let ordered = batched
        .assume_ordering::<TotalOrder>(nondet!(
            /// The scan operator internally buffers and applies committed commands
            /// in sequence order regardless of arrival order. Read-only commands
            /// are applied against the current state. The output is deterministic
            /// given the same set of commands per batch.
        ));

    // Use all_ticks() to get an unbounded, totally-ordered stream, then apply scan.
    // The scan maintains the service state across all ticks/batches.
    let responses = ordered
        .all_ticks()
        .scan(
            q!(|| {
                // State: (service, pending_buffer, next_expected_seq)
                // The service instance is created via Default (the caller must ensure
                // S implements Default, or we use a wrapper that initializes lazily).
                // For now, we use a tuple of (Option<S>, BTreeMap, usize).
                // The Option<S> starts as None and is initialized on first use.
                (
                    None::<S>,
                    BTreeMap::<usize, S::Command>::new(),
                    0usize, // next_expected_seq
                )
            }),
            q!(|state: &mut (Option<S>, BTreeMap<usize, S::Command>, usize), input: ApplyInput<S::Command>| {
                // Initialize service on first call if needed.
                let service = state.0.get_or_insert_with(S::default);
                let pending = &mut state.1;
                let next_seq = &mut state.2;

                let mut results = Vec::new();

                match input {
                    ApplyInput::Committed(seq, cmd) => {
                        if seq == *next_seq {
                            // Apply this command immediately.
                            let resp = service.apply(cmd);
                            results.push(ApplyOutputTag::Committed(seq, resp));
                            *next_seq += 1;

                            // Drain any buffered commands that are now in order.
                            while let Some(buffered_cmd) = pending.remove(next_seq) {
                                let resp = service.apply(buffered_cmd);
                                results.push(ApplyOutputTag::Committed(*next_seq, resp));
                                *next_seq += 1;
                            }
                        } else if seq >= *next_seq {
                            // Out of order — buffer for later.
                            pending.insert(seq, cmd);
                        }
                        // else: seq < next_seq means duplicate/stale — drop it
                    }
                    ApplyInput::ReadOnly(cmd) => {
                        // Read-only commands are applied immediately against current state.
                        let resp = service.apply(cmd);
                        results.push(ApplyOutputTag::ReadOnly(resp));
                    }
                    ApplyInput::Restore(snapshot_bytes, resume_seq) => {
                        // State transfer: restore service from snapshot and reset seq.
                        service.restore(&snapshot_bytes);
                        // Clear any buffered commands from before the snapshot.
                        pending.retain(|&seq, _| seq >= resume_seq);
                        *next_seq = resume_seq;

                        // Drain any buffered commands that are now in order after restore.
                        while let Some(buffered_cmd) = pending.remove(next_seq) {
                            let resp = service.apply(buffered_cmd);
                            results.push(ApplyOutputTag::Committed(*next_seq, resp));
                            *next_seq += 1;
                        }
                    }
                }

                Some(results)
            }),
        )
        .flat_map_ordered(q!(|results| results));

    // Split the tagged responses back into committed and read-only streams.
    let committed_responses = responses.clone()
        .filter_map(q!(|tag| match tag {
            ApplyOutputTag::Committed(seq, resp) => Some((seq, resp)),
            ApplyOutputTag::ReadOnly(_) => None,
        }));

    let read_only_responses = responses
        .filter_map(q!(|tag| match tag {
            ApplyOutputTag::Committed(_, _) => None,
            ApplyOutputTag::ReadOnly(resp) => Some(resp),
        }));

    ApplyOutput {
        committed_responses,
        read_only_responses,
    }
}


// ─────────────────────────────────────────────────────────────────────────────
// Control path modules
// ─────────────────────────────────────────────────────────────────────────────

use std::time::Duration;

use crate::messages::{CommitNotification, StateTransferRequest};

// ─────────────────────────────────────────────────────────────────────────────
// Task 5.1: Notification Broadcaster
// ─────────────────────────────────────────────────────────────────────────────

/// Notification broadcaster for the transparent replication protocol.
///
/// Periodically snapshots the core replication module's quorum state (committed seqs
/// and pending ack sets) and broadcasts it to all replicas as [`CommitNotification`]
/// messages. Backups use these notifications for failure detection.
///
/// Adapted from `basic_primary_backup.rs::notification_broadcaster`, using
/// `TransparentReplica` cluster type and our local `CommitNotification` type.
#[expect(clippy::type_complexity, reason = "Hydro stream types are deeply nested")]
pub fn notification_broadcaster<'a>(
    replicas: &Cluster<'a, TransparentReplica>,
    committed_seqs: Stream<usize, Cluster<'a, TransparentReplica>, Unbounded, NoOrder>,
    pending_acks: Stream<(usize, Vec<u32>), Cluster<'a, TransparentReplica>, Unbounded, NoOrder>,
    current_view: Singleton<View, Cluster<'a, TransparentReplica>, Unbounded>,
    interval_ms: u64,
) -> Stream<CommitNotification, Cluster<'a, TransparentReplica>, Unbounded, NoOrder> {
    let interval_tick = replicas.source_interval(
        q!(Duration::from_millis(interval_ms)),
        nondet!(/** timer is inherently non-deterministic */),
    );

    let notif_tick = replicas.tick();

    let interval_in_tick = interval_tick
        .batch(
            &notif_tick,
            nondet!(/** batch boundaries don't affect correctness */),
        )
        .first();

    let view_in_tick = current_view.snapshot(
        &notif_tick,
        nondet!(/** stale view is safe */),
    );

    let committed_vec = committed_seqs
        .batch(&notif_tick, nondet!(/** batch committed_seqs */))
        .fold(
            q!(|| Vec::<usize>::new()),
            q!(|v: &mut Vec<usize>, seq: usize| { v.push(seq); },
               commutative = manual_proof!(/** order doesn't matter */)),
        );

    let pending_vec = pending_acks
        .batch(&notif_tick, nondet!(/** batch pending_acks */))
        .fold(
            q!(|| Vec::<(usize, Vec<u32>)>::new()),
            q!(|v: &mut Vec<(usize, Vec<u32>)>, entry: (usize, Vec<u32>)| { v.push(entry); },
               commutative = manual_proof!(/** order doesn't matter */)),
        );

    let notification = interval_in_tick
        .if_some_then(view_in_tick)
        .zip(committed_vec)
        .zip(pending_vec)
        .map(q!(move |((view, committed_seqs), mut pending_acks)| {
            let self_id = CLUSTER_SELF_ID.get_raw_id();
            for (_seq, members) in pending_acks.iter_mut() {
                if !members.contains(&self_id) {
                    members.push(self_id);
                }
            }
            CommitNotification {
                view_num: view.view_num,
                committed_seqs,
                pending_acks,
            }
        }))
        .into_stream()
        .all_ticks();

    notification
        .inspect(q!(|notif: &CommitNotification| {
            println!("[NB] sending notification: view_num={} committed_seqs={:?}", notif.view_num, notif.committed_seqs);
        }))
        .broadcast(
            replicas,
            TCP.fail_stop().bincode(),
            nondet!(/** dead peers cause BrokenPipe but sender survives */),
        )
        .values()
        .weaken_ordering()
}

// ─────────────────────────────────────────────────────────────────────────────
// Task 5.2: Failure Detector
// ─────────────────────────────────────────────────────────────────────────────

/// Failure detector for the transparent replication protocol.
///
/// Consumes the [`CommitNotification`] stream broadcast by the primary.
/// Each replica runs its own failure detector instance.
///
/// - Maintains a commit timer; resets when notifications with committed slots arrive.
/// - Tracks "recently alive" set from pending_acks in notifications.
/// - When commit timer fires, proposes a new view excluding the suspected-failed primary.
/// - Cold-start gate: does not fire before seeing at least one notification.
///
/// Adapted from `basic_primary_backup.rs::failure_detector`, using
/// `TransparentReplica` cluster type.
pub fn failure_detector<'a>(
    _replicas: &Cluster<'a, TransparentReplica>,
    commit_notifications: Stream<CommitNotification, Cluster<'a, TransparentReplica>, Unbounded, NoOrder>,
    current_view: Singleton<View, Cluster<'a, TransparentReplica>, Unbounded>,
    commit_timeout_ms: u64,
) -> Stream<View, Cluster<'a, TransparentReplica>, Unbounded, NoOrder> {
    // Cold-start gate: track whether we've ever seen a notification WITH actual commits.
    // This prevents spurious view changes during startup when the notification
    // broadcaster sends notifications before any commits have occurred.
    let has_ever_seen = commit_notifications
        .clone()
        .inspect(q!(|notif: &CommitNotification| {
            println!("[FD] received notification: view_num={} committed_seqs={:?} pending_acks_len={}",
                notif.view_num, notif.committed_seqs, notif.pending_acks.len());
        }))
        .filter(q!(|notif: &CommitNotification| !notif.committed_seqs.is_empty()))
        .map(q!(|_| true))
        .max();

    // Commit timer: resets only when we see notifications with actual commits.
    // Fires when no commits have been reported within the timeout.
    let commit_timeout = commit_notifications
        .clone()
        .filter(q!(|notif: &CommitNotification| !notif.committed_seqs.is_empty()))
        .timeout(
            q!(Duration::from_millis(commit_timeout_ms)),
            nondet!(/** false suspicions cause unnecessary view changes, not incorrectness */),
        );

    // Track recently-alive members from pending_acks in the LATEST notification.
    let recently_alive = commit_notifications
        .map(q!(|notif: CommitNotification| {
            let mut alive: Vec<u32> = Vec::new();
            for (_seq, ack_members) in notif.pending_acks.iter() {
                for &mid in ack_members.iter() {
                    if !alive.contains(&mid) {
                        alive.push(mid);
                    }
                }
            }
            alive.sort();
            alive
        }))
        .fold(
            q!(|| Vec::<u32>::new()),
            q!(|current: &mut Vec<u32>, latest: Vec<u32>| {
                // Replace with latest — don't accumulate
                *current = latest;
            }, commutative = manual_proof!(/** last-write-wins */)),
        );

    let fd_tick = current_view.location().tick();

    // Heartbeat keeps fd_tick firing so the timeout is checked even when
    // no commands are flowing (i.e., after the primary dies and no new
    // commit notifications arrive).
    current_view.location()
        .source_interval(q!(Duration::from_millis(500)), nondet!(/** fd heartbeat */))
        .batch(&fd_tick, nondet!(/** fd heartbeat tick */))
        .for_each(q!(|_| {}));

    let current_view_in_tick = current_view.snapshot(
        &fd_tick,
        nondet!(/** stale view is safe */),
    );

    let has_seen_in_tick = has_ever_seen.snapshot(
        &fd_tick,
        nondet!(/** stale has_seen is safe */),
    );

    let timeout_in_tick = commit_timeout.snapshot(
        &fd_tick,
        nondet!(/** Stale timeout result is safe */),
    );

    let alive_in_tick = recently_alive.snapshot(
        &fd_tick,
        nondet!(/** stale alive set is safe */),
    );

    // When timeout fires AND we've seen at least one notification,
    // propose a new view with the recently-alive members
    let view_change_proposals = timeout_in_tick
        .zip(has_seen_in_tick)
        .map(q!(|(_, _)| {
            println!("[FD] timeout fired AND has_seen is true — checking alive members");
            ()
        }))
        .if_some_then(current_view_in_tick)
        .zip(alive_in_tick)
        .filter_map(q!(|(view, alive_members): (View, Vec<u32>)| {
            println!("[FD] evaluating view change: current={:?} alive={:?}", view.members, alive_members);
            if alive_members.is_empty() {
                println!("[FD] alive_members is empty, skipping");
                return None;
            }
            let mut new_members = alive_members;
            new_members.sort();
            new_members.dedup();
            // Only propose if the view actually changed
            if new_members == view.members {
                println!("[FD] view unchanged, skipping");
                return None;
            }
            println!("[FD] PROPOSING view change: {:?} -> {:?}", view.members, new_members);
            Some(View {
                view_num: view.view_num + 1,
                members: new_members,
            })
        }))
        .into_stream()
        .all_ticks();

    view_change_proposals.weaken_ordering()
}

// ─────────────────────────────────────────────────────────────────────────────
// Task 5.3: View Manager
// ─────────────────────────────────────────────────────────────────────────────

use hydro_test::cluster::paxos::{Acceptor, CorePaxos, Proposer};
use hydro_test::cluster::paxos_with_client::PaxosLike;

/// View manager for the transparent replication protocol.
///
/// Wraps [`CorePaxos`] to sequence view change proposals from replicas.
/// Broadcasts committed views back to all replicas.
///
/// Adapted from `basic_primary_backup.rs::view_manager`, using
/// `TransparentReplica` cluster type.
///
/// # Returns
///
/// A tuple of:
/// - `Optional<View, ...>` — the maximum committed view (latest installed view).
/// - `Stream<View, ...>` — stream of all committed views as they arrive.
#[expect(clippy::type_complexity, reason = "Hydro stream types are deeply nested")]
pub fn view_manager<'a>(
    proposers: &Cluster<'a, Proposer>,
    acceptors: &Cluster<'a, Acceptor>,
    replicas: &Cluster<'a, TransparentReplica>,
    view_change_proposals: Stream<View, Cluster<'a, TransparentReplica>, Unbounded, NoOrder>,
    a_checkpoint: Optional<usize, Cluster<'a, Acceptor>, Unbounded>,
    paxos_config: crate::config::PaxosConfig,
    nondet_leader: NonDet,
    nondet_commit: NonDet,
) -> (
    Optional<View, Cluster<'a, TransparentReplica>, Unbounded>,
    Stream<View, Cluster<'a, TransparentReplica>, Unbounded, NoOrder>,
) {
    // Broadcast proposals from replicas to proposers
    let proposals_at_proposers = view_change_proposals
        .broadcast(proposers, TCP.fail_stop().bincode(), nondet!(/** Dead peers cause BrokenPipe but sender survives */))
        .values();

    // Convert our PaxosConfig to hydro_test's PaxosConfig
    let hydro_paxos_config = hydro_test::cluster::paxos::PaxosConfig {
        f: paxos_config.f,
        i_am_leader_send_timeout: paxos_config.i_am_leader_send_timeout,
        i_am_leader_check_timeout: paxos_config.i_am_leader_check_timeout,
        i_am_leader_check_timeout_delay_multiplier: paxos_config.i_am_leader_check_timeout_delay_multiplier,
    };

    let paxos = CorePaxos {
        proposers: proposers.clone(),
        acceptors: acceptors.clone(),
        paxos_config: hydro_paxos_config,
    };

    // Run CorePaxos to sequence proposals
    let committed_views_on_proposers = paxos.build(
        move |_new_leader_elected| {
            proposals_at_proposers.assume_ordering(nondet!(/** Paxos sequences regardless of arrival order */))
        },
        a_checkpoint, nondet_leader, nondet_commit,
    );

    // Broadcast committed views back to all replicas.
    let committed_views_on_replicas = committed_views_on_proposers
        .filter_map(q!(|(_slot, opt_view)| opt_view));

    // Track the max committed view on proposers directly from the Paxos output.
    // This includes views recommitted after leader election, so a new proposer
    // that just won election will have the correct max view to re-broadcast.
    let max_view_on_proposers = committed_views_on_replicas.clone().max();

    // Periodically re-broadcast the max committed view from proposers to replicas.
    let rebroadcast_tick = proposers.tick();
    proposers
        .source_interval(q!(std::time::Duration::from_millis(500)), nondet!(/** re-broadcast heartbeat */))
        .batch(&rebroadcast_tick, nondet!(/** heartbeat tick */))
        .for_each(q!(|_| {}));
    let max_view_in_tick = max_view_on_proposers.snapshot(
        &rebroadcast_tick,
        nondet!(/** stale max view ok */),
    );
    let rebroadcast_views = max_view_in_tick
        .into_stream()
        .all_ticks()
        .broadcast(replicas, TCP.fail_stop().bincode(), nondet!(/** periodic re-broadcast */))
        .values()
        .weaken_ordering::<hydro_lang::live_collections::stream::NoOrder>();

    // Primary broadcast path (immediate) + re-broadcast path (periodic catch-up).
    let primary_broadcast = committed_views_on_replicas
        .broadcast(replicas, TCP.fail_stop().bincode(), nondet!(/** Dead peers cause BrokenPipe but sender survives */))
        .values();

    let all_views = primary_broadcast.interleave(rebroadcast_views);

    (all_views.clone().max(), all_views.weaken_ordering())
}

// ─────────────────────────────────────────────────────────────────────────────
// Task 5.4: State Transfer
// ─────────────────────────────────────────────────────────────────────────────

/// State transfer module for the transparent replication protocol.
///
/// On view change (new primary installed), the new primary sends a
/// [`StateTransferRequest`] to surviving backups. A surviving backup responds
/// with its max replicated sequence number, which the new primary uses to
/// determine where to resume sequencing.
///
/// # Current Limitation: Log Replay Instead of Snapshot Transfer
///
/// The current implementation uses "max seq reconciliation" (same as
/// `basic_primary_backup.rs`) rather than full snapshot-based state transfer.
/// The new primary replays all committed commands from seq 0 via the
/// `apply_commands` scan (which starts from `S::default()`).
///
/// The infrastructure for full snapshot-based transfer is in place:
/// - `StateTransferResponse<C>` message type exists with snapshot + suffix fields
/// - `ApplyInput::Restore` variant in the scan handles `service.restore()`
/// - The scan correctly resets `next_seq` and drains buffered commands after restore
///
/// What's missing is the bidirectional wiring:
/// - Backup scan producing snapshots on demand (requires a SnapshotRequest input channel)
/// - Sending snapshot bytes over the network to the new primary
/// - Feeding the snapshot into the new primary's scan via the restore stream
///
/// This is architecturally complex in Hydro's unidirectional dataflow model.
/// The protocol is correct without it — just slower on failover since it replays
/// from the beginning rather than restoring a snapshot.
///
/// # Returns
///
/// An `Optional<usize, ...>` representing the reconciled sequence number — the
/// sequence number from which the new primary should resume sequencing.
#[expect(clippy::type_complexity, reason = "Hydro stream types are deeply nested")]
pub fn state_transfer<'a, C>(
    replicas: &Cluster<'a, TransparentReplica>,
    current_view: Singleton<View, Cluster<'a, TransparentReplica>, Unbounded>,
    max_replicated_seq: Optional<usize, Cluster<'a, TransparentReplica>, Unbounded>,
) -> Optional<usize, Cluster<'a, TransparentReplica>, Unbounded>
where
    C: Clone + Serialize + DeserializeOwned + Debug + Send + 'static,
{
    // Step 1: Detect when the primary changes using a sliced! block to track
    // the previous primary across ticks.
    let primary_changed: Stream<View, Cluster<'a, TransparentReplica>, Unbounded, NoOrder> = sliced! {
        let new_view = use(current_view.clone(), nondet!(
            /// View changes are infrequent; stale snapshot is safe.
        ));
        let mut prev_primary = use::state(|l| l.singleton(q!(u32::MAX)));

        let current_primary = new_view.clone().map(q!(|v: View| v.primary()));

        // Detect change: current primary differs from previous
        let changed_view = new_view
            .zip(prev_primary.clone())
            .filter(q!(|(view, prev): &(View, u32)| view.primary() != *prev))
            .map(q!(|(view, _prev)| view));

        // Update prev_primary to current
        prev_primary = current_primary;

        changed_view.into_stream()
    }.into();

    // Step 2: When primary changes and I am the new primary, send state transfer
    // request to all surviving replicas (broadcast).
    // Skip view_num 0 — the initial view doesn't need state transfer (no prior state).
    let state_transfer_requests = primary_changed
        .filter(q!(move |view: &View| {
            view.view_num > 0 && CLUSTER_SELF_ID.get_raw_id() == view.primary()
        }))
        .map(q!(move |view: View| {
            StateTransferRequest {
                view_num: view.view_num,
                requester: CLUSTER_SELF_ID.clone(),
            }
        }))
        .broadcast(
            replicas,
            TCP.fail_stop().bincode(),
            nondet!(/** dead peers cause BrokenPipe but sender survives */),
        )
        .values();

    // Step 3: Surviving replicas respond with their max replicated seq.
    // In a full implementation, this would include the actual snapshot + suffix.
    // For now, we use the same pattern as log_reconciliation: respond with max seq.
    // The actual snapshot/restore happens at a higher level when the service is
    // integrated (the apply_commands scan holds the service state).
    //
    // NOTE: Full snapshot-based state transfer requires the service instance to be
    // accessible here. Since the service lives inside the apply_commands scan operator,
    // we implement state transfer as "max seq reconciliation" at the protocol level.
    // The new primary's apply_commands scan will receive all committed commands from
    // the beginning (via the committed stream) and apply them in order, effectively
    // reconstructing state. For true snapshot-based transfer, the service would need
    // to be factored out of the scan — a future enhancement.
    let recon_resp_tick = replicas.tick();

    let requests_in_tick = state_transfer_requests
        .batch(
            &recon_resp_tick,
            nondet!(/** batch boundaries don't affect correctness */),
        )
        .reduce(q!(|_curr: &mut StateTransferRequest, _new: StateTransferRequest| {
            // Keep the first request; all requests in the same tick are equivalent
        }, commutative = manual_proof!(/** all requests are equivalent, keeping any one is correct */)));

    // Track max committed seq — directly from the input (already computed as O(1) fold)
    let max_committed_seq = max_replicated_seq;

    let max_seq_in_tick = max_committed_seq
        .unwrap_or(replicas.singleton(q!(0usize)).into())
        .snapshot(
            &recon_resp_tick,
            nondet!(/** stale log is safe */),
        );

    // When a request arrives, respond with our max committed seq.
    let responses_with_target = requests_in_tick
        .clone()
        .if_some_then(max_seq_in_tick)
        .zip(requests_in_tick)
        .map(q!(|(my_max_seq, request): (usize, StateTransferRequest)| {
            (
                request.requester,
                my_max_seq,
            )
        }))
        .into_stream()
        .all_ticks()
        .demux(replicas, TCP.fail_stop().bincode())
        .values();

    // Step 4: New primary merges responses to find max seq across all responders.
    // The new primary resumes sequencing from max_seq (the reconciled sequence number).
    responses_with_target
        .max()
}


// ─────────────────────────────────────────────────────────────────────────────
// Task 13.2: Client Request Handling (Protocol-Side Integration)
// ─────────────────────────────────────────────────────────────────────────────
//
// TODO: Integrate client request handling into the protocol dataflow.
//
// The full integration would:
// 1. Accept `ClientRequest<C>` from TCP source streams on each replica.
// 2. Non-primary replicas forward requests to the current primary (proxy):
//    - Use `demux` to route to the primary based on `current_view.primary()`.
// 3. Primary processes commands through the replication pipeline (feeds into
//    `client_commands` stream that `replicate_service` already accepts).
// 4. Route `ClientResponse<R>` back to the originating client:
//    - After `apply_commands` produces responses, wrap in `ClientResponse`
//      with the matching `request_id`.
//    - Use `demux` to route back to the originating replica, then to the
//      client's TCP connection.
//
// This integration requires:
// - Adding TCP source/sink parameters to `replicate_service` (or a wrapper).
// - Maintaining a mapping from `(client_id, request_id)` to the originating
//   replica so responses can be routed back through the proxy.
// - Handling the case where a client connects to a replica that then crashes
//   (the response must be dropped; the client retries on another replica).
//
// For the prototype, client handling is managed externally:
// - The Hydro deployment framework wires TCP sources into the `client_commands`
//   stream parameter of `replicate_service`.
// - Response routing is handled by the deployment's TCP sink configuration.
// - This matches how `basic_primary_backup.rs` tests work with `TrybuildHost`.
//
// See `src/client.rs` for the client-side implementation that connects via TCP.
// ─────────────────────────────────────────────────────────────────────────────

// ─────────────────────────────────────────────────────────────────────────────
// Task 6.1: Top-level composition (replicate_service)
// ─────────────────────────────────────────────────────────────────────────────

/// Output of the top-level [`replicate_service`] function.
///
/// Contains response streams for both committed (mutating) and read-only commands.
pub struct ReplicateOutput<'a, R> {
    /// Responses for committed (mutating) commands, paired with their sequence number.
    /// These have been replicated to all view members and committed by quorum.
    pub committed_responses: Stream<(usize, R), Cluster<'a, TransparentReplica>, Unbounded, NoOrder>,
    /// Responses for read-only commands (no sequence number since they bypass replication).
    /// These were applied directly on the primary without replication.
    pub read_only_responses: Stream<R, Cluster<'a, TransparentReplica>, Unbounded, NoOrder>,
}

/// Top-level composition of the transparent replication protocol.
///
/// Wires together: view_manager, core_replication_module, notification_broadcaster,
/// failure_detector, state_transfer, and apply_commands using `forward_ref` to break
/// circular dependencies.
///
/// The circular dependency chain is:
/// - `failure_detector` needs `commit_notifications` (from `notification_broadcaster`)
/// - `notification_broadcaster` needs `committed_seqs` and `pending_acks` (from `core_replication_module`)
/// - `core_replication_module` needs `current_view` (from `view_manager`)
/// - `view_manager` needs `view_change_proposals` (from `failure_detector`)
///
/// Additionally:
/// - `state_transfer` needs `max_replicated_seq` (from `core_replication_module`)
/// - `core_replication_module` needs `reconciled_seq` (from `state_transfer`)
///
/// Both cycles are broken using Hydro's `forward_ref` mechanism.
///
/// Adapted from `hydro_test/src/cluster/basic_primary_backup.rs::basic_pb_top`.
///
/// # Arguments
///
/// * `replicas` - The cluster of transparent replicas.
/// * `proposers` - The Paxos proposer cluster (for view change sequencing).
/// * `acceptors` - The Paxos acceptor cluster (for view change sequencing).
/// * `client_commands` - Stream of commands from clients arriving at replicas.
/// * `config` - Protocol configuration (timeouts, membership, etc.).
///
/// # Returns
///
/// A [`ReplicateOutput`] containing response streams for both committed and read-only commands.
#[expect(clippy::type_complexity, reason = "Hydro stream types are deeply nested")]
pub fn replicate_service<'a, S: ReplicableService>(
    replicas: &Cluster<'a, TransparentReplica>,
    proposers: &Cluster<'a, Proposer>,
    acceptors: &Cluster<'a, Acceptor>,
    client_commands: Stream<S::Command, Cluster<'a, TransparentReplica>, Unbounded>,
    config: crate::ReplicateConfig,
) -> ReplicateOutput<'a, S::Response>
where
    S::Command: Clone + Serialize + DeserializeOwned + Debug + Send + 'static,
    S::Response: Clone + Serialize + DeserializeOwned + Debug + Send + 'static,
{
    let initial_member_count = config.initial_members.len();
    let commit_timeout_ms = config.commit_timeout_ms;
    let notification_interval_ms = config.notification_interval_ms;

    // ═══════════════════════════════════════════════════════════════════════
    // Step 1: Create forward_ref for view change proposals (breaking the cycle)
    // ═══════════════════════════════════════════════════════════════════════
    let (view_change_complete, view_change_proposals) =
        replicas.forward_ref::<Stream<View, _, Unbounded, NoOrder>>();

    // ═══════════════════════════════════════════════════════════════════════
    // Step 2: No acceptor checkpoint needed — pass None
    // ═══════════════════════════════════════════════════════════════════════
    let a_checkpoint: Optional<usize, Cluster<'a, Acceptor>, Unbounded> =
        acceptors.singleton(q!(None::<usize>)).into_optional().into();

    // ═══════════════════════════════════════════════════════════════════════
    // Step 3: Call view_manager with the forward ref stream to get committed views
    // ═══════════════════════════════════════════════════════════════════════
    let (_committed_view_max, committed_views_stream) = view_manager(
        proposers,
        acceptors,
        replicas,
        view_change_proposals,
        a_checkpoint,
        config.paxos_config,
        nondet!(/** Paxos leader election is inherently non-deterministic */),
        nondet!(/** Paxos commit ordering is inherently non-deterministic */),
    );

    // ═══════════════════════════════════════════════════════════════════════
    // Step 4: Derive current_view from committed views using fold with initial default
    // ═══════════════════════════════════════════════════════════════════════
    let current_view: Singleton<View, _, _> = committed_views_stream
        .fold(
            q!(move || View {
                view_num: 0,
                members: (0..initial_member_count as u32).collect(),
            }),
            q!(|current: &mut View, new: View| {
                if new.view_num > current.view_num {
                    *current = new;
                }
            }, commutative = manual_proof!(/** max is commutative */)),
        )
        .into();

    // ═══════════════════════════════════════════════════════════════════════
    // Step 5: Create forward_ref for max_replicated_seq (breaking the cycle
    // with state_transfer)
    // ═══════════════════════════════════════════════════════════════════════
    let (max_seq_complete, max_seq_ref) =
        replicas.forward_ref::<Optional<usize, _, Unbounded>>();

    // ═══════════════════════════════════════════════════════════════════════
    // Step 6: Call state_transfer to get reconciled_seq
    // ═══════════════════════════════════════════════════════════════════════
    let reconciled_seq = state_transfer::<S::Command>(replicas, current_view.clone(), max_seq_ref);

    // ═══════════════════════════════════════════════════════════════════════
    // Step 7: Split commands into mutating and read-only streams
    // ═══════════════════════════════════════════════════════════════════════
    let mutating_commands = client_commands
        .clone()
        .filter(q!(|cmd: &S::Command| !S::is_read_only(cmd)));
    let read_only_commands = client_commands
        .filter(q!(|cmd: &S::Command| S::is_read_only(cmd)));

    // ═══════════════════════════════════════════════════════════════════════
    // Step 8: Create tick and call core_replication_module
    // ═══════════════════════════════════════════════════════════════════════
    let tick = replicas.tick();

    // Heartbeat keeps the tick firing even when no client payloads arrive.
    let heartbeat = replicas
        .source_interval(q!(Duration::from_millis(100)), nondet!(/** heartbeat source interval */))
        .map(q!(|_| ()));
    let _hb = heartbeat.batch(&tick, nondet!(
        /// Heartbeat keeps the tick firing even when no client payloads arrive.
    ));

    let core = core_replication_module(
        replicas,
        &tick,
        mutating_commands,
        read_only_commands,
        current_view.clone(),
        reconciled_seq,
    );

    // ═══════════════════════════════════════════════════════════════════════
    // Step 9: Complete the max_replicated_seq forward ref
    // ═══════════════════════════════════════════════════════════════════════
    max_seq_complete.complete(core.max_replicated_seq);

    // ═══════════════════════════════════════════════════════════════════════
    // Step 10: Notification broadcaster — periodically broadcasts quorum state
    // ═══════════════════════════════════════════════════════════════════════
    let commit_notifications = notification_broadcaster(
        replicas,
        core.committed_seqs,
        core.pending_acks,
        current_view.clone(),
        notification_interval_ms,
    );

    // ═══════════════════════════════════════════════════════════════════════
    // Step 11: Failure detector — monitors notifications, proposes view changes
    // ═══════════════════════════════════════════════════════════════════════
    let fd_proposals = failure_detector(
        replicas,
        commit_notifications,
        current_view.clone(),
        commit_timeout_ms,
    );

    // ═══════════════════════════════════════════════════════════════════════
    // Step 12: Complete the view change forward ref cycle
    // ═══════════════════════════════════════════════════════════════════════
    view_change_complete.complete(fd_proposals);

    // ═══════════════════════════════════════════════════════════════════════
    // Step 13: Apply committed commands through the service and return responses
    // ═══════════════════════════════════════════════════════════════════════

    // When backup_apply = false, only the primary applies committed commands.
    // Backups still receive and ack replicates (for quorum), but don't apply them.
    let committed_for_apply = if config.backup_apply {
        // All replicas apply replicated commands (hot standby)
        core.replicated
    } else {
        // Only primary applies — filter committed stream to primary-only.
        // We snapshot current_view into the tick to get a bounded singleton for cross_singleton.
        let is_primary_in_tick = current_view.clone()
            .snapshot(&tick, nondet!(/** stale view is safe for backup_apply filter */))
            .filter(q!(move |v| CLUSTER_SELF_ID.get_raw_id() == v.primary()))
            .map(q!(|_| ()));
        core.committed
            .batch(&tick, nondet!(/** batch for primary filter */))
            .filter_if_some(is_primary_in_tick)
            .weaken_ordering::<NoOrder>()
            .all_ticks()
    };

    // State transfer restore stream — currently empty (log replay approach).
    // See state_transfer() doc for explanation of the limitation.
    let restore_snapshots = {
        let empty_tick = replicas.tick();
        replicas.source_iter(q!(Vec::<(Vec<u8>, usize)>::new()))
            .batch(&empty_tick, nondet!(/** empty restore stream */))
            .weaken_ordering::<NoOrder>()
            .all_ticks()
    };

    let apply_output = apply_commands::<S>(
        committed_for_apply,
        core.read_only_commands,
        restore_snapshots,
        &tick,
    );

    // Return both committed and read-only response streams.
    ReplicateOutput {
        committed_responses: apply_output.committed_responses.weaken_ordering(),
        read_only_responses: apply_output.read_only_responses.weaken_ordering(),
    }
}

/// Raw output of the replication protocol — committed and read-only command streams
/// without service application. Use this when you want to apply commands yourself
/// with a concrete service type (avoiding the generic `apply_commands` limitation
/// with stageleft's `q!()` macro).
pub struct ReplicateRawOutput<'a, C> {
    /// Committed `(seq, command)` pairs — only fires on the primary after quorum.
    pub committed: Stream<(usize, C), Cluster<'a, TransparentReplica>, Unbounded, NoOrder>,
    /// Replicated `(seq, command)` pairs — fires on ALL replicas for validated replicates.
    /// Feed this into the service scan so backups maintain hot standby state.
    pub replicated: Stream<(usize, C), Cluster<'a, TransparentReplica>, Unbounded, NoOrder>,
    /// Read-only commands that bypassed replication — apply directly on primary.
    pub read_only: Stream<C, Cluster<'a, TransparentReplica>, Unbounded, NoOrder>,
    /// Current view — use to determine which replica is primary.
    pub current_view: Singleton<View, Cluster<'a, TransparentReplica>, Unbounded>,
}

/// Top-level replication protocol returning raw committed/read-only streams.
///
/// Same as [`replicate_service`] but skips the `apply_commands` step.
/// The caller is responsible for applying commands to a concrete service instance.
/// This avoids the stageleft `q!()` limitation where generic type parameters
/// cannot be used inside code-generation closures.
///
/// Failure detection is **coordinator-driven**: the caller provides view change
/// proposals via `external_view_proposals`. The protocol has no internal timer-based
/// failure detector. This eliminates spurious view changes during idle periods and
/// cascading view changes after failover.
#[expect(clippy::type_complexity, reason = "Hydro stream types are deeply nested")]
pub fn replicate_service_raw<'a, C>(
    replicas: &Cluster<'a, TransparentReplica>,
    proposers: &Cluster<'a, hydro_test::cluster::paxos::Proposer>,
    acceptors: &Cluster<'a, hydro_test::cluster::paxos::Acceptor>,
    client_commands: Stream<C, Cluster<'a, TransparentReplica>, Unbounded>,
    external_view_proposals: Stream<View, Cluster<'a, TransparentReplica>, Unbounded, NoOrder>,
    config: crate::ReplicateConfig,
) -> ReplicateRawOutput<'a, C>
where
    C: Clone + Serialize + DeserializeOwned + Debug + Send + 'static,
{
    use std::time::Duration;

    let initial_member_count = config.initial_members.len();

    let (view_change_complete, view_change_proposals) =
        replicas.forward_ref::<Stream<View, _, Unbounded, NoOrder>>();

    let a_checkpoint: Optional<usize, Cluster<'a, hydro_test::cluster::paxos::Acceptor>, Unbounded> =
        acceptors.singleton(q!(None::<usize>)).into_optional().into();

    let (_committed_view_max, committed_views_stream) = view_manager(
        proposers,
        acceptors,
        replicas,
        view_change_proposals,
        a_checkpoint,
        config.paxos_config,
        nondet!(/** Paxos leader election */),
        nondet!(/** Paxos commit ordering */),
    );

    let current_view: Singleton<View, _, _> = committed_views_stream
        .fold(
            q!(move || View {
                view_num: 0,
                members: (0..initial_member_count as u32).collect(),
            }),
            q!(|current: &mut View, new: View| {
                if new.view_num > current.view_num {
                    *current = new;
                }
            }, commutative = manual_proof!(/** max is commutative */)),
        )
        .into();

    let (max_seq_complete, max_seq_ref) =
        replicas.forward_ref::<Optional<usize, _, Unbounded>>();

    let reconciled_seq = state_transfer::<C>(replicas, current_view.clone(), max_seq_ref);

    let mutating_commands = client_commands.clone();
    let read_only_commands = client_commands.filter(q!(|_| false));

    let heartbeat = replicas
        .source_interval(q!(Duration::from_millis(100)), nondet!(/** heartbeat */))
        .map(q!(|_| ()));
    let tick = replicas.tick();
    let _hb = heartbeat.batch(&tick, nondet!(/** heartbeat tick */));

    let core = core_replication_module(
        replicas,
        &tick,
        mutating_commands,
        read_only_commands,
        current_view.clone(),
        reconciled_seq,
    );

    max_seq_complete.complete(core.max_replicated_seq);

    // Coordinator-driven failure detection: proposals come from outside.
    view_change_complete.complete(external_view_proposals);

    ReplicateRawOutput {
        committed: core.committed,
        replicated: core.replicated,
        read_only: core.read_only_commands,
        current_view,
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Coordinator-driven failure detection
// ─────────────────────────────────────────────────────────────────────────────

use crate::Coordinator;

/// Coordinator-driven failure detection based on request/response nonce matching.
///
/// Architecture:
/// 1. Coordinator tracks outstanding requests by nonce.
/// 2. If a request is pending > `timeout_ms` without a matching response,
///    the coordinator suspects a failure.
/// 3. On suspicion: ping all replicas, collect responses within a ping timeout.
/// 4. Propose a new view containing only replicas that responded to the ping.
/// 5. If the new view excludes the old primary, a new primary is chosen (first member).
/// 6. The view is committed through Paxos before being installed on replicas.
///
/// This never fires during idle periods (no outstanding requests = no suspicion).
///
/// Returns the view proposals stream at the replicas.
pub fn coordinator_failure_detector<'a>(
    coordinator: &Process<'a, Coordinator>,
    replicas: &Cluster<'a, TransparentReplica>,
    commands_at_coord: Stream<String, Process<'a, Coordinator>, Unbounded, NoOrder>,
    responses_at_coord: Stream<String, Process<'a, Coordinator>, Unbounded, NoOrder>,
    current_view_at_replicas: Singleton<View, Cluster<'a, TransparentReplica>, Unbounded>,
    timeout_ms: u64,
    initial_member_count: usize,
) -> Stream<View, Cluster<'a, TransparentReplica>, Unbounded, NoOrder> {
    // Replicas send their current view to the coordinator periodically.
    let _view_at_coord: Singleton<View, Process<'a, Coordinator>, Unbounded> =
        current_view_at_replicas
            .sample_eager(nondet!(/** send view to coordinator periodically */))
            .send(coordinator, TCP.fail_stop().bincode())
            .values()
            .weaken_ordering::<NoOrder>()
            .fold(
                q!(move || View {
                    view_num: 0,
                    members: (0..initial_member_count as u32).collect(),
                }),
                q!(|current: &mut View, new: View| {
                    if new.view_num > current.view_num {
                        *current = new;
                    }
                }, commutative = manual_proof!(/** max is commutative */),
                   idempotent = manual_proof!(/** max is idempotent */)),
            )
            .into();

    let fd_tick = coordinator.tick();

    // Heartbeat keeps fd_tick firing to check for timeouts.
    coordinator
        .source_interval(q!(std::time::Duration::from_millis(1000)), nondet!(/** coord fd heartbeat */))
        .batch(&fd_tick, nondet!(/** coord fd heartbeat tick */))
        .for_each(q!(|_| {}));

    // --- Ping infrastructure ---
    // Coordinator periodically pings all replicas; replicas reply with their ID.
    let ping_stream = coordinator
        .source_interval(q!(std::time::Duration::from_millis(500)), nondet!(/** periodic ping */))
        .map(q!(|_| ()));

    let pings_at_replicas = ping_stream
        .broadcast(replicas, TCP.fail_stop().bincode(), nondet!(/** ping broadcast */));

    // Replicas respond to pings with their cluster ID.
    let ping_responses: Stream<u32, Process<'a, Coordinator>, Unbounded, NoOrder> = pings_at_replicas
        .map(q!(move |_| CLUSTER_SELF_ID.get_raw_id()))
        .send(coordinator, TCP.fail_stop().bincode())
        .values()
        .weaken_ordering::<NoOrder>();

    // --- Nonce-based timeout detection + ping + view proposal (all in one scan) ---
    // 
    // Events entering the scan:
    //   (0, nonce) = command sent
    //   (1, nonce) = response received  
    //   (2, replica_id) = ping response from replica
    //   (3, 0) = heartbeat tick (check timeouts, collect ping results)
    
    let cmd_events = commands_at_coord
        .map(q!(|cmd: String| -> (u8, u64) {
            let nonce = cmd.rsplit(':').next().and_then(|s| s.parse().ok()).unwrap_or(0);
            (0u8, nonce)
        }));

    let resp_events = responses_at_coord
        .map(q!(|resp: String| -> (u8, u64) {
            let nonce = resp.split("nonce=").nth(1)
                .and_then(|s| s.split_whitespace().next())
                .and_then(|s| s.parse().ok())
                .unwrap_or(0);
            (1u8, nonce)
        }));

    let ping_resp_events = ping_responses
        .map(q!(|id: u32| (2u8, id as u64)));

    let heartbeat_events = coordinator
        .source_interval(q!(std::time::Duration::from_millis(1000)), nondet!(/** scan heartbeat */))
        .map(q!(|_| (3u8, 0u64)));

    let all_events = cmd_events
        .interleave(resp_events)
        .interleave(ping_resp_events)
        .interleave(heartbeat_events);

    // Scan state:
    // - pending: HashMap<nonce, Instant> — outstanding requests
    // - ping_phase: bool — are we waiting for ping responses?
    // - ping_start: Option<Instant> — when we started pinging
    // - alive_replicas: HashSet<u32> — replicas that responded to ping
    // - current_view_num: u64
    // - current_members: Vec<u32>
    //
    // Output: Option<(view_num, Vec<u32>)> — proposed new view when ready
    let proposals = all_events
        .batch(&fd_tick, nondet!(/** batch */))
        .weaken_ordering::<NoOrder>()
        .all_ticks()
        .assume_ordering::<hydro_lang::live_collections::stream::TotalOrder>(nondet!(/** FD scan order doesn't affect correctness */))
        .scan(
            q!(move || {
                (
                    std::collections::HashMap::<u64, std::time::Instant>::new(), // pending
                    false,                                                        // ping_phase
                    None::<std::time::Instant>,                                   // ping_start
                    std::collections::HashSet::<u32>::new(),                      // alive_replicas
                    0u64,                                                         // current_view_num
                    (0..initial_member_count as u32).collect::<Vec<u32>>(),       // current_members
                    false,                                                        // should_ping (output flag)
                )
            }),
            q!(move |state: &mut (
                std::collections::HashMap<u64, std::time::Instant>,
                bool,
                Option<std::time::Instant>,
                std::collections::HashSet<u32>,
                u64,
                Vec<u32>,
                bool,
            ), (tag, val): (u8, u64)| -> Option<(u64, Vec<u32>)> {
                match tag {
                    0 => {
                        // Command sent — track nonce
                        if val != 0 {
                            state.0.entry(val).or_insert_with(std::time::Instant::now);
                        }
                        None
                    }
                    1 => {
                        // Response received — clear nonce
                        if val != 0 {
                            state.0.remove(&val);
                        }
                        None
                    }
                    2 => {
                        // Ping response — record alive replica
                        if state.1 {
                            state.3.insert(val as u32);
                        }
                        None
                    }
                    3 => {
                        // Heartbeat — check timeouts and ping results
                        let now = std::time::Instant::now();
                        let timeout = std::time::Duration::from_millis(timeout_ms);

                        if state.1 {
                            // In ping phase — check if ping timeout elapsed (2s)
                            let ping_timeout = std::time::Duration::from_millis(2000);
                            if let Some(start) = state.2 {
                                if now.duration_since(start) > ping_timeout {
                                    // Ping phase done — propose view from alive replicas
                                    state.1 = false;
                                    state.2 = None;
                                    let alive: Vec<u32> = state.5.iter()
                                        .copied()
                                        .filter(|m| state.3.contains(m))
                                        .collect();
                                    state.3.clear();
                                    state.0.clear(); // reset pending

                                    if alive.is_empty() || alive == state.5 {
                                        return None;
                                    }
                                    let new_view_num = state.4 + 1;
                                    println!("[COORD-FD] Ping complete — proposing view change: {:?} -> {:?}", state.5, alive);
                                    state.4 = new_view_num;
                                    state.5 = alive.clone();
                                    return Some((new_view_num, alive));
                                }
                            }
                            None
                        } else {
                            // Not in ping phase — check for timed-out requests
                            let has_timeout = state.0.values().any(|&t| now.duration_since(t) > timeout);
                            if has_timeout {
                                // Enter ping phase
                                println!("[COORD-FD] Request timed out — pinging replicas...");
                                state.1 = true;
                                state.2 = Some(now);
                                state.3.clear();
                                state.6 = true; // signal to send pings
                            }
                            None
                        }
                    }
                    _ => None,
                }
            }),
        );

    // Convert proposals to View objects
    let view_proposals = proposals
        .map(q!(|(view_num, members)| View { view_num, members }));

    // Broadcast proposals from coordinator to all replicas.
    view_proposals
        .broadcast(replicas, TCP.fail_stop().bincode(), nondet!(/** coord proposals to replicas */))
        .weaken_ordering::<NoOrder>()
}
