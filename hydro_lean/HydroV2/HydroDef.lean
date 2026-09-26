import HydroV2.HydroGenKnot
import HydroV2.HydroParam

/-!
# `hydro def`: the single program-module annotation

One annotation per module — Rust-attribute style (user ruling: no
separate registration/generation command tails in program files):

    hydro def M (H : HydroSem L mem) … := <the Rust-mirror body>
    hydro [p1bPairDecEq] def M …   := …   -- unfold hints for the scripts
    hydro inline def C …           := …   -- wrapper marking (`leCore`)

`hydro def` elaborates the definition, validates + records its
composition spec (the strict module grammar of D45), and then runs the
**whole generation pipeline** automatically, with the phase set routed
from the spec:

- the def itself contains `HydroSem.fix`/`fixTick` (a knot wrapper) →
  the structural knot route (`hydro_knot`) + the free theorem
  (`hydro_param`);
- a callee transitively reaches a knot (a glue module:
  `leader_election`, `pcBody`, `paxos_core`) → simp-discovery namings
  (`hydro_glue`) + causality + wf threading + monotonicity + the free
  theorem;
- otherwise (a knot-free content module) → the choice-route namings
  (`hydro_couple`) + causality + wf + mono + the free theorem.

Elaboration budgets stay call-site-visible (user ruling): a module
whose body or scripts need more than the default budget carries a
`set_option maxHeartbeats … in` prefix on the whole `hydro def`
command — one bump, visible where it is paid.

The `hydro_*` phase commands remain available (the engine's validation
files use them); program files use only the annotation.
-/

namespace HydroV2

namespace HydroGen

open Lean Elab Command

