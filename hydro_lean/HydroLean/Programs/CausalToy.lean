import HydroLean.Hydro.CausalAvail
import HydroLean.Hydro.TStream

/-!
# The gateless toy — where plain `batchC` admits acausality

**Status: boundary witness (keep) — the executable record that the D21
boundary rule is tight** (docs/11-causal-availability.md): with no trigger
gate, an acausal cut is *realizable* under plain `batchC` (`#guard`) and
`causalTickLoop` excludes it with a zero-hypothesis theorem.

The minimal two-location request-response loop with NO protocol
protection (contrast: Paxos's election trigger gate, which excludes the
analogous violation — `AcausalExploration.lean`):

- Location `P` ticks; at tick `t` it consumes a batch of replies, then
  emits the request token `t` (its outputs do not depend on its inputs —
  the sharpest case, because the completed availability is then
  well-defined without any fixpoint, and the acausal cut is *realizable*,
  not just writable).
- The environment `A` echoes: replies = requests (the identity pipeline,
  `NoOrder`-transported).

**The property**: a reply consumed at tick `t` is a request sent at some
tick `< t` (no reply to a message not yet sent).

1. Under plain `batchC` against the completed availability, the property
   is FALSE: the decision `[[3]]` (receive reply 3 at tick 0) is legal and
   realized (`#guard`s below) — the acausal cut in its purest form.
2. Under `causalTickLoop` (Option C), the property is a THEOREM with no
   hypotheses (`toy_no_early_reply`), two lines from the combinator's
   elimination principle (`causalTickLoop_causal` → `CausalCuts.mem_availAt`).
   The same acausal decision BLOCKS (`#guard` below): tick 0's
   availability is `env([]) = []`.

This is the program class that genuinely needs framework-level causality;
Paxos is NOT in it (its gate derives the needed causal fact — see the
write-up).
-/

namespace HydroLean.Programs.CausalToy

open HydroLean.Hydro

/-- The environment: replies echo requests (identity pipeline). -/
def echoEnv : List Nat →ₘ List Nat := MonoMap.id

/-- P's step: state = tick counter; output = the request token `t`.
Outputs deliberately ignore the consumed batch (gateless, unguarded). -/
def pStep (t : Nat) (_replies : List Nat) : Nat × List Nat :=
  (t + 1, [t])

/-- The completed request stream of a `fuel`-tick run (for the `batchC`
version: outputs don't depend on inputs, so this is well-defined without
a fixpoint — the completed availability the current combinators check
against). -/
def completedRequests (fuel : Nat) : List Nat := List.range fuel

/-! ## 1. Plain `batchC`: the acausal cut is legal and realized -/

/-- Receive reply `3` at tick 0 — before request `3` is sent at tick 3. -/
def acausalDecision : List (List Nat) := [[3], [], [], []]

-- The acausal decision is legal against the completed availability: all
-- four ticks realize, and tick 0 consumed a reply to a not-yet-sent
-- request.
#guard batchC (completedRequests 4) [] acausalDecision
  = [[3], [], [], []]
-- The violation, machine-checked: the reply consumed at tick 0 is ≥ 0
-- (trivially "from the future" — request 3 is sent at tick 3 > 0).
#guard ((batchC (completedRequests 4) [] acausalDecision).head?.getD []
  |>.any (fun r => decide (r ≥ 0 + 1))) = true

/-! ## 2. `causalTickLoop`: the same cut blocks; the property is a theorem -/

-- The acausal decision BLOCKS at tick 0 (availability env([]) = []).
#guard causalTickLoop echoEnv pStep 0 [] [] acausalDecision = []

/-- A causal decision: echo each request back on the next tick. -/
def causalDecision : List (List Nat) := [[], [0], [1], [2]]

#guard (causalTickLoop echoEnv pStep 0 [] [] causalDecision).map Prod.fst
  = [[], [0], [1], [2]]

/-- **The toy property, zero hypotheses** (Option C): in every realized
`causalTickLoop` run of the echo loop, a reply consumed at tick `t` is a
request emitted at a tick `< t` — i.e. its value is `< t`, since `pStep`
emits token `u` at tick `u`. Proof = the combinator's elimination
principle; nothing about `pStep`'s closure is re-analyzed beyond its
emitted tokens. -/
theorem toy_no_early_reply (s₀ : Nat) (d : List (List Nat)) {t : Nat}
    (ht : t < ((causalTickLoop echoEnv pStep s₀ [] [] d).map
      Prod.fst).length)
    {r : Nat}
    (hr : r ∈ ((causalTickLoop echoEnv pStep s₀ [] [] d).map
      Prod.fst)[t]'ht) :
    r ∈ (((causalTickLoop echoEnv pStep s₀ [] [] d).map
      Prod.snd).take t).flatten := by
  have := (causalTickLoop_causal echoEnv pStep s₀ d).mem_availAt ht hr
  exact this

/-- Specialized: with `pStep` starting at 0, the consumed reply's VALUE is
below the tick index (token `u` is emitted at output tick `u`). -/
theorem toy_reply_lt_tick (d : List (List Nat)) {t : Nat}
    (ht : t < ((causalTickLoop echoEnv pStep 0 [] [] d).map
      Prod.fst).length)
    {r : Nat}
    (hr : r ∈ ((causalTickLoop echoEnv pStep 0 [] [] d).map
      Prod.fst)[t]'ht) :
    r < t := by
  have hmem := toy_no_early_reply 0 d ht hr
  -- the outputs of the run are the singleton tokens [s], [s+1], …
  have houts : ∀ (s : Nat) (outs : List (List Nat)) (consumed : List Nat)
      (d : List (List Nat)),
      ∃ k, (causalTickLoop echoEnv pStep s outs consumed d).map Prod.snd
        = (List.range' s k).map (fun u => [u]) := by
    intro s outs consumed d
    induction d generalizing s outs consumed with
    | nil => exact ⟨0, rfl⟩
    | cons b ds ih =>
      unfold causalTickLoop
      by_cases hb : BatchLegal (availAt echoEnv outs outs.length)
          consumed b
      · rw [if_pos hb, List.map_cons]
        obtain ⟨k, hk⟩ := ih (s + 1) (outs ++ [(pStep s b).2])
          (consumed ++ b)
        refine ⟨k + 1, ?_⟩
        rw [List.range'_succ, List.map_cons]
        exact congrArg (List.cons [s]) hk
      · rw [if_neg hb]
        exact ⟨0, rfl⟩
  obtain ⟨k, hk⟩ := houts 0 [] [] d
  rw [hk, ← List.map_take] at hmem
  have hflat : ∀ (l : List Nat), (l.map fun u => [u]).flatten = l := by
    intro l
    induction l with
    | nil => rfl
    | cons x xs ih => simp [ih]
  rw [hflat] at hmem
  obtain ⟨idx, hidx, rfl⟩ := List.mem_iff_getElem.mp hmem
  rw [List.length_take, List.length_range'] at hidx
  rw [List.getElem_take, List.getElem_range'] at *
  omega

end HydroLean.Programs.CausalToy
