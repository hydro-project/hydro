import Hydro.HydroGenKnot
import Hydro.HydroGenProject

/-!
# `hydro def`: the program-module command

`hydro def` is the one annotation a program module carries. It elaborates the
module, derives its program and face theorem, and runs the generation
pipeline for the module and for every knot its `fix` blocks hoisted.

The surface forms, one example each:

    hydro def M (H : HydroSem L mem) (x : τ) : σ ensures out => P x out :=
      let a := …
      ghost have h : S := proof
      a prove field := e                       -- a faced module
    hydro [p1bPairDecEq] def M … := …          -- unfold hints for the generated proofs
    fix (w : H.Stream α) via f := … complete (e); rest   -- a cycle, inside a module body

The `tick` block (`HydroTick.lean`) builds on this file's ghost layer.

The command runs, in order:

1. elaborate the declaration under the name `M._spec` (the contract-faced pair
   `∀ xs, {out : τ // …}`), which also hoists each `fix` block's knots
   `<M>.<wire>` and chain body `<M>.body` as top-level constants;
2. derive the program `M : ∀ xs, τ` and the face theorem `M.ensures` from
   `M._spec` (`HydroGhost.deriveModule`);
3. register each hoisted knot's composition spec and run its pipeline, in
   emission order (innermost first);
4. replay the invariant and ghost theorems proven during elaboration into the
   persistent environment (the body may have elaborated in a speculative branch);
5. register `M`'s spec and run its pipeline;
6. inside every pipeline run, project the production graph and its denotation
   lemma (`Project.genProject`).

The pipeline's phase set is routed from the composition spec (`runPipeline`):
a def whose value contains `HydroSem.fix`/`fixTick` is a knot and takes the
knot-stack route (`runKnotStackT`); a def with a callee that transitively
reaches a knot is a glue module and takes `runGlueT` plus step monotonicity,
wf threading and monotonicity; every other def is a knot-free content module
and takes `runCoupleT` plus the same three phases. Every route ends in
`Project.genProject`.

The phases are `TermElabM` cores (`HydroGen.lean`, `HydroGenKnot.lean`,
`HydroGenProject.lean`); `hydro def` is their only command surface, and the
toy suite (`HydroGenToy.lean`, `HydroGenCheck.lean`) pins every artifact
class they produce. The engine files carry no heartbeat budget: a `hydro def`
runs under the file's `maxHeartbeats`, and Lean counts the def and every
declaration it generates as one command.
-/

namespace Hydro

namespace HydroGen

open Lean Elab Command

/-- Run the generation pipeline for a registered module, with the phase set
routed from its recorded composition spec: a knot takes `runKnotStackT`; a
glue module (a callee reaches a knot) takes `runGlueT`, `runModSMonoT`,
`runModWfT`, `runModMonoT`; a knot-free content module takes `runCoupleT`
and the same three; every route ends in `Project.genProject`. With
`HYDRO_PHASE_DIAG` set, `runPipeline` prints each phase's wall time. -/
def runPipeline (mName : Name) (hints : Array Name) :
    CommandElabM Unit := do
  let env ← getEnv
  let gi := (getGenInfo env mName).getD {}
  let phase := fun (what : String) (act : TermElabM Unit) => do
    let t0 ← IO.monoMsNow
    liftTermElabM act
    if (← IO.getEnv "HYDRO_PHASE_DIAG").isSome then
      IO.eprintln s!"[phase] {mName} {what}: {(← IO.monoMsNow) - t0}ms"
  if gi.hasFix then
    -- a knot: the structural route
    phase "knot-stack" <| runKnotStackT mName hints
  else if gi.callees.any (reachesKnotN env) then
    -- a glue module: simp discovers the `sr`/`rr` equations over folded callees
    phase "glue" <| runGlueT mName hints
    phase "smono" <| runModSMonoT mName hints
    phase "wf" <| runModWfT mName hints
    phase "mono" <| runModMonoT mName
  else
    -- a knot-free content module: the `sr`/`rr` equations through
    -- `Classical.choose`
    phase "couple" <| runCoupleT mName hints
    phase "smono" <| runModSMonoT mName hints
    phase "wf" <| runModWfT mName hints
    phase "mono" <| runModMonoT mName
  -- the production graph and its denotation lemma
  phase "project" <| Project.genProject mName

end HydroGen

namespace HydroGhost