/-- Synthesize the extras bracket for a phase command. -/
private def hintsStx (hints : Array Ident) :
    CommandElabM (TSyntax ``hydroGenIds) :=
  `(hydroGenIds| [$[$hints],*])

/-- Run the generation pipeline for a registered module, phase set
routed from the recorded composition spec. -/
def runPipeline (mName : Name) (hints : Array Ident) :
    CommandElabM Unit := do
  let env ← getEnv
  let gi := (getGenInfo env mName).getD {}
  let M : Ident := mkIdent mName
  let ids ← hintsStx hints
  if gi.hasFix then
    -- knot wrapper: the structural route
    elabCommand (← `(hydro_knot $M $ids:hydroGenIds))
    elabCommand (← `(hydro_param $M $ids:hydroGenIds))
  else if gi.callees.any (reachesKnotN env) then
    -- glue module: simp-discovery namings over folded callees
    elabCommand (← `(hydro_glue $M $ids:hydroGenIds))
    elabCommand (← `(hydro_causal $M $ids:hydroGenIds))
    elabCommand (← `(hydro_wf $M $ids:hydroGenIds))
    elabCommand (← `(hydro_mono $M))
    elabCommand (← `(hydro_param $M $ids:hydroGenIds))
  else
    -- knot-free content module: the choice-route namings
    elabCommand (← `(hydro_couple $M $ids:hydroGenIds))
    elabCommand (← `(hydro_causal $M $ids:hydroGenIds))
    elabCommand (← `(hydro_wf $M $ids:hydroGenIds))
    elabCommand (← `(hydro_mono $M))
    elabCommand (← `(hydro_param $M $ids:hydroGenIds))

end HydroGen

/-! ## The inline `fix` block (Half 1)

Term syntax (active wherever `HydroV2.HydroDef` is imported):

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

namespace HydroFix

/-- `(w : τ)` — one cycle-wire binder. -/
syntax fixBinder := "(" ident " : " term ")"

/-- The `fix` block, `let`-style (position-disciplined body, rest after
a `;` or linebreak). An optional `invariant` clause is parsed and
reserved for the Half-2 ghost layer. -/
syntax (name := fixTerm)
  withPosition("fix " fixBinder+ " via " term (" invariant " term)?
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

/-- Single-source `fix` elaboration mode: `none` = not inside one
(`complete` is an error); `some true` = pass 1 (slice the chain up to
`complete` — the continuation is NOT elaborated and ghost clauses do
not queue); `some false` = pass 2 (wires closed; `complete` skips to
its continuation). -/
initialize fixPassMode : IO.Ref (Option Bool) ← IO.mkRef none

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
body then exposes the chain values by projection (the pcBody/leBody
shape, now generated). Tupled: lets the continuation references,
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

@[term_elab fixTerm] def elabFixTerm : TermElab := fun stx expectedType? => do
  let binders := stx[1].getArgs
  let fuelsStx := stx[3]
  unless stx[4].getArgs.isEmpty do
    throwErrorAt stx[4] "fix: `invariant` clauses are reserved for the \
      ghost layer (Half 2) and not yet supported"
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
  let τs ← tyStxs.mapM fun t => elabType t
  let fuelStxs ← splitFuels fuelsStx n
  let decls : Array (Name × (Array Expr → TermElabM Expr)) :=
    (names.zip τs).map fun (nm, τ) => (nm, fun _ => pure τ)
  withLocalDeclsD decls fun ws => do
    let prodTy ← mkProdN τs
    -- pass 1 (single-source): the `complete` marker returns the
    -- wire-closing tuple; the inline continuation is not elaborated
    -- and ghost clauses do not queue
    if singleSource then fixPassMode.set (some true)
    let body ←
      try elabTermEnsuringType bodyStx prodTy
      finally if singleSource then fixPassMode.set none
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
    -- inline let-bound aliases whose VALUES are `H`-free (module
    -- aliases like `let core := leCore variant …`); `H`-dependent
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
    let capsIds := postH.filter fun f =>
      bodyIds.contains f && hDep f
    let dataPreE := dataPre.map mkFVar
    let postHE := postH.map mkFVar
    let capsE := capsIds.map mkFVar
    let hTy ← inferType hExpr
    -- hoist one component to a top-level knot def; returns the
    -- constant applied to dataPre + H + postH (wire params pending)
    let hoist := fun (knotName : Name) (isTick : Bool) (w τw : Expr)
        (paramWs : Array Expr) (b f : Expr) => do
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
      let bodyVal ← instantiateMVars (← mkLambdaFVars bodyOuter genBody)
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
        (← mkLambdaFVars (dataPreE ++ #[hExpr] ++ postHE ++ paramWs)
          knotBody)
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
    -- (`declName ++ body` — the elaboration artifact that used to be
    -- the hand-written `pcBody`/`leBody`), queued for the enclosing
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
        (← mkLambdaFVars (dataPreE ++ #[hExpr] ++ postHE ++ ws) chainBody)
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
    -- Bekić: emit K₁ … Kₙ; component i's body substitutes earlier
    -- wires by their knots re-closed at (loop var, later params)
    let mut kApps : Array Expr := #[]
    for i in [0:n] do
      let τw := τs[i]!
      let isTick :=
        τw.getAppFn.constName? == some ``HydroSem.TickSingleton
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
      fixPassMode.set (some false)
      try
        bindClosed names τs closed ws 0 bodyStx expectedType?
          (post := swapChainLets chainSpine chainApp n nTotal 0 0)
      finally fixPassMode.set none

@[term_elab completeTerm] def elabComplete : TermElab :=
    fun stx expectedType? => do
  match ← fixPassMode.get with
  | none =>
    throwErrorAt stx "complete: only allowed inside a single-source \
      `fix` body"
  | some true =>
    -- pass 1: the wire-closing tuple IS the knot body value
    elabTerm stx[1] expectedType?
  | some false =>
    -- pass 2: wires are closed; continue below the marker
    elabTerm stx[3] expectedType?

end HydroFix

/-! ## The ghost layer (Half 2)

Contract-face and proof-structuring sugar (active wherever
`HydroV2.HydroDef` is imported):

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
  hand-written `∀ hv`/`match` ritual is generated.
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

/-- Pending ghost clauses (tactic replay syntax, source order), queued
by the `ghost` term elaborators and drained by `prove`. -/
initialize pendingGhosts : IO.Ref (Array (TSyntax `tactic)) ← IO.mkRef #[]

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
  unless (← HydroFix.fixPassMode.get) == some true do
    pendingGhosts.modify (·.push tac)
  elabTerm stx[7] expectedType?

