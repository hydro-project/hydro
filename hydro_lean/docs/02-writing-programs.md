# Writing Hydro Programs in Lean

> **Status: historical.** This document describes the pre-decisions-as-inputs
> layers (multiset carriers, sealed components, `Holds` simulation runs),
> which were deleted as unused after the (a)/(b) deliverables were migrated
> to the decisions-as-inputs surface (docs/10; typed M-form stages with
> colocated verified faces). Names below refer to deleted code — recover
> from jj history. Kept as the record of the design and of why it was
> superseded.

This document covers the surface layer: how Rust Hydro programs are expressed in
Lean so that unbounded correctness proofs are possible. The guiding rule
(`DESIGN.md`, "Naming policy"): **surface-API names mirror the Rust API verbatim**
(`broadcast_closed`, `fold_commutative`-shaped names, `snapshot_atomic`, ...);
divergence in semantics between same-named things is a bug, not a naming choice.

## Carriers: what a stream *is*, per marker

A live collection's Lean carrier is determined by its Rust type markers
(`Hydro/Markers.lean` documents the mapping; `StreamOrder` = `TotalOrder | NoOrder`,
`Retries` = `ExactlyOnce | AtLeastOnce`, with `weakensTo` mirroring Rust's `Into`
conversions):

| Rust markers | Lean carrier | File |
|---|---|---|
| `TotalOrder + ExactlyOnce` | `Stream α := List α` | `Hydro/Stream.lean` |
| `NoOrder + ExactlyOnce` | `MStream α := Multiset α` (quotient of `List` by `Perm`) | `Hydro/MsetStream.lean`, `Collections/Multiset.lean` |
| `TotalOrder + AtLeastOnce` | `DupList α` (adjacent-dup-free canonical lists; `cat_assoc` proven) | `Collections/DupList.lean` |
| `Singleton`/`Optional` (evolving values) | trajectories, `Traj α β := List α → β` | `Hydro/SingletonTraj.lean` (see below) |

The quotient is the point: a `NoOrder` stream doesn't have an unknown order — it has
*no* order, so schedule-dependent arrival is unobservable by type. Operations that
survive the quotient are exactly the congruent ones: `MStream.foldCommutative`
requires `Multiset.AccComm f` — the Rust
`fold(..., commutative = manual_proof!(...))` promise as a real hypothesis, demanded
by `Quotient.lift` — while `Stream.fold` on `List` has no side condition. Bridging:
`toMStream : Stream α → MStream α` (Rust `weaken_ordering`), with transport lemmas
(`toMStream_countKey`, `toMStream_foldCommutative`, ...).

Stream operators mirror the Rust `Stream` API: `map`, `filter`, `filterMap`,
`flatMap`, `chain`, `enumerate`, `fold`, `reduce`, `crossSingleton`, `antiJoin`,
`filterNotIn`, `keys`, `countKey`, `countKeyP`, `keyedFold` (the `into_keyed().fold`
pattern), `Stream.join` (`Hydro/Join.lean`). Everything is computable — programs run
under `#eval`/`#guard`.

## Ticks: the `sliced!` correspondence

A Rust `sliced!` block is a tick loop (`Hydro/Tick.lean`):

```lean
structure TickLoop (In St Out) where
  init : St                    -- the `use::state_null()` / `use::state(...)` variables
  step : St → In → St × Out    -- one tick: consume a batch, update state, emit
```

with `run`, `runFrom`, `outputs`, `finalState`, and `allTicks` (Rust `all_ticks()`:
concatenation of per-tick outputs). Determinism *given the batch sequence* is free —
`step` is a function. Two structural facts matter constantly:

- `runFrom_append` — running on `bs₁ ++ bs₂` = run `bs₁`, continue with `bs₂`.
- `TickLoop.run_characterize` — the batching-invariance proof device: exhibit
  cumulative-prefix functions (state-of-prefix, output-of-prefix) commuting with one
  step, and the loop's behavior depends only on the *flatten* of the batching.

Atomicity (`yield_atomic` / "state before ack") is *inherent*: one `step` produces
`(state', outputs)` together. `Hydro/Atomic.lean` packages the consequence as
`logThenAck`/`logThenAck_ack` (an ack emitted at tick i is computed from the
post-write state) — the shape the Paxos acceptor uses.

## Nondeterminism as data (`Hydro/NonDet.lean`)

Every Rust `nondet!` guard becomes an explicit, structured value that theorems
universally quantify over:

- `Batching α := List (List α)` with `Batching.of b input : Prop := b.flatten = input`
  — an adversarial split into per-tick batches (Rust `use::batch(...)`). Canonical
  instances: `whole`, `singletons`. Multiset version: `MBatching`
  (`Hydro/MsetNonDet.lean`), with `Batching.toMBatching_of` relating them.
