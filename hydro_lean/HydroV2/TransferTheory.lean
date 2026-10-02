import HydroV2.Transfer
import HydroV2.Rel

/-!
# HydroV2 · the transfer theorem package

The statement layer over `Transfer.lean`'s `CorrSem`: tightness
(machine reads *equal* denotational reads at derived decisions, for
every schedule and horizon), and the per-wire **end-of-time** kit
(stabilized wires attain their denotational pools exactly).

## The theorem package

1. **Safety transfer** (`CorrSem`'s carrier `.property` readings): for
   every schedule and every step horizon, the machine's observations are
   coupled below *some* denotational run — with the coupling already an
   **equality** at every decision-mediated observation (`snapshot_tight`,
   `batch_tight`, re-exporting `sched_snap_eq`/`batchCuts_real`): the
   values side can express exactly what the machine consumed, because
   truncation *is* the decision at consumption sites.
2. **End-of-time equality** (per wire): once a wire's history
   stabilizes (`StabilizesAt`), generous delivery attains the full
   content, consumption attains the full pool, fold reads attain and
   hold the limit value, and a cycle whose Kleene chain fixes at depth
   `N` attains its fixed point's stabilized view. Stabilization is
   **per-wire and compositional** — a partially-churning program (say,
   forever-heartbeats beside a stabilizing consensus core) still gets
   exact equality on its stabilized wires. There is deliberately *no*
   global per-run quiescence predicate.

## What is *not* claimed: adequacy is false by design

There is no `∀ decisions, ∃ schedule` theorem, and there cannot be:
the machine realizes strictly fewer behaviors than the denotational
decision space licenses. Counterexample: `emitBatchesUnordered`
publishes a concrete list whose order is *determined* by the program —
the type forgets the order, the machine does not — so a downstream
`assume_ordering` can only ever realize that one emission order per
schedule, while the denotational `selectOrder` space licenses every
permutation. Likewise intra-sender reordering on `NoOrder` wires
(the machine is per-pair FIFO; only cross-sender shuffles are
schedule-reachable) and `RetryPool` re-reads the machine never
performs. The quotient is a deliberate over-approximation: safety
transfer needs machine ⊆ denotation only, and the surplus makes
program obligations *stronger* (robust to weaker transports), at the
sole cost of conservativity — never soundness.

## Upgrade path (doc note only — not implemented)

With a fuel-less (ω-continuous / lfp) `Values` fix, the derived
decisions lose their fuel component and end-of-time equality
strengthens to an ω-limit statement under fairness: the machine's
view-chain limit equals the lfp with **no stabilization hypothesis** —
`StabilizesAt` becomes the attained-at-finite-`T` special case. The
forever-counter cycle (which today gets only per-horizon safety ⊑)
then gains a real limit theorem.
-/

namespace HydroV2

/-! ## Tightness (∀ schedule ∀ horizon: reads are *equalities*) -/

/-- **Snapshot tightness**: for every tick skeleton and every horizon,
the machine's snapshot trace of a fold *equals* the denotational
`snapTrace` at derived cuts — provided only that the horizon view is
coupled (which the transfer provides). -/
theorem snapshot_tight {α σ : Type} [DecidableEq α] (ord : StrOrd)
    (ret : Retries) (g : σ → α → σ) (init : σ) (ok : FoldOkP ord ret g)
    {pool : PoolCarrier α ord ret} (src : StepHist α)
    (p : Nat → Bool) (T : Nat)
    (hcpl : ListLe ord ret (src.view T) pool) :
    ∃ d, (tickSteps p T).map (fun st => (src.view st).foldl g init)
      = snapTrace ord ret g init ok pool d := by
  have hws := viewsChain src (tickSteps_pairwise p T)
  have hcov : ∀ w ∈ (tickSteps p T).map src.view,
      ListLe ord ret w pool := by
    intro w hw
    obtain ⟨s, hs, rfl⟩ := List.mem_map.mp hw
    exact ListLe.of_prefix (src.mono_le (tickSteps_le hs)) hcpl
  obtain ⟨d, hd⟩ := sched_snap_eq ord ret g init ok pool
    ((tickSteps p T).map src.view) hws hcov
  refine ⟨d, ?_⟩
  rw [← hd, List.map_map]
  rfl

/-- **Batch tightness** (unordered): the machine's realized batch
increments are exactly what count-legal denotational batching emits at
the derived increments. -/
theorem batch_tight {α : Type} [DecidableEq α] {pool : Multiset α}
    (src : StepHist α) (p : Nat → Bool) (T : Nat)
    (hcpl : Multiset.ofList (src.view T) ≤ pool) :
    batchCuts pool 0
      ((batchesFrom src.view (tickSteps p T) 0).map
        (fun b => Multiset.ofList b))
      = (batchesFrom src.view (tickSteps p T) 0).map
        (fun b => Multiset.ofList b) := by
  have hcov : ∀ s ∈ tickSteps p T,
      Multiset.ofList (src.view s) ≤ pool := by
    intro s hs
    exact le_trans
      (Multiset.coe_le.mpr
        ((src.mono_le (tickSteps_le hs)).sublist.subperm)) hcpl
  exact batchCuts_real src.view (tickSteps p T) []
    (viewsChain src (tickSteps_pairwise p T)) hcov

/-! ## End of time (per-wire stabilization ⇒ exact equality)

Per-wire and compositional: each lemma is "inputs stabilized ⇒ output
stabilized, attaining the exact denotational content". The remaining
ops follow the same per-op pattern (pointwise ops preserve
stabilization verbatim; consumption ops attain their pools once
delivery and ticking are generous). Cycles need the genuinely
per-program hypothesis that the Kleene chain fixes — see
`fix_diag_attains`. -/

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

/-- …and it attains the wire exactly: nothing is left in flight. -/
theorem deliver_id_attains {α : Type} {h : StepHist α} {T : Nat}
    (hs : StabilizesAt h T) :
    (h.deliver (fun s => s)).view (T + (h.view T).length + 1)
      = h.view T := by
  show (h.view (T + (h.view T).length)).take
      (cumMax (fun s => s) (T + (h.view T).length + 1)) = h.view T
  rw [cumMax_id, hs (h.view T).length, List.take_of_length_le (by omega)]

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
    StabilizesAt (HydroV2.merge2 a b) T := by
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
    StabilizesAt (HydroV2.mergeN k) T := by
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

/-! ### Consumption attains the wire (generous ticking) -/

/-- The consumed-so-far length after a run of ticks. -/
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

theorem lastLen_append_singleton {α : Type} (src : Nat → List α) :
    ∀ (ss : List Nat) (s : Nat) (c : Nat),
      lastLen src (ss ++ [s]) c = (src s).length
  | [], _, _ => rfl
  | _ :: ss, s, _ => lastLen_append_singleton src ss s _

/-- Ticking every step consumes exactly the wire so far. -/
theorem flatten_batches_range {α : Type} (h : StepHist α) :
    ∀ t, (batchesFrom h.view (List.range (t + 1)) 0).flatten
      = h.view t
  | 0 => by
    show ((h.view 0).drop 0 :: []).flatten = h.view 0
    simp
  | t + 1 => by
    rw [List.range_succ, batchesFrom_append, List.flatten_append,
      flatten_batches_range h t]
    have hlast : lastLen h.view (List.range (t + 1)) 0
        = (h.view t).length := by
      rw [List.range_succ, lastLen_append_singleton]
    rw [hlast]
    show h.view t ++ ((h.view (t + 1)).drop (h.view t).length ++ []) = _
    rw [List.append_nil]
    exact prefix_append_drop (h.mono t)

theorem tickSteps_true (t : Nat) :
    tickSteps (fun _ => true) t = List.range (t + 1) := by
  simp [tickSteps]

/-- Under generous ticking, a stabilized wire's consumption attains it
exactly and holds. -/
theorem batch_flat_attains {α : Type} {h : StepHist α} {T : Nat}
    (hs : StabilizesAt h T) (t : Nat) :
    (batchesFrom h.view (tickSteps (fun _ => true) (T + t)) 0).flatten
      = h.view T := by
  rw [tickSteps_true, flatten_batches_range, hs t]

/-! ### Cycles: the knot attains its fixed point -/

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

/-- **The knot attains its fixed point**: if the Kleene chain fixes at
depth `N` and the fixed point's wire stabilizes at `T` (and the raw
diagonal is monotone — guardedness, `rfl`-checkable per program), the
machine's cycle diagonal attains the fixed point's stabilized view and
holds it. The derived denotational fuel is `N`; by
`iterate_stab_of_fixed`, every fuel ≥ `N` denotes the same pool, so
the equality is against *the* denotation. -/
theorem fix_diag_attains {n : Nat} {α : Type} [DecidableEq α]
    {f : (Fin n → StepHist α) → (Fin n → StepHist α)} {N T : Nat}
    (hfix : iterate f (fun _ => StepHist.bot) (N + 1)
      = iterate f (fun _ => StepHist.bot) N)
    (hstab : ∀ i, StabilizesAt
      (iterate f (fun _ => StepHist.bot) N i) T)
    (hmono : ∀ t i,
      ((iterate f (fun _ => StepHist.bot) (t + 1)) i).view t
        <+: ((iterate f (fun _ => StepHist.bot) (t + 2)) i).view (t + 1))
    (i : Fin n) (t : Nat) :
    (famHist (fun t i =>
        ((iterate f (fun _ => StepHist.bot) (t + 1)) i).view t) i).view
      (N + T + t)
      = (iterate f (fun _ => StepHist.bot) N i).view T := by
  show famFreeze (fun t i =>
      ((iterate f (fun _ => StepHist.bot) (t + 1)) i).view t)
      (N + T + t) i = _
  rw [famFreeze_of_mono (fun t i => hmono t i)]
  have hit : iterate f (fun _ => StepHist.bot) (N + T + t + 1)
      = iterate f (fun _ => StepHist.bot) N :=
    iterate_stab_of_fixed hfix (by omega)
  rw [hit]
  have : N + T + t = T + (N + t) := by omega
  rw [this]
  exact (hstab i) (N + t)

end HydroV2
