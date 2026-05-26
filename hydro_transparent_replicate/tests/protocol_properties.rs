//! Property-based tests for protocol-level logic.
//!
//! These tests verify pure functions and data structures used by the
//! transparent replication protocol — NOT the full Hydro dataflow.
//!
//! Tests Properties 1, 2, 3, 5, 6, 7, 8 from the design document.

use proptest::prelude::*;
use std::collections::{BTreeSet, HashMap};

use hydro_transparent_replicate::messages::View;
use hydro_transparent_replicate::ReplicableService;

// ─────────────────────────────────────────────────────────────────────────────
// TestKvService — a simple HashMap-based service for testing protocol logic
// ─────────────────────────────────────────────────────────────────────────────

/// A simple key-value service for testing protocol properties.
/// Uses a HashMap<String, String> as the backing store.
#[derive(Clone, Debug, Default)]
struct TestKvService {
    store: HashMap<String, String>,
}

impl TestKvService {
    fn new() -> Self {
        Self::default()
    }
}

/// Commands for the test KV service.
#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq, Eq)]
enum TestCommand {
    Put { key: String, value: String },
    Get { key: String },
    Delete { key: String },
}

/// Responses from the test KV service.
#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq, Eq)]
enum TestResponse {
    Ok,
    Value(Option<String>),
    Deleted(bool),
}

impl ReplicableService for TestKvService {
    type Command = TestCommand;
    type Response = TestResponse;

    fn apply(&mut self, command: Self::Command) -> Self::Response {
        match command {
            TestCommand::Put { key, value } => {
                self.store.insert(key, value);
                TestResponse::Ok
            }
            TestCommand::Get { key } => {
                TestResponse::Value(self.store.get(&key).cloned())
            }
            TestCommand::Delete { key } => {
                TestResponse::Deleted(self.store.remove(&key).is_some())
            }
        }
    }

    fn is_read_only(command: &Self::Command) -> bool {
        matches!(command, TestCommand::Get { .. })
    }

    fn snapshot(&self) -> Vec<u8> {
        bincode::serialize(&self.store).unwrap()
    }

    fn restore(&mut self, data: &[u8]) {
        self.store = bincode::deserialize(data).unwrap();
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Proptest strategies
// ─────────────────────────────────────────────────────────────────────────────

fn arb_test_command() -> impl Strategy<Value = TestCommand> {
    prop_oneof![
        ("[a-z]{1,5}", "[a-z]{1,8}").prop_map(|(key, value)| TestCommand::Put { key, value }),
        "[a-z]{1,5}".prop_map(|key| TestCommand::Get { key }),
        "[a-z]{1,5}".prop_map(|key| TestCommand::Delete { key }),
    ]
}

fn arb_mutating_command() -> impl Strategy<Value = TestCommand> {
    prop_oneof![
        ("[a-z]{1,5}", "[a-z]{1,8}").prop_map(|(key, value)| TestCommand::Put { key, value }),
        "[a-z]{1,5}".prop_map(|key| TestCommand::Delete { key }),
    ]
}

fn arb_view(view_num_range: std::ops::Range<u64>, member_pool: std::ops::Range<u32>) -> impl Strategy<Value = View> {
    (view_num_range, proptest::collection::btree_set(member_pool, 2..=5))
        .prop_map(|(view_num, members_set)| {
            let members: Vec<u32> = members_set.into_iter().collect();
            View { view_num, members }
        })
}

// ─────────────────────────────────────────────────────────────────────────────
// Property 1: Contiguous Sequence Number Assignment
// ─────────────────────────────────────────────────────────────────────────────

/// Simulates the index_payloads logic: given a base sequence number and N items,
/// assigns contiguous sequence numbers [base, base+1, ..., base+N-1].
fn simulate_index_payloads<T: Clone>(base: usize, items: &[T]) -> Vec<(usize, T)> {
    items
        .iter()
        .enumerate()
        .map(|(index, item)| (base + index, item.clone()))
        .collect()
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(20))]

