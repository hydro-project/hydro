# Recovery Guarantees for Cluster Broadcast

## The Problem

Hydro's coordination analysis proves that a Cluster's observable outputs
are future-monotone by verifying that (a) the per-member dataflow
preserves monotonicity (backward walk), and (b) the upstream data
arriving via broadcast is itself monotone (forward propagation).

The proof of propagation Rule (i) — "Cluster with monotone upstream" —
assumes that **all cluster members receive the same input stream**.
This holds for static membership (all members present from the start),
but breaks with dynamic membership: a member that joins at time *t*
has missed all messages before *t*.

The journal-backed pattern in MicroDB solves this via replay: a new
member replays the journal from the beginning (or restores from a
checkpoint and replays the delta), so it ends up with the same
effective input as a member present from time 0.  But the analysis
has no way to see this — it only sees the broadcast.

This document sketches a design for expressing recovery guarantees
in Hydro's type system, so the coordination analysis can distinguish
"broadcast with recovery" (SEQ-eligible) from "broadcast without
recovery" (conservative).

## The Contract

A recovery mechanism must guarantee **input-prefix equivalence**:

> For any member that joins (or restarts) at time *t*, the combination
> of restore + live traffic produces an effective input stream that is
> prefix-equivalent to what a member present from time 0 would see,
> with the same `Ordering` and at-least-once delivery guarantees.

"Prefix-equivalent" means: the effective stream is a prefix of the
same infinite sequence.  A member at time *t* may be behind (shorter
prefix), but it never disagrees with a member at time *t'* > *t* on
the elements they share.

This is exactly the property that Rule (i) needs: if every member's
effective input is a prefix of the same sequence, then the SPMD
dataflow produces prefixes of the same output sequence, giving SEQ.

## Two Reference Implementations

### 1. Log-Based Recovery (MicroDB Journal)

The journal is a TotalOrder stream of `JournalRecord` values,
delivered to all cluster members via broadcast from a Paxos-ordered
log.  The journal contains two kinds of records:

- **`Committed(ChangeList)`**: a write transaction's effects.
- **`CheckpointPart { entries, target_time, target_seqno }`**:
  a snapshot of the KV state at a point in the journal.

A new member receives the journal from the beginning.  The scan
in `journal_backed_service` processes records in order:

```
journal_stream.batch(&tick).across_ticks(|s| {
    s.scan(|| KvState::new(), |state, record| {
        process_journal_record(state, record)
    })
})
```

Checkpoint records are an optimization: instead of replaying every
committed record from slot 0, the journal can start with a checkpoint
(state at slot *k*) followed by committed records from slot *k+1*
onward.  The scan handles both — `CheckpointPart` bulk-loads state,
`Committed` applies incremental changes.  The result is the same
`KvState` either way.

**Why this satisfies the contract**: The journal is a TotalOrder
stream.  Every member (including late joiners) processes the same
sequence of records from the same starting point.  Checkpoints don't
change the effective input — they're a compressed representation of
a prefix.  The scan is deterministic, so every member produces the
same output sequence.

**What the analysis needs to know**: The broadcast of `journal_stream`
to the Cluster has a recovery mechanism that guarantees input-prefix
equivalence.  The mechanism is: the journal infrastructure delivers
the complete log (possibly starting from a checkpoint) to every
member, including late joiners.

### 2. State-Transfer Recovery

An alternative to log replay: a new member receives a snapshot of
the current state from a live member, then processes live traffic
from that point forward.

In MicroDB terms, this would look like:

```
// State transfer: new member receives KvState from a live member
let initial_state = live_member_state
    .send_to_new_member(&cluster, TCP.fail_stop().bincode());

// Then processes live journal records from the transfer point
let journal_from_transfer = journal_stream
    .filter(|r| r.seqno > transfer_point);

journal_from_transfer.batch(&tick).across_ticks(|s| {
    s.scan(|| initial_state, |state, record| {
        process_journal_record(state, record)
    })
})
```

