import Hydro.HydroGen
import Hydro.Project.Model.StepMain
import Hydro.Project.Wire

/-!
# Hydro · `HydroGenProject` — the `_project` phase: compile a module to its graph

For every `hydro def M` (knots and chain bodies included, since they are
registered modules themselves) the phase emits

- `M.dag : <M's non-wire, non-schedule binders> → Project.Dag Γ_M outs_M`
  — the module's `let`-chain as an A-normal-form graph: every wire
  `let` one `letN`, nested operator applications ANF'd into anonymous
  nodes, callee invocations `call N.dag`, a `fix … complete` block one
  `fixN` whose body is `call M.body.dag` (the elaborator's hoisted chain
  body IS a registered module), the body's re-application at the closed
  knots (the elaborator's Bekić continuation) read off the `fixN`'s
  exposed outputs, data `let`s kept as Lean `let`s;
- `M.dag_denote : ∀ pacing, Dag.denote pacing (M.dag …) (M.schedOf pacing …) (inputs)
  = outputs (M (SchedSem L mem pacing) …)` — the graph is **pacing-free**: the data
  terms are elaborated at `H := SchedSem L mem pacing` and then taken at
  `Pacing.idle` (the instance names the pacing only through carrier types, which do
  not depend on it; the kernel judges the two sides equal), so one graph serves
  every pacing, and an interleaving of processes is an interleaving of *the* graph
  (`Project/WiredRun.lean`). Proven by unfolding the module's own chain and rewriting every callee
  through its `N.dag_denote` (callees stay folded: no whole-program
  defeq).

The walk is over the elaborated `Expr` of `M`'s value, type-directed
(`inferType` of each subterm decides wire vs. data) and total over the
module grammar: anything carrier-typed that is not an operator
application, a callee invocation, a knot constant or a bound wire is an
error.
-/

namespace Hydro

namespace HydroGen

open Lean Elab Term Meta

namespace Project

/-- Which `Node` constructor an operator field compiles to (the
constructors take the field's arguments in the field's order, with
instance arguments explicit). -/
def nodeCtorOf : Name → Option Name
  | ``HydroSem.map => some ``Hydro.Project.Node.map
  | ``HydroSem.filterMap => some ``Hydro.Project.Node.filterMap
  | ``HydroSem.broadcast_closed => some ``Hydro.Project.Node.broadcast_closed
  | ``HydroSem.demux => some ``Hydro.Project.Node.demux
  | ``HydroSem.values => some ``Hydro.Project.Node.values
  | ``HydroSem.into_keyed => some ``Hydro.Project.Node.into_keyed
  | ``HydroSem.kassume_ordering => some ``Hydro.Project.Node.kassume_ordering
  | ``HydroSem.kfold_early_stop => some ``Hydro.Project.Node.kfold_early_stop
  | ``HydroSem.kget_max_key => some ``Hydro.Project.Node.kget_max_key
  | ``HydroSem.weaken_retries => some ``Hydro.Project.Node.weaken_retries
  | ``HydroSem.union => some ``Hydro.Project.Node.union
  | ``HydroSem.assume_ordering => some ``Hydro.Project.Node.assume_ordering
  | ``HydroSem.fold => some ``Hydro.Project.Node.fold
  | ``HydroSem.fold_monotone => some ``Hydro.Project.Node.fold_monotone
  | ``HydroSem.snapshot => some ``Hydro.Project.Node.snapshot
  | ``HydroSem.batch => some ``Hydro.Project.Node.batch
  | ``HydroSem.source_interval => some ``Hydro.Project.Node.source_interval
  | ``HydroSem.firstTick => some ``Hydro.Project.Node.firstTick
  | ``HydroSem.mapTick => some ``Hydro.Project.Node.mapTick
  | ``HydroSem.zipTick => some ``Hydro.Project.Node.zipTick
  | ``HydroSem.defer_tick => some ``Hydro.Project.Node.defer_tick
  | ``HydroSem.allTicks => some ``Hydro.Project.Node.allTicks
  | ``HydroSem.flattenOrdered => some ``Hydro.Project.Node.flattenOrdered
  | ``HydroSem.flattenUnordered => some ``Hydro.Project.Node.flattenUnordered
  | ``HydroSem.tinto_keyed => some ``Hydro.Project.Node.tinto_keyed
  | ``HydroSem.tassume_ordering => some ``Hydro.Project.Node.tassume_ordering
  | ``HydroSem.reduce_watermark => some ``Hydro.Project.Node.reduce_watermark
  | ``HydroSem.kentries => some ``Hydro.Project.Node.kentries
  | ``HydroSem.kmap => some ``Hydro.Project.Node.kmap
  | ``HydroSem.kallTicks => some ``Hydro.Project.Node.kallTicks
  | ``HydroSem.kfold => some ``Hydro.Project.Node.kfold
  | ``HydroSem.ksnapshot => some ``Hydro.Project.Node.ksnapshot
  | ``HydroSem.tfold => some ``Hydro.Project.Node.tfold
  | ``HydroSem.latest => some ``Hydro.Project.Node.latest
  | ``HydroSem.latest_monotone => some ``Hydro.Project.Node.latest_monotone
  | ``HydroSem.time => some ``Hydro.Project.Node.time
  | ``HydroSem.snapshot_bounded => some ``Hydro.Project.Node.snapshot_bounded
  | ``HydroSem.tick_scan => some ``Hydro.Project.Node.tick_scan
  | _ => none

/-- The `Kind` of a carrier type `@HydroSem.<Family> L mem H args…`
(`none` for a non-carrier type). -/
def kindOfType (ty : Expr) : MetaM (Option Expr) := do
  let ty ← whnfR ty
  let .const n _ := ty.getAppFn | return none
  let args := ty.getAppArgs
  unless args.size ≥ 3 do return none
  let L := args[0]!; let mem := args[1]!
  let rest := args.extract 3 args.size
  let mk (ctor : Name) : Expr := mkAppN (mkConst ctor) (#[L, mem] ++ rest)
  match n with
  | ``HydroSem.Stream => return some (mk ``Hydro.Project.Kind.stream)
  | ``HydroSem.KeyedStream => return some (mk ``Hydro.Project.Kind.keyed)
  | ``HydroSem.Singleton => return some (mk ``Hydro.Project.Kind.sing)
  | ``HydroSem.KeyedSingleton => return some (mk ``Hydro.Project.Kind.ksing)
  | ``HydroSem.Atomic => return some (mk ``Hydro.Project.Kind.atomic)
  | ``HydroSem.AtomicStream => return some (mk ``Hydro.Project.Kind.atomicStream)
  | ``HydroSem.AtomicKeyedStream => return some (mk ``Hydro.Project.Kind.atomicKeyed)
  | ``HydroSem.AtomicKeyedSingleton => return some (mk ``Hydro.Project.Kind.atomicKSing)
  | _ => return none

/-- A `KnotKind` from a knot wire's carrier type. -/
def knotKindOfType (ty : Expr) : MetaM Expr := do
  let ty ← whnfR ty
  let .const n _ := ty.getAppFn | throwError "project: knot type is not a carrier: {ty}"
  let args := ty.getAppArgs
  let L := args[0]!; let mem := args[1]!
  let rest := args.extract 3 args.size
  match n with
  | ``HydroSem.Stream => return mkAppN (mkConst ``Hydro.Project.KnotKind.stream) (#[L, mem] ++ rest)
  | ``HydroSem.Atomic => return mkAppN (mkConst ``Hydro.Project.KnotKind.atomic) (#[L, mem] ++ rest)
  | _ => throwError "project: a knot wire must be a Stream or an Atomic: {ty}"

/-- Is this type a carrier (wire) type? -/
def isWireType (ty : Expr) : MetaM Bool := return (← kindOfType ty).isSome

/-- The flattened components of a (right-nested) product type of wires:
their kinds, or `none` if some component is not a wire. -/
partial def kindsOfTupleType (ty : Expr) : MetaM (Option (Array Expr)) := do
  let ty ← whnfR ty
  if ty.isAppOfArity ``Prod 2 then
    let some a ← kindsOfTupleType ty.appFn!.appArg! | return none
    let some b ← kindsOfTupleType ty.appArg! | return none
    return some (a ++ b)
  match ← kindOfType ty with
  | some k => return some #[k]
  | none => return none

/-- Project a right-nested tuple expression's component `i` of `k`
(structural when the tuple is a literal). -/
partial def tupleComponent (e : Expr) (i k : Nat) : MetaM Expr := do
  if k == 1 then return e
  let e' := e.consumeMData
  if e'.isAppOfArity ``Prod.mk 4 then
    if i == 0 then return e'.getAppArgs[2]!
    else tupleComponent e'.getAppArgs[3]! (i - 1) (k - 1)
  else
    if i == 0 then mkAppM ``Prod.fst #[e]
    else tupleComponent (← mkAppM ``Prod.snd #[e]) (i - 1) (k - 1)

/-! ## The walker state -/

/-- How a wire-ish fvar is addressed in the graph. -/
inductive Bind where
  /-- A single wire: the context depth at which it was pushed (its
  position among the pushed kinds, counted from the bottom). -/
  | wire (pos : Nat)
  /-- A tuple of wires: each component's position. -/
  | tuple (poss : Array Nat)
  deriving Inhabited

/-- A node's schedule component, for `M.recordOf` (D77): where in the module's
schedule bundle the cursors came from. -/
inductive SComp where
  | unit
  /-- A network node: its `TransportDec` argument in the program term. -/
  | leaf (d : Expr)
  /-- A callee: its full graph arguments (its schedule bundle among them). -/
  | call (C : Name) (dagArgs : Array Expr)
  /-- A knot: the body's components. -/
  | nested (cs : Array SComp)
  deriving Inhabited

/-- An emitted graph entry. -/
inductive Entry where
  /-- A node binding `κs` kinds, emitted when the context (bottom-first)
  was `ctxAt`; `sched` is its `Node.Sched` component (D77). -/
  | node (n : Expr) (κs : Array Expr) (ctxAt : Array Expr) (sched : Expr) (comp : SComp)
  /-- A data `let` the later entries may mention. -/
  | dataLet (fv : FVarId) (name : Name) (ty val : Expr)
  deriving Inhabited

structure WState where
  /-- The context, bottom-first (position `i` ↦ kind). -/
  kinds : Array Expr := #[]
  binds : List (FVarId × Bind) := []
  entries : Array Entry := #[]
  /-- The body module of the current `fix` group and the closed knot
  fvars (in knot order): the body's re-application at those fvars reads
  the `fixN`'s outputs. -/
  knotBody : Option (Name × Array FVarId × Nat) := none  -- (body module, knot fvars, base of the fixN node)

abbrev WalkM := StateRefT WState TermElabM

def mkKindList (L mem : Expr) (ks : Array Expr) : Expr :=
  ks.foldr (fun k acc => mkApp3 (mkConst ``List.cons [Level.one]) (mkApp2 (mkConst ``Hydro.Project.Kind) L mem) k acc)
    (mkApp (mkConst ``List.nil [Level.one]) (mkApp2 (mkConst ``Hydro.Project.Kind) L mem))

/-- The current context as a `List Kind` (top-first). -/
def ctxExpr (L mem : Expr) (kinds : Array Expr) : Expr := mkKindList L mem kinds.reverse

/-- The de Bruijn variable for position `pos` in a context of `kinds`
(bottom-first). -/
def mkVarAt (L mem : Expr) (kinds : Array Expr) (pos : Nat) : MetaM Expr := do
  let n := kinds.size
  unless pos < n do throwError "project: variable position out of range"
  let idx := n - 1 - pos
  let κ := kinds[pos]!
  -- walk from the top: context top-first is kinds.reverse
  let top := kinds.reverse
  let rec go (i : Nat) (ctx : List Expr) : MetaM Expr := do
    match ctx with
    | [] => throwError "project: empty context"
    | κ' :: rest =>
      let restE := mkKindList L mem rest.toArray
      if i == 0 then
        return mkApp4 (mkConst ``Hydro.Project.Var.zero) L mem restE κ
      else
        let inner ← go (i - 1) rest
        return mkApp6 (mkConst ``Hydro.Project.Var.succ) L mem restE κ κ' inner
  go idx top.toList

/-- Emit a node binding the kinds `κs` (top-first, as `letN` types
them); the context (bottom-first) grows by their reverse. -/
def emitNode (n : Expr) (κs : Array Expr) (sched : Expr := mkConst ``Unit.unit) (comp : SComp := .unit) :
    WalkM Unit := do
  let s ← get
  set { s with entries := s.entries.push (.node n κs s.kinds sched comp), kinds := s.kinds ++ κs.reverse }

/-- The context position of output `i` of a node binding `k` kinds,
emitted when the context had `base` entries. -/
def componentPos (base k i : Nat) : Nat := base + k - 1 - i

/-! ## Converting terms

Wire arguments are first resolved to context *positions* (ANF: an
operator application in argument position is emitted as an anonymous
node, growing the context); the variables are built afterwards, at the
node's final context. -/

/-- `Prod.fst`/`Prod.snd` chains: the base and the path (outermost last;
`true` = fst). -/
partial def projPath (e : Expr) : Option (Expr × List Bool) :=
  let e := e.consumeMData
  if e.isAppOfArity ``Prod.fst 3 then
    match projPath e.appArg! with
    | some (b, p) => some (b, p ++ [true])
    | none => some (e.appArg!, [true])
  else if e.isAppOfArity ``Prod.snd 3 then
    match projPath e.appArg! with
    | some (b, p) => some (b, p ++ [false])
    | none => some (e.appArg!, [false])
  else none

/-- The component index selected by a `.1`/`.2` path on a right-nested
`k`-tuple. -/
partial def pathIndex (path : List Bool) (k : Nat) : WalkM Nat := do
  match path, k with
  | [], 1 => return 0
  | [], _ => throwError "project: tuple used where a single wire is expected"
  | true :: _, 1 => throwError "project: projection of a non-tuple"
  | true :: _, _ => return 0
  | false :: rest, k =>
    if k ≤ 1 then throwError "project: projection path too long"
    return 1 + (← pathIndex rest (k - 1))

/-- The kinds of a tick shape, flattened. -/
partial def flattenShapeKinds (L mem ℓ : Expr) (sh : Expr) : WalkM (Array Expr) := do
  let sh ← whnfR sh
  if sh.isAppOfArity ``TickShape.pair 2 then
    return (← flattenShapeKinds L mem ℓ sh.appFn!.appArg!) ++ (← flattenShapeKinds L mem ℓ sh.appArg!)
  let args := sh.getAppArgs
  if sh.isAppOfArity ``TickShape.sing 1 then
    return #[mkApp4 (mkConst ``Hydro.Project.Kind.atomic) L mem ℓ args[0]!]
  if sh.isAppOfArity ``TickShape.stream 4 then
    return #[mkAppN (mkConst ``Hydro.Project.Kind.atomicStream) (#[L, mem, ℓ] ++ args)]
  if sh.isAppOfArity ``TickShape.ksing 4 then
    return #[mkAppN (mkConst ``Hydro.Project.Kind.atomicKSing) (#[L, mem, ℓ] ++ args)]
  if sh.isAppOfArity ``TickShape.kstream 6 then
    return #[mkAppN (mkConst ``Hydro.Project.Kind.atomicKeyed) (#[L, mem, ℓ] ++ args)]
  throwError "project: tick shape is not a literal: {sh}"

/-- A tree of positions following a tick shape. -/
inductive PosTree where
  | leaf (pos : Nat)
  | pair (a b : PosTree)

/-- Build `VarsOf Γ ℓ sh` from a position tree at the final context. -/
partial def buildVarsOf (L mem : Expr) (kinds : Array Expr) (Γ ℓ : Expr) (sh : Expr) :
    PosTree → WalkM Expr
  | .leaf pos => mkVarAt L mem kinds pos
  | .pair pa pb => do
    let sh ← whnfR sh
    unless sh.isAppOfArity ``TickShape.pair 2 do throwError "project: shape/tuple mismatch"
    let a := sh.appFn!.appArg!; let b := sh.appArg!
    let va ← buildVarsOf L mem kinds Γ ℓ a pa
    let vb ← buildVarsOf L mem kinds Γ ℓ b pb
    let tya := mkAppN (mkConst ``Hydro.Project.VarsOf) #[L, mem, Γ, ℓ, a]
    let tyb := mkAppN (mkConst ``Hydro.Project.VarsOf) #[L, mem, Γ, ℓ, b]
    return mkApp4 (mkConst ``Prod.mk [Level.one, Level.one]) tya tyb va vb

/-- The substitution `HList (Var Γ) Γ_N` from the converted wire variables. -/
partial def mkSubst (L mem ΓN Γ : Expr) (vars : Array Expr) : WalkM Expr := do
  let rec go (ks : Expr) (i : Nat) : WalkM Expr := do
    let ks ← whnfR ks
    if ks.isAppOfArity ``List.nil 1 then
      return mkApp3 (mkConst ``Hydro.Project.HList.nil [Level.one]) L mem (mkApp3 (mkConst ``Hydro.Project.Var) L mem Γ)
    if ks.isAppOfArity ``List.cons 3 then
      let κ := ks.getAppArgs[1]!; let rest := ks.getAppArgs[2]!
      unless i < vars.size do throwError "project: callee wire count mismatch"
      let tail ← go rest (i + 1)
      return mkAppN (mkConst ``Hydro.Project.HList.cons [Level.one])
        #[L, mem, mkApp3 (mkConst ``Hydro.Project.Var) L mem Γ, κ, rest, vars[i]!, tail]
    throwError "project: callee context is not a list literal: {ks}"
  go ΓN 0


/-- Split a full callee application's arguments into the graph's binders
(`H` ↦ `pacing`, every non-wire argument) and its wire inputs. -/
def splitCalleeArgs (pacing : Expr) (c : Name) (args : Array Expr) : MetaM (Array Expr × Array Expr) := do
  let info ← getConstInfo c
  forallTelescope info.type fun xs _ => do
    let mut dagArgs : Array Expr := #[]
    let mut wireArgs : Array Expr := #[]
    for i in [0:xs.size] do
      unless i < args.size do throwError "project: partial callee application of {c}"
      let xty ← inferType xs[i]!
      if xty.getAppFn.isConstOf ``HydroSem then
        dagArgs := dagArgs.push pacing
      else if ← isWireType xty then
        wireArgs := wireArgs.push args[i]!
      else
        dagArgs := dagArgs.push args[i]!
    pure (dagArgs, wireArgs)

/-- A decision type that is execution data at the machine: delivery cursors and
timing scripts (`IntervalDec`/`ClockDec`). -/
def isSchedDecTy (ty : Expr) : Bool :=
  ty.isAppOf ``HydroSem.TransportDec || ty.isAppOf ``HydroSem.IntervalDec ||
  ty.isAppOf ``HydroSem.ClockDec

/-- The graph arguments a callee's `C.dag` keeps (its `dagKeep` mask; D77). -/
def keptArgs (c : Name) (dagArgs : Array Expr) : MetaM (Array Expr) := do
  let some gi := getGenInfo (← getEnv) c | throwError "project: callee {c} unregistered"
  let keep := gi.dagKeep
  unless keep.size == dagArgs.size do
    throwError "project: callee {c}: {dagArgs.size} graph arguments, keep mask of {keep.size}"
  return (dagArgs.zip keep).filterMap fun (a, k) => if k then some a else none

/-- The graph arguments a callee's `C.recordOf` takes (its `dagRecKeep` mask). -/
def recArgs (c : Name) (dagArgs : Array Expr) : MetaM (Array Expr) := do
  let some gi := getGenInfo (← getEnv) c | throwError "project: callee {c} unregistered"
  let keep := gi.dagRecKeep
  unless keep.size == dagArgs.size do
    throwError "project: callee {c}: {dagArgs.size} graph arguments, record mask of {keep.size}"
  return (dagArgs.zip keep).filterMap fun (a, k) => if k then some a else none

/-- `C.dag` at the kept arguments and `C.schedOf` at all of them. -/
def calleeGraph (c : Name) (dagArgs : Array Expr) : MetaM (Expr × Expr) := do
  let N := mkAppN (mkConst (c ++ `dag)) (← keptArgs c dagArgs)
  let SN := mkAppN (mkConst (c ++ `schedOf)) dagArgs
  return (N, SN)

/-- The elements of a `List` literal. -/
partial def listLitElems (e : Expr) : Option (Array Expr) :=
  let e := e.consumeMData
  if e.isAppOfArity ``List.nil 1 then some #[]
  else if e.isAppOfArity ``List.cons 3 then
    (listLitElems e.appArg!).map (#[e.appFn!.appArg!] ++ ·)
  else none

/-- The input context and output kinds of a callee graph `N`. -/
def dagShape (N : Expr) : MetaM (Expr × Expr × Array Expr × Array Expr) := do
  let Nty ← whnfR (← inferType N)
  unless Nty.isAppOfArity ``Hydro.Project.Dag 4 do throwError "project: not a Dag: {Nty}"
  let ΓN := Nty.getAppArgs[2]!; let outsN := Nty.getAppArgs[3]!
  let some inKs := listLitElems ΓN | throwError "project: graph context is not a literal: {ΓN}"
  let some outKs := listLitElems outsN | throwError "project: graph outputs are not a literal: {outsN}"
  pure (ΓN, outsN, inKs, outKs)

mutual

/-- A wire-typed term to a context position (ANF: an operator
application becomes an anonymous node). -/
partial def toPos (L mem S pacing : Expr) (e : Expr) : WalkM Nat := do
  let e := e.consumeMData.headBeta
  if let .fvar f := e then
    match (← get).binds.lookup f with
    | some (.wire pos) => return pos
    | some (.tuple _) => throwError "project: tuple-bound wire used as a wire: {e}"
    | none => throwError "project: unbound wire fvar {e}"
  if let some (base, path) := projPath e then
    return ← resolveProj L mem S pacing base path
  let κs ← toNode L mem S pacing e
  unless κs.size == 1 do throwError "project: a multi-output node in wire position: {e}"
  return (← get).kinds.size - 1

/-- Resolve `base.<path>` to a context position. -/
partial def resolveProj (L mem S pacing : Expr) (base : Expr) (path : List Bool) : WalkM Nat := do
  let st ← get
  if let .fvar f := base.consumeMData then
    match st.binds.lookup f with
    | some (.tuple poss) =>
      let i ← pathIndex path poss.size
      return poss[i]!
    | some (.wire _) => throwError "project: projection of a single wire {base}"
    | none => throwError "project: projection of an unbound fvar {base}"
  -- the knot body re-applied at the closed knots (the Bekić continuation)
  if let some (bodyName, knotFvs, basePos) := st.knotBody then
    if base.getAppFn.isConstOf bodyName then
      let args := base.getAppArgs
      if args.size ≥ knotFvs.size then
        let tail := args.extract (args.size - knotFvs.size) args.size
        if (tail.zip knotFvs).all (fun (a, f) => a.consumeMData == .fvar f) then
          let some outs ← kindsOfTupleType (← inferType base)
            | throwError "project: knot body application is not a wire tuple"
          let i ← pathIndex path outs.size
          if i < knotFvs.size then
            throwError "project: the knot body's own next-value component is referenced after `complete`"
          return componentPos basePos outs.size i
  -- a callee application projected without a let: ANF the whole call
  let κs ← toNode L mem S pacing base
  let i ← pathIndex path κs.size
  return componentPos ((← get).kinds.size - κs.size) κs.size i

/-- A shaped tick input (a right-nested tuple of wires following the
shape) to positions. -/
partial def toPosTree (L mem S pacing : Expr) (sh : Expr) (e : Expr) : WalkM PosTree := do
  let sh ← whnfR sh
  if sh.isAppOfArity ``TickShape.pair 2 then
    let a := sh.appFn!.appArg!; let b := sh.appArg!
    let e := e.consumeMData.headBeta
    let (ea, eb) ← if e.isAppOfArity ``Prod.mk 4 then
        pure (e.getAppArgs[2]!, e.getAppArgs[3]!)
      else pure (← mkAppM ``Prod.fst #[e], ← mkAppM ``Prod.snd #[e])
    let pa ← toPosTree L mem S pacing a ea
    let pb ← toPosTree L mem S pacing b eb
    return .pair pa pb
  else
    return .leaf (← toPos L mem S pacing e)

/-- An operator / callee application to a node; emits it and returns the
bound kinds. -/
partial def toNode (L mem S pacing : Expr) (e : Expr) : WalkM (Array Expr) := do
  let e := e.consumeMData.headBeta
  let f := e.getAppFn
  let args := e.getAppArgs
  let .const c _ := f | throwError "project: unrecognized wire term {e}"
  let env ← getEnv
  -- a `HydroSem` operator
  if let some ctor := nodeCtorOf c then
    unless args.size ≥ 3 && args[2]! == S do
      throwError "project: operator {c} not at the machine instance: {e}"
    let rest := args.extract 3 args.size
    if c == ``HydroSem.tick_scan then
      -- {ℓ} sts ins outs x g seed
      let ℓ := rest[0]!; let sts := rest[1]!; let ins := rest[2]!; let outs := rest[3]!
      let tree ← toPosTree L mem S pacing ins rest[4]!
      let kinds := (← get).kinds
      let Γ := ctxExpr L mem kinds
      let x ← buildVarsOf L mem kinds Γ ℓ ins tree
      let n := mkAppN (mkConst ctor) #[L, mem, Γ, ℓ, sts, ins, outs, x, rest[5]!, rest[6]!]
      let κs ← flattenShapeKinds L mem ℓ outs
      emitNode n κs
      return κs
    -- resolve wire arguments to positions first
    let mut poss : Array (Option Nat) := #[]
    for a in rest do
      if ← isWireType (← inferType a) then
        poss := poss.push (some (← toPos L mem S pacing a))
      else
        poss := poss.push none
    let kinds := (← get).kinds
    let Γ := ctxExpr L mem kinds
    let mut out : Array Expr := #[L, mem, Γ]
    -- a network node's delivery cursors and a timer's verdicts are execution data,
    -- not program data (D77): they leave the node and become its `Node.Sched`
    -- component; `Unit` decisions are `()` (the bundle is not program data either)
    let mut sched : Expr := mkConst ``Unit.unit
    let mut comp : SComp := .unit
    for i in [0:rest.size] do
      match poss[i]! with
      | some pos => out := out.push (← mkVarAt L mem kinds pos)
      | none =>
        let ty ← whnfR (← inferType rest[i]!)
        if isSchedDecTy ty then
          sched := rest[i]!; comp := .leaf rest[i]!
        else
          let tyD ← whnf ty
          if tyD.isConstOf ``Unit || tyD.isConstOf ``PUnit then
            out := out.push (mkConst ``Unit.unit)
          else
            out := out.push rest[i]!
    let n := mkAppN (mkConst ctor) out
    let some κ ← kindOfType (← inferType e) | throwError "project: operator result is not a wire: {e}"
    emitNode n #[κ] sched comp
    return #[κ]
  if c == ``HydroSem.fix || c == ``HydroSem.fixTick || c == ``HydroSem.fix_stream || c == ``HydroSem.fix_tick then
    throwError "project: a raw knot in the body ({c}); knots must be the elaborator's hoisted constants"
  -- a callee module
  if let some gi := getGenInfo env c then
    if gi.isKnot then throwError "project: knot constant {c} outside a knot group"
    let dagName := c ++ `dag
    unless env.contains dagName do throwError "project: callee {c} has no `{dagName}` (phase order)"
    let (dagArgs, wireArgs) ← splitCalleeArgs pacing c args
    let mut poss : Array Nat := #[]
    for w in wireArgs do
      poss := poss.push (← toPos L mem S pacing w)
    let kinds := (← get).kinds
    let Γ := ctxExpr L mem kinds
    let mut vars : Array Expr := #[]
    for pos in poss do
      vars := vars.push (← mkVarAt L mem kinds pos)
    let (N, SN) ← calleeGraph c dagArgs
    let (ΓN, outsN, _, _) ← dagShape N
    let subst ← mkSubst L mem ΓN Γ vars
    let n := mkAppN (mkConst ``Hydro.Project.Node.call) #[L, mem, Γ, ΓN, outsN, N, subst]
    let some κs ← kindsOfTupleType (← inferType e)
      | throwError "project: callee result is not a wire tuple: {e}"
    emitNode n κs SN (.call c dagArgs)
    return κs
  throwError "project: unrecognized invocation head `{c}` in {e}"

end

/-! ## The walk -/

/-- Build the graph term from the emitted entries and the output positions. -/
def assemble (L mem : Expr) (st : WState) (outPoss : Array Nat) : TermElabM Expr := do
  let kinds := st.kinds
  let Γfin := ctxExpr L mem kinds
  let outKinds := outPoss.map (fun p => kinds[p]!)
  let outsE := mkKindList L mem outKinds
  -- the returned variables
  let varTy := mkApp3 (mkConst ``Hydro.Project.Var) L mem Γfin
  let mut xs := mkApp3 (mkConst ``Hydro.Project.HList.nil [Level.one]) L mem varTy
  for i in [0:outPoss.size] do
    let j := outPoss.size - 1 - i
    let v ← mkVarAt L mem kinds outPoss[j]!
    let restKs := mkKindList L mem (outKinds.extract (j + 1) outKinds.size)
    xs := mkAppN (mkConst ``Hydro.Project.HList.cons [Level.one]) #[L, mem, varTy, outKinds[j]!, restKs, v, xs]
  let mut acc := mkAppN (mkConst ``Hydro.Project.Dag.ret) #[L, mem, Γfin, outsE, xs]
  for i in [0:st.entries.size] do
    match st.entries[st.entries.size - 1 - i]! with
    | .node n κs ctxAt _ _ =>
      acc := mkAppN (mkConst ``Hydro.Project.Dag.letN) #[L, mem, ctxExpr L mem ctxAt, mkKindList L mem κs, outsE, n, acc]
    | .dataLet fv _ _ _ =>
      acc ← mkLetFVars #[mkFVar fv] acc
  pure acc

/-- The graph's schedule (`Dag.Sched`, D77): the nodes' components as a right-nested
tuple along the chain, under the same data `let`s. -/
def assembleSched (st : WState) : TermElabM Expr := do
  let mut acc : Expr := mkConst ``Unit.unit
  for i in [0:st.entries.size] do
    match st.entries[st.entries.size - 1 - i]! with
    | .node _ _ _ sched _ => acc ← mkAppM ``Prod.mk #[sched, acc]
    | .dataLet fv _ _ _ => acc ← mkLetFVars #[mkFVar fv] acc
  pure acc

/-- The node components of a walk, in chain order. -/
def compsOf (st : WState) : Array SComp :=
  st.entries.filterMap fun | .node _ _ _ _ c => some c | _ => none

/-- One `let` of the chain. -/
structure LetItem where
  fv : FVarId
  name : Name
  ty : Expr
  val : Expr
  deriving Inhabited

/-- Peel the `let`-telescope of a body into fvars, then run `k` on the
items and the final expression. -/
partial def withLets (e : Expr) (acc : Array LetItem) (k : Array LetItem → Expr → TermElabM α) :
    TermElabM α := do
  let e := e.consumeMData
  match e with
  | .letE n ty v b _ =>
    withLetDecl n ty v fun x => withLets (b.instantiate1 x) (acc.push ⟨x.fvarId!, n, ty, v⟩) k
  | _ =>
    if e.isAppOf ``letFun && e.getAppNumArgs ≥ 4 then
      let args := e.getAppArgs
      let ty := args[0]!; let v := args[2]!; let f := args[3]!
      if f.isLambda then
        let n := f.bindingName!
        withLetDecl n ty v fun x =>
          withLets (mkAppN (f.bindingBody!.instantiate1 x) (args.extract 4 args.size)) (acc.push ⟨x.fvarId!, n, ty, v⟩) k
      else k acc e
    else k acc e

/-- Is `c` a hoisted knot constant of the current module? -/
def isKnotOf (env : Environment) (M : Name) (c : Name) : Bool :=
  match getGenInfo env c with
  | some gi => gi.isKnot && c.getPrefix == M
  | none => false

/-- Find the first application of `B` whose trailing arguments are the knot fvars. -/
partial def findBodyApp (B : Name) (knotFvs : Array FVarId) (e : Expr) : Option Expr :=
  let e := e.consumeMData
  if e.getAppFn.isConstOf B then
    let args := e.getAppArgs
    if args.size ≥ knotFvs.size then
      let tail := args.extract (args.size - knotFvs.size) args.size
      if (tail.zip knotFvs).all (fun (a, f) => a.consumeMData == .fvar f) then some e
      else none
    else none
  else
    match e with
    | .app f a => (findBodyApp B knotFvs f).orElse fun _ => findBodyApp B knotFvs a
    | .lam _ _ b _ => findBodyApp B knotFvs b
    | .letE _ _ v b _ => (findBodyApp B knotFvs v).orElse fun _ => findBodyApp B knotFvs b
    | .mdata _ b => findBodyApp B knotFvs b
    | .proj _ _ b => findBodyApp B knotFvs b
    | _ => none

/-- Process the chain items from index `i`; returns the output positions. -/
partial def walkItems (L mem S pacing : Expr) (M : Name) (items : Array LetItem) (i : Nat) (final : Expr) :
    WalkM (Array Nat) := do
  if h : i < items.size then
    let it := items[i]
    let env ← getEnv
    let v := it.val.consumeMData.headBeta
    let headC? : Option Name := match v.getAppFn with | .const c _ => some c | _ => none
    -- a knot group
    if let some c := headC? then
      if isKnotOf env M c then
        let mut j := i
        while h' : j < items.size do
          match (items[j]).val.consumeMData.headBeta.getAppFn with
          | .const c' _ => if isKnotOf env M c' then j := j + 1 else break
          | _ => break
        let group := items.extract i j
        let knotFvs := group.map (·.fv)
        let n := group.size
        let some gi := getGenInfo env c | throwError "project: knot {c} unregistered"
        let some B := gi.callees.find? (fun d => !(isKnotOf env M d))
          | throwError "project: knot {c} has no body module"
        let bodyDag := B ++ `dag
        unless env.contains bodyDag do throwError "project: knot body {B} has no `{bodyDag}`"
        let rest := mkAppN (mkConst ``Unit.unit) ((items.extract j items.size).map (·.val) |>.push final)
        let some bapp := findBodyApp B knotFvs rest
          | throwError "project: the knot body {B} is never applied at the closed knots"
        let mut ks : Array Expr := #[]
        for g in group do ks := ks.push (← knotKindOfType g.ty)
        let kkTy := mkApp2 (mkConst ``Hydro.Project.KnotKind) L mem
        let ksList := ks.foldr (fun k acc => mkApp3 (mkConst ``List.cons [Level.one]) kkTy k acc)
          (mkApp (mkConst ``List.nil [Level.one]) kkTy)
        let mut knotKinds : Array Expr := #[]
        for g in group do
          let some kk ← kindOfType g.ty | throwError "project: knot type is not a wire"
          knotKinds := knotKinds.push kk
        -- the body graph: a sub-walk in the context extended by the knots
        let outer ← get
        let base := outer.kinds.size
        let mut sub : WState :=
          { kinds := outer.kinds ++ knotKinds.reverse, binds := outer.binds, entries := #[], knotBody := none }
        for jj in [0:n] do
          sub := { sub with binds := (knotFvs[jj]!, Bind.wire (componentPos base n jj)) :: sub.binds }
        let subRun : WalkM (Array Expr × Array Nat) := do
          let κs ← toNode L mem S pacing bapp
          let b1 := (← get).kinds.size - κs.size
          let outPoss := (Array.range κs.size).map (fun kk => componentPos b1 κs.size kk)
          pure (κs, outPoss)
        let ((κsB, outPoss), sub') ← subRun.run sub
        let bodyE ← assemble L mem sub' outPoss
        let schedB ← assembleSched sub'
        unless κsB.size ≥ n do throwError "project: knot body {B} returns fewer outputs than knots"
        let exposed := κsB.extract n κsB.size
        let Γ := ctxExpr L mem outer.kinds
        let node := mkAppN (mkConst ``Hydro.Project.Node.fixN) #[L, mem, Γ, ksList, mkKindList L mem exposed, bodyE]
        emitNode node κsB schedB (.nested (compsOf sub'))
        for jj in [0:n] do
          modify fun s => { s with binds := (knotFvs[jj]!, Bind.wire (componentPos base κsB.size jj)) :: s.binds }
        modify fun s => { s with knotBody := some (B, knotFvs, base) }
        return ← walkItems L mem S pacing M items j final
    -- a wire-typed let
    if ← isWireType it.ty then
      let pos ← toPos L mem S pacing v
      modify fun s => { s with binds := (it.fv, Bind.wire pos) :: s.binds }
      return ← walkItems L mem S pacing M items (i + 1) final
    -- a tuple of wires
    if let some κs ← kindsOfTupleType it.ty then
      let k := κs.size
      let isCallee := match headC? with
        | some c => (getGenInfo env c).isSome && !(isKnotOf env M c) && !(v.isAppOfArity ``Prod.mk 4)
        | none => false
      if isCallee then
        let κs' ← toNode L mem S pacing v
        let base := (← get).kinds.size - κs'.size
        let poss := (Array.range κs'.size).map (fun c => componentPos base κs'.size c)
        modify fun s => { s with binds := (it.fv, Bind.tuple poss) :: s.binds }
      else
        let mut poss : Array Nat := #[]
        for c in [0:k] do
          poss := poss.push (← toPos L mem S pacing (← tupleComponent v c k))
        modify fun s => { s with binds := (it.fv, Bind.tuple poss) :: s.binds }
      return ← walkItems L mem S pacing M items (i + 1) final
    -- data
    modify fun s => { s with entries := s.entries.push (.dataLet it.fv it.name it.ty it.val) }
    return ← walkItems L mem S pacing M items (i + 1) final
  else
    let fty ← inferType final
    let some κs ← kindsOfTupleType fty | throwError "project: module output is not a wire tuple: {fty}"
    let mut poss : Array Nat := #[]
    for c in [0:κs.size] do
      poss := poss.push (← toPos L mem S pacing (← tupleComponent final c κs.size))
    return poss

/-! ## The phase -/

/-- Right-nested `HList.cons` chain of `Kind.sched` values. -/
def mkEnvList (L mem : Expr) (ks : Array Expr) (vals : Array Expr) : Expr := Id.run do
  let f := mkApp2 (mkConst ``Hydro.Project.Kind.sched) L mem
  let mut acc := mkApp3 (mkConst ``Hydro.Project.HList.nil [Level.zero]) L mem f
  for i in [0:ks.size] do
    let j := ks.size - 1 - i
    let restKs := mkKindList L mem (ks.extract (j + 1) ks.size)
    acc := mkAppN (mkConst ``Hydro.Project.HList.cons [Level.zero]) #[L, mem, f, ks[j]!, restKs, vals[j]!, acc]
  acc

/-! ## The opened chain

The denotation lemma is proved in two hops. The kernel judges
`Dag.denote M.dag env = T` where `T` is the module's own chain with every
callee application `C S args` replaced by the components of
`Dag.denote (C.dag …) ⟨args⟩` and every knot group written as the Bekić
cascade over the body graph — so neither side ever unfolds a callee
graph (same-head applications compare argument-wise). The second hop
rewrites each `Dag.denote (C.dag …) ⟨…⟩` with `C.dag_denote` and closes
by `rfl` against the module, where only hoisted knot constants and
`Prod` eta remain. -/

/-- `Kind.sched κ`. -/
def schedTy (L mem κ : Expr) : Expr := mkApp3 (mkConst ``Hydro.Project.Kind.sched) L mem κ

/-- `HList Kind.sched ks`. -/
def hlistTy (L mem : Expr) (ks : Expr) : Expr :=
  mkApp4 (mkConst ``Hydro.Project.HList [Level.zero]) L mem (mkApp2 (mkConst ``Hydro.Project.Kind.sched) L mem) ks

/-- Component `i` of an `HList Kind.sched ks` value, by `Prod` projections
with the carrier types explicit. -/
def hlistComponent (L mem : Expr) (ks : Array Expr) (D : Expr) (i : Nat) : Expr := Id.run do
  let mut cur := D
  for j in [0:i] do
    cur := mkApp3 (mkConst ``Prod.snd [Level.zero, Level.zero]) (schedTy L mem ks[j]!)
      (hlistTy L mem (mkKindList L mem (ks.extract (j + 1) ks.size))) cur
  mkApp3 (mkConst ``Prod.fst [Level.zero, Level.zero]) (schedTy L mem ks[i]!)
    (hlistTy L mem (mkKindList L mem (ks.extract (i + 1) ks.size))) cur

/-- The right-nested product of carrier types `ks[from:]`. -/
def schedTupleTy (L mem : Expr) (ks : Array Expr) (from_ : Nat) : Expr :=
  if from_ + 1 ≥ ks.size then schedTy L mem ks[from_]!
  else mkApp2 (mkConst ``Prod [Level.zero, Level.zero]) (schedTy L mem ks[from_]!) (schedTupleTy L mem ks (from_ + 1))
termination_by ks.size - from_

/-- The right-nested tuple of the components (the shape a callee returns). -/
def schedTuple (L mem : Expr) (ks : Array Expr) (comps : Array Expr) (from_ : Nat := 0) : Expr :=
  if from_ + 1 ≥ ks.size then comps[from_]!
  else mkApp4 (mkConst ``Prod.mk [Level.zero, Level.zero]) (schedTy L mem ks[from_]!)
    (schedTupleTy L mem ks (from_ + 1)) comps[from_]! (schedTuple L mem ks comps (from_ + 1))
termination_by ks.size - from_

/-- The number of leading `∀` binders. -/
partial def forallArity (e : Expr) : Nat :=
  match e.consumeMData with
  | .forallE _ _ b _ => 1 + forallArity b
  | _ => 0

/-- Replace every full application of a (non-knot) callee module `C S args…`
by the component tuple of `Dag.denote pacing (C.dag …) ⟨wires⟩`. -/
def openCallees (L mem pacing : Expr) (M : Name) (e : Expr) : TermElabM Expr :=
  Meta.transform e (pre := fun e => do
    let env ← getEnv
    let .const c _ := e.getAppFn | return .continue
    let some gi := getGenInfo env c | return .continue
    if gi.isKnot || isKnotOf env M c then return .continue
    let dagName := c ++ `dag
    unless env.contains dagName do return .continue
    let info ← getConstInfo c
    unless e.getAppNumArgs == forallArity info.type do return .continue
    let (dagArgs, wireArgs) ← splitCalleeArgs pacing c e.getAppArgs
    let (N, SN) ← calleeGraph c dagArgs
    let (ΓN, outsN, inKs, outKs) ← dagShape N
    unless inKs.size == wireArgs.size do throwError "project: callee {c} wire arity mismatch"
    let D := mkAppN (mkConst ``Hydro.Project.Dag.denote) #[L, mem, pacing, ΓN, outsN, N, SN, mkEnvList L mem inKs wireArgs]
    let comps := (Array.range outKs.size).map (hlistComponent L mem outKs D)
    return .visit (schedTuple L mem outKs comps))

/-- `xs.get (Var m)` for `xs : HList Kind.sched ks` (context literal `ks`). -/
def hlistGetLit (L mem : Expr) (ks : Array Expr) (xs : Expr) (m : Nat) : MetaM Expr := do
  -- `mkVarAt` indexes bottom-first; `ks` here is top-first
  let v ← mkVarAt L mem ks.reverse (ks.size - 1 - m)
  return mkAppN (mkConst ``Hydro.Project.HList.get [Level.zero])
    #[L, mem, mkApp2 (mkConst ``Hydro.Project.Kind.sched) L mem, mkKindList L mem ks, ks[m]!, xs, v]

/-- The opened chain `T` of items `i…` and the final tuple. -/
partial def graphsTerm (L mem S pacing : Expr) (M : Name) (items : Array LetItem) (i : Nat) (final : Expr) :
    TermElabM Expr := do
  if h : i < items.size then
    let it := items[i]
    let env ← getEnv
    let v := it.val.consumeMData.headBeta
    if let .const c _ := v.getAppFn then
      if isKnotOf env M c then
        -- the knot group i..j, exactly as `walkItems` delimits it
        let mut j := i
        while h' : j < items.size do
          match (items[j]).val.consumeMData.headBeta.getAppFn with
          | .const c' _ => if isKnotOf env M c' then j := j + 1 else break
          | _ => break
        let group := items.extract i j
        let knotFvs := group.map (·.fv)
        let n := group.size
        let some gi := getGenInfo env c | throwError "project: knot {c} unregistered"
        let some B := gi.callees.find? (fun d => !(isKnotOf env M d))
          | throwError "project: knot {c} has no body module"
        let rest := mkAppN (mkConst ``Unit.unit) ((items.extract j items.size).map (·.val) |>.push final)
        let some bapp := findBodyApp B knotFvs rest
          | throwError "project: the knot body {B} is never applied at the closed knots"
        let mut ks : Array Expr := #[]
        let mut knotKinds : Array Expr := #[]
        for g in group do
          ks := ks.push (← knotKindOfType g.ty)
          let some kk ← kindOfType g.ty | throwError "project: knot type is not a wire"
          knotKinds := knotKinds.push kk
        let kkTy := mkApp2 (mkConst ``Hydro.Project.KnotKind) L mem
        let ksList := ks.foldr (fun k acc => mkApp3 (mkConst ``List.cons [Level.one]) kkTy k acc)
          (mkApp (mkConst ``List.nil [Level.one]) kkTy)
        let kindTy := mkApp2 (mkConst ``Hydro.Project.Kind) L mem
        let ksKinds := mkApp4 (mkConst ``List.map [Level.one, Level.one]) kkTy kindTy
          (mkApp2 (mkConst ``Hydro.Project.KnotKind.toKind) L mem) ksList
        let xsTy := hlistTy L mem ksKinds
        let (dagArgsB, wireArgsB) ← splitCalleeArgs pacing B bapp.getAppArgs
        let (NB, SB) ← calleeGraph B dagArgsB
        let (ΓB, outsB, inKsB, outKsB) ← dagShape NB
        unless inKsB.size == wireArgsB.size do throwError "project: knot body {B} wire arity mismatch"
        unless outKsB.size ≥ n do throwError "project: knot body {B} returns fewer outputs than knots"
        let exposed := mkKindList L mem (outKsB.extract n outKsB.size)
        let closedVal ← withLocalDeclD `xs xsTy fun xs => do
          let mut knotTerms : Array Expr := #[]
          for m in [0:n] do knotTerms := knotTerms.push (← hlistGetLit L mem knotKinds xs m)
          let wireArgs' := wireArgsB.map (fun a => a.replaceFVars (knotFvs.map mkFVar) knotTerms)
          let D := mkAppN (mkConst ``Hydro.Project.Dag.denote)
            #[L, mem, pacing, ΓB, outsB, NB, SB, mkEnvList L mem inKsB wireArgs']
          let takeE := mkAppN (mkConst ``Hydro.Project.HList.take [Level.zero])
            #[L, mem, mkApp2 (mkConst ``Hydro.Project.Kind.sched) L mem, ksKinds, exposed, D]
          mkLambdaFVars #[xs] takeE
        let closedVal := mkAppN (mkConst ``Hydro.Project.bekic) #[L, mem, pacing, ksList, closedVal]
        withLetDecl `closed xsTy closedVal fun closed => do
          let mut restT ← graphsTerm L mem S pacing M items j final
          for m' in [0:n] do
            let m := n - 1 - m'
            let g : LetItem := group[m]!
            let val ← hlistGetLit L mem knotKinds closed m
            restT := mkLet g.name g.ty val (restT.abstract #[mkFVar g.fv])
          mkLetFVars #[closed] restT
      else
        let restT ← graphsTerm L mem S pacing M items (i + 1) final
        let v' ← openCallees L mem pacing M it.val
        return mkLet it.name it.ty v' (restT.abstract #[mkFVar it.fv])
    else
      let restT ← graphsTerm L mem S pacing M items (i + 1) final
      let v' ← openCallees L mem pacing M it.val
      return mkLet it.name it.ty v' (restT.abstract #[mkFVar it.fv])
  else
    openCallees L mem pacing M final

/-- The binder telescope of a module value, `H` replaced by the machine
instance; `k` gets the graph's binders, the full argument list, the wire
inputs, `(L, mem, S, pacing)` and the body. -/
partial def teleModule (e : Expr) (dagBinders : Array Expr) (allArgs : Array Expr)
    (wires : Array (FVarId × Expr)) (LmemS? : Option (Expr × Expr × Expr × Expr))
    (k : Array Expr → Array Expr → Array (FVarId × Expr) → (Expr × Expr × Expr × Expr) → Expr →
      TermElabM Unit) : TermElabM Unit := do
  match e with
  | .lam n ty b bi =>
    if ty.getAppFn.isConstOf ``HydroSem then
      let L := ty.getAppArgs[0]!; let mem := ty.getAppArgs[1]!
      let pacingTy := mkApp2 (mkConst ``Pacing) L mem
      withLocalDecl `pacing .default pacingTy fun pacing => do
        let S := mkApp3 (mkConst ``SchedSem) L mem pacing
        teleModule (b.instantiate1 S) (dagBinders.push pacing) (allArgs.push S) wires (some (L, mem, S, pacing)) k
    else
      withLocalDecl n bi ty fun x => do
        if ← isWireType ty then
          teleModule (b.instantiate1 x) dagBinders (allArgs.push x) (wires.push (x.fvarId!, ty)) LmemS? k
        else
          teleModule (b.instantiate1 x) (dagBinders.push x) (allArgs.push x) wires LmemS? k
  | _ =>
    let some lms := LmemS? | throwError "project: module has no `HydroSem` binder"
    k dagBinders allArgs wires lms e

/-- Hop 2 of the denotation lemma, as one kernel-judged term:
`Eq.trans (Eq.trans graphs simpProof) (Eq.refl T')` — `graphs : lhs = T` (hop 1),
`simpProof : T = T'` rewrites each `Dag.denote (C.dag …) ⟨…⟩` by `C.dag_denote`
(callee bodies stay opaque constants), and `Eq.refl T'` is accepted at type
`T' = rhs` by the kernel's definitional judgement — the module's own chain
against `T'` differs only in hoisted knot constants and `Prod` eta.
`Meta.isDefEq` takes tens of seconds on that last step; the kernel, milliseconds. -/
def addDagDenoteClosed (name : Name) (stmt : Expr) (graphsThm : Name) (lemmas : Array Name) :
    TermElabM Unit := do
  let stmt ← instantiateMVars stmt
  let val ← forallTelescope stmt fun xs body => do
    let some (ty, lhs, rhs) := body.eq? | throwError "project: `{name}` is not an equation"
    let u ← getLevel ty
    let hG := mkAppN (mkConst graphsThm) xs
    let some (_, _, T) := (← instantiateMVars (← inferType hG)).eq?
      | throwError "project: `{graphsThm}` is not an equation"
    let mut thms : SimpTheorems := {}
    for l in lemmas do thms ← thms.addConst l
    let ctx ← Simp.mkContext {} (simpTheorems := #[thms]) (congrTheorems := ← getSimpCongrTheorems)
    let (r, _) ← Meta.simp T ctx
    let T' ← instantiateMVars r.expr
    let hS ← match r.proof? with
      | some p => instantiateMVars p
      | none => pure (mkApp2 (mkConst ``Eq.refl [u]) ty T)
    unless Kernel.isDefEqGuarded (← getEnv) (← getLCtx) T' rhs do
      throwError "project: the kernel refused the closed chain of `{name}` against the module\n{T'}\n=?=\n{rhs}"
    let h1 := mkApp6 (mkConst ``Eq.trans [u]) ty lhs T T' hG hS
    let hR := mkApp2 (mkConst ``Eq.refl [u]) ty T'
    mkLambdaFVars xs (mkApp6 (mkConst ``Eq.trans [u]) ty lhs T' rhs h1 hR)
  addThm name stmt val

/-! ## The wire codecs

`M.wireCodecs : ∀ binders, Dag.Codecs (M.dag binders)` — one `Codec` per
network node, from the payload types' `Wire` instances; a
callee's codecs are its own `C.wireCodecs`. Skipped (with a note) when a
payload type has no instances. -/

mutual

/-- The codec tuple of a node term. -/
partial def codecsOfNode (L mem : Expr) (n : Expr) : MetaM Expr := do
  let n := n.consumeMData
  let .const c _ := n.getAppFn | throwError "project: codecs: not a node constructor: {n}"
  let args := n.getAppArgs
  if c == ``Hydro.Project.Node.broadcast_closed then
    let α := args[5]!
    mkAppOptM ``Hydro.Codec.ofWire #[α, none]
  else if c == ``Hydro.Project.Node.demux then
    let α := args[5]!
    let ty := mkApp2 (mkConst ``Prod [Level.zero, Level.zero]) (mkConst ``Nat) α
    mkAppOptM ``Hydro.Codec.ofWire #[ty, none]
  else if c == ``Hydro.Project.Node.call then
    let N := args[5]!.consumeMData
    let .const dagName _ := N.getAppFn | throwError "project: codecs: callee graph is not a constant"
    let C := dagName.getPrefix
    unless (← getEnv).contains (C ++ `wireCodecs) do
      throwError "project: codecs: callee {C} has no wire codecs"
    -- the callee's codecs take the payload-type instances after the graph binders
    let mut e := mkAppN (mkConst (C ++ `wireCodecs)) N.getAppArgs
    repeat
      let ty ← whnf (← inferType e)
      match ty with
      | .forallE _ bty _ .instImplicit =>
        let inst ← synthInstance bty
        e := mkApp e inst
      | _ => break
    return e
  else if c == ``Hydro.Project.Node.fixN then
    codecsOfDag L mem args[5]!
  else
    return mkConst ``Unit.unit

/-- The codec tuple of a graph term (`letN` chain, `let`s transparent). -/
partial def codecsOfDag (L mem : Expr) (d : Expr) : MetaM Expr := do
  let d := d.consumeMData
  match d with
  | .letE nm ty v b _ =>
    withLetDecl nm ty v fun x => do
      let r ← codecsOfDag L mem (b.instantiate1 x)
      mkLetFVars #[x] r
  | _ =>
    let .const c _ := d.getAppFn | throwError "project: codecs: not a graph constructor: {d}"
    let args := d.getAppArgs
    if c == ``Hydro.Project.Dag.ret then return mkConst ``Unit.unit
    unless c == ``Hydro.Project.Dag.letN do throwError "project: codecs: unexpected graph head {c}"
    -- Dag.letN L mem Γ κs outs n rest
    let Γ := args[2]!; let κs := args[3]!; let _outs := args[4]!; let n := args[5]!; let rest := args[6]!
    let cn ← codecsOfNode L mem n
    let cr ← codecsOfDag L mem rest
    let tyN := mkAppN (mkConst ``Hydro.Project.Node.Codecs) #[L, mem, Γ, κs, n]
    let Γ' := mkApp3 (mkConst ``HAppend.hAppend [Level.one, Level.one, Level.one])
      (mkApp (mkConst ``List [Level.one]) (mkApp2 (mkConst ``Hydro.Project.Kind) L mem))
      (mkApp (mkConst ``List [Level.one]) (mkApp2 (mkConst ``Hydro.Project.Kind) L mem))
    let _ := Γ'
    let tyR ← inferType cr
    return mkApp4 (mkConst ``Prod.mk [Level.zero, Level.zero]) tyN tyR cn cr

end

/-- Emit `M.wireCodecs`, or note why not. -/
def genWireCodecs (M : Name) (L mem : Expr) (dagBinders : Array Expr) (dagVal dagTy : Expr) : TermElabM Unit := do
  let dagName := M ++ `dag
  try
    -- `L`/`mem` are binders of the graph; locate them by position
    let some iL := dagBinders.findIdx? (· == L) | throwError "project: codecs: `L` is not a graph binder"
    let some imem := dagBinders.findIdx? (· == mem) | throwError "project: codecs: `mem` is not a graph binder"
    -- payload types are type parameters of the module (Rust: `P: Serialize +
    -- Deserialize`): the codecs take `[Wire X]` for each
    let withInsts {α} (xs : Array Expr) (k : Array Expr → TermElabM α) : TermElabM α := do
      let mut tps : Array Expr := #[]
      for i in [0:xs.size] do
        -- every type parameter but the location type
        if i ≠ iL && (← inferType xs[i]!).isType then tps := tps.push xs[i]!
      let rec go (i : Nat) (acc : Array Expr) : TermElabM α := do
        if h : i < tps.size then
          withLocalDecl `instW .instImplicit (mkApp (mkConst ``Hydro.Wire []) tps[i]) fun w =>
            go (i + 1) (acc.push w)
        else k acc
      go 0 #[]
    let cod ← lambdaTelescope dagVal fun xs body => withInsts xs fun insts => do
      let c ← codecsOfDag xs[iL]! xs[imem]! body
      mkLambdaFVars (xs ++ insts) c
    let cod ← instantiateMVars cod
    let ty ← forallTelescope dagTy fun xs _ => withInsts xs fun insts => do
      let dagApp := mkAppN (mkConst dagName) xs
      let dty ← inferType dagApp
      let Γ := dty.getAppArgs[2]!; let outs := dty.getAppArgs[3]!
      mkForallFVars (xs ++ insts) (mkAppN (mkConst ``Hydro.Project.Dag.Codecs) #[xs[iL]!, xs[imem]!, Γ, outs, dagApp])
    addAndCompile (.defnDecl {
      name := M ++ `wireCodecs, levelParams := [], type := ty, value := cod,
      hints := .regular (getMaxHeight (← getEnv) cod + 1), safety := .safe })
    logInfo m!"hydro_project {M}: {M ++ `wireCodecs}"
  catch e =>
    logInfo m!"hydro_project {M}: no wire codecs ({e.toMessageData})"


/-! ## The schedule bundle (D77)

A module's `…Sched` bundle (one trailing binder) carries exactly its network
nodes' cursors — directly (`TransportDec` fields) or through the sub-bundles it
passes to callees. The graph drops it; `M.schedOf` packs it into `Dag.Sched`
and `M.recordOf` unpacks a `Dag.Sched` into it, so every schedule of the graph
is some bundle's (`M.schedOf_recordOf`, by kernel `rfl`: `Prod`/`Unit` eta). -/

/-- The free variables of an expression. -/
def fvarsOf (e : Expr) : Array FVarId := (Lean.collectFVars {} e).fvarIds

/-- Which graph binders the graph keeps: those the body mentions, closed under the
types of kept binders (right to left). -/
def keepMask (binders : Array Expr) (body : Expr) : MetaM (Array Bool) := do
  let n := binders.size
  let mut keep : Array Bool := Array.replicate n false
  let mut needed : Array FVarId := (fvarsOf body)
  for i' in [0:n] do
    let i := n - 1 - i'
    let b := binders[i]!
    if needed.contains b.fvarId! then
      keep := keep.set! i true
      let bty ← instantiateMVars (← inferType b)
      needed := needed ++ (fvarsOf bty)
  return keep

/-- The projection path of `e` from a binder: `e = b.f₁.f₂…` through structure
projections. -/
partial def bundlePath (e : Expr) : Option (FVarId × List Name) := do
  let e := e.consumeMData
  match e with
  | .fvar f => some (f, [])
  | .proj _ _ b => bundlePath b  -- anonymous projections carry no field name; unsupported
  | .app .. =>
    let fn := e.getAppFn
    let .const c _ := fn | none
    -- a projection function `S.f params x`
    let (b, path) ← bundlePath e.appArg!
    some (b, path ++ [c.componentsRev.head!])
  | _ => none

/-- The bundle binders the schedule components project from (the roots of their
projection paths; a callee's are the roots of the bundles it is passed). -/
partial def compBinders (c : SComp) : MetaM (Array FVarId) := do
  match c with
  | .unit => return #[]
  | .leaf d =>
    let some (b, _) := bundlePath d | throwError "project: a node's decision is not a bundle projection: {d}"
    return #[b]
  | .call C args =>
    let some gi := getGenInfo (← getEnv) C | throwError "project: callee {C} unregistered"
    gi.schedIdxs.mapM fun si => do
      let some (b, _) := bundlePath args[si]!
        | throwError "project: callee {C}'s bundle is not a bundle projection: {args[si]!}"
      return b
  | .nested cs => cs.foldlM (fun acc c => return acc ++ (← compBinders c)) #[]

/-- The schedule bundles: the dropped decision binders (ascending) — records of
decisions, bare delivery/timing decisions, `Unit` decisions — whether or not a
component projects from them (a `Unit` decision is rebuilt from nothing). A bundle a
component projects from must be dropped. -/
def schedBindersOf (binders : Array Expr) (keep : Array Bool) (comps : Array SComp) (body : Expr) :
    MetaM (Array Nat) := do
  let used ← comps.foldlM (fun acc c => return acc ++ (← compBinders c)) #[]
  let mut found : Array Nat := #[]
  for i in [0:binders.size] do
    if used.contains binders[i]!.fvarId! && keep[i]! then
      throwError "project: bundle {binders[i]!} is also mentioned by the graph:\n{← ppExpr (← instantiateMVars body)}"
    if !keep[i]! then
      let tyR ← whnfR (← inferType binders[i]!)
      let ty ← whnf tyR
      let isBundle ← do
        if isSchedDecTy tyR || ty.isConstOf ``Unit || ty.isConstOf ``PUnit then pure true
        else if let .const st _ := ty.getAppFn then pure (isStructure (← getEnv) st)
        else pure false
      if isBundle then found := found.push i
  return found

/-- Component `j` of a right-nested `n`-tuple. -/
def tupleProj (e : Expr) (j n : Nat) : Expr := Id.run do
  let mut cur := e
  for _ in [0:j] do cur := Expr.proj ``Prod 1 cur
  if j + 1 < n then Expr.proj ``Prod 0 cur else cur

/-- A right-nested tuple. -/
def mkTuple (vals : Array Expr) : MetaM Expr := do
  let mut acc := vals.back!
  for i' in [1:vals.size] do
    acc ← mkAppM ``Prod.mk #[vals[vals.size - 1 - i']!, acc]
  return acc

/-- A right-nested product type. -/
def mkTupleTy (tys : Array Expr) : MetaM Expr := do
  let mut acc := tys.back!
  for i' in [1:tys.size] do
    acc ← mkAppM ``Prod #[tys[tys.size - 1 - i']!, acc]
  return acc

/-- A projection path from a bundle binder. -/
abbrev BPath := FVarId × List Name

/-- The accesses into a `Dag.Sched` value `S`, by bundle path: `(path, access)` for
leaves and `(path, callee, its graph args, access, which of its bundles, how many)` for
callees. -/
partial def schedAccesses (S : Expr) (comps : Array SComp) :
    MetaM (Array (BPath × Expr) × Array (BPath × Name × Array Expr × Expr × Nat × Nat)) := do
  let mut leaves := #[]
  let mut calls := #[]
  let mut cur := S
  for c in comps do
    let here := Expr.proj ``Prod 0 cur
    match c with
    | .unit => pure ()
    | .leaf d =>
      let some path := bundlePath d | throwError "project: a node's decision is not a bundle projection: {d}"
      leaves := leaves.push (path, here)
    | .call C args =>
      let some gi := getGenInfo (← getEnv) C | throwError "project: callee {C} unregistered"
      let n := gi.schedIdxs.size
      for j in [0:n] do
        let si := gi.schedIdxs[j]!
        let some path := bundlePath args[si]!
          | throwError "project: callee {C}'s bundle is not a bundle projection: {args[si]!}"
        calls := calls.push (path, C, args, here, j, n)
    | .nested cs =>
      let (l, k) ← schedAccesses here cs
      leaves := leaves ++ l; calls := calls ++ k
    cur := Expr.proj ``Prod 1 cur
  return (leaves, calls)

/-- Rebuild a value of the bundle type `ty` at `path` from the accesses. -/
partial def buildBundle (ty : Expr) (path : BPath)
    (leaves : Array (BPath × Expr)) (calls : Array (BPath × Name × Array Expr × Expr × Nat × Nat)) : MetaM Expr := do
  if let some (_, acc) := leaves.find? (·.1 == path) then return acc
  if let some (_, C, args, acc, j, n) := calls.find? (·.1 == path) then
    let rargs ← recArgs C args
    return tupleProj (mkApp (mkAppN (mkConst (C ++ `recordOf)) rargs) acc) j n
  let tyW ← whnf ty
  if tyW.isConstOf ``Unit || tyW.isConstOf ``PUnit then return mkConst ``Unit.unit
  let .const st lvls := tyW.getAppFn
    | throwError "project: bundle field at {path.2} is neither a decision nor a sub-bundle: {ty}"
  unless isStructure (← getEnv) st do
    throwError "project: bundle field at {path.2} is neither a decision nor a sub-bundle: {ty}"
  let ctor := getStructureCtor (← getEnv) st
  let params := tyW.getAppArgs
  let ctorApp := mkAppN (mkConst ctor.name lvls) params
  let mut ctorTy ← whnf (← inferType ctorApp)
  let mut vals : Array Expr := #[]
  for f in getStructureFields (← getEnv) st do
    let .forallE _ fty b _ := ctorTy | throwError "project: structure constructor arity"
    let v ← buildBundle fty (path.1, path.2 ++ [f]) leaves calls
    vals := vals.push v
    ctorTy ← whnf (b.instantiate1 v)
  return mkAppN ctorApp vals

/-- Emit `M.recordOf` and `M.schedOf_recordOf`. -/
def genRecordOf (M : Name) (binders : Array Expr) (keep : Array Bool) (sis : Array Nat) (comps : Array SComp)
    (schedTyE : Expr) : TermElabM Unit := do
  let sbTys ← sis.mapM fun si => do instantiateMVars (← inferType binders[si]!)
  -- the binders the records need: the kept ones and those the bundles' types mention
  let tyFvs := sbTys.foldl (fun acc t => acc ++ fvarsOf t) #[]
  let recKeep := (Array.range binders.size).map fun i => keep[i]! || tyFvs.contains binders[i]!.fvarId!
  let recBinders := (binders.zip recKeep).filterMap fun (b, k) => if k then some b else none
  for i in [0:binders.size] do
    unless recKeep[i]! || sis.contains i do
      throwError "project: graph binder {binders[i]!} is neither kept, a bundle, nor needed by a bundle's type"
  registerInfoT M fun gi => { gi with dagRecKeep := recKeep }
  let recTyBody ← mkTupleTy sbTys
  withLocalDeclD `S schedTyE fun S => do
    let (leaves, calls) ← schedAccesses S comps
    let mut recs := #[]
    for j in [0:sis.size] do
      recs := recs.push (← buildBundle sbTys[j]! (binders[sis[j]!]!.fvarId!, []) leaves calls)
    let rec_ ← mkTuple recs
    let recVal ← instantiateMVars (← mkLambdaFVars (recBinders.push S) rec_)
    let recTy ← instantiateMVars (← mkForallFVars (recBinders.push S) recTyBody)
    addAndCompile (.defnDecl {
      name := M ++ `recordOf, levelParams := [], type := recTy, value := recVal,
      hints := .regular (getMaxHeight (← getEnv) recVal + 1), safety := .safe })
    -- `M.schedOf binders[bundles := M.recordOf S] = S`
    let recApp := mkApp (mkAppN (mkConst (M ++ `recordOf)) recBinders) S
    let mut args := binders
    for j in [0:sis.size] do
      args := args.set! sis[j]! (tupleProj recApp j sis.size)
    let lhs := mkAppN (mkConst (M ++ `schedOf)) args
    let stmt ← instantiateMVars (← mkForallFVars (recBinders.push S) (← mkEq lhs S))
    unless ← addThmByKernelRfl (M ++ `schedOf_recordOf) stmt do
      let so := ((← getEnv).find? (M ++ `schedOf)).bind (·.value?)
      throwError "project: the kernel refused `{M ++ `schedOf_recordOf}`:{indentExpr stmt}\nrecordOf:{indentExpr recVal}\nschedOf:{indentExpr (so.getD (mkConst ``Unit.unit))}\nS type:{indentExpr (← whnf schedTyE)}"
    logInfo m!"hydro_project {M}: {M ++ `recordOf}, {M ++ `schedOf_recordOf}"

/-- Emit `M.dag`, `M.schedOf` (and `M.recordOf`), and `M.dag_denote`. -/
def genProject (M : Name) : TermElabM Unit := do
  let env ← getEnv
  let some gi := getGenInfo env M | return
  if gi.isKnot then return
  if env.contains (M ++ `dag) then return
  let info ← getConstInfo M
  let some val := info.value? | throwError "project: {M} has no value"
  teleModule val #[] #[] #[] none fun dagBinders allArgs wires (L, mem, S, pacing) body => do
    -- the input context: wires in order (top-first), so bottom-first is reversed
    let mut wireKinds : Array Expr := #[]
    for (_, ty) in wires do
      let some k ← kindOfType ty | throwError "project: input is not a wire"
      wireKinds := wireKinds.push k
    let nW := wires.size
    let binds0 : List (FVarId × Bind) := (List.range nW).map (fun i => (wires[i]!.1, Bind.wire (nW - 1 - i)))
    let st0 : WState := { kinds := wireKinds.reverse, binds := binds0, entries := #[], knotBody := none }
    withLets body #[] fun items final => do
      let diag := (← IO.getEnv "HYDRO_PHASE_DIAG").isSome
      let tick := fun (what : String) (t0 : Nat) => do
        if diag then IO.eprintln s!"[phase] {M} project/{what}: {(← IO.monoMsNow) - t0}ms"
        IO.monoMsNow
      let t ← IO.monoMsNow
      let (outPoss, st) ← (walkItems L mem S pacing M items 0 final).run st0
      let t ← tick "walk" t
      let dagBody ← instantiateMVars (← assemble L mem st outPoss)
      -- the graph is pacing-free: the data terms' `SchedSem L mem pacing` residue is
      -- taken at `Pacing.idle` (the denotation lemma below still quantifies `pacing`)
      let dagBody := dagBody.replaceFVar pacing (mkApp2 (mkConst ``Hydro.Pacing.idle) L mem)
      -- D77: the graph keeps the binders it mentions (and those their types need);
      -- the schedule bundle — mentioned only by the network nodes' cursors, now
      -- schedule components — is dropped
      let keep ← keepMask dagBinders dagBody
      let kept := (dagBinders.zip keep).filterMap fun (b, k) => if k then some b else none
      let schedBody ← instantiateMVars (← assembleSched st)
      let comps := compsOf st
      let schedIdxs ← schedBindersOf dagBinders keep comps dagBody
      registerInfoT M fun gi => { gi with dagKeep := keep, schedIdxs := schedIdxs }
      let dagVal ← mkLambdaFVars kept dagBody
      let dagVal ← instantiateMVars dagVal
      let t ← tick "assemble" t
      let dagTy ← inferType dagVal
      let t ← tick "inferType" t
      let dagName := M ++ `dag
      addAndCompile (.defnDecl {
        name := dagName, levelParams := [], type := dagTy, value := dagVal,
        hints := .regular (getMaxHeight env dagVal + 1), safety := .safe })
      enableRealizationsForConst dagName
      genWireCodecs M L mem kept dagVal dagTy
      -- `M.schedOf : ∀ binders, Dag.Sched (M.dag kept)` — the module's bundle in graph shape
      let dagAppK := mkAppN (mkConst dagName) kept
      let dagTyK ← inferType dagAppK
      let schedTyE := mkAppN (mkConst ``Hydro.Project.Dag.Sched)
        #[L, mem, dagTyK.getAppArgs[2]!, dagTyK.getAppArgs[3]!, dagAppK]
      let schedVal ← instantiateMVars (← mkLambdaFVars dagBinders schedBody)
      let schedOfTy ← instantiateMVars (← mkForallFVars dagBinders schedTyE)
      addAndCompile (.defnDecl {
        name := M ++ `schedOf, levelParams := [], type := schedOfTy, value := schedVal,
        hints := .regular (getMaxHeight env schedVal + 1), safety := .safe })
      -- `M.recordOf : ∀ binders', Dag.Sched (M.dag kept) → <bundle>`, and the isomorphism
      unless schedIdxs.isEmpty do
        genRecordOf M dagBinders keep schedIdxs comps schedTyE
      let t ← tick "addAndCompile" t
      -- the denotation lemma, over an arbitrary input environment `env : Env Γ0`
      -- (the inputs appear as `env.get vᵢ`; a variable here keeps the lemma a
      -- robust `simp` rewrite — a literal `HList.cons …` environment would put
      -- carrier types into the discrimination-tree keys, where reducible
      -- unfolding of `Kind.sched` makes lemma and target keys disagree)
      let Γ0 := mkKindList L mem wireKinds
      let outKinds := outPoss.map (fun p => st.kinds[p]!)
      let wireFVars := wires.map (fun (f, _) => mkFVar f)
      withLocalDeclD `env (hlistTy L mem Γ0) fun envV => do
      let mut gets : Array Expr := #[]
      for i in [0:wires.size] do gets := gets.push (← hlistGetLit L mem wireKinds envV i)
      let schedApp := mkAppN (mkConst (M ++ `schedOf)) dagBinders
      let lhs := mkAppN (mkConst ``Hydro.Project.Dag.denote)
        #[L, mem, pacing, Γ0, mkKindList L mem outKinds, dagAppK, schedApp, envV]
      let prog := (mkAppN (mkConst M) allArgs).replaceFVars wireFVars gets
      let mut comps : Array Expr := #[]
      for c in [0:outKinds.size] do
        comps := comps.push (← tupleComponent prog c outKinds.size)
      let rhs := mkEnvList L mem outKinds comps
      let binders := dagBinders.push envV
      let stmt ← mkForallFVars binders (← mkEq lhs rhs)
      let stmt ← instantiateMVars stmt
      let gi := (getGenInfo (← getEnv) M).getD {}
      let t ← tick "statement" t
      let route ←
        if gi.callees.isEmpty then
          -- no callee graphs to keep closed: one kernel judgement
          unless ← addThmByKernelRfl (M ++ `dag_denote) stmt do
            throwError "project: the kernel refused `{M ++ `dag_denote}` (set HYDRO_PHASE_DIAG=1 and compare `{dagName}` against `{M}`)"
          pure "kernel rfl"
        else
          -- hop 1 (kernel): the graph against the opened chain
          let T ← graphsTerm L mem S pacing M items 0 final
          let T := (← instantiateMVars T).replaceFVars wireFVars gets
          let mut compsT : Array Expr := #[]
          for c in [0:outKinds.size] do
            compsT := compsT.push (← tupleComponent T c outKinds.size)
          let stmtT ← instantiateMVars (← mkForallFVars binders (← mkEq lhs (mkEnvList L mem outKinds compsT)))
          let t ← tick "graphs term" t
          unless ← addThmByKernelRfl (M ++ `dag_denote_graphs) stmtT do
            throwError "project: the kernel refused `{M ++ `dag_denote_graphs}` (set HYDRO_PHASE_DIAG=1 and compare `{dagName}` against the opened chain of `{M}`)"
          let t ← tick "graphs kernel" t
          -- hop 2: callee graphs closed by their own lemmas (a `simp` rewrite on the
          -- opened chain, bodies stay opaque), then the module by kernel judgement
          let mut lemmas : Array Name := #[]
          for c in gi.callees do
            if !(isKnotOf (← getEnv) M c) && (← getEnv).contains (c ++ `dag_denote) then
              lemmas := lemmas.push (c ++ `dag_denote)
          addDagDenoteClosed (M ++ `dag_denote) stmt (M ++ `dag_denote_graphs) lemmas
          let _ ← tick "close" t
          pure "kernel graphs + simp + kernel"
      let _ ← tick "dag_denote" t
      logInfo m!"hydro_project {M}: {dagName}, {M ++ `dag_denote} ({route})"

end Project

end HydroGen

end Hydro
