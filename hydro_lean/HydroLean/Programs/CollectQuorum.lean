import HydroLean.Hydro.Stream

/-!
# Port of `hydro_std::quorum::collect_quorum` (Rust: `hydro_std/src/quorum.rs`)

`collect_quorum(responses, min, max)` consumes a stream of keyed responses
`(K, Result<(), E>)` — e.g. votes from a cluster of `max` acceptors — and
emits each key that accumulates at least `min` successful responses, exactly
once. In Rust it is written as a `sliced!` (tick) block:

```rust
let new_inputs = use::batch(responses, nondet!(
    /// We always persist values that have not reached quorum, so even
    /// with arbitrary batching we always produce deterministic quorum results.
));
let mut not_all = use::state_null();
let mut min_but_not_max = use::state_null();
let current_responses = not_all.chain(new_inputs);
let count_per_key = current_responses.into_keyed().fold((0,0), count ok/err);
...
```

The Lean port mirrors that structure: the looped `state_null` variables become
the `QuorumState` fields, one tick becomes `collectQuorumTick.step`, and the
`nondet!` batching guard becomes an explicit `Batching` parameter — the
justification quoted above becomes a *theorem* (`CollectQuorumCorrect`,
proved in `Programs/CollectQuorumProof.lean` in a later wave): the `all_ticks`
output is the same for **every** batching of the input.

The Rust `manual_proof!(/** increment counters is commutative */)` obligation
is discharged by construction: the per-key aggregation is `countKeyP`, and
`Stream.countKeyP_perm` proves its order-invariance.
-/

namespace HydroLean.Programs

open HydroLean.Hydro

universe u v

variable {κ : Type u} {E : Type v}

/-- The looped state of the `sliced!` block: the Rust `use::state_null`
variables. `notAll` holds responses for keys that have not yet received all
`max` responses (Rust: `not_all`); `minButNotMax` holds keys already emitted
(reached `min` successes) but still awaiting their remaining responses
(Rust: `min_but_not_max`; unused in the `min == max` branch). -/
structure QuorumState (κ : Type u) (E : Type v) where
  notAll : Stream (κ × Except E Unit)
  minButNotMax : Stream κ
deriving Repr

/-- One tick of `collect_quorum` (the body of the Rust `sliced!` block),
structurally mirroring the Rust code; intermediate values carry the Rust
names. `Except E Unit` is Rust's `Result<(), E>` (`.ok ()` = `Ok(())`). -/
def collectQuorumTick [DecidableEq κ] (min max : Nat) :
    TickLoop (Stream (κ × Except E Unit)) (QuorumState κ E) (Stream κ) where
  init := ⟨[], []⟩
  step s newInputs :=
    -- let current_responses = not_all.chain(new_inputs);
    let currentResponses := s.notAll.chain newInputs
    -- let count_per_key = current_responses.into_keyed().fold((0,0), ok/err)
    let successCount := fun k => currentResponses.countKeyP k Except.isOk
    let totalCount := fun k => currentResponses.countKey k
    -- let reached_min_count = count_per_key.filter(success >= min).keys()
    let reachedMinCount :=
      currentResponses.keys.filter (fun k => decide (min ≤ successCount k))
    if min = max then
      -- not_all = current_responses.anti_join(reached_min_count);
      -- out     = reached_min_count
      (⟨currentResponses.antiJoin reachedMinCount, []⟩, reachedMinCount)
    else
      -- let received_from_all = count_per_key.filter(success+error >= max).keys();
      let receivedFromAll :=
        currentResponses.keys.filter (fun k => decide (max ≤ totalCount k))
      -- not_all = current_responses.anti_join(received_from_all);
      -- out = reached_min_count.filter_not_in(min_but_not_max);
      -- min_but_not_max = reached_min_count.filter_not_in(received_from_all);
      (⟨currentResponses.antiJoin receivedFromAll,
        reachedMinCount.filterNotIn receivedFromAll⟩,
       reachedMinCount.filterNotIn s.minButNotMax)

/-! ## Specification (proved in a later wave)

Usage contract (matching quorum-of-`max`-participants deployments, where each
of `max` cluster members responds at most once per key — e.g. Paxos acceptors):
no key receives more than `max` responses. -/

/-- Each key appears at most `max` times in the input. -/
def AtMostMaxResponses [DecidableEq κ] (max : Nat)
    (input : Stream (κ × Except E Unit)) : Prop :=
  ∀ k, input.countKey k ≤ max

/-- Functional specification of the success output: each key with at least
`min` successful responses is emitted **exactly once** (and nothing else is
emitted). Note this is batching-independent: it mentions only the input. -/
def QuorumSpec [DecidableEq κ] (min : Nat)
    (input : Stream (κ × Except E Unit)) (emitted : Stream κ) : Prop :=
  emitted.Nodup ∧
    ∀ k, k ∈ emitted ↔
      (k ∈ input.map Prod.fst ∧ min ≤ input.countKeyP k Except.isOk)

/-- **Unbounded correctness of `collect_quorum`** (statement): for all key and
error types, all `1 ≤ min ≤ max`, all inputs satisfying the response contract,
and **all batchings** of the input, the emitted keys satisfy `QuorumSpec`.

