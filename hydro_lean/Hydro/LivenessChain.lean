import Hydro.Liveness

/-!
# Hydro · liveness over decision chains (prototype)

**Behaviors, denotationally, are ascending chains in decision space.**

The rung-1 prototype (`Liveness.lean`, D48) proved `cq_live` *at the
machine*: fairness as predicates on schedule parameters, the proof
walking `StepHist` views. That works, but it re-does protocol
reasoning machine-side — exactly what the house architecture forbids
at scale. This file is the corrected frame:

- A machine run at horizon `T` corresponds (via the corner) to a
  **derived decision** `d(T)`, and derivation is horizon-monotone. So
  an infinite behavior *denotes an ascending chain in decision
  space*, and the temporal operators live over **chains**, not steps:
  `◇P := ∃ chain point satisfying P`, `◇□`, `□◇`, `⟿` are quantifier
  shapes along the chain (`ChEventually`/`ChEvAlways`/…).
- **Fairness is a class of chains**, stated in decision vocabulary:
  WF(tick) becomes *exhaustion* (the cut chain eventually consumes
  the whole pool); ◇□-stability premises become *chain-component
  stabilization* (`ChStabilizes`); SF becomes a coupling between an
  enabledness trajectory and an action trajectory (`ChSF`) — all
  predicates on chains and their induced `Values` runs, no schedule
  vocabulary.
- **All protocol reasoning happens at `Values`, along the chain**
  (`cq_complete_mem`, `cq_chain_live`, `cq_chain_live_stable` below —
  their proofs touch only the colocated contract).
- The machine-side residue is exactly **two generic kits**, proven
  per-operator, never per-program:
  1. *chain-fairness transfer* — a fair schedule's derived chain is a
     fair chain (`cqDerivedChain_isCutChain`,
     `cqDerivedChain_complete_at`, `cqDerivedChain_exhausts`);
  2. *tightness* — the machine observation at `T` IS the `Values` run
     at `d(T)` (`collect_quorum_tight`), read through the corner's own
     body coupling (`collect_quorum_co_wf₁` + `CoTick.scan_comm` +
     `corr_batch`), never through a hand mirror of the block.
- The headline `cq_live` (statement unchanged since D48) is proven by
  gluing the `Values` chain theorem through the kits: the quorum
  crossing argument is consumed **only** through the contract's
  faces. Its D48 machine-walk proof against a hand register mirror
  is gone with the mirror (D65); `cq_live_chain` names the same
  theorem.

## Why this is not the classical "fairness isn't denotational" trap

The 1980s obstruction (fair merge is not Scott-continuous) rules out
folding fairness into the semantic *domain* — fairness of a behavior
is not determined by its finite prefixes. We do not do that: the
semantics stays finite-horizon and fueled; fairness is a predicate
**on chains** in the logic above the semantics, and liveness
conclusions are ∃-chain-point shaped, so finite points witness them.
Only ω-limit statements (progress under unbounded load — rung (c) of
the ladder) would touch completed decisions, and that is exactly the
already-queued lfp upgrade.

See `LIVENESS.md` for the revised architecture, the TLA+ Rosetta
re-anchored to chains (incl. how leader change and lossy-links-with-
retries land), and the open forks.
-/

namespace Hydro

/-! ## Temporal combinators over chain positions

A chain is `ℕ`-indexed; a *trajectory property* is `P : ℕ → Prop`
about the denotation at each chain point. The TLA+ operators are
quantifier shapes over positions. -/

/-- `◇P`: some chain point satisfies `P`. -/
def ChEventually (P : Nat → Prop) : Prop := ∃ n, P n

/-- `□P`: every chain point satisfies `P`. -/
def ChAlways (P : Nat → Prop) : Prop := ∀ n, P n

/-- `◇□P`: from some point on, `P` holds. -/
def ChEvAlways (P : Nat → Prop) : Prop := ∃ n, ∀ m, n ≤ m → P m

/-- `□◇P`: `P` holds at arbitrarily late points. -/
def ChAlwEventually (P : Nat → Prop) : Prop := ∀ n, ∃ m, n ≤ m ∧ P m

/-- `P ⟿ Q`: every `P`-point is followed by a `Q`-point. -/
def ChLeadsTo (P Q : Nat → Prop) : Prop := ∀ n, P n → ∃ m, n ≤ m ∧ Q m

