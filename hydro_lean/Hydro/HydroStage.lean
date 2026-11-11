import Hydro.Project.Fused
import Hydro.HydroGen

/-!
# `HydroStage`: the production activation compiled to straight-line code

`Dag.iterateBytes m tk d cod …` is an interpreter: a walk over the graph `d`
(data) that dispatches on every node, rebuilds the typed environment at every
`letN` and computes the knownness masks and the knot rounds anew at every
activation, although every one of those is a function of the graph alone. The
compiler cannot see through any of it, because the graph arrives as a value.

`hydro_run` evaluates the interpreter **at elaboration time** on the closed
graph. It executes the fused sweep (`Dag.sweepF`, `Project/Fused.lean`)
symbolically: the sweep's control flow (masks, phases, knot rounds, wire numbers)
runs over literal data in `MetaM`, and every data step (a node's `locStep`,
`release`, `pushStep`, its inbox, its oracle, its encoding) becomes one `let`
whose value is the operator's own arm, unfolded. The result is one
straight-line function per program and process with no `Dag`, no `HList`
spine, no mask and no dispatch: the operators applied to local variables, with
the program's closures as literal lambdas the compiler can inline and
specialize.

The staged function takes the shell's `TickReadings` (the clock and the timers'
fire counts); each timed node's timing is `Node.Timing.ofReadings m r n`,
reduced at the node, so no per-activation `Dag.Timing.ofReadings` walk over the
graph remains.

The bridge is a kernel judgement (`addThmByKernelRfl`): the kernel normalizes
the interpreter on the closed graph. `name.eq_impl : name … =
Dag.iterateBytesImpl … (Dag.Timing.ofReadings m r d) …`, then `name.eq : name …
= Dag.iterateBytes …` through `Dag.iterateBytes_eq_impl`. The wired headline is
stated over the staged function itself (`paxosRun`/`paxosBody`,
`Paxos/WiredSafety.lean`; its proof opens with `paxosRun.eq`) and the shell
calls that definition. Nothing in the semantics, the generator's `_project`
phase or the proof modules changes: this file consumes `M.dag` as the
executables do.

## What the stager must and must not reduce

* Never `whnf` with δ to reach a constructor through a projection. `whnfCore`'s
  default projection mode unfolds anything to reach a constructor, and the
  cells' projection chains are deep (`(Node.memoStateBox m n st).1` with `st`
  a 100-deep chain), so this file's own procedure (`reduceHead`) reduces
  projections, and the cells' and codecs' projections are primitive
  `Expr.proj`s.
* Never let an unfolding see a nested unreduced argument. Smart unfolding
  reduces its discriminants, so an inbox nested inside an enqueue nested inside
  a memo would be reduced three times; each is a `let` first.
* Unfold the type-level functions (`typeLevel`) wherever they occur. The
  compiler's front end (`toLCNF`) infers and `whnf`s the type of every subterm,
  and a type over a free variable is never cached, so a folded
  `Dag.LocState m (D cfg)` would be re-walked per projection.
* Never unfold a `let`-bound variable (`whnfNZ`, `zetaDelta := false`): the
  lets form the program.

The command reports its three phases (stage, compile, kernel) with their
heartbeats. Each phase runs under its own budget (`withCurrHeartbeats`, what
Lean gives each declaration of a block), none overridden. The compile and
kernel times it prints are meaningful only with `-DElab.async=false`.
-/

namespace Hydro

namespace HydroStage

open Lean Elab Term Meta Command Hydro.Project

/-! ## Reduction with a whitelist

The staged `let`s hold the operators' arms. An arm is obtained by unfolding the
interpreter's definitions at the node (`Node.locStep`, `Node.release`, …) and
reducing the match on the node's constructor: `memo`/`memo2`/
`memoBox` stay (the kernel sees through them; the compiler materializes them),
the operators' library calls stay (the compiler inlines what it wants), and
`let`s inside an arm are kept as `let`s (sharing). -/

/-- The type-level functions of the machine model: a node's or a graph's cells, codecs,
oracle, sends, inbox and timing types, computed from the node. The stager unfolds them wherever
they occur (the lets' types, the arms' implicit arguments and binder types), because the compiler's
front end infers the type of every subterm and `whnf`s it, and a type with a free variable
is never cached; left folded, each occurrence would be a matcher reduction over the node,
thousands of times. -/
def typeLevel : Array Name := #[
  ``Node.LocState, ``Dag.LocState, ``Node.Codecs, ``Dag.Codecs, ``Node.Oracle, ``Dag.Oracle,
  ``Node.Sends, ``Dag.Sends, ``Node.Inbox, ``Dag.Inbox, ``Node.Timing, ``Dag.Timing,
  ``StateBox, ``Kind.locDelta]

/-- The heads the stager may unfold. -/
def unfoldable : Array Name := #[
  ``Node.locStep, ``Node.release, ``Node.pushStep, ``Node.memoState, ``Node.memoStateBox,
  ``StateBox.val, ``Node.enqueueInbox, ``Node.inboxOfWire, ``Node.oracleOf, ``Node.encodeMsgs,
  ``Node.wireEncode, ``Node.fillerLoc, ``Node.sendsNil, ``memoEnv, ``Node.Timing.ofReadings, ``Dag.Timing.ofReadings,
  ``HList.get, ``HList.take, ``HList.drop, ``HList.append, ``HList.map, ``HList.head, ``HList.tail,
  ``HList.cons, ``HList.nil, ``readShapeLoc, ``unzipShapeLoc, ``PendingOf.pushLoc,
  ``Prod.fst, ``Prod.snd, ``knotFillLoc, ``KnotKind.fillLoc, ``shapeKinds, ``List.map, ``List.append,
  ``List.length, ``KnotKind.toKind, ``Function.comp, ``id] ++ typeLevel

