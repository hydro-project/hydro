import HydroV2.CoupleProj
import HydroV2.EagerProj

/-!
# Knot tactics: the structural naming recipe, packaged

The D39 measurements (`FINDINGS.md`) established the law: kernel/
elaborator defeq through `k` nested knots is exponential in `k` —
whole-program projection identities are unprovable past two knots
(80+ min budget-blown on the 5-knot paxos stack), while per-knot
structural lemmas land in seconds. This file packages the per-knot
recipe — proven by hand five times in `Paxos/CoupleKnots.lean` — as
tactic macros, so a knot's naming lemma is one line:

* `co_knot_sr [defs…] [rules…]` — the machine-leg identity of one
  knot: `(knot @ CoupleSem …).sr = knot @ SchedSem …`. Unfolds the
  knot (and its body defs), rewrites the corner fix to the machine
  diagonal (`co_hfix_*_sr`, `co_*fixSrC_eq`), aligns the target's
  `H.fix` wrapper by `Eq.trans` (never by higher-order `rw` — constant
  bodies like a hoisted `pcSeqF` defeat `rw`'s pattern matcher), splits
  the knot boundary with `congrArg₂`/`funext`, and folds the body with
  the caller's projection rules (body-level namings + inner-knot
  lemmas) plus the standard probe/sched closers.

* `co_knot_rr hsr [defs…] [rules…]` — the reader-leg identity:
  `(knot @ CoupleSem …).rr = knot @ Values (derived decisions) …`.
  Same skeleton; additionally bridges the machine diagonal inside the
  reader iterate via the knot's own machine-leg lemma `hsr` (passed
  fully applied).

The recipe's invariants, honored here so callers never re-learn them:

* **defeq never crosses a knot boundary** — the only defeq steps are
  record-scale (`congrArg₂ _ rfl`: fuel legs) and leaf-scale (the
  trailing `rfl`s);
* **rules are `rw`-looped, not `simp`ed** — `simp` will not rewrite
  module projections nested in contract-typed (dependent subtype)
  argument positions even when the instance is reducibly defeq;
* **inner-knot rules repeat** — each `rw` consumes one instantiation;
  the `repeat first` loop drains them all, `rr` rules before `sr`
  rules (the caller's list order is preserved ahead of the built-in
  closers).
-/

namespace HydroV2

open Lean Elab Tactic

/-- Build `repeat (first | rw [r₁] | … | rw [rₙ] | rw [closer₁] | …)`
from the caller's rule list followed by the standard corner closers
(probe projections). Used by the knot macros; also useful standalone
when folding a body whose knots are already named. -/
macro "co_body_rw" "[" rules:term,* "]" : tactic => do
  let closers : Array (TSyntax `tactic) := #[
    ← `(tactic| rw [co_probeC_rr]),
    ← `(tactic| rw [co_probeC_sr]),
    ← `(tactic| rw [co_tick_probeC_rr]),
    ← `(tactic| rw [co_tick_probeC_sr])]
  let userRws ← rules.getElems.mapM fun r => `(tactic| rw [$r:term])
  let alts := userRws ++ closers
  let failTac ← `(tactic| fail "co_body_rw: no rule applies")
  let alt ← alts.foldrM (init := failTac) fun t acc =>
    `(tactic| first | $t:tactic | $acc:tactic)
  `(tactic| repeat $alt:tactic)

/-- The machine-leg knot lemma, from its body rules. See module
docstring. `defs` must contain the knot def and every body def between
the fix and the projections the rules rewrite (e.g. `[leFails, leCore]`
or `[pcSeqF]`). -/
macro "co_knot_sr" "[" defs:ident,* "]" "[" rules:term,* "]" : tactic => do
  let ds := defs.getElems
  `(tactic| (
    unfold $[$ds]*
    -- target side: `H.fix` wrapper → `fix_stream`/`fix_tick`, by
    -- transitivity (higher-order `rw` cannot match constant bodies)
    first
      | refine Eq.trans ?_ (sched_hfix_stream' _ _ _).symm
      | refine Eq.trans ?_ (sched_hfix_tick' _ _ _).symm
    -- corner side: project the fix to the machine diagonal
    first
      | rw [co_hfix_stream_sr, co_fixSrC_eq]
      | rw [co_hfix_tick_sr, co_tick_fixSrC_eq]
    -- the knot boundary: fuel legs by record-scale defeq, then one
    -- generic iterate
    refine congrArg₂ _ rfl ?_
    funext x
    co_body_rw [$rules,*]
    try simp only [co_schedC_sr, co_tick_schedC_sr]
    try with_reducible rfl
    try exact rfl))

/-- The reader-leg knot lemma, from the knot's machine-leg lemma
(`hsr`, fully applied) and its body rules. See module docstring. -/
macro "co_knot_rr" hsr:term:max "[" defs:ident,* "]" "[" rules:term,* "]" :
    tactic => do
  let ds := defs.getElems
  `(tactic| (
    have h := $hsr
    unfold $[$ds]* at h ⊢
    first
      | rw [co_hfix_stream_sr] at h
      | rw [co_hfix_tick_sr] at h
    first
      | refine Eq.trans ?_ (values_hfix_stream' _ _ _).symm
      | refine Eq.trans ?_ (values_hfix_tick' _ _ _).symm
    first
      | rw [co_hfix_stream_rr]
      | rw [co_hfix_tick_rr]
    refine congrArg₂ _ rfl ?_
    funext v
    -- bridge the machine diagonal inside the reader iterate
    rw [h]
    co_body_rw [$rules,*]
    try simp only [co_schedC_sr, co_tick_schedC_sr]
    try with_reducible rfl
    try exact rfl))

end HydroV2