theorem ChEvAlways.alwEventually {P : Nat → Prop}
    (h : ChEvAlways P) : ChAlwEventually P := by
  obtain ⟨n, hn⟩ := h
  intro m
  exact ⟨max n m, Nat.le_max_right _ _, hn _ (Nat.le_max_left _ _)⟩

theorem ChAlwEventually.eventually {P : Nat → Prop}
    (h : ChAlwEventually P) : ChEventually P :=
  (h 0).imp fun _ hm => hm.2

theorem ChEventually.mono {P Q : Nat → Prop} (h : ChEventually P)
    (hpq : ∀ n, P n → Q n) : ChEventually Q :=
  h.imp hpq

theorem ChLeadsTo.trans {P Q R : Nat → Prop} (hpq : ChLeadsTo P Q)
    (hqr : ChLeadsTo Q R) : ChLeadsTo P R := by
  intro n hp
  obtain ⟨m, hnm, hq⟩ := hpq n hp
  obtain ⟨o, hmo, hr⟩ := hqr m hq
  exact ⟨o, hnm.trans hmo, hr⟩

theorem ChEventually.leadsTo {P Q : Nat → Prop} (h : ChEventually P)
    (hpq : ChLeadsTo P Q) : ChEventually Q := by
  obtain ⟨n, hp⟩ := h
  exact (hpq n hp).imp fun _ hm => hm.2

/-! ## Fairness vocabulary over chains

The general shapes (clients today and tomorrow):

- `ChStabilizes` — a chain *component* is eventually constant. This
  is the ◇□-stability premise shape: "eventually one leader" is
  `ChStabilizes` of the election-relevant component of the derived
  decision chain (see `LIVENESS.md` — the leader-change story).
- `ChSF` — strong fairness as a coupling between an enabledness
  trajectory and an action trajectory (`□◇Enabled ⟹ □◇Taken`). Its
  first real client is lossy links with retries, which also needs the
  lossy-cursor model fork (`LIVENESS.md`); the vocabulary is ready. -/

/-- ◇□-stabilization of a chain component. -/
def ChStabilizes {β : Type _} (f : Nat → β) : Prop :=
  ∃ n b, ∀ m, n ≤ m → f m = b

/-- Strong fairness between two trajectories: recurring enabledness
forces recurring occurrence. -/
def ChSF (Enabled Taken : Nat → Prop) : Prop :=
  ChAlwEventually Enabled → ChAlwEventually Taken

/-! ## Cut chains (the `batch` decision component)

For the `collect_quorum` exemplar the relevant decision component is
the cut decision (`Values.BatchDec` = realized batch increments). A
*cut chain* is what a machine behavior denotes at a batch site:
prefix-ascending (later horizons extend the consumed history) and
legal (every point passes the denotational guards verbatim — derived
decisions always do). -/

variable {α : Type} [DecidableEq α]

/-- An ascending, legal chain of cut decisions over pool `v`. -/
structure IsCutChain (v : Multiset α) (c : Nat → List (Multiset α)) :
    Prop where
  mono : ∀ n, c n <+: c (n + 1)
  legal : ∀ n, batchCuts v 0 (c n) = c n

theorem IsCutChain.mono_le {v : Multiset α}
    {c : Nat → List (Multiset α)} (hc : IsCutChain v c) {n m : Nat}
    (h : n ≤ m) : c n <+: c m := by
  induction m with
  | zero => cases Nat.le_zero.mp h; exact List.prefix_refl _
  | succ m ih =>
    rcases Nat.lt_or_ge n (m + 1) with hlt | hge
    · exact (ih (Nat.lt_succ_iff.mp hlt)).trans (hc.mono m)
    · cases Nat.le_antisymm h hge; exact List.prefix_refl _

/-- **WF(tick), denotationally**: the chain eventually consumes the
whole pool. This is the *denotational shadow* of tick fairness — no
schedule vocabulary. -/
def Exhausts (v : Multiset α) (c : Nat → List (Multiset α)) : Prop :=
  ChEventually fun n => cqConsumed v (c n) = v