/-- `whnf` that never unfolds a `let`-bound variable. The lets bind the staged program's
earlier steps; unfolding one would re-evaluate it symbolically and, on a condition over
symbolic inputs, to no end. -/
def whnfNZ (e : Expr) : MetaM Expr :=
  withConfig (fun c => { c with zetaDelta := false }) (whnf e)

/-- An `if` whose condition is decided by a closed instance: the branch taken (the
`enc`/`fin` flags of the fused sweep are literals here). `none` if the instance does not
reduce to a constructor. -/
def decideIte (e : Expr) : MetaM (Option Expr) := do
  if e.isAppOfArity ``ite 5 then
    let inst ← whnfNZ (e.getArg! 2)
    if inst.isAppOf ``Decidable.isTrue then return some (e.getArg! 3)
    if inst.isAppOf ``Decidable.isFalse then return some (e.getArg! 4)
    return none
  if e.isAppOfArity ``dite 5 then
    let inst ← whnfNZ (e.getArg! 2)
    if inst.isAppOfArity ``Decidable.isTrue 2 then return some (mkApp (e.getArg! 3) (inst.getArg! 1))
    if inst.isAppOfArity ``Decidable.isFalse 2 then return some (mkApp (e.getArg! 4) (inst.getArg! 1))
    return none
  return none

/-- Head-reduce `e`, unfolding only whitelisted constants (no ζ: `let`s are kept). -/
partial def reduceHead (e : Expr) : MetaM Expr := do
  -- a projection: reduce the structure by this procedure (never `whnf` with delta, whose
  -- default would unfold anything to reach a constructor), then project
  if let .proj _ i c := e then
    let c' ← reduceHead c
    match ← projectCore? c' i with
    | some v => return ← reduceHead v
    | none => return if c' == c then e else e.updateProj! c'
  -- no ζ (the lets form the program), no projection-with-delta (handled above)
  let e ← withConfig (fun c => { c with zeta := false, zetaDelta := false, proj := .yes }) (whnfCore e)
  if let .proj .. := e then return ← reduceHead e
  match e.getAppFn with
  | .const n _ =>
    if unfoldable.contains n then
      match ← unfoldDefinition? e with
      | some e' => reduceHead e'
      | none => return e
    else if n == ``ite || n == ``dite then
      match ← decideIte e with
      | some e' => reduceHead e'
      | none => return e
    else return e
  | _ => return e

