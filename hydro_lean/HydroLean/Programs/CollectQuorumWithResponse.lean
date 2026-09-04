import HydroLean.Hydro.Tick
import HydroLean.Hydro.Stream

/-!
# Port of `hydro_std::quorum::collect_quorum_with_response` (quorum.rs:7–87)

Like `collect_quorum` (see `Programs/CollectQuorum.lean`) but for
request-response protocols where the *payloads* of successful responses must
be delivered downstream: emits every `(key, value)` success once the key has
accumulated ≥ `min` successes, for keys with at most `max` total responses.
Used by Paxos `p_p1b` (paxos.rs:545–546) to collect the accepted logs carried
by P1b messages.

Structure mirrors the Rust `sliced!` block statement-for-statement; the
`nondet!` batching guard ("we always persist values that have not reached
quorum, so even with arbitrary batching we always produce deterministic quorum
results") becomes the explicit `Batching` universally quantified in
correctness statements. The Rust
`manual_proof!(/** increment counters is commutative */)` is discharged by
`Stream.countKeyP_perm` order-invariance, as in `collect_quorum`.

The full `CollectQuorumCorrect`-style proof is future work (the invariant
machinery of `Programs/CollectQuorumProof.lean` transfers); this file provides
the executable component and its spec (`QuorumWithResponseSpec`).
-/

namespace HydroLean.Programs

open HydroLean.Hydro

universe u v w

variable {κ : Type u} {V : Type v} {E : Type w}

/-- Looped state (`use::state_null` variables, quorum.rs:28–29): `notAll` =
responses whose key has not yet received all `max` responses (or, in the
`min = max` branch, not yet reached quorum); `minButNotMax` = keys already
emitted but still awaiting stragglers. -/
structure QuorumWRState (κ : Type u) (V : Type v) (E : Type w) where
  notAll : Stream (κ × Except E V)
  minButNotMax : Stream κ

/-- One tick of `collect_quorum_with_response` (quorum.rs:22–78), mirroring
the Rust `sliced!` body with the Rust `let` names. -/
def collectQuorumWRTick [DecidableEq κ] (min max : Nat) :
    TickLoop (Stream (κ × Except E V)) (QuorumWRState κ V E)
      (Stream (κ × V)) where
  init := ⟨[], []⟩
  step s newInputs :=
    let currentResponses := s.notAll.chain newInputs
    -- count_per_key = into_keyed().fold((0,0), ok/err) (quorum.rs:33–42)
    let successCount := fun k => currentResponses.countKeyP k Except.isOk
    let totalCount := fun k => currentResponses.countKey k
    -- not_reached_min_count / reached_min_count (quorum.rs:44–52)
    let notReachedMin := fun k => decide (successCount k < min)
    let reachedMinCount :=
      currentResponses.keys.filter (fun k => decide (min ≤ successCount k))
    let emit (out : Stream (κ × Except E V)) : Stream (κ × V) :=
      -- filter_map Ok (quorum.rs:74–77)
      out.filterMap fun (k, r) =>
        match r with
        | .ok v => some (k, v)
        | .error _ => none
    if min = max then
      -- not_all = current.anti_join(reached_min); out = current.anti_join(not_reached_min)
      -- (quorum.rs:54–57)
      (⟨currentResponses.antiJoin reachedMinCount, []⟩,
        emit (currentResponses.filter (fun (k, _) => !notReachedMin k)))
    else
      -- received_from_all (quorum.rs:59–61)
      let receivedFromAll :=
        currentResponses.keys.filter (fun k => decide (max ≤ totalCount k))
      -- not_all = current.anti_join(received_from_all) (quorum.rs:63)
      -- out = current.anti_join(not_reached_min).anti_join(min_but_not_max)
      -- (quorum.rs:65–67)
      -- min_but_not_max = reached_min.filter_not_in(received_from_all)
      -- (quorum.rs:69)
      (⟨currentResponses.antiJoin receivedFromAll,
        reachedMinCount.filterNotIn receivedFromAll⟩,
        emit ((currentResponses.filter (fun (k, _) => !notReachedMin k)).antiJoin
          s.minButNotMax))

/-- Specification. Note a genuine (Lean-port-discovered) subtlety: in the
`min < max` branch the emitted *multiset of values* is batching-dependent —
a straggler success for an already-emitted key is delivered if co-batched
with the quorum-reaching batch but suppressed (by `min_but_not_max`) if it
arrives later. What IS batching-invariant, and what downstream consumers
(Paxos `p_p1b`, which caps at `quorum_size` logs via `fold_early_stop`) rely
on, is: (1) emissions are genuine input successes (no fabrication, no
duplication), and (2) every key reaching `min` successes has **at least
`min`** successes emitted. This spec states exactly that. -/
def QuorumWithResponseSpec [DecidableEq κ] [DecidableEq V]
    (min : Nat)
    (input : Stream (κ × Except E V)) (emitted : Stream (κ × V)) : Prop :=
  (∀ k v, (emitted.filter (fun p => p.1 == k && p.2 == v)).length
      ≤ (input.filter (fun p => p.1 == k &&
          match p.2 with | .ok w => w == v | .error _ => false)).length) ∧
  (∀ k, min ≤ input.countKeyP k Except.isOk →
    min ≤ (emitted.filter (fun p => p.1 == k)).length) ∧
  (∀ k, input.countKeyP k Except.isOk < min →
    (emitted.filter (fun p => p.1 == k)).length = 0)

/-! Smoke tests mirroring the Rust unit tests
(`collect_quorum_with_response_preserves_order`,
`collect_quorum_with_response_no_order`, quorum.rs:165–211). -/

private def okv (n : Nat) : Except Unit Nat := .ok n

private def runWR (min max : Nat) (input : Stream (Nat × Except Unit Nat))
    (strat : Stream (Nat × Except Unit Nat) → Batching (Nat × Except Unit Nat)) :
    List (Nat × Nat) :=
  (collectQuorumWRTick min max).allTicksOutput (strat input)

private def wrIn1 : Stream (Nat × Except Unit Nat) :=
  [(1, okv 10), (1, okv 11), (1, okv 12), (2, okv 20), (2, okv 21),
   (3, okv 30), (3, okv 31), (3, okv 32)]

-- preserves_order test (3,3): key 1 has 3 oks, key 2 only 2, key 3 has 3
#guard runWR 3 3 wrIn1 .whole
  = [(1, 10), (1, 11), (1, 12), (3, 30), (3, 31), (3, 32)]
#guard runWR 3 3 wrIn1 .singletons
  = [(1, 10), (1, 11), (1, 12), (3, 30), (3, 31), (3, 32)]
-- no_order test (2,2): keys 1 and 3 reach quorum
#guard (runWR 2 2 [(1, okv 1), (1, okv 2), (2, okv 3), (3, okv 4), (3, okv 5)]
    .whole).map Prod.fst = [1, 1, 3, 3]
-- general branch (2,3): co-batched straggler delivered with the quorum batch…
#guard runWR 2 3 [(1, okv 1), (1, okv 2), (1, okv 3)] .whole
  = [(1, 1), (1, 2), (1, 3)]
-- …but a straggler in a LATER batch is suppressed by `min_but_not_max`
-- (quorum.rs:65–69): the emitted multiset is batching-dependent beyond the
-- ≥ min guarantee (see `QuorumWithResponseSpec` doc).
#guard runWR 2 3 [(1, okv 1), (1, okv 2), (1, okv 3)] .singletons
  = [(1, 1), (1, 2)]

end HydroLean.Programs