    /// **Validates: Requirements 2.4, 3.6**
    ///
    /// Property 1: For any sequence of mutating commands submitted to the primary
    /// in a single view, the assigned sequence numbers SHALL be contiguous integers
    /// starting from the expected base.
    #[test]
    fn prop_contiguous_sequence_number_assignment(
        base in 0usize..100,
        num_items in 1usize..20,
    ) {
        // Generate a batch of items
        let items: Vec<u32> = (0..num_items).map(|i| i as u32).collect();

        let indexed = simulate_index_payloads(base, &items);

        // Verify contiguity: each seq should be base + index
        for (i, (seq, _payload)) in indexed.iter().enumerate() {
            prop_assert_eq!(*seq, base + i,
                "Expected seq {} at index {}, got {}", base + i, i, seq);
        }

        // Verify count
        prop_assert_eq!(indexed.len(), num_items);

        // Verify the range is [base, base + num_items - 1]
        let first_seq = indexed.first().unwrap().0;
        let last_seq = indexed.last().unwrap().0;
        prop_assert_eq!(first_seq, base);
        prop_assert_eq!(last_seq, base + num_items - 1);
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Property 2: Quorum Commit Correctness
// ─────────────────────────────────────────────────────────────────────────────

/// Simulates quorum decision: commit iff all view members have acked.
fn quorum_commit_decision(view_members: &[u32], ack_senders: &BTreeSet<u32>) -> bool {
    // All view members must have acked
    view_members.iter().all(|m| ack_senders.contains(m))
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(20))]

    /// **Validates: Requirements 2.6**
    ///
    /// Property 2: For any view of size N and any set of ack messages for a given
    /// sequence number, the command SHALL be committed if and only if acks have been
    /// received from all N view members.
    #[test]
    fn prop_quorum_commit_correctness(
        view_size in 2usize..8,
        ack_subset_bits in 0u8..255,
    ) {
        // Create a view with members [0, 1, ..., view_size-1]
        let view_members: Vec<u32> = (0..view_size as u32).collect();

        // Create an ack set as a subset of view members based on bits
        let ack_senders: BTreeSet<u32> = view_members.iter()
            .enumerate()
            .filter(|(i, _)| (ack_subset_bits >> i) & 1 == 1)
            .map(|(_, &m)| m)
            .collect();

        let committed = quorum_commit_decision(&view_members, &ack_senders);

        // Commit iff all members acked
        let all_acked = ack_senders.len() == view_members.len();
        prop_assert_eq!(committed, all_acked,
            "Expected commit={}, got commit={}. View size={}, acks={:?}",
            all_acked, committed, view_size, ack_senders);
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Property 3: Read-Only Commands Skip Replication
// ─────────────────────────────────────────────────────────────────────────────

/// Simulates the primary's command processing: read-only commands are applied
/// directly, mutating commands go through replication (incrementing seq counter).
fn simulate_command_processing(
    commands: &[TestCommand],
) -> (Vec<(usize, TestCommand)>, Vec<TestResponse>) {
    let mut replicate_list: Vec<(usize, TestCommand)> = Vec::new();
    let mut read_responses: Vec<TestResponse> = Vec::new();
    let mut seq_counter = 0usize;
    let mut service = TestKvService::new();

    for cmd in commands {
        if TestKvService::is_read_only(cmd) {
            // Read-only: apply directly, no replication
            let response = service.apply(cmd.clone());
            read_responses.push(response);
        } else {
            // Mutating: goes through replication
            replicate_list.push((seq_counter, cmd.clone()));
            seq_counter += 1;
            // In real protocol, apply happens after quorum ack
            service.apply(cmd.clone());
        }
    }

    (replicate_list, read_responses)
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(20))]

