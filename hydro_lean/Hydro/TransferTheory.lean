import Hydro.Transfer

/-!
# Per-wire stabilization (`StabilizesAt`)

The schedule-side stabilization vocabulary the liveness layer packages (`Hydro/Liveness.lean`,
`Hydro/LivenessChain.lean`): a machine wire whose history is constant from `T` on
(`StabilizesAt`), and the per-op preservation lemmas — pointwise ops preserve stabilization,
generous delivery attains the sent content, fan-in merges stabilize when their inputs do. The
notion is per-wire and compositional: a partially-churning program still gets facts on its
stabilized wires, and there is no global quiescence predicate.

Tightness — the machine's decision-mediated reads equal the denotation's at the derived
decisions — is read off the coupling corner's generated artifacts per module
(`collect_quorum_tight` in `Hydro/LivenessChain.lean`), not stated here.

## What is not claimed

There is no `∀ decisions, ∃ schedule` theorem, and there cannot be one: the machine realizes
strictly fewer behaviors than the denotational decision space licenses. `flattenUnordered`
publishes one program-determined order per schedule while the denotational `selectOrder`
space licenses every permutation; the machine is per-pair FIFO; `RetryPool` re-reads are
never performed by the machine. The quotient is an over-approximation: safety transfer needs
machine ⊆ denotation only, and the surplus makes program obligations stronger at the cost of
conservativity, never soundness.
-/

namespace Hydro

/-! ## End of time: per-wire stabilization gives exact equality

Each lemma is "inputs stabilized ⇒ output stabilized, attaining the denotational content".
Pointwise ops preserve stabilization; consumption ops attain their pools once delivery and
ticking are generous. -/

/-- A wire has no more work at `T`: its history is constant from `T` on. -/
def StabilizesAt {α : Type} (h : StepHist α) (T : Nat) : Prop :=
  ∀ t, h.view (T + t) = h.view T

/-- The view at any step `≥ T` is the view at `T`. -/
theorem StabilizesAt.view_ge {α : Type} {h : StepHist α} {T t : Nat}
    (hs : StabilizesAt h T) (ht : T ≤ t) : h.view t = h.view T := by
  have := hs (t - T)
  rwa [Nat.add_sub_cancel' ht] at this

/-- A constant wire is stable from step 0. -/
theorem stabilizesAt_const {α : Type} (l : List α) :
    StabilizesAt (StepHist.const l) 0 := fun _ => rfl

/-- `map` preserves stabilization at the same step. -/
theorem StabilizesAt.map {α β : Type} {h : StepHist α} {T : Nat}
    (f : α → β) (hs : StabilizesAt h T) :
    StabilizesAt (h.map f) T := fun t =>
  congrArg (List.map f) (hs t)

/-- `filterMap` preserves stabilization at the same step. -/
theorem StabilizesAt.filterMap {α β : Type} {h : StepHist α} {T : Nat}
    (f : α → Option β) (hs : StabilizesAt h T) :
    StabilizesAt (h.filterMap f) T := fun t =>
  congrArg (List.filterMap f) (hs t)

/-- The identity cursor delivers step for step: its cumulative max is itself. -/
theorem cumMax_id : ∀ t, cumMax (fun s => s) t = t
  | 0 => rfl
  | t + 1 => by
    show max (cumMax (fun s => s) t) (t + 1) = t + 1
    rw [cumMax_id t]
    exact Nat.max_eq_right (Nat.le_succ t)

/-- Generous delivery (every step active, identity cursor) of a wire stable at `T` is stable
at `T + length + 1`: the cursor needs `length` steps past `T` to cover the content, and
delivery reads one step back. -/
theorem StabilizesAt.deliver_id {α : Type} {h : StepHist α} {T : Nat}
    (hs : StabilizesAt h T) :
    StabilizesAt (h.deliver (fun _ => true) (fun s => s)) (T + (h.view T).length + 1) := by
  intro t
  have key : ∀ u, T ≤ u → (h.view T).length ≤ u + 1 →
      (h.deliver (fun _ => true) (fun s => s)).view (u + 1) = h.view T := by
    intro u hu hlen
    show gateView (fun _ => true) (fun t => (h.view t).take (cumMax (fun s => s) (t + 1))) (u + 1) = h.view T
    rw [gateView_true, cumMax_id]
    obtain ⟨w, rfl⟩ := Nat.exists_eq_add_of_le hu
    rw [hs w, List.take_of_length_le (by omega)]
  show (h.deliver (fun _ => true) (fun s => s)).view (T + (h.view T).length + 1 + t) = _
  rw [show T + (h.view T).length + 1 + t
      = (T + (h.view T).length + t) + 1 from by omega,
    key (T + (h.view T).length + t) (by omega) (by omega),
    key (T + (h.view T).length) (by omega) (by omega)]

/-- A stabilized wire's increments vanish from `T` on. -/
theorem StabilizesAt.inc_nil {α : Type} {h : StepHist α} {T t : Nat}
    (hs : StabilizesAt h T) (ht : T ≤ t) : h.inc t = [] := by
  show (h.view (t + 1)).drop (h.view t).length = []
  rw [hs.view_ge (Nat.le_succ_of_le ht), hs.view_ge ht]
  exact List.drop_length

/-- Two-way fan-in stabilizes when both inputs have; its content is then the sum of theirs
(`coe_merge2View`). -/
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

/-- Family fan-in stabilizes when every sender has. -/
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

/-- The consumed length after a run of ticks: the next batch's cursor. -/
def lastLen {α : Type} (src : Nat → List α) : List Nat → Nat → Nat
  | [], c => c
  | s :: rest, _ => lastLen src rest (src s).length

/-- A batch walk over appended tick lists splits at `lastLen`. -/
theorem batchesFrom_append {α : Type} (src : Nat → List α) :
    ∀ (ss ss' : List Nat) (c : Nat),
      batchesFrom src (ss ++ ss') c
        = batchesFrom src ss c ++ batchesFrom src ss' (lastLen src ss c)
  | [], _, _ => rfl
  | s :: ss, ss', c => by
    show (src s).drop c :: batchesFrom src (ss ++ ss') (src s).length = _
    rw [batchesFrom_append src ss ss' (src s).length]
    rfl

/-- `famFreeze` is an identity operator on a member-wise prefix-monotone family. -/
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
