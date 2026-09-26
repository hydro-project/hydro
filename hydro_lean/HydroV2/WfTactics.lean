import HydroV2.KnotTactics
import HydroV2.MonoRel

/-!
# Wf tactics: exposing and dispatching the knot wf triple

Companions to `KnotTactics.lean` for the corner's `wf` obligations
(the `hcaus ∧ hchain ∧ hcplj` triple carried by `CoupleSem`'s
`fix_stream`/`fix_tick`; see `Couple.lean` and the validated pattern
`cc_wf` in `CoupleCheck.lean`). Same design laws as the naming layer:
defeq never crosses a knot/corner boundary (all re-expression is
syntactic `rw` with the body-naming lemmas), and causality goals walk
by head dispatch (`causal_step` — no failing-branch `whnf`).

These macros are also the codegen templates for the planned
`hydro def` elaborator: everything they consume (body-naming lemmas,
per-op causal lemmas, module names) is syntax-directed.
-/

namespace HydroV2

open HydroSem

variable {L : Type} {mem : L → Nat}
  {pacing : (ℓ : L) → Fin (mem ℓ) → Nat → Bool} {Tc Td : Nat}
  {hjT : Tc ≤ Td}

/-! ## Corner-gadget projections (syntactic collapse for the machine
re-expression) -/

@[simp] theorem co_schedEmbed_sr {n : Nat} {α : Type} [DecidableEq α]
    {ord : StrOrd} {ret : Retries} (x : Fin n → StepHist α) :
    (CoStream.schedEmbed (T := Tc) (ord := ord) (ret := ret) x).sr
      = x := rfl

@[simp] theorem co_tick_schedEmbed_sr {n : Nat} {σ : Type}
    (x : Fin n → Nat → Trace σ) :
    (CoTickSing.schedEmbed (T := Tc) x).sr = x := rfl

@[simp] theorem co_probe2_sr {n : Nat} {α : Type} [DecidableEq α]
    {ord : StrOrd} {ret : Retries} (x : Fin n → StepHist α)
    (v : Fin n → PoolCarrier α ord ret) :
    (CoStream.probe2 (T := Tc) x v).sr = x := rfl

@[simp] theorem co_probe2_rr {n : Nat} {α : Type} [DecidableEq α]
    {ord : StrOrd} {ret : Retries} (x : Fin n → StepHist α)
    (v : Fin n → PoolCarrier α ord ret) :
    (CoStream.probe2 (T := Tc) x v).rr = v := rfl

@[simp] theorem co_tick_probe2_sr {n : Nat} {σ : Type}
    (x : Fin n → Nat → Trace σ) (v : TickV n σ .unbounded) :
    (CoTickSing.probe2 (T := Tc) x v).sr = x := rfl

@[simp] theorem co_tick_probe2_rr {n : Nat} {σ : Type}
    (x : Fin n → Nat → Trace σ) (v : TickV n σ .unbounded) :
    (CoTickSing.probe2 (T := Tc) x v).rr = v := rfl

/-- Causality transfers along a pointwise naming of the body: to show
`f` causal it suffices to name `f = g` pointwise (the body-naming
lemma at `schedEmbed`-embedded wires — definitional, body-scale) and
walk `g` (the `SchedSem`-spelled body). Fully generic in the
agreement relation. -/
theorem causal_of_eq {γ δ : Type} {f g : γ → δ}
    {R : Nat → γ → γ → Prop} {S : Nat → δ → δ → Prop}
    (hfg : ∀ x, f x = g x)
    (hg : ∀ h x y, R h x y → S h (g x) (g y)) :
    ∀ h x y, R h x y → S h (f x) (f y) := by
  intro h x y hxy
  rw [hfg x, hfg y]
  exact hg h x y hxy

/-! ## The dispatch step (instance-flexible variant)