Two corollaries justify the Rust `nondet!` comment: (1) determinism — any two
batchings yield outputs that are permutations of each other (both are nodup
enumerations of the same set); (2) the Rust simulator's exhaustive tests are
instances of this statement at fixed inputs. -/
def CollectQuorumCorrect : Prop :=
  ∀ (κ : Type) (E : Type) (_ : DecidableEq κ) (min max : Nat),
    1 ≤ min → min ≤ max →
    ∀ (input : Stream (κ × Except E Unit)) (b : Batching (κ × Except E Unit)),
      b.of input →
      AtMostMaxResponses max input →
      QuorumSpec min input ((collectQuorumTick (κ := κ) (E := E) min max).allTicksOutput b)

/-! ## Executable smoke tests

These reproduce the Rust unit tests from `hydro_std/src/quorum.rs`
(`collect_quorum_functionality`, `collect_quorum_min_equals_max`,
`collect_quorum_single_response`, `collect_quorum_no_double_quorum_before_max`)
and additionally check batching-invariance on concrete instances (what the
Rust simulator's `exhaustive` does for bounded schedules). `#guard` fails the
build on regression. -/

private def ok : Except Unit Unit := .ok ()
private def err : Except Unit Unit := .error ()

private def runQ (min max : Nat) (input : Stream (Nat × Except Unit Unit))
    (strat : Stream (Nat × Except Unit Unit) → Batching (Nat × Except Unit Unit)) :
    List Nat :=
  (collectQuorumTick min max).allTicksOutput (strat input)

-- Test case 1 (Rust: functionality/1): exact minimum quorum (2/3).
#guard runQ 2 3 [(1, ok), (1, ok)] .whole = [1]
#guard runQ 2 3 [(1, ok), (1, ok)] .singletons = [1]
-- Test case 2: mixed results reach quorum (2 success, 1 error).
#guard runQ 2 3 [(2, ok), (2, ok), (2, err)] .whole = [2]
#guard runQ 2 3 [(2, ok), (2, ok), (2, err)] .singletons = [2]
-- Test case 3: no quorum (1 success, 2 errors).
#guard runQ 2 3 [(3, ok), (3, err), (3, err)] .whole = []
#guard runQ 2 3 [(3, ok), (3, err), (3, err)] .singletons = []
-- Test case 4: extra responses after quorum are ignored.
#guard runQ 2 3 [(4, ok), (4, ok), (4, ok)] .whole = [4]
#guard runQ 2 3 [(4, ok), (4, ok), (4, ok)] .singletons = [4]
-- Test case 5: only errors.
#guard runQ 2 3 [(5, err), (5, err), (5, err)] .whole = []
-- Test case 6: quorum exactly at max (err, ok, ok).
#guard runQ 2 3 [(6, err), (6, ok), (6, ok)] .whole = [6]
#guard runQ 2 3 [(6, err), (6, ok), (6, ok)] .singletons = [6]
-- Rust: collect_quorum_min_equals_max (2,2).
#guard runQ 2 2 [(1, ok), (1, ok), (2, ok), (2, err), (3, ok), (3, ok)] .whole = [1, 3]
#guard runQ 2 2 [(1, ok), (1, ok), (2, ok), (2, err), (3, ok), (3, ok)] .singletons = [1, 3]
-- Rust: collect_quorum_single_response (1,1).
#guard runQ 1 1 [(1, ok), (2, err), (3, ok)] .whole = [1, 3]
#guard runQ 1 1 [(1, ok), (2, err), (3, ok)] .singletons = [1, 3]
-- Rust: collect_quorum_no_double_quorum_before_max (2,4).
#guard runQ 2 4
  [(1, ok), (1, ok), (1, ok), (1, ok), (2, err), (2, ok), (2, ok), (2, err)]
  .whole = [1, 2]
#guard runQ 2 4
  [(1, ok), (1, ok), (1, ok), (1, ok), (2, err), (2, ok), (2, ok), (2, err)]
  .singletons = [1, 2]
-- Batching invariance on an interleaved-keys instance. NOTE: the Rust output
-- is `Stream<K, NoOrder>` — and this is visible here: different batchings emit
-- the same *set* of keys in different *orders* (e.g. key 2 can reach quorum
-- before key 1 under fine batching). The batching-invariant denotation is the
-- multiset; we compare canonically sorted outputs, which is what the `NoOrder`
-- quotient makes equal (wave 2 wires in the `Multiset` carrier).
private def sortedQ (o : List Nat) : List Nat := o.mergeSort (· ≤ ·)
private def mixed : Stream (Nat × Except Unit Unit) :=
  [(1, ok), (2, ok), (1, err), (2, ok), (1, ok), (3, err), (1, ok), (3, ok)]
#guard sortedQ (runQ 2 4 mixed .whole) = sortedQ (runQ 2 4 mixed .singletons)
#guard sortedQ (runQ 2 4 mixed (fun _ =>
      [[(1, ok), (2, ok), (1, err)], [], [(2, ok), (1, ok), (3, err)], [(1, ok), (3, ok)]]))
    = sortedQ (runQ 2 4 mixed .whole)
-- Error stream (Rust: functionality/2 expects [(2, ())]) — the error side is
-- a pure `filterMap` of the raw input (quorum.rs:82–85; the module face is
-- `collect_quorumM`'s second component, `CollectQuorumStreams.lean`).
#guard (([(2, ok), (2, ok), (2, err)] : Stream (Nat × Except Unit Unit)).filterMap
    fun (k, r) => match r with
      | .error e => some (k, e)
      | .ok _ => none) = [(2, ())]

end HydroLean.Programs
