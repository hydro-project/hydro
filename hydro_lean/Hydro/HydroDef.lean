import Hydro.HydroGenKnot
import Hydro.HydroParam

/-!
# `hydro def`: the single program-module annotation

One annotation per module — Rust-attribute style (user ruling: no
separate registration/generation command tails in program files):

    hydro def M (H : HydroSem L mem) … := <the Rust-mirror body>
    hydro [p1bPairDecEq] def M …   := …   -- unfold hints for the scripts

`hydro def` elaborates the definition, validates + records its
composition spec (the strict module grammar of D45), and then runs the
**whole generation pipeline** automatically, with the phase set routed
from the spec:

- the def itself contains `HydroSem.fix`/`fixTick` (a knot wrapper —
  the `fix` block's hoisted `<M>.<wire>` constants) → the structural
  knot route (`runKnotStackT`) + the free theorem (`genParam`);
- a callee transitively reaches a knot (a glue module:
  `leader_election`, `paxos_core`) → simp-discovery namings
  (`runGlueT`) + causality + wf threading + monotonicity + the free
  theorem;
- otherwise (a knot-free content module) → the choice-route namings
  (`runCoupleT`) + causality + wf + mono + the free theorem.

The phases are `TermElabM` cores (`HydroGen.lean`, `HydroGenKnot.lean`,
`HydroParam.lean`); `hydro def` is the only surface (the toy suite in
`HydroGenToy.lean`/`HydroGenCheck.lean` pins every artifact class).

Elaboration budgets stay call-site-visible (user ruling): a module
whose body or scripts need more than the default budget carries a
`set_option maxHeartbeats … in` prefix on the whole `hydro def`
command — one bump, visible where it is paid.

The phases have no standalone command surface (D67): `hydro def` is the
only entry point, and the toy suite calls the `TermElabM` cores directly.
-/

namespace Hydro

namespace HydroGen

open Lean Elab Command

/-- Run the generation pipeline for a registered module, phase set
routed from the recorded composition spec. The phases are the
`TermElabM` cores (`runKnotStackT`, `runGlueT`, `runCoupleT`,
`runModCausalT`, `runModWfT`, `runModMonoT`, `genParam`); `hydro def`
is the only surface. -/
def runPipeline (mName : Name) (hints : Array Name) :
    CommandElabM Unit := do
  let env ← getEnv
  let gi := (getGenInfo env mName).getD {}
  if gi.hasFix then
    -- knot wrapper: the structural route
    liftTermElabM <| runKnotStackT mName hints
  else if gi.callees.any (reachesKnotN env) then
    -- glue module: simp-discovery namings over folded callees
    liftTermElabM <| runGlueT mName hints
    liftTermElabM <| runModCausalT mName hints
    liftTermElabM <| runModWfT mName hints
    liftTermElabM <| runModMonoT mName
  else
    -- knot-free content module: the choice-route namings
    liftTermElabM <| runCoupleT mName hints
    liftTermElabM <| runModCausalT mName hints
    liftTermElabM <| runModWfT mName hints
    liftTermElabM <| runModMonoT mName
  let names ← liftTermElabM <| genParam mName hints
  logInfo m!"hydro_param {mName}: {names.size} lemmas"

end HydroGen

namespace HydroGhost

/-- Pending ghost clauses (tactic replay syntax, source order), queued
by the `ghost` term elaborators — and by the `fix` construct itself
(the closed-knot `invariant` instance) — and drained by `prove`.
(Declared before the `fix` machinery, which queues into it.) -/
initialize pendingGhosts :
    IO.Ref (Array (Lean.TSyntax `tactic)) ← IO.mkRef #[]

end HydroGhost

/-! ## The inline `fix` block (Half 1)

Term syntax (active wherever `Hydro.HydroDef` is imported):

    fix (w₁ : τ₁) … (wₙ : τₙ) via (f₁, …, fₙ) := body
    rest

closes the mutual cycle `(w₁, …, wₙ) = body` (each `wᵢ` in scope in
`body`, which returns the `n`-tuple of wires; `fᵢ` are the `H.FixDec`
fuels, one per component) and continues with the closed wires in
scope. This is Rust's `forward_ref` reading; the elaborator performs
the **Bekić decomposition** to exactly the hand-written cascade shape
(last component outermost) and hoists each component to a top-level
named knot per D38/D40 — instance-generic body, curried caps tuple,
folded content — so kernel defeq never crosses a knot and proof time
stays the named-body cost. The hoisted defs are named
`<enclosing>.<wire>` and recorded for the `hydro def` post-pass, which
registers them and runs their knot/param generation before the
enclosing module's own pipeline. -/

namespace HydroGhost
/-- The module name of a contract-faced pair's declaration: `hydro def M`
elaborates the `ensures`-typed pair under `M._spec`. -/
def moduleOfSpec (n : Lean.Name) : Lean.Name :=
  match n with
  | Lean.Name.str p "_spec" => p
  | _ => n
end HydroGhost

namespace HydroFix

/-- `(w : τ)` — one cycle-wire binder. -/
syntax fixBinder := "(" ident " : " term ")"

/-- One named invariant obligation (`base := …` / `step := …` — the
idents are checked at elaboration, `prove`-field style). -/
syntax invField := ident " := " term

/-- The `invariant` clause (Verus-ordered: before the body). Names one
cycle wire, binds the fix body's chain `let`s at the current wire
(unprimed) and at the chain top (primed), states the invariant
predicate over them, and carries the `base`/`step` obligations. The
elaborator packages the bounded-chain induction (the generated
`stages` vocabulary) and hoists `<def>.<wire>.inv` — consumers see
only the closed knot; no proof outside the `fix` mentions stages. -/
syntax fixInvariant :=
  " invariant " ident "(" ident+ ")" " => " term
    (", " invField)+

/-- The `fix` block, `let`-style (position-disciplined body, rest after
a `;` or linebreak), with an optional Verus-style `invariant` clause
before the body. -/
syntax (name := fixTerm)
  withPosition("fix " fixBinder+ " via " term (fixInvariant)?
    " := " term) (optSemicolon(term))? : term

/-- `complete (e₁, …, eₙ)` — inside a single-source `fix` body: the
wire-closing tuple (Rust's `complete_cycle.complete(…)`). The `let`
chain above stays in scope for the continuation below, where the
cycle binders now denote the **closed** knots. -/
syntax (name := completeTerm)
  withPosition("complete " term) optSemicolon(term) : term

open Lean Parser Elab Term Meta HydroGen

/-- Emitted-knot queue: the fix elaborator records the hoisted knot
defs (in emission order) for the enclosing `hydro def`'s post-pass. -/
initialize pendingKnots : IO.Ref (Array Name) ← IO.mkRef #[]

/-- Invariant theorems (`<def>.<wire>.inv`) proven inside the `fix`
elaboration. The def's body may elaborate in SPECULATIVE branches whose
env additions are discarded (async/incremental elaboration); the knot
pipeline survives because the `hydro def` post-pass re-runs it at
command level — the packaged inductions are replayed the same way
(closed exprs cross branches through the ref). -/
initialize pendingInvs : IO.Ref (Array (Name × Expr × Expr)) ←
  IO.mkRef #[]

/-- The enclosing `hydro def`'s unfold hints (set by the annotation
before the def elaborates; the eager knot-stack drain inside `fix`
elaboration needs them). -/
initialize pendingHints : IO.Ref (Array Name) ← IO.mkRef #[]

/-- The single-source `fix` pass stack, innermost first (`true` =
pass 1, `false` = pass 2). A `complete` marker belongs to the
INNERMOST active fix; consuming it (pass 2's skip) pops the level so
an ENCLOSING fix's marker inside the continuation sees ITS own
pass. -/
initialize fixPassMode : IO.Ref (List Bool) ← IO.mkRef []

/-- Split the fuels syntax: a `(f₁, …, fₙ)` tuple for `n > 1`, a plain
term for `n = 1`. -/
private def splitFuels (stx : Syntax) (n : Nat) :
    TermElabM (Array Syntax) := do
  if n == 1 then
    return #[stx]
  if stx.isOfKind ``Lean.Parser.Term.tuple then
    -- `(a, b, c)` = "(" >> a >> ", " >> sepBy(term, ", ") >> ")"
    let elems := #[stx[1][0]] ++ stx[1][2].getSepArgs
    unless elems.size == n do
      throwErrorAt stx "fix: {n} wires but {elems.size} fuels"
    return elems
  throwErrorAt stx "fix: expected a ({n}-component) fuel tuple"

/-- π_i of a right-nested `n`-tuple expression built by `Prod.mk`
literals (after zeta): structural selection, no `Prod.fst/snd`
residue — reproduces the hand-written per-knot body shape. -/
private partial def tupleProj (e : Expr) (i n : Nat) : TermElabM Expr := do
  if n == 1 then return e
  let e' := e.consumeMData
  if e'.isAppOfArity ``Prod.mk 4 then
    if i == 0 then return e'.getAppArgs[2]!
    else tupleProj e'.getAppArgs[3]! (i - 1) (n - 1)
  else
    -- not a literal tuple: fall back to projections
    if i == 0 then mkAppM ``Prod.fst #[e]
    else tupleProj (← mkAppM ``Prod.snd #[e]) (i - 1) (n - 1)

/-- Zeta-reduce all `let`s in an expression (structural; the kernel
pays one beta per binding — module calls stay folded). -/
private partial def zetaLets (e : Expr) : CoreM Expr :=
  Core.transform e (post := fun e => do
    match e with
    | .letE _ _ v b _ => return .visit (b.instantiate1 v)
    | _ =>
      if e.isAppOf ``letFun && e.getAppNumArgs ≥ 4 then
        let args := e.getAppArgs
        return .visit (mkAppN (args[3]!.beta #[args[2]!]) args[4:args.size])
      else
        return .continue)

/-- Close a seed fvar set over the fvars occurring in the types of its
members, preserving local-context order. -/
private def fvarClosure (seed : Array FVarId) : MetaM (Array FVarId) := do
  let lctx ← getLCtx
  let mut inSet : Std.HashSet FVarId := {}
  let mut frontier := seed
  while !frontier.isEmpty do
    let mut next : Array FVarId := #[]
    for f in frontier do
      if inSet.contains f then continue
      inSet := inSet.insert f
      let used := (Lean.CollectFVars.main
        (← instantiateMVars (lctx.get! f).type) {}).fvarIds
      next := next ++ used.filter (fun x => !inSet.contains x)
    frontier := next
  let mut out : Array FVarId := #[]
  for decl in lctx do
    if inSet.contains decl.fvarId then out := out.push decl.fvarId
  pure out

/-- Right-nested product type of an array of types. -/
private def mkProdN (ts : Array Expr) : MetaM Expr := do
  if ts.isEmpty then return mkConst ``Unit
  let mut acc := ts.back!
  for i in [1:ts.size] do
    acc ← mkAppM ``Prod #[ts[ts.size - 1 - i]!, acc]
  pure acc

/-- Right-nested tuple of an array of values. -/
private def mkTupleN (es : Array Expr) : MetaM Expr := do
  if es.isEmpty then return mkConst ``Unit.unit
  let mut acc := es.back!
  for i in [1:es.size] do
    acc ← mkAppM ``Prod.mk #[es[es.size - 1 - i]!, acc]
  pure acc

/-- The Bekić wire closure: `K_j` (applied to data + H + deps) closed
at wire arguments `x_{j+1} … x_{n-1}` where `x_k` = the loop variable
(`k = i`), the pending param wire (`k > i`), or the recursive closure
(`k < i`). -/
private partial def closeWire (kApps ws : Array Expr) (i n j : Nat)
    (loopV : Expr) : MetaM Expr := do
  let mut wireArgs : Array Expr := #[]
  for k in [j+1:n] do
    if k == i then wireArgs := wireArgs.push loopV
    else if k > i then wireArgs := wireArgs.push ws[k]!
    else wireArgs := wireArgs.push (← closeWire kApps ws i n k loopV)
  pure (mkAppN kApps[j]! wireArgs)

/-- Let-bind the closed wires under their author names and elaborate
the continuation; `post` runs on the elaborated continuation at the
innermost point (wire fvars still live). -/
private partial def bindClosed (names : Array Name) (τs closed ws : Array Expr)
    (i : Nat) (restStx : Syntax) (expectedType? : Option Expr)
    (post : Expr → TermElabM Expr := pure) :
    TermElabM Expr := do
  if h : i < names.size then
    withLetDecl names[i] τs[i]! closed[i]! fun x => do
      let inner ← bindClosed names τs closed ws (i + 1) restStx expectedType?
        post
      -- force pending tactic blocks NOW: their proof terms may mention
      -- the wire fvars being bound (delayed-assignment leak otherwise)
      synthesizeSyntheticMVarsNoPostponing
      let inner ← instantiateMVars inner
      mkLetFVars #[x] (inner.replaceFVar ws[i]! x)
  else
    post (← elabTerm restStx expectedType?)

/-- Product-leaf count of a type (`A × B × C` ↦ 3, flat wire ↦ 1). -/
private partial def prodLeaves (ty : Expr) : Nat :=
  match ty.consumeMData with
  | .app (.app (.const ``Prod _) a) b => prodLeaves a + prodLeaves b
  | _ => 1

/-- Projections of `e : ty` at every product leaf, in order. -/
private partial def leafProjs (e ty : Expr) : TermElabM (Array Expr) := do
  match ty.consumeMData with
  | .app (.app (.const ``Prod _) a) b =>
    let l ← leafProjs (← mkAppM ``Prod.fst #[e]) a
    let r ← leafProjs (← mkAppM ``Prod.snd #[e]) b
    return l ++ r
  | _ => return #[e]

/-- Right-nested tuple of `k` consecutive projections of `chainApp`
starting at flat index `start` (of `nTotal`), shaped like `ty`. -/
private partial def reassemble (chainApp : Expr) (nTotal : Nat)
    (start : Nat) (ty : Expr) : TermElabM (Expr × Nat) := do
  match ty.consumeMData with
  | .app (.app (.const ``Prod _) a) b =>
    let (l, start') ← reassemble chainApp nTotal start a
    let (r, start'') ← reassemble chainApp nTotal start' b
    return (← mkAppM ``Prod.mk #[l, r], start'')
  | _ =>
    return (← tupleProj chainApp start nTotal, start + 1)

/-- All identifier head-names occurring in a syntax tree. -/
private partial def collectIdents (stx : Syntax) : Std.HashSet Name :=
  go stx {}
where
  go (stx : Syntax) (acc : Std.HashSet Name) : Std.HashSet Name :=
    match stx with
    | .ident _ _ nm _ => acc.insert nm.eraseMacroScopes
    | .node _ _ args => args.foldl (fun a s => go s a) acc
    | _ => acc

/-- Single-source chain peeling: enter the pass-1 body's `let` spine
and extend the wire-closing tuple at the tail — the hoisted chain
body then exposes the chain values by projection (the D40 chain-body
shape, generated). Tupled: lets the continuation references,
flattened to product leaves (flat wires — the leg granularity the
generation pipeline expects). Skipped (values stay inline, proof-leg
consumers only): subtype-valued lets (the module-call contract fetch
points) and lets only the pre-`complete` ghosts mention. Returns the
extended body and the let spine `(name, leafCount)` (0 = skipped). -/
private partial def extendChain (nWires : Nat) (e : Expr)
    (keep : Name → Bool) (leafFVars : Array Expr) :
    TermElabM (Expr × Array (Name × Nat)) := do
  let step (nm : Name) (ty v b : Expr) :
      TermElabM (Expr × Array (Name × Nat)) := do
    withLetDecl nm ty v fun x => do
      let ty' ← instantiateMVars ty
      let tuple := keep nm
        && ty'.getAppFn.constName? != some ``Subtype
      let k := if tuple then prodLeaves ty' else 0
      let leaves ← if tuple then leafProjs x ty' else pure #[]
      let (tail, names) ← extendChain nWires (b.instantiate1 x) keep
        (leafFVars ++ leaves)
      return (← mkLetFVars #[x] tail, #[(nm, k)] ++ names)
  match e.consumeMData with
  | .letE nm ty v b _ => step nm ty v b
  | e' =>
    if e'.isAppOf ``letFun && e'.getAppNumArgs ≥ 4 then
      let args := e'.getAppArgs
      match args[3]! with
      | .lam nm ty b _ => step nm ty args[2]! b
      | _ => throwError "fix: single-source chain: letFun without \
          lambda"
    else
      let mut comps : Array Expr := #[]
      for i in [0:nWires] do
        comps := comps.push (← tupleProj e' i nWires)
      return (← mkTupleN (comps ++ leafFVars), #[])

/-- Single-source pass 2 post-pass: swap the chain `let` values for
projections of the hoisted chain-body constant (the module value then
computes through the registered name — the walker's module boundary —
while the surface chain appears once). Matches the pass-1 let spine
by name, in order; skipped lets keep their inline values; the proof
legs reference the let fvars, so they are untouched (the swapped
values are definitionally the same). -/
private partial def swapChainLets (spine : Array (Name × Nat))
    (chainApp : Expr) (nWires nTotal : Nat) (j leafIdx : Nat)
    (e : Expr) : TermElabM Expr := do
  if j >= spine.size then return e
  let (nm, k) := spine[j]!
  let swapVal (ty v : Expr) : TermElabM (Expr × Nat) := do
    if k == 0 then return (v, leafIdx)
    let (v', _) ← reassemble chainApp nTotal (nWires + leafIdx) ty
    return (v', leafIdx + k)
  match e.consumeMData with
  | .letE nm' ty v b nd =>
    unless nm' == nm do
      throwError "fix: single-source pass-2 let spine mismatch: \
        expected `{nm}`, found `{nm'}`"
    let (v', leafIdx') ← swapVal ty v
    return .letE nm' ty v' (← swapChainLets spine chainApp nWires
      nTotal (j + 1) leafIdx' b) nd
  | e' =>
    if e'.isAppOf ``letFun && e'.getAppNumArgs ≥ 4 then
      let args := e'.getAppArgs
      match args[3]! with
      | .lam nm' ty b bi =>
        unless nm' == nm do
          throwError "fix: single-source pass-2 let spine mismatch: \
            expected `{nm}`, found `{nm'}`"
        let (v', leafIdx') ← swapVal ty args[2]!
        let b' ← swapChainLets spine chainApp nWires nTotal (j + 1)
          leafIdx' b
        return mkAppN e'.getAppFn
          (args.set! 2 v' |>.set! 3 (Expr.lam nm' ty b' bi))
      | _ => throwError "fix: single-source pass-2: letFun without \
          lambda at let `{nm}`"
    else
      throwError "fix: single-source pass-2: expected the chain `let` \
        spine (`{nm}`), found {e'.ctorName}"

/-- Walk the hoisted chain constant's `let` spine at the given
argument list, returning each chain `let`'s (source name, value) with
earlier lets zeta-substituted — the program's own runs, spelled by the
program itself — plus the fully-substituted result tuple (the wire
closings followed by the kept-let leaves). -/
private partial def chainRunsAt (chainName : Name) (args : Array Expr) :
    TermElabM (Array (Name × Expr) × Expr) := do
  let info ← getConstInfo chainName
  let some v := info.value? | throwError "fix: chain has no value"
  go ((v.beta args).headBeta) #[]
where
  go (e : Expr) (out : Array (Name × Expr)) :
      TermElabM (Array (Name × Expr) × Expr) := do
    let e' := e.consumeMData
    if let .letE nm _ v b _ := e' then
      go (b.instantiate1 v) (out.push (nm, v))
    else if e'.isAppOf ``letFun && e'.getAppNumArgs ≥ 4 then
      let args' := e'.getAppArgs
      if let .lam nm _ b _ := args'[3]! then
        go (b.instantiate1 args'[2]!) (out.push (nm, args'[2]!))
      else
        pure (out, e')
    else
      pure (out, e')

/-- Elaborate a `fix` `invariant` clause (Verus-ordered, named
`base`/`step` legs): packages the bounded-chain induction over the
generated `stages` vocabulary and hoists `<def>.<wire>.inv` — the
invariant at the CLOSED knot. Consumers (prove legs) never mention
stages; the step proof reasons about two wire valuations `w ⊑ w⊤`
with the chain `let`s bound at both under their source names
(unprimed at `w`, primed at `w⊤`). -/
private def elabFixInvariantClause (declName : Name)
    (wireNames : Array Name) (chainName : Name) (nWires nTotal : Nat)
    (invStx : Syntax) : TermElabM Unit := do
  let wireNm := invStx[1].getId
  unless wireNames.size > 0 && wireNames[0]! == wireNm do
    throwError "fix invariant: only the innermost cycle wire \
      (`{wireNames[0]!}`) is supported; got `{wireNm}`"
  let letIds := invStx[3].getArgs.map (·.getId)
  let predStx := invStx[6]
  -- obligation fields: `base := …, step := …`
  let fields : Array (Name × Syntax) :=
    invStx[7].getArgs.map (fun g =>
      (g[1][0].getId.eraseMacroScopes, g[1][2]))
  let (baseStx, stepStx) ←
    match fields.map (·.1) with
    | #[`base, `step] => pure (fields[0]!.2, fields[1]!.2)
    | _ => throwError "fix invariant: expected `base := …, step := …`"
  let kName := declName ++ wireNm
  let stagesName := kName ++ `stages
  unless (← getEnv).contains stagesName do
    throwError "fix invariant: no generated stage vocabulary for \
      `{kName}`"
  let fixStmt := (← getConstInfo (stagesName.appendAfter "_fix")).type
  forallTelescope fixStmt fun xs eqTy => do
  let some (_, closedK, stagesFuel) := eqTy.eq?
    | throwError "fix invariant: malformed stages_fix"
  let fuel := stagesFuel.appArg!
  let τw ← inferType stagesFuel
  -- seed / F from the stages definition (λ binders m, iterate F seed m)
  let sInfo ← getConstInfo stagesName
  let sVal := (sInfo.value!.beta xs).headBeta
  let iterApp := sVal.bindingBody!
  unless iterApp.isAppOf ``iterate && iterApp.getAppNumArgs == 4 do
    throwError "fix invariant: stages value is not an iterate"
  let F := iterApp.getAppArgs[1]!
  let seed := iterApp.getAppArgs[2]!
  -- the chain constant's argument list, with the invariant wire as a
  -- hole: knot telescope = ctx ++ [H] ++ postH ++ params; the chain
  -- adds the innermost wire before the params
  let kArgs := closedK.getAppArgs
  let nParams := nWires - 1
  let chainArgsAt := fun (w : Expr) =>
    kArgs.extract 0 (kArgs.size - nParams) ++ #[w]
      ++ kArgs.extract (kArgs.size - nParams) kArgs.size
  -- the pointwise carrier family (from the knot's H-generic result
  -- head: `HydroSem.Stream` vs `HydroSem.Ticked`)
  let kTy := (← getConstInfo kName).type
  let cf ← forallTelescope kTy fun _ resTy => do
    match resTy.getAppFn.constName? with
    | some n =>
      match n with
      | ``HydroSem.Stream => pure CarrierFam.stream
      | ``HydroSem.Ticked => pure CarrierFam.ticked
      | _ => throwError "fix invariant: unsupported wire carrier \
          `{n}`"
    | none => throwError "fix invariant: knot result head"
  -- runs (the program's own chain `let`s) and the wire's ONE-STEP
  -- image at the module boundary (the `complete` closing leg — the
  -- obligations are spelled at the program's own module applications,
  -- never at the folded body constant)
  let runOf : Array (Name × Expr) → Name → TermElabM Expr :=
    fun runs nm => do
    match runs.find? (·.1 == nm) with
    | some (_, v) => pure v
    | none => throwError "fix invariant: `{nm}` is not a chain `let` \
        of the fix body (available: {runs.map (·.1)})"
  let closingAt : Expr → TermElabM Expr := fun w => do
    let (_, fin) ← chainRunsAt chainName (chainArgsAt w)
    tupleProj fin 0 nTotal
  let (runsClosed, _) ← chainRunsAt chainName (chainArgsAt closedK)
  withLocalDecl `w .default τw fun wE => do
  let (runsW, _) ← chainRunsAt chainName (chainArgsAt wE)
  let runFor : Name → TermElabM Expr := fun id => do
    let s := id.toString
    if s.endsWith "'" then
      runOf runsClosed (Name.mkSimple (s.dropRight 1))
    else
      runOf runsW id
    -- obligations. Two devices make the proofs read like the program:
  -- (a) ensures-face auto-premises — for each requested chain `let`
  -- whose value is a module application, the construct prefetches
  -- the colocated face (`M.ensures …`) at every
  -- valuation the obligation mentions (`hx` at the current wire,
  -- `hx_step` at its one-step image, `hx'` at the chain top);
  -- (b) named-run binderification — each obligation is stated over
  -- opaque binders carrying the runs' SOURCE names (`le`, `sp`,
  -- `le_step`, `le'`, …, plus `<wire>'` for the closed knot), so
  -- proofs never respell a module application.
  let faceOf : Expr → TermElabM (Option Expr) := fun runVal => do
    -- a module application `M … (H := Values) …` with a face theorem
    -- `M.ensures` (binders = the module's, minus the interpretation)
    let .const m lvls := runVal.getAppFn | return none
    unless (← getEnv).contains (m ++ `ensures) do return none
    let hIdx? ← forallTelescope (← getConstInfo m).type fun xs _ => do
      xs.findIdxM? fun x => do
        pure ((← inferType x).getAppFn.isConstOf ``HydroSem)
    let some hIdx := hIdx? | return none
    let args := runVal.getAppArgs
    unless hIdx < args.size do return none
    return some (mkAppN (mkConst (m ++ `ensures) lvls) (args.eraseIdx! hIdx))
  let facesAt : Expr → String → TermElabM (Array (Name × Expr × Expr)) :=
    fun wv suffix => do
      let (runs, _) ← chainRunsAt chainName (chainArgsAt wv)
      let mut out := #[]
      for id in letIds do
        unless id.toString.endsWith "'" do
          -- tolerate non-chain ids (the tick route's opaque-param
          -- spectators): no face to prefetch
          if let some (_, v) := runs.find? (·.1 == id) then
            if let some prf ← faceOf v then
              out := out.push
                (Name.mkSimple ("h" ++ id.toString ++ suffix),
                 ← whnf (← inferType prf), prf)
      pure out
  let facesClosed ← do
    let mut out : Array (Name × Expr × Expr) := #[]
    for id in letIds do
      if id.toString.endsWith "'" then
        if let some prf ← faceOf (← runOf runsClosed
            (Name.mkSimple (id.toString.dropRight 1))) then
          out := out.push (Name.mkSimple ("h" ++ id.toString),
            ← whnf (← inferType prf), prf)
    pure out
  let runsBinders : Expr → String → TermElabM (Array (Name × Expr)) :=
    fun wv suffix => do
      let (runs, _) ← chainRunsAt chainName (chainArgsAt wv)
      let mut out := #[]
      for id in letIds do
        unless id.toString.endsWith "'" do
          out := out.push
            (Name.mkSimple (id.toString ++ suffix), ← runOf runs id)
      pure out
  let closedBinders ← do
    let mut out : Array (Name × Expr) := #[]
    for id in letIds do
      if id.toString.endsWith "'" then
        out := out.push (id, ← runOf runsClosed
          (Name.mkSimple (id.toString.dropRight 1)))
    pure out
  -- the top's own one-step order (`htop`: the chain top is below its
  -- image — the generated chain mono at `fuel ≤ fuel + 1`)
  let closedStep ← closingAt closedK
  let htopTy ← valuesRelOf cf τw closedK closedStep
  let htopPrf ← do
    let monoLe ← mkAppM ``Nat.le_succ #[fuel]
    let sMono := mkAppN (mkConst (stagesName.appendAfter "_mono")) xs
    mkExpectedTypeHint (← mkAppOptM' sMono #[none, none, some monoLe])
      htopTy
  let htopPrem : Array (Name × Expr × Expr) :=
    #[(`htop, htopTy, htopPrf)]
  -- an obligation: named-run LET binders (transparent — mono lemmas
  -- and faces defeq through them; application self-instantiates),
  -- premises, conclusion; the type is spelled at the binders
  -- (containing values replaced first), the proof is elaborated
  -- against it and returned applied at the premise proofs
  let mkObligation : Array (Name × Expr) →
      Array (Name × Expr × Expr) → Expr → Syntax →
      TermElabM Expr := fun binders prems concl prfStx => do
    let rec bindLets (i : Nat) (bs : Array Expr)
        (k : Array Expr → TermElabM Expr) : TermElabM Expr := do
      if i < binders.size then
        let (n, v) := binders[i]!
        withLetDecl n (← inferType v) v fun b =>
          bindLets (i + 1) (bs.push b) k
      else k bs
    bindLets 0 #[] fun bs => do
    -- value→binder replacement, by DECREASING value size (a containing
    -- let value is strictly larger than its subterms, so outer lets
    -- bind before the inner values they contain are rewritten away);
    -- ties (equal values across valuations, e.g. a wire-independent
    -- run at `w` and at its image) resolve to the EARLIER binder
    let order := (Array.range binders.size).qsort (fun i j =>
      let si := binders[i]!.2.sizeWithoutSharing
      let sj := binders[j]!.2.sizeWithoutSharing
      si > sj || (si == sj && i < j))
    let repl : Expr → Expr := fun e0 => Id.run do
      let mut e := e0
      for idx in order do
        let v := binders[idx]!.2
        let b := bs[idx]!
        e := e.replace (fun x => if x == v then some b else none)
      pure e
    let premDecls := prems.map fun (n, ty, _) => (n, repl ty)
    withLocalDecls (premDecls.map fun (n, ty) =>
        (n, BinderInfo.default, fun _ => pure ty)) fun ps => do
    let oblTy ← mkForallFVars (bs ++ ps) (repl concl)
    let prf ← Term.elabTermEnsuringType prfStx oblTy
    Term.synthesizeSyntheticMVarsNoPostponing
    let prf ← instantiateMVars prf
    pure (mkAppN prf (prems.map (·.2.2)))
  let monoName := stagesName.appendAfter "_mono"
  let sMono := mkAppN (mkConst monoName) xs
  let wStep ← closingAt wE
  -- ===== route-specific middle: the invariant family `I`, the base
  -- proof `I(seed)`, and the `iterate_invariant` step function =====
  let mkFixRoute : TermElabM ((Expr → TermElabM Expr) × Expr × Expr) := do
    -- elaborate the predicate over opaque binders for the named runs
    let mut declList : Array (Name × BinderInfo × Expr) := #[]
    for id in letIds do
      declList := declList.push
        (id, .default, ← inferType (← runFor id))
    let P ← withLocalDecls (declList.map fun (n, bi, ty) =>
        (n, bi, fun _ => pure ty)) fun ls => do
      let p ← Term.elabTermEnsuringType predStx (mkSort .zero)
      Term.synthesizeSyntheticMVarsNoPostponing
      mkLambdaFVars ls (← instantiateMVars p)
    let IAt : Expr → TermElabM Expr := fun w => do
      let (runs, _) ← chainRunsAt chainName (chainArgsAt w)
      let mut args : Array Expr := #[]
      for id in letIds do
        let s := id.toString
        if s.endsWith "'" then
          args := args.push (← runOf runsClosed
            (Name.mkSimple (s.dropRight 1)))
        else
          args := args.push (← runOf runs id)
      pure (P.beta args)
    -- base: faces at the seed and at the top ⊢ I(seed)
    let iSeed ← IAt seed
    let facesSeed ← facesAt seed ""
    let baseProof ← mkObligation
      (#[(wireNm.appendAfter "'", closedK)]
        ++ (← runsBinders seed "") ++ closedBinders)
      (htopPrem ++ facesClosed ++ facesSeed) iSeed baseStx
    -- step: ⊑-premises and faces at the wire, its image (the
    -- `complete` closing leg), and the top ⊢ I preserved
    let stepProofAt ← do
      let relW ← valuesRelOf cf τw wE closedK
      let relB ← valuesRelOf cf τw wStep closedK
      -- the ONE-STEP chain order at the stage (`stages_mono` at
      -- `m ≤ m + 1`): ticked carriers only grow by appending ticks, so
      -- this is what reduces a stage step to a per-tick argument
      -- (`prefix_ext_invariant`)
      let relWB ← valuesRelOf cf τw wE wStep
      let iW ← IAt wE
      let iB ← IAt wStep
      let facesW ← facesAt wE ""
      let facesB ← facesAt wStep "_step"
      let binders :=
        #[(wireNm.appendAfter "'", closedK)]
          ++ (← runsBinders wE "") ++ (← runsBinders wStep "_step")
          ++ closedBinders
      let core ← mkArrow relW (← mkArrow relB (← mkArrow relWB
        (← mkArrow iW (← pure iB))))
      let prf ← mkObligation binders
        (htopPrem ++ facesClosed ++ facesW ++ facesB) core stepStx
      -- abstract the wire itself (bound OUTSIDE the run binders so the
      -- runs' types may mention it)
      mkLambdaFVars #[wE] prf
    let stepFn ← withLocalDecl `m .default (mkConst ``Nat) fun mE => do
      withLocalDecl `hm .default
          (← mkAppM ``LT.lt #[mE, fuel]) fun hmE => do
      let iterM ← mkAppOptM ``iterate
        #[some τw, some F, some seed, some mE]
      withLocalDecl `ih .default (← IAt iterM) fun ihE => do
      let leOf ← mkAppM ``Nat.le_of_lt #[hmE]
      let ltLe ← mkAppM ``Nat.succ_le_of_lt #[hmE]
      let leSucc ← mkAppM ``Nat.le_succ #[mE]
      let rel1 ← mkAppOptM' sMono #[none, none, some leOf]
      let rel2 ← mkAppOptM' sMono #[none, none, some ltLe]
      let rel0 ← mkAppOptM' sMono #[none, none, some leSucc]
      let relW ← valuesRelOf cf τw iterM closedK
      let relB ← valuesRelOf cf τw (← closingAt iterM) closedK
      let relWB ← valuesRelOf cf τw iterM (← closingAt iterM)
      let rel1 ← mkExpectedTypeHint rel1 relW
      let rel2 ← mkExpectedTypeHint rel2 relB
      let rel0 ← mkExpectedTypeHint rel0 relWB
      let prf := mkApp4 ((stepProofAt.beta #[iterM]).headBeta)
        rel1 rel2 rel0 ihE
      -- the invariant family is stated at the iterate; the step lands
      -- at the closing leg — definitionally the next iterate
      let prf ← mkExpectedTypeHint prf
        (← IAt ((F.beta #[iterM]).headBeta))
      mkLambdaFVars #[mE, hmE, ihE] prf
    pure (IAt, baseProof, stepFn)
  let (IAt, baseProof, stepFn) ← mkFixRoute
  -- compose: iterate_invariant + stages_mono (the ⊑ premises) +
  -- stages_fix (closed = fueled stage, definitionally)
  let IFn ← withLocalDecl `w .default τw fun v => do
    mkLambdaFVars #[v] (← IAt v)
  let invProof ← mkAppOptM ``iterate_invariant
    #[some τw, some F, some seed, some IFn, some fuel,
      some baseProof, some stepFn]
  let iClosed ← IAt closedK
  let invProof ← mkExpectedTypeHint invProof iClosed
  let stmt ← mkForallFVars xs iClosed
  let val ← instantiateMVars (← mkLambdaFVars xs invProof)
  if val.hasExprMVar then
    throwError "fix invariant: residual metavariables"
  -- loud guard (D58): a mis-named hypothesis inside an obligation
  -- tactic error-recovers into a term with DEAD free variables; only
  -- the async kernel would reject it, far from the cause — fail here
  let fvs := (Lean.CollectFVars.main val {}).fvarIds
  let fvs2 := (Lean.CollectFVars.main stmt {}).fvarIds
  unless fvs.isEmpty && fvs2.isEmpty do
    let lctx ← getLCtx
    throwError "fix invariant: residual free variables in the \
      packaged induction (an obligation proof likely references an \
      unknown name): val \
      {fvs.map (fun f => (lctx.find? f).map (·.userName))}, stmt \
      {fvs2.map (fun f => (lctx.find? f).map (·.userName))}"
  HydroGen.addThm (kName ++ `inv) stmt val
  pendingInvs.modify (·.push (kName ++ `inv, stmt, val))
  -- auto-supply the closed-knot instance to the enclosing prove leg
  -- (ghost-queue a `have h<wire>_inv := <def>.<wire>.inv args…`,
  -- applied at the def's own binders by NAME — they are all in scope
  -- there; the invariant's premises stay general). The prove leg
  -- never respells the module application. NOTE: args come from the
  -- OUTER telescope `xs` only (re-telescoping `stmt` would walk into
  -- the invariant's own binders).
  let mut argIds : Array Ident := #[]
  let mut argsOk := true
  for x in xs do
    let d ← x.fvarId!.getDecl
    if d.binderInfo.isExplicit then
      let nm := d.userName.eraseMacroScopes
      if nm.isAnonymous || !nm.isAtomic then
        argsOk := false
      else
        argIds := argIds.push (mkIdent nm)
  if argsOk then
    let invId := mkCIdent (kName ++ `inv)
    let hName := mkIdent (Name.mkSimple
      ("h" ++ wireNm.toString ++ "_inv"))
    let tac ← `(tactic| have $hName := $invId $argIds*)
    -- nested in an enclosing single-source fix: the inner block
    -- elaborates once per outer pass — queue the ghost only on the
    -- pass whose continuation elaborates (not the outer pass 1)
    unless (← fixPassMode.get).contains true do
      HydroGhost.pendingGhosts.modify (·.push tac)
  logInfo m!"fix invariant: {kName ++ `inv}"

/-- Abstract `e` over `xs`, turning LET-bound fvars into OPAQUE lambda
binders: a hoisted module takes the enclosing chain's runs as plain
inputs — their defining values stay at the call site (abstracting them
as `let`s would pull the values in, leaking any fvars THEY reference,
and would break the application arity). -/
private partial def mkLambdaFVarsOpaque (xs : Array Expr) (e : Expr) :
    TermElabM Expr := do
  go 0 #[] #[] e
where
  go (i : Nat) (newXs : Array Expr) (subst : Array (Expr × Expr))
      (e : Expr) : TermElabM Expr := do
    if i == xs.size then
      let e := subst.foldl (fun e (o, n) => e.replaceFVar o n) e
      mkLambdaFVars newXs e
    else
      let x := xs[i]!
      let d ← x.fvarId!.getDecl
      if d.isLet then
        let ty := subst.foldl (fun t (o, n) => t.replaceFVar o n)
          (← instantiateMVars d.type)
        withLocalDeclD d.userName ty fun fresh =>
          go (i + 1) (newXs.push fresh) (subst.push (x, fresh)) e
      else
        go (i + 1) (newXs.push x) subst e

@[term_elab fixTerm] def elabFixTerm : TermElab := fun stx expectedType? => do
  let binders := stx[1].getArgs
  let fuelsStx := stx[3]
  let invStx? : Option Syntax :=
    if stx[4].getArgs.isEmpty then none else some stx[4][0]
  let bodyStx := stx[6]
  -- single-source form: no trailing rest — the body carries a
  -- `complete` marker and the continuation follows it inline
  let restStx? : Option Syntax :=
    if stx[7].getArgs.isEmpty then none else some stx[7][1]
  let singleSource := restStx?.isNone
  let n := binders.size
  let names := binders.map (·[1].getId)
  let tyStxs : Array Syntax := binders.map (·[3])
  let some declName ← getDeclName?
    | throwError "fix: no enclosing declaration"
  -- the `hydro def` command elaborates the contract-faced pair under
  -- `M._spec` and derives the module `M` from it: the hoisted knots
  -- belong to `M`
  let declName := HydroGhost.moduleOfSpec declName
  let τs ← tyStxs.mapM fun t => elabType t
  let fuelStxs ← splitFuels fuelsStx n
  let decls : Array (Name × (Array Expr → TermElabM Expr)) :=
    (names.zip τs).map fun (nm, τ) => (nm, fun _ => pure τ)
  withLocalDeclsD decls fun ws => do
    let prodTy ← mkProdN τs
    -- pass 1 (single-source): the `complete` marker returns the
    -- wire-closing tuple; the inline continuation is not elaborated
    -- and ghost clauses do not queue
    if singleSource then fixPassMode.modify (true :: ·)
    let body ←
      try elabTermEnsuringType bodyStx prodTy
      finally if singleSource then fixPassMode.modify (·.drop 1)
    synthesizeSyntheticMVarsNoPostponing
    let body ← instantiateMVars body
    let fuels ← fuelStxs.mapM fun f => do
      let e ← elabTerm f none
      synthesizeSyntheticMVarsNoPostponing
      instantiateMVars e
    -- the ambient interpretation fvar: the unique `HydroSem`-typed
    -- free variable of the wire types
    let lctx ← getLCtx
    let tyFvars := (τs.foldl (fun (s : CollectFVars.State) τ =>
      Lean.CollectFVars.main τ s) {}).fvarIds
    let some hFvar := tyFvars.find? (fun f =>
        (lctx.get! f).type.getAppFn.constName? == some ``HydroSem)
      | throwErrorAt stx "fix: wire types mention no `HydroSem` \
          interpretation variable"
    let hExpr := mkFVar hFvar
    -- inline let-bound aliases whose VALUES are `H`-free; `H`-dependent
    -- locals stay as captures
    let inlineLets (e₀ : Expr) : TermElabM Expr := do
      let mut e := e₀
      for _ in [0:8] do
        let fvars := (Lean.CollectFVars.main e {}).fvarIds
        let mut subst : Array (FVarId × Expr) := #[]
        for f in fvars do
          if let some d := lctx.find? f then
            if let some v := d.value? then
              unless v.containsFVar hFvar do
                subst := subst.push (f, v)
        if subst.isEmpty then break
        for (f, v) in subst do
          e := e.replaceFVar (mkFVar f) v
      pure e
    let rawBody ← inlineLets body
    let body ← zetaLets rawBody
    let fuels ← fuels.mapM inlineLets
    -- free-variable analysis: caps = `H`-dependent fvars free in the
    -- BODY (they become tuple components); fuels/types may mention
    -- further binders (e.g. the fuel decision) that are knot-def
    -- binders but not captured
    let bodySeed := Lean.CollectFVars.main body {}
    let seed₀ := (fuels.toList ++ τs.toList).foldl
      (fun (s : CollectFVars.State) e => Lean.CollectFVars.main e s) bodySeed
    let wIds := ws.map (·.fvarId!)
    let all ← fvarClosure ((seed₀.fvarIds.filter
      (fun f => !wIds.contains f)).push hFvar)
    let bodyIds := (Lean.CollectFVars.main body {}).fvarIds
    let hPos := (lctx.get! hFvar).index
    let typeOf : Std.HashMap FVarId Expr ←
      all.foldlM (fun m f => do
        pure (m.insert f (← instantiateMVars (lctx.get! f).type)))
        ({} : Std.HashMap FVarId Expr)
    let hDep (f : FVarId) : Bool := (typeOf[f]?.getD (mkConst ``Unit)).containsFVar hFvar
    -- the knot's binder layout mirrors the hand convention: the
    -- enclosing def's pre-`H` data stay pre-`H` params; every other
    -- binder (H-free data, decisions, wires, fuels) sits after `H` in
    -- context order; `H`-dependent fvars OCCURRING IN THE BODY are
    -- additionally the caps-tuple components
    let dataPre := all.filter fun f =>
      f != hFvar && (lctx.get! f).index < hPos && !hDep f
    let postH := all.filter fun f =>
      f != hFvar && ((lctx.get! f).index > hPos || hDep f)
    let dataPreE := dataPre.map mkFVar
    let postHE := postH.map mkFVar
    -- caps = H-dependent post-`H` fvars the (possibly single-source-
    -- replaced) body references — resolved via a cell so the hoist
    -- closure sees the final classification (a dec appearing only in
    -- `via` fuels still reaches the chain application's arguments)
    let capsCell ← IO.mkRef (postH.filter fun f =>
      bodyIds.contains f && hDep f)
    let hTy ← inferType hExpr
    -- hoist one component to a top-level knot def; returns the
    -- constant applied to dataPre + H + postH (wire params pending)
    let hoist := fun (knotName : Name) (isTick : Bool) (w τw : Expr)
        (paramWs : Array Expr) (b f : Expr) => do
      let capsE := (← capsCell.get).map mkFVar
      let caps := capsE ++ paramWs
      let capsTys ← caps.mapM fun c => do instantiateMVars (← inferType c)
      -- Γ as an explicit lambda, abstracting H from the caps types
      let capsProd ← mkProdN capsTys
      let gamma := Lean.mkLambda `H'' .default hTy
        (capsProd.abstract #[hExpr])
      let genBody ←
        withLocalDeclD `H'' hTy fun h'' => do
        let capsProd'' := (capsProd.abstract #[hExpr]).instantiate1 h''
        withLocalDeclD `caps capsProd'' fun capsV => do
        let τw'' := (τw.abstract #[hExpr]).instantiate1 h''
        withLocalDeclD `l τw'' fun wV => do
          let mut projs : Array Expr := #[]
          let mut cur := capsV
          for i in [0:caps.size] do
            if i + 1 == caps.size then
              projs := projs.push cur
            else
              projs := projs.push (← mkAppM ``Prod.fst #[cur])
              cur ← mkAppM ``Prod.snd #[cur]
          let bAbs := b.abstract (#[hExpr] ++ caps ++ #[w])
          let bInst := bAbs.instantiateRev (#[h''] ++ projs ++ #[wV])
          mkLambdaFVars #[h'', capsV, wV] bInst
      let capsTuple ← mkTupleN caps
      -- hoist the generic body itself to a named `@[reducible]`
      -- constant (the D40 law: knot bodies are top-level names; the
      -- inline form is kernel-pathological at composed-body scale —
      -- the hand system's `pcSeqF` lesson). Binders: the data params +
      -- the H-free post-`H` fvars the body references raw.
      let bodyName := knotName ++ `body
      let bodyOuterIds ← fvarClosure
        (Lean.CollectFVars.main genBody {}).fvarIds
      let bodyOuter := (bodyOuterIds.filter (· != hFvar)).map mkFVar
      let bodyVal ← instantiateMVars
        (← mkLambdaFVarsOpaque bodyOuter genBody)
      unless (← getEnv).contains bodyName do
        let bodyDecl := Declaration.defnDecl {
          name := bodyName, levelParams := [],
          type := (← inferType bodyVal), value := bodyVal,
          hints := .regular (Lean.getMaxHeight (← getEnv) bodyVal + 1),
          safety := .safe }
        try Lean.addAndCompile bodyDecl
        catch _ => addDecl bodyDecl
        Lean.enableRealizationsForConst bodyName
        setReducibilityStatus bodyName .reducible
        modifyEnv fun env => inlineRegistry.addEntry env bodyName
      let bodyApp := mkAppN (mkConst bodyName) bodyOuter
      -- H.fix / H.fixTick with Γ passed explicitly (higher-order
      -- unification would otherwise pick the non-abstracting Γ)
      let knotBody ←
        if isTick then
          mkAppOptM ``HydroSem.fixTick
            #[none, none, some hExpr, some gamma, none, none,
              some f, some capsTuple, some bodyApp]
        else
          mkAppOptM ``HydroSem.fix
            #[none, none, some hExpr, some gamma, none, none, none,
              none, none, some f, some capsTuple, some bodyApp]
      let val ← instantiateMVars
        (← mkLambdaFVarsOpaque
          (dataPreE ++ #[hExpr] ++ postHE ++ paramWs) knotBody)
      let ty ← inferType val
      unless (← getEnv).contains knotName do
        let decl := Declaration.defnDecl {
          name := knotName, levelParams := [], type := ty, value := val,
          hints := .regular (Lean.getMaxHeight (← getEnv) val + 1),
          safety := .safe }
        try Lean.addAndCompile decl
        catch _ => addDecl decl
        Lean.enableRealizationsForConst knotName
        pendingKnots.modify (·.push knotName)
      pure (mkAppN (mkConst knotName) (dataPreE ++ #[hExpr] ++ postHE))
    -- single-source form: hoist the whole chain as a named module
    -- (`declName ++ body` — the D40 chain-body artifact, generated),
    -- queued for the enclosing
    -- `hydro def`'s registration pass FIRST (innermost-first); the
    -- per-wire slicing below then projects the registered constant,
    -- so the generation walk has its module boundary
    let mut body := body
    let mut chainSpine : Array (Name × Nat) := #[]
    let mut nTotal := n
    let chainName := declName ++ `body
    if singleSource then
      -- tuple only the lets the post-`complete` continuation mentions
      let contIdents : Std.HashSet Name :=
        match bodyStx.find? (·.isOfKind ``completeTerm) with
        | some c => collectIdents c[3]
        | none => {}
      let (chainBody, spine) ← extendChain n rawBody
        (fun nm => contIdents.contains nm.eraseMacroScopes) #[]
      chainSpine := spine
      nTotal := n + (spine.map (·.2)).foldl (·+·) 0
      let chainVal ← instantiateMVars
        (← mkLambdaFVarsOpaque (dataPreE ++ #[hExpr] ++ postHE ++ ws)
          chainBody)
      unless (← getEnv).contains chainName do
        let chainDecl := Declaration.defnDecl {
          name := chainName, levelParams := [],
          type := (← inferType chainVal), value := chainVal,
          hints := .regular (Lean.getMaxHeight (← getEnv) chainVal + 1),
          safety := .safe }
        try Lean.addAndCompile chainDecl
        catch _ => addDecl chainDecl
        Lean.enableRealizationsForConst chainName
        pendingKnots.modify (·.push chainName)
      body := mkAppN (mkConst chainName)
        (dataPreE ++ #[hExpr] ++ postHE ++ ws)
      -- the replaced body references every postH fvar
      let bodyIds2 := (Lean.CollectFVars.main body {}).fvarIds
      capsCell.set (postH.filter fun f =>
        bodyIds2.contains f && hDep f)
    -- Bekić: emit K₁ … Kₙ; component i's body substitutes earlier
    -- wires by their knots re-closed at (loop var, later params)
    let mut kApps : Array Expr := #[]
    for i in [0:n] do
      let τw := τs[i]!
      let isTick :=
        τw.getAppFn.constName? == some ``HydroSem.Ticked
      let mut compBody ← tupleProj (← zetaLets body) i nTotal
      for j in [0:i] do
        if compBody.containsFVar ws[j]!.fvarId! then
          compBody := compBody.replaceFVar ws[j]!
            (← closeWire kApps ws i n j ws[i]!)
      let paramWs := (Array.range n).filterMap fun k =>
        if k > i then some ws[k]! else none
      let kApp ← hoist (declName ++ names[i]!) isTick ws[i]! τw paramWs
        compBody fuels[i]!
      kApps := kApps.push kApp
    -- ==== eager knot-stack generation ====
    -- the hoisted chain + knots' generated vocabulary (couple/glue
    -- namings, `K_co_*`, `K_mono₁`, `K.stages*`) becomes available to
    -- ghost clauses and prove legs of the ENCLOSING `hydro def` — the
    -- knot induction can cite the any-body Kleene-chain lemmas from
    -- inside the program. The post-def pipeline pass skips
    -- already-generated stacks (every `run*T` is idempotent); only
    -- the param phase stays post-def.
    let hintsE ← pendingHints.get
    for k in (← pendingKnots.get) do
      HydroGen.registerSpec k
      -- idempotence across a nested fix's re-elaboration (the pending
      -- list spans the whole command; an inner fix's knots were
      -- generated by ITS eager pass)
      if (← getEnv).contains
          (k.appendAfter s!"_co_sr{HydroGen.subscript 1}") then
        continue
      let env ← getEnv
      let gi := (HydroGen.getGenInfo env k).getD {}
      let dbg := (← IO.getEnv "HYDRO_DEF_DBG").isSome
      if gi.hasFix then
        if dbg then IO.eprintln s!"[hydro def] knot stack {k}"
        HydroGen.runKnotStackT k hintsE
      else
        if gi.callees.any (HydroGen.reachesKnotN env) then
          if dbg then IO.eprintln s!"[hydro def] glue {k}"
          HydroGen.runGlueT k hintsE
        else
          if dbg then IO.eprintln s!"[hydro def] couple {k}"
          HydroGen.runCoupleT k hintsE
        if dbg then IO.eprintln s!"[hydro def] causal {k}"
        HydroGen.runModCausalT k hintsE
        if dbg then IO.eprintln s!"[hydro def] wf {k}"
        HydroGen.runModWfT k hintsE
        if dbg then IO.eprintln s!"[hydro def] mono {k}"
        HydroGen.runModMonoT k
      if dbg then IO.eprintln s!"[hydro def] done {k}"
    -- ==== the `invariant` clause (Verus-ordered) ====
    -- packaged bounded-chain induction over the just-generated
    -- `stages` vocabulary; hoists `<def>.<wire>.inv` for the prove
    -- legs — no stage index escapes the construct
    if let some invStx := invStx? then
      unless singleSource do
        throwErrorAt invStx "fix invariant: only single-source \
          (`complete`) fix blocks are supported"
      elabFixInvariantClause declName names chainName n nTotal invStx
    -- closed wires (outermost = last component)
    let kAppsF := kApps
    let mut closed : Array Expr := Array.replicate n (mkConst ``Unit.unit)
    for ridx in [0:n] do
      let i := n - 1 - ridx
      let mut wireArgs : Array Expr := #[]
      for k in [i+1:n] do
        wireArgs := wireArgs.push closed[k]!
      closed := closed.set! i (mkAppN kAppsF[i]! wireArgs)
    match restStx? with
    | some restStx =>
      bindClosed names τs closed ws 0 restStx expectedType?
    | none =>
      -- pass 2 (single-source): re-elaborate the body with the wires
      -- bound to the closed knots; `complete` skips to its inline
      -- continuation, the chain's `let`s stay in scope — their values
      -- swapped for projections of the hoisted chain body (the module
      -- value computes through the registered constant)
      let chainApp := mkAppN (mkConst chainName)
        (dataPreE ++ #[hExpr] ++ postHE ++ ws)
      fixPassMode.modify (false :: ·)
      try
        bindClosed names τs closed ws 0 bodyStx expectedType?
          (post := swapChainLets chainSpine chainApp n nTotal 0 0)
      finally fixPassMode.modify (·.drop 1)

@[term_elab completeTerm] def elabComplete : TermElab :=
    fun stx expectedType? => do
  match ← fixPassMode.get with
  | [] =>
    throwErrorAt stx "complete: only allowed inside a single-source \
      `fix` body"
  | true :: _ =>
    -- pass 1: the wire-closing tuple IS the knot body value
    elabTerm stx[1] expectedType?
  | false :: rest =>
    -- pass 2: wires are closed; continue below the marker. This
    -- marker is CONSUMED — the continuation belongs to the enclosing
    -- fix (if any), so its own marker must see its own pass
    fixPassMode.set rest
    try elabTerm stx[3] expectedType?
    finally fixPassMode.modify (false :: ·)

end HydroFix

/-! ## The ghost layer (Half 2)

Contract-face and proof-structuring sugar (active wherever
`Hydro.HydroDef` is imported):

    hydro def M (H : HydroSem L mem) (args…) :
        τ ensures out => P out := 
      let a := …
      ghost let g := ‹spec-only value›
      ghost have h : S := proof
      (value…)
      prove
        field₁ := e₁,
        field₂ := e₂

- `τ ensures out => P` (type position) elaborates to the standard
  contract face `{out : τ // ∀ hv : H = Values L mem, match H, hv,
  x₁…xₖ, out with | _, rfl, x₁…xₖ, out => P}`, transporting every
  binder whose type mentions the interpretation variable — the
  hand-written `∀ hv`/`match` ritual is generated. The `hydro def`
  command elaborates this pair under `M._spec` and derives the module
  `M : ∀ xs, τ` (the plain program — a non-dependent function) and the
  face theorem `M.ensures : ∀ xs[H := Values L mem], P xs (M (Values L
  mem) xs)` from it (`HydroGhost.deriveModule`): call sites consume
  `M H …` and `M.ensures …` directly (no `.val`, no `.property rfl`),
  and a callee application is an ordinary function application whose
  arguments simp can rewrite (FINDINGS 0c-iii).
- `ghost have`/`ghost let` interleave in the `let` chain: spec-only
  bindings stated at the program point where they hold, **stripped
  from the computational leg** and replayed (in order) inside the
  proof leg after `intro hv; subst hv` — so they are stated under the
  `Values` substitution with every computational `let` in scope.
  Since the interpretation variable is eliminated by `subst`, ghost
  statements spell the instance explicitly (`Values L mem`), exactly
  as the hand-written proofs do.
- `value prove f₁ := e₁, …` closes the def: the computational value,
  paired with the contract proof assembled field-by-field (structure
  contracts) with all ghost facts in scope.

Binary/relational statements (Flo monotonicity, eager naming) are NOT
faces: they are the generated `_param` free theorems instantiated at
the matching `HRelC` instance (`monoC`/`eagC`) — see `MonoHRel.lean`. -/

namespace HydroGhost

/-- `τ ensures out => P` — the contract-face former (type position of
a `hydro def`). -/
syntax:10 (name := ensuresTerm) term:11 " ensures " ident " => " term : term

/-- `ghost have h : S := prf` — a spec-only fact at this program
point (proof-leg only). The type ascription is optional for
sub-contract fetches (`….property rfl`). -/
syntax (name := ghostHave)
  withPosition("ghost " "have " ident (" : " term)? " := " term)
  optSemicolon(term) : term

/-- `ghost obtain ⟨…⟩ := e` — destructure a spec-only fact at this
program point (proof-leg only). -/
syntax (name := ghostObtain)
  withPosition("ghost " "obtain " rcasesPat " := " term)
  optSemicolon(term) : term

/-- `ghost let x := v` — a spec-only value at this program point
(proof-leg only). -/
syntax (name := ghostLet)
  withPosition("ghost " "let " ident (" : " term)? " := " term)
  optSemicolon(term) : term

/-- `ghost witness e` — choose the witness of an existential
`ensures` face at this program point (proof-leg only): the contract
`∃ w, P w` is refined to `P e`, with every earlier ghost fact in
scope. -/
syntax (name := ghostWitness)
  withPosition("ghost " "witness " term)
  optSemicolon(term) : term

/-- `ghost intro h₁ h₂ …` — name the hypotheses of an implication-
shaped `ensures` face (the `requires` analogue: `ensures out => R → P`
introduces `R` here, so later ghost clauses can use it). -/
syntax (name := ghostIntro)
  withPosition("ghost " "intro " ident+)
  optSemicolon(term) : term

/-- `ghost subst h` — substitute an equation hypothesis (typically one
named by `ghost intro`) into the contract goal and every ghost fact. -/
syntax (name := ghostSubst)
  withPosition("ghost " "subst " ident)
  optSemicolon(term) : term

/-- `value prove f₁ := e₁, f₂ := e₂, …` — close an `ensures`-faced def:
the computational value plus the field-by-field contract assembly. -/
syntax proveField := ident " := " term
syntax:10 (name := proveTerm)
  term:11 " prove " proveField,+ : term

open Lean Parser Elab Term Meta

@[term_elab ensuresTerm] def elabEnsuresTerm : TermElab :=
    fun stx _expectedType? => do
  let τStx : TSyntax `term := ⟨stx[0]⟩
  let outId : Ident := ⟨stx[2]⟩
  let pStx : TSyntax `term := ⟨stx[4]⟩
  -- the interpretation variable: the unique `HydroSem`-typed fvar
  let lctx ← getLCtx
  let mut hFvar? : Option LocalDecl := none
  for d in lctx do
    if !d.isImplementationDetail then
      if (← instantiateMVars d.type).getAppFn.constName?
          == some ``HydroSem then
        if hFvar?.isSome then
          throwErrorAt stx "ensures: multiple `HydroSem` variables"
        hFvar? := some d
  let some hDecl := hFvar?
    | throwErrorAt stx "ensures: no `HydroSem` interpretation variable \
        in scope"
  let hTy ← instantiateMVars hDecl.type
  -- `HydroSem L mem`
  let lStx ← exprToSyntax hTy.getAppArgs[0]!
  let memStx ← exprToSyntax hTy.getAppArgs[1]!
  let hId := mkIdent hDecl.userName
  -- transported binders: everything whose type depends on `H`
  let mut deps : Array Ident := #[]
  for d in lctx do
    if !d.isImplementationDetail && d.fvarId != hDecl.fvarId then
      if (← instantiateMVars d.type).containsFVar hDecl.fvarId then
        deps := deps.push (mkIdent d.userName)
  let face ← `({ $outId : $τStx //
    ∀ hv : $hId = Values $lStx $memStx,
      match $hId:term, hv, $[$deps:term],*, $outId:term with
      | _, rfl, $[$deps:term],*, $outId:term => $pStx })
  elabTerm face none

@[term_elab ghostHave] def elabGhostHave : TermElab :=
    fun stx expectedType? => do
  let name : Ident := ⟨stx[2]⟩
  let val : TSyntax `term := ⟨stx[5]⟩
  let tac ← match stx[3].getArgs with
    | #[] => `(tactic| have $name := $val)
    | tyArgs => do
      let ty : TSyntax `term := ⟨tyArgs[1]!⟩
      `(tactic| have $name : $ty := $val)
  unless (← HydroFix.fixPassMode.get).contains true do
    pendingGhosts.modify (·.push tac)
  elabTerm stx[7] expectedType?

@[term_elab ghostObtain] def elabGhostObtain : TermElab :=
    fun stx expectedType? => do
  let pat : TSyntax `rcasesPat := ⟨stx[2]⟩
  let val : TSyntax `term := ⟨stx[4]⟩
  let tac ← `(tactic| obtain $pat := $val)
  unless (← HydroFix.fixPassMode.get).contains true do
    pendingGhosts.modify (·.push tac)
  elabTerm stx[6] expectedType?

@[term_elab ghostLet] def elabGhostLet : TermElab :=
    fun stx expectedType? => do
  let name : Ident := ⟨stx[2]⟩
  let val : TSyntax `term := ⟨stx[5]⟩
  let tac ← match stx[3].getArgs with
    | #[] => `(tactic| let $name := $val)
    | tyArgs => do
      let ty : TSyntax `term := ⟨tyArgs[1]!⟩
      `(tactic| let $name : $ty := $val)
  unless (← HydroFix.fixPassMode.get).contains true do
    pendingGhosts.modify (·.push tac)
  elabTerm stx[7] expectedType?

@[term_elab ghostWitness] def elabGhostWitness : TermElab :=
    fun stx expectedType? => do
  let val : TSyntax `term := ⟨stx[2]⟩
  let tac ← `(tactic| refine ⟨$val, ?_⟩)
  unless (← HydroFix.fixPassMode.get).contains true do
    pendingGhosts.modify (·.push tac)
  elabTerm stx[4] expectedType?

@[term_elab ghostIntro] def elabGhostIntro : TermElab :=
    fun stx expectedType? => do
  let names : Array Ident := stx[2].getArgs.map (⟨·⟩)
  let tac ← `(tactic| intro $[$names:ident]*)
  unless (← HydroFix.fixPassMode.get).contains true do
    pendingGhosts.modify (·.push tac)
  elabTerm stx[4] expectedType?

@[term_elab ghostSubst] def elabGhostSubst : TermElab :=
    fun stx expectedType? => do
  let name : Ident := ⟨stx[2]⟩
  let tac ← `(tactic| subst $name:ident)
  unless (← HydroFix.fixPassMode.get).contains true do
    pendingGhosts.modify (·.push tac)
  elabTerm stx[4] expectedType?

@[term_elab proveTerm] def elabProveTerm : TermElab :=
    fun stx expectedType? => do
  let valStx : TSyntax `term := ⟨stx[0]⟩
  let fields := stx[2].getSepArgs
  let ghosts ← pendingGhosts.get
  pendingGhosts.set #[]
  let fieldItems : Array (TSyntax ``Parser.Term.structInstField) ←
    fields.mapM fun f => do
      let name : Ident := ⟨f[0]⟩
      let val : TSyntax `term := ⟨f[2]⟩
      `(Parser.Term.structInstField| $name:ident := $val)
    let assembled ← `({ $fieldItems:structInstField,* })
    let prf ← if ghosts.isEmpty then
      `(by
        intro hv
        subst hv
        exact $assembled)
    else
      `(by
        intro hv
        subst hv
        $[$ghosts:tactic]*
        exact $assembled)
  let pair ← `(⟨$valStx, $prf⟩)
  elabTerm pair expectedType?

/-- The computational value of a contract-faced pair: the `let` chain
with `Subtype.mk … val prf` at its tail replaced by `val`. -/
private partial def stripPair (e : Expr) : MetaM Expr := do
  match e with
  | .letE n t v b nd => return .letE n t v (← stripPair b) nd
  | .mdata _ b => stripPair b
  | _ =>
    if e.isAppOfArity ``Subtype.mk 4 then return e.getArg! 2
    throwError "hydro def: contract pair not found at the body's tail:\
      {indentExpr e}"

/-- **The module from its contract-faced pair.** `hydro def M … :
τ ensures out => P := body prove …` elaborates the pair under
`M._spec : ∀ xs, {out : τ // ∀ hv : H = Values L mem, match … => P}`;
this derives

- `M : ∀ xs, τ` — the plain program (the pair's value with the
  `Subtype.mk` stripped): a NON-dependent function, so a callee's
  arguments are ordinary rewrite positions for simp (the dependent
  `{out // … args …}` result type made them fixed, which is why callee
  namings never fired under a callee application — FINDINGS 0c-iii);
- `M.ensures : ∀ xs[H := Values L mem], P xs (M (Values L mem) xs)` —
  the face, stated directly at the denotation (no `hv`/`match`
  ritual), proved by `(M._spec … ).property rfl` (one delta each side).

A pair without an `ensures` face (plain `τ`) derives `M` only. -/
def deriveModule (specName mName : Name) : TermElabM Unit := do
  let ci ← getConstInfo specName
  let some specVal := ci.value?
    | throwError "hydro def: {specName} has no value"
  let lvls := ci.levelParams.map mkLevelParam
  -- the module's type: the face's carrier
  let (mTy, isFace) ← forallTelescope ci.type fun xs b => do
    if b.isAppOfArity ``Subtype 2 then
      pure (← mkForallFVars xs (b.getArg! 0), true)
    else pure (← mkForallFVars xs b, false)
  let mVal ← if isFace then
      lambdaTelescope specVal fun xs body => do
        mkLambdaFVars xs (← stripPair body)
    else pure specVal
  let mVal ← instantiateMVars mVal
  addAndCompile (.defnDecl {
    name := mName, levelParams := ci.levelParams, type := mTy, value := mVal,
    hints := .regular (getMaxHeight (← getEnv) mVal + 1), safety := .safe })
  -- the generators register the module as a simp unfold target, which
  -- realizes auxiliary constants (`eq_def`)
  enableRealizationsForConst mName
  if let some doc ← findDocString? (← getEnv) specName then
    addDocStringCore mName doc
  unless isFace do return
  -- the face theorem, at `H := Values L mem`
  let hIdx? ← forallTelescope ci.type fun xs _ => do
    xs.findIdxM? fun x => do
      pure ((← inferType x).getAppFn.isConstOf ``HydroSem)
  let some hIdx := hIdx?
    | throwError "hydro def: {mName}: no `HydroSem` binder"
  forallBoundedTelescope ci.type hIdx fun pre tyH => do
    let hTy := tyH.bindingDomain!
    let V := mkApp2 (mkConst ``Values) (hTy.getArg! 0) (hTy.getArg! 1)
    let restV := tyH.bindingBody!.instantiate1 V
    forallTelescope restV fun post b => do
      let pred := b.getArg! 1
      let allArgs := pre ++ #[V] ++ post
      let mApp := mkAppN (mkConst mName lvls) allArgs
      let rflV ← mkEqRefl V
      let s0 := (mkApp pred mApp).headBeta
      unless s0.isForall do
        throwError "hydro def: {mName}: unexpected face shape:{indentExpr s0}"
      let s1 := s0.bindingBody!.instantiate1 rflV
      -- the `match H, hv, … with | _, rfl, … => P` on `rfl`: one iota
      let s2 ← whnfCore s1
      let s2 ← if s2.isAppOf ``Eq.rec || s2.isAppOf ``Eq.ndrec then whnf s1 else pure s2
      let prf := mkApp
        (← mkProjection (mkAppN (mkConst specName lvls) allArgs) `property) rflV
      let thmTy ← mkForallFVars (pre ++ post) s2
      let thmVal ← mkLambdaFVars (pre ++ post) prf
      addDecl (.thmDecl {
        name := mName ++ `ensures, levelParams := ci.levelParams,
        type := ← instantiateMVars thmTy, value := ← instantiateMVars thmVal })

end HydroGhost


namespace HydroGen

open Lean Parser Elab Command

/-! ## The decision census (the nondet / sched-det split)

The decision families of `HydroSem` divide into three kinds (see the
classification table in `Sem.lean`): **nondets** (protocol
nondeterminism, real at `Values`, mirrors of Rust `nondet!` sites —
what proofs reason about), **sched-dets** (adversary/runtime freedom,
`Unit` at `Values` — the transports, silently ∀-quantified like the
delivery cursors), and **fuels** (the `fix`
unfolding artifact). `#nondet_census M (nondets := a) (scheds := b)
(fuels := c)` walks `M`'s signature — raw decision binders and
decision-record binders alike, recursively through nested records —
and checks the leaf counts, so each module's doc-cited Rust `nondet!`
tally stays honest by build. -/

namespace Census

open Lean Elab Command Meta

/-- Leaf classification of a decision type by its head constant. -/
inductive DecKind where
  | nondet | sched | fuel
  deriving BEq, Repr

private def classifyHead : Name → Option DecKind
  | ``HydroSem.SnapDec | ``HydroSem.BatchDec | ``HydroSem.OrdBatchDec
  | ``HydroSem.OrderSelDec
  | ``HydroSem.SampleDec | ``HydroSem.TimerDec | ``HydroSem.PulseDec =>
    some .nondet
  | ``HydroSem.TransportDec => some .sched
  | ``HydroSem.FixDec => some .fuel
  | _ => none

/-- Collect decision leaves (with their access paths) of a type:
family applications are leaves; structure types recurse through their
fields. -/
private partial def leavesOf (env : Environment) (path : String)
    (ty : Expr) : MetaM (Array (String × DecKind)) := do
  let ty ← instantiateMVars ty
  match ty.getAppFn.constName? with
  | some h =>
    if let some k := classifyHead h then
      return #[(path, k)]
    else if isStructure env h then
      let mut acc := #[]
      for f in getStructureFields env h do
        let projTy := (← getConstInfo (h ++ f)).type
        let fieldTy ← forallTelescope projTy fun _ body => pure body
        acc := acc ++ (← leavesOf env (path ++ "." ++ f.toString) fieldTy)
      return acc
    else
      return #[]
  | none => return #[]

syntax "#nondet_census " ident ("(" ident " := " num ")")+ : command

elab_rules : command
  | `(command| #nondet_census $M:ident $[($labels:ident := $nums:num)]*) => do
    let mut a := 0; let mut b := 0; let mut c := 0
    for (l, n) in labels.zip nums do
      match l.getId with
      | `nondets => a := n.getNat
      | `scheds => b := n.getNat
      | `fuels => c := n.getNat
      | _ => throwErrorAt l "#nondet_census: unknown label `{l.getId}` \
          (expected nondets/scheds/fuels)"
    let mName ← liftCoreM <| realizeGlobalConstNoOverload M
    liftTermElabM do
      let env ← getEnv
      let info ← getConstInfo mName
      let leaves ← forallTelescope info.type fun xs _ => do
        let mut acc := #[]
        for x in xs do
          let d ← x.fvarId!.getDecl
          acc := acc ++ (← leavesOf env d.userName.toString d.type)
        pure acc
      let count (k : DecKind) := leaves.filter (·.2 == k) |>.size
      let found := (count .nondet, count .sched, count .fuel)
      let expect := (a, b, c)
      unless found == expect do
        let listing := leaves.foldl (fun s l =>
          s ++ s!"\n  {l.1} : {repr l.2}") ""
        throwErrorAt M "#nondet_census {mName}: found (nondets, scheds, \
          fuels) = {found}, expected {expect}; leaves:{listing}"
      logInfo m!"#nondet_census {mName}: nondets {found.1}, \
        scheds {found.2.1}, fuels {found.2.2}"

end Census

/-- `hydro [hints]? def M … := …` — the module annotation: elaborate
(with the `fix` block syntax in scope), register the composition spec
— for the knots the `fix` elaborator hoisted first — and run the
generation pipeline for each. -/
elab doc:(docComment)? "hydro " hints:(hydroGenIds)?
    d:Parser.Command.declaration : command => do
  -- graft a leading doc comment onto the wrapped declaration's
  -- modifiers (so `/-- … -/ hydro def M` documents `M`)
  let d ← match doc with
    | none => pure d
    | some dc =>
      let mods := d.raw[0]
      if mods.isOfKind ``Parser.Command.declModifiers
          && mods[0].getArgs.isEmpty then
        let mods := mods.setArg 0 (mkNullNode #[dc.raw])
        pure ⟨d.raw.setArg 0 mods⟩
      else
        throwErrorAt d "hydro def: duplicate doc comment"
  HydroFix.pendingKnots.set #[]
  HydroFix.pendingInvs.set #[]
  HydroGhost.pendingGhosts.set #[]
  let hintNames ← elabExtras (hints.map (·.raw))
  HydroFix.pendingHints.set hintNames
  let some declId := d.raw.find? (·.isOfKind ``Parser.Command.declId)
    | throwErrorAt d "hydro def: no declaration name found"
  -- a module elaborates its contract-faced pair under `M._spec`; the
  -- module `M` (the plain program — a non-dependent function) and its
  -- face theorem `M.ensures` are derived from it below.
  let specIdStx : Syntax :=
    mkIdentFrom declId[0] (declId[0].getId ++ `_spec)
  let d : TSyntax ``Parser.Command.declaration := ⟨d.raw.replaceM (m := Id) fun n =>
    if n == declId then some (declId.setArg 0 specIdStx) else none⟩
  try
    elabCommand d
  finally
    HydroFix.pendingHints.set #[]
  let specName ← liftCoreM <| realizeGlobalConstNoOverload specIdStx
  let mName := HydroGhost.moduleOfSpec specName
  liftTermElabM <| HydroGhost.deriveModule specName mName
  -- the emitted knots first (emission order = innermost-first), then
  -- the module itself
  let knots := (← HydroFix.pendingKnots.get).foldl
    (fun (acc : Array Name) k => if acc.contains k then acc else acc.push k)
    #[]
  HydroFix.pendingKnots.set #[]
  for k in knots do
    liftTermElabM <| registerSpec k
    logInfo m!"hydro def {mName}: emitted knot {k}"
    runPipeline k hintNames
  -- replay the packaged invariant inductions (proven inside a possibly
  -- speculative elaboration branch) into the persistent env
  let invs ← HydroFix.pendingInvs.get
  HydroFix.pendingInvs.set #[]
  for (n, stmt, val) in invs do
    unless (← getEnv).contains n do
      liftTermElabM <| HydroGen.addThm n stmt val
      logInfo m!"hydro def {mName}: replayed {n}"
  liftTermElabM <| registerSpec mName
  let gi := (getGenInfo (← getEnv) mName).getD {}
  logInfo m!"hydro def {mName}: callees {gi.callees}, \
    inlines {gi.inlines}, fix := {gi.hasFix}"
  runPipeline mName hintNames


end HydroGen

end Hydro
