import Hydro.TransferTheory
import Hydro.Std.Quorum

/-!
# Hydro · liveness under fairness (prototype)

The first rungs of the liveness ladder (`LIVENESS.md`): TLA+-style
fairness, natively — **fairness is a subset of the schedule space**,
and *eventually* is ∃-horizon attainment over the same prefix-monotone
histories the safety theorems quantify.

Because the machine is deterministic given its schedule tuple
(pacing, cursors, timing decisions), a TLA+
"behavior" *is* a schedule point, and the WF conditions become
predicates on the parameters we already quantify over:

- `FairTicks p` — the member ticks infinitely often (WF of the tick
  action);
- `FairCursor c` — the delivery cursor grows without bound: every
  sent message is eventually delivered (WF of the deliver action;
  `cumMax` makes enabledness persistent, so WF suffices and SF has no
  client here).

Safety theorems remain quantified over *all* schedules; liveness
quantifies over the fair subset. **No semantic change anywhere**: the
machine, the denotation, and every safety artifact are untouched.

## The per-op fair kit (WF1 analogues)

`TransferTheory.lean`'s end-of-time kit proves *generous*-schedule
attainment (identity cursors, always-tick) at explicit horizons. The
fair kit generalizes the schedule and existentializes the horizon:

- `deliver_fair_attains` — a stabilized wire is eventually delivered
  in full through *any* fair cursor;
- `ticks_fair_attains` — a stabilized wire is eventually consumed in
  full by *any* fair tick skeleton (consume-all-at-tick).

Composition along a program is leads-to transitivity; the register
scans (`scan_emit_ind`) are the induction rule. See `LIVENESS.md` for
the full Rosetta and the ladder.

## The load-bearing theorem lives in `LivenessChain.lean`

`cq_live` — `collect_quorum` liveness on the real step machine — is
stated and proven there, through the chain frame: the `Values` point
lemma consumes the colocated contract (`emit_count`), the two generic
kits (chain-fairness transfer, tightness) carry it to the machine wire.
This file holds the schedule-side vocabulary and the per-op fair kit
only. The rung-1 machine-walk proof (D48), which re-proved the crossing
count against a hand-written register-machine mirror of the quorum
block, is retired with that mirror (FINDINGS D65): the machine's
per-tick emissions are the construct's own step on the runtime's lists,
read through the corner's generic body coupling, never respelled.
-/

namespace Hydro

/-! ## Fairness: predicates on the schedule space -/

/-- WF(tick): the skeleton ticks infinitely often. -/
def FairTicks (p : Nat → Bool) : Prop :=
  ∀ n, ∃ t, n ≤ t ∧ p t = true

/-- WF(deliver): the delivery cursor grows without bound — every sent
message is eventually delivered. -/
def FairCursor (c : Nat → Nat) : Prop :=
  ∀ n, ∃ t, n ≤ cumMax c t

/-- The generous skeleton is fair. -/
theorem fairTicks_generous : FairTicks (fun _ => true) :=
  fun n => ⟨n, Nat.le_refl n, rfl⟩

/-- The generous cursor is fair. -/
theorem fairCursor_generous : FairCursor (fun s => s) :=
  fun n => ⟨n, by rw [cumMax_id]⟩

/-- A non-trivially fair skeleton: ticking only at odd steps. -/
theorem fairTicks_odd : FairTicks (fun t => t % 2 == 1) := by
  intro n
  refine ⟨2 * n + 1, by omega, ?_⟩
  simp [Nat.add_mod, Nat.mul_mod_right]

/-! ## The per-op fair kit -/

