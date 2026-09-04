import Hydro.Transfer

/-!
# Hydro · the per-wire end-of-time kit (`StabilizesAt`)

The schedule-side **stabilization** vocabulary the liveness layer
packages (`Liveness.lean`, `LivenessChain.lean`): a machine wire whose
history is constant from `T` on (`StabilizesAt`), and the per-op
preservation lemmas — pointwise ops preserve stabilization verbatim,
generous delivery attains the sent content, fan-in merges stabilize
when their inputs do. Per-wire and compositional: a partially-churning
program still gets exact facts on its stabilized wires; there is
deliberately *no* global quiescence predicate.

Tightness — the machine's decision-mediated reads EQUAL the
denotation's at the derived decisions — is no longer a hand kit here:
it is read off the coupling corner's generated artifacts per module
(`collect_quorum_tight`, LivenessChain.lean, FINDINGS D65), the
specification of the generic kit liveness rung 2 builds.

## What is *not* claimed: adequacy is false by design

There is no `∀ decisions, ∃ schedule` theorem, and there cannot be:
the machine realizes strictly fewer behaviors than the denotational
decision space licenses (`flattenUnordered` publishes one
program-determined order per schedule while the denotational
`selectOrder` space licenses every permutation; the machine is
per-pair FIFO; `RetryPool` re-reads the machine never performs). The
quotient is a deliberate over-approximation: safety transfer needs
machine ⊆ denotation only, and the surplus makes program obligations
*stronger*, at the sole cost of conservativity — never soundness.
-/

namespace Hydro

/-! ## End of time (per-wire stabilization ⇒ exact equality)

Per-wire and compositional: each lemma is "inputs stabilized ⇒ output
stabilized, attaining the exact denotational content". The remaining
ops follow the same per-op pattern (pointwise ops preserve
stabilization verbatim; consumption ops attain their pools once
delivery and ticking are generous). -/

/-- A wire has no more work at `T`: its history is constant from `T`. -/
def StabilizesAt {α : Type} (h : StepHist α) (T : Nat) : Prop :=
  ∀ t, h.view (T + t) = h.view T

