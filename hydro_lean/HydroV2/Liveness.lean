import HydroV2.TransferTheory
import HydroV2.Std.Quorum

/-!
# HydroV2 · liveness under fairness (prototype)

The first rungs of the liveness ladder (`LIVENESS.md`): TLA+-style
fairness, natively — **fairness is a subset of the schedule space**,
and *eventually* is ∃-horizon attainment over the same prefix-monotone
histories the safety theorems quantify.

Because the machine is deterministic given its schedule tuple
(pacing, cursors, timing decisions, emission linearizations), a TLA+
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

## The load-bearing theorem: `cq_live`

`collect_quorum` liveness on the real step machine: if the response
wire stabilizes holding a quorum of `Ok` votes for `k` (under the
usage contract), the member's tick skeleton reaches a tick past the
stabilization point within the emission-claim supply, and the claimed
linearizations are honest wherever defined, then **`k` is eventually
on the output wire** — `∃ T, k ∈ output.view T`.

### The emission-supply finding

Liveness surfaces a finitary carrier that safety never notices:
`SchedSem`'s `EmitDec n β = Fin n → List (List β)` is a *finite* list
of per-tick linearization claims, while a fair skeleton ticks forever
— so every finite claim list eventually exhausts and `emitLin` stops
emitting. For safety (∀-horizon, prefix-closed observations) this is
invisible; for liveness it is exactly a WF(emit) obligation. Hence
`cq_live`'s supply premise (`Tw` below): *the claim list covers the
ticks up to one fair tick past stabilization* — the honest,
schedule-level spelling available today. The model-level alternative
(upgrade `EmitDec` to claim *streams* `Nat → List β`, making
`FairTicks` alone sufficient) is a small, safety-preserving Sched
change catalogued in `LIVENESS.md` as a ratification fork.
-/

namespace HydroV2

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

/-- Validated emission is monotone in the value trace (a longer run
emits an extension — including the jam case, which stops both). -/
theorem emitLin_prefix {β : Type} [DecidableEq β] :
    ∀ {vs vs' : Trace (Multiset β)} (ls : List (List β)),
      vs <+: vs' → emitLin vs ls <+: emitLin vs' ls
  | [], _, _, _ => List.nil_prefix
  | _ :: _, _, [], h => by
    obtain ⟨e, rfl⟩ := h
    exact List.prefix_refl _
  | m :: vs, _, l :: ls, h => by
    obtain ⟨e, rfl⟩ := h
    show emitLin (m :: vs) (l :: ls) <+: emitLin (m :: (vs ++ e)) (l :: ls)
    show (if (↑l : Multiset β) = m then l :: emitLin vs ls else [])
      <+: (if (↑l : Multiset β) = m then l :: emitLin (vs ++ e) ls else [])
    split
    · exact List.cons_prefix_cons.mpr
        ⟨rfl, emitLin_prefix ls ⟨e, rfl⟩⟩
    · exact List.nil_prefix

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

/-! ## Emission legality (the WF(emit) vocabulary) -/

/-- The claimed linearizations are honest wherever both the claim list
and the run trace are defined. -/
def OverlapLegal {β : Type} [DecidableEq β]
    (ls : List (List β)) (vs : Trace (Multiset β)) : Prop :=
  ∀ j (hj : j < vs.length) (hl : j < ls.length),
    (↑(ls[j]) : Multiset β) = vs[j]

/-- Under overlap-legality, validated emission emits every claim it
has values for: `emitLin vs ls = ls.take vs.length` (which is all of
`ls` if the claims run out first). -/
theorem emitLin_of_overlap {β : Type} [DecidableEq β] :
    ∀ (vs : Trace (Multiset β)) (ls : List (List β)),
      OverlapLegal ls vs → emitLin vs ls = ls.take vs.length
  | [], ls, _ => by simp [emitLin]
  | _ :: _, [], _ => by simp [emitLin]
  | m :: vs, l :: ls, hleg => by
    have h0 : (↑l : Multiset _) = m :=
      hleg 0 (by simp) (by simp)
    show (if (↑l : Multiset _) = m then l :: emitLin vs ls else []) = _
    rw [if_pos h0, emitLin_of_overlap vs ls
      (fun j hj hl => hleg (j + 1) (by simpa using hj) (by simpa using hl))]
    rfl

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

/-! ## `collect_quorum` liveness on the step machine -/

variable {K E : Type} [DecidableEq K] [DecidableEq E]

/-- The machine's per-tick emission trace of `collect_quorum` (the
scan of the register machine over the consume-all batches), named so
the emission-legality premise is stateable. -/
def cqMachineTrace (pacing : Unit → Fin 1 → Nat → Bool)
    (h : Fin 1 → StepHist (K × Except E Unit)) (mn mx : Nat)
    (i : Fin 1) (t : Nat) : Trace (Multiset K) :=
  scanAcrossTicksTrace (fun s b => cqTick mn mx s (Multiset.ofList b))
    CQState.init
    (batchesFrom ((h i).view) (tickSteps (pacing () i) t) 0)