omit [DecidableEq α] in
/-- Multiset-list sums split over append (manual: keeps instance
resolution trivial). -/
private theorem msum_append (a b : List (Multiset α)) :
    (a ++ b).sum = a.sum + b.sum := by
  induction a with
  | nil => rw [List.nil_append, List.sum_nil, Multiset.zero_add]
  | cons x a ih =>
    rw [List.cons_append, List.sum_cons, List.sum_cons, ih,
      Multiset.add_assoc]

/-- Along a legal ascending chain, the consumed pool only grows — so
exhaustion is persistent: `◇complete = ◇□complete`. -/
theorem IsCutChain.consumed_persists {v : Multiset α}
    {c : Nat → List (Multiset α)} (hc : IsCutChain v c) {n m : Nat}
    (hnm : n ≤ m) (hn : cqConsumed v (c n) = v) :
    cqConsumed v (c m) = v := by
  refine le_antisymm (cqConsumed_le _ _) ?_
  have hsum : cqConsumed v (c n) ≤ cqConsumed v (c m) := by
    show (batchCuts v 0 (c n)).sum ≤ (batchCuts v 0 (c m)).sum
    rw [hc.legal n, hc.legal m]
    obtain ⟨e, he⟩ := hc.mono_le hnm
    rw [← he, msum_append]
    exact Multiset.le_add_right _ _
  rwa [hn] at hsum

/-! ## The `Values` chain theorems (all protocol content lives here)

Schedule-free, decision-quantified: the quorum crossing argument is
consumed **only** through the colocated contract (`emit_count`). -/

variable {K E : Type} [DecidableEq K] [DecidableEq E]

/-- **The point lemma (V)**: at any *complete* cut decision — one
whose consumed pool is the whole pool — a key with a quorum of `Ok`
votes is in the `Values` output. Pure contract reasoning. -/
theorem cq_complete_mem (v : Fin 1 → Multiset (K × Except E Unit))
    (d : (Values Unit (fun _ => 1)).BatchDec 1 (K × Except E Unit))
    (mn mx : Nat) (k : K) (h1 : 1 ≤ mn) (hmm : mn ≤ mx)
    (hcomplete : cqConsumed (v 0) (d 0) = v 0)
    (hcap : cqKeyCount (v 0) k ≤ mx)
    (hq : mn ≤ cqOkCount (v 0) k) :
    k ∈ (collect_quorum (Values Unit (fun _ => 1)) () v mn mx
      d).1 0 := by
  have hens := collect_quorum.ensures (L := Unit) (mem := fun _ => 1) () v mn mx
    d
  have hcount := hens.emit_count 0 k h1 hmm
    (by rw [hcomplete]; exact hcap)
  rw [hcomplete, if_pos hq] at hcount
  rw [← Multiset.count_pos, hcount]
  exact Nat.zero_lt_one

/-- **`◇committed` at `Values`**: along any exhausting cut chain, the
quorum key is eventually in the output. -/
theorem cq_chain_live (v : Fin 1 → Multiset (K × Except E Unit))
    (c : Nat → (Values Unit (fun _ => 1)).BatchDec 1
      (K × Except E Unit))
    (mn mx : Nat) (k : K) (h1 : 1 ≤ mn) (hmm : mn ≤ mx)
    (hcap : cqKeyCount (v 0) k ≤ mx)
    (hq : mn ≤ cqOkCount (v 0) k)
    (hex : Exhausts (v 0) (fun n => c n 0)) :
    ChEventually fun n =>
      k ∈ (collect_quorum (Values Unit (fun _ => 1)) () v mn mx
        (c n)).1 0 :=
  hex.mono fun n hn => cq_complete_mem v (c n) mn mx k h1 hmm hn hcap hq

/-- **`◇□committed` at `Values`**: along a *legal ascending* exhausting
chain the commitment persists — the ◇/◇□ collapse for upward-closed
observations, now at the denotation. -/
theorem cq_chain_live_stable (v : Fin 1 → Multiset (K × Except E Unit))
    (c : Nat → (Values Unit (fun _ => 1)).BatchDec 1
      (K × Except E Unit))
    (mn mx : Nat) (k : K) (h1 : 1 ≤ mn) (hmm : mn ≤ mx)
    (hcap : cqKeyCount (v 0) k ≤ mx)
    (hq : mn ≤ cqOkCount (v 0) k)
    (hchain : IsCutChain (v 0) (fun n => c n 0))
    (hex : Exhausts (v 0) (fun n => c n 0)) :
    ChEvAlways fun n =>
      k ∈ (collect_quorum (Values Unit (fun _ => 1)) () v mn mx
        (c n)).1 0 := by
  obtain ⟨n₀, hn₀⟩ := hex
  refine ⟨n₀, fun m hm => ?_⟩
  exact cq_complete_mem v (c m) mn mx k h1 hmm
    (hchain.consumed_persists hm hn₀) hcap hq