theorem StabilizesAt.view_ge {α : Type} {h : StepHist α} {T t : Nat}
    (hs : StabilizesAt h T) (ht : T ≤ t) : h.view t = h.view T := by
  have := hs (t - T)
  rwa [Nat.add_sub_cancel' ht] at this

theorem stabilizesAt_const {α : Type} (l : List α) :
    StabilizesAt (StepHist.const l) 0 := fun _ => rfl

theorem StabilizesAt.map {α β : Type} {h : StepHist α} {T : Nat}
    (f : α → β) (hs : StabilizesAt h T) :
    StabilizesAt (h.map f) T := fun t =>
  congrArg (List.map f) (hs t)

theorem StabilizesAt.filterMap {α β : Type} {h : StepHist α} {T : Nat}
    (f : α → Option β) (hs : StabilizesAt h T) :
    StabilizesAt (h.filterMap f) T := fun t =>
  congrArg (List.filterMap f) (hs t)

/-- The generous cursor delivers step-for-step. -/
theorem cumMax_id : ∀ t, cumMax (fun s => s) t = t
  | 0 => rfl
  | t + 1 => by
    show max (cumMax (fun s => s) t) (t + 1) = t + 1
    rw [cumMax_id t]
    exact Nat.max_eq_right (Nat.le_succ t)

/-- Generous delivery stabilizes once the wire has, after catching up
to its content length… -/
theorem StabilizesAt.deliver_id {α : Type} {h : StepHist α} {T : Nat}
    (hs : StabilizesAt h T) :
    StabilizesAt (h.deliver (fun s => s)) (T + (h.view T).length + 1) := by
  intro t
  have key : ∀ u, T ≤ u → (h.view T).length ≤ u + 1 →
      (h.deliver (fun s => s)).view (u + 1) = h.view T := by
    intro u hu hlen
    show (h.view u).take (cumMax (fun s => s) (u + 1)) = h.view T
    rw [cumMax_id]
    obtain ⟨w, rfl⟩ := Nat.exists_eq_add_of_le hu
    rw [hs w, List.take_of_length_le (by omega)]
  show (h.deliver (fun s => s)).view (T + (h.view T).length + 1 + t) = _
  rw [show T + (h.view T).length + 1 + t
      = (T + (h.view T).length + t) + 1 from by omega,
    key (T + (h.view T).length + t) (by omega) (by omega),
    key (T + (h.view T).length) (by omega) (by omega)]

/-- A stabilized wire's increments vanish. -/
theorem StabilizesAt.inc_nil {α : Type} {h : StepHist α} {T t : Nat}
    (hs : StabilizesAt h T) (ht : T ≤ t) : h.inc t = [] := by
  show (h.view (t + 1)).drop (h.view t).length = []
  rw [hs.view_ge (Nat.le_succ_of_le ht), hs.view_ge ht]
  exact List.drop_length

/-- Fan-in stabilizes with its inputs (and its content is exactly the
sum of theirs — `coe_merge2View`). -/
theorem StabilizesAt.merge2 {α : Type} [DecidableEq α]
    {a b : StepHist α} {T : Nat}
    (ha : StabilizesAt a T) (hb : StabilizesAt b T) :
    StabilizesAt (Hydro.merge2 a b) T := by
  intro t
  induction t with
  | zero => rfl
  | succ t ih =>
    show merge2View a b (T + t) ++ a.inc (T + t) ++ b.inc (T + t) = _
    rw [ha.inc_nil (Nat.le_add_right T t), hb.inc_nil (Nat.le_add_right T t),
      List.append_nil, List.append_nil]
    exact ih

theorem StabilizesAt.mergeN {m : Nat} {α : Type} [DecidableEq α]
    {k : Fin m → StepHist α} {T : Nat}
    (hk : ∀ j, StabilizesAt (k j) T) :
    StabilizesAt (Hydro.mergeN k) T := by
  intro t
  induction t with
  | zero => rfl
  | succ t ih =>
    show mergeNView k (T + t)
        ++ (List.finRange m).flatMap (fun j => (k j).inc (T + t)) = _
    have hflat : (List.finRange m).flatMap
        (fun j => (k j).inc (T + t)) = [] := by
      refine List.flatMap_eq_nil_iff.mpr (fun j _ => ?_)
      exact (hk j).inc_nil (Nat.le_add_right T t)
    rw [hflat, List.append_nil]
    exact ih

/-! ### Consumption: batches over a split tick list -/

/-- The consumed length after a run of ticks (the next batch's cursor). -/
def lastLen {α : Type} (src : Nat → List α) : List Nat → Nat → Nat
  | [], c => c
  | s :: rest, _ => lastLen src rest (src s).length

theorem batchesFrom_append {α : Type} (src : Nat → List α) :
    ∀ (ss ss' : List Nat) (c : Nat),
      batchesFrom src (ss ++ ss') c
        = batchesFrom src ss c ++ batchesFrom src ss' (lastLen src ss c)
  | [], _, _ => rfl
  | s :: ss, ss', c => by
    show (src s).drop c :: batchesFrom src (ss ++ ss') (src s).length = _
    rw [batchesFrom_append src ss ss' (src s).length]
    rfl

/-- Freezing is a no-op on member-wise prefix-monotone families. -/
theorem famFreeze_of_mono {n : Nat} {β : Type} [DecidableEq β]
    {h : Nat → Fin n → List β}
    (hm : ∀ t i, h t i <+: h (t + 1) i) : ∀ t, famFreeze h t = h t
  | 0 => rfl
  | t + 1 => by
    show (if (List.finRange n).all
        (fun i => (famFreeze h t i).isPrefixOf (h (t + 1) i))
      then h (t + 1) else famFreeze h t) = h (t + 1)
    rw [if_pos]
    rw [List.all_eq_true]
    intro i _
    rw [famFreeze_of_mono hm t]
    exact List.isPrefixOf_iff_prefix.mpr (hm t i)

end Hydro