`causal_step`'s applies synthesize instance arguments and require
assignment (`synthAssignedInstances := true`), which fails on wires
whose element types carry custom `DecidableEq` instances (e.g. the
P1b reply pairs' `p1bPairDecEq`). This variant unifies them instead. -/

open Lean Elab Tactic Meta in
private def coCausalDispatch : List (Name × Name) := [
  (``HydroSem.map, ``causal_map),
  (``HydroSem.filterMap, ``causal_filterMap),
  (``HydroSem.broadcast, ``causal_broadcast),
  (``HydroSem.demux, ``causal_demux),
  (``HydroSem.values, ``causal_values),
  (``HydroSem.weaken_retries, ``causal_weaken_retries),
  (``HydroSem.union, ``causal_union),
  (``HydroSem.assume_ordering, ``causal_assume_ordering),
  (``HydroSem.fold, ``causal_fold),
  (``HydroSem.fold_monotone, ``causal_fold_monotone),
  (``HydroSem.snapshot, ``causal_snapshot),
  (``HydroSem.batch, ``causal_batch),
  (``HydroSem.batch_ordered, ``causal_batch_ordered),
  (``HydroSem.assume_ordering_batch, ``causal_assume_ordering_batch),
  (``HydroSem.mapBatchWith, ``causal_mapBatchWith),
  (``HydroSem.mapBatch, ``causal_mapBatch),
  (``HydroSem.mapBatchesWith, ``causal_mapBatchesWith),
  (``HydroSem.filterMapBatchesWith, ``causal_filterMapBatchesWith),
  (``HydroSem.scan_batches_across_ticks,
    ``causal_scan_batches_across_ticks),
  (``HydroSem.fold_batches_across_ticks_monotone,
    ``causal_fold_batches_across_ticks_monotone),
  (``HydroSem.scan_batches_unordered_across_ticks,
    ``causal_scan_batches_unordered_across_ticks),
  (``HydroSem.scan_batches_unordered, ``causal_scan_batches_unordered),
  (``HydroSem.scan_batches_unordered₂,
    ``causal_scan_batches_unordered₂),
  (``HydroSem.scan_across_ticks, ``causal_scan_across_ticks),
  (``HydroSem.mapTick, ``causal_mapTick),
  (``HydroSem.zipTick, ``causal_zipTick),
  (``HydroSem.fold_across_ticks_monotone,
    ``causal_fold_across_ticks_monotone),
  (``HydroSem.mapMonotone, ``causal_mapMonotone),
  (``HydroSem.forgetBound, ``causal_forgetBound),
  (``HydroSem.defer, ``causal_defer),
  (``HydroSem.mapBatchesUnordered, ``causal_mapBatchesUnordered),
  (``HydroSem.emitBatches, ``causal_emitBatches),
  (``HydroSem.emitMultisetBatches, ``causal_emitMultisetBatches),
  (``HydroSem.emitBatchesUnordered, ``causal_emitBatchesUnordered),
  (``HydroSem.timeout_snapshot, ``causal_timeout_snapshot),
  (``HydroSem.source_interval_batch, ``causal_source_interval_batch),
  (``HydroSem.sample_every, ``causal_sample_every),
  (``HydroSem.allTicks, ``causal_allTicks),
  (``HydroSem.fix_stream, ``causal_fix_stream),
  (``HydroSem.fix_tick, ``causal_fix_tick)]

open Lean Elab Tactic Meta in
elab "co_causal_step" : tactic => do
  let g ← getMainGoal
  let ty ← instantiateMVars (← g.getType)
  let args := ty.getAppArgs
  if args.size < 2 then
    throwError "co_causal_step: not an agreement goal"
  let lhs := args[args.size - 2]!
  let head := lhs.getAppFn
  let .const headName _ := head |
    throwError "co_causal_step: left wire head is not a constant"
  match coCausalDispatch.lookup headName with
  | some lem =>
    liftMetaTactic fun g => do
      let e ← mkConstWithFreshMVarLevels lem
      g.apply e { synthAssignedInstances := false }
  | none =>
    throwError "co_causal_step: no dispatch entry for {headName}"

/-! ## The exposure macro -/

/-- Expose a knot's wf triple: unfold the knot defs, push `.wf`
through the ops (`co_wf_simp`), split. Leaves goals `hcaus`, `hchain`,
`hcplj` (in that order) plus any residual input-wf conjuncts (closed
by the ambient wf hypotheses via `assumption`). -/
macro "co_knot_wf" "[" defs:ident,* "]"
    "[" ids:Lean.Parser.Tactic.simpLemma,* "]" : tactic => do
  let ds := defs.getElems
  `(tactic| (
    unfold $[$ds]*
    co_wf_simp [HydroSem.fix, HydroSem.fixTick, $ids,*]
    refine ⟨?hcaus, ?hchain, ?hcplj⟩))

/-- The `hcaus` closer. `nm` is the pointwise body-naming function —
a lambda `fun x => <body-naming lemma at (CoStream.schedEmbed x)>`
(the caller instantiates the D40 body lemma; `causal_of_eq` consumes
it, so no rewriting ever crosses the corner boundary). `defs` are the
`SchedSem`-side defs to open (the hoisted body); the walker then
dispatches per op head. Per-knot causality facts already in context
close `causal_fix_*` recursion hypotheses by `assumption`. -/
macro "co_causal" nm:term:max "[" defs:ident,* "]"
    "[" ids:Lean.Parser.Tactic.simpLemma,* "]" : tactic => do
  let ds := defs.getElems
  `(tactic| (
    refine causal_of_eq $nm ?_
    intro h x y hxy
    simp only [co_schedEmbed_sr, co_tick_schedEmbed_sr]
    unfold $[$ds]*
    dsimp only
    repeat'
      first
      | with_reducible assumption
      | with_reducible exact SAgree.refl _ _
      | with_reducible exact TAgree.refl _ _
      | with_reducible exact KAgree.refl _ _
      | with_reducible exact FAgree.refl _ _
      | with_reducible exact VAgree.refl _ _
      | (with_reducible refine SAgree.mono ?_ (by assumption); omega)
      | (with_reducible refine TAgree.mono ?_ (by assumption); omega)
      | (with_reducible refine KAgree.mono ?_ (by assumption); omega)
      | (with_reducible refine FAgree.mono ?_ (by assumption); omega)
      | co_causal_step
      | causal_step [$ids,*]))

end HydroV2
