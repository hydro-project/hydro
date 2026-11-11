import Hydro.CoupleProj

/-!
# Knot tactics

The tactics that prove a knot's `sr` and `rr` equations: the machine-run identity
`(knot @ CoupleSem …).sr = knot @ SchedSem …` and the denotational-run identity
`(knot @ CoupleSem …).rr = knot @ Values …` at the derived decisions. The knot generator
(`Hydro/HydroGenKnot.lean`) emits `K_co_sr₁`/`K_co_rr₁` with them, so each of a knot's `sr`
and `rr` equations is one tactic call.

Definitional equality never crosses a knot boundary: unfolding through `k` nested knots is
exponential in `k`, so a whole-program projection identity is out of reach past two knots
while a per-knot structural lemma is immediate. The tactics therefore work one knot at a
time, with definitional steps only at record scale (`congrArg₂ _ rfl` for the fuel fields) and
leaf scale (the trailing `rfl`s), and fold the body by rewriting with the caller's rules: the
body-level `sr`/`rr` equations and the inner knots' own lemmas.

* `co_knot_sr [defs…] [rules…]` unfolds the knot and its body definitions, rewrites the
  corner fix to the machine diagonal (`co_hfix_*_sr`, `co_*fixSrC_eq`), aligns the target's
  `H.fix` wrapper by `Eq.trans` with `sched_hfix_*'`, splits the knot boundary with
  `congrArg₂`/`funext`, and folds the body with `co_body_rw`.
* `co_knot_rr hsr [defs…] [rules…]` has the same skeleton and in addition bridges the machine
  diagonal inside the denotational iterate with the knot's own `sr` equation `hsr`, passed
  fully applied.

Two facts about the rewriting fix its shape. The alignment of the `H.fix` wrapper goes
through `Eq.trans` because a higher-order `rw` cannot match a constant body (a hoisted
`pcSeqF`). The rules are applied by a `rw` loop and not by `simp`, because `simp` does not
rewrite module projections nested in dependent-subtype argument positions even when the
instance is reducibly definitionally equal; each `rw` consumes one instantiation, so the
`repeat first` loop drains the inner-knot rules, in the caller's order ahead of the built-in
closers.
-/

namespace Hydro

open Lean Elab Tactic

/-- `repeat (first | rw [r₁] | … | rw [rₙ] | rw [closer₁] | …)` from the caller's rule list
followed by the probe projection closers. Used by the knot tactics; also usable standalone to
fold a body whose inner knots already have their `sr`/`rr` equations in the rule list. -/
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

/-- The knot's `sr` equation, from its body rules; see the module header. `defs` must
contain the knot definition and every body definition between the fix and the projections
the rules rewrite (the knot's hoisted `<M>.<wire>`/`<M>.body` constants). -/
macro "co_knot_sr" "[" defs:ident,* "]" "[" rules:term,* "]" : tactic => do
  let ds := defs.getElems
  `(tactic| (
    unfold $[$ds]*
    -- target side: the `H.fix` wrapper to `fix_stream`/`fix_tick`, by transitivity
    first
      | refine Eq.trans ?_ (sched_hfix_stream' _ _ _).symm
      | refine Eq.trans ?_ (sched_hfix_tick' _ _ _).symm
    -- corner side: project the fix to the machine diagonal
    first
      | rw [co_hfix_stream_sr, co_fixSrC_eq]
      | rw [co_hfix_tick_sr, co_tick_fixSrC_eq]
    -- the knot boundary: fuel fields by record-scale defeq, then one generic iterate
    refine congrArg₂ _ rfl ?_
    funext x
    co_body_rw [$rules,*]
    try simp only [co_schedC_sr, co_tick_schedC_sr]
    try with_reducible rfl
    try exact rfl))

/-- The knot's `rr` equation, from the knot's `sr` equation (`hsr`, fully applied)
and its body rules; see the module header. -/
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
    -- bridge the machine diagonal inside the denotational iterate
    rw [h]
    co_body_rw [$rules,*]
    try simp only [co_schedC_sr, co_tick_schedC_sr]
    try with_reducible rfl
    try exact rfl))

end Hydro