/-! ## Kit 1: chain-fairness transfer (fairness lives here, only here)

The derived decision chain of a machine behavior, and the per-op facts
that a fair schedule's derived chain is a fair chain. Generic in the
wire; proven once. -/

/-- The derived cut chain of a machine wire under a tick skeleton:
horizon `T ↦` the realized batch increments up to `T`. This is the
denotational trajectory the corner extracts from the run. -/
def cqDerivedChain (hi : StepHist α) (p : Nat → Bool)
    (T : Nat) : List (Multiset α) :=
  (batchesFrom hi.view (tickSteps p T) 0).map
    (fun b => Multiset.ofList b)

omit [DecidableEq α] in
/-- A stabilized wire's views are all below the stabilized pool. -/
theorem stab_view_le {hi : StepHist α} {T₀ : Nat}
    (hstab : StabilizesAt hi T₀) (t : Nat) :
    Multiset.ofList (hi.view t) ≤ Multiset.ofList (hi.view T₀) := by
  rcases le_total t T₀ with h | h
  · exact Multiset.coe_le.mpr (hi.mono_le h).sublist.subperm
  · rw [hstab.view_ge h]

/-- The derived chain is an ascending, legal cut chain (no fairness
needed — every machine behavior denotes a chain). -/
theorem cqDerivedChain_isCutChain {hi : StepHist α} {T₀ : Nat}
    (hstab : StabilizesAt hi T₀) (p : Nat → Bool) :
    IsCutChain (Multiset.ofList (hi.view T₀)) (cqDerivedChain hi p) := by
  refine ⟨fun n => ?_, fun n => ?_⟩
  · obtain ⟨e, he⟩ := batchesFrom_prefix hi.view (tickSteps_prefix p n) 0
    refine ⟨e.map (fun b => Multiset.ofList b), ?_⟩
    show (batchesFrom hi.view (tickSteps p n) 0).map
        (fun b => Multiset.ofList b) ++ _
      = (batchesFrom hi.view (tickSteps p (n + 1)) 0).map
          (fun b => Multiset.ofList b)
    rw [← List.map_append, he]
  · have := corr_batch (fun _ : Fin 1 => p) n (fun _ => hi)
      (fun _ => Multiset.ofList (hi.view T₀))
      (fun _ => stab_view_le hstab n) 0
    exact this

omit [DecidableEq α] in
/-- The consume-all telescope at a specific fair tick: the flattened
batches up to a tick at/after stabilization are the whole wire. -/
theorem tickSteps_flatten_stab {hi : StepHist α} {T₀ : Nat}
    (hstab : StabilizesAt hi T₀) {p : Nat → Bool} {Tw : Nat}
    (hT : T₀ ≤ Tw) (hpT : p Tw = true) :
    (batchesFrom hi.view (tickSteps p Tw) 0).flatten = hi.view T₀ := by
  obtain ⟨hne, hlast⟩ := tickSteps_getLast hpT
  cases hts : tickSteps p Tw with
  | nil => exact absurd hts hne
  | cons s ss =>
    have hpw : List.Pairwise (· ≤ ·) (s :: ss) := by
      rw [← hts]; exact tickSteps_pairwise p Tw
    rw [batchesFrom_flatten hi s ss 0 hpw (Nat.zero_le _),
      List.drop_zero]
    have : (s :: ss).getLast (List.cons_ne_nil s ss) = Tw := by
      rw [← hlast, List.getLast_congr _ _ hts]
    rw [this]
    exact hstab.view_ge hT