/-- Pending ghost clauses as tactic syntax, in source order. The `ghost` term
elaborators and the `fix` construct (the closed-knot `invariant` instance)
queue into it; `prove` drains it. Declared before the `fix` machinery because
`fix` queues into it. -/
initialize pendingGhosts :
    IO.Ref (Array (Lean.TSyntax `tactic)) ← IO.mkRef #[]

/-- Whether the elaborator has queued a goal-shaped ghost clause (`ghost
witness`/`intro`/`subst`, one that refines the `ensures` goal itself). From then
on the elaborator cannot hoist a `ghost have`, because its context is the
refined goal's, so it stays a replayed tactic. -/
initialize ghostGoalShaped : IO.Ref Bool ← IO.mkRef false

/-- The context arity of each hoisted ghost theorem: how many leading binders
bind the closed-over context, as opposed to the fact's own `∀`s. -/
initialize ghostArity : IO.Ref (Std.HashMap Lean.Name Nat) ← IO.mkRef {}

end HydroGhost

/-! ## The `fix` block

Term syntax, active wherever `Hydro.HydroDef` is imported:

    fix (w₁ : τ₁) … (wₙ : τₙ) via (f₁, …, fₙ) := body
    rest

closes the mutual cycle `(w₁, …, wₙ) = body`: each `wᵢ` is in scope in `body`,
which returns the `n`-tuple of wires; the `fᵢ` are the `H.FixDec` fuels, one
per component; `rest` continues with the closed wires in scope. This is Rust's
`forward_ref` reading of a cycle.

The elaborator performs the **Bekić decomposition**: it closes component `i`'s
body over the earlier components' knots and hoists each component to a
top-level knot `<M>.<wire>` whose body is an instance-generic constant
`<M>.<wire>.body` applied to a tuple of captured wires. Kernel defeq therefore
never crosses a knot, and a proof about a knot costs what its body constant
`<M>.<wire>.body` costs. The elaborator records the hoisted names for the
`hydro def` command, which registers them and runs their pipelines before the
enclosing module's own. -/

namespace HydroGhost
/-- The module name of a contract-faced pair's declaration: `hydro def M`
elaborates the `ensures`-typed pair under `M._spec`, so `M._spec ↦ M`. -/
def moduleOfSpec (n : Lean.Name) : Lean.Name :=
  match n with
  | Lean.Name.str p "_spec" => p
  | _ => n
end HydroGhost

namespace HydroFix

/-- `(w : τ)`: one cycle-wire binder of a `fix` block. -/
syntax fixBinder := "(" ident " : " term ")"

/-- One obligation of an `invariant` clause, `base := …` or `step := …`. The
elaborator checks the field names, as it does for `prove` fields. -/
syntax invField := ident " := " term

/-- The `invariant` clause of a `fix` block: the loop invariant of the knot's
Kleene chain, written before the body as in Verus.

    fix (w : τ) via f
      invariant w (le sp le' sp') => P
        , base := proof
        , step := proof
      := body

`w` names a cycle wire (the innermost); the optional parenthesised list names
chain `let`s of the body that the predicate and the proofs may mention, unprimed
for the value at the current stage and primed (`le'`) for the value at the closed
knot. `P` is stated over `w` (the wire at the current stage), `w'` (the wire at
the closed knot) and those names.

The elaborator packages a bounded-chain induction over the knot's generated
`stages` vocabulary and hoists two theorems: `<M>.<w>.inv`, the predicate at the
closed knot, which the elaborator queues for the enclosing field proofs as
`h<w>_inv`; and `<M>.<w>.inv_stages`, the predicate at every stage up to the
fuel. The two obligations state `P` at the program's own chain `let`s, through
binders that carry the `let`s' source names:

- `base`: `P` at the seed. In scope: `w'`, the listed lets at the seed (unprimed)
  and at the closed knot (primed), `htop : w' ⊑ body w'` (the chain top is below
  its image) and the colocated faces `h<let>`/`h<let>'` of every listed let whose
  value is a module application.
- `step`: one body step preserves `P`. In scope: `w` and `w'`, the listed
  lets at `w` (unprimed), at `body w` (`<let>_step`) and at the closed knot
  (primed), the chain order `w ⊑ w'`, `body w ⊑ w'` and `w ⊑ body w`, `htop`,
  the faces `h<let>`, `h<let>_step`, `h<let>'`, and the induction hypothesis.

When the predicate speaks of the wire alone (it mentions no listed let and not
`w'`; the list may still bind lets for the proofs), the body may **assume** it of
its loop input, as a Verus loop invariant does: the elaborator conjoins it into
the hoisted chain module's `requires`, the body's `ghost pre` proofs see it as
`h<w>_inv`, and the knot `wf` feeds `inv_stages` to the body at the stage it
probes (the `wf` only probes the body at Kleene stages, so it needs the stage
form). In that mode the induction is unbounded, so the elaborator offers
`htop` to neither obligation and the `⊑` premises not to `step`.
A predicate over chain lets is a
statement about the body's output, not its input, so it is export-only. -/
syntax fixInvariant :=
  " invariant " ident ("(" ident+ ")")? " => " term
    (", " invField)+

/-- The `fix` block, `let`-style: the body is position-disciplined and the rest
follows after a `;` or a line break; an optional `invariant` clause precedes the
body. The `via` fuel is a `term:max` (a projection or a parenthesised tuple), so
a clause keyword that follows it belongs to the fix and not to the module
clause's trailing parser. Without a trailing rest the block is single-source:
the body carries a `complete` marker and the continuation follows it inline. -/
syntax (name := fixTerm)
  withPosition("fix " fixBinder+ " via " term:max (fixInvariant)?
    " := " term) (optSemicolon(term))? : term

/-- `complete (e₁, …, eₙ)`, inside a single-source `fix` body: the wire-closing
tuple, Rust's `complete_cycle.complete(…)`. The `let` chain above it stays in
scope for the continuation below, where the cycle binders denote the **closed**
knots. -/
syntax (name := completeTerm)
  withPosition("complete " term) optSemicolon(term) : term

open Lean Parser Elab Term Meta HydroGen

/-- The hoisted knot defs of the current command, in emission order, for the
enclosing `hydro def`'s registration pass. -/
initialize pendingKnots : IO.Ref (Array Name) ← IO.mkRef #[]

/-- Theorems proven inside the `fix` and ghost elaboration (`<M>.<w>.inv`,
`<M>.pre.<h>`, `<M>.ghost.<h>`) as closed `(name, statement, value)` triples. The
def's body may elaborate in a speculative branch, and Lean discards that
branch's environment additions (async and incremental elaboration), so the
`hydro def` command re-adds any that are missing; closed expressions cross
branches through the ref. -/
initialize pendingInvs : IO.Ref (Array (Name × Expr × Expr)) ←
  IO.mkRef #[]

/-- The enclosing `hydro def`'s unfold hints, set by the command before the def
elaborates; the eager knot-stack generation inside `fix` elaboration needs
them. -/
initialize pendingHints : IO.Ref (Array Name) ← IO.mkRef #[]

/-- The single-source `fix` pass stack, innermost first (`true` = pass 1,
`false` = pass 2). A `complete` marker belongs to the innermost active fix;
consuming it in pass 2 pops the level, so an enclosing fix's marker inside the
continuation sees its own pass. -/
initialize fixPassMode : IO.Ref (List Bool) ← IO.mkRef []

/-- Split the fuels syntax: a `(f₁, …, fₙ)` tuple for `n > 1`, a plain term for
`n = 1`. -/
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

/-- π_i of a right-nested `n`-tuple expression built from `Prod.mk` literals
(after zeta): structural selection with no `Prod.fst/snd` residue. Each
per-knot body has this shape. A non-literal tuple falls back to projections. -/
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

/-- Zeta-reduce every `let` in an expression. Structural: the kernel pays one
beta per binding and module calls stay folded. -/
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

/-- The Bekić wire closure: `K_j` (applied to data, `H` and deps) closed at the
wire arguments `x_{j+1} … x_{n-1}`, where `x_k` is the loop variable when
`k = i`, the pending param wire when `k > i`, and the recursive closure when
`k < i`. -/
private partial def closeWire (kApps ws : Array Expr) (i n j : Nat)
    (loopV : Expr) : MetaM Expr := do
  let mut wireArgs : Array Expr := #[]
  for k in [j+1:n] do
    if k == i then wireArgs := wireArgs.push loopV
    else if k > i then wireArgs := wireArgs.push ws[k]!
    else wireArgs := wireArgs.push (← closeWire kApps ws i n k loopV)
  pure (mkAppN kApps[j]! wireArgs)

/-- Let-bind the closed wires under their author names and elaborate the
continuation; `post` runs on the elaborated continuation at the innermost
point, while the wire fvars are still live. -/
private partial def bindClosed (names : Array Name) (τs closed ws : Array Expr)
    (i : Nat) (restStx : Syntax) (expectedType? : Option Expr)
    (post : Expr → TermElabM Expr := pure) :
    TermElabM Expr := do
  if h : i < names.size then
    withLetDecl names[i] τs[i]! closed[i]! fun x => do
      let inner ← bindClosed names τs closed ws (i + 1) restStx expectedType?
        post
      -- force pending tactic blocks now: their proof terms may mention the
      -- wire fvars being bound, and a delayed assignment would leak them
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

/-- Single-source chain peeling: enter the pass-1 body's `let` spine and extend
the wire-closing tuple at the tail, so the hoisted chain body exposes the chain
values by projection. Tupled: the lets the continuation references, flattened
to product leaves (the generation pipeline expects one flat wire per output).
Skipped, with their values left inline for the field proofs: subtype-
valued lets (the module-call contract fetch points) and lets only the
pre-`complete` ghosts mention. Returns the extended body and the let spine as
`(name, leafCount)` pairs, `0` marking a skipped let. -/
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

/-- Single-source pass-2 post-pass: swap the chain `let` values for projections
of the hoisted chain-body constant, so the module value computes through the
registered name (the walker's module boundary) while the surface chain appears
once. Matches the pass-1 let spine by name, in order; skipped lets keep their
inline values. The field proofs reference the let fvars, so the swap leaves
them unchanged; the swapped values are definitionally the same. -/
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

/-- Pass-2 dead-let sweep over the instantiated module value. The sweep drops a
skipped chain `let` (one with an inline value) that nothing after it mentions,
because only the wire closing consumed it; otherwise the production graph would
carry a second copy of its callee as a dead re-application. Only the spine's
skipped names are candidates. Runs after instantiation because a postponed
continuation is `?m x…` over every local, so before it no let looks dead. -/
private partial def dropDeadLets (skipped : Array Name) (e : Expr) : Expr :=
  match e with
  | .letE nm ty v b nd =>
    let b' := dropDeadLets skipped b
    if skipped.contains nm && !b'.hasLooseBVar 0 then b'.lowerLooseBVars 1 1
    else .letE nm ty v b' nd
  | .mdata d b => .mdata d (dropDeadLets skipped b)
  | e' =>
    if e'.isAppOf ``letFun && e'.getAppNumArgs ≥ 4 then
      let args := e'.getAppArgs
      match args[3]! with
      | .lam nm ty b bi =>
        let b' := dropDeadLets skipped b
        if skipped.contains nm && !b'.hasLooseBVar 0 then b'.lowerLooseBVars 1 1
        else mkAppN e'.getAppFn (args.set! 3 (Expr.lam nm ty b' bi))
      | _ => e'
    else e'

/-- Walk the hoisted chain constant's `let` spine at the given argument list.
Returns each chain `let`'s `(source name, value)` with earlier lets
zeta-substituted (the program's own runs, as the program spells them) and the
fully substituted result tuple: the wire closings followed by the kept-let
leaves. -/
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

-- An obligation: `let` binders that carry the runs' source names (transparent,
-- so the mono lemmas and faces are defeq through them and application
-- self-instantiates), premises, conclusion. `mkObligation` spells the type at
-- the binders (replacing containing values first), elaborates the proof against
-- that type and returns the proof applied at the premise proofs.
private def mkObligation (binders : Array (Name × Expr))
    (prems : Array (Name × Expr × Expr)) (concl : Expr) (prfStx : Syntax) :
    TermElabM Expr := do
  let rec bindLets (i : Nat) (bs : Array Expr)
      (k : Array Expr → TermElabM Expr) : TermElabM Expr := do
    if i < binders.size then
      let (n, v) := binders[i]!
      withLetDecl n (← inferType v) v fun b =>
        bindLets (i + 1) (bs.push b) k
    else k bs
  bindLets 0 #[] fun bs => do
  -- value→binder replacement by decreasing value size: a containing let value
  -- is strictly larger than its subterms, so outer lets bind before the inner
  -- values they contain are rewritten away; ties (equal values across
  -- valuations, e.g. a wire-independent run at `w` and at its image) resolve
  -- to the earlier binder
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
  -- every listed let stays a binder of the obligation, used or not: the
  -- proofs' `intro` arity equals the number of listed names
  let oblTy ← mkForallFVars (bs ++ ps) (repl concl) (usedLetOnly := false)
  if (← IO.getEnv "HYDRO_DEF_DBG").isSome then
    logInfo m!"fix obligation:{indentExpr oblTy}"
  let prf ← Term.elabTermEnsuringType prfStx oblTy
  Term.synthesizeSyntheticMVarsNoPostponing
  let prf ← instantiateMVars prf
  pure (mkAppN prf (prems.map (·.2.2)))

/-- The chain lets a `fix` `invariant` clause binds in its predicate
(`invariant w (le sp le' sp') => …`); empty when no list is given. -/
private partial def fixInvLetIds (invStx : Syntax) : Array Name :=
  -- collect the identifiers of the optional node in order, whatever the
  -- parser's layout of the group
  go invStx[2] #[]
where
  go (stx : Syntax) (acc : Array Name) : Array Name :=
    match stx with
    | .ident _ _ nm _ => acc.push nm
    | .node _ _ args => args.foldl (fun a s => go s a) acc
    | _ => acc

/-- Whether a `fix` `invariant` predicate speaks of the wire alone: it mentions
none of the listed chain lets and not `w'`. The body may then assume it. The
test is syntactic, so `fixInvAssumable` decides it before the body
elaborates. -/
private def fixInvAssumable (invStx : Syntax) : Bool :=
  let wireNm := invStx[1].getId
  let ids := collectIdents invStx[4]
  !(fixInvLetIds invStx).any ids.contains
    && !ids.contains (wireNm.appendAfter "'")

/-- Elaborate a `fix` `invariant` clause for the knot `kName = <def>.<wire>`:
package the bounded-chain induction over the generated `stages` vocabulary and
hoist `<def>.<wire>.inv` (the invariant at the closed knot) and
`<def>.<wire>.inv_stages` (at every stage up to the fuel; the knot `wf`'s feed
when the body assumes the invariant). Consumers never mention stages: the step
proof reasons about two wire valuations `w ⊑ w⊤` with the chain `let`s bound at
both under their source names (unprimed at `w`, primed at `w⊤`). Runs from the
knot stack generator's hook, after `stages` and before `wf`; idempotent. -/
private def elabFixInvariantClause (declName : Name)
    (wireNames : Array Name) (chainName : Name) (nWires nTotal : Nat)
    (invStx : Syntax) (kName : Name) : TermElabM Unit := do
  let wireNm := invStx[1].getId
  unless wireNames.size > 0 && wireNames[0]! == wireNm do
    throwErrorAt invStx "fix invariant: only the innermost cycle wire \
      (`{wireNames[0]!}`) is supported; got `{wireNm}`"
  unless kName == declName ++ wireNm do return
  if (← getEnv).contains (kName ++ `inv) then return
  HydroGen.withDeclDiag s!"{kName}.inv (invariant clause)" do
  let letIds := fixInvLetIds invStx
  let assumable := fixInvAssumable invStx
  let predStx := invStx[4]
  -- obligation fields: `base := …, step := …`
  let fields : Array (Name × Syntax) :=
    invStx[5].getArgs.map (fun g =>
      (g[1][0].getId.eraseMacroScopes, g[1][2]))
  let (baseStx, stepStx) ←
    match fields.map (·.1) with
    | #[`base, `step] => pure (fields[0]!.2, fields[1]!.2)
    | _ => throwErrorAt invStx "fix invariant: expected `base := …, step := …`"
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
  -- the chain constant's argument list with the invariant wire as a hole: the
  -- knot telescope is ctx ++ [H] ++ postH ++ params, and the chain adds the
  -- innermost wire before the params
  let kArgs := closedK.getAppArgs
  let nParams := nWires - 1
  let chainArgsAt := fun (w : Expr) =>
    kArgs.extract 0 (kArgs.size - nParams) ++ #[w]
      ++ kArgs.extract (kArgs.size - nParams) kArgs.size
  -- the pointwise carrier family, from the knot's `H`-generic result head
  -- (`HydroSem.Stream` or `HydroSem.Atomic`)
  let kTy := (← getConstInfo kName).type
  let cf ← forallTelescope kTy fun _ resTy => do
    match resTy.getAppFn.constName? with
    | some n =>
      match n with
      | ``HydroSem.Stream => pure CarrierFam.stream
      | ``HydroSem.Atomic => pure CarrierFam.atomic
      | _ => throwError "fix invariant: unsupported wire carrier \
          `{n}`"
    | none => throwError "fix invariant: knot result head"
  -- runs (the program's own chain `let`s) and the wire's one-step image at the
  -- module boundary (`closingAt`, the wire's `complete` closing): the
  -- construct spells the obligations at the program's own module
  -- applications, never at the folded body constant
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
      runOf runsClosed (Name.mkSimple (s.dropEnd 1).toString)
    else
      runOf runsW id
    -- obligations. Two devices make the proofs read like the program:
  -- (a) ensures-face auto-premises: for each requested chain `let` whose value
  -- is a module application, the construct prefetches the colocated face
  -- (`M.ensures …`) at every valuation the obligation mentions (`hx` at the
  -- current wire, `hx_step` at its one-step image, `hx'` at the chain top);
  -- (b) source-name binders: the construct states each obligation over
  -- opaque binders that carry the runs' source `let` names (`le`, `sp`,
  -- `le_step`, `le'`, …, plus `<wire>'` for the closed knot), so proofs never
  -- respell a module application.
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
          -- tolerate non-chain ids (the tick route's opaque-param spectators):
          -- no face to prefetch
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
            (Name.mkSimple (id.toString.dropEnd 1).toString)) then
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
          (Name.mkSimple (id.toString.dropEnd 1).toString))
    pure out
  -- the top's own one-step order (`htop`: the chain top is below its image,
  -- by the generated chain mono at `fuel ≤ fuel + 1`)
  let closedStep ← closingAt closedK
  let htopTy ← valuesRelOf cf τw closedK closedStep
  let htopPrf ← do
    let monoLe ← mkAppM ``Nat.le_succ #[fuel]
    let sMono := mkAppN (mkConst (stagesName.appendAfter "_mono")) xs
    mkExpectedTypeHint (← mkAppOptM' sMono #[none, none, some monoLe])
      htopTy
  let htopPrem : Array (Name × Expr × Expr) :=
    #[(`htop, htopTy, htopPrf)]
  let monoName := stagesName.appendAfter "_mono"
  let sMono := mkAppN (mkConst monoName) xs
  let wStep ← closingAt wE
  -- the route-specific middle: the invariant family `I`, the base proof
  -- `I(seed)`, and the `iterate_invariant` step function
  let mkFixRoute : TermElabM ((Expr → TermElabM Expr) × Expr × Expr × Expr) := do
    -- elaborate the predicate over opaque binders: the wire at the current
    -- stage, the wire at the closed knot (`w'`), and one binder per listed
    -- chain `let`, under its source name
    let mut declList : Array (Name × BinderInfo × Expr) :=
      #[(wireNm, .default, τw), (wireNm.appendAfter "'", .default, τw)]
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
      let mut args : Array Expr := #[w, closedK]
      for id in letIds do
        let s := id.toString
        if s.endsWith "'" then
          args := args.push (← runOf runsClosed
            (Name.mkSimple (s.dropEnd 1).toString))
        else
          args := args.push (← runOf runs id)
      pure (P.beta args)
    -- base: faces at the seed and at the top ⊢ I(seed)
    let iSeed ← IAt seed
    let facesSeed ← facesAt seed ""
    -- the construct offers the top-relative premises (`htop`, the `⊑` chain)
    -- only when the predicate may speak of the top (not assumable): an assumed
    -- invariant's induction is unbounded, so its step may use nothing
    -- relative to the fuel
    let topPrems := if assumable then #[] else htopPrem
    let baseProof ← mkObligation
      (#[(wireNm.appendAfter "'", closedK)]
        ++ (← runsBinders seed "") ++ closedBinders)
      (topPrems ++ facesClosed ++ facesSeed) iSeed baseStx
    -- step: ⊑-premises and faces at the wire, at its image (`closingAt`, the
    -- wire's `complete` closing) and at the top ⊢ I preserved
    let stepProofAt ← do
      let relW ← valuesRelOf cf τw wE closedK
      let relB ← valuesRelOf cf τw wStep closedK
      -- the one-step chain order at the stage (`stages_mono` at `m ≤ m + 1`):
      -- ticked carriers grow only by appending ticks, so this premise reduces
      -- a stage step to a per-tick argument (`prefix_ext_invariant`)
      let relWB ← valuesRelOf cf τw wE wStep
      let iW ← IAt wE
      let iB ← IAt wStep
      let facesW ← facesAt wE ""
      let facesB ← facesAt wStep "_step"
      let binders :=
        #[(wireNm.appendAfter "'", closedK)]
          ++ (← runsBinders wE "") ++ (← runsBinders wStep "_step")
          ++ closedBinders
      let core ← if assumable then mkArrow iW iB else
        mkArrow relW (← mkArrow relB (← mkArrow relWB
          (← mkArrow iW (← pure iB))))
      let prf ← mkObligation binders
        (topPrems ++ facesClosed ++ facesW ++ facesB) core stepStx
      -- abstract the wire itself, bound outside the run binders so the runs'
      -- types may mention it
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
      let prf := if assumable then mkApp ((stepProofAt.beta #[iterM]).headBeta) ihE
        else mkApp4 ((stepProofAt.beta #[iterM]).headBeta) rel1 rel2 rel0 ihE
      -- `IAt` states the invariant family at the iterate; the step lands at
      -- the wire's `complete` closing (`closingAt`), definitionally the next
      -- iterate
      let prf ← mkExpectedTypeHint prf
        (← IAt ((F.beta #[iterM]).headBeta))
      mkLambdaFVars #[mE, hmE, ihE] prf
    pure (IAt, baseProof, stepFn, stepProofAt)
  let (IAt, baseProof, stepFn, stepProofAt) ← mkFixRoute
  -- compose: `iterate_invariant` + `stages_mono` (the ⊑ premises) +
  -- `stages_fix` (the closed knot equals the fueled stage by definition)
  let IFn ← withLocalDecl `w .default τw fun v => do
    mkLambdaFVars #[v] (← IAt v)
  let invProof ← mkAppOptM ``iterate_invariant
    #[some τw, some F, some seed, some IFn, some fuel,
      some baseProof, some stepFn]
  let iClosed ← IAt closedK
  let invProof ← mkExpectedTypeHint invProof iClosed
  let stmt ← mkForallFVars xs iClosed
  let val ← mkLambdaFVars xs invProof
  -- the stage form: at every stage when the body assumes the invariant, else
  -- up to the fuel (the same induction cut at `m`, the step's chain premises
  -- being relative to the fuel)
  let stagesApp := fun (m : Expr) => mkApp (mkAppN (mkConst stagesName) xs) m
  let (stStmt, stVal) ← withLocalDecl `m .default (mkConst ``Nat) fun mE => do
    let concl ← IAt (stagesApp mE)
    if assumable then
      -- unbounded (the step used nothing relative to the fuel): the knot `wf`'s
      -- feed at any stage
      let stepAll ← withLocalDecl `v .default τw fun vE => do
        withLocalDecl `hv .default (← IAt vE) fun hvE => do
        let prf := mkApp ((stepProofAt.beta #[vE]).headBeta) hvE
        let prf ← mkExpectedTypeHint prf (← IAt ((F.beta #[vE]).headBeta))
        mkLambdaFVars #[vE, hvE] prf
      let prf ← mkAppOptM ``iterate_invariant_all
        #[some τw, some F, some seed, some IFn, some baseProof, some stepAll, some mE]
      let prf ← mkExpectedTypeHint prf concl
      pure ((← mkForallFVars (xs ++ #[mE]) concl),
        (← mkLambdaFVars (xs ++ #[mE]) prf))
    else
      withLocalDecl `hm .default (← mkAppM ``LE.le #[mE, fuel]) fun hmE => do
      let stepCut ← withLocalDecl `m' .default (mkConst ``Nat) fun m'E => do
        withLocalDecl `hm' .default (← mkAppM ``LT.lt #[m'E, mE]) fun hm'E => do
        let lt ← mkAppM ``Nat.lt_of_lt_of_le #[hm'E, hmE]
        mkLambdaFVars #[m'E, hm'E] (mkApp (mkApp stepFn m'E) lt)
      let prf ← mkAppOptM ``iterate_invariant
        #[some τw, some F, some seed, some IFn, some mE, some baseProof, some stepCut]
      let prf ← mkExpectedTypeHint prf concl
      pure ((← mkForallFVars (xs ++ #[mE, hmE]) concl),
        (← mkLambdaFVars (xs ++ #[mE, hmE]) prf))
  for (nm, stmt, val) in #[(kName ++ `inv, stmt, val),
      (kName ++ `inv_stages, stStmt, stVal)] do
    let val ← instantiateMVars val
    let stmt ← instantiateMVars stmt
    if val.hasExprMVar || stmt.hasExprMVar then
      throwError "fix invariant: residual metavariables in {nm}"
    -- an unknown hypothesis name inside an obligation tactic error-recovers
    -- into a term with dead free variables, which only the async kernel would
    -- reject, far from the cause: fail here instead
    let fvs := (Lean.CollectFVars.main val {}).fvarIds
    let fvs2 := (Lean.CollectFVars.main stmt {}).fvarIds
    unless fvs.isEmpty && fvs2.isEmpty do
      let lctx ← getLCtx
      throwError "fix invariant: residual free variables in the \
        packaged induction {nm} (an obligation proof likely references an \
        unknown name): val \
        {fvs.map (fun f => (lctx.find? f).map (·.userName))}, stmt \
        {fvs2.map (fun f => (lctx.find? f).map (·.userName))}"
    HydroGen.addThm nm stmt val
    pendingInvs.modify (·.push (nm, stmt, val))
  if assumable then
    -- the body assumes the invariant: the knot `wf` feeds the stage form to
    -- the body module's `hreq`, and the closed form closes the chain module's
    -- `hreq` conjunct in the enclosing module's wf (a `hydro_pre_close`
    -- closer)
    HydroGen.knotLoopInv.modify (·.insert kName (kName ++ `inv_stages))
    HydroGen.currentPres.modify fun ps =>
      if ps.contains (kName ++ `inv) then ps else ps.push (kName ++ `inv)
  -- supply the closed-knot instance to the enclosing field proofs: ghost-queue
  -- `have h<wire>_inv := <def>.<wire>.inv args…`, applied at the def's own
  -- binders by name (they are all in scope there; the invariant's premises
  -- stay general). The args come from the outer telescope `xs` only, because
  -- re-telescoping `stmt` would walk into the invariant's own binders.
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
    -- nested in an enclosing single-source fix, the inner block elaborates
    -- once per outer pass: queue the ghost only on the pass whose continuation
    -- elaborates (not the outer pass 1)
    unless (← fixPassMode.get).contains true do
      HydroGhost.pendingGhosts.modify (·.push tac)
  logInfo m!"fix invariant: {kName ++ `inv}, {kName ++ `inv_stages}    {if assumable then " (assumed by the body)" else ""}"

/-- Abstract `e` over `xs`, turning let-bound fvars into opaque lambda binders.
A hoisted module takes the enclosing chain's runs as plain inputs, and their
defining values stay at the call site: abstracting them as `let`s would pull
the values in, leaking any fvars they reference, and would break the
application arity. -/
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
  -- single-source form: no trailing rest; the body carries a `complete` marker
  -- and the continuation follows it inline
  let restStx? : Option Syntax :=
    if stx[7].getArgs.isEmpty then none else some stx[7][1]
  let singleSource := restStx?.isNone
  let n := binders.size
  let names := binders.map (·[1].getId)
  let tyStxs : Array Syntax := binders.map (·[3])
  let some declName ← getDeclName?
    | throwError "fix: no enclosing declaration"
  -- the `hydro def` command elaborates the contract-faced pair under `M._spec`
  -- and derives the module `M` from it: the hoisted knots belong to `M`
  let declName := HydroGhost.moduleOfSpec declName
  -- the hoisted chain body is `<M>.body`; a fix nested in another's body is
  -- `<M>.<wire>.body`, so the two chains do not collide
  let nested := !(← fixPassMode.get).isEmpty
  let chainName := if nested then declName ++ names[0]! ++ `body else declName ++ `body
  -- an assumable `invariant` (over the wire alone) is in force for the body
  -- (its `ghost pre` proofs see it as `h<w>_inv`); the elaborator records it as
  -- the hoisted chain module's `requires` conjunct before anything of the body
  -- elaborates
  let assumable := match invStx? with
    | some inv => fixInvAssumable inv
    | none => false
  if let some invStx := invStx? then
    unless singleSource do
      throwErrorAt invStx "fix invariant: only single-source (`complete`) \
        fix blocks are supported"
    unless names.size > 0 && names[0]! == invStx[1].getId do
      throwErrorAt invStx "fix invariant: only the innermost cycle wire \
        (`{names[0]!}`) is supported; got `{invStx[1].getId}`"
    if assumable then
      HydroGen.loopInvariants.modify (·.insert chainName invStx[4])
      HydroGen.fixLoopStack.modify ((names[0]!, invStx[4]) :: ·)
  try
    let τs ← tyStxs.mapM fun t => elabType t
    let fuelStxs ← splitFuels fuelsStx n
    let decls : Array (Name × (Array Expr → TermElabM Expr)) :=
      (names.zip τs).map fun (nm, τ) => (nm, fun _ => pure τ)
    withLocalDeclsD decls fun ws => do
      let prodTy ← mkProdN τs
      -- pass 1 (single-source): the `complete` marker returns the wire-closing
      -- tuple, and pass 1 does not elaborate the inline continuation. The
      -- body's ghost clauses do queue during pass 1 (a `ghost pre` there needs
      -- the earlier ghost facts for its theorem), and the elaborator restores
      -- the queue afterwards, so none leaks out of the pass
      let savedGhosts ← HydroGhost.pendingGhosts.get
      if singleSource then fixPassMode.modify (true :: ·)
      let body ←
        try elabTermEnsuringType bodyStx prodTy
        finally
          if singleSource then
            fixPassMode.modify (·.drop 1)
            HydroGhost.pendingGhosts.set savedGhosts
      synthesizeSyntheticMVarsNoPostponing
      let body ← instantiateMVars body
      let fuels ← fuelStxs.mapM fun f => do
        let e ← elabTerm f none
        synthesizeSyntheticMVarsNoPostponing
        instantiateMVars e
      -- the ambient interpretation fvar: the unique `HydroSem`-typed free
      -- variable of the wire types
      let lctx ← getLCtx
      let tyFvars := (τs.foldl (fun (s : CollectFVars.State) τ =>
        Lean.CollectFVars.main τ s) {}).fvarIds
      let some hFvar := tyFvars.find? (fun f =>
          (lctx.get! f).type.getAppFn.constName? == some ``HydroSem)
        | throwErrorAt stx "fix: wire types mention no `HydroSem` \
            interpretation variable"
      let hExpr := mkFVar hFvar
      -- inline let-bound aliases whose values are `H`-free; `H`-dependent
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
      -- free-variable analysis: caps are the `H`-dependent fvars free in the
      -- body (they become tuple components); fuels and types may mention
      -- further binders (e.g. the fuel decision) that are knot-def binders but
      -- not captured. The seed is the raw body, `let` spine included, so a
      -- binder that only a continuation-kept chain `let` mentions is a knot
      -- binder too.
      let bodySeed := Lean.CollectFVars.main rawBody {}
      let seed₀ := (fuels.toList ++ τs.toList).foldl
        (fun (s : CollectFVars.State) e => Lean.CollectFVars.main e s) bodySeed
      let wIds := ws.map (·.fvarId!)
      let all ← fvarClosure ((seed₀.fvarIds.filter
        (fun f => !wIds.contains f)).push hFvar)
      let bodyIds := (Lean.CollectFVars.main rawBody {}).fvarIds
      let hPos := (lctx.get! hFvar).index
      let typeOf : Std.HashMap FVarId Expr ←
        all.foldlM (fun m f => do
          pure (m.insert f (← instantiateMVars (lctx.get! f).type)))
          ({} : Std.HashMap FVarId Expr)
      let hDep (f : FVarId) : Bool := (typeOf[f]?.getD (mkConst ``Unit)).containsFVar hFvar
      -- the knot's binder layout: the enclosing def's pre-`H` data stay pre-`H`
      -- params; every other binder (`H`-free data, decisions, wires, fuels)
      -- sits after `H` in context order; the `H`-dependent fvars occurring in
      -- the body are additionally the caps-tuple components
      let dataPre := all.filter fun f =>
        f != hFvar && (lctx.get! f).index < hPos && !hDep f
      let postH := all.filter fun f =>
        f != hFvar && ((lctx.get! f).index > hPos || hDep f)
      let dataPreE := dataPre.map mkFVar
      let postHE := postH.map mkFVar
      -- caps = `H`-dependent post-`H` fvars the (possibly single-source-
      -- replaced) body references; resolved through a cell so the hoist
      -- closure sees the final classification (a dec appearing only in `via`
      -- fuels still reaches the chain application's arguments)
      let capsCell ← IO.mkRef (postH.filter fun f =>
        bodyIds.contains f && hDep f)
      let hTy ← inferType hExpr
      -- hoist one component to a top-level knot def; returns the constant
      -- applied to dataPre + H + postH (wire params pending)
      let hoist := fun (knotName : Name) (isTick : Bool) (w τw : Expr)
          (paramWs : Array Expr) (b f : Expr) => do
        let capsE := (← capsCell.get).map mkFVar
        let caps := capsE ++ paramWs
        let capsTys ← caps.mapM fun c => do instantiateMVars (← inferType c)
        -- Γ as an explicit lambda, abstracting `H` from the caps types
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
        -- the generic body is its own top-level `@[reducible]` def: knot
        -- bodies are top-level constants because the inline form is
        -- kernel-pathological at composed-body scale. Binders: the data params
        -- plus the `H`-free post-`H` fvars the body references raw.
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
        -- `H.fix` / `H.fixTick` with Γ passed explicitly, because higher-order
        -- unification would pick the non-abstracting Γ
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
        -- the Bekić parameters: the knot's trailing wire binders that are knot
        -- wires of this block; its `_strict₁` lemma takes them at lag `1`
        HydroGen.registerInfoT knotName fun gi => { gi with knotParams := paramWs.size }
        pure (mkAppN (mkConst knotName) (dataPreE ++ #[hExpr] ++ postHE))
      -- single-source form: hoist the whole chain as its own top-level module
      -- def (`<M>.body`) and queue it for the enclosing `hydro def`'s
      -- registration pass first (innermost-first); the per-wire slicing below
      -- then projects the registered constant, so the generation walk has its
      -- module boundary
      let mut body := body
      let mut chainSpine : Array (Name × Nat) := #[]
      let mut nTotal := n
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
      -- Bekić: emit K₁ … Kₙ; component i's body substitutes earlier wires by
      -- their knots re-closed at (loop var, later params)
      let mut kApps : Array Expr := #[]
      for i in [0:n] do
        let τw := τs[i]!
        let isTick :=
          τw.getAppFn.constName? == some ``HydroSem.Atomic
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
      -- eager knot-stack generation: the hoisted chain's and knots' generated
      -- vocabulary (the couple/glue `sr`/`rr` equations, `K_co_*`, `K_mono₁`,
      -- `K.stages*`) is available to the ghost clauses and field proofs of the
      -- enclosing
      -- `hydro def`, so the knot induction can cite the any-body Kleene-chain
      -- lemmas from inside the program. The post-def pipeline pass skips
      -- already-generated stacks (every `run*T` is idempotent); only the param
      -- phase stays post-def.
      let hintsE ← pendingHints.get
      -- the `invariant` clause elaborates between the knot's `stages` and `wf`
      -- (the hook; see `elabFixInvariantClause`): the `wf` consumes its stage
      -- form when the body assumes it
      let savedHook ← HydroGen.loopInvHook.get
      if let some invStx := invStx? then
        HydroGen.loopInvHook.set (some fun k =>
          elabFixInvariantClause declName names chainName n nTotal invStx k)
      try
        for k in (← pendingKnots.get) do
          HydroGen.registerSpec k
          -- idempotence across a nested fix's re-elaboration: the pending list
          -- spans the whole command, and an inner fix's own eager pass already
          -- generated its knots
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
            if dbg then IO.eprintln s!"[hydro def] smono {k}"
            HydroGen.runModSMonoT k hintsE
            if dbg then IO.eprintln s!"[hydro def] wf {k}"
            HydroGen.runModWfT k hintsE
            if dbg then IO.eprintln s!"[hydro def] mono {k}"
            HydroGen.runModMonoT k
            -- the chain's graph and its graded causality: the knot stack's
            -- strictness facts (`K_strict₁`, the `wf`'s `hstrict`) are the
            -- chain graph's lag analysis at the knot wires
            -- (`<chain>.dag_agreeL`), so the elaborator projects the chain
            -- before its knots
            if dbg then IO.eprintln s!"[hydro def] project {k}"
            HydroGen.Project.genProject k
          if dbg then IO.eprintln s!"[hydro def] done {k}"
      finally HydroGen.loopInvHook.set savedHook
      -- the `invariant` clause: a packaged bounded-chain induction over the
      -- just-generated `stages` vocabulary, hoisting `<def>.<wire>.inv` for the
      -- field proofs; no stage index escapes the construct. Idempotent: the hook
      -- already ran it when this pass generated the knot stack.
      if let some invStx := invStx? then
        elabFixInvariantClause declName names chainName n nTotal invStx
          (declName ++ names[0]!)
      -- closed wires (the outermost is the last component)
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
        -- pass 2 (single-source): re-elaborate the body with the wires bound to
        -- the closed knots; `complete` skips to its inline continuation and the
        -- chain's `let`s stay in scope, their values swapped for projections of
        -- the hoisted chain body (the module value computes through the
        -- registered constant)
        let chainApp := mkAppN (mkConst chainName)
          (dataPreE ++ #[hExpr] ++ postHE ++ ws)
        fixPassMode.modify (false :: ·)
        try
          let v ← bindClosed names τs closed ws 0 bodyStx expectedType?
            (post := swapChainLets chainSpine chainApp n nTotal 0 0)
          synthesizeSyntheticMVarsNoPostponing
          let v ← instantiateMVars v
          pure (dropDeadLets (chainSpine.filterMap fun (nm, k) =>
            if k == 0 then some nm else none) v)
        finally fixPassMode.modify (·.drop 1)
  finally
    if assumable then HydroGen.fixLoopStack.modify (·.drop 1)

@[term_elab completeTerm] def elabComplete : TermElab :=
    fun stx expectedType? => do
  match ← fixPassMode.get with
  | [] =>
    throwErrorAt stx "complete: only allowed inside a single-source \
      `fix` body"
  | true :: _ =>
    -- pass 1: the wire-closing tuple is the knot body value
    elabTerm stx[1] expectedType?
  | false :: rest =>
    -- pass 2: the wires are closed; continue below the marker.
    -- `elabComplete` consumes the marker: the continuation belongs to the
    -- enclosing fix (if any), so its own marker must see its own pass
    fixPassMode.set rest
    try elabTerm stx[3] expectedType?
    finally fixPassMode.modify (false :: ·)

end HydroFix

/-! ## The ghost layer

Contract faces and proof-structuring sugar, active wherever `Hydro.HydroDef`
is imported:

    hydro def M (H : HydroSem L mem) (args…) :
        τ ensures out => P out :=
      let a := …
      ghost let g := ‹spec-only value›
      ghost have h : S := proof
      (value…)
      prove
        field₁ := e₁,
        field₂ := e₂

- `τ ensures out => P` (type position) elaborates to the contract face
  `{out : τ // ∀ hv : H = Values L mem, match H, hv, x₁…xₖ, out with
  | _, rfl, x₁…xₖ, out => P}`, transporting every binder whose type mentions
  the interpretation variable. The `hydro def` command elaborates this pair
  under `M._spec` and derives from it the module `M : ∀ xs, τ` (the plain
  program, a non-dependent function) and the face theorem
  `M.ensures : ∀ xs[H := Values L mem], P xs (M (Values L mem) xs)`
  (`HydroGhost.deriveModule`). Call sites consume `M H …` and `M.ensures …`
  directly, and a callee application is an ordinary function application whose
  arguments simp can rewrite.
- `ghost have`/`ghost let` interleave in the `let` chain: spec-only bindings
  stated at the program point where they hold. The elaborator **strips them
  from the computational value** and replays them in order inside the contract
  proof after `intro hv; subst hv`, so their statements see the `Values`
  substitution with every computational `let` in scope. `subst` eliminates the
  interpretation variable, so ghost statements spell the instance explicitly
  (`Values L mem`).
- `value prove f₁ := e₁, …` closes the def: the computational value paired with
  the contract proof assembled field by field (structure contracts), with every
  ghost fact in scope.

Binary and relational statements (Flo monotonicity, causality) are not faces:
monotonicity follows from instantiating the program at `MonoRel` (the
⊑-diagonal, `MonoRel.lean`/`ValuesMono.lean`), and causality follows from the
typed graph's `Dag.denote_causal`/`dag_agreeL` (`Graph/Causal.lean`). -/

namespace HydroGhost

/-- `τ ensures out => P`: the contract-face former, in the type position of a
`hydro def`. Elaborates to `{out : τ // ∀ hv : H = Values L mem, match … => P}`
with every `H`-dependent binder transported through the match. -/
syntax:10 (name := ensuresTerm) term:11 " ensures " ident " => " term : term

/-- `τ requires R ensures out => P`: the face with the module's **coupling
precondition** `R`, a fact about the inputs' content at the denotation that
the module uses (at its conditional-fold sites through `ghost pre`, and for its
callees' own clauses) and every caller proves at the call site, as a Verus
`requires`. It does not weaken the `ensures` face, which consumers use
unconditionally; it is a hypothesis of the module's machine/denotation
coupling lemmas (`M_co_wfᵢ … (hreq : R)`). -/
syntax:10 (name := requiresEnsuresTerm)
  term:11 " requires " term:11 " ensures " ident " => " term : term

/-- `τ requires R`: the coupling precondition alone, for a module that uses a
fact about its inputs (at its `ghost pre` sites or for its callees' clauses)
but states no `ensures` face of its own. -/
syntax:10 (name := requiresTerm) term:11 " requires " term:11 : term

/-- `ghost have h : S := prf`: a spec-only fact at this program point, visible
only in the contract proof. The type ascription is optional for sub-contract
fetches (`….property rfl`). `elabGhostHave` hoists the fact to the theorem
`<M>.ghost.<h>` and replays it by application. -/
syntax (name := ghostHave)
  withPosition("ghost " "have " ident (" : " term)? " := " term)
  optSemicolon(term) : term

/-- `ghost obtain ⟨…⟩ := e`: destructure a spec-only fact at this program
point, only in the contract proof. -/
syntax (name := ghostObtain)
  withPosition("ghost " "obtain " rcasesPat " := " term)
  optSemicolon(term) : term

/-- `ghost let x := v`: a spec-only value at this program point, only in the
contract proof. -/
syntax (name := ghostLet)
  withPosition("ghost " "let " ident (" : " term)? " := " term)
  optSemicolon(term) : term

/-- `ghost witness e`: choose the witness of an existential `ensures` face at
this program point, only in the contract proof. The clause refines the
contract `∃ w, P w` to `P e`, with every earlier ghost fact in scope. -/
syntax (name := ghostWitness)
  withPosition("ghost " "witness " term)
  optSemicolon(term) : term

/-- `ghost intro h₁ h₂ …`: name the hypotheses of an implication-shaped
`ensures` face. A conditional face `ensures out => R → P` introduces `R` here,
so later ghost clauses can use it. -/
syntax (name := ghostIntro)
  withPosition("ghost " "intro " ident+)
  optSemicolon(term) : term

/-- `ghost pre h : S := prf`: **the site fact of a conditional fold or of a
callee's `requires`**, which you prove at `Values` before the site. You state
`S` over the program's wires by name; `prf` may use every earlier ghost fact and
the module's own `requires` clause as `hreq`. The elaborator packages the fact
as the theorem `M.pre.<h> : ∀ binders, let wires…; PreFact R S`, which the
generated `_co_wf` lemmas consume through `hydro_pre_close`, and queues it as
`h : PreFact R S` for the field proofs. A `hydro def` whose conditional fold has
no matching `ghost pre` does not elaborate: the program does not compile
without the proof. -/
syntax (name := ghostPre)
  withPosition("ghost " &"pre " ident " : " term " := " term)
  optSemicolon(term) : term

/-- `ghost subst h`: substitute an equation hypothesis (typically one from
`ghost intro`) into the contract goal and every ghost fact. -/
syntax (name := ghostSubst)
  withPosition("ghost " "subst " ident)
  optSemicolon(term) : term

/-- `value prove f₁ := e₁, f₂ := e₂, …`: close an `ensures`-faced def with the
computational value and the field-by-field contract assembly. Elaborates to
`⟨value, by intro hv; subst hv; ‹replayed ghosts›; exact {f₁ := e₁, …}⟩`. -/
syntax proveField := ident " := " term
syntax:10 (name := proveTerm)
  term:11 " prove " proveField,+ : term

open Lean Parser Elab Term Meta

/-- The ensures face former, shared by the plain and the `requires` forms. -/
def elabFace (stx : Syntax) (τStx pStx : TSyntax `term) (outId : Ident) :
    TermElabM Expr := do
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

@[term_elab ensuresTerm] def elabEnsuresTerm : TermElab :=
    fun stx _expectedType? => do
  let τStx : TSyntax `term := ⟨stx[0]⟩
  let outId : Ident := ⟨stx[2]⟩
  let pStx : TSyntax `term := ⟨stx[4]⟩
  elabFace stx τStx pStx outId

/-- Run `k` in the **ghost world**: the local context with the unique
`HydroSem` variable removed and `H := Values L mem` substituted in every
binder's type and every let-wire's value. Statements elaborated here are the
ones the field proofs see after `subst hv`. `k` receives the interpretation
variable's declaration and the `Values` instance. -/
def withValuesLCtx (stx : Syntax) (k : LocalDecl → Expr → TermElabM α) : TermElabM α := do
  let lctx ← getLCtx
  let mut hFvar? : Option LocalDecl := none
  for d in lctx do
    if !d.isImplementationDetail then
      if (← instantiateMVars d.type).getAppFn.constName? == some ``HydroSem then
        if hFvar?.isSome then throwErrorAt stx "hydro def: multiple `HydroSem` variables"
        hFvar? := some d
  let some hDecl := hFvar?
    | throwErrorAt stx "hydro def: no `HydroSem` interpretation variable in scope"
  let hTy ← instantiateMVars hDecl.type
  let hE := hDecl.toExpr
  let V := mkApp2 (mkConst ``Values) (hTy.getArg! 0) (hTy.getArg! 1)
  let mut lctxV := lctx.erase hDecl.fvarId
  for d in lctx do
    if !d.isImplementationDetail && d.fvarId != hDecl.fvarId then
      let ty := (← instantiateMVars d.type).replaceFVar hE V
      let v? ← match d.value? with
        | some v => pure (some ((← instantiateMVars v).replaceFVar hE V))
        | none => pure none
      lctxV := lctxV.modifyLocalDecl d.fvarId fun d' =>
        let d' := d'.setType ty
        match v? with
        | some v => d'.setValue v
        | none => d'
  withLCtx lctxV (← getLocalInstances) (k hDecl V)

@[term_elab requiresEnsuresTerm] def elabRequiresEnsuresTerm : TermElab :=
    fun stx _expectedType? => do
  let τStx : TSyntax `term := ⟨stx[0]⟩
  let rStx : Syntax := stx[2]
  let outId : Ident := ⟨stx[4]⟩
  let pStx : TSyntax `term := ⟨stx[6]⟩
  -- the clause must elaborate here, over the header binders in the ghost
  -- world, so the elaborator rejects an unknown name at once
  withValuesLCtx stx fun _ _ => do
    let _ ← Term.withSynthesize <| elabType (⟨rStx⟩ : TSyntax `term)
  HydroGen.currentRequires.set (some rStx)
  elabFace stx τStx pStx outId

@[term_elab requiresTerm] def elabRequiresTerm : TermElab :=
    fun stx expectedType? => do
  let τStx : TSyntax `term := ⟨stx[0]⟩
  let rStx : Syntax := stx[2]
  withValuesLCtx stx fun _ _ => do
    let _ ← Term.withSynthesize <| elabType (⟨rStx⟩ : TSyntax `term)
  HydroGen.currentRequires.set (some rStx)
  elabType τStx

/-- The `ghost pre` elaborator: package the site fact as a closed theorem
`M.pre.<h>` over the context it needs (the module's binders, the let-chain as
`let`s) with `H` instantiated to `Values`, proven by the earlier ghost
tactics and the user's proof; then queue `h` for the field proofs. -/
@[term_elab ghostPre] def elabGhostPre : TermElab :=
    fun stx expectedType? => do
  let name : Ident := ⟨stx[2]⟩
  let tyStx : TSyntax `term := ⟨stx[4]⟩
  let prfStx : TSyntax `term := ⟨stx[6]⟩
  do
    let some declName ← getDeclName?
      | throwErrorAt stx "ghost pre: no enclosing declaration"
    let mName := moduleOfSpec declName
    -- the let-chain so far may carry postponed instance problems (the
    -- elaborator has not finished the enclosing term): settle them first
    Term.synthesizeSyntheticMVarsNoPostponing
    let lctx ← getLCtx
    let (stmt, ghosts, hDecl, xs) ← withValuesLCtx stx fun hDecl _V => do
      let sE ← Term.withSynthesize <| elabType tyStx
      let sE ← instantiateMVars sE
      -- the clause in force here: the def's `requires` and the enclosing `fix`
      -- blocks' loop-wire clauses (the chain module's `hreq`)
      let rE ← match ← HydroGen.requiresStxHere with
        | some r => do
          let e ← Term.withSynthesize <| elabType (⟨r⟩ : TSyntax `term)
          instantiateMVars e
        | none => pure (mkConst ``True)
      let body := mkApp2 (mkConst ``PreFact) rE sE
      -- close over only the context the statement needs, through binder
      -- types and let-wire values: every binder of the theorem then occurs in
      -- `R → S`, so the closer determines them all by unification with no
      -- dangling metavariable
      let lctxV ← getLCtx
      let mut inSet : Std.HashSet FVarId := {}
      let mut frontier := (Lean.CollectFVars.main body {}).fvarIds
      while !frontier.isEmpty do
        let mut next : Array FVarId := #[]
        for f in frontier do
          if inSet.contains f then continue
          inSet := inSet.insert f
          let d := lctxV.get! f
          let mut used := (Lean.CollectFVars.main (← instantiateMVars d.type) {}).fvarIds
          if let some v := d.value? then
            used := used ++ (Lean.CollectFVars.main (← instantiateMVars v) {}).fvarIds
          next := next ++ used.filter (fun x => !inSet.contains x)
        frontier := next
      let mut xs : Array Expr := #[]
      for d in lctxV do
        if !d.isImplementationDetail && inSet.contains d.fvarId then
          xs := xs.push d.toExpr
      let stmt ← instantiateMVars (← mkForallFVars xs body)
      if stmt.hasExprMVar then
        throwErrorAt stx "ghost pre: the statement has metavariables:{indentExpr stmt}"
      pure (stmt, ← pendingGhosts.get, hDecl, xs)
    let mut introTms : Array (TSyntax `term) := #[]
    for x in xs do
      let d := lctx.get! x.fvarId!
      let nm := d.userName
      introTms := introTms.push
        (if nm.hasMacroScopes || !nm.isAtomic then ⟨(mkIdent `_).raw⟩ else ⟨(mkIdent nm).raw⟩)
    -- the proof: the earlier ghost tactics, then the user's proof, in the
    -- re-introduced context (`H` is `Values`, so no `subst`)
    let introTac ← `(tactic| intro $[$introTms:term]*)
    -- the clause hypothesis under its names: the module's own `requires` as
    -- `hreq`, each enclosing fix's assumed `invariant` as `h<w>_inv` (the
    -- clause tactics destructure the left-nested conjunction `(R ∧ I₁) ∧ I₂ …`)
    let partNames : Array Ident ← do
      let mut out : Array Ident := #[]
      if (← HydroGen.currentRequires.get).isSome then out := out.push (mkIdent `hreq)
      for (w, _) in (← HydroGen.fixLoopStack.get).reverse do
        out := out.push (mkIdent (Name.mkSimple ("h" ++ w.toString ++ "_inv")))
      pure out
    let clauseTacs : Array (TSyntax `tactic) ← do
      if partNames.size ≤ 1 then
        let hId := partNames.getD 0 (mkIdent `hreq)
        pure #[← `(tactic| intro $hId:ident)]
      else
        let hall := mkIdent `hreq_all
        let mut tacs : Array (TSyntax `tactic) := #[← `(tactic| intro $hall:ident)]
        let mut cur := hall
        for k in [0:partNames.size - 1] do
          let idx := partNames.size - 1 - k
          let right := partNames[idx]!
          if idx == 1 then
            let left := partNames[0]!
            tacs := tacs.push (← `(tactic| obtain ⟨$left:ident, $right:ident⟩ := $cur:ident))
          else
            let left := mkIdent (Name.mkSimple s!"hreq_all{idx}")
            tacs := tacs.push (← `(tactic| obtain ⟨$left:ident, $right:ident⟩ := $cur:ident))
            cur := left
        pure tacs
    let exactTac ← `(tactic| exact $prfStx)
    let allTacs : Array (TSyntax `tactic) := #[introTac] ++ clauseTacs ++ ghosts ++ #[exactTac]
    let tac ← `(tactic| ($[$allTacs:tactic]*))
    let thmName := mName ++ (Name.mkSimple "pre" ++ name.getId)
    let val ← HydroGen.withDeclDiag (toString thmName) <| withLCtx {} {} <| Term.withoutErrToSorry do
      let mvar ← mkFreshExprSyntheticOpaqueMVar stmt
      let remaining ← Tactic.run mvar.mvarId! <| Tactic.withoutRecover do
        Tactic.evalTactic tac; Tactic.pruneSolvedGoals
      let remaining ← remaining.filterM fun g => do
        pure (!(← g.isAssigned) && !(← g.isDelayedAssigned))
      unless remaining.isEmpty do
        throwErrorAt stx "ghost pre {name.getId}: unsolved goals\n{goalsToMessageData remaining}"
      Term.synthesizeSyntheticMVarsNoPostponing
      instantiateMVars mvar
    let val ← instantiateMVars val
    if val.hasExprMVar then
      throwErrorAt stx "ghost pre {name.getId}: the proof has metavariables"
    unless (← getEnv).contains thmName do
      HydroGen.addThm thmName stmt val
    HydroFix.pendingInvs.modify (·.push (thmName, stmt, val))
    HydroGen.currentPres.modify fun ps => if ps.contains thmName then ps else ps.push thmName
    -- the field proofs see `h : PreFact R S`: the theorem at the def's own
    -- binders by name (the statement's `let`s unfold on application). The
    -- elaborator reads the argument names off the theorem, because a `fix`
    -- body elaborates twice and the wires' binder status may differ per pass
    let thmTy := (← getConstInfo thmName).type
    let mut argIds : Array Ident := #[]
    let mut cur := thmTy
    while cur.isForall do
      if cur.bindingInfo!.isExplicit then
        argIds := argIds.push (mkIdent cur.bindingName!)
      cur := cur.bindingBody!
    let _ := hDecl
    let _ := xs
    let thmId := mkCIdent thmName
    let haveTac ← `(tactic| have $name := $thmId $argIds*)
    pendingGhosts.modify (·.push haveTac)
    logInfo m!"ghost pre: {thmName}"
  elabTerm stx[8] expectedType?

/-- **The hoisted ghost fact, replayed.** `hydro_ghost_have h := thm n` adds `h`
as the theorem `thm` applied to the current context by binder name: its first
`n` binders bind the closed-over context, module binders and hypotheses taken
from the local of the same name and instance binders by synthesis. The theorem
states the fact over the context's let-wires, not their values: the tactic
instantiates each `let` binder of the theorem by the current let-local of that
name, so `h`'s type
mentions `a_log i` and never the unfolded `tick_scan …`, and `rw [h i]` finds
the wire. The value is the plain application; the two agree by zeta. -/
elab "hydro_ghost_have " h:Lean.Parser.ident " := " thm:Lean.Parser.ident n:Lean.Parser.numLit : tactic =>
  open Elab.Tactic in withMainContext do
  let nArity : Nat := (⟨n.raw⟩ : NumLit).getNat
  let thmName ← realizeGlobalConstNoOverload thm
  let info ← getConstInfo thmName
  let lctx ← getLCtx
  let mut e := Lean.mkConst thmName (info.levelParams.map mkLevelParam)
  let mut ty := info.type
  for _ in [0:nArity] do
    match ty with
    | .forallE nm d b bi =>
      let arg ← if bi.isInstImplicit then
          Meta.synthInstance (← instantiateMVars d)
        else
          match lctx.findFromUserName? nm with
          | some decl => pure decl.toExpr
          | none => throwError "hydro_ghost_have: `{nm}` (a binder of {thmName}) is not in scope"
      e := mkApp e arg
      ty := b.instantiate1 arg
    | .letE nm _ _ b _ =>
      match lctx.findFromUserName? nm with
      | some decl => ty := b.instantiate1 decl.toExpr
      | none => throwError "hydro_ghost_have: the wire `{nm}` (a let of {thmName}) is not in scope"
    | _ => throwError "hydro_ghost_have: {thmName} has fewer than {nArity} context binders"
  let stated ← instantiateMVars ty
  let goal ← getMainGoal
  let goal' ← goal.assert h.getId stated e
  let (_, goal'') ← goal'.intro1P
  replaceMainGoal [goal'']

/-- **`ghost have`, hoisted.** `elabGhostHave` proves the fact once, as the
closed theorem `<M>.ghost.<h>` over the context it needs (module binders,
let-wires, the destructured witnesses and the earlier ghost facts it mentions),
and every consumer (the later ghost clauses, the `ghost pre` theorems, the field
proofs, the knot bodies' second pass) replays it by application
(`hydro_ghost_have h := M.ghost.h n`), so no consumer re-elaborates the
preceding ghost chain. The elaboration context is the replayed ghost context:
`elabGhostHave` runs the pending clauses on a scratch goal and elaborates the
statement and proof in the resulting local context. After a goal-shaped clause
(`ghost witness`/`intro`/`subst`) the fact's context is the refined goal's, so
it stays a replayed tactic. -/
@[term_elab ghostHave] def elabGhostHave : TermElab :=
    fun stx expectedType? => do
  let name : Ident := ⟨stx[2]⟩
  let val : TSyntax `term := ⟨stx[5]⟩
  let tyStx? : Option (TSyntax `term) := match stx[3].getArgs with
    | #[] => none
    | tyArgs => some ⟨tyArgs[1]!⟩
  let queueTactic : TermElabM Unit := do
    let tac ← match tyStx? with
      | none => `(tactic| have $name := $val)
      | some ty => `(tactic| have $name : $ty := $val)
    pendingGhosts.modify (·.push tac)
  match ← getDeclName? with
  | none => queueTactic
  | some declName =>
    if ← ghostGoalShaped.get then queueTactic
    else
      let mName := moduleOfSpec declName
      let thmName := mName ++ (Name.mkSimple "ghost" ++ name.getId)
      Term.synthesizeSyntheticMVarsNoPostponing
      unless (← getEnv).contains thmName do
        let ghosts ← pendingGhosts.get
        let (stmt, thmVal) ← withValuesLCtx stx fun _hDecl _V => do
          -- the replayed ghost context, on a scratch goal
          let scratch ← mkFreshExprSyntheticOpaqueMVar (mkConst ``True)
          let replay ← if ghosts.isEmpty then `(tactic| skip)
            else `(tactic| ($[$ghosts:tactic]*))
          let gs ← Term.withoutErrToSorry <| Tactic.run scratch.mvarId! <|
            Tactic.withoutRecover (Tactic.evalTactic replay)
          let some g := gs.head?
            | throwErrorAt stx "ghost have {name.getId}: the ghost replay closed the scratch goal"
          g.withContext do
            let elabFact : TermElabM (Expr × Expr) := Term.withoutErrToSorry do
              match tyStx? with
              | some ty => do
                -- statement and proof together: the proof's unification settles
                -- the statement's open instances (`(w i)[n]?`'s index type from
                -- a reader's `n : Nat`)
                let (S, prf) ← Term.withSynthesize do
                  let S ← elabType ty
                  let prf ← elabTermEnsuringType val S
                  pure (S, prf)
                pure (← instantiateMVars S, ← instantiateMVars prf)
              | none => do
                let prf ← Term.withSynthesize <| elabTerm val none
                let prf ← instantiateMVars prf
                pure (← instantiateMVars (← inferType prf), prf)
            -- each hoisted fact is its own declaration, so it gets the
            -- declaration budget (`maxHeartbeats` counts from here, as for a
            -- top-level theorem) and does not share one command's budget with
            -- every fact before it.
            -- Registered modules are opaque to the fact's elaboration: a
            -- default-transparency `whnf` of a hypothesis about a wire
            -- (`elabAppArgs` on `hmem : p2a ∈ Multiset.map f (List.sum
            -- (payloads_to_send r))`) zeta-deltas the wire and then
            -- delta-unfolds every callee module it reaches, the whole program
            -- down to its tick formers, before getting stuck. A ghost reasons
            -- about callees through their faces (`M.ensures`), never through
            -- their bodies, so `elabGhostHave` marks the callee module
            -- constants `[irreducible]` while the fact elaborates and restores
            -- them after; the generators unfold by equation lemma
            -- (`simp only [M]`) and are unaffected.
            -- The module's own knots and chain bodies (`<M>.<w>`, `<M>.body`)
            -- stay open, so a ghost may read its own structure (the `MonoRel`
            -- instantiation of its pass body, the knot vocabulary).
            let env ← getEnv
            let mods := (HydroGen.genRegistry.getState env).foldl (init := (#[] : Array Lean.Name))
              fun acc n _ =>
                if env.contains n && !mName.isPrefixOf n
                    && getReducibilityStatusCore env n == .semireducible then acc.push n
                else acc
            for m in mods do setReducibilityStatus m .irreducible
            let (S, prf) ← try HydroGen.withDeclDiag s!"ghost {thmName}" (withCurrHeartbeats elabFact)
              finally for m in mods do setReducibilityStatus m .semireducible
            if (← IO.getEnv "HYDRO_GHOST_DIAG").isSome then
              IO.eprintln s!"[ghost] {thmName} cmd-used={((← IO.getNumHeartbeats) - (← readThe Core.Context).initHeartbeats) / 1000}k"
            if S.hasExprMVar || prf.hasExprMVar then
              throwErrorAt stx "ghost have {name.getId}: the fact has metavariables:{indentExpr S}"
            -- close over only the context the fact needs, through binder
            -- types and let-values
            let lctx ← getLCtx
            let mut inSet : Std.HashSet FVarId := {}
            let mut frontier := (Lean.CollectFVars.main S {}).fvarIds
              ++ (Lean.CollectFVars.main prf {}).fvarIds
            while !frontier.isEmpty do
              let mut next : Array FVarId := #[]
              for f in frontier do
                if inSet.contains f then continue
                inSet := inSet.insert f
                let d := lctx.get! f
                let mut used := (Lean.CollectFVars.main (← instantiateMVars d.type) {}).fvarIds
                if let some v := d.value? then
                  used := used ++ (Lean.CollectFVars.main (← instantiateMVars v) {}).fvarIds
                next := next ++ used.filter (fun x => !inSet.contains x)
              frontier := next
            let mut xs : Array Expr := #[]
            for d in lctx do
              if !d.isImplementationDetail && inSet.contains d.fvarId then
                if d.value?.isNone && d.binderInfo.isExplicit && d.userName.hasMacroScopes then
                  throwErrorAt stx "ghost have {name.getId}: the fact depends on the \
                    inaccessible hypothesis {d.userName}; name it"
                xs := xs.push d.toExpr
            let stmt ← instantiateMVars (← mkForallFVars xs S)
            let thmVal ← instantiateMVars (← mkLambdaFVars xs prf)
            if stmt.hasExprMVar || thmVal.hasExprMVar then
              throwErrorAt stx "ghost have {name.getId}: the hoisted theorem has metavariables"
            ghostArity.modify (·.insert thmName xs.size)
            pure (stmt, thmVal)
        HydroGen.addThm thmName stmt thmVal
        HydroFix.pendingInvs.modify (·.push (thmName, stmt, thmVal))
      -- the replay: the theorem at the context's names, with the arity that
      -- `ghostArity` recorded for it, because a `fix` body elaborates twice
      let some arity := (← ghostArity.get).get? thmName
        | throwErrorAt stx "ghost have {name.getId}: hoisted theorem {thmName} has no recorded arity"
      let thmId := mkCIdent thmName
      let arityStx := Syntax.mkNumLit (toString arity)
      let tac ← `(tactic| hydro_ghost_have $name := $thmId $arityStx)
      pendingGhosts.modify (·.push tac)
  elabTerm stx[7] expectedType?

@[term_elab ghostObtain] def elabGhostObtain : TermElab :=
    fun stx expectedType? => do
  let pat : TSyntax `rcasesPat := ⟨stx[2]⟩
  let val : TSyntax `term := ⟨stx[4]⟩
  let tac ← `(tactic| obtain $pat := $val)
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
  pendingGhosts.modify (·.push tac)
  elabTerm stx[7] expectedType?

@[term_elab ghostWitness] def elabGhostWitness : TermElab :=
    fun stx expectedType? => do
  let val : TSyntax `term := ⟨stx[2]⟩
  let tac ← `(tactic| refine ⟨$val, ?_⟩)
  ghostGoalShaped.set true
  pendingGhosts.modify (·.push tac)
  elabTerm stx[4] expectedType?

@[term_elab ghostIntro] def elabGhostIntro : TermElab :=
    fun stx expectedType? => do
  let names : Array Ident := stx[2].getArgs.map (⟨·⟩)
  let tac ← `(tactic| intro $[$names:ident]*)
  ghostGoalShaped.set true
  pendingGhosts.modify (·.push tac)
  elabTerm stx[4] expectedType?

@[term_elab ghostSubst] def elabGhostSubst : TermElab :=
    fun stx expectedType? => do
  let name : Ident := ⟨stx[2]⟩
  let tac ← `(tactic| subst $name:ident)
  ghostGoalShaped.set true
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

/-- The computational value of a contract-faced pair: the `let` chain with the
`Subtype.mk … val prf` at its tail replaced by `val`. -/
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
`M._spec : ∀ xs, {out : τ // ∀ hv : H = Values L mem, match … => P}`; this
derives

- `M : ∀ xs, τ`, the plain program: the pair's value with the `Subtype.mk`
  stripped. It is a non-dependent function, so a callee's arguments are
  ordinary rewrite positions for simp (a dependent `{out // … args …}` result
  type would make them fixed, and the callees' `sr`/`rr` equations would not
  fire under a callee application);
- `M.ensures : ∀ xs[H := Values L mem], P xs (M (Values L mem) xs)`, the face,
  stated at `Values` without the `hv`/`match` wrapper; its proof is
  `(M._spec … ).property rfl` (one delta each side).

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
  -- the generators register the module as a simp unfold target, which realizes
  -- auxiliary constants (`eq_def`)
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

/-! ## The decision census: the nondet / sched-det split

The decision families of `HydroSem` divide into three kinds (the classification
table is in `Sem.lean`): **nondets** (protocol nondeterminism, real at
`Values`, mirrors of Rust `nondet!` sites, what proofs reason about),
**sched-dets** (adversary and runtime freedom, `Unit` at `Values`: the
transports, ∀-quantified like the delivery cursors), and **fuels** (the `fix`
unfolding artifact). `#nondet_census M (nondets := a) (scheds := b)
(fuels := c)` walks `M`'s signature, raw decision binders and decision-record
binders alike and recursively through nested records, and checks the leaf
counts, so the build checks each module's doc-cited Rust `nondet!` tally. -/

namespace Census

open Lean Elab Command Meta

/-- Leaf classification of a decision type by its head constant. -/
inductive DecKind where
  | nondet | sched | fuel
  deriving BEq, Repr

private def classifyHead : Name → Option DecKind
  | ``HydroSem.SnapDec | ``HydroSem.BatchDec
  | ``HydroSem.OrderSelDec | ``HydroSem.TickOrderSelDec
  | ``HydroSem.TicksDec | ``HydroSem.ClockDec =>
    some .nondet
  | ``HydroSem.TransportDec | ``HydroSem.IntervalDec => some .sched
  | ``HydroSem.FixDec => some .fuel
  | _ => none

/-- Collect the decision leaves of a type with their access paths: family
applications are leaves; structure types recurse through their fields. -/
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

/-- `hydro [hints]? def M … := …`, the module command: elaborate the declaration
under `M._spec` with the `fix` and ghost syntax in scope, derive `M` and
`M.ensures`, register the composition spec of each knot the `fix` elaborator
hoisted and run its pipeline, replay the theorems proven during elaboration,
then register `M` and run its pipeline. -/
elab doc:(docComment)? "hydro " hints:(hydroGenIds)?
    d:Parser.Command.declaration : command => do
  -- graft a leading doc comment onto the wrapped declaration's modifiers, so
  -- `/-- … -/ hydro def M` documents `M`
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
  HydroGhost.ghostGoalShaped.set false
  HydroGhost.ghostArity.set {}
  HydroGen.currentRequires.set none
  HydroGen.currentPres.set #[]
  HydroGen.loopInvariants.set {}
  HydroGen.fixLoopStack.set []
  HydroGen.knotLoopInv.set {}
  HydroGen.loopInvHook.set none
  let hintNames ← elabExtras (hints.map (·.raw))
  HydroFix.pendingHints.set hintNames
  let some declId := d.raw.find? (·.isOfKind ``Parser.Command.declId)
    | throwErrorAt d "hydro def: no declaration name found"
  -- a module elaborates its contract-faced pair under `M._spec`; the module `M`
  -- (the plain program, a non-dependent function) and its face theorem
  -- `M.ensures`: `deriveModule` derives both from it below
  let specIdStx : Syntax :=
    mkIdentFrom declId[0] (declId[0].getId ++ `_spec)
  let d : TSyntax ``Parser.Command.declaration := ⟨d.raw.replaceM (m := Id) fun n =>
    if n == declId then some (declId.setArg 0 specIdStx) else none⟩
  let pdiag := (← IO.getEnv "HYDRO_PHASE_DIAG").isSome
  let tD0 ← IO.monoMsNow
  try
    elabCommand d
  finally
    HydroFix.pendingHints.set #[]
  if pdiag then IO.eprintln s!"[phase] {specIdStx.getId} def-elab: {(← IO.monoMsNow) - tD0}ms"
  let specName ← liftCoreM <| realizeGlobalConstNoOverload specIdStx
  let mName := HydroGhost.moduleOfSpec specName
  let tD1 ← IO.monoMsNow
  liftTermElabM <| HydroGhost.deriveModule specName mName
  if pdiag then IO.eprintln s!"[phase] {mName} derive: {(← IO.monoMsNow) - tD1}ms"
  -- the emitted knots first (emission order is innermost-first), then the
  -- module itself
  let knots := (← HydroFix.pendingKnots.get).foldl
    (fun (acc : Array Name) k => if acc.contains k then acc else acc.push k)
    #[]
  HydroFix.pendingKnots.set #[]
  for k in knots do
    let tK ← IO.monoMsNow
    liftTermElabM <| registerSpec k
    if pdiag then IO.eprintln s!"[phase] {k} registerSpec: {(← IO.monoMsNow) - tK}ms"
    logInfo m!"hydro def {mName}: emitted knot {k}"
    runPipeline k hintNames
  -- replay the theorems proven during elaboration (possibly in a speculative
  -- branch) into the persistent environment
  let invs ← HydroFix.pendingInvs.get
  HydroFix.pendingInvs.set #[]
  for (n, stmt, val) in invs do
    unless (← getEnv).contains n do
      liftTermElabM <| HydroGen.addThm n stmt val
      logInfo m!"hydro def {mName}: replayed {n}"
  let tR ← IO.monoMsNow
  liftTermElabM <| registerSpec mName
  if pdiag then IO.eprintln s!"[phase] {mName} registerSpec: {(← IO.monoMsNow) - tR}ms"
  let gi := (getGenInfo (← getEnv) mName).getD {}
  logInfo m!"hydro def {mName}: callees {gi.callees}, \
    inlines {gi.inlines}, fix := {gi.hasFix}"
  runPipeline mName hintNames
  if pdiag then IO.eprintln s!"[phase] {mName} TOTAL: {(← IO.monoMsNow) - tD0}ms"


end HydroGen

/-! ## `#guard_eval`: executable witnesses by the compiled evaluator

`#guard` decides its proposition by kernel reduction (`decide`). The `Values`
denotation of a keyed read (`kviews`/`legalView`/`readAt`, `Finset.max`) is
cheap to execute and very slow to `whnf`, so executable smoke tests of programs
run the compiled code instead (the evaluator the `falsify`/`explore`/`paxos`/
`twopc` executables use) and fail the build on `false`. -/
elab "#guard_eval " t:term : command => do
  let stx ← `(#eval (if ($t) then (pure () : IO Unit)
    else throw (IO.userError "#guard_eval: the proposition evaluated to false")))
  Lean.Elab.Command.elabCommand stx

end Hydro