/-- The machine's output wire family (pre-freeze). -/
def cqMachineFam (pacing : Unit → Fin 1 → Nat → Bool)
    (h : Fin 1 → StepHist (K × Except E Unit)) (mn mx : Nat)
    (demit : Fin 1 → List (List K)) (t : Nat) (i : Fin 1) : List K :=
  (emitLin (cqMachineTrace pacing h mn mx i t) (demit i)).flatten

omit [DecidableEq E] in
/-- The emission family is member-wise prefix-monotone, so the
machine's `famFreeze` is a no-op on it. -/
theorem cqMachineFam_mono (pacing : Unit → Fin 1 → Nat → Bool)
    (h : Fin 1 → StepHist (K × Except E Unit)) (mn mx : Nat)
    (demit : Fin 1 → List (List K)) (t : Nat) (i : Fin 1) :
    cqMachineFam pacing h mn mx demit t i
      <+: cqMachineFam pacing h mn mx demit (t + 1) i :=
  flatten_prefix (emitLin_prefix (demit i)
    (scanAcrossTicksTrace_prefix _ _
      (batchesFrom_prefix _ (tickSteps_prefix _ t) 0)))

/-- **`collect_quorum` liveness** (the first real liveness theorem on
the step machine): if the response wire stabilizes at `T₀` holding a
quorum of `Ok` votes for `k` under the usage contract, the tick
skeleton reaches a tick at/after `T₀` within the emission-claim supply
(`Tw` — the WF(tick) ∧ WF(emit) premise; see the header), and the
claimed linearizations are honest wherever defined, then `k` is
**eventually on the output wire**. -/
theorem cq_live {pacing : Unit → Fin 1 → Nat → Bool}
    (h : Fin 1 → StepHist (K × Except E Unit)) {T₀ : Nat}
    (hstab : StabilizesAt (h 0) T₀)
    (mn mx : Nat) (demit : Fin 1 → List (List K)) (k : K)
    (h1 : 1 ≤ mn) (hmm : mn ≤ mx)
    (hcap : cqKeyCount (Multiset.ofList ((h 0).view T₀)) k ≤ mx)
    (hq : mn ≤ cqOkCount (Multiset.ofList ((h 0).view T₀)) k)
    (hsupply : ∃ Tw, T₀ ≤ Tw ∧ pacing () 0 Tw = true ∧
      (tickSteps (pacing () 0) Tw).length ≤ (demit 0).length)
    (hleg : ∀ t, OverlapLegal (demit 0)
      (cqMachineTrace pacing h mn mx 0 t)) :
    ∃ T, k ∈ ((collect_quorum (SchedSem Unit (fun _ => 1) pacing) ()
      h mn mx () demit).val.1 0).view T := by
  obtain ⟨Tw, hT₀Tw, hpTw, hsup⟩ := hsupply
  refine ⟨Tw, ?_⟩
  -- 1. Name the machine output: famHist of the emission family.
  have hout : ((collect_quorum (SchedSem Unit (fun _ => 1) pacing) ()
        h mn mx () demit).val.1 0).view Tw
      = famFreeze (fun t i => cqMachineFam pacing h mn mx demit t i)
          Tw 0 := rfl
  rw [hout, famFreeze_of_mono
    (fun t i => cqMachineFam_mono pacing h mn mx demit t i)]
  -- 2. The consumed pool at `Tw` is the full stabilized wire.
  obtain ⟨hne, hlast⟩ := tickSteps_getLast hpTw
  have hflat : (batchesFrom ((h 0).view)
      (tickSteps (pacing () 0) Tw) 0).flatten = (h 0).view T₀ := by
    cases hts : tickSteps (pacing () 0) Tw with
    | nil => exact absurd hts hne
    | cons s ss =>
      have hpw : List.Pairwise (· ≤ ·) (s :: ss) := by
        rw [← hts]; exact tickSteps_pairwise _ Tw
      rw [batchesFrom_flatten (h 0) s ss 0 hpw (Nat.zero_le _),
        List.drop_zero]
      have : (s :: ss).getLast (List.cons_ne_nil s ss) = Tw := by
        rw [← hlast, List.getLast_congr _ _ hts]
      rw [this]
      exact hstab.view_ge hT₀Tw
  -- 3. The crossing count: `k` is emitted (exactly once) in the scan.
  set batches := batchesFrom ((h 0).view)
    (tickSteps (pacing () 0) Tw) 0 with hbatches
  have hsum : (batches.map Multiset.ofList).sum
      = Multiset.ofList ((h 0).view T₀) := by
    rw [← ofList_flatten, hflat]
  have htrace : cqMachineTrace pacing h mn mx 0 Tw
      = scanAcrossTicksTrace (cqTick mn mx) CQState.init
          (batches.map Multiset.ofList) :=
    (scanAcrossTicksTrace_map Multiset.ofList (cqTick mn mx)
      batches CQState.init).symm
  have hcount := cq_run_count mn mx h1 hmm k
    (batches.map Multiset.ofList) (by rw [hsum]; exact hcap)
  rw [hsum, if_pos hq] at hcount
  have hmem : k ∈ (scanAcrossTicksTrace (cqTick mn mx) CQState.init
      (batches.map Multiset.ofList)).sum := by
    rw [← Multiset.count_pos, hcount]
    exact Nat.zero_lt_one
  rw [← htrace] at hmem
  obtain ⟨j, hj, hkj⟩ := mem_sum_index hmem
  -- 4. The legal claim at tick `j` carries `k` onto the wire.
  have hjlen : j < (demit 0).length := by
    have : (cqMachineTrace pacing h mn mx 0 Tw).length
        = (tickSteps (pacing () 0) Tw).length := by
      rw [cqMachineTrace, scanAcrossTicksTrace_length]
      exact batchesFrom_length ..
    omega
  have hclaim : (↑((demit 0)[j]) : Multiset K)
      = (cqMachineTrace pacing h mn mx 0 Tw)[j] :=
    hleg Tw j hj hjlen
  show k ∈ cqMachineFam pacing h mn mx demit Tw 0
  rw [cqMachineFam, emitLin_of_overlap _ _ (hleg Tw)]
  refine List.mem_flatten.mpr ⟨(demit 0)[j], ?_, ?_⟩
  · refine List.mem_take_iff_getElem.mpr ⟨j, by omega, rfl⟩
  · have : k ∈ (↑((demit 0)[j]) : Multiset K) := by
      rw [hclaim]; exact hkj
    exact Multiset.mem_coe.mp this