**Why this satisfies the contract**: The transferred state is the
result of processing journal records 0..*k*.  Live traffic starts
from record *k+1*.  The effective input is the same sequence as a
member that processed all records from 0 — just with the prefix
0..*k* compressed into a state snapshot.

**What the analysis needs to know**: Same as log-based — the
combination of state transfer + live traffic produces an effective
input that is prefix-equivalent to the full stream.

## Design Direction 1: `manual_proof!` Annotation

The simplest approach: let the user assert the recovery guarantee
on the broadcast, analogous to how `manual_proof!` works for
commutativity.

```rust
let journal = paxos_log
    .broadcast(&cluster, TCP.fail_stop().bincode())
    .with_recovery(manual_proof!(/**
        Journal replay from slot 0 (or checkpoint + delta)
        ensures every member sees the complete TotalOrder
        input prefix before processing live traffic.
    */));
```

### What changes in the analysis

The `Network` node in the IR would carry a `has_recovery: bool` flag
(or a richer `RecoveryProof` type).  The coordination analysis would
check this flag:

- **`has_recovery = true`**: Treat as complete delivery.  The backward
  walk discharges at the broadcast receive (as today).  The channel
  label reflects the sending side's proof.  Rule (i) applies: Cluster
  with monotone upstream → local label.  SEQ is achievable.

- **`has_recovery = false`** (default): Be conservative.  The backward
  walk still discharges (per-member monotonicity holds — each member's
  *effective* input is monotone, just possibly incomplete).  But the
  channel propagation would cap the label:
  - **SetInclusion**: safe.  A late joiner sees a subset that grows.
    The eventual set is the same.  CONV is achievable.
  - **Prefix**: unsafe without recovery.  A late joiner misses the
    prefix.  Cap at CONV (or SELF if the downstream has per-member
    nondeterminism).

This is a small change: the `propagate_forward` function already
distinguishes `label` from `label_fixed_membership`.  The recovery
flag would determine which label to use, replacing the current
heuristic (which floors at SELF when the sending-side proof fails).

### What the user asserts

The `manual_proof!` asserts:

1. **Completeness**: every member (including late joiners) eventually
   receives the complete input prefix from the beginning.
2. **Ordering**: the effective input has the same `Ordering` as the
   original stream (TotalOrder if the original is TotalOrder).
3. **Idempotency tolerance**: the recovery mechanism may deliver
   duplicates (e.g., overlapping checkpoint + log replay).  The
   downstream must handle this — either via idempotent processing
   or explicit dedup.  (This is already in Hydro's type system via
   the `Retries` parameter.)

### Pros and cons

**Pro**: Minimal implementation effort.  One new flag on `Network`,
one check in `propagate_forward`, one new method on the broadcast
builder.

**Con**: The user can assert recovery incorrectly, invalidating the
guarantee.  Same trust model as `manual_proof!` for commutativity.

## Design Direction 2: Structured Recovery Combinator

A more ambitious approach: provide a `RecoverableStream` combinator
that structurally guarantees input-prefix equivalence, so the
analysis can verify the recovery guarantee without `manual_proof!`.

### The idea

A `RecoverableStream<T>` wraps a broadcast stream with a recovery
protocol.  The combinator takes:

1. **The live stream**: the broadcast from upstream.
2. **A restore source**: a stream or singleton that provides the
   initial state for late joiners (checkpoint, state transfer, etc.).
3. **A merge function**: how to combine restored state with live
   traffic to produce the effective input.

The combinator guarantees that the effective input is prefix-equivalent
to the live stream, regardless of when the member joins.

### Sketch for log-based recovery