- `Interleaving xs ys m` — an arrival order merging two streams (order-preserving);
  the n-ary version is `Net.IsInterleaving` (below).
- `SnapshotSchedule` — monotone cuts for observing evolving values (below).

The proof discipline (documented in the module): a program with N guard sites is a
Lean function with N extra parameters; unbounded correctness is
`∀ input choices, spec (prog input choices)`. This is the same decision space the
Rust simulator's hooks script — bounded `#guard` tests and unbounded theorems share
specs.

## Networking (`Hydro/Net.lean`): edges are functions, not machinery

Per Gyatso, network channels get **no special treatment** — they are identity or
weakening operators:

- `sendBincode` (process→process TCP): the identity — observationally equivalent to
  no operator (§3.5.1).
- `broadcast_closed n s : ClusterVal n (Stream α)` — static-membership broadcast
  (Rust `Stream::broadcast_closed`; the deployment-fixed case). The type-level
  `Fin n` *is* the closed-membership assumption; see
  `Programs/TwoPCOpenMembership.lean` for what breaks under dynamic
  `Stream::broadcast` (doc 03).
- `demux` (process→cluster by member id): deterministic, per-member FIFO.
- Cluster→process fan-in: `interleave bufs : MStream (Fin n × α)` — the *multiset*
  of tagged messages, a deterministic function. The deferral principle is the lemma
  **`IsInterleaving.ofList_eq`**: every adversarially materialized arrival order `l`
  of the per-member buffers satisfies `Multiset.ofList l = interleave bufs` — the
  quotient forgets exactly the adversarial choice. Corollary
  `foldComm_interleaving_independent`: a commutative fold over a cluster receive is
  interleaving-independent, with no per-program proof.

## Singletons and optionals: trajectories

An unbounded `Singleton` is not a value — it is a **trajectory**
(`Traj α β := List α → β`): its value as a function of the consumed input prefix
(`Traj.ofFold`, `map`, `zip`, `lvar`; `settled` = value on the full input).
Observation is always through a guard:

- Plain `snapshot` (Rust `Singleton::snapshot(tick, nondet!)`): an adversarial
  monotone cut — `observeAt t input (s : SnapshotSchedule) i`, with `observe_sound`
  (prefix-closed properties hold at every observation), `observe_mono`,
  `observe_settles` (eventual determinism of the settled value).
- Atomic reads (`latest_atomic`, `snapshot_atomic`, Rust `use::atomic`): the cut is
  **pinned** to the tick's own batching — `observeAtomic t bs i` with
  `observeAtomic_eq_observeAt` (so all snapshot lemmas transfer) and
  `observeAtomic_reflects_batch` (the "up to date with tick input" guarantee).

Typed wrappers derive invariants from markers instead of manual proofs — `Monotonic`
singletons, `InitNone` optionals, keyed variants — see doc 04. Worked example:
`Programs/SnapshotExamples.lean` proves the dissertation's Fig 4.16 `live_sum_query`
end-to-end (`live_sum_query_sound` et al.).

## Worked example: the `collect_quorum` port

Rust (`hydro_std/src/quorum.rs`, abridged):

```rust
let new_inputs = use::batch(responses, nondet!(
    /// We always persist values that have not reached quorum, so even
    /// with arbitrary batching we always produce deterministic quorum results.
));
let mut not_all = use::state_null();
let mut min_but_not_max = use::state_null();
let current_responses = not_all.chain(new_inputs);
let count_per_key = current_responses.into_keyed().fold((0,0),
    |acc, r| ...,  commutative = manual_proof!(/** increment counters is commutative */));
...
```

Lean (`Programs/CollectQuorum.lean`):

- the `use::state_null` variables → `structure QuorumState := (notAll) (minButNotMax)`;
- the block body → `collectQuorumTick min max : TickLoop (Stream (κ × Except E Unit)) (QuorumState κ E) (Stream κ)`,
  with intermediate values carrying the Rust names, both branches (min = max and
  general);
- the `nondet!` batching guard → an explicit `Batching` parameter of `collectQuorum`;
- the `manual_proof!` commutativity → discharged by `Stream.countKeyP_perm`
  (counting is order-invariant);
- the English justification quoted above → the *theorem* `CollectQuorumCorrect`
  (proved in `CollectQuorumProof.lean`): for **every** input and **every** batching,
  the emitted keys are exactly those with ≥ min successes — under the formalized
  contract `1 ≤ min ≤ max` and `AtMostMaxResponses` (both surfaced by the proof;
  see [FINDINGS.md](../FINDINGS.md) §C);
- the Rust unit tests → `#guard`s on concrete batchings of the same spec.

How that program then gets a *type-derived* deterministic denotation is doc 03; how
it composes into `sequence_payload` and Paxos is docs 04 and 06.