/-- **Chain-fairness transfer, pointwise**: at any tick at/after
stabilization, the derived chain point is *complete*. -/
theorem cqDerivedChain_complete_at {hi : StepHist α} {T₀ : Nat}
    (hstab : StabilizesAt hi T₀) {p : Nat → Bool} {Tw : Nat}
    (hT : T₀ ≤ Tw) (hpT : p Tw = true) :
    cqConsumed (Multiset.ofList (hi.view T₀)) (cqDerivedChain hi p Tw)
      = Multiset.ofList (hi.view T₀) := by
  show (batchCuts _ 0 _).sum = _
  rw [(cqDerivedChain_isCutChain hstab p).legal Tw]
  show ((batchesFrom hi.view (tickSteps p Tw) 0).map
    (fun b => Multiset.ofList b)).sum = _
  rw [← ofList_flatten, tickSteps_flatten_stab hstab hT hpT]

/-- **Chain-fairness transfer**: a fair schedule's derived chain
exhausts — WF(tick) becomes the denotational fairness class. -/
theorem cqDerivedChain_exhausts {hi : StepHist α} {T₀ : Nat}
    (hstab : StabilizesAt hi T₀) {p : Nat → Bool}
    (hp : FairTicks p) :
    Exhausts (Multiset.ofList (hi.view T₀)) (cqDerivedChain hi p) := by
  obtain ⟨Tw, hT, hpT⟩ := hp T₀
  exact ⟨Tw, cqDerivedChain_complete_at hstab hT hpT⟩

/-! ## Kit 2: tightness (the machine observation *is* the `Values` run
at the derived chain point)

The machine's output wire at horizon `T` is, as a multiset, the `Values`
run of the module at the DERIVED cut chain point `d(T)` — read through
the corner, never through a hand mirror of the block: the corner's `wf`
for the module (`collect_quorum_co_wf₁`) is the body's leg-independence
and well-formedness (`CoTick.scanWf`), under which the machine's tick
former coerced IS the denotation's on the coerced inputs
(`CoTick.scan_comm`), and the derived batch decision is exact at its own
horizon (`corr_batch`). This is the first instance of the generic
tightness kit rung 2 generalizes (FINDINGS D65). -/

