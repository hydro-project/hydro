import Hydro.KnotTactics
import Hydro.MonoRel

/-!
# Wf tactics: the corner-gadget projections and the tick-monotonicity step

Companions to `KnotTactics.lean` for the corner's `wf` obligations
(the `hstrict ∧ hchain ∧ hcplj` triple carried by `CoupleSem`'s
`fix_stream`/`fix_tick`; see `Couple.lean`). Same design laws as the
naming layer: defeq never crosses a knot/corner boundary (all
re-expression is syntactic `rw` with the body-naming lemmas), and
tick-monotonicity goals walk by head dispatch (`co_smono_step` — no
failing-branch `whnf`).

The generator (`HydroGen.lean`'s `_smono` phase, run by `hydro def`)
emits these scripts; everything they consume (body-naming lemmas,
per-op `tmono_*` laws, module names) is syntax-directed.
-/

namespace Hydro

open HydroSem

variable {L : Type} {mem : L → Nat}
  {pacing : Pacing L mem} {Tc Td : Nat}
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
    (v : MemberMap n (PoolCarrier α ord ret)) :
    (CoStream.probe2 (T := Tc) x v).sr = x := rfl

@[simp] theorem co_probe2_rr {n : Nat} {α : Type} [DecidableEq α]
    {ord : StrOrd} {ret : Retries} (x : Fin n → StepHist α)
    (v : MemberMap n (PoolCarrier α ord ret)) :
    (CoStream.probe2 (T := Tc) x v).rr = v := rfl

@[simp] theorem co_tick_probe2_sr {n : Nat} {σ : Type}
    (x : Fin n → Nat → Trace σ) (v : TickV n σ) :
    (CoTickSing.probe2 (T := Tc) x v).sr = x := rfl

@[simp] theorem co_tick_probe2_rr {n : Nat} {σ : Type}
    (x : Fin n → Nat → Trace σ) (v : TickV n σ) :
    (CoTickSing.probe2 (T := Tc) x v).rr = v := rfl

/-! ## The dispatch step -/

open Lean Elab Tactic Meta in
/-- The `TMono` laws per tick-producing machine op (`SchedMono.lean`;
the `_smono` phase's dispatch, D71). -/
private def coSMonoDispatch : List (Name × Name) := [
  (``HydroSem.snapshot, ``tmono_snapshot),
  (``HydroSem.batch, ``tmono_batch),
  (``HydroSem.firstTick, ``tmono_firstTick),
  (``HydroSem.mapTick, ``tmono_mapTick),
  (``HydroSem.zipTick, ``tmono_zipTick),
  (``HydroSem.defer_tick, ``tmono_defer_tick),
  (``HydroSem.flattenOrdered, ``tmono_flattenOrdered),
  (``HydroSem.flattenUnordered, ``tmono_flattenUnordered),
  (``HydroSem.tinto_keyed, ``tmono_tinto_keyed),
  (``HydroSem.tassume_ordering, ``tmono_tassume_ordering),
  (``HydroSem.reduce_watermark, ``tmono_reduce_watermark),
  (``HydroSem.ksnapshot, ``tmono_ksnapshot),
  (``HydroSem.tfold, ``tmono_tfold),
  (``HydroSem.time, ``tmono_time),
  (``HydroSem.snapshot_bounded, ``tmono_snapshot_bounded),
  (``HydroSem.tick_scan, ``tmono_tick_scan)]

/-! ## Shaped tick carriers: the shape of a wire/in-tick tuple type -/

open Lean Meta in
/-- The `TickShape` of a (possibly product) tick carrier type, and
whether it is a wire shape (`AtomicOf`/`Atomic`/`AtomicStream`: `true`)
or an in-tick shape (`BoundedOf`/`BoundedSingleton`/`BoundedStream`:
`false`). -/
partial def shapeOfCarrierType (ty : Expr) : MetaM (Option (Expr × Bool)) := do
  let ty ← instantiateMVars ty
  match ty.getAppFn with
  | .const n _ =>
    let args := ty.getAppArgs
    -- `AtomicOf {L} Tk TS TKS TKStr ℓ sh` / `BoundedOf BSing BStr BKS BKStr sh`
    if n == ``AtomicOf && args.size == 7 then
      return some (args[6]!, true)
    if n == ``BoundedOf && args.size == 5 then
      return some (args[4]!, false)
    if n == ``HydroSem.Atomic && args.size == 5 then
      return some (mkApp (mkConst ``TickShape.sing) args[4]!, true)
    if n == ``HydroSem.AtomicStream && args.size == 8 then
      return some (mkApp4 (mkConst ``TickShape.stream)
        args[4]! args[5]! args[6]! args[7]!, true)
    if n == ``HydroSem.BoundedSingleton && args.size == 4 then
      return some (mkApp (mkConst ``TickShape.sing) args[3]!, false)
    if n == ``HydroSem.BoundedStream && args.size == 7 then
      return some (mkApp4 (mkConst ``TickShape.stream)
        args[3]! args[4]! args[5]! args[6]!, false)
    if n == ``HydroSem.AtomicKeyedSingleton && args.size == 8 then
      return some (mkApp4 (mkConst ``TickShape.ksing)
        args[4]! args[5]! args[6]! args[7]!, true)
    if n == ``HydroSem.BoundedKeyedSingleton && args.size == 7 then
      return some (mkApp4 (mkConst ``TickShape.ksing)
        args[3]! args[4]! args[5]! args[6]!, false)
    if n == ``HydroSem.AtomicKeyedStream && args.size == 10 then
      return some (mkAppN (mkConst ``TickShape.kstream)
        #[args[4]!, args[5]!, args[6]!, args[7]!, args[8]!, args[9]!], true)
    if n == ``HydroSem.BoundedKeyedStream && args.size == 9 then
      return some (mkAppN (mkConst ``TickShape.kstream)
        #[args[3]!, args[4]!, args[5]!, args[6]!, args[7]!, args[8]!], false)
    if n == ``Prod && args.size == 2 then
      let some (a, ka) ← shapeOfCarrierType args[0]! | return none
      let some (b, _) ← shapeOfCarrierType args[1]! | return none
      return some (mkApp2 (mkConst ``TickShape.pair) a b, ka)
    return none
  | _ => return none

open Lean Meta in
/-- For a relation goal whose subject is `Prod.fst s`/`Prod.snd s` on a
shaped tick tuple: the pair shape `(a, b)` of `s`, the kind, and which
projection. -/
def projShapeOfSubject (subj : Expr) :
    MetaM (Option (Expr × Expr × Bool × Bool)) := do
  let subj ← instantiateMVars subj.headBeta
  -- the projection, as an application or a raw `.proj` node
  let (isFst, strct) ← do
    if subj.isAppOfArity ``Prod.fst 3 then pure (true, subj.getArg! 2)
    else if subj.isAppOfArity ``Prod.snd 3 then pure (false, subj.getArg! 2)
    else match subj with
      | .proj ``Prod 0 s => pure (true, s)
      | .proj ``Prod 1 s => pure (false, s)
      | _ => return none
  -- a projection of a literal pair reduces (the walkers' `dsimp`), it
  -- is not a tuple-shaped wire
  if strct.headBeta.isAppOfArity ``Prod.mk 4 then return none
  if (← whnfR strct).isAppOfArity ``Prod.mk 4 then return none
  -- the tuple's shape, from its type (whnf to the product if needed)
  let ty ← instantiateMVars (← inferType strct)
  let some (sh, k) ← (do
      if let some r ← shapeOfCarrierType ty then pure (some r)
      else
        let ty' ← whnf ty
        shapeOfCarrierType ty')
    | return none
  -- a pair shape, by construction
  unless sh.isAppOfArity ``TickShape.pair 2 do
    let sh' ← whnf sh
    unless sh'.isAppOfArity ``TickShape.pair 2 do return none
    return some (sh'.getArg! 0, sh'.getArg! 1, k, isFst)
  return some (sh.getArg! 0, sh.getArg! 1, k, isFst)

open Lean Meta in
/-- Apply `lem` to the goal with its two explicit `TickShape` arguments
pinned to `a` and `b` (every other argument a fresh metavariable, solved
by unification against the goal). -/
def applyWithShapes (g : MVarId) (lem : Name) (a b : Expr) :
    MetaM (List MVarId) := do
  let e ← mkConstWithFreshMVarLevels lem
  let ty ← inferType e
  let (mvars, bis, _) ← forallMetaTelescopeReducing ty
  let mut shapePos : Array Nat := #[]
  for i in [0:mvars.size] do
    if bis[i]! == .default then
      let mty ← inferType mvars[i]!
      if mty.isConstOf ``TickShape then shapePos := shapePos.push i
  unless shapePos.size ≥ 2 do
    throwError "applyWithShapes: {lem} has no two explicit shape arguments"
  unless ← isDefEq mvars[shapePos[0]!]! a do
    throwError "applyWithShapes: shape {a} does not fit {lem}"
  unless ← isDefEq mvars[shapePos[1]!]! b do
    throwError "applyWithShapes: shape {b} does not fit {lem}"
  let app := mkAppN e (mvars.extract 0 (shapePos[1]! + 1))
  g.apply app { synthAssignedInstances := false }

open Lean Elab Tactic Meta in
/-- One step of the tick-monotonicity walk (`TMono`/`TMonoOf` goals):
shaped projections step through the tuple's leafwise fact; a
`tick_scan` leaf restates in shaped form; every other head dispatches
to its `tmono_*` law. -/
elab "co_smono_step" : tactic => do
  let g ← getMainGoal
  let ty ← instantiateMVars (← g.getType)
  let args := ty.getAppArgs
  if args.size < 1 then
    throwError "co_smono_step: not a monotonicity goal"
  let lhs := args[args.size - 1]!.consumeMData.headBeta
  if let some (a, b, true, isFst) ← projShapeOfSubject lhs then
    liftMetaTactic fun g =>
      applyWithShapes g (if isFst then ``TMonoOf_fst else ``TMonoOf_snd) a b
    return
  let head := lhs.getAppFn
  let .const headName _ := head |
    throwError "co_smono_step: wire head is not a constant ({lhs})"
  if headName == ``HydroSem.tick_scan && ty.isAppOfArity ``TMono 3
      && lhs.getAppNumArgs == 10 then
    let outs := lhs.getArg! 6
    let g' ← g.withContext do
      let newTy ← mkAppM ``TMonoOf #[outs, lhs]
      g.change newTy
    replaceMainGoal [g']
    let lem := (coSMonoDispatch.lookup headName).get!
    liftMetaTactic fun g => do
      let e ← mkConstWithFreshMVarLevels lem
      g.apply e { synthAssignedInstances := false }
    return
  match coSMonoDispatch.lookup headName with
  | some lem =>
    liftMetaTactic fun g => do
      let e ← mkConstWithFreshMVarLevels lem
      g.apply e { synthAssignedInstances := false }
  | none =>
    throwError "co_smono_step: no dispatch entry for {headName}"

end Hydro