@[term_elab ghostObtain] def elabGhostObtain : TermElab :=
    fun stx expectedType? => do
  let pat : TSyntax `rcasesPat := ⟨stx[2]⟩
  let val : TSyntax `term := ⟨stx[4]⟩
  let tac ← `(tactic| obtain $pat := $val)
  unless (← HydroFix.fixPassMode.get) == some true do
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
  unless (← HydroFix.fixPassMode.get) == some true do
    pendingGhosts.modify (·.push tac)
  elabTerm stx[7] expectedType?

@[term_elab ghostWitness] def elabGhostWitness : TermElab :=
    fun stx expectedType? => do
  let val : TSyntax `term := ⟨stx[2]⟩
  let tac ← `(tactic| refine ⟨$val, ?_⟩)
  unless (← HydroFix.fixPassMode.get) == some true do
    pendingGhosts.modify (·.push tac)
  elabTerm stx[4] expectedType?

@[term_elab ghostIntro] def elabGhostIntro : TermElab :=
    fun stx expectedType? => do
  let names : Array Ident := stx[2].getArgs.map (⟨·⟩)
  let tac ← `(tactic| intro $[$names:ident]*)
  unless (← HydroFix.fixPassMode.get) == some true do
    pendingGhosts.modify (·.push tac)
  elabTerm stx[4] expectedType?

@[term_elab ghostSubst] def elabGhostSubst : TermElab :=
    fun stx expectedType? => do
  let name : Ident := ⟨stx[2]⟩
  let tac ← `(tactic| subst $name:ident)
  unless (← HydroFix.fixPassMode.get) == some true do
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

end HydroGhost


namespace HydroGen

open Lean Parser Elab Command

/-- `hydro [hints]? def M … := …` — the module annotation: elaborate
(with the `fix` block syntax in scope), register the composition spec
— for the knots the `fix` elaborator hoisted first — and run the
generation pipeline for each.

`hydro inline def C … := …` — mark a wrapper def (an `H`-generic
abbreviation that is not a module of its own, e.g. `leCore`) to be
opened during spec walks and generation; no pipeline. -/
elab doc:(docComment)? "hydro " hints:(hydroGenIds)?
    inl:("inline ")? d:Parser.Command.declaration : command => do
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
  HydroGhost.pendingGhosts.set #[]
  elabCommand d
  let some declId := d.raw.find? (·.isOfKind ``Parser.Command.declId)
    | throwErrorAt d "hydro def: no declaration name found"
  let mName ← liftCoreM <| realizeGlobalConstNoOverload declId[0]
  if inl.isSome then
    modifyEnv fun env => inlineRegistry.addEntry env mName
    -- inline wrappers must be `@[reducible]`: knot `.body` constants
    -- keep them under a *partial* application where `simp only` can't
    -- reach them, so the param walk's `withReducible` unification has
    -- to cross them by delta (the hand-written `pcSeqF` precedent)
    liftCoreM <| setReducibilityStatus mName .reducible
    logInfo m!"hydro inline def {mName}"
    return
  let hintNames ← elabExtras (hints.map (·.raw))
  let hintIds := hintNames.map Lean.mkIdent
  -- the emitted knots first (emission order = innermost-first), then
  -- the module itself
  let knots := (← HydroFix.pendingKnots.get).foldl
    (fun (acc : Array Name) k => if acc.contains k then acc else acc.push k)
    #[]
  HydroFix.pendingKnots.set #[]
  for k in knots do
    liftTermElabM <| registerSpec k
    logInfo m!"hydro def {mName}: emitted knot {k}"
    runPipeline k hintIds
  liftTermElabM <| registerSpec mName
  let gi := (getGenInfo (← getEnv) mName).getD {}
  logInfo m!"hydro def {mName}: callees {gi.callees}, \
    inlines {gi.inlines}, fix := {gi.hasFix}"
  runPipeline mName hintIds


end HydroGen

end HydroV2