theorem cumMax_mono_le (c : Nat → Nat) {t t' : Nat} (h : t ≤ t') :
    cumMax c t ≤ cumMax c t' := by
  induction t' with
  | zero => cases Nat.le_zero.mp h; exact Nat.le_refl _
  | succ t' ih =>
    rcases Nat.lt_or_ge t (t' + 1) with hlt | hge
    · exact (ih (Nat.lt_succ_iff.mp hlt)).trans (cumMax_mono c t')
    · cases Nat.le_antisymm h hge; exact Nat.le_refl _

/-- **Fair delivery attains** (WF1 for `deliver`): a stabilized wire is
eventually delivered in full through any fair cursor, and holds. -/
theorem deliver_fair_attains {α : Type} {h : StepHist α} {T₀ : Nat}
    {c : Nat → Nat} (hs : StabilizesAt h T₀) (hc : FairCursor c) :
    ∃ T, ∀ t, T ≤ t → (h.deliver c).view t = h.view T₀ := by
  obtain ⟨t₀, ht₀⟩ := hc (h.view T₀).length
  refine ⟨max (T₀ + 1) (t₀ + 1), fun t ht => ?_⟩
  have hT₀ : T₀ + 1 ≤ t := le_trans (Nat.le_max_left _ _) ht
  have ht₀' : t₀ + 1 ≤ t := le_trans (Nat.le_max_right _ _) ht
  obtain ⟨u, rfl⟩ : ∃ u, t = u + 1 :=
    ⟨t - 1, by omega⟩
  show (h.view u).take (cumMax c (u + 1)) = h.view T₀
  rw [hs.view_ge (by omega)]
  exact List.take_of_length_le
    (le_trans ht₀ (cumMax_mono_le c (by omega)))

/-- A prefix of tick-step lists yields a prefix of batch traces. -/
theorem batchesFrom_prefix {α : Type} (src : Nat → List α)
    {ss ss' : List Nat} (h : ss <+: ss') (c : Nat) :
    batchesFrom src ss c <+: batchesFrom src ss' c := by
  obtain ⟨e, rfl⟩ := h
  rw [batchesFrom_append]
  exact ⟨_, rfl⟩

/-- Dropping through a prefix decomposes: for `a <+: b` and
`n ≤ a.length`, `b.drop n = a.drop n ++ b.drop a.length`. -/
theorem drop_prefix_decomp {α : Type} {a b : List α}
    (hab : a <+: b) {n : Nat} (hn : n ≤ a.length) :
    b.drop n = a.drop n ++ b.drop a.length := by
  obtain ⟨e, rfl⟩ := hab
  rw [List.drop_append_of_le_length hn, List.drop_left]

/-- The last element of a `(· ≤ ·)`-pairwise cons list dominates its
head. -/
theorem pairwise_le_getLast {s : Nat} :
    ∀ {ss : List Nat}, List.Pairwise (· ≤ ·) (s :: ss) →
      s ≤ (s :: ss).getLast (List.cons_ne_nil s ss)
  | [], _ => Nat.le_refl s
  | s' :: ss, h => by
    rw [List.getLast_cons (List.cons_ne_nil s' ss)]
    exact (List.pairwise_cons.mp h).1 _
      (List.getLast_mem (List.cons_ne_nil s' ss))

/-- **Consume-all telescopes**: over an ascending run of tick steps,
the concatenation of the per-tick batches is exactly the wire's view at
the last tick (minus the initially-consumed prefix `c`). -/
theorem batchesFrom_flatten {α : Type} (h : StepHist α) :
    ∀ (s : Nat) (ss : List Nat) (c : Nat),
      List.Pairwise (· ≤ ·) (s :: ss) → c ≤ (h.view s).length →
      (batchesFrom h.view (s :: ss) c).flatten
        = (h.view ((s :: ss).getLast (List.cons_ne_nil s ss))).drop c
  | s, [], c, _, _ => by
    show ((h.view s).drop c :: []).flatten = _
    simp [List.getLast]
  | s, s' :: ss, c, hp, hc => by
    have hss' : s ≤ s' := (List.pairwise_cons.mp hp).1 s' (by simp)
    have hp' : List.Pairwise (· ≤ ·) (s' :: ss) :=
      (List.pairwise_cons.mp hp).2
    have hpre : h.view s <+: h.view s' := h.mono_le hss'
    have hc' : (h.view s).length ≤ (h.view s').length :=
      hpre.length_le
    show ((h.view s).drop c ::
        batchesFrom h.view (s' :: ss) (h.view s).length).flatten = _
    rw [List.flatten_cons,
      batchesFrom_flatten h s' ss (h.view s).length hp' hc',
      List.getLast_cons (List.cons_ne_nil s' ss)]
    have hlast : s ≤ (s' :: ss).getLast (List.cons_ne_nil s' ss) := by
      have := pairwise_le_getLast hp
      rwa [List.getLast_cons (List.cons_ne_nil s' ss)] at this
    exact (drop_prefix_decomp (h.mono_le hlast) hc).symm

/-- A ticking step is the last tick step of its own horizon. -/
theorem tickSteps_getLast {p : Nat → Bool} {t : Nat}
    (hp : p t = true) :
    ∃ hne : tickSteps p t ≠ [],
      (tickSteps p t).getLast hne = t := by
  have hsplit : tickSteps p t = (List.range t).filter p ++ [t] := by
    unfold tickSteps
    rw [List.range_succ, List.filter_append]
    simp [hp]
  refine ⟨by rw [hsplit]; simp, ?_⟩
  rw [List.getLast_congr _ _ hsplit]
  exact List.getLast_concat ..

/-- **Fair consumption attains** (WF1 for `batch`): a stabilized wire
is eventually consumed in full by any fair tick skeleton. -/
theorem ticks_fair_attains {α : Type} (h : StepHist α) {T₀ : Nat}
    (hstab : StabilizesAt h T₀) {p : Nat → Bool} (hp : FairTicks p) :
    ∃ T, T₀ ≤ T ∧
      (batchesFrom h.view (tickSteps p T) 0).flatten = h.view T₀ := by
  obtain ⟨T, hT₀T, hpT⟩ := hp T₀
  obtain ⟨hne, hlast⟩ := tickSteps_getLast hpT
  refine ⟨T, hT₀T, ?_⟩
  cases hts : tickSteps p T with
  | nil => exact absurd hts hne
  | cons s ss =>
    have hpw : List.Pairwise (· ≤ ·) (s :: ss) := by
      rw [← hts]; exact tickSteps_pairwise p T
    rw [batchesFrom_flatten h s ss 0 hpw (Nat.zero_le _), List.drop_zero]
    have : (s :: ss).getLast (List.cons_ne_nil s ss) = T := by
      rw [← hlast, List.getLast_congr _ _ hts]
    rw [this]
    exact hstab.view_ge hT₀T

/-! ## Prefix lemmas for the emission pipeline -/

/-- Flattening preserves prefixes. -/
theorem flatten_prefix {β : Type} {a b : List (List β)}
    (h : a <+: b) : a.flatten <+: b.flatten := by
  obtain ⟨e, rfl⟩ := h
  rw [List.flatten_append]
  exact ⟨_, rfl⟩

/-- One batch per tick. -/
theorem batchesFrom_length {α : Type} (src : Nat → List α) :
    ∀ (ss : List Nat) (c : Nat), (batchesFrom src ss c).length = ss.length
  | [], _ => rfl
  | _ :: ss, _ => congrArg (· + 1) (batchesFrom_length src ss _)

/-! ## Multiset plumbing -/

theorem ofList_flatten {β : Type} [DecidableEq β] :
    ∀ (l : List (List β)),
      Multiset.ofList l.flatten = (l.map Multiset.ofList).sum
  | [] => rfl
  | x :: l => by
    show Multiset.ofList (x ++ l.flatten) = _
    rw [show Multiset.ofList (x ++ l.flatten)
        = Multiset.ofList x + Multiset.ofList l.flatten from rfl,
      ofList_flatten l]
    rfl

theorem mem_sum_index {β : Type} [DecidableEq β] {k : β} :
    ∀ {l : List (Multiset β)}, k ∈ l.sum →
      ∃ j, ∃ hj : j < l.length, k ∈ l[j]
  | [], h => by simp at h
  | m :: l, h => by
    rw [List.sum_cons, Multiset.mem_add] at h
    rcases h with h | h
    · exact ⟨0, by simp, h⟩
    · obtain ⟨j, hj, hk⟩ := mem_sum_index h
      exact ⟨j + 1, by simpa using hj, hk⟩

end Hydro
