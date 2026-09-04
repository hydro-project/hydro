import HydroV2.HydroDef

/-!
# The `tick` construct (Rust `sliced!`/`use::state` — D58/D59)

A `tick` block declares per-location loop state (`use::state`), the
input it consumes tick-by-tick, and a PER-TICK body (the Rust sliced
body operates on this tick's values); `rebind` gives the next states,
`emit` the tick's emission. An optional Verus-style loop invariant
`P(emissions-so-far, states…, spectators…)` carries value-level
cross-iteration facts; its obligations trail the program:

    tick (state r₁ : τ₁ := seed₁) … (state rₙ : τₙ := seedₙ)
      (input x := <tick-singleton carrier>)
      (invariant (<emit> <r₁> … <rₙ> <spectators…>) => P)? :=
      <per-tick let-chain over r₁ … rₙ x (this tick's values)>
      rebind (r₁ := e₁, …, rₙ := eₙ)
      emit (<emit> := e)
      (prove init := <P at [] seeds>, tick := <one-tick step>)?;
    <continuation — `x` (the input carrier) and `<emit>` in scope>

Reads of `rᵢ` are the PREVIOUS tick's value and the cycle's unit lag
is the construct's own shape (Hydro compiles no instantaneous cycle),
so the denotation is a STRUCTURAL fold over the location's ticks —
`HydroSem.scan_across_ticks`, the construct's semantic former (the
composite `cycle_with_initial + defer_tick + body`; not a Rust
operator). The elaboration is a plain syntax transform — no operator
analysis:

    let x := <carrier>
    let <emit>_step := fun me (r₁, …, rₙ) x => (…, ((e₁, …), e))
    let <emit> := H.scan_across_ticks x <emit>_step (seed₁, …)
    ghost have h<emit>_inv : ∀ i, P (<emit> i) (states…) (specs…) :=
      fun i => scanAcrossTicks_invariant … (init) (tick)
    <continuation>

The invariant obligations are the generic structural induction's two
premises (`scanAcrossTicks_invariant`): `init` = P at the empty run,
`tick` = ONE loop body step (`P out states → P (out ++ [emission])
states'` with the step values definitional). Cross-iteration SHAPE
facts (output prefix-growth across ticks) are the construct's own
per-op lemmas (`scanAcrossTicksTrace_prefix`); the invariant carries
the value-level ones. `fix` + `base`/`step` remains the construct for
async-lag cycles (`forward_ref` through the network — induction over
exchange rounds, not ticks). -/

namespace HydroV2

namespace HydroTick

/-- A state binder: `(state r : σ := seed)`. -/
syntax tickState := atomic("(" "state ") ident " : " term " := " term ")"

/-- The consumed input: `(input x := <carrier>)` — the block runs
once per tick of this carrier; inside the body, `x` is THIS tick's
value. -/
syntax tickInput := atomic("(" &"input ") ident " := " term ")"

/-- The loop invariant — predicate only; its obligations trail the
program in the block tail's `prove` clause (Verus order). -/
syntax tickInvariant := atomic("(" &"invariant ") "(" ident+ ")" " => " term ")"

/-- The block tail: next states, the emission, and the trailing
obligations, followed by the continuation. -/
syntax (name := tickTail)
  withPosition("rebind " "(" (ident " := " term),+ ")"
    (&"emit " "(" (ident " := " term),+ ")")?
    ("prove " &"init " ":= " term ", " "tick " ":= " term)?)
    optSemicolon(term) : term

/-- The `tick` block. -/
syntax (name := tickTerm)
  withPosition("tick " tickState+ (tickInput)? (tickInvariant)?
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

/-- The `tick` elaborator: desugar to the structural fold (see the
module docstring) and delegate — a syntax transform. -/
@[term_elab tickTerm] def elabTickTerm : TermElab :=
    fun stx expectedType? => do
  -- states
  let states := stx[1].getArgs.map fun st =>
    ((⟨st[2]⟩ : Ident), (⟨st[4]⟩ : TSyntax `term),
     (⟨st[6]⟩ : TSyntax `term))
  -- input (mandatory: the loop runs over its ticks)
  let some inp := stx[2].getArgs[0]?
    | throwErrorAt stx "tick: an `(input x := …)` clause is required \
        (the block runs once per tick of this carrier)"
  let inpId : Ident := ⟨inp[2]⟩
  let inpTerm : TSyntax `term := ⟨inp[4]⟩
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
  -- the tail: rebinds (one per state, in order), the emission, proofs
  let some tail := bodyStx.find? (·.isOfKind ``tickTail)
    | throwErrorAt stx "tick: the block body must end with \
        `rebind (…) emit (…)`"
  let rebinds := tail[2].getSepArgs.map fun e =>
    ((⟨e[0]⟩ : Ident), (⟨e[2]⟩ : TSyntax `term))
  unless rebinds.size == states.size
      && (rebinds.zip states).all
          (fun (r, s) => r.1.getId == s.1.getId) do
    throwErrorAt tail "tick: `rebind` must assign every `state` of \
      this block, in declaration order \
      ({states.map (·.1.getId)})"
  if tail[4].getArgs.isEmpty then
    throwErrorAt tail "tick: an `emit (x := …)` leg is required"
  let emits := tail[4][2].getSepArgs.map fun e =>
    ((⟨e[0]⟩ : Ident), (⟨e[2]⟩ : TSyntax `term))
  unless emits.size == 1 do
    throwErrorAt tail "tick: exactly one `emit` leg (v1)"
  let (emitId, emitTerm) := emits[0]!
  let prove? : Option (TSyntax `term × TSyntax `term) :=
    if tail[5].getArgs.isEmpty then none
    else some (⟨tail[5][3]⟩, ⟨tail[5][7]⟩)
  let contStx : TSyntax `term := ⟨tail[7]⟩
  -- step result: ((e₁, …, eₙ), emission)
  let rebindTuple : TSyntax `term ← do
    let ts := rebinds.map (·.2)
    if ts.size == 1 then pure ts[0]!
    else `(($(ts[0]!), $(ts[1:].toArray),*))
  let pairStx : TSyntax `term ← `(($rebindTuple, $emitTerm))
  -- the per-tick body with the tail swapped for the step's result
  let (stepBodyRaw, _) := replaceTail bodyStx pairStx
  let stepBody : TSyntax `term := ⟨stepBodyRaw⟩
  -- the step function: states productified by the construct
  let meId := mkIdent `me
  let stId := mkIdent `st
  let stepLam : TSyntax `term ← do
    if states.size == 1 then
      let (rId, rTy, _) := states[0]!
      `(fun $meId ($rId : $rTy) $inpId => $stepBody)
    else
      let pats := states.map (fun (rId, _, _) => rId)
      let tys := states.map (·.2.1)
      let σTy : TSyntax `term ← do
        let mut t : TSyntax `term := tys.back!
        for ty in tys.reverse.toList.drop 1 do
          t ← `($ty × $t)
        pure t
      `(fun $meId ($stId : $σTy) $inpId =>
          match $stId:ident with
          | ($(pats[0]!), $(pats[1:].toArray),*) => $stepBody)
  let seedTuple : TSyntax `term ← do
    let ss := states.map (·.2.2)
    if ss.size == 1 then
      let (_, rTy, s) := states[0]!
      `(($s : $rTy))
    else `(($(ss[0]!), $(ss[1:].toArray),*))
  let stepId := mkIdent (emitId.getId.appendAfter "_step")
  -- assemble; the invariant becomes a `ghost have` (stated under the
  -- `Values` substitution, where the fold's denotation is definitional)
  let core : TSyntax `term ←
    `(let $stepId := $stepLam
      let $emitId := ($hId).scan_across_ticks $inpId $stepId $seedTuple
      $contStx)
  let withInv : TSyntax `term ← do
    match inv?, prove? with
    | some inv, some (initE, tickE) =>
      let ids : Array Ident := inv[3].getArgs.map (⟨·⟩)
      let predE : TSyntax `term := ⟨inv[6]⟩
      unless ids.size ≥ 1 + states.size do
        throwErrorAt inv "tick invariant: name the emit leg, then \
          every state, then any spectators"
      -- positional: ids = emit, states…, spectators…
      let stateProjs : Array (TSyntax `term) ← do
        let base : TSyntax `term ←
          `(scanAcrossTicksState ($stepId $(mkIdent `i))
            $seedTuple ($inpId $(mkIdent `i)))
        if states.size == 1 then pure #[base]
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
      let specIds := ids.extract (1 + states.size) ids.size
      let specApps : Array (TSyntax `term) ←
        specIds.mapM fun sid => `(($sid $(mkIdent `i)))
      let iId := mkIdent `i
      let hName := mkIdent (Name.mkSimple
        ("h" ++ emitId.getId.toString ++ "_inv"))
      let stTy : TSyntax `term ← do
        if states.size == 1 then pure states[0]!.2.1
        else do
          let tys := states.map (·.2.1)
          let mut t : TSyntax `term := tys.back!
          for ty in tys.reverse.toList.drop 1 do
            t ← `($ty × $t)
          pure t
      -- P over the lemma's (out, state) with the construct's
      -- productified state opened back into the named components
      let p2 : TSyntax `term ← do
        let outV := mkIdent `out
        let stV := mkIdent `st
        let stArgs : Array (TSyntax `term) ← do
          if states.size == 1 then pure #[⟨stV⟩]
          else do
            let mut out : Array (TSyntax `term) := #[]
            let mut cur : TSyntax `term := ⟨stV⟩
            for k in [0:states.size] do
              if k + 1 == states.size then
                out := out.push cur
              else
                out := out.push (← `(($cur).1))
                cur ← `(($cur).2)
            pure out
        `(fun ($outV : List _) ($stV : $stTy) =>
          (fun $ids* => $predE) $outV $stArgs* $specApps*)
      let stmt : TSyntax `term ←
        `(∀ $iId:ident, (fun $ids* => $predE) ($emitId $iId)
            $stateProjs* $specApps*)
      -- the obligations, ascribed with their full types (so the
      -- user's terms elaborate with concrete expected types — the
      -- generic lemma's premises, per instance)
      -- the instance binder is pinned through the input (an applied
      -- anchoring lambda — the obligation itself is its body); without
      -- this, predicates with no spectators leave the binder's type
      -- uninferable
      let initTy : TSyntax `term ←
        `(∀ $iId:ident,
          (fun (_ : List _) => $p2 [] $seedTuple) ($inpId $iId))
      let nId := mkIdent `n
      let hnId := mkIdent `hn
      let outId := mkIdent `out
      let stVId := mkIdent `stv
      let tickTy : TSyntax `term ←
        `(∀ $iId:ident ($nId : Nat)
            ($hnId : $nId < ($inpId $iId).length)
            ($outId : List _) ($stVId : $stTy),
          ($outId).length = $nId → $p2 $outId $stVId →
          $p2 ($outId ++ [(($stepId $iId) $stVId
              (($inpId $iId)[$nId]'$hnId)).2])
            (($stepId $iId) $stVId (($inpId $iId)[$nId]'$hnId)).1)
      let hinitId := mkIdent `hinit
      let htickId := mkIdent `htick
      let prf : TSyntax `term ←
        `(by
          have $hinitId : $initTy := $initE
          have $htickId : $tickTy := $tickE
          exact fun $iId => scanAcrossTicks_invariant ($stepId $iId)
            $p2 ($inpId $iId) $seedTuple ($hinitId $iId)
            ($htickId $iId))
      `(let $stepId := $stepLam
        let $emitId := ($hId).scan_across_ticks $inpId $stepId
          $seedTuple
        ghost have $hName : $stmt := $prf;
        $contStx)
    | some _, none =>
      throwErrorAt stx "tick: an `invariant` clause needs its \
        trailing `prove init := …, tick := …`"
    | none, some _ =>
      throwErrorAt stx "tick: `prove init/tick` without an \
        `invariant` clause"
    | none, none => pure core
  let full : TSyntax `term ←
    `(let $inpId := $inpTerm
      $withInv)
  elabTerm full expectedType?

end HydroTick

end HydroV2