```rust
// The journal stream, broadcast to the cluster
let journal_broadcast = paxos_log
    .broadcast(&cluster, TCP.fail_stop().bincode());

// Wrap with log-based recovery: the journal infrastructure
// delivers the complete log to late joiners
let journal = journal_broadcast.with_log_recovery(
    // How to obtain the log prefix for a late joiner:
    // the journal service delivers from slot 0 (or checkpoint)
    restore_from: journal_service.replay_from_start(),
    // The merge: replay prefix, then switch to live
    // (the journal service handles the cutover)
);
```

### Sketch for state-transfer recovery

```rust
let journal_broadcast = paxos_log
    .broadcast(&cluster, TCP.fail_stop().bincode());

let journal = journal_broadcast.with_state_transfer_recovery(
    // How to obtain state for a late joiner:
    state_source: live_member.current_state(),
    // How to derive the effective input from state + live:
    // apply live records starting from the state's seqno
    merge: |state, live_stream| {
        live_stream
            .filter(|r| r.seqno > state.current_seqno)
            .scan(state, process_journal_record)
    },
);
```

### What the analysis would verify

The combinator's type signature encodes the guarantee:

```rust
impl<T, L: Location, O: Ordering> RecoverableStream<T, L, O> {
    /// The effective stream has the same Ordering as the original.
    /// Input-prefix equivalence is guaranteed by construction.
    fn into_stream(self) -> Stream<T, L, Unbounded, O> { ... }
}
```

The analysis sees `RecoverableStream` as a source with verified
recovery — no `manual_proof!` needed.  The `Network` node would
carry `has_recovery = true` automatically.

### What makes this hard

The combinator needs to express the relationship between the
restore source and the live stream — specifically, that the restore
source provides a prefix of the same sequence.  This is a semantic
property that depends on the recovery infrastructure (journal
service, state transfer protocol, etc.).

For log-based recovery, the guarantee comes from the journal
service: it delivers records in order from slot 0.  The combinator
can verify this structurally if the journal service exposes a
typed API:

```rust
trait JournalService<T> {
    /// Returns a stream that replays the complete journal from the
    /// beginning, then seamlessly transitions to live records.
    /// The returned stream is TotalOrder and prefix-equivalent to
    /// the live broadcast.
    fn replay_from_start(&self) -> Stream<T, Cluster, Unbounded, TotalOrder>;
}
```

For state-transfer recovery, the guarantee is harder to verify
structurally.  The combinator would need to know that:
- The transferred state is a deterministic function of some prefix
  of the journal.
- The live stream starts from exactly the next record after that
  prefix.
- No records are lost or duplicated in the gap.

This is essentially a correctness proof for the state-transfer
protocol — hard to encode in types alone.  A `manual_proof!` on
the merge function may be unavoidable for this case.

### Pros and cons

**Pro**: Common patterns (log replay) get structural verification.
No trust required for the happy path.

**Con**: Significant API design effort.  State-transfer recovery
still needs `manual_proof!` for the merge correctness.  The
combinator API may be over-engineered for the current use cases.

## Recommendation

Start with Direction 1 (`manual_proof!` annotation).  It's minimal,
it unblocks the analysis for MicroDB today, and it establishes the
right contract (input-prefix equivalence) that Direction 2 can later
verify structurally.

The implementation plan:

1. Add `has_recovery: bool` to the `Network` IR node (or a
   `RecoveryProof` enum: `None`, `ManualProof(String)`).
2. Add `.with_recovery(manual_proof!(...))` to the broadcast builder
   API.
3. In `propagate_forward`, use the recovery flag to determine whether
   a broadcast-to-Cluster edge gets the full label (SEQ-eligible) or
   the conservative label (CONV cap).
4. Update COORDINATION.md to document the recovery assumption.
5. Update the MicroDB coordination test to use `.with_recovery(...)`.

Direction 2 can be pursued later as a library of recovery combinators
that produce `has_recovery = true` without `manual_proof!`.  The
`JournalService` trait is the natural starting point — it captures
the log-replay pattern that MicroDB already uses.