/-! ## Non-vacuity witnesses (D15)

A concrete run on a *non-trivially* fair schedule (ticks only at odd
steps): two `Ok` votes for key `7`, quorum `min = max = 2`, one honest
emission claim. The output wire is empty before the first tick and
carries `7` from the first tick on — the machine computes the `T` the
theorem promises. -/

private def wOdd : Unit → Fin 1 → Nat → Bool := fun _ _ t => t % 2 == 1

private def wResp : Fin 1 → StepHist (Nat × Except Nat Unit) :=
  fun _ => StepHist.const [(7, .ok ()), (7, .ok ())]

private def wOut : StepHist Nat :=
  (collect_quorum (SchedSem Unit (fun _ => 1) wOdd) ()
    wResp 2 2 () (fun _ => [[7]])).val.1 0

-- before the first tick: nothing on the wire
#guard wOut.view 0 = []
-- from the first (odd-step) tick on: the quorum key is out, and holds
#guard wOut.view 1 = [7]
#guard wOut.view 4 = [7]

/-- The single claim is honest at every horizon: whichever step the
first tick lands on, the first batch of a constant wire is its whole
content. -/
private theorem wLegal (t : Nat) :
    OverlapLegal [[7]] (cqMachineTrace wOdd wResp 2 2 0 t) := by
  intro j hj hl
  have hj0 : j = 0 := by
    have : j < 1 := hl
    omega
  subst hj0
  unfold cqMachineTrace at hj ⊢
  set tr0 := scanAcrossTicksTrace
    (fun s b => cqTick 2 2 s (Multiset.ofList b)) CQState.init
    (batchesFrom (wResp 0).view (tickSteps (wOdd () 0) t) 0) with htr
  suffices hopt : tr0[0]? = some (↑[7] : Multiset Nat) by
    rw [List.getElem?_eq_getElem hj] at hopt
    exact (Option.some.inj hopt).symm
  rw [htr]
  cases hts : tickSteps (wOdd () 0) t with
  | nil =>
    rw [htr, hts] at hj
    exact absurd hj (by simp [batchesFrom])
  | cons s ss =>
    show some ((cqTick 2 2 CQState.init
        (Multiset.ofList [(7, .ok ()), (7, .ok ())])).2) = _
    decide

/-- The premises of `cq_live` are jointly satisfiable on the witness
run (the theorem is not vacuous): every hypothesis is discharged
concretely, fairness non-trivially by the odd-tick skeleton. -/
example : ∃ T, 7 ∈ ((collect_quorum (SchedSem Unit (fun _ => 1) wOdd)
    () wResp 2 2 () (fun _ => [[7]])).val.1 0).view T :=
  cq_live (T₀ := 0) wResp (fun _ => rfl) 2 2 _ 7
    (by decide) (by decide) (by decide) (by decide)
    ⟨1, Nat.zero_le _, by decide, by decide⟩ wLegal

-- Axiom audit (expected: within Lean's standard three; migrate into
-- `AxCheck.lean` at the next edit of that file).
#print axioms cq_live
#print axioms deliver_fair_attains
#print axioms ticks_fair_attains

end HydroV2