/-- Deep reduction: `reduceHead` at every subterm whose head is whitelisted, under
binders too (the arms' closures read the environment through `HList.get`). -/
partial def reduceDeep (e : Expr) : MetaM Expr := do
  Meta.transform e (pre := fun e => do
    match e.getAppFn with
    | .const n _ =>
      if unfoldable.contains n || n == ``ite || n == ``dite then
        let e' ← reduceHead e
        if e' == e then return .continue else return .visit e'
      else return .continue
    | _ => return .continue)

/-! ## The graph, inspected -/

/-- A list literal (`List.cons`/`List.nil`), reducing as needed. -/
partial def listLit (e : Expr) : MetaM (Array Expr) := do
  let e ← whnf e
  if e.isAppOfArity ``List.cons 3 then
    return #[e.getArg! 1] ++ (← listLit (e.getArg! 2))
  else if e.isAppOfArity ``List.nil 1 then return #[]
  else throwError "hydro_run: expected a list literal, got{indentExpr e}"

/-- A `Var` literal as a de Bruijn index. -/
partial def varIdx (v : Expr) : MetaM Nat := do
  let v ← whnfR v
  if v.isAppOf ``Var.zero then return 0
  else if v.isAppOfArity ``Var.succ 6 then return (← varIdx (v.getArg! 5)) + 1
  else throwError "hydro_run: expected a variable literal, got{indentExpr v}"

/-- An `HList (Var Γ) κs` literal as indices (`HList.cons` is a reducible pair). -/
partial def varsLit (xs : Expr) : MetaM (Array Nat) := do
  let xs ← whnfR xs
  if xs.isAppOfArity ``Prod.mk 4 then
    return #[← varIdx (xs.getArg! 2)] ++ (← varsLit (xs.getArg! 3))
  else if xs.isAppOf ``PUnit.unit then return #[]
  else throwError "hydro_run: expected a variable list, got{indentExpr xs}"

/-- The variables a shaped tick input reads (`VarsOf Γ ℓ sh`), by the shape. -/
partial def shapeVars (sh x : Expr) : MetaM (Array Nat) := do
  let sh ← whnfR sh
  if sh.isAppOf ``TickShape.pair then
    let x ← whnfR x
    unless x.isAppOfArity ``Prod.mk 4 do throwError "hydro_run: shaped input is not a pair{indentExpr x}"
    return (← shapeVars (sh.getArg! 0) (x.getArg! 2)) ++ (← shapeVars (sh.getArg! 1) (x.getArg! 3))
  else return #[← varIdx x]

/-! ## The stager's bookkeeping: the sweep's `Part`, at the meta level -/

inductive Phase | todo | released | done
  deriving BEq, Repr

/-- A wire in the environment: its kind, its increment (a let-bound variable, when known)
and its knownness. -/
structure Wire where
  kind : Expr
  val : Option Expr
  known : Bool
  deriving Inhabited

mutual
/-- What the stager knows about a node: its phase, its stored outputs and sends, its cells
(the variable holding them now), its codec and timing projections, its wire number, and its
sub-structure. -/
inductive NodeSt where
  | mk (node : Expr) (kinds ctxKinds : Array Expr) (cell cod tm : Expr) (base : Nat)
       (phase : Phase) (outs : Array Expr) (snd : Option Expr) (visited : Bool) (sub : SubSt)
inductive SubSt where
  | plain
  | call (args : Array Nat) (body : Array NodeSt) (retVars : Array Nat)
  | knot (ks : Array Expr) (bound : Array Bool) (kv : Array (Option Expr)) (body : Array NodeSt) (retVars : Array Nat)
end

namespace NodeSt
def node : NodeSt → Expr | .mk n .. => n
def kinds : NodeSt → Array Expr | .mk _ k .. => k
def ctxKinds : NodeSt → Array Expr | .mk _ _ c .. => c
def cell : NodeSt → Expr | .mk _ _ _ c .. => c
def cod : NodeSt → Expr | .mk _ _ _ _ c .. => c
def tm : NodeSt → Expr | .mk _ _ _ _ _ t .. => t
def base : NodeSt → Nat | .mk _ _ _ _ _ _ b .. => b
def phase : NodeSt → Phase | .mk _ _ _ _ _ _ _ p .. => p
def outs : NodeSt → Array Expr | .mk _ _ _ _ _ _ _ _ o .. => o
def snd : NodeSt → Option Expr | .mk _ _ _ _ _ _ _ _ _ s .. => s
def visited : NodeSt → Bool | .mk _ _ _ _ _ _ _ _ _ _ v .. => v
def sub : NodeSt → SubSt | .mk _ _ _ _ _ _ _ _ _ _ _ s => s
def withCell (st : NodeSt) (c : Expr) : NodeSt := match st with | .mk n k cx _ cd t b p o s v su => .mk n k cx c cd t b p o s v su
def withPhase (st : NodeSt) (p : Phase) : NodeSt := match st with | .mk n k cx c cd t b _ o s v su => .mk n k cx c cd t b p o s v su
def withOuts (st : NodeSt) (o : Array Expr) : NodeSt := match st with | .mk n k cx c cd t b p _ s v su => .mk n k cx c cd t b p o s v su
def withSnd (st : NodeSt) (s : Option Expr) : NodeSt := match st with | .mk n k cx c cd t b p o _ v su => .mk n k cx c cd t b p o s v su
def withVisited (st : NodeSt) (v : Bool) : NodeSt := match st with | .mk n k cx c cd t b p o s _ su => .mk n k cx c cd t b p o s v su
def withSub (st : NodeSt) (su : SubSt) : NodeSt := match st with | .mk n k cx c cd t b p o s v _ => .mk n k cx c cd t b p o s v su
end NodeSt

/-- The fixed data of a staging. -/
structure Ctx where
  L : Expr
  mem : Expr
  m : Expr
  tk : Expr
  arr : Expr
  batches : Expr
  /-- Every `let` introduced, in order (`mkLetFVars` at the end). -/
  lets : IO.Ref (Array Expr)
  /-- Node ordinals for the `let` names. -/
  counter : IO.Ref Nat

def kindTy (c : Ctx) : Expr := mkApp2 (mkConst ``Kind) c.L c.mem
def kindList (c : Ctx) (ks : Array Expr) : Expr :=
  ks.foldr (fun k acc => mkApp3 (mkConst ``List.cons [Level.one]) (kindTy c) k acc)
    (mkApp (mkConst ``List.nil [Level.one]) (kindTy c))
def locDeltaF (c : Ctx) : Expr := mkApp3 (mkConst ``Kind.locDelta) c.L c.mem c.m

/-- `HList.cons`/`nil` over `Kind.locDelta m` from values and kinds (top-first). -/
def hlistLoc (c : Ctx) (ks vals : Array Expr) : Expr := Id.run do
  let f := locDeltaF c
  let mut acc := mkApp3 (mkConst ``HList.nil [Level.zero]) c.L c.mem f
  for i in [0:ks.size] do
    let j := ks.size - 1 - i
    acc := mkAppN (mkConst ``HList.cons [Level.zero]) #[c.L, c.mem, f, ks[j]!, kindList c (ks.extract (j + 1) ks.size), vals[j]!, acc]
  return acc

def boolLit (b : Bool) : Expr := mkConst (if b then ``Bool.true else ``Bool.false)
def unitE : Expr := mkConst ``Unit.unit

/-- `Var` for index `i` in the top-first context `ks`. -/
partial def mkVarLit (c : Ctx) (ks : Array Expr) (i : Nat) : MetaM Expr := do
  let target := ks[i]!
  let rec go (j : Nat) (rest : List Expr) : MetaM Expr := do
    match rest with
    | [] => throwError "hydro_run: variable out of range"
    | κ :: tl =>
      if j = 0 then return mkAppN (mkConst ``Var.zero) #[c.L, c.mem, kindList c tl.toArray, κ]
      else
        let v ← go (j - 1) tl
        return mkAppN (mkConst ``Var.succ) #[c.L, c.mem, kindList c tl.toArray, target, κ, v]
  go i ks.toList

/-- Bind `let x : ty := v`, record the variable. -/
def bindLet (c : Ctx) (x : Name) (ty v : Expr) (k : Expr → TermElabM Expr) : TermElabM Expr := do
  let ty ← reduceDeep ty
  withLetDecl x ty v fun fv => do
    c.lets.modify (·.push fv)
    k fv

/-- Bind `let x := v` with the type inferred and the value deep-reduced. A value that is
already a variable, a constant, or of type `Unit`/`PUnit` (a stateless node's cells, a plain
node's oracle or sends) is used as it is, with no `let`. -/
def bindRed (c : Ctx) (x : Name) (v : Expr) (k : Expr → TermElabM Expr) : TermElabM Expr := do
  let v ← reduceDeep v
  if v.isFVar || v.isConst then return ← k v
  let ty ← inferType v
  let tyW ← whnfNZ ty
  if tyW.isConstOf ``Unit || tyW.isConstOf ``PUnit || tyW.isAppOf ``PUnit then
    return ← k (mkConst ``Unit.unit)
  bindLet c x ty v k

/-- The node constructor's name. -/
def nodeCtor (n : Expr) : MetaM Name := do
  let n ← whnf n
  match n.getAppFn with
  | .const nm _ => return nm
  | _ => throwError "hydro_run: not a node constructor{indentExpr n}"

def isNetwork (ctor : Name) : Bool := ctor == ``Node.broadcast_closed || ctor == ``Node.demux
def isStrict (ctor : Name) : Bool := isNetwork ctor || ctor == ``Node.defer_tick

/-- The variables a plain node reads. -/
def nodeInputs (n : Expr) : MetaM (Array Nat) := do
  let n ← whnf n
  let ctor ← nodeCtor n
  let a := n.getAppArgs
  if ctor == ``Node.tick_scan then
    -- tick_scan {L mem} {Γ} {ℓ} sts ins outs x g seed
    return ← shapeVars a[5]! a[7]!
  let mut vs : Array Nat := #[]
  for arg in a do
    let ty ← inferType arg
    if (← whnfR ty).isAppOf ``Var then vs := vs.push (← varIdx arg)
  return vs

/-! ## Building the bookkeeping from the graph -/

mutual
/-- Walk a graph literal: one `NodeSt` per `letN`, with the cell/codec/timing
projections of `s`/`cod`/`tm`, the wire numbers threaded; returns the nodes, the
`ret` variables and the next wire number. -/
partial def buildDag (c : Ctx) (ctxKinds : Array Expr) (d s cod r : Expr) (base : Nat) :
    MetaM (Array NodeSt × Array Nat × Nat) := do
  let d ← whnf d
  if d.isAppOfArity ``Dag.letN 7 then
    -- letN {L mem} {Γ} {κs outs} n rest
    let n := d.getArg! 5
    let rest := d.getArg! 6
    let κs ← listLit (d.getArg! 3)
    -- primitive projections: `whnf` of an `Expr.proj` chain is structural, while a
    -- `Prod.snd` chain goes through the projection-function machinery at every level
    let sN := mkProj ``Prod 0 s; let sR := mkProj ``Prod 1 s
    let cN := mkProj ``Prod 0 cod; let cR := mkProj ``Prod 1 cod
    -- this activation's timing at the node, from the shell's readings
    -- (`Node.Timing.ofReadings` is a function of the readings and the node; no timing
    -- tuple is built)
    let tN ← mkAppM ``Node.Timing.ofReadings #[c.m, r, n]
    let (st, base') ← buildNode c ctxKinds n κs sN cN tN r base
    let (nodes, retVars, base'') ← buildDag c (κs ++ ctxKinds) rest sR cR r base'
    return (#[st] ++ nodes, retVars, base'')
  else if d.isAppOfArity ``Dag.ret 5 then
    return (#[], ← varsLit (d.getArg! 4), base)
  else throwError "hydro_run: not a graph literal{indentExpr d}"

partial def buildNode (c : Ctx) (ctxKinds : Array Expr) (n : Expr) (κs : Array Expr) (s cod tm r : Expr) (base : Nat) :
    MetaM (NodeSt × Nat) := do
  let n ← whnf n
  let ctor ← nodeCtor n
  let a := n.getAppArgs
  if ctor == ``Node.call then
    -- call {L mem} {Γ} {Γ'} {outs} N args
    let N := a[5]!
    let args ← varsLit a[6]!
    let Γ' ← listLit a[3]!
    let (body, retVars, base') ← buildDag c Γ' N s cod r base
    return (.mk n κs ctxKinds s cod tm base .todo #[] none false (.call args body retVars), base')
  else if ctor == ``Node.fixN then
    -- fixN {L mem} {Γ} ks {outs} body
    let ks ← listLit a[3]!
    let kKinds ← ks.mapM fun k => reduceHead (mkApp3 (mkConst ``KnotKind.toKind) c.L c.mem k)
    let (body, retVars, base') ← buildDag c (kKinds ++ ctxKinds) a[5]! s cod r base
    return (.mk n κs ctxKinds s cod tm base .todo #[] none false
      (.knot ks (ks.map fun _ => false) (ks.map fun _ => none) body retVars), base')
  else
    let base' := if isNetwork ctor then base + 1 else base
    return (.mk n κs ctxKinds s cod tm base .todo #[] none false .plain, base')
end

/-! ## The sweep, executed symbolically -/

def natLit (n : Nat) : Expr := mkNatLit n

/-- The bucket of wire node `k`: `(arr[k]?).getD (bucketOf batches k)`. -/
def bucketExpr (c : Ctx) (k : Nat) : TermElabM Expr := do
  let arrS ← Term.exprToSyntax c.arr
  let bS ← Term.exprToSyntax c.batches
  let kS := Syntax.mkNumLit (toString k)
  let e ← Term.elabTerm (← `((($arrS)[$kS]?).getD (bucketOf $bS $kS))) none
  Term.synthesizeSyntheticMVarsNoPostponing
  instantiateMVars e

/-- Bind the components of an `HList` literal of outputs, each through `memo`. -/
partial def bindOuts (c : Ctx) (nm : String) (κs : Array Expr) (h : Expr) (i : Nat) (acc : Array Expr)
    (k : Array Expr → TermElabM Expr) : TermElabM Expr := do
  if i = κs.size then return ← k acc
  let h ← reduceHead h
  unless h.isAppOfArity ``Prod.mk 4 do throwError "hydro_run: {nm}: outputs are not a list literal{indentExpr h}"
  let mx ← mkAppM ``memo #[h.getArg! 2]
  bindRed c (Name.mkSimple s!"{nm}_o{i}") mx fun v =>
    bindOuts c nm κs (h.getArg! 3) (i + 1) (acc.push v) k

/-- Bind the destructured result `r : State × HList outs × Sends` of a step (after
reduction a pair literal, possibly under `let`s, which are bound as lets): the state,
the outputs (one variable per output kind, through `memo` as `memoEnv` does), the sends. -/
partial def bindResult (c : Ctx) (nm : String) (κs : Array Expr) (r : Expr)
    (k : Expr → Array Expr → Expr → TermElabM Expr) : TermElabM Expr := do
  let r ← reduceHead r
  if r.isLet then
    let v ← reduceDeep r.letValue!
    let ty ← reduceDeep r.letType!
    withLetDecl r.letName! ty v fun fv => do
      c.lets.modify (·.push fv)
      bindResult c nm κs (r.letBody!.instantiate1 fv) k
  else if r.isAppOfArity ``Prod.mk 4 then
    let st := r.getArg! 2
    let rest ← reduceHead (r.getArg! 3)
    unless rest.isAppOfArity ``Prod.mk 4 do throwError "hydro_run: {nm}: result is not a triple{indentExpr rest}"
    bindRed c (Name.mkSimple s!"{nm}_st") st fun stV => do
      bindOuts c nm κs (rest.getArg! 2) 0 #[] fun outVs => do
        bindRed c (Name.mkSimple s!"{nm}_snd") (rest.getArg! 3) fun sndV =>
          k stV outVs sndV
  else throwError "hydro_run: {nm}: result does not reduce to a pair{indentExpr r}"

/-- A projection chain `Δv.2.2….1` (as `Expr.proj` or `Prod.fst`/`Prod.snd` applications)
read off the environment variable: the wire index. -/
partial def projIdx (Δv : Expr) (e : Expr) : Option Nat :=
  if e == Δv then none
  else match e with
    | .proj ``Prod 0 x => tailDepth x
    | .app (.app (.app (.const ``Prod.fst _) _) _) x => tailDepth x
    | _ => none
where
  tailDepth (x : Expr) : Option Nat :=
    if x == Δv then some 0
    else match x with
      | .proj ``Prod 1 y => (tailDepth y).map (· + 1)
      | .app (.app (.app (.const ``Prod.snd _) _) _) y => (tailDepth y).map (· + 1)
      | _ => none

/-- Replace the reads of the fresh environment variable `Δv` (`HList.get Δv v` with `v` a
literal, or its reduced form, a projection chain) by the wires' variables. -/
def substEnv (Δv : Expr) (env : Array Wire) (e : Expr) : MetaM Expr := do
  let read (i : Nat) : MetaM Expr := do
    match env[i]!.val with
    | some w => return w
    | none => throwError "hydro_run: a stepped node reads an unknown wire (index {i})"
  Meta.transform e (pre := fun e => do
    if e.isAppOfArity ``HList.get 7 && e.getArg! 5 == Δv then
      return .done (← read (← varIdx (e.getArg! 6)))
    else match projIdx Δv e with
      | some i => return .done (← read i)
      | none => return .continue)

/-- Apply a node-level interpreter function to a fresh environment variable of the
node's context, reduce, and substitute the reads: the arm over the wires' variables. -/
def withEnvVar (c : Ctx) (ctxKinds : Array Expr) (env : Array Wire) (mk : Expr → MetaM Expr) : MetaM Expr := do
  let ΔTy := mkApp4 (Lean.mkConst ``HList [Level.zero]) c.L c.mem (locDeltaF c) (kindList c ctxKinds)
  withLocalDeclD `Δenv ΔTy fun Δv => do
    let e ← mk Δv
    let e ← reduceHead e
    let e ← substEnv Δv env e
    let e ← reduceDeep e
    let e ← substEnv Δv env e
    if e.containsFVar Δv.fvarId! then
      throwError "hydro_run: the environment did not reduce away:{indentExpr e}"
    return e

def knownWires (st : NodeSt) : Array Wire :=
  (Array.range st.kinds.size).map fun j => { kind := st.kinds[j]!, val := some st.outs[j]!, known := true }

mutual

/-- One sweep over a graph's nodes: the environment `env` (top-first wires), returns the
updated nodes and the `ret`'s wires. -/
partial def sweepDag (c : Ctx) (nodes : Array NodeSt) (retVars : Array Nat) (env : Array Wire) (i : Nat)
    (k : Array NodeSt → Array Wire → TermElabM Expr) : TermElabM Expr := do
  if h : i < nodes.size then
    sweepNode c nodes[i] env fun st' env' =>
      sweepDag c (nodes.set i st') retVars env' (i + 1) k
  else
    k nodes (retVars.map fun v => env[v]!)

/-- One sweep over a node. -/
partial def sweepNode (c : Ctx) (st : NodeSt) (env : Array Wire)
    (k : NodeSt → Array Wire → TermElabM Expr) : TermElabM Expr := do
  let enq := !st.visited
  let st := st.withVisited true
  match st.sub with
  | .call args body retVars =>
    sweepDag c body retVars (args.map fun v => env[v]!) 0 fun body' outs =>
      k ((st.withSub (.call args body' retVars)).withOuts (outs.map (·.val.getD unitE))) (outs ++ env)
  | .knot ks bound kv body retVars =>
    knotLoop c st ks bound kv body retVars env ks.size k
  | .plain =>
    let ctor ← nodeCtor st.node
    let ins ← nodeInputs st.node
    let known := ins.all fun v => env[v]!.known
    let ord ← c.counter.modifyGet fun n => (n, n + 1)
    let nm := s!"n{ord}_{ctor.getString!}"
    let withCell (kc : Expr → TermElabM Expr) : TermElabM Expr := do
      if enq then
        if isNetwork ctor then
          let bk ← bucketExpr c st.base
          let ib ← mkAppM ``Node.inboxOfWire #[c.m, st.node, st.cod, natLit st.base, bk]
          -- the inbox, then the enqueue, then the memo, each a let: an unfolding reduces
          -- its discriminants, so a nested unreduced argument would be reduced again inside
          bindRed c (Name.mkSimple s!"{nm}_ib") (mkProj ``Prod 0 ib) fun ibV => do
            bindRed c (Name.mkSimple s!"{nm}_enq") (← mkAppM ``Node.enqueueInbox #[c.m, st.node, ibV, st.cell]) fun enqV => do
              bindRed c (Name.mkSimple s!"{nm}_cell") (← mkAppM ``Node.memoState #[c.m, st.node, enqV]) kc
        else bindRed c (Name.mkSimple s!"{nm}_cell") st.cell kc
      else kc st.cell
    match st.phase with
    | .done => k st (knownWires st ++ env)
    | .released =>
      if known then
        let r ← withEnvVar c st.ctxKinds env fun Δv => mkAppM ``Node.pushStep #[c.m, st.node, Δv, st.cell, st.snd.get!]
        let r ← reduceHead r
        unless r.isAppOfArity ``Prod.mk 4 do throwError "hydro_run: {nm}: push did not reduce{indentExpr r}"
        bindRed c (Name.mkSimple s!"{nm}_pst") (r.getArg! 2) fun stV =>
          bindRed c (Name.mkSimple s!"{nm}_psnd") (r.getArg! 3) fun sndV => do
            let st := ((st.withPhase .done).withCell stV).withSnd (some sndV)
            k st (knownWires st ++ env)
      else k st (knownWires st ++ env)
    | .todo =>
      withCell fun cell => do
        let st := st.withCell cell
        if known || isStrict ctor then
          let ora ← mkAppM ``Node.oracleOf #[c.m, st.node, st.tm, cell]
          bindRed c (Name.mkSimple s!"{nm}_ora") ora fun oraV => do
            let fn := if known then ``Node.locStep else ``Node.release
            let r ← withEnvVar c st.ctxKinds env fun Δv => mkAppM fn #[c.m, c.tk, st.node, oraV, Δv, cell]
            bindResult c nm st.kinds r fun stV outVs sndV => do
              let ms ← mkAppM ``Node.memoState #[c.m, st.node, stV]
              bindRed c (Name.mkSimple s!"{nm}_mst") ms fun mstV => do
                let st := (((st.withPhase (if known then .done else .released)).withCell mstV).withOuts outVs).withSnd (some sndV)
                k st (knownWires st ++ env)
        else
          -- skipped this sweep: fillers nobody reads
          k st (st.kinds.map (fun κ => ({ kind := κ, val := none, known := false } : Wire)) ++ env)

/-- The knot loop (`knotLoopF`): sweep the body with the knots bound so far, bind the
knots that came out known, until a sweep binds nothing new (or the fuel is out). -/
partial def knotLoop (c : Ctx) (st : NodeSt) (ks : Array Expr) (bound : Array Bool) (kv : Array (Option Expr))
    (body : Array NodeSt) (retVars : Array Nat) (env : Array Wire) (fuel : Nat)
    (k : NodeSt → Array Wire → TermElabM Expr) : TermElabM Expr := do
  let kKinds ← ks.mapM fun kk => reduceHead (mkApp3 (mkConst ``KnotKind.toKind) c.L c.mem kk)
  let kWires : Array Wire := (Array.range ks.size).map fun j =>
    { kind := kKinds[j]!, val := if bound[j]! then kv[j]! else none, known := bound[j]! }
  sweepDag c body retVars (kWires ++ env) 0 fun body' outs => do
    let newK := outs.extract 0 ks.size
    let bound' := (Array.range ks.size).map fun j => bound[j]! || newK[j]!.known
    let kv' := (Array.range ks.size).map fun j => if bound[j]! then kv[j]! else newK[j]!.val
    if bound' == bound || fuel = 0 then
      let allOuts := (Array.range ks.size).map (fun j => ({ kind := kKinds[j]!, val := kv'[j]!, known := bound'[j]! } : Wire))
        ++ outs.extract ks.size outs.size
      let st' := ((st.withSub (.knot ks bound' kv' body' retVars)).withPhase (if bound'.all id then .done else .todo)).withOuts (allOuts.map (·.val.getD unitE))
      k st'
        (allOuts ++ env)
    else knotLoop c st ks bound' kv' body' retVars env (fuel - 1) k

end

/-! ## Assembly -/

mutual
/-- The cells after the activation, as the nested tuple `Dag.LocState`. -/
partial def cellsOf (c : Ctx) (nodes : Array NodeSt) (i : Nat) : TermElabM Expr := do
  if h : i < nodes.size then
    let st := nodes[i]
    let cellN ← nodeCells c st
    let cellR ← cellsOf c nodes (i + 1)
    mkAppM ``Prod.mk #[cellN, cellR]
  else return unitE

partial def nodeCells (c : Ctx) (st : NodeSt) : TermElabM Expr := do
  match st.sub with
  | .plain => return st.cell
  | .call _ body _ => cellsOf c body 0
  | .knot _ _ _ body _ => cellsOf c body 0
end

/-- The messages of the activation, **with the fused sweep's nesting**: per graph a
right-nested chain `nodeMsgs n ++ (… ++ [])`, a sub-graph's chain spliced in as one element.
`List.append` is not associative definitionally on symbolic lists, so the kernel judgement
needs the same shape. -/
partial def msgsOf (c : Ctx) (nodes : Array NodeSt) (k : Expr → TermElabM Expr) : TermElabM Expr := do
  let msgTy := Lean.mkConst ``WireMsg
  let nil := mkApp (Lean.mkConst ``List.nil [Level.zero]) msgTy
  let app (a b : Expr) : Expr := mkApp3 (Lean.mkConst ``List.append [Level.zero]) msgTy a b
  -- `go` returns the chain of a node list (CPS, binding the encodings as lets)
  let rec go (todo : List NodeSt) (k : Expr → TermElabM Expr) : TermElabM Expr := do
    match todo with
    | [] => k nil
    | st :: rest =>
      match st.sub with
      | .plain =>
        let ctor ← nodeCtor st.node
        if isNetwork ctor then
          let e ← mkAppM ``Node.encodeMsgs #[c.m, st.node, st.cod, natLit st.base, boolLit true, st.snd.get!]
          bindRed c (Name.mkSimple s!"n{st.base}_msgs") e fun v =>
            go rest fun acc => k (app v acc)
        else go rest k   -- `[] ++ acc` reduces to `acc`
      | .call _ body _ => go body.toList fun sub => go rest fun acc => k (app sub acc)
      | .knot _ _ _ body _ => go body.toList fun sub => go rest fun acc => k (app sub acc)
  go nodes.toList k

/-! ## The command -/

/-- `hydro_run name binders* where member := m, graph := d, codecs := c`: the straight-line
activation `name binders tk r chans Δ s` (`r : TickReadings m`), with the kernel-checked
`name.eq_impl : name … = Dag.iterateBytesImpl m tk d c (Dag.Timing.ofReadings m r d) chans Δ s`
and `name.eq : name … = Dag.iterateBytes …`. A doc comment goes to `name`. -/
syntax (name := hydroRun) (docComment)? "hydro_run " ident bracketedBinder* " where " &"member" " := " term ", " &"graph" " := " term ", " &"codecs" " := " term : command

@[command_elab hydroRun] def elabHydroRun : CommandElab := fun stx => do
  match stx with
  | `($[$doc?:docComment]? hydro_run $nm:ident $bs* where member := $mT, graph := $dT, codecs := $cT) =>
    liftTermElabM do
    Term.elabBinders bs fun binders => do
      let dag ← Term.elabTerm dT none
      Term.synthesizeSyntheticMVarsNoPostponing
      let dag ← instantiateMVars dag
      let dagTy ← whnfR (← inferType dag)
      unless dagTy.isAppOfArity ``Dag 4 do throwError "hydro_run: not a graph:{indentExpr dagTy}"
      let L := dagTy.getArg! 0; let mem := dagTy.getArg! 1
      let ΓE := dagTy.getArg! 2; let outsE := dagTy.getArg! 3
      let mE ← Term.elabTermEnsuringType mT (mkApp2 (mkConst ``Member) L mem)
      Term.synthesizeSyntheticMVarsNoPostponing
      let mE ← instantiateMVars mE
      let codE ← Term.elabTermEnsuringType cT (← mkAppM ``Dag.Codecs #[dag])
      Term.synthesizeSyntheticMVarsNoPostponing
      let codE ← instantiateMVars codE
      let Γk ← listLit ΓE
      let outsK ← listLit outsE
      let rTy := mkApp3 (Lean.mkConst ``TickReadings) L mem mE
      let memberTy := mkApp2 (mkConst ``Member) L mem
      let chansTy := mkApp (mkConst ``List [Level.zero])
        (mkApp2 (mkConst ``Sigma [Level.zero, Level.zero]) memberTy (mkLambda `src .default memberTy (mkConst ``ByteArray)))
      let ΔTy := mkApp4 (mkConst ``HList [Level.zero]) L mem (mkApp3 (mkConst ``Kind.locDelta) L mem mE) ΓE
      let sTy ← mkAppM ``Dag.LocState #[mE, dag]
      withLocalDeclD `tk (mkConst ``Bool) fun tk =>
      withLocalDeclD `r rTy fun r =>
      withLocalDeclD `chans chansTy fun chans =>
      withLocalDeclD `Δ ΔTy fun Δ =>
      withLocalDeclD `s sTy fun s => do
        let lets ← IO.mkRef (#[] : Array Expr)
        let counter ← IO.mkRef (0 : Nat)
        let elabT (stx : Term) : TermElabM Expr := do
          let e ← Term.elabTerm stx none
          Term.synthesizeSyntheticMVarsNoPostponing
          instantiateMVars e
        let t0 ← IO.monoMsNow
        let hb0 ← IO.getNumHeartbeats
        let parsedV ← elabT (← `(($(← Term.exprToSyntax chans)).map fun c => (⟨c.1, parseFrames c.2⟩ : (_src : Member _ _) × List WireMsg × ByteArray)))
        -- the cells and the codecs, each a `let` at its reduced type, the nested product
        -- of the nodes' concrete cell/codec types: the kernel checks the defeq once, and
        -- the compiler's per-projection type inference then never unfolds
        -- `Dag.LocState`/`Dag.Codecs` over the graph again
        let sTy' ← reduceDeep sTy
        let cTy' ← reduceDeep (← inferType codE)
        let (body, nNodes, wireN) ← withLetDecl `cells sTy' s fun sV => do
          lets.modify (·.push sV)
          withLetDecl `cod cTy' codE fun codV => do
          lets.modify (·.push codV)
          withLetDecl `parsed (← inferType parsedV) parsedV fun parsed => do
          lets.modify (·.push parsed)
          let batchesV ← elabT (← `(($(← Term.exprToSyntax parsed)).map fun c => (⟨c.1, c.2.1⟩ : (_src : Member _ _) × List WireMsg)))
          withLetDecl `batches (← inferType batchesV) batchesV fun batches => do
            lets.modify (·.push batches)
            let c0 : Ctx := { L, mem, m := mE, tk, arr := unitE, batches, lets, counter }
            let (nodes, retVars, wireN) ← buildDag c0 Γk dag sV codV r 0
            let arrV ← mkAppM ``bucketArray #[natLit wireN, batches]
            withLetDecl `arr (← inferType arrV) arrV fun arr => do
              lets.modify (·.push arr)
              let c : Ctx := { c0 with arr }
              let env0 : Array Wire ← (Array.range Γk.size).mapM fun i => do
                let v ← mkAppM ``HList.get #[Δ, ← mkVarLit c Γk i]
                return { kind := Γk[i]!, val := some v, known := true }
              let body ← sweepDag c nodes retVars env0 0 fun nodes' outs => do
                let cells ← cellsOf c nodes' 0
                let outsH := hlistLoc c outsK (outs.map fun w => w.val.getD unitE)
                msgsOf c nodes' fun msgs => do
                  let tailsV ← elabT (← `(($(← Term.exprToSyntax parsed)).map fun c => c.2.2))
                  let res ← mkAppM ``Prod.mk #[cells, ← mkAppM ``Prod.mk #[outsH, ← mkAppM ``Prod.mk #[msgs, tailsV]]]
                  mkLetFVars (← lets.get) res
              return (body, nodes.size, wireN)
        let t1 ← IO.monoMsNow
        let hb1 ← IO.getNumHeartbeats
        -- the phases below are each a budget of their own (`withCurrHeartbeats`, as Lean
        -- gives every declaration of a block): their heartbeats are reported, not pooled
        -- with the stage's
        withCurrHeartbeats do
        let body ← instantiateMVars body
        let allB := binders ++ #[tk, r, chans, Δ, s]
        -- the binders' types in the context may hold universe metavariables assigned
        -- during the stage: instantiate after abstracting
        let fn ← instantiateMVars (← mkLambdaFVars allB body)
        -- restore the sharing the unfoldings lost (the compiler's and the kernel's
        -- traversals cache by pointer)
        let fn := ShareCommon.shareCommon' fn
        let fnTy ← instantiateMVars (← inferType fn)
        if fn.hasMVar || fnTy.hasMVar then
          throwError "hydro_run: metavariables remain in the staged function{indentExpr fnTy}"
        let name := (← getCurrNamespace) ++ nm.getId
        let env ← getEnv
        let hints := ReducibilityHints.regular (getMaxHeight env fn + 1)
        let dv : DefinitionVal := { name := name, levelParams := [], type := fnTy, value := fn, hints := hints, safety := .safe }
        addDecl (.defnDecl dv)
        if let some doc := doc? then
          addDocStringCore name (← getDocStringText doc)
        withCurrHeartbeats <| compileDecl (.defnDecl dv)
        let hb2 ← IO.getNumHeartbeats
        let t2 ← IO.monoMsNow
        withCurrHeartbeats do
        -- the bridge: `name … = Dag.iterateBytesImpl …` by the kernel, then `= Dag.iterateBytes …`
        let lhs := mkAppN (Lean.mkConst name) allB
        let tmE ← mkAppM ``Dag.Timing.ofReadings #[mE, r, dag]
        let implApp ← mkAppM ``Dag.iterateBytesImpl #[mE, tk, dag, codE, tmE, chans, Δ, s]
        let itApp ← mkAppM ``Dag.iterateBytes #[mE, tk, dag, codE, tmE, chans, Δ, s]
        let stmtImpl ← instantiateMVars (← mkForallFVars allB (← mkEq lhs implApp))
        unless ← Hydro.HydroGen.addThmByKernelRfl (name ++ `eq_impl) stmtImpl do
          throwError "hydro_run: the kernel refused `{name ++ `eq_impl}`"
        let t3 ← IO.monoMsNow
        let stmt ← instantiateMVars (← mkForallFVars allB (← mkEq lhs itApp))
        let prf ← instantiateMVars <| ← mkLambdaFVars allB (← do
          let h1 := mkAppN (Lean.mkConst (name ++ `eq_impl)) allB
          let implArgs := implApp.getAppArgs
          let mut h2 : Expr := Lean.mkConst ``Dag.iterateBytes_eq_impl
          for a in implArgs do
            h2 ← mkCongrFun h2 a
          mkEqTrans h1 (← mkEqSymm h2))
        Hydro.HydroGen.addThm (name ++ `eq) stmt prf
        let hb3 ← IO.getNumHeartbeats
        logInfo m!"hydro_run {name}: {nNodes} top-level nodes, {wireN} wire nodes, {(← lets.get).size} lets; stage {t1 - t0} ms ({(hb1 - hb0) / 1000}k heartbeats), compile {t2 - t1} ms ({(hb2 - hb1) / 1000}k), kernel {t3 - t2} ms ({(hb3 - hb2) / 1000}k)"
  | _ => throwUnsupportedSyntax

end HydroStage

end Hydro