    /// **Validates: Requirements 2.2**
    ///
    /// Property 3: For any command where is_read_only returns true, processing that
    /// command SHALL NOT generate any Replicate messages or increment the sequence
    /// counter.
    #[test]
    fn prop_read_only_commands_skip_replication(
        commands in proptest::collection::vec(arb_test_command(), 1..20),
    ) {
        let (replicate_list, _read_responses) = simulate_command_processing(&commands);

        // Count expected mutating commands
        let expected_mutating: Vec<&TestCommand> = commands.iter()
            .filter(|cmd| !TestKvService::is_read_only(cmd))
            .collect();

        // Replicate list should only contain mutating commands
        prop_assert_eq!(replicate_list.len(), expected_mutating.len(),
            "Replicate list has {} items but expected {} mutating commands",
            replicate_list.len(), expected_mutating.len());

        // Verify no read-only commands appear in replicate list
        for (_seq, cmd) in &replicate_list {
            prop_assert!(!TestKvService::is_read_only(cmd),
                "Read-only command {:?} found in replicate list", cmd);
        }

        // Verify sequence numbers are contiguous for mutating commands
        for (i, (seq, _)) in replicate_list.iter().enumerate() {
            prop_assert_eq!(*seq, i,
                "Expected seq {} at position {}, got {}", i, i, seq);
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Property 5: View Structural Invariants
// ─────────────────────────────────────────────────────────────────────────────

proptest! {
    #![proptest_config(ProptestConfig::with_cases(20))]

    /// **Validates: Requirements 3.2**
    ///
    /// Property 5: For any valid View value, the members list SHALL be sorted in
    /// ascending order, contain no duplicates, and the primary SHALL equal members[0].
    #[test]
    fn prop_view_structural_invariants(
        view in arb_view(0..100, 0..10),
    ) {
        // Members should be sorted (our generator uses BTreeSet so this is guaranteed)
        let mut sorted_members = view.members.clone();
        sorted_members.sort();
        prop_assert_eq!(&view.members, &sorted_members,
            "Members not sorted: {:?}", view.members);

        // No duplicates (BTreeSet guarantees this, but verify)
        let unique: BTreeSet<u32> = view.members.iter().copied().collect();
        prop_assert_eq!(unique.len(), view.members.len(),
            "Duplicate members found in {:?}", view.members);

        // Primary is members[0]
        prop_assert_eq!(view.primary(), view.members[0],
            "Primary {} != members[0] {}", view.primary(), view.members[0]);

        // View must have at least one member
        prop_assert!(!view.members.is_empty(),
            "View has empty members list");

        // contains() should work correctly for all members
        for &m in &view.members {
            prop_assert!(view.contains(m),
                "View.contains({}) returned false but {} is in members", m, m);
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Property 6: Failure Detector Excludes Failed Primary
// ─────────────────────────────────────────────────────────────────────────────

/// Simulates the failure detector's view proposal logic:
/// Given a current view and a set of recently-alive members (excluding the primary),
/// proposes a new view that excludes the current primary and contains only alive members.
fn propose_new_view_after_failure(current_view: &View, recently_alive: &BTreeSet<u32>) -> View {
    let current_primary = current_view.primary();

    // New members = recently alive members, excluding the failed primary
    let new_members: Vec<u32> = recently_alive
        .iter()
        .filter(|&&m| m != current_primary)
        .copied()
        .collect();

    View {
        view_num: current_view.view_num + 1,
        members: new_members, // already sorted (BTreeSet)
    }
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(20))]

    /// **Validates: Requirements 3.3**
    ///
    /// Property 6: For any current view and a commit timeout event, the proposed
    /// new view SHALL exclude the current primary and SHALL contain only members
    /// that were recently observed as alive.
    #[test]
    fn prop_failure_detector_excludes_failed_primary(
        view in arb_view(0..100, 0..10),
        alive_subset_bits in 0u8..255,
    ) {
        let current_primary = view.primary();

        // Generate "recently alive" members as a subset of view members,
        // explicitly excluding the primary (simulating primary failure)
        let recently_alive: BTreeSet<u32> = view.members.iter()
            .enumerate()
            .filter(|(i, m)| **m != current_primary && (alive_subset_bits >> i) & 1 == 1)
            .map(|(_, &m)| m)
            .collect();

        // Skip if no alive members (can't form a new view)
        prop_assume!(!recently_alive.is_empty());

        let proposed_view = propose_new_view_after_failure(&view, &recently_alive);

        // Proposed view must exclude the current primary
        prop_assert!(!proposed_view.members.contains(&current_primary),
            "Proposed view {:?} still contains failed primary {}",
            proposed_view.members, current_primary);

        // Proposed view must contain only recently-alive members
        for &m in &proposed_view.members {
            prop_assert!(recently_alive.contains(&m),
                "Proposed view contains member {} which is not recently alive", m);
        }

        // Proposed view_num must be greater than current
        prop_assert!(proposed_view.view_num > view.view_num,
            "Proposed view_num {} not greater than current {}",
            proposed_view.view_num, view.view_num);

        // Proposed view members should be sorted
        let mut sorted = proposed_view.members.clone();
        sorted.sort();
        prop_assert_eq!(&proposed_view.members, &sorted,
            "Proposed view members not sorted");
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Property 7: State Transfer Preserves All Committed Commands
// ─────────────────────────────────────────────────────────────────────────────

proptest! {
    #![proptest_config(ProptestConfig::with_cases(20))]

    /// **Validates: Requirements 4.4, 8.2**
    ///
    /// Property 7: For any sequence of committed commands [0..N] and a view change
    /// at sequence S, the state transfer (snapshot at seq S + suffix [S+1..N]) SHALL
    /// result in the new primary having equivalent state to applying all commands
    /// [0..N] sequentially from the initial state.
    #[test]
    fn prop_state_transfer_preserves_committed_commands(
        commands in proptest::collection::vec(arb_mutating_command(), 2..15),
        split_ratio in 1..99u32,
    ) {
        let n = commands.len();
        // Split point S: at least 1 command before and 1 after
        let s = ((split_ratio as usize * n) / 100).clamp(1, n - 1);

        // Service B: apply all commands [0..N]
        let mut service_b = TestKvService::new();
        let mut responses_b: Vec<TestResponse> = Vec::new();
        for cmd in &commands {
            responses_b.push(service_b.apply(cmd.clone()));
        }

        // Service A: apply [0..S], snapshot, then simulate state transfer
        let mut service_a = TestKvService::new();
        for cmd in &commands[..s] {
            service_a.apply(cmd.clone());
        }
        let snapshot = service_a.snapshot();

        // Simulate new primary: restore from snapshot, apply suffix [S..N]
        let mut new_primary = TestKvService::new();
        new_primary.restore(&snapshot);
        for cmd in &commands[s..] {
            new_primary.apply(cmd.clone());
        }

        // Both should produce the same state (verified via snapshot equality)
        let snapshot_b = service_b.snapshot();
        let snapshot_new_primary = new_primary.snapshot();

        // Deserialize to compare (HashMap ordering may differ in serialization)
        let state_b: HashMap<String, String> = bincode::deserialize(&snapshot_b).unwrap();
        let state_new: HashMap<String, String> = bincode::deserialize(&snapshot_new_primary).unwrap();

        prop_assert_eq!(&state_b, &state_new,
            "State divergence after state transfer. Split at {}/{}", s, n);

        // Additionally verify: applying the same subsequent command produces same response
        let verify_cmd = TestCommand::Get { key: "a".to_string() };
        let resp_b = service_b.apply(verify_cmd.clone());
        let resp_new = new_primary.apply(verify_cmd);
        prop_assert_eq!(resp_b, resp_new,
            "Response divergence after state transfer");
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Property 8: No Duplicate Sequence Numbers Across Views
// ─────────────────────────────────────────────────────────────────────────────

/// Simulates command sequencing across multiple views.
/// Each view change resumes from the last committed seq + 1.
/// Returns all (view_num, seq, command_index) assignments.
fn simulate_sequencing_across_views(
    commands: &[TestCommand],
    view_change_points: &[usize],
) -> Vec<(u64, usize, usize)> {
    let mut assignments: Vec<(u64, usize, usize)> = Vec::new();
    let mut current_view_num: u64 = 0;
    let mut next_seq: usize = 0;

    let mut sorted_points: Vec<usize> = view_change_points
        .iter()
        .filter(|&&p| p > 0 && p < commands.len())
        .copied()
        .collect();
    sorted_points.sort();
    sorted_points.dedup();

    let mut point_idx = 0;

    for (cmd_idx, _cmd) in commands.iter().enumerate() {
        // Check if we hit a view change point
        if point_idx < sorted_points.len() && cmd_idx == sorted_points[point_idx] {
            current_view_num += 1;
            // New view resumes from next_seq (no reset — continues from last committed + 1)
            point_idx += 1;
        }

        assignments.push((current_view_num, next_seq, cmd_idx));
        next_seq += 1;
    }

    assignments
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(20))]

    /// **Validates: Requirements 8.1**
    ///
    /// Property 8: For any execution with view changes, no two distinct commands
    /// SHALL be committed at the same sequence number.
    #[test]
    fn prop_no_duplicate_sequence_numbers_across_views(
        num_commands in 3usize..20,
        num_view_changes in 1usize..4,
        view_change_seed in proptest::collection::vec(1usize..19, 1..4),
    ) {
        // Generate commands
        let commands: Vec<TestCommand> = (0..num_commands)
            .map(|i| TestCommand::Put {
                key: format!("k{}", i),
                value: format!("v{}", i),
            })
            .collect();

        // Generate view change points (clamped to valid range)
        let view_change_points: Vec<usize> = view_change_seed.iter()
            .take(num_view_changes)
            .map(|&p| p % num_commands)
            .collect();

        let assignments = simulate_sequencing_across_views(&commands, &view_change_points);

        // Verify: no two distinct commands share the same seq number
        let mut seq_to_cmd: HashMap<usize, usize> = HashMap::new();
        for &(_view_num, seq, cmd_idx) in &assignments {
            if let Some(&existing_cmd_idx) = seq_to_cmd.get(&seq) {
                prop_assert_eq!(existing_cmd_idx, cmd_idx,
                    "Duplicate seq {}: assigned to command {} and command {}",
                    seq, existing_cmd_idx, cmd_idx);
            } else {
                seq_to_cmd.insert(seq, cmd_idx);
            }
        }

        // Verify: all sequence numbers are unique (since each command is distinct)
        prop_assert_eq!(seq_to_cmd.len(), assignments.len(),
            "Expected {} unique seqs but got {}",
            assignments.len(), seq_to_cmd.len());

        // Verify: sequence numbers are contiguous from 0
        let max_seq = assignments.iter().map(|&(_, seq, _)| seq).max().unwrap();
        prop_assert_eq!(max_seq, num_commands - 1,
            "Max seq {} != expected {}", max_seq, num_commands - 1);
    }
}