/-- **Tightness for `collect_quorum`**: the machine output at `T`, read
as a multiset, is the `Values` output at the derived chain point. -/
theorem collect_quorum_tight {pacing : Unit → Fin 1 → Nat → Bool}
    (h : Fin 1 → StepHist (K × Except E Unit)) {T₀ : Nat}
    (hstab : StabilizesAt (h 0) T₀) (mn mx : Nat) (T : Nat) :
    Multiset.ofList (((collect_quorum (SchedSem Unit (fun _ => 1) pacing) ()
        h mn mx ()).1 0).view T)
      = (collect_quorum (Values Unit (fun _ => 1)) ()
          (fun _ => Multiset.ofList ((h 0).view T₀)) mn mx
          (fun i => cqDerivedChain (h i) (pacing () i) T)).1 0 := by
  -- every view of the (one) member sits below the stabilized pool
  have hc : ∀ (T' : Nat) (i : Fin 1),
      Multiset.ofList ((h i).view T')
        ≤ (fun _ : Fin 1 => Multiset.ofList ((h 0).view T₀)) i := fun T' i => by
    obtain rfl : i = 0 := Subsingleton.elim _ _
    exact stab_view_le hstab T'
  -- the corner at horizon `T`: its `wf` IS the block's body coupling
  -- (no body is respelled — `change` reads the conditions off the
  -- instantiation), its machine leg is the machine run, its denotation
  -- leg is the `Values` run at the derived cuts (`co_transfer`, the
  -- generator's own projection walk, at the EXPLICIT derived decision)
  have hwf := collect_quorum_co_wf₁ (L := Unit) (mem := fun _ => 1) (pacing := pacing)
    (Tc := T) (Td := T) (hjT := Nat.le_refl T) ()
    (CoStream.inputC (ord := .noOrder) (ret := .exactlyOnce) h _ hc) mn mx
    (CoDec.batch _) trivial
  change CoTick.scanWf _ _ _ _ _ at hwf
  obtain ⟨-, heoS, -, heoO, h1, h2, h3⟩ := hwf
  have hsr : (collect_quorum (CoupleSem Unit (fun _ => 1) pacing T T (Nat.le_refl T)) ()
        (CoStream.inputC (ord := .noOrder) (ret := .exactlyOnce) h _ hc) mn mx
        (CoDec.batch _)).1.sr
      = (collect_quorum (SchedSem Unit (fun _ => 1) pacing) () h mn mx ()).1 :=
    collect_quorum_co_sr₁ (pacing := pacing) (Tc := T) (Td := T) (hjT := Nat.le_refl T) ()
      (CoStream.inputC (ord := .noOrder) (ret := .exactlyOnce) h _ hc) mn mx (CoDec.batch _)
  have hrr : (collect_quorum (CoupleSem Unit (fun _ => 1) pacing T T (Nat.le_refl T)) ()
        (CoStream.inputC (ord := .noOrder) (ret := .exactlyOnce) h _ hc) mn mx
        (CoDec.batch _)).1.rr
      = (collect_quorum (Values Unit (fun _ => 1)) ()
          (fun _ => Multiset.ofList ((h 0).view T₀)) mn mx
          (fun i => cqDerivedChain (h i) (pacing () i) T)).1 := by
    co_transfer [collect_quorum]
  rw [← hsr, ← hrr]
  -- the machine leg: the machine former on the realized batches, per
  -- step, flattened onto the wire; the denotation leg: the denotation
  -- former on the derived cuts, summed
  change Multiset.ofList (famFreeze (fun t i =>
      (scanAcrossTicksTrace
        (CoTick.gS (L := Unit) (mem := fun _ => 1) (ℓ := ())
          (.pair (.stream (K × Except E Unit) inferInstance .noOrder .exactlyOnce)
            (.stream K inferInstance .noOrder .exactlyOnce))
          (.stream (K × Except E Unit) inferInstance .noOrder .exactlyOnce)
          (.stream K inferInstance .noOrder .exactlyOnce) _ i)
        (SchedTick.seed (.pair (.stream (K × Except E Unit) inferInstance .noOrder .exactlyOnce)
            (.stream K inferInstance .noOrder .exactlyOnce)) ((), ()))
        (batchesFrom ((h i).view) (tickSteps (pacing () i) t) 0)).flatten) T 0)
    = (scanAcrossTicksTrace
        (CoTick.gR (L := Unit) (mem := fun _ => 1) (ℓ := ())
          (.pair (.stream (K × Except E Unit) inferInstance .noOrder .exactlyOnce)
            (.stream K inferInstance .noOrder .exactlyOnce))
          (.stream (K × Except E Unit) inferInstance .noOrder .exactlyOnce)
          (.stream K inferInstance .noOrder .exactlyOnce) _ (0 : Fin 1))
        (ValuesTick.seed (.pair (.stream (K × Except E Unit) inferInstance .noOrder .exactlyOnce)
            (.stream K inferInstance .noOrder .exactlyOnce)) ((), ()))
        (batchCuts (Multiset.ofList ((h 0).view T₀)) 0
          (cqDerivedChain (h 0) (pacing () 0) T))).sum
  rw [famFreeze_of_mono, ofList_flatten, (cqDerivedChain_isCutChain hstab (pacing () 0)).legal T]
  swap
  · exact fun t i => flatten_prefix (scanAcrossTicksTrace_prefix _ _
      (batchesFrom_prefix _ (tickSteps_prefix _ t) 0))
  congr 1
  have := CoTick.scan_comm _ _ _ _ heoS heoO h1 h2 h3 0
    (batchesFrom ((h 0).view) (tickSteps (pacing () 0) T) 0) (SchedTick.seed _ ((), ()))
  rw [CoTick.coerce_seed] at this
  exact this

/-! ## The headline: `collect_quorum` liveness through the chain frame

`cq_live` is the rung-1 theorem — statement unchanged since D48 — now
proven ONLY this way: the `Values` point lemma carries all protocol
content (faces only), the chain-fairness kit is generic, tightness is
the corner's own body coupling. The D48 machine-walk proof (over the
hand mirror `cqTickS`) is gone with the mirror (D65). -/

/-- `collect_quorum` liveness: `Values` point lemma (`cq_complete_mem`)
∘ chain-fairness transfer (`cqDerivedChain_complete_at`) ∘ tightness
(`collect_quorum_tight`). -/
theorem cq_live {pacing : Unit → Fin 1 → Nat → Bool}
    (h : Fin 1 → StepHist (K × Except E Unit)) {T₀ : Nat}
    (hstab : StabilizesAt (h 0) T₀)
    (mn mx : Nat) (k : K)
    (h1 : 1 ≤ mn) (hmm : mn ≤ mx)
    (hcap : cqKeyCount (Multiset.ofList ((h 0).view T₀)) k ≤ mx)
    (hq : mn ≤ cqOkCount (Multiset.ofList ((h 0).view T₀)) k)
    (hsupply : ∃ Tw, T₀ ≤ Tw ∧ pacing () 0 Tw = true) :
    ∃ T, k ∈ ((collect_quorum (SchedSem Unit (fun _ => 1) pacing) ()
      h mn mx ()).1 0).view T := by
  obtain ⟨Tw, hT₀Tw, hpTw⟩ := hsupply
  refine ⟨Tw, ?_⟩
  -- (V): the `Values` run at the (complete) derived chain point has `k`.
  have hval := cq_complete_mem
    (fun _ => Multiset.ofList ((h 0).view T₀))
    (fun i => cqDerivedChain (h i) (pacing () i) Tw) mn mx k h1 hmm
    (cqDerivedChain_complete_at hstab hT₀Tw hpTw) hcap hq
  -- (tightness): that run IS the machine's wire at `Tw`.
  rw [← collect_quorum_tight h hstab mn mx Tw] at hval
  exact Multiset.mem_coe.mp hval

/-- The chain-factored name (D50) — the same theorem. -/
theorem cq_live_chain {pacing : Unit → Fin 1 → Nat → Bool}
    (h : Fin 1 → StepHist (K × Except E Unit)) {T₀ : Nat}
    (hstab : StabilizesAt (h 0) T₀)
    (mn mx : Nat) (k : K)
    (h1 : 1 ≤ mn) (hmm : mn ≤ mx)
    (hcap : cqKeyCount (Multiset.ofList ((h 0).view T₀)) k ≤ mx)
    (hq : mn ≤ cqOkCount (Multiset.ofList ((h 0).view T₀)) k)
    (hsupply : ∃ Tw, T₀ ≤ Tw ∧ pacing () 0 Tw = true) :
    ∃ T, k ∈ ((collect_quorum (SchedSem Unit (fun _ => 1) pacing) ()
      h mn mx ()).1 0).view T :=
  cq_live h hstab mn mx k h1 hmm hcap hq hsupply

/-! ## Non-vacuity (D15): the premises are jointly satisfiable on a
non-trivially fair witness (`wOdd`: every other step), the derived
chain's fairness is computed concretely, and the machine computes the
`T` the theorem promises. -/

private def wOdd : Unit → Fin 1 → Nat → Bool := fun _ _ t => t % 2 == 1

private def wResp : Fin 1 → StepHist (Nat × Except Nat Unit) :=
  fun _ => StepHist.const [(7, .ok ()), (7, .ok ())]

private def wOut : StepHist Nat :=
  (collect_quorum (SchedSem Unit (fun _ => 1) wOdd) ()
    wResp 2 2 ()).1 0

-- before the first tick: nothing on the wire
#guard wOut.view 0 = []
-- from the first (odd-step) tick on: the quorum key is out, and holds
#guard wOut.view 1 = [7]
#guard wOut.view 4 = [7]

-- the derived chain exhausts at the first odd tick, concretely:
#guard cqConsumed (Multiset.ofList ((wResp 0).view 0))
    (cqDerivedChain (wResp 0) (wOdd () 0) 1)
  = Multiset.ofList ((wResp 0).view 0)
-- and before it, nothing is consumed:
#guard cqConsumed (Multiset.ofList ((wResp 0).view 0))
    (cqDerivedChain (wResp 0) (wOdd () 0) 0)
  = (0 : Multiset (Nat × Except Nat Unit))

/-- The premises of `cq_live` are jointly satisfiable on the witness. -/
example : ∃ T, 7 ∈ ((collect_quorum
    (SchedSem Unit (fun _ => 1) wOdd) () wResp 2 2 ()).1 0).view T :=
  cq_live (T₀ := 0) wResp (fun _ => rfl) 2 2 7
    (by decide) (by decide) (by decide) (by decide)
    ⟨1, Nat.zero_le _, by decide⟩

-- Axiom audits (expected: within Lean's standard three).
#print axioms collect_quorum_tight
#print axioms cq_chain_live
#print axioms cq_chain_live_stable
#print axioms cq_live
#print axioms cq_live_chain

end Hydro
