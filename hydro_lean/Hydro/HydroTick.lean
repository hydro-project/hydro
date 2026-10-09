import Hydro.HydroDef

/-!
# The `tick` construct (Rust `sliced!`/`use::state` — D58/D59/D60)

A `tick` block declares per-location loop state (`use::state` —
registers `(state r : σ := seed)` and persisted streams
`(state s : H.BoundedStream …)`, Rust's `state_null`; none for a
stateless per-tick computation — Rust's tick-level stream operators),
the wires it consumes tick-by-tick, and a PER-TICK body over THIS tick's
values — written with the in-tick operators (`H.bmap`, `H.bcount`,
`H.bsZip`, …) over `H.BoundedSingleton`/`H.BoundedStream` values, as
the Rust sliced body is; `rebind` gives the next states, `emit` the
tick's singleton emissions (wires `H.Ticked ℓ _`), `yield` the tick's
stream emissions (`yield_atomic`: wires `H.TickStream ℓ _ _ _`). An
optional Verus-style loop invariant `P(emissions-so-far, states…,
spectators…)` carries value-level cross-iteration facts; its
obligations trail the program:

    tick (state r₁ : τ₁ := seed₁) … (state sₖ : H.BoundedStream α ord ret) …
      (input x₁ := <wire₁>) … (input xₘ := <wireₘ>)
      (invariant (<out> <r₁> … <rₙ> <spectators…>) => P)? :=
      <per-tick let-chain over r₁ … rₙ x₁ … xₘ (this tick's values)>
      rebind (r₁ := e₁, …, rₙ := eₙ)
      emit (a := …, …)? yield (b := …, …)?
      (prove init := <P at [] seeds>, tick := <one-tick step>)?;
    <continuation — the xⱼ (the input wires) and the outputs in scope>

Reads of `rᵢ` are the PREVIOUS tick's value and the cycle's unit lag
is the construct's own shape (Hydro compiles no instantaneous cycle),
so the denotation is a STRUCTURAL fold over the location's ticks —
`HydroSem.tick_scan`, the construct's semantic former (one field for
every arity: states, inputs and emissions are SHAPES, `TickShape`,
zipped at the type level). The elaboration is a plain syntax transform
— no operator analysis; the only type-directed step is reading each
input wire's kind (`Ticked` ↦ `.sing`, `TickStream` ↦ `.stream`) and
each state's kind (`BoundedStream` ↦ `.stream`, else `.sing`) off its
type; the output shape's structure comes from the `emit`/`yield`
keywords, its leaf parameters are holes unification fills from the
body:

    let x₁ := <wire₁> … let xₘ := <wireₘ>
    let <out₁>_step := fun me st inpt =>
      let r₁ := st.1 …                        -- the states (a shaped tuple)
      let x₁ := inpt.1 …                      -- this tick's inputs
      <body>
      ((e₁, (…)), (a, (b, …)))                -- next states, emissions
    let <out₁>_r := H.tick_scan <sts> <ins> <outs> (x₁, …) <out₁>_step (seed₁, …)
    let a := <out₁>_r.1 …                     -- the output wires
    ghost have h<out₁>_inv : ∀ i, P (<out₁> i) (states…) (specs…) :=
      fun i => scanAcrossTicks_invariant … (init) (tick)
    <continuation>

The invariant obligations are the generic structural induction's two
premises (`scanAcrossTicks_invariant`): `init` = P at the empty run,
`tick` = ONE loop body step (`P out states → P (out ++ [emission])
states'` with the step values definitional at `Values`, where every
in-tick operator IS its value). v2: the invariant clause is available
on single-output blocks (one `emit` or one `yield` leg — the fold's
output wire is the fold trace itself there) over any number of inputs
(the invariant and its obligations see the ZIPPED input trace, the
tuple the body sees) and any states — registers or persisted streams
(D65: a stream state's binder is typed by the user at the `Values`
carrier, its seed is the grade's bottom through `ValuesTick.seed`).
`fix` + `base`/`step` remains the construct
for async-lag cycles (`forward_ref` through the network — induction
over exchange rounds, not ticks). -/

namespace Hydro

namespace HydroTick

/-- A state binder: `(state r : σ := seed)` — a register (Rust
`Singleton<σ, Tick, Bounded>` `use::state`), or
`(state s : H.BoundedStream α ord ret)` — a persisted stream (Rust
`Stream<…, Tick, Bounded>` `use::state`/`state_null`, seeded empty, so no
seed is written). The kind is read off the type (`BoundedStream` ↦
`.stream`, anything else ↦ `.sing`). -/
syntax tickState := atomic("(" "state ") ident " : " term (" := " term)? ")"

/-- A consumed input: `(input x := <wire>)` — the block runs once per
tick; inside the body, `x` is THIS tick's value (a `BoundedSingleton`
for a `Ticked` wire, a `BoundedStream` for a `TickStream` wire). -/
syntax tickInput := atomic("(" &"input ") ident " := " term ")"

/-- An invariant binder: `x` or `(x : τ)` (typed binders are needed when
the predicate's body cannot infer the type on its own — at `Values`
the fold's output elements are `BoundedSingleton τ`, only
definitionally `τ`). -/
syntax tickInvBinder := ident <|> ("(" ident " : " term ")")
/-- The loop invariant — predicate only; its obligations trail the
program in the block tail's `prove` clause (Verus order). -/
syntax tickInvariant := atomic("(" &"invariant ") "(" tickInvBinder+ ")" " => " term ")"

/-- The block tail: next states, the emissions, and the trailing
obligations, followed by the continuation. -/
syntax (name := tickTail)
  withPosition(("rebind " "(" (ident " := " term),+ ")")?
    (&"emit " "(" (ident " := " term),+ ")")?
    (&"yield " "(" (ident " := " term),+ ")")?
    ("prove " &"init " ":= " term ", " "tick " ":= " term)?)
    optSemicolon(term) : term

/-- The `tick` block. -/
syntax (name := tickTerm)
  withPosition("tick " tickState* tickInput+ (tickInvariant)?
    " := " term) : term

open Lean Parser Elab Term Meta

/-- Replace the (unique) `tickTail` marker in a syntax tree, returning
the found node (marker-level surgery only — the construct never
inspects operator applications). -/
partial def replaceTail (s : Syntax) (new : Syntax) :
    Syntax × Option Syntax :=
  if s.isOfKind ``tickTail then
    (new, some s)
  else
    match s with
    | .node info kind args => Id.run do
      let mut args := args
      let mut found := none
      for i in [0:args.size] do
        if found.isNone then
          let (a', f?) := replaceTail args[i]! new
          if f?.isSome then
            args := args.set! i a'
            found := f?
      (.node info kind args, found)
    | _ => (s, none)

@[term_elab tickTail] def elabTickTail : TermElab := fun stx _ =>
  throwErrorAt stx "tick: `rebind` outside a `tick` block"

/-- Right-nested pair of terms: `(t₁, (t₂, t₃))`. -/
def nestPairs (ts : Array (TSyntax `term)) : TermElabM (TSyntax `term) := do
  let mut t := ts.back!
  for u in ts.reverse.toList.drop 1 do
    t ← `(($u, $t))
  pure t

/-- Right-nested pair SHAPE of shapes. -/
def nestShapes (ts : Array (TSyntax `term)) : TermElabM (TSyntax `term) := do
  let mut t := ts.back!
  for u in ts.reverse.toList.drop 1 do
    t ← `(TickShape.pair $u $t)
  pure t

/-- The `k`-th component of a right-nested `n`-tuple, as projections. -/
def nestProj (t : TSyntax `term) (k n : Nat) : TermElabM (TSyntax `term) := do
  if n == 1 then return t
  let mut cur := t
  for _ in [0:k] do
    cur ← `(($cur).2)
  if k + 1 < n then `(($cur).1) else pure cur

/-- A declared state: name, written type, shape (`.sing σ` /
`.stream …`), whether it is a stream, and its seed (`()` for a stream). -/
structure StateInfo where
  id : Ident
  ty : TSyntax `term
  shape : TSyntax `term
  isStream : Bool
  seed : TSyntax `term
  deriving Inhabited


/-- The syntax pieces `mkRegGhost` reads off the construct's state. -/
structure RegCtx where
  out0Id : Ident
  stepId : Ident
  iId : Ident
  nId : Ident
  eId : Ident
  seedVal : TSyntax `term
  /-- the output legs, their `_at` reader names, and their read clauses
  over the abstract register -/
  outIds : Array Ident
  atNames : Array Ident
  regAtRhss : Array (TSyntax `term)
  inpTrace : TSyntax `term
  stBefore : TSyntax `term
  tickIds : Array Ident
  tickReads : Array (TSyntax `term)
  tickTuple : TSyntax `term
  inputIds : Array Ident
  stateTys : Array (TSyntax `term)

/-- Reassemble one zip read from component reads. -/
def zipReadOf (hs : Array (TSyntax `term)) : TermElabM (TSyntax `term) := do
  let mut t := hs.back!
  for h in hs.reverse.toList.drop 1 do
    t ← `(Trace.getElem?_zip_eq_some'.mpr ⟨$h, $t⟩)
  pure t

/-- `h<out>_reg` (stateful blocks): the register ∃-ABSTRACTED — `∃ reg,
reg i 0 = seed ∧ (read: tick n's emission is the step on `reg i n`) ∧
(step: `reg i (n+1)` is the step's state on a read) ∧ (stall: once an
input ends the register freezes) [∧ (the invariant before every tick,
over `reg i n`)]`. Consumers `ghost obtain ⟨reg, …⟩` it and never see the
fold (FINDINGS D63 `hlog_pool`, D64 E7). Returns (name, statement, proof). -/
def mkRegGhost (c : RegCtx) (inv? : Option (TSyntax `term × TSyntax `term)) :
    TermElabM (Ident × TSyntax `term × TSyntax `term) := do
  let regName := mkIdent (Name.mkSimple ("h" ++ c.out0Id.getId.toString ++ "_reg"))
  let regId := mkIdent `reg
  let iId := c.iId; let nId := c.nId; let eId := c.eId; let stepId := c.stepId
  let stTyAll : TSyntax `term ← do
    if c.stateTys.isEmpty then `(Unit)
    else if c.stateTys.size == 1 then pure c.stateTys[0]!
    else do
      let mut t : TSyntax `term := c.stateTys.back!
      for ty in c.stateTys.reverse.toList.drop 1 do
        t ← `($ty × $t)
      pure t
  -- the zipped input's `none` read from one component's `none`
  let zipNone : Nat → TSyntax `term → TermElabM (TSyntax `term) := fun k h => do
    let m := c.inputIds.size
    if m == 1 then pure h
    else do
      let mut t := h
      if k + 1 < m then t ← `(Trace.getElem?_zip_eq_none_left $t)
      for _ in [0:k] do
        t ← `(Trace.getElem?_zip_eq_none_right $t)
      pure t
  -- the statement: one read clause per output leg
  let regAts : Array (TSyntax `term) ← (c.outIds.zip c.regAtRhss).mapM fun (oId, rhs) =>
    `(∀ $iId:ident ($nId : Nat) ($eId : _), ($oId $iId)[$nId]? = some $eId ↔ $rhs)
  let regSucc ← do
    let tickBinders : Array (TSyntax ``Parser.Term.bracketedBinder) ←
      c.tickIds.mapM fun t => `(bracketedBinder| ($t:ident : _))
    let core ← `($regId $iId ($nId + 1) = (($stepId $iId) ($regId $iId $nId) $(c.tickTuple)).1)
    `(∀ $iId:ident ($nId : Nat) $tickBinders*,
      $(← c.tickReads.foldrM (fun r acc => `($r → $acc)) core))
  let regStalls : Array (TSyntax `term) ← c.inputIds.mapM fun xId =>
    `(∀ $iId:ident ($nId : Nat), ($xId $iId)[$nId]? = none →
      $regId $iId ($nId + 1) = $regId $iId $nId)
  let regZero ← `(∀ $iId:ident, $regId $iId 0 = $(c.seedVal))
  let clauses : Array (TSyntax `term) :=
    #[regZero] ++ regAts ++ #[regSucc] ++ regStalls ++ (inv?.map (·.1)).toArray
  let body ← clauses.pop.foldrM (fun cl acc => `($cl ∧ $acc)) clauses.back!
  let stmt ← `(∃ $regId:ident : Fin _ → Nat → $stTyAll, $body)
  -- the proof: the fold's own register, each clause a `Trace` lemma
  let hTickIds : Array Ident := c.tickIds.map fun t =>
    mkIdent (t.getId.appendAfter "_h")
  let succPrf ← do
    let read ← zipReadOf (hTickIds.map fun h => (⟨h⟩ : TSyntax `term))
    let binders : Array Ident := #[iId, nId] ++ c.tickIds ++ hTickIds
    `(fun $binders* =>
      scanAcrossTicksState_take_succ? ($stepId $iId) $(c.seedVal) $(c.inpTrace) $read)
  let hnId := mkIdent `hnone
  let stallPrfs : Array (TSyntax `term) ←
    (List.range c.inputIds.size).toArray.mapM fun k => do
      let z ← zipNone k ⟨hnId⟩
      `(fun $iId:ident $nId:ident $hnId:ident =>
        scanAcrossTicksState_take_stall ($stepId $iId) $(c.seedVal) $(c.inpTrace) $z)
  let comps : Array (TSyntax `term) :=
    #[← `(fun $iId:ident => rfl)] ++ (c.atNames.map fun a => (⟨a⟩ : TSyntax `term))
      ++ #[succPrf] ++ stallPrfs ++ (inv?.map (·.2)).toArray
  let prf ← `(⟨fun $iId:ident $nId:ident => $(c.stBefore), $comps,*⟩)
  pure (regName, stmt, prf)

/-- The `tick` elaborator: desugar to the shaped structural fold (see
the module docstring) and delegate — a syntax transform. -/
@[term_elab tickTerm] def elabTickTerm : TermElab :=
    fun stx expectedType? => do
  -- states: `(state r : σ := seed)` registers, `(state s : H.BoundedStream …)`
  -- persisted streams — the kind read off the written type
  let mut states : Array StateInfo := #[]
  for st in stx[1].getArgs do
    let rId : Ident := ⟨st[2]⟩
    let rTy : TSyntax `term := ⟨st[4]⟩
    let seed? : Option (TSyntax `term) :=
      if st[5].getArgs.isEmpty then none else some ⟨st[5][1]⟩
    let e ← withSynthesize (postpone := .yes) <| elabType rTy
    let ty ← whnfR (← instantiateMVars e)
    match ty.getAppFn.constName?, ty.getAppArgs with
    | some ``HydroSem.BoundedStream, #[_, _, _, α, inst, ord, ret] =>
      if seed?.isSome then
        throwErrorAt st "tick: a stream state is seeded empty — no `:= seed`"
      let shape ← exprToSyntax (mkApp4 (mkConst ``TickShape.stream) α inst ord ret)
      states := states.push ⟨rId, rTy, shape, true, ← `((() : Unit))⟩
    | _, _ =>
      let some seed := seed?
        | throwErrorAt st "tick: a register state needs its seed (`:= seed`)"
      states := states.push ⟨rId, rTy, ← `(TickShape.sing $rTy), false, ← `(($seed : $rTy))⟩
  -- inputs (≥ 1: the loop runs over their ticks)
  let inputs := stx[2].getArgs.map fun inp =>
    ((⟨inp[2]⟩ : Ident), (⟨inp[4]⟩ : TSyntax `term))
  if inputs.isEmpty then
    throwErrorAt stx "tick: at least one `(input x := …)` clause is \
      required (the block runs once per tick of its inputs)"
  let inv? : Option Syntax := stx[3].getArgs[0]?
  let bodyStx := stx[5]
  -- the interpretation variable, by type (found, not assumed)
  let lctx ← getLCtx
  let mut hId? : Option Ident := none
  for d in lctx do
    if !d.isImplementationDetail then
      if (← instantiateMVars d.type).getAppFn.constName?
          == some ``HydroSem then
        hId? := some (mkIdent d.userName)
  let some hId := hId?
    | throwErrorAt stx "tick: no `HydroSem` interpretation variable \
        in scope"
  -- input shapes, read off the wires' TYPES (`Ticked` ↦ `.sing`,
  -- `TickStream` ↦ `.stream`)
  let mut inShapes : Array (TSyntax `term) := #[]
  for (xId, wire) in inputs do
    let e ← withSynthesize (postpone := .yes) <| elabTerm wire none
    let ty ← instantiateMVars (← inferType e)
    let ty ← whnfR ty
    let shape : Expr ← do
      match ty.getAppFn.constName?, ty.getAppArgs with
      | some ``HydroSem.Ticked, #[_, _, _, _, τ] =>
        pure (mkApp (mkConst ``TickShape.sing) τ)
      | some ``HydroSem.TickStream, #[_, _, _, _, α, inst, ord, ret] =>
        pure (mkApp4 (mkConst ``TickShape.stream) α inst ord ret)
      | _, _ => throwErrorAt wire "tick: input `{xId}` must be a `Ticked` \
          or `TickStream` wire (its type is {ty})"
    inShapes := inShapes.push (← exprToSyntax shape)
  let insShape ← nestShapes inShapes
  -- the tail: rebinds (one per state, in order), the emissions, proofs
  let some tail := bodyStx.find? (·.isOfKind ``tickTail)
    | throwErrorAt stx "tick: the block body must end with \
        `rebind (…)? emit (…)` / `yield (…)`"
  -- tail = [rebind?, emit?, yield?, prove?, ;?, cont]
  let rebinds : Array (Ident × TSyntax `term) :=
    if tail[0].getArgs.isEmpty then #[]
    else tail[0][2].getSepArgs.map fun e =>
      ((⟨e[0]⟩ : Ident), (⟨e[2]⟩ : TSyntax `term))
  unless rebinds.size == states.size
      && (rebinds.zip states).all
          (fun (r, s) => r.1.getId == s.1.getId) do
    throwErrorAt tail "tick: `rebind` must assign every `state` of \
      this block, in declaration order \
      ({states.map (·.1.getId)}) — and only those"
  let emits : Array (Ident × TSyntax `term) :=
    if tail[1].getArgs.isEmpty then #[]
    else tail[1][2].getSepArgs.map fun e => ((⟨e[0]⟩ : Ident), (⟨e[2]⟩ : TSyntax `term))
  let yields : Array (Ident × TSyntax `term) :=
    if tail[2].getArgs.isEmpty then #[]
    else tail[2][2].getSepArgs.map fun e => ((⟨e[0]⟩ : Ident), (⟨e[2]⟩ : TSyntax `term))
  if emits.isEmpty && yields.isEmpty then
    throwErrorAt tail "tick: at least one `emit (x := …)` or \
      `yield (x := …)` leg is required"
  -- outputs: emits (singleton legs) then yields (stream legs); the
  -- shape's leaves are holes the body's type fills
  let outs := emits ++ yields
  let mut outShapes : Array (TSyntax `term) := #[]
  for _ in emits do outShapes := outShapes.push (← `(TickShape.sing _))
  for _ in yields do outShapes := outShapes.push (← `(TickShape.stream _ _ _ _))
  let outsShape ← nestShapes outShapes
  let prove? : Option (TSyntax `term × TSyntax `term) :=
    if tail[3].getArgs.isEmpty then none
    else some (⟨tail[3][3]⟩, ⟨tail[3][7]⟩)
  let contStx : TSyntax `term := ⟨tail[5]⟩
  -- the state shape: the states' shapes zipped; a stateless block
  -- carries the unit register
  let stsShape : TSyntax `term ←
    if states.isEmpty then `(TickShape.sing Unit)
    else nestShapes (states.map (·.shape))
  -- step result: (next states tuple, emissions tuple)
  let rebindTuple ←
    if states.isEmpty then `(($hId).bsPure ())
    else nestPairs (rebinds.map (·.2))
  let outTuple ← nestPairs (outs.map (·.2))
  let pairStx : TSyntax `term ← `(($rebindTuple, $outTuple))
  -- the per-tick body with the tail swapped for the step's result
  let (stepBodyRaw, _) := replaceTail bodyStx pairStx
  let stepBody : TSyntax `term := ⟨stepBodyRaw⟩
  -- the step function: registers and inputs opened into their names
  let meId := mkIdent `me
  let stId := mkIdent `st
  let inptId := mkIdent `inpt
  let stepLam : TSyntax `term ← do
    -- inputs: one binder, or projections of the tuple
    let withInputs : TSyntax `term ← do
      if inputs.size == 1 then pure stepBody
      else do
        let mut b := stepBody
        for k in (List.range inputs.size).reverse do
          let (xId, _) := inputs[k]!
          let pr ← nestProj (⟨inptId⟩ : TSyntax `term) k inputs.size
          b ← `(let $xId := $pr
                $b)
        pure b
    let inptBinder : Ident := if inputs.size == 1 then inputs[0]!.1 else inptId
    -- the in-tick carrier of a state: `H.BoundedSingleton σ` for a
    -- register, the written `H.BoundedStream …` for a stream
    let carrier : StateInfo → TermElabM (TSyntax `term) := fun st =>
      if st.isStream then pure st.ty else `(($hId).BoundedSingleton $(st.ty))
    -- states: none (unit), one binder, or projections of the tuple
    if states.isEmpty then
      `(fun $meId ($stId : ($hId).BoundedSingleton Unit) $inptBinder => $withInputs)
    else if states.size == 1 then
      let st := states[0]!
      let cty ← carrier st
      `(fun $meId ($(st.id) : $cty) $inptBinder => $withInputs)
    else
      let mut b := withInputs
      for k in (List.range states.size).reverse do
        let st := states[k]!
        let pr ← nestProj (⟨stId⟩ : TSyntax `term) k states.size
        let cty ← carrier st
        b ← `(let $(st.id) : $cty := $pr
              $b)
      `(fun $meId
          ($stId : BoundedOf ($hId).BoundedSingleton ($hId).BoundedStream $stsShape)
          $inptBinder => $b)
  -- the seeds, as the state shape's `SeedOf` tuple
  let seedTuple : TSyntax `term ← do
    if states.isEmpty then `((() : Unit))
    else nestPairs (states.map (·.seed))
  let out0Id := outs[0]!.1
  let stepId := mkIdent (out0Id.getId.appendAfter "_step")
  let resId := mkIdent (out0Id.getId.appendAfter "_r")
  let inTuple ← nestPairs (inputs.map fun (xId, _) => (⟨xId⟩ : TSyntax `term))
  -- the output wires, projected off the result tuple — ascribed with
  -- their LEAF carrier types (`H.Ticked _ _` / `H.TickStream _ _ _ _`,
  -- the holes solved by unification), so downstream type-directed
  -- machinery sees carrier heads, not `TickedOf … (.sing _)`
  let outLets : TSyntax `term → TermElabM (TSyntax `term) := fun cont => do
    let mut b := cont
    for k in (List.range outs.size).reverse do
      let (oId, _) := outs[k]!
      let pr ← nestProj (⟨resId⟩ : TSyntax `term) k outs.size
      let ty : TSyntax `term ←
        if k < emits.size then `(($hId).Ticked _ _)
        else `(($hId).TickStream _ _ _ _)
      b ← `(let $oId : $ty := $pr
            $b)
    pure b
  -- assemble. Every block gets its READERS as ghosts, per output leg
  -- (any-body truths, stated under the `Values` substitution where the
  -- fold's denotation is definitional): `h<out>_run` (the wire IS the
  -- structural fold of the step over the zipped inputs) and `h<out>_at`
  -- (tick `n`'s emission is the step on the register before `n` and
  -- the inputs' reads at `n`, option-indexed and destructured per input
  -- — the `[n]?` vocabulary of `Trace.lean`'s TickReads). An
  -- `invariant` clause adds `h<out>_inv` (the whole run) and
  -- `h<out>_inv_take` (every prefix: the register BEFORE a tick).
  let iId := mkIdent `i
  let nId := mkIdent `n
  -- the input trace at `Values`: one wire's trace, or the zip of the
  -- wires' traces in declaration order (`ValuesTick.slice` on the input
  -- shape, spelled directly)
  let inpTrace : TSyntax `term ← do
    let apps : Array (TSyntax `term) ← inputs.mapM fun (xId, _) =>
      `(($xId $iId))
    let mut t := apps.back!
    for u in apps.reverse.toList.drop 1 do
      t ← `(Trace.zip $u $t)
    pure t
  -- the register's seed at the denotation: the literal seeds for
  -- registers; through `ValuesTick.seed` when a stream state is present
  -- (its seed is the grade's bottom, not a written term)
  let seedVal : TSyntax `term ←
    if states.any (·.isStream) then `(ValuesTick.seed $stsShape $seedTuple)
    else pure seedTuple
  -- the register before tick `n`
  let stBefore : TSyntax `term ←
    `(scanAcrossTicksState ($stepId $iId) $seedVal (($inpTrace).take $nId))
  -- per-input tick reads: binder names and their `[n]?` hypotheses
  let tickIds : Array Ident := inputs.map fun (xId, _) =>
    mkIdent (xId.getId.appendAfter "_t")
  let tickReads : Array (TSyntax `term) ← (inputs.zip tickIds).mapM
    fun ((xId, _), tId) => `((($xId $iId)[$nId]? = some $tId))
  let tickTuple ← nestPairs (tickIds.map fun t => (⟨t⟩ : TSyntax `term))
  -- the simp set that reads the former at the denotation (one-step
  -- rewrite rules, never a definitional `show` of the zipped form —
  -- FINDINGS 0c-iii: that reduces the whole shaped former)
  let outIdents : Array Ident := outs.map (·.1)
  let outLemmas : Array (TSyntax `Lean.Parser.Tactic.simpLemma) ← outIdents.mapM fun o =>
    `(Lean.Parser.Tactic.simpLemma| $o:ident)
  let readerSimp : TSyntax `tactic ← `(tactic|
    simp only [$outLemmas,*, $resId:ident, ValuesTick.tick_scan_stream,
      ValuesTick.tick_scan_sing, ValuesTick.tick_scan_pair, ValuesTick.unslice_sing,
      ValuesTick.unslice_stream, ValuesTick.unslice_pair,
      ValuesTick.slice_pair, ValuesTick.slice_sing,
      ValuesTick.slice_stream, ValuesTick.seed_sing, ValuesTick.seed_pair])
  let eId := mkIdent `e
  let xId := mkIdent `x
  let hxId := mkIdent `hx
  -- the zip destructuring of one read `hx : Z[n]? = some x`, as the
  -- component reads (projections of `x`)
  let compReads : Array (TSyntax `term) ← do
    let m := inputs.size
    let mut out : Array (TSyntax `term) := #[]
    let mut cur : TSyntax `term := ⟨hxId⟩
    for k in [0:m] do
      if m == 1 then
        out := out.push cur
      else if k + 1 == m then
        out := out.push cur
      else
        out := out.push (← `((Trace.getElem?_zip_eq_some'.mp $cur).1))
        cur ← `((Trace.getElem?_zip_eq_some'.mp $cur).2)
    pure out
  let compProjs : Array (TSyntax `term) ← (List.range inputs.size).toArray.mapM
    fun k => nestProj (⟨xId⟩ : TSyntax `term) k inputs.size
  let hTickIds : Array Ident := tickIds.map fun t =>
    mkIdent (t.getId.appendAfter "_h")
  let mprPat : Array (TSyntax `rcasesPat) ←
    (tickIds ++ hTickIds).mapM fun t => `(rcasesPat| $t:ident)
  let mprRead ← zipReadOf (hTickIds.map fun h => (⟨h⟩ : TSyntax `term))
  -- the readers of one output leg (`k` of `outs.size`): `h<out>_run` (the
  -- wire IS the structural fold of the step over the zipped inputs,
  -- projected to the leg) and `h<out>_at` (tick `n`'s emission on the
  -- leg is the leg of the step on the register before `n` and the
  -- inputs' reads at `n`, option-indexed and destructured per input —
  -- the `[n]?` vocabulary of `Trace.lean`'s TickReads)
  let legReaders : Nat → TermElabM (Ident × TSyntax `term × TSyntax `term
      × Ident × TSyntax `term × TSyntax `term × TSyntax `term) := fun k => do
    let oId := outs[k]!.1
    let runName := mkIdent (Name.mkSimple ("h" ++ oId.getId.toString ++ "_run"))
    let atName := mkIdent (Name.mkSimple ("h" ++ oId.getId.toString ++ "_at"))
    let pId := mkIdent `p
    let legOf : TSyntax `term → TermElabM (TSyntax `term) := fun t =>
      nestProj t k outs.size
    let runStmt : TSyntax `term ←
      if outs.size == 1 then
        `(∀ $iId:ident, $oId $iId
            = scanAcrossTicksTrace ($stepId $iId) $seedVal $inpTrace)
      else
        `(∀ $iId:ident, $oId $iId
            = (scanAcrossTicksTrace ($stepId $iId) $seedVal $inpTrace).map
                (fun $pId => $(← legOf ⟨pId⟩)))
    let runPrf : TSyntax `term ← `(fun $iId:ident => by
      first
      | ($readerSimp:tactic; done)
      | rfl
      | ($readerSimp:tactic; rfl)
      | ($readerSimp:tactic; simp only [List.map_map]; rfl))
    let stepLeg ← legOf (← `((($stepId $iId) $stBefore $tickTuple).2))
    let atRhsOf : TSyntax `term → TermElabM (TSyntax `term) := fun leg => do
      let body ← `($eId = $leg)
      let conj ← tickReads.foldrM (fun r acc => `($r ∧ $acc)) body
      let mut t := conj
      for tId in tickIds.reverse do
        t ← `(∃ $tId:ident, $t)
      pure t
    let atRhs ← atRhsOf stepLeg
    let atStmt : TSyntax `term ←
      `(∀ $iId:ident ($nId : Nat) ($eId : _),
          ($oId $iId)[$nId]? = some $eId ↔ $atRhs)
    let mpWitness : Array (TSyntax `term) :=
      compProjs ++ compReads ++ #[← `(rfl)]
    let hrdId := mkIdent `hrd
    let hpId := mkIdent `hp
    let atPrf : TSyntax `term ←
      if outs.size == 1 then
        -- the wire's `[n]?` is the fold's (`_run`, instances up to the
        -- denotation's unfolding — closed by defeq, never by keyed `rw`),
        -- then the fold's read is the step on the register before `n` and
        -- the zipped input's read, split into the component reads
        `(fun $iId:ident $nId:ident $eId:ident => by
          have $hrdId := scanAcrossTicksTrace_getElem?_eq_some ($stepId $iId) $seedVal
            $inpTrace $nId $eId
          refine Iff.trans ?_ (Iff.trans $hrdId ?_)
          · rw [$runName:ident $iId]
            try exact Iff.rfl
          · constructor
            · rintro ⟨$xId:ident, $hxId:ident, rfl⟩
              exact ⟨$mpWitness,*⟩
            · rintro ⟨$mprPat,*, rfl⟩
              exact ⟨$tickTuple, $mprRead, rfl⟩)
      else
        -- the leg's `[n]?` is the mapped fold's: a tuple read, projected
        let heId := mkIdent `he
        let hpId' := mkIdent `hp'
        let qId := mkIdent `q
        let mpWitness' : Array (TSyntax `term) := compProjs ++ compReads ++
          #[← `(($heId).trans (congrArg (fun $qId => $(← legOf ⟨qId⟩)) $hpId'))]
        let mprPat' : Array (TSyntax `rcasesPat) ←
          (tickIds ++ hTickIds ++ #[heId]).mapM fun t => `(rcasesPat| $t:ident)
        `(fun $iId:ident $nId:ident $eId:ident => by
          rw [$runName:ident $iId]
          refine Iff.trans Trace.getElem?_map_eq_some ?_
          constructor
          · rintro ⟨$pId:ident, $hpId:ident, $heId:ident⟩
            obtain ⟨$xId:ident, $hxId:ident, $hpId':ident⟩ :=
              (scanAcrossTicksTrace_getElem?_eq_some ($stepId $iId) $seedVal
                $inpTrace $nId $pId).mp $hpId
            exact ⟨$mpWitness',*⟩
          · rintro ⟨$mprPat',*⟩
            exact ⟨_, (scanAcrossTicksTrace_getElem?_eq_some ($stepId $iId) $seedVal
              $inpTrace $nId _).mpr ⟨$tickTuple, $mprRead, rfl⟩, $heId⟩)
    -- the leg's read, over an abstract register (for `_reg`)
    let regLeg ← legOf (← `((($stepId $iId) ($(mkIdent `reg) $iId $nId) $tickTuple).2))
    let regAtRhs ← atRhsOf regLeg
    pure (runName, runStmt, runPrf, atName, atStmt, atPrf, regAtRhs)
  let legs ← (List.range outs.size).toArray.mapM legReaders
  -- `h<out>_reg` (stateful blocks): the register ∃-abstracted (the
  -- builder `mkRegGhost`, D64 E7), one read clause per output leg
  let regCtx : RegCtx := {
    out0Id, stepId, iId, nId, eId, seedVal, inpTrace, stBefore,
    tickIds, tickReads, tickTuple,
    outIds := outIdents,
    atNames := legs.map (·.2.2.2.1),
    regAtRhss := legs.map (·.2.2.2.2.2.2),
    inputIds := inputs.map (·.1),
    -- a stream state's written type names `H`, gone after the ghost's
    -- `subst`: let the step's binder type fill it
    stateTys := ← states.mapM fun st => if st.isStream then `(_) else pure st.ty }
  let legGhosts : TSyntax `term → TermElabM (TSyntax `term) := fun cont => do
    let mut b := cont
    for (runName, runStmt, runPrf, atName, atStmt, atPrf, _) in legs.reverse do
      b ← `(ghost have $runName : $runStmt := $runPrf;
            ghost have $atName : $atStmt := $atPrf;
            $b)
    pure b
  let readers : TSyntax `term → TermElabM (TSyntax `term) := fun cont => do
    if states.isEmpty then legGhosts cont
    else
      let (regName, regStmt, regPrf) ← mkRegGhost regCtx none
      legGhosts (← `(ghost have $regName : $regStmt := $regPrf;
        $cont))
  -- the invariant-carrying variant: `_reg` also states the invariant
  -- before every tick over the abstract register
  let readersInv : TSyntax `term → TSyntax `term → TermElabM (TSyntax `term) :=
    fun invTake cont => do
      let hTakeName := mkIdent (Name.mkSimple
        ("h" ++ out0Id.getId.toString ++ "_inv_take"))
      let (regName, regStmt, regPrf) ← mkRegGhost regCtx (some (invTake, ⟨hTakeName⟩))
      legGhosts (← `(ghost have $regName : $regStmt := $regPrf;
        $cont))
  let core : TSyntax `term ← do
    let withReaders ← readers contStx
    let tailLets ← outLets withReaders
    `(let $stepId := $stepLam
      let $resId := ($hId).tick_scan $stsShape $insShape $outsShape $inTuple $stepId $seedTuple
      $tailLets)
  let withInv : TSyntax `term ← do
    match inv?, prove? with
    | some inv, some (initE, tickE) =>
      unless outs.size == 1 do
        throwErrorAt inv "tick invariant: available on single-output \
          blocks (one `emit` or one `yield` leg; v2)"
      let emitId := out0Id
      -- binders: `x` or `(x : τ)`
      let binders : Array (Ident × Option (TSyntax `term)) :=
        inv[3].getArgs.map fun b =>
          if b[0].isIdent then ((⟨b[0]⟩ : Ident), none)
          else ((⟨b[0][1]⟩ : Ident), some (⟨b[0][3]⟩ : TSyntax `term))
      let ids : Array Ident := binders.map (·.1)
      -- the output binder must be typed `(out : List τ)`: τ names the
      -- step's emission type explicitly (at `Values` the step's
      -- components are `BoundedSingleton _`, only definitionally the
      -- plain types the obligations' simp lemmas need)
      let outElemTy : TSyntax `term ← do
        match binders[0]!.2 with
        | some ty =>
          match ty with
          | `(List $τ) => pure τ
          | _ => throwErrorAt ty "tick invariant: the output binder \
              must be typed `(out : List τ)`"
        | none => throwErrorAt inv "tick invariant: type the output \
            binder as `(out : List τ)`"
      let funBinders : Array (TSyntax `term) ← binders.mapM fun (x, ty?) =>
        match ty? with
        | some ty => `(($x : $ty))
        | none => `($x:ident)
      let predE : TSyntax `term := ⟨inv[6]⟩
      let predFun : TSyntax `term ← `(fun $funBinders* => $predE)
      unless ids.size ≥ 1 + states.size do
        throwErrorAt inv "tick invariant: name the output leg, then \
          every state, then any spectators"
      -- positional: ids = output, states…, spectators…
      let stateProjsOf : TSyntax `term → TermElabM (Array (TSyntax `term)) :=
        fun base => do
          if states.isEmpty then pure #[]
          else if states.size == 1 then pure #[base]
          else do
            let mut out : Array (TSyntax `term) := #[]
            let mut cur := base
            for k in [0:states.size] do
              if k + 1 == states.size then
                out := out.push cur
              else
                out := out.push (← `(($cur).1))
                cur ← `(($cur).2)
            pure out
      let stateProjs ← stateProjsOf
        (← `(scanAcrossTicksState ($stepId $iId) $seedVal $inpTrace))
      let stateProjsTake ← stateProjsOf stBefore
      let specIds := ids.extract (1 + states.size) ids.size
      let specApps : Array (TSyntax `term) ←
        specIds.mapM fun sid => `(($sid $iId))
      let hName := mkIdent (Name.mkSimple
        ("h" ++ emitId.getId.toString ++ "_inv"))
      let hTakeName := mkIdent (Name.mkSimple
        ("h" ++ emitId.getId.toString ++ "_inv_take"))
      -- the state tuple's type: a register's written type; a stream
      -- state's is left to unification (its written type names `H`,
      -- gone under the ghost's `Values` substitution — as `_reg` does)
      let stTy : TSyntax `term ← do
        if states.isEmpty then `(Unit)
        else if states.size == 1 then
          if states[0]!.isStream then `(_) else pure states[0]!.ty
        else do
          let tys ← states.mapM fun st => if st.isStream then `(_) else pure st.ty
          let mut t : TSyntax `term := tys.back!
          for ty in tys.reverse.toList.drop 1 do
            t ← `($ty × $t)
          pure t
      -- P over the lemma's (out, state) with the construct's
      -- productified state opened back into the named components
      let p2 : TSyntax `term ← do
        let outV := mkIdent `out
        let stV := mkIdent `st
        let stArgs ← stateProjsOf ⟨stV⟩
        `(fun ($outV : List $outElemTy) ($stV : $stTy) =>
          $predFun $outV $stArgs* $specApps*)
      let stmt : TSyntax `term ←
        `(∀ $iId:ident, $predFun ($emitId $iId)
            $stateProjs* $specApps*)
      let stmtTake : TSyntax `term ←
        `(∀ $iId:ident ($nId : Nat), $predFun (($emitId $iId).take $nId)
            $stateProjsTake* $specApps*)
      -- the obligations, ascribed with their full types (so the
      -- user's terms elaborate with concrete expected types — the
      -- generic lemma's premises, per instance); the instance binder
      -- is pinned through the input (an applied anchoring lambda).
      -- The `tick` obligation is the Verus loop body: this tick's
      -- input reads (`(xⱼ i)[n]? = some xⱼ_t`, one per input), the
      -- emissions so far and the register, and the step's two
      -- components on the named reads — no zipped tuple, no bound.
        let initTy : TSyntax `term ←
          `(∀ $iId:ident,
            (fun (_ : List _) => $p2 [] $seedVal) $inpTrace)
      let outId := mkIdent `out
      let stVId := mkIdent `stv
      let tickBinders : Array (TSyntax ``Parser.Term.bracketedBinder) ←
        tickIds.mapM fun t => `(bracketedBinder| ($t:ident : _))
      let tickHyps : TSyntax `term → TermElabM (TSyntax `term) := fun body =>
        tickReads.foldrM (fun r acc => `($r → $acc)) body
      let tickCore : TSyntax `term ←
        `(($outId).length = $nId → $p2 $outId $stVId →
          $p2 (List.append $outId [@Prod.snd $stTy $outElemTy (($stepId $iId) $stVId
              $tickTuple)])
            (@Prod.fst $stTy $outElemTy (($stepId $iId) $stVId $tickTuple)))
      let tickTy : TSyntax `term ←
        `(∀ $iId:ident ($nId : Nat) ($outId : List $outElemTy) ($stVId : $stTy)
            $tickBinders*, $(← tickHyps tickCore))
      let hinitId := mkIdent `hinit
      let htickId := mkIdent `htick
      let hrunId := mkIdent `hrun
      -- the generic lemma's step from the user's: one zip read, split
      -- into the component reads
      let bridge : TSyntax `term ← do
        let hlenId := mkIdent `hlen
        let ihId := mkIdent `ih
        let outV := mkIdent `out
        let stV := mkIdent `st
        `(fun $nId:ident $outV:ident $stV:ident $xId:ident $hxId:ident
            $hlenId:ident $ihId:ident =>
          $htickId $iId $nId $outV $stV $compProjs* $compReads* $hlenId $ihId)
      let prf : TSyntax `term ←
        `(by
          have $hinitId : $initTy := $initE
          have $htickId : $tickTy := $tickE
          intro $iId:ident
          have $hrunId := scanAcrossTicks_invariant? ($stepId $iId)
            $p2 $inpTrace $seedVal ($hinitId $iId) $bridge
          first
          | ($readerSimp:tactic; exact $hrunId)
          | exact $hrunId)
      let prfTake : TSyntax `term ←
        `(by
          have $hinitId : $initTy := $initE
          have $htickId : $tickTy := $tickE
          intro $iId:ident $nId:ident
          have $hrunId := scanAcrossTicks_invariant_take? ($stepId $iId)
            $p2 $inpTrace $seedVal ($hinitId $iId) $bridge $nId
          first
          | ($readerSimp:tactic; exact $hrunId)
          | exact $hrunId)
      -- the output leg's carrier: `emit` ↦ `Ticked`, `yield` ↦ `TickStream`
      let emitTy : TSyntax `term ←
        if emits.size == 1 then `(($hId).Ticked _ _)
        else `(($hId).TickStream _ _ _ _)
      -- the invariant before every tick, over the abstract register
      -- (the `_reg` clause consumers read instead of `_inv_take`)
      let regInvTake : TSyntax `term ← do
        let regProjs ← stateProjsOf (← `($(mkIdent `reg) $iId $nId))
        `(∀ $iId:ident ($nId : Nat), $predFun (($emitId $iId).take $nId)
            $regProjs* $specApps*)
      let withReaders ← readersInv regInvTake contStx
      let tailLets ← outLets withReaders
      `(let $stepId := $stepLam
        let $resId := ($hId).tick_scan $stsShape $insShape $outsShape $inTuple $stepId $seedTuple
        let $emitId : $emitTy := $resId
        ghost have $hName : $stmt := $prf;
        ghost have $hTakeName : $stmtTake := $prfTake;
        $tailLets)
    | some _, none =>
      throwErrorAt stx "tick: an `invariant` clause needs its \
        trailing `prove init := …, tick := …`"
    | none, some _ =>
      throwErrorAt stx "tick: `prove init/tick` without an \
        `invariant` clause"
    | none, none => pure core
  -- the input wires, bound in order
  let mut full : TSyntax `term := withInv
  for (xId, wire) in inputs.reverse do
    full ← `(let $xId := $wire
      $full)
  elabTerm full expectedType?

end HydroTick

end Hydro
