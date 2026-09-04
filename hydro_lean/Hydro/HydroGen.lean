import Hydro.CoupleProj
import Hydro.KnotTactics
import Hydro.WfTactics

/-!
# Hydro · `HydroGen` — the program-machinery generator

Given a program module written in the house style (one Rust fn = one
`hydro def`, generic over `H : HydroSem`, colocated `ensures` contract,
explicit decision arguments), the `HydroGen` phases — run by `hydro
def` from the module's recorded composition spec (`HydroDef.lean`
`runPipeline`) — generate the mechanical artifacts that previously
lived in hand-written files:

- the **couple** phase (`runCoupleT`) — the corner naming stack: per-leg machine namings
  `M_co_srᵢ` (the corner run's machine leg IS the `SchedSem` run) and
  denotational namings. Modules with a **content** decision get the
  choice triple `M_co_rr_ex` / `M_vdec` / `M_co_rrᵢ` — the derived
  decision is *named by choice* from the ∃-lemma, no wire replay
  (resolving `CoupleDec.lean`'s TEMP note by deletion); modules whose
  decisions are all machine data get plain equations.
- the **causal** phase (`runModCausalT`) — per-leg step-causality congruences over the
  `SchedSem` legs (head-dispatched per-op walker).
- the **wf** phase (`runModWfT`) — corner wf-threading (inputs' `wf` ⟹ outputs' `wf`).
- the **mono** phase (`runModMonoT`) — per-leg `Values` monotonicity via the `MonoRel`
  instantiation (grade-directed relations).
- the **knot** phase (`runKnotStackT`, `HydroGenKnot.lean`) — the knot
  stack for a `HydroSem.fix`/`fixTick` wrapper (naming, causality,
  chain, `wf` triple, Kleene stages); the **glue** phase (`runGlueT`)
  — simp-discovery namings for modules over folded knot callees; the
  **param** phase (`genParam`, `HydroParam.lean`) — the free theorems.

Statements are synthesized at the `Expr` level from the module's
**type** alone (binder classification: wires / primitive decisions /
plain data); proofs run the fixed house scripts (`co_transfer`, the
`Exists.intro` pin, the causal walker). Generated names are stable
and greppable: `<module>_co_sr₁`, `<module>_co_rr_ex`, `<module>_vdec`,
`<module>_causal₁`, `<module>_co_wf₁`, `<module>_mono₁`, …

Design rules honored (D33/D38–D41): statements per MODULE (kernel
cost module-sized, never whole-program), per-operator proof content
only (the scripts assemble once-proven op lemmas), zero new proof
principles.
-/

namespace Hydro

open Lean Elab Term Meta Command

namespace HydroGen

/-! ## Vocabulary classification -/

/-- The signature decision families. -/
inductive DecFam where
  | transport | orderSel | tickOrderSel | snap | batch | ordBatch
  | sample | timer | pulse | fixd
  deriving BEq, Repr, Inhabited

def decFamOfHead : Name → Option DecFam
  | ``HydroSem.TransportDec => some .transport
  | ``HydroSem.OrderSelDec => some .orderSel
  | ``HydroSem.TickOrderSelDec => some .tickOrderSel
  | ``HydroSem.SnapDec => some .snap
  | ``HydroSem.BatchDec => some .batch
  | ``HydroSem.OrdBatchDec => some .ordBatch
  | ``HydroSem.SampleDec => some .sample
  | ``HydroSem.TimerDec => some .timer
  | ``HydroSem.PulseDec => some .pulse
  | ``HydroSem.FixDec => some .fixd
  | _ => none

/-- Content decisions: `Values`-side data the corner derives from its
machine leg (`Unit` at the machine). `fixd` is content but *pinned*
(the corner's fuel is `Td + 1` by construction), so it is not
∃-quantified. -/
def DecFam.isContent : DecFam → Bool
  | .orderSel | .tickOrderSel | .snap | .batch | .ordBatch => true
  | _ => false

/-- Machine-side pass-through (the corner's decision type = the
machine's). -/
def DecFam.schedPass : DecFam → Bool
  | .transport | .sample | .timer | .pulse => true
  | _ => false

/-- `Values`-side pass-through (data in both worlds). -/
def DecFam.valuesPass : DecFam → Bool
  | .sample | .timer | .pulse => true
  | _ => false

/-- The carrier families (`ticked` = `HydroSem.Atomic`, the tick-located
trace; `bounded` = `HydroSem.BoundedStream`, one tick's content —
appears only inside tick bodies, never as a module wire). -/
inductive CarrierFam where
  | stream | keyed | sing | ksing | atomic | atomicStream | atomicKSing | atomicKeyed
  | bounded | bsing | bopt | bkeyed | bksing
  deriving BEq, Repr, Inhabited

def carrierFamOfHead : Name → Option CarrierFam
  | ``HydroSem.Stream => some .stream
  | ``HydroSem.KeyedStream => some .keyed
  | ``HydroSem.Singleton => some .sing
  | ``HydroSem.KeyedSingleton => some .ksing
  | ``HydroSem.Atomic => some .atomic
  | ``HydroSem.AtomicStream => some .atomicStream
  | ``HydroSem.AtomicKeyedSingleton => some .atomicKSing
  | ``HydroSem.AtomicKeyedStream => some .atomicKeyed
  | ``HydroSem.BoundedStream => some .bounded
  | ``HydroSem.BoundedSingleton => some .bsing
  | ``HydroSem.BoundedOptional => some .bopt
  | ``HydroSem.BoundedKeyedStream => some .bkeyed
  | ``HydroSem.BoundedKeyedSingleton => some .bksing
  | _ => none

/-- Argument classification (from the binder type, `H` visible). -/
inductive ArgKind where
  | wire (f : CarrierFam)
  | primDec (f : DecFam)
  /-- A decision record: a structure parameterized by the
  interpretation whose fields are decision families (possibly
  nested records). -/
  | decRec (s : Name)
  | plain
  deriving BEq, Repr, Inhabited

/-- Replace one interpretation instance by another inside a term. -/
def reinterp (fromI toI : Expr) (e : Expr) : Expr :=
  e.replace fun x => if x == fromI then some toI else none

def classifyArg (corner : Expr) (t : Expr) : MetaM ArgKind := do
  if let .const n _ := t.getAppFn then
    if let some cf := carrierFamOfHead n then return .wire cf
    if let some df := decFamOfHead n then return .primDec df
    if isStructure (← getEnv) n && t.getAppArgs.any (· == corner) then
      return .decRec n
  return .plain

/-! ### Decision-record field trees -/

/-- A decision record's field tree (relative to one interpretation
instance). -/
inductive DecFieldTree where
  | prim (field : Name) (f : DecFam)
  | node (field : Name) (s : Name) (sub : Array DecFieldTree)
  deriving Inhabited, Repr

/-- Enumerate the decision fields of a record VALUE `e` (typed at an
interpretation containing `inst`). -/
partial def decFieldTreeOf (inst : Expr) (e : Expr) :
    TermElabM (Array DecFieldTree) := do
  let ty ← whnfR (← inferType e)
  let .const s _ := ty.getAppFn |
    throwError "HydroGen: decision record type head is not a const"
  let mut out := #[]
  for f in getStructureFields (← getEnv) s do
    let p ← mkProjection e f
    let pty ← whnfR (← inferType p)
    let mut done := false
    if let .const n _ := pty.getAppFn then
      if let some df := decFamOfHead n then
        out := out.push (.prim f df)
        done := true
      else if isStructure (← getEnv) n
          && pty.getAppArgs.any (· == inst) then
        out := out.push (.node f n (← decFieldTreeOf inst p))
        done := true
    unless done do
      throwError "HydroGen: field {f} of {s} is not a decision \
        family or nested decision record ({pty})"
  pure out

/-- Flattened field paths with families. -/
partial def flattenTree (t : Array DecFieldTree)
    (path : List Name := []) : Array (List Name × DecFam) := Id.run do
  let mut out := #[]
  for n in t do
    match n with
    | .prim f df => out := out.push (path ++ [f], df)
    | .node f _ sub => out := out ++ flattenTree sub (path ++ [f])
  pure out

/-- Does the record carry any content (or fix) decision? Such records
are the ∃-unit of the choice naming. -/
def treeHasContent (t : Array DecFieldTree) : Bool :=
  (flattenTree t).any fun (_, f) => f.isContent || f == .fixd

def applyFieldPath (e : Expr) : List Name → MetaM Expr
  | [] => pure e
  | f :: p => do applyFieldPath (← mkProjection e f) p

/-- Rebuild the record `e` (typed with instance `fromI` inside) at the
target instance `toI`, mapping each primitive decision field through
`mode`. -/
partial def repackRec (fromI toI : Expr)
    (mode : DecFam → Expr → TermElabM Expr) (e : Expr) :
    TermElabM Expr := do
  let ty ← whnfR (← inferType e)
  let .const s _ := ty.getAppFn |
    throwError "HydroGen: repack head is not a const"
  let ctor := getStructureCtor (← getEnv) s
  let tyArgs := ty.getAppArgs.map (reinterp fromI toI)
  let mut fields : Array Expr := #[]
  for f in getStructureFields (← getEnv) s do
    let p ← mkProjection e f
    let pty ← whnfR (← inferType p)
    let v ← do
      if let .const n _ := pty.getAppFn then
        if let some df := decFamOfHead n then mode df p
        else repackRec fromI toI mode p
      else throwError "HydroGen: repack field {f}"
    fields := fields.push v
  mkAppOptM ctor.name ((tyArgs ++ fields).map some)

/-- The nested anonymous-constructor skeleton for a record's
∃-witness: `_` for fields carrying `Values`-side data (pinned by
unification), literal `()` for machine-only fields (`Unit` at
`Values` — they never appear in the reduced equations, so a hole
would stay unassigned). -/
partial def skeletonOf (t : Array DecFieldTree) : String :=
  "⟨" ++ String.intercalate ", "
    (t.toList.map fun
      | .prim _ f =>
        if f.schedPass && !f.valuesPass then "()" else "_"
      | .node _ _ sub => skeletonOf sub) ++ "⟩"

/-- Output-leg path steps. -/
inductive LegStep where
  | val | fst | snd
  /-- A named structure-field projection (H-parameterized wire
  bundles). -/
  | field (name : Name)
  deriving BEq, Repr, Inhabited

def applyLeg (e : Expr) : List LegStep → MetaM Expr
  | [] => pure e
  | .val :: p => do applyLeg (← mkProjection e `val) p
  | .fst :: p => do applyLeg (← mkProjection e `fst) p
  | .snd :: p => do applyLeg (← mkProjection e `snd) p
  | .field f :: p => do applyLeg (← mkProjection e f) p

/-- Peel the output type into leaf legs (paths + leaf types).
`Subtype` → `.val`; products → components; interpretation-indexed
structures of carriers (wire bundles) → field projections; anything
else → leaf. -/
partial def peelLegsTy (ty : Expr) (path : List LegStep) :
    MetaM (Array (List LegStep × Expr)) := do
  let f := ty.getAppFn
  if let .const n _ := f then
    if n == ``Subtype then
      return (← peelLegsTy (ty.getArg! 0) (path ++ [.val]))
    if n == ``Prod then
      let l ← peelLegsTy (ty.getArg! 0) (path ++ [.fst])
      let r ← peelLegsTy (ty.getArg! 1) (path ++ [.snd])
      return l ++ r
    if carrierFamOfHead n |>.isSome then
      return #[(path, ty)]
    if isStructure (← getEnv) n then
      -- a wire bundle iff every field type is carrier-headed
      let fields := getStructureFields (← getEnv) n
      if !fields.isEmpty then
        let res? ← withLocalDecl `o .default ty fun oE => do
          let mut out := #[]
          for fd in fields do
            let p ← mkProjection oE fd
            let fldTy ← inferType p
            let ok := if let .const fh _ := fldTy.getAppFn then
              (carrierFamOfHead fh).isSome else false
            if !ok || fldTy.containsFVar oE.fvarId! then
              return none
            out := out.push (path ++ [.field fd], fldTy)
          pure (some out)
        if let some out := res? then
          return out
  return #[(path, ty)]

def peelLegs (ty : Expr) (path : List LegStep) :
    MetaM (Array (List LegStep)) := do
  return (← peelLegsTy ty path).map (·.1)

/-- Subscript numerals for generated names. -/
def subscript (n : Nat) : String :=
  String.ofList (n.repr.toList.map fun c =>
    Char.ofNat (c.toNat - '0'.toNat + '₀'.toNat))

/-! ## The generation registry -/

structure GenInfo where
  srLemmas : Array Name := #[]
  rrLemmas : Array Name := #[]
  exLemma? : Option Name := none
  vdec? : Option Name := none
  causalLemmas : Array Name := #[]
  /-- Per-leg machine tick-monotonicity (`M_smono_i : TMono …` for a
  tick-family leg, `TMonoTriv …` otherwise; D71). -/
  smonoLemmas : Array Name := #[]
  wfLemmas : Array Name := #[]
  monoLemmas : Array Name := #[]
  /-- Named fuel-pin projections of the choice spec (`(M_vdec …).fuel
  = Td + 1` per `fixd` field) — the knot route rewrites the `Values`
  fix's fuel leg with these. -/
  pinLemmas : Array Name := #[]
  /-- Knot modules (the knot phase): consumers keep them folded (their
  `wf` goes through `K_co_wf₁`, never through `co_wf_simp`). -/
  isKnot : Bool := false
  /-- The composition spec recorded by `hydro def`/`hydro_register`:
  the module invocations the body makes (validated against the module
  grammar at definition time), the inline wrappers it opens, and
  whether it ties a knot. Generators consume this instead of guessing
  (no transitive scans, no discovery). -/
  callees : Array Name := #[]
  inlines : Array Name := #[]
  hasFix : Bool := false
  hasSpec : Bool := false
  /-- The module carries a `requires` clause (D71): its `_co_wf` lemmas
  take one more hypothesis — the clause at the denotation legs of its
  inputs — after the input `wf`s, and callers supply it. -/
  hasRequires : Bool := false
  /-- `Project` (D77): which of the module's graph binders (`H` ↦ `pacing`, then
  every non-wire argument, in order) `M.dag` keeps — the schedule bundle is not
  program data and is dropped; `M.schedOf` takes them all. -/
  dagKeep : Array Bool := #[]
  /-- `Project` (D77): the indices (among the graph binders, ascending) of the module's
  schedule bundles — the decision and schedule records `M.recordOf` rebuilds from a
  `Dag.Sched` (a right-nested tuple when there are several). -/
  schedIdxs : Array Nat := #[]
  /-- `Project` (D77): which graph binders `M.recordOf` takes — the kept ones and those
  the bundles' types mention. -/
  dagRecKeep : Array Bool := #[]
  deriving Inhabited

initialize genRegistry :
    SimplePersistentEnvExtension (Name × GenInfo) (NameMap GenInfo) ←
  registerSimplePersistentEnvExtension {
    addImportedFn := fun as =>
      as.foldl (fun m es => es.foldl (fun m (n, i) => m.insert n i) m) {}
    addEntryFn := fun m (n, i) => m.insert n i
  }

def getGenInfo (env : Environment) (n : Name) : Option GenInfo :=
  (genRegistry.getState env).find? n

/-- `H`-generic wrapper constants opened during spec walks and
generation: the `fix` elaborator registers each hoisted chain body
(`<M>.body`) here. -/
initialize inlineRegistry :
    SimplePersistentEnvExtension Name NameSet ←
  registerSimplePersistentEnvExtension {
    addImportedFn := fun as =>
      as.foldl (fun m es => es.foldl (fun m n => m.insert n) m) {}
    addEntryFn := fun m n => m.insert n
  }

/-! ## The `requires` clause and the `ghost pre` facts (D71, R3)

A conditional fold (`Pre ≠ fun _ _ => True`) leaves a pairwise content
conjunct in the corner's `wf`; a callee with a `requires` clause leaves
its clause as a hypothesis of its `_co_wf` lemmas. Both are closed in
the enclosing module's `_co_wf` by the facts the program proved at the
denotation BEFORE the site (`ghost pre h : S := prf`, packaged as the
theorems `M.pre_h : ∀ binders, let wires…; R → S` with `R` the module's
own `requires`), tried by `hydro_pre_close` as closers; the module's
own `requires` becomes the lemma's trailing hypothesis `hreq`. Both
live in command-scoped refs (set by `hydro def`, read by the phases). -/

/-- The current `hydro def`'s `requires` clause, as source (re-elaborated
by name in each binder context that needs it). -/
initialize currentRequires : IO.Ref (Option Syntax) ← IO.mkRef none

/-- The `ghost pre` theorems registered so far in the current `hydro def`. -/
initialize currentPres : IO.Ref (Array Name) ← IO.mkRef #[]

/-- The assumed knot `invariant`s of the current `hydro def` (D71):
hoisted chain-body module ↦ the invariant `I`, as source over the wire's
NAME. The chain module's `requires` is the conjunction `R ∧ I` (the
def's clause, then the wire's); its `_co_wf` lemma's `hreq` has that
shape, and the body's `ghost pre` proofs see it as `hreq` and `h<w>_inv`. -/
initialize loopInvariants : IO.Ref (NameMap Syntax) ← IO.mkRef {}

/-- The assumed invariants in force at the current elaboration point
(innermost first; wire name and predicate): the `fix` elaborator pushes
its clause around the body, so a `ghost pre` inside the body states
`R ∧ I` — the same proposition the chain module's `_co_wf` takes as
`hreq`. -/
initialize fixLoopStack : IO.Ref (List (Name × Syntax)) ← IO.mkRef []

/-- Knot ↦ its assumed invariant's STAGE theorem (`<def>.<w>.inv_stages :
∀ binders m, I (stages binders m)`, unbounded) — produced by the `fix`
elaborator between the knot's `stages` and `wf` generation (the hook
below), and consumed by the knot `wf`'s graded leg to feed the body
module's `hreq` at the Kleene stage. -/
initialize knotLoopInv : IO.Ref (NameMap Name) ← IO.mkRef {}

/-- The `fix` elaborator's `invariant` hook: called by the knot stack
generator with the knot's name after its `stages` vocabulary exists and
before its `wf` is generated (the clause's induction is an
`iterate_invariant` over the stages; the `wf` needs its conclusion).
The hook lives in `HydroDef` (it elaborates user proofs against the
chain's runs), so it is injected, not imported. -/
initialize loopInvHook : IO.Ref (Option (Name → TermElabM Unit)) ← IO.mkRef none

/-- The `requires` proposition of module `M` as source: the def's clause
and (for a hoisted chain body) its loop-wire clause, conjoined `R ∧ Rw`.
`none` when neither applies. -/
def requiresStxFor (M : Name) : TermElabM (Option Syntax) := do
  let r? ← currentRequires.get
  let rw? := (← loopInvariants.get).find? M
  match r?, rw? with
  | none, none => pure none
  | some r, none => pure (some r)
  | none, some rw => pure (some rw)
  | some r, some rw => do
    let c ← `(term| $(⟨r⟩) ∧ $(⟨rw⟩))
    pure (some c.raw)

/-- The `requires` proposition in force at a `ghost pre`: the def's clause
conjoined with each enclosing `fix`'s loop-wire clause (outermost first),
spelled exactly as `requiresStxFor` spells the chain module's. -/
def requiresStxHere : TermElabM (Option Syntax) := do
  let r? ← currentRequires.get
  let rws ← fixLoopStack.get
  let parts := (match r? with | some r => [r] | none => []) ++ (rws.reverse.map (·.2))
  match parts with
  | [] => pure none
  | p :: rest => do
    let mut acc := p
    for q in rest do
      let c ← `(term| $(⟨acc⟩) ∧ $(⟨q⟩))
      acc := c.raw
    pure (some acc)

/-- Elaborate module `M`'s `requires` clause at its **`Values`**
binders (the clause is stated over the inputs' denotation legs): the
module's type opened with `H := Values L mem`, the clause elaborated by
name there, abstracted over those binders — a closed `fun ctx args => R`.
`none` when the module has no clause. -/
def requiresFnAt (M : Name) : TermElabM (Option Expr) := do
  let some r ← requiresStxFor M | return none
  let info ← getConstInfo M
  let hIdx? ← forallTelescope info.type fun xs _ => do
    xs.findIdxM? fun x => do pure ((← inferType x).getAppFn.isConstOf ``HydroSem)
  let some hIdx := hIdx? | throwError "HydroGen: {M} has no `HydroSem` binder"
  forallBoundedTelescope info.type hIdx fun pre tyH => do
    let hTy := tyH.bindingDomain!
    let V := mkApp2 (mkConst ``Values) (hTy.getArg! 0) (hTy.getArg! 1)
    let restV := tyH.bindingBody!.instantiate1 V
    -- function-output modules: the result's own binders are wires too
    forallTelescope restV fun post resTy => do
      forallTelescope resTy fun ys _ => do
        let rE ← Term.withSynthesize <| elabType (⟨r⟩ : TSyntax `term)
        let rE ← instantiateMVars rE
        pure (some (← mkLambdaFVars (pre ++ post ++ ys) rE))

/-! ## Proof-by-script -/

/-- Per-declaration cost diagnostics (D72): `HYDROGEN_HB_DIAG=1` prints
`[hb] <label>: <raw heartbeats / 1000 = maxHeartbeats units> <ms>` for
every generated declaration. Measurement only — it changes no
accounting: a `hydro def`'s generated declarations count against the
command they are elaborated in, as Lean counts them (the user's ruling,
D72: heartbeats are a proxy; proof TIME is the goal, and the two
remaining `set_option maxHeartbeats … in` call sites are measured facts
about that time, not engine defaults). -/
def withDeclDiag (label : String) (x : TermElabM α) : TermElabM α := do
  let hb0 ← IO.getNumHeartbeats
  let t0 ← IO.monoMsNow
  let r ← x
  if (← IO.getEnv "HYDROGEN_HB_DIAG").isSome then
    IO.eprintln s!"[hb] {label}: {((← IO.getNumHeartbeats) - hb0) / 1000}k {(← IO.monoMsNow) - t0}ms"
  pure r

/-- Prove `goal` by a tactic script given as a string (generated
scripts reference only global names). -/
def proveByTac (goal : Expr) (tac : String) (label : String := "") : TermElabM Expr :=
  withDeclDiag (if label == "" then s!"script {tac.take 60}" else label) do
  let env ← getEnv
  let stx ← match Parser.runParserCategory env `tactic tac with
    | .ok s => pure s
    | .error e => throwError "HydroGen: tactic parse error: {e}\n{tac}"
  let byStx ← `(by $(⟨stx⟩):tactic)
  -- the goal is closed; elaborate in an empty local context so the
  -- script's `intro` names cannot be shadowed by the generator's
  -- open telescopes
  -- per-declaration heartbeats (`withDeclDiag` above): a timeout
  -- here is a diagnostic failure of the script structure, never a
  -- budget problem
  -- recovery OFF (Lean's own knob): a failing alternative is an exception,
  -- never an admitted goal
  -- generated simp sets are deliberately over-approximate (one list per phase
  -- serves every module shape), so the unused-simp-arg linter is noise here;
  -- it has two entry points (tactic-time option read, and the command-level
  -- info-tree pass), so disable the option AND skip the info tree — a
  -- generated proof has nothing to hover over
  let val ← withLCtx {} {} <| Term.withoutErrToSorry <| withEnableInfoTree false <|
      withOptions (fun o => Lean.Elab.Tactic.linter.unusedSimpArgs.set o false) do
    let mvar ← mkFreshExprSyntheticOpaqueMVar goal
    let remaining ← Tactic.run mvar.mvarId! <| Tactic.withoutRecover do
      Tactic.evalTactic stx; Tactic.pruneSolvedGoals
     
    let remaining ← remaining.filterM fun g => do
      pure (!(← g.isAssigned) && !(← g.isDelayedAssigned))
    unless remaining.isEmpty do
      throwError "HydroGen: unsolved goals ({remaining.length}) under\n{tac}\n{goalsToMessageData remaining}"
    Term.synthesizeSyntheticMVarsNoPostponing
    instantiateMVars mvar
  let _ := byStx
  let val ← instantiateMVars val
  if val.hasExprMVar || val.hasLevelMVar then
    throwError "HydroGen: proof has metavariables under\n{tac}"
  pure val

def addThm (name : Name) (stmt : Expr) (val : Expr) : TermElabM Unit := do
  let stmt ← instantiateMVars stmt
  let val ← instantiateMVars val
  addDecl (.thmDecl { name, levelParams := [], type := stmt, value := val })

/-- Prove a closed `∀ …, lhs = rhs` by `Eq.refl`, judged by the KERNEL directly.

`Meta.isDefEq` (what `rfl`/`simp` run) re-traverses unshared subterms and can be
50× slower than the kernel on large typed-IR terms (every `Node Γ κs` carries its
context); the kernel is the trusted judge of every proof anyway, so routing the
check straight to it loses nothing. The pre-check with `Kernel.isDefEqGuarded`
exists only so a refusal returns `false` cleanly (a failed `addDecl` would leave
the name behind as an axiom) and the caller can fall back to a tactic script
with a readable error. -/
def addThmByKernelRfl (name : Name) (stmt : Expr) : TermElabM Bool := do
  let stmt ← instantiateMVars stmt
  let ok ← forallTelescope stmt fun _ body => do
    let some (_, lhs, rhs) := body.eq? | return false
    return Kernel.isDefEqGuarded (← getEnv) (← getLCtx) lhs rhs
  unless ok do return false
  let prf ← forallTelescope stmt fun xs body => do
    let some (ty, lhs, _) := body.eq? | throwError "addThmByKernelRfl: not an equation"
    mkLambdaFVars xs (mkApp2 (mkConst ``Eq.refl [← getLevel ty]) ty lhs)
  addThm name stmt prf
  return true

def addThmByTac (name : Name) (stmt : Expr) (tac : String) :
    TermElabM Unit := do
  let stmt ← instantiateMVars stmt
  let val ← proveByTac stmt tac (label := toString name)
  if let some pat ← IO.getEnv "HYDROGEN_TAC_DIAG" then
    if pat == "1" || (toString name).endsWith pat then
      IO.eprintln s!"[tac] {name}\n  stmt: {← ppExpr stmt}\n  tac: {tac}"
  addThm name stmt val

def addDefn (name : Name) (ty val : Expr) : TermElabM Unit := do
  let ty ← instantiateMVars ty
  let val ← instantiateMVars val
  addDecl (.defnDecl {
    name, levelParams := [], type := ty, value := val,
    hints := .regular (getMaxHeight (← getEnv) val + 1),
    safety := .safe })
  -- consumers may register the def as a simp unfold target, which
  -- realizes auxiliary constants (`eq_def`) — allowed only after this
  enableRealizationsForConst name

/-- Right-nested `And` projection `i` of `k` conjuncts. -/
partial def andProj (e : Expr) (i k : Nat) : MetaM Expr := do
  if k == 1 then return e
  if i == 0 then mkAppM ``And.left #[e]
  else andProj (← mkAppM ``And.right #[e]) (i - 1) (k - 1)

def mkAndN (es : Array Expr) : MetaM Expr := do
  if es.isEmpty then return mkConst ``True
  let mut acc := es.back!
  for i in [1:es.size] do
    acc ← mkAppM ``And #[es[es.size - 1 - i]!, acc]
  pure acc

/-! ## The module telescope -/

structure ModCtx where
  mName : Name
  ctxArgs : Array Expr
  L : Expr
  memE : Expr
  pacing : Expr
  Tc : Expr
  Td : Expr
  hjT : Expr
  corner : Expr
  sched : Expr
  values : Expr
  args : Array Expr
  kinds : Array ArgKind
  /-- For `.decRec` args: the field tree. -/
  trees : Array (Option (Array DecFieldTree))
  /-- Extra simp/unfold hints from the command line. -/
  extras : Array Name := #[]
  resTy : Expr
  legs : Array (List LegStep)
  legTys : Array Expr
  /-- Function-output modules (`leader_election`): the common leg
  prefix reaching the function, and the number of REAL module args —
  `args[realCount:]` are the function's binders (wires), applied after
  `legPre`. -/
  legPre : List LegStep := []
  realCount : Nat := 0

/-- Open module `M`'s signature at the corner and run `k`. -/
def withModCtx (M : Name) (k : ModCtx → TermElabM α)
    (extras : Array Name := #[]) : TermElabM α := do
  let info ← getConstInfo M
  let hIdx ← forallTelescope info.type fun xs _ => do
    let mut r := none
    for h : i in [0:xs.size] do
      if r.isNone then
        let t ← inferType xs[i]
        if t.getAppFn.isConstOf ``HydroSem then r := some i
    match r with
    | some i => pure i
    | none => throwError "HydroGen: {M} has no `HydroSem` binder"
  forallBoundedTelescope info.type (some hIdx) fun ctxArgs tyH => do
  let hTy := tyH.bindingDomain!
  let L := hTy.getArg! 0
  let memE := hTy.getArg! 1
  let Lstx ← exprToSyntax L
  let memStx ← exprToSyntax memE
  let pacingTy ← elabType
    (← `(Pacing $Lstx $memStx))
  withLocalDecl `pacing .implicit pacingTy fun pacing => do
  withLocalDecl `Tc .implicit (mkConst ``Nat) fun Tc => do
  withLocalDecl `Td .implicit (mkConst ``Nat) fun Td => do
  withLocalDecl `hjT .implicit (← mkAppM ``Nat.le #[Tc, Td]) fun hjT => do
  let corner := mkAppN (mkConst ``CoupleSem) #[L, memE, pacing, Tc, Td, hjT]
  let sched := mkAppN (mkConst ``SchedSem) #[L, memE, pacing]
  let values := mkAppN (mkConst ``Values) #[L, memE]
  let rest := tyH.bindingBody!.instantiate1 corner
  forallTelescope rest fun args resTy => do
    let mut kinds := #[]
    let mut trees := #[]
    for a in args do
      let kk ← classifyArg corner (← inferType a)
      kinds := kinds.push kk
      match kk with
      | .decRec _ => trees := trees.push (some (← decFieldTreeOf corner a))
      | _ => trees := trees.push none
    let legsT ← peelLegsTy resTy []
    -- function-output modules: introduce the function binders as wire
    -- args and re-peel the applied result
    if h : legsT.size = 1 then
      let (path0, leafTy) := legsT[0]
      if leafTy.isForall then
        return ← forallTelescope leafTy fun xtras resTy' => do
          let mut kinds := kinds
          let mut trees := trees
          for x in xtras do
            let kk ← classifyArg corner (← inferType x)
            kinds := kinds.push kk
            trees := trees.push none
          let legsT' ← peelLegsTy resTy' []
          k { mName := M, ctxArgs, L, memE, pacing, Tc, Td, hjT,
              corner, sched, values, args := args ++ xtras, kinds,
              trees, extras, resTy := resTy',
              legs := legsT'.map (·.1), legTys := legsT'.map (·.2),
              legPre := path0, realCount := args.size }
    k { mName := M, ctxArgs, L, memE, pacing, Tc, Td, hjT, corner,
        sched, values, args, kinds, trees, extras, resTy,
        legs := legsT.map (·.1), legTys := legsT.map (·.2),
        realCount := args.size }

/-- The module applied at an interpretation with mapped args, at leg
`leg` (function-output modules apply the trailing mapped args after
`legPre`). -/
def ModCtx.legAppAt (mc : ModCtx) (interp : Expr)
    (mapped : Array Expr) (leg : List LegStep) : MetaM Expr := do
  let app := mkAppN (mkConst mc.mName)
    (mc.ctxArgs ++ #[interp] ++ mapped[0:mc.realCount])
  let app ← applyLeg app mc.legPre
  let app := mkAppN app mapped[mc.realCount:mapped.size]
  applyLeg app leg

def ModCtx.appCorner (mc : ModCtx) : Expr :=
  mkAppN (mkConst mc.mName) (mc.ctxArgs ++ #[mc.corner] ++ mc.args)

def ModCtx.srArgs (mc : ModCtx) : TermElabM (Array Expr) := do
  let mut out := #[]
  for a in mc.args, k in mc.kinds do
    match k with
    | .wire _ => out := out.push (← mkProjection a `sr)
    | .primDec f =>
      out := out.push (if f.schedPass then a else mkConst ``Unit.unit)
    | .decRec _ =>
      out := out.push (← repackRec mc.corner mc.sched
        (fun f p => pure (if f.schedPass then p else mkConst ``Unit.unit)) a)
    | .plain => out := out.push a
  pure out

def ModCtx.rrArgsDirect (mc : ModCtx) : TermElabM (Array Expr) := do
  let valuesMode : DecFam → Expr → TermElabM Expr := fun f p => do
    match f with
    | .fixd => mkAppM ``HAdd.hAdd #[mc.Td, mkNatLit 1]
    | _ =>
      if f.valuesPass then pure p
      else pure (mkConst ``Unit.unit)
  let mut out := #[]
  for a in mc.args, k in mc.kinds do
    match k with
    | .wire _ => out := out.push (← mkProjection a `rr)
    | .primDec .fixd =>
      out := out.push (← mkAppM ``HAdd.hAdd #[mc.Td, mkNatLit 1])
    | .primDec f =>
      out := out.push (if f.valuesPass then a else mkConst ``Unit.unit)
    | .decRec _ =>
      out := out.push (← repackRec mc.corner mc.values valuesMode a)
    | .plain => out := out.push a
  pure out

/-- The module's `requires` clause at the corner's inputs: wires at
their denotation legs, plain args as they are; decision binders get
fresh placeholders — the clause may not mention them (it is a fact
about CONTENT), and a placeholder surviving the instantiation is the
error that says so. -/
def ModCtx.requiresAt (mc : ModCtx) (fn : Expr) : TermElabM Expr := do
  let mut out : Array Expr := #[]
  for a in mc.args, k in mc.kinds do
    match k with
    | .wire _ => out := out.push (← mkProjection a `rr)
    | .plain => out := out.push a
    | _ =>
      let ty := reinterp mc.corner mc.values (← inferType a)
      out := out.push (← mkFreshExprMVar ty)
  let r := (fn.beta (mc.ctxArgs ++ out)).headBeta
  let r ← instantiateMVars r
  if r.hasExprMVar then
    throwError "HydroGen: {mc.mName}: the `requires` clause mentions a decision \
      argument — a clause is a fact about the inputs' content only"
  pure r

/-- Content units: args whose `Values`-side data the corner derives —
content primitive decisions and content-carrying decision records. -/
def ModCtx.contentDecs (mc : ModCtx) : Array Nat := Id.run do
  let mut out := #[]
  for i in [0:mc.args.size] do
    match mc.kinds[i]! with
    | .primDec f => if f.isContent then out := out.push i
    | .decRec _ =>
      if treeHasContent (mc.trees[i]!).get! then out := out.push i
    | _ => pure ()
  pure out

def ModCtx.allBinders (mc : ModCtx) : Array Expr :=
  mc.ctxArgs ++ #[mc.pacing, mc.Tc, mc.Td, mc.hjT] ++ mc.args

/-- Callee modules with registered generation info. When the module
has a recorded composition spec (`hydro def`/`hydro_register`), the
spec's callee list is authoritative (no scanning). Otherwise fall
back to a transitive scan through unregistered `Hydro` glue defs
(fallback for ad-hoc/unregistered modules). -/
def calleeInfos (M : Name) : MetaM (Array (Name × GenInfo)) := do
  let env ← getEnv
  if let some gi := getGenInfo env M then
    if gi.hasSpec then
      return gi.callees.filterMap fun c =>
        (getGenInfo env c).map fun i => (c, i)
  let mut seen : NameSet := {}
  let mut queue : Array (Name × Nat) := #[(M, 0)]
  let mut out := #[]
  let mut qi := 0
  while h : qi < queue.size do
    let (cur, depth) := queue[qi]
    qi := qi + 1
    let some info := env.find? cur | continue
    let some v := info.value? | continue
    for c in v.getUsedConstants do
      if !seen.contains c then
        seen := seen.insert c
        if let some gi := getGenInfo env c then
          out := out.push (c, gi)
        else if depth < 3 && (`Hydro).isPrefixOf c then
          if let some ci := env.find? c then
            if ci matches .defnInfo _ then
              queue := queue.push (c, depth + 1)
  pure out

/-- The callee naming rules (sr+rr lemma names) for scripts. -/
def calleeNamingRules (M : Name) : MetaM (Array Name) := do
  let cs ← calleeInfos M
  pure <| cs.foldl (init := #[]) fun acc (_, gi) =>
    acc ++ gi.srLemmas ++ gi.rrLemmas

def rulesSuffix (rules : Array Name) : String :=
  String.join (rules.toList.map fun r => s!", {r}")

/-! ## `hydro def`: the composition spec, recorded at definition time

The module grammar: a body is lets/tuples of core-operator
applications (`H.map`, `H.zipTick`, …), applications of registered
hydro modules, `HydroSem.fix`/`fixTick` knots, and inline wrappers
(`hydro_inline`) — over data (interp-free terms) and ghost contracts
(proofs, skipped wholesale). Anything else that touches the interp is
rejected HERE, at the definition, instead of surfacing later as a
generator failure on a downstream module. -/

structure SpecState where
  callees : NameSet := {}
  inlines : NameSet := {}
  hasFix : Bool := false

private def specAllowedHeads : NameSet :=
  ({} : NameSet)
    |>.insert ``letFun |>.insert ``ite |>.insert ``dite
    |>.insert ``cond |>.insert ``id

/-- Walk a module body, validating the grammar and collecting the
spec. `interps` are the interp-typed locals in scope (the module's
`H`, plus any `H'` bound by polymorphic fix bodies). Relevance is
type-based: interp-generic sub-values (partially-applied modules,
polymorphic fix bodies) carry no interp fvar, but their types mention
`HydroSem`; plain data (`q!` closures) does not and is skipped. -/
partial def specWalk (inlineSet : NameSet) (interps : Array FVarId)
    (e : Expr) : StateRefT SpecState MetaM Unit := do
  if ← Meta.isProof e then return
  let ty ← Meta.inferType e
  if ty.isSort then return
  unless ty.hasAnyFVar (fun id => interps.contains id)
      || (ty.find? (·.isConstOf ``HydroSem)).isSome
      || e.hasAnyFVar (fun id => interps.contains id) do
    return
  match e with
  | .mdata _ b => specWalk inlineSet interps b
  | .proj _ _ s => specWalk inlineSet interps s
  | .lam .. =>
    Meta.lambdaBoundedTelescope e 1 fun xs b => do
      let mut interps := interps
      for x in xs do
        if (← Meta.inferType x).getAppFn.isConstOf ``HydroSem then
          interps := interps.push x.fvarId!
      specWalk inlineSet interps b
  | .forallE .. =>
    Meta.forallBoundedTelescope e (some 1) fun xs b => do
      let mut interps := interps
      for x in xs do
        if (← Meta.inferType x).getAppFn.isConstOf ``HydroSem then
          interps := interps.push x.fvarId!
      specWalk inlineSet interps b
  | .letE n t v b _ => do
    specWalk inlineSet interps v
    Meta.withLetDecl n t v fun x =>
      specWalk inlineSet interps (b.instantiate1 x)
  | .app .. => do
    let e' := e.headBeta
    unless e'.isApp do return (← specWalk inlineSet interps e')
    let f := e'.getAppFn
    let args := e'.getAppArgs
    match f with
    | .fvar .. | .bvar .. =>
      for a in args do specWalk inlineSet interps a
    | .const c _ => do
      let env ← getEnv
      if c == ``HydroSem.fix || c == ``HydroSem.fixTick then
        modify fun s => { s with hasFix := true }
        for a in args do specWalk inlineSet interps a
      else if c.getPrefix == ``HydroSem then
        -- core operator (or carrier type): structure field apps
        for a in args do specWalk inlineSet interps a
      else if (getGenInfo env c).isSome then
        modify fun s => { s with callees := s.callees.insert c }
        for a in args do specWalk inlineSet interps a
      else if inlineSet.contains c then
        modify fun s => { s with inlines := s.inlines.insert c }
        -- open the wrapper and walk its interior (its callees are
        -- this module's callees)
        let some ci := env.find? c
          | throwError "hydro def: inline {c} not found"
        let some v := ci.value?
          | throwError "hydro def: inline {c} has no value"
        specWalk inlineSet interps (v.beta args)
      else if (← getProjectionFnInfo? c).isSome then
        for a in args do specWalk inlineSet interps a
      else if env.find? c matches some (.ctorInfo _) then
        for a in args do specWalk inlineSet interps a
      else if specAllowedHeads.contains c || c.isInternalDetail then
        for a in args do specWalk inlineSet interps a
      else
        throwError "hydro def: unrecognized invocation head `{c}` in\
          {indentExpr e}\n(register it with `hydro_register`/\
          `hydro def`, or mark it `hydro_inline`)"
    | _ =>
      for a in args do specWalk inlineSet interps a
  | _ => return

/-- Build and record the composition spec for a module definition. -/
def registerSpec (M : Name) : TermElabM Unit := do
  let env ← getEnv
  let some info := env.find? M
    | throwError "hydro def: {M} not found"
  let some v := info.value?
    | throwError "hydro def: {M} has no value"
  let inlineSet := inlineRegistry.getState env
  let walk : StateRefT SpecState MetaM Unit :=
    Meta.lambdaTelescope v fun xs b => do
      let mut interps : Array FVarId := #[]
      for x in xs do
        if (← Meta.inferType x).getAppFn.isConstOf ``HydroSem then
          interps := interps.push x.fvarId!
      specWalk inlineSet interps b
  let (_, st) ← walk.run {}
  let cur := (getGenInfo env M).getD {}
  modifyEnv fun env => genRegistry.addEntry env (M,
    { cur with callees := st.callees.toArray,
               inlines := st.inlines.toArray,
               hasFix := st.hasFix, hasSpec := true })

/-- Per-module extra simp/unfold hints (e.g. custom `DecidableEq`
instances the module's matches scrutinize), supplied by the module
annotation: `hydro [p1bPairDecEq] def M …`. -/
structure GenExtras where
  extras : Array Name := #[]

/-! ## The couple phase: the naming stack -/

/-- The choice triple for modules with one content unit (a content
primitive decision or a content-carrying decision record):
`M_co_rr_ex`, `M_vdec := Classical.choose …`, per-leg
`M_co_rrᵢ := (Classical.choose_spec …).ᵢ`, and named fuel pins. -/
def genRrEx (mc : ModCtx) :
    TermElabM (Name × Name × Array Name × Array Name) := do
  let content := mc.contentDecs
  unless content.size == 1 do
    throwError "HydroGen: {mc.mName}: exactly one content unit \
      supported (found {content.size})"
  let cIdx := content[0]!
  let rules ← calleeNamingRules mc.mName
  -- outer machine-data binders: one per wire (its machine leg), one
  -- per machine-data decision, one per machine-data FIELD of a
  -- decision record (flattened paths)
  let mut outerReqs : Array (Nat × List Name × Name × Expr) := #[]
  for i in [0:mc.args.size] do
    match mc.kinds[i]! with
    | .wire cf =>
      outerReqs := outerReqs.push
        (i, [], Name.mkSimple s!"x{i}",
         reinterp mc.corner mc.sched (← inferType mc.args[i]!))
      -- a fold singleton's corner carrier also remembers its machine
      -- SOURCE (the wire the derived read cuts are computed from):
      -- one more machine-data binder, pinned to the carrier's `src`
      if cf == .sing then
        let sty ← inferType mc.args[i]!
        let a := sty.getAppArgs
        -- `HydroSem.Singleton L mem H ℓ α σ inst ord ret b`
        let srcTy := mkAppN (mkConst ``HydroSem.Stream)
          #[a[0]!, a[1]!, mc.sched, a[3]!, a[4]!, a[6]!, a[7]!, a[8]!]
        outerReqs := outerReqs.push
          (i, [`src], Name.mkSimple s!"x{i}_src", srcTy)
    | .primDec f =>
      if f.schedPass && !f.isContent then
        outerReqs := outerReqs.push
          (i, [], Name.mkSimple s!"d{i}",
           reinterp mc.corner mc.sched (← inferType mc.args[i]!))
    | .decRec _ =>
      for (path, f) in flattenTree (mc.trees[i]!).get! do
        if f.schedPass && !f.isContent then
          let pv ← applyFieldPath mc.args[i]! path
          let nm := Name.mkSimple
            (s!"d{i}_" ++ String.intercalate "_" (path.map toString))
          outerReqs := outerReqs.push
            (i, path, nm, reinterp mc.corner mc.sched (← inferType pv))
    | .plain => pure ()
  let plainArgs := (mc.args.zip mc.kinds).filterMap fun (a, k) =>
    if k == .plain then some a else none
  withLocalDecls
      (outerReqs.map fun (_, _, n, ty) =>
        (n, BinderInfo.default, fun _ => pure ty))
      fun outers => do
  -- lookup by (argIdx, path)
  let outerIdxOf : Std.HashMap (Nat × List Name) Nat := Id.run do
    let mut m := {}
    for h : j in [0:outerReqs.size] do
      let r := outerReqs[j]
      m := m.insert (r.1, r.2.1) j
    pure m
  let outerAt := fun (i : Nat) (p : List Name) =>
    (outerIdxOf.get? (i, p)).map fun j => outers[j]!
  -- the ∃-bound Values decision (unit cIdx)
  let vDecTy := reinterp mc.corner mc.values (← inferType mc.args[cIdx]!)
  withLocalDecl `dV .default vDecTy fun dV => do
  -- the inner ∀: lowered couple horizon + re-bound non-plain args
  withLocalDecl `Tc' .default (mkConst ``Nat) fun Tc' => do
  withLocalDecl `h'
      .default (← mkAppM ``Nat.le #[Tc', mc.Td]) fun h' => do
  let corner' := mkAppN (mkConst ``CoupleSem)
    #[mc.L, mc.memE, mc.pacing, Tc', mc.Td, h']
  let mut innerReqs : Array (Nat × Name × Expr) := #[]
  for i in [0:mc.args.size] do
    if mc.kinds[i]! != .plain then
      innerReqs := innerReqs.push
        (i, Name.mkSimple s!"w{i}",
         reinterp mc.corner corner' (← inferType mc.args[i]!))
  withLocalDecls
      (innerReqs.map fun (_, n, ty) =>
        (n, BinderInfo.default, fun _ => pure ty))
      fun inners => do
  let innerOf : Std.HashMap Nat Expr := Id.run do
    let mut m := {}
    for r in innerReqs, o in inners do
      m := m.insert r.1 o
    pure m
  let innerArg := fun (i : Nat) => (innerOf.get? i).getD mc.args[i]!
  -- pins: wire machine legs, machine-data decisions, and record
  -- machine-data fields equal the outer binders
  let mut pinReqs : Array (Name × Expr) := #[]
  for h : j in [0:outerReqs.size] do
    let (i, path, nm, _) := outerReqs[j]
    let lhs ← match mc.kinds[i]! with
      | .wire _ => mkProjection (innerArg i) (if path == [`src] then `src else `sr)
      | .decRec _ => applyFieldPath (innerArg i) path
      | _ => pure (innerArg i)
    pinReqs := pinReqs.push
      (nm.appendBefore "hp_", ← mkEq lhs outers[j]!)
  withLocalDecls
      (pinReqs.map fun (n, ty) =>
        (n, BinderInfo.default, fun _ => pure ty))
      fun pins => do
  -- conclusion: per-leg equations
  let innerAll := (Array.range mc.args.size).map innerArg
  let mut valuesArgs := #[]
  for i in [0:mc.args.size] do
    let v ← match mc.kinds[i]! with
      | .wire _ => mkProjection (innerArg i) `rr
      | .primDec .fixd => mkAppM ``HAdd.hAdd #[mc.Td, mkNatLit 1]
      | .primDec f =>
        if f.isContent then pure dV
        else if f.valuesPass then pure (outerAt i []).get!
        else pure (mkConst ``Unit.unit)
      | .decRec _ =>
        if i == cIdx then pure dV
        else
          -- non-content record: Values repack with pass fields from
          -- the outers
          let mut fieldIdx := (0 : Nat)
          let outerFieldOf : Std.HashMap (List Name) Expr := Id.run do
            let mut m := {}
            for (path, f) in flattenTree (mc.trees[i]!).get! do
              let _ := f
              if let some o := outerAt i path then
                m := m.insert path o
            pure m
          let _ := fieldIdx
          let _ := outerFieldOf
          repackRec corner' mc.values
            (fun f p => do
              match f with
              | .fixd => mkAppM ``HAdd.hAdd #[mc.Td, mkNatLit 1]
              | _ =>
                if f.valuesPass then pure p
                else pure (mkConst ``Unit.unit))
            (innerArg i)
      | .plain => pure mc.args[i]!
    valuesArgs := valuesArgs.push v
  let mut legEqs := #[]
  for leg in mc.legs do
    let lhs ← mkProjection (← mc.legAppAt corner' innerAll leg) `rr
    let rhs ← mc.legAppAt mc.values valuesArgs leg
    legEqs := legEqs.push (← mkEq lhs rhs)
  -- fixd fields of a content record are carried but unused by the
  -- body (the knots consume them): pin them to `Td + 1` by explicit
  -- trailing conjuncts (closed by `rfl`-unification), so no witness
  -- hole is left unassigned
  let mut fuelPins := 0
  if let .decRec _ := mc.kinds[cIdx]! then
    for (path, f) in flattenTree (mc.trees[cIdx]!).get! do
      if f == .fixd then
        let lhs ← applyFieldPath dV path
        legEqs := legEqs.push
          (← mkEq lhs (← mkAppM ``HAdd.hAdd #[mc.Td, mkNatLit 1]))
        fuelPins := fuelPins + 1
    -- likewise, decision fields the module body never projects (a
    -- record carried only for a construct's own decision — e.g. an
    -- enclosed knot's fuel): unused ⇒ identity coupling — pin the
    -- witness hole to a default inhabitant so it is not left
    -- unassigned (soft: fields without `Inhabited` stay quantified).
    -- Only the TOP-LEVEL projection is sound evidence: a used
    -- sub-record may flow whole into a callee, whose own leaf
    -- projections are invisible to the body's constant census.
    if let .decRec _ := mc.kinds[cIdx]! then
      let usedConsts : Lean.NameSet :=
        match (← getConstInfo mc.mName).value? with
        | some v => v.getUsedConstants.foldl (·.insert ·) {}
        | none => {}
      for (path, f) in flattenTree (mc.trees[cIdx]!).get! do
        unless f == .fixd do
          let srcTop ← applyFieldPath mc.args[cIdx]! (path.take 1)
          let projName? : Option Name ← match srcTop with
            | .proj s i _ =>
              pure (((getStructureFields (← getEnv) s)[i]?).map (s ++ ·))
            | e => pure e.getAppFn.constName?
          let used := match projName? with
            | some pn => usedConsts.contains pn
            | none => true
          unless used do
          let lhs ← applyFieldPath dV path
          let dflt? ← try
              pure (some (← mkAppOptM ``Inhabited.default
                #[some (← whnf (← inferType lhs)), none]))
            catch _ => pure (none : Option Expr)
          match dflt? with
          | some dflt =>
            legEqs := legEqs.push (← mkEq lhs dflt)
            fuelPins := fuelPins + 1
          | none =>
            logInfo m!"hydro_couple {mc.mName}: unused dec field \
              {path} has no Inhabited default — left ∃-quantified"
  let conj ← mkAndN legEqs
  let innerForall ← mkForallFVars (#[Tc', h'] ++ inners ++ pins) conj
  let exStmtBody ← mkAppM ``Exists #[← mkLambdaFVars #[dV] innerForall]
  let exBinders := mc.ctxArgs ++ #[mc.pacing, mc.Td] ++ plainArgs ++ outers
  let exStmt ← mkForallFVars exBinders exStmtBody
  -- proof script
  let introNames := String.intercalate " "
    (["Tc'", "h'"]
      ++ (innerReqs.toList.map fun r => s!"{r.2.1}")
      ++ (pinReqs.toList.map fun (n, _) => s!"{n}"))
  let totalGoals := mc.legs.size + fuelPins
  let refinePart :=
    if totalGoals == 1 then "("
    else "refine ⟨" ++ String.intercalate ", "
      (List.replicate totalGoals "?_") ++ "⟩ <;> ("
  let pinRws := String.join (pinReqs.toList.map fun (n, _) =>
    s!"(try rw [{n}]); ")
  -- callee namings on the corner side by the glue engine
  -- (`co_residuals`, HydroGenKnot): the redexes simp's congruence
  -- cannot reach, without ever unifying against the witness holes on
  -- the `Values` side; the closing `rfl` then only assigns the holes
  let calleeRws :=
    if rules.isEmpty then ""
    else s!"co_residuals [{(rulesSuffix rules).drop 2}]; "
  -- the ∃-witness skeleton: plain hole for a primitive decision,
  -- nested constructor skeleton for a record (projection constraints
  -- are not unifiable — the skeleton exposes per-field holes)
  let witness := match mc.kinds[cIdx]! with
    | .decRec _ => skeletonOf (mc.trees[cIdx]!).get!
    | _ => "_"
  -- diagnostic mode: no closer — the normalized goal (with the
  -- witness holes) prints as the unsolved goal
  let diagEnv ← IO.getEnv "HYDROGEN_SR_DIAG"
  let closer := if diagEnv.isSome then "" else "(try exact rfl)"
  let script :=
    s!"(intros; apply Exists.intro (({witness})); intro {introNames}; {refinePart}" ++
    s!"(try co_simp [{mc.mName}{rulesSuffix rules}{rulesSuffix mc.extras}]); " ++
    s!"{calleeRws}{pinRws}{closer}))"
  let exName := mc.mName.appendAfter "_co_rr_ex"
  addThmByTac exName exStmt script
  -- the choice-named decision
  let exApp := mkAppN (mkConst exName) exBinders
  let vdecName := mc.mName.appendAfter "_vdec"
  let vdecTyAll ← mkForallFVars exBinders vDecTy
  let vdecVal ← mkLambdaFVars exBinders
    (← mkAppM ``Classical.choose #[exApp])
  addDefn vdecName vdecTyAll vdecVal
  -- per-leg rr namings from choose_spec
  let mut rrNames := #[]
  let spec ← mkAppM ``Classical.choose_spec #[exApp]
  -- instantiate the spec at the corner args (outer values := machine
  -- projections of the corner args; pins by rfl)
  let mut specOuterVals : Array Expr := #[]
  for r in outerReqs do
    let (i, path, _, _) := r
    let v ← match mc.kinds[i]! with
      | .wire _ => mkProjection mc.args[i]! (if path == [`src] then `src else `sr)
      | .decRec _ => applyFieldPath mc.args[i]! path
      | _ => pure mc.args[i]!
    specOuterVals := specOuterVals.push v
  let specInst := (← mkLambdaFVars outers spec).beta specOuterVals
  let specApp0 := mkAppN specInst
    (#[mc.Tc, mc.hjT] ++ (innerReqs.map fun r => mc.args[r.1]!))
  let mut pinProofs := #[]
  for v in specOuterVals do
    pinProofs := pinProofs.push (← mkEqRefl v)
  let specApp := mkAppN specApp0 pinProofs
  for i in [0:mc.legs.size] do
    let prf ← andProj specApp i (mc.legs.size + fuelPins)
    -- statement: the corner-args instance of the leg equation, with
    -- the choice term spelled via the *named* `M_vdec` (definitional)
    let ty := (← inferType prf).replace fun e =>
      if e.isAppOfArity ``Classical.choose 3 then
        let arg := e.appArg!
        if arg.getAppFn.isConstOf exName then
          some (mkAppN (mkConst vdecName) arg.getAppArgs)
        else none
      else none
    let name := mc.mName.appendAfter s!"_co_rr{subscript (i + 1)}"
    let stmt ← mkForallFVars mc.allBinders ty
    let val ← mkLambdaFVars mc.allBinders prf
    addThm name stmt val
    rrNames := rrNames.push name
  -- named fuel pins (the knot route's `Values`-fix fuel legs)
  let mut pinNames := #[]
  for k in [0:fuelPins] do
    let prf ← andProj specApp (mc.legs.size + k) (mc.legs.size + fuelPins)
    let ty := (← inferType prf).replace fun e =>
      if e.isAppOfArity ``Classical.choose 3 then
        let arg := e.appArg!
        if arg.getAppFn.isConstOf exName then
          some (mkAppN (mkConst vdecName) arg.getAppArgs)
        else none
      else none
    let name := mc.mName.appendAfter s!"_vdec_fuel{subscript (k + 1)}"
    let stmt ← mkForallFVars mc.allBinders ty
    let val ← mkLambdaFVars mc.allBinders prf
    addThm name stmt val
    pinNames := pinNames.push name
  pure (exName, vdecName, rrNames, pinNames)

/-! ## The causal phase: per-leg step causality over the machine legs -/

/-- The agreement relation for a carrier family (the `SchedCausal`
vocabulary). -/
def agreeRelOf : CarrierFam → Name
  | .stream => ``SAgree
  | .keyed => ``SAgree
  | .sing => ``FAgree
  | .ksing => ``FAgree
  | .atomic => ``TAgree
  | .atomicStream => ``TAgree
  | .atomicKSing => ``TAgree
  | .atomicKeyed => ``TAgree
  -- one tick's content is never a module wire (it lives inside tick
  -- bodies); plain equality if it ever is
  | .bounded => ``Eq
  | .bsing => ``Eq
  | .bopt => ``Eq
  | .bkeyed => ``Eq
  | .bksing => ``Eq

/-- The terminal subject key under literal projections: descend
through `Prod.fst`/`Prod.snd` applications and native `.proj` steps
(recording the projection path), through `Subtype.val` transparently
(the `.val` coercion on ensures-faced callees) and through the member
read `MemberMap.get m i` (D73: a `Values` carrier is a `MemberMap`, so
a relation's subject `(K …).2.1 i` is `MemberMap.get (K …).2.1 i` — the
read is transparent, the index is dropped, and the key is the carrier's,
exactly as the bare application `(K …).2.1 i` keyed before);
stop at the first non-projection head constant. `none` for anything
uncertain (mvar/fvar/lambda heads). -/
partial def subjKeyOf (e : Expr) (path : List Nat := []) :
    Option (Name × List Nat) :=
  match e.getAppFn with
  | .const n _ =>
    let args := e.getAppArgs
    if n == ``Prod.fst && args.size ≥ 3 then
      subjKeyOf args[2]! (0 :: path)
    else if n == ``Prod.snd && args.size ≥ 3 then
      subjKeyOf args[2]! (1 :: path)
    else if n == ``Subtype.val && args.size ≥ 3 then
      subjKeyOf args[2]! path
    else if (n == ``MemberMap.get || n == ``MemberMap.cells) && args.size ≥ 3 then
      subjKeyOf args[2]! path
    else some (n, path)
  | .proj S i s =>
    if S == ``Subtype && i == 0 then subjKeyOf s path
    else if S == ``MemberMap && i == 0 then subjKeyOf s path
    else subjKeyOf s (i :: path)
  | .mdata _ b => subjKeyOf b path
  | _ => none

/-- `subjKeyOf` modulo `@[reducible]` heads (D72): a knot's generic
body constant (`<K>.body`, hoisted `@[reducible]` so the `fix` laws see
through it) keys as what it unfolds to — the chain module's leg — since
`with_reducible apply` sees through it too. -/
partial def subjKeyOfM (e : Expr) (path : List Nat := []) :
    MetaM (Option (Name × List Nat)) := do
  let e := e.headBeta
  match e.getAppFn with
  | .const n _ =>
    let args := e.getAppArgs
    if n == ``Prod.fst && args.size ≥ 3 then
      subjKeyOfM args[2]! (0 :: path)
    else if n == ``Prod.snd && args.size ≥ 3 then
      subjKeyOfM args[2]! (1 :: path)
    else if n == ``Subtype.val && args.size ≥ 3 then
      subjKeyOfM args[2]! path
    else if (n == ``MemberMap.get || n == ``MemberMap.cells) && args.size ≥ 3 then
      subjKeyOfM args[2]! path
    else if (← getReducibilityStatus n) == .reducible then
      match ← unfoldDefinition? e with
      | some e' => subjKeyOfM e' path
      | none => pure (some (n, path))
    else pure (some (n, path))
  | .proj S i s =>
    if S == ``Subtype && i == 0 then subjKeyOfM s path
    else if S == ``MemberMap && i == 0 then subjKeyOfM s path
    else subjKeyOfM s (i :: path)
  | .mdata _ b => subjKeyOfM b path
  | _ => pure none

/-- Keyed rule application (D72): `apply` a callee rule only when its
conclusion CAN match the goal without unfolding a folded callee — same
relation head and arity, and every argument pair whose terminal keys
(`subjKeyOf`: head constant + literal projection path) are both known
agrees on the key. Rules that fail the key test are skipped before any
unification.

Why not `first | with_reducible apply r₁ | … | with_reducible apply rₙ`
(the walks' shape through D71): a rule whose conclusion is
`(K ?args).2.2.1` against a goal `(K args).2.2.2.1` FAILS to unify — but
the unifier only learns that at the bottom of the shared `Prod.snd`
spine, and on the way back up it retries every level in its unfolded
`.proj` form (`Prod.snd` is reducible), so a failing rule costs ~1.7×
per shared projection level: body_smono₁₄ against the 16-leg
`leader_election.body`'s leg 15 took 7 s to fail, body_smono₁₀ 0.8 s —
and a walk tries every rule in order. Failures are not cached when the
rule side carries metavariables. The key test is O(spine depth). -/
elab "hydro_rule_apply" "[" rules:ident,* "]" : tactic =>
  open Elab.Tactic in do
  let g ← getMainGoal
  let gt ← instantiateMVars (← g.getType)
  -- the goal's relation at reducible transparency (a shaped leaf's
  -- `TAgreeOf h (.stream …) x y` IS `TAgree h x y`)
  let gt ← g.withContext <| withReducible <| whnf gt.consumeMData.headBeta
  -- a relation's head and argument list, with the two spellings of a
  -- structure projection (`S.f s` application / native `.proj S i s`)
  -- read as one head `S.f` over the single argument `s`
  let relHeadArgs : Expr → MetaM (Option (Name × Array Expr)) := fun e => do
    match e with
    | .proj S i s =>
      let fields := getStructureFields (← getEnv) S
      pure (fields[i]?.map fun f => (S ++ f, #[s]))
    | _ =>
      let .const h _ := e.getAppFn | pure none
      if let some pi ← getProjectionFnInfo? h then
        let args := e.getAppArgs
        pure (args[pi.numParams]?.map fun s => (h, #[s]))
      else pure (some (h, e.getAppArgs))
  let gRel ← g.withContext <| relHeadArgs gt
  let gKeys ← g.withContext do
    match gRel with
    | some (h, args) => pure (some (h, ← args.mapM (subjKeyOfM ·)))
    | none => pure none
  for r in rules.getElems do
    let n ← realizeGlobalConstNoOverloadWithInfo r
    let info ← getConstInfo n
    -- key test (no unification): a known mismatch — same relation head
    -- and arity, two known subject keys that differ — is skipped;
    -- anything uncertain falls through to `apply`
    let skip ← g.withContext do
      let some (hg, gks) := gKeys | pure false
      let (_, _, concl) ← forallMetaTelescope info.type
      let some (hl, cArgs) ← relHeadArgs concl.consumeMData | pure false
      unless hl == hg do return false
      unless cArgs.size == gks.size do return false
      for al in cArgs, kg? in gks do
        if let (some kl, some kg) := (← subjKeyOfM al, kg?) then
          if kl != kg then return true
      return false
    let diag := (← IO.getEnv "HYDROGEN_RULE_DIAG").isSome
    if diag then
      let (_, _, concl) ← forallMetaTelescope info.type
      let cRel ← relHeadArgs concl.consumeMData
      let cks ← match cRel with
        | some (h, args) => do pure s!"{h} {repr (← args.mapM (subjKeyOfM ·))}"
        | none => pure "none"
      let gks := match gKeys with
        | some (h, ks) => s!"{h} {repr ks}"
        | none => "none"
      IO.eprintln s!"[rule] {n}: goal keys {gks} | rule keys {cks}"
    if skip then
      if diag then IO.eprintln s!"[rule] {n}: key-skipped"
      continue
    let s ← saveState
    let t0 ← IO.monoMsNow
    try
      let gs ← g.withContext <| withReducible do
        g.apply (← mkConstWithFreshMVarLevels n) { synthAssignedInstances := false }
      if diag then IO.eprintln s!"[rule] {n}: applied {(← IO.monoMsNow) - t0}ms"
      replaceMainGoal gs
      return
    catch _ =>
      if diag then IO.eprintln s!"[rule] {n}: apply FAILED {(← IO.monoMsNow) - t0}ms"
      s.restore
  throwError "hydro_rule_apply: no rule applies"

/-- The `co_causal_walk` recipe (public copy of the house pattern):
refl/assumption/mono leaves, caller rules, then the per-op head
dispatch. -/
macro "hydro_causal_walk" "[" rules:ident,* "]" : tactic => do
  let userRws : Array (Lean.TSyntax `tactic) ←
    if rules.getElems.isEmpty then pure #[] else do
      pure #[← `(tactic| hydro_rule_apply [$rules,*])]
  let base : Array (Lean.TSyntax `tactic) := #[
    ← `(tactic| with_reducible assumption),
    ← `(tactic| with_reducible exact TAgree.refl _ _),
    ← `(tactic| with_reducible exact SAgree.refl _ _),
    ← `(tactic| with_reducible exact FAgree.refl _ _),
    ← `(tactic| with_reducible exact VAgree.refl _ _)]
  let steps : Array (Lean.TSyntax `tactic) := #[
    ← `(tactic| co_causal_step),
    -- shaped tick tuples: leafwise agreement splits; constructor
    -- projections reduce
    ← `(tactic| (with_reducible refine And.intro ?_ ?_)),
    -- a program-level `if` over static parameters
    ← `(tactic| split),
    ← `(tactic| dsimp only [])]
  let alts := base ++ userRws ++ steps
  let failTac ← `(tactic| fail "hydro_causal_walk: no rule applies")
  let alt ← alts.foldrM (init := failTac) fun t acc =>
    `(tactic| first | $t:tactic | $acc:tactic)
  `(tactic| repeat' $alt:tactic)

/-- Per-leg step-causality congruences `M_causal₁ …`: agreement below
`h` on the wire inputs transports through the module's `SchedSem`
legs. -/
def genCausal (mc : ModCtx) : TermElabM (Array Name) := do
  withLocalDecl `h .implicit (mkConst ``Nat) fun hvar => do
  -- machine-typed args; wires doubled (implicit) + agreement hyps;
  -- plain args stay shared (their fvars appear in dependent types)
  let mut decls : Array (Nat × Name × BinderInfo × Expr) := #[]
  for i in [0:mc.args.size] do
    let ty := reinterp mc.corner mc.sched (← inferType mc.args[i]!)
    let nm ← mc.args[i]!.fvarId!.getUserName
    match mc.kinds[i]! with
    | .wire _ =>
      decls := decls.push (i, nm, .implicit, ty)
      decls := decls.push (i, nm.appendAfter "'", .implicit, ty)
    | .plain => pure ()
    | _ => decls := decls.push (i, nm, .default, ty)
  withLocalDecls (decls.map fun (_, n, bi, ty) =>
      (n, bi, fun _ => pure ty)) fun xs => do
  -- index maps
  let mut fstOf : Std.HashMap Nat Expr := {}
  let mut sndOf : Std.HashMap Nat Expr := {}
  let mut j := 0
  for i in [0:mc.args.size] do
    match mc.kinds[i]! with
    | .wire _ =>
      fstOf := fstOf.insert i xs[j]!
      sndOf := sndOf.insert i xs[j+1]!
      j := j + 2
    | .plain =>
      fstOf := fstOf.insert i mc.args[i]!
      sndOf := sndOf.insert i mc.args[i]!
    | _ =>
      fstOf := fstOf.insert i xs[j]!
      sndOf := sndOf.insert i xs[j]!
      j := j + 1
  -- binders in dependency (argument) order
  let mut argBinders : Array Expr := #[]
  for i in [0:mc.args.size] do
    match mc.kinds[i]! with
    | .wire _ =>
      argBinders := argBinders.push (fstOf.get? i).get!
      argBinders := argBinders.push (sndOf.get? i).get!
    | .plain => argBinders := argBinders.push mc.args[i]!
    | _ => argBinders := argBinders.push (fstOf.get? i).get!
  -- agreement hypotheses
  let mut hypDecls : Array (Name × BinderInfo × Expr) := #[]
  for i in [0:mc.args.size] do
    if let .wire cf := mc.kinds[i]! then
      let rel ← mkAppM (agreeRelOf cf)
        #[hvar, (fstOf.get? i).get!, (sndOf.get? i).get!]
      hypDecls := hypDecls.push
        (Name.mkSimple s!"ha{i}", .default, rel)
  withLocalDecls (hypDecls.map fun (n, bi, ty) =>
      (n, bi, fun _ => pure ty)) fun hyps => do
  let sideAll := fun (side : Std.HashMap Nat Expr) =>
    (Array.range mc.args.size).map fun i => (side.get? i).get!
  let callees ← calleeInfos mc.mName
  let calleeCausal := callees.foldl (init := #[]) fun acc (_, gi) =>
    acc ++ gi.causalLemmas
  let mut names := #[]
  for i in [0:mc.legs.size] do
    let leg := mc.legs[i]!
    -- the leg's carrier family, from its type head
    let legHead := mc.legTys[i]!.getAppFn
    let cf ← match legHead with
      | .const n _ =>
        match carrierFamOfHead n with
        | some cf => pure cf
        | none => throwError "HydroGen: {mc.mName}: leg {i} is not a \
            carrier ({n})"
      | _ => throwError "HydroGen: {mc.mName}: leg {i} head not const"
    let concl ← mkAppM (agreeRelOf cf)
      #[hvar, ← mc.legAppAt mc.sched (sideAll fstOf) leg,
        ← mc.legAppAt mc.sched (sideAll sndOf) leg]
    let stmt ← mkForallFVars
      (mc.ctxArgs ++ #[mc.pacing, hvar] ++ argBinders ++ hyps) concl
    let name := mc.mName.appendAfter s!"_causal{subscript (i + 1)}"
    let ruleList := String.intercalate ", "
      (calleeCausal.toList.map toString)
    addThmByTac name stmt
      s!"(intros; simp only [{mc.mName}{rulesSuffix mc.extras}]; hydro_causal_walk [{ruleList}])"
    names := names.push name
  pure names

/-! ## The smono phase: machine tick-monotonicity (D71)

Per-leg `M_smono_i`: the machine leg of a tick-family wire is
step-monotone (`TMono`) when every tick-family wire input is. The fact
is PROVEN about `SchedSem`'s definitions (`SchedMono.lean`), never
assumed; the corner's `mono` field consumes it through the tick knot's
W4 conjunct (`genKnotWf`), where the body's monotonicity makes the
knot's diagonal monotone. -/

/-- The unary monotonicity fact a wire's carrier family carries into
the `_smono` lemmas: `TMono` for tick families, the pinning placeholder
`TMonoTriv` otherwise (every wire binder is unified from its
hypothesis). -/
def smonoRelOf : CarrierFam → Name
  | .atomic | .atomicStream | .atomicKSing | .atomicKeyed => ``TMono
  | _ => ``TMonoTriv

def CarrierFam.isTick : CarrierFam → Bool
  | .atomic | .atomicStream | .atomicKSing | .atomicKeyed => true
  | _ => false

/-- The tick-monotonicity walk: hypothesis/trivial leaves, callee
rules, the per-op head dispatch (`co_smono_step`). -/
macro "hydro_smono_walk" "[" rules:ident,* "]" : tactic => do
  let userRws : Array (Lean.TSyntax `tactic) ←
    if rules.getElems.isEmpty then pure #[] else do
      pure #[← `(tactic| hydro_rule_apply [$rules,*])]
  let base : Array (Lean.TSyntax `tactic) := #[
    ← `(tactic| with_reducible assumption),
    ← `(tactic| with_reducible exact TMonoTriv.intro _)]
  let steps : Array (Lean.TSyntax `tactic) := #[
    ← `(tactic| co_smono_step),
    ← `(tactic| (with_reducible refine And.intro ?_ ?_)),
    ← `(tactic| split),
    ← `(tactic| dsimp only [])]
  let alts := base ++ userRws ++ steps
  let failTac ← `(tactic| fail "hydro_smono_walk: no rule applies")
  let alt ← alts.foldrM (init := failTac) fun t acc =>
    `(tactic| first | $t:tactic | $acc:tactic)
  `(tactic| repeat' $alt:tactic)

/-- Per-leg machine tick-monotonicity `M_smono_i …`: tick-monotone
tick-family wire inputs give a tick-monotone tick-family leg. Binders
mirror `genCausal` (wires implicit, one hypothesis per wire). -/
def genSMono (mc : ModCtx) : TermElabM (Array Name) := do
  let mut decls : Array (Nat × Name × BinderInfo × Expr) := #[]
  for i in [0:mc.args.size] do
    let ty := reinterp mc.corner mc.sched (← inferType mc.args[i]!)
    let nm ← mc.args[i]!.fvarId!.getUserName
    match mc.kinds[i]! with
    | .wire _ => decls := decls.push (i, nm, .implicit, ty)
    | .plain => pure ()
    | _ => decls := decls.push (i, nm, .default, ty)
  withLocalDecls (decls.map fun (_, n, bi, ty) =>
      (n, bi, fun _ => pure ty)) fun xs => do
  let mut valOf : Std.HashMap Nat Expr := {}
  let mut j := 0
  for i in [0:mc.args.size] do
    match mc.kinds[i]! with
    | .plain => valOf := valOf.insert i mc.args[i]!
    | _ =>
      valOf := valOf.insert i xs[j]!
      j := j + 1
  let argBinders := (Array.range mc.args.size).map fun i => (valOf.get? i).get!
  let mut hypDecls : Array (Name × BinderInfo × Expr) := #[]
  for i in [0:mc.args.size] do
    if let .wire cf := mc.kinds[i]! then
      let rel ← mkAppM (smonoRelOf cf) #[(valOf.get? i).get!]
      hypDecls := hypDecls.push (Name.mkSimple s!"hm{i}", .default, rel)
  withLocalDecls (hypDecls.map fun (n, bi, ty) =>
      (n, bi, fun _ => pure ty)) fun hyps => do
  let callees ← calleeInfos mc.mName
  let calleeSMono := callees.foldl (init := #[]) fun acc (_, gi) =>
    acc ++ gi.smonoLemmas.filter (· != Name.anonymous)
  let mut names := #[]
  for i in [0:mc.legs.size] do
    let leg := mc.legs[i]!
    let legHead := mc.legTys[i]!.getAppFn
    let cf ← match legHead with
      | .const n _ =>
        match carrierFamOfHead n with
        | some cf => pure cf
        | none => throwError "HydroGen: {mc.mName}: leg {i} is not a \
            carrier ({n})"
      | _ => throwError "HydroGen: {mc.mName}: leg {i} head not const"
    let concl ← mkAppM (smonoRelOf cf)
      #[← mc.legAppAt mc.sched argBinders leg]
    let stmt ← mkForallFVars
      (mc.ctxArgs ++ #[mc.pacing] ++ argBinders ++ hyps) concl
    let name := mc.mName.appendAfter s!"_smono{subscript (i + 1)}"
    if cf.isTick then
      let ruleList := String.intercalate ", "
        (calleeSMono.toList.map toString)
      addThmByTac name stmt
        s!"(intros; simp only [{mc.mName}{rulesSuffix mc.extras}]; hydro_smono_walk [{ruleList}])"
    else
      addThmByTac name stmt "(intros; exact TMonoTriv.intro _)"
    names := names.push name
  pure names

/-! ## The wf phase: corner wf threading -/

/-- Close a `wf` goal at a FOLDED callee boundary by the callee's
generated wf lemma, at default transparency (contract subtypes carry
matcher spellings that reducible unification cannot cross). The
syntactic guard — the goal must BE a `wf` projection whose subject
mentions the lemma's module — keeps default-transparency unification
off unrelated goals, where it would delta whole modules. -/
elab "hydro_wf_close" "[" ids:ident,* "]" : tactic =>
  open Elab.Tactic in do
  let goal ← getMainGoal
  let gt ← instantiateMVars (← goal.getType)
  let env ← getEnv
  let subj? ← do
    match gt with
    | .proj tn idx st =>
      let fields := getStructureFields env tn
      pure (if fields[idx]? == some `wf then some st else none)
    | _ =>
      if gt.isApp then
        if let .const c _ := gt.getAppFn then
          if c.componentsRev.head? == some (.mkSimple "wf") &&
              (← getProjectionFnInfo? c).isSome then
            pure (some gt.appArg!)
          else pure none
        else pure none
      else pure none
  let some subj := subj?
    | throwError "hydro_wf_close: not a wf projection goal"
  -- the projection path down to the module head (leg spelling):
  -- default-transparency `apply` is only safe when the LEG matches —
  -- unifying mismatched legs whnfs through the module delta
  let pathOf : Expr → List Nat := fun e => Id.run do
    let mut path : List Nat := []
    let mut cur := e
    for _ in [0:32] do
      match cur with
      | .proj _ i st => path := i :: path; cur := st
      | _ =>
        if cur.isApp then
          let f := cur.getAppFn
          if let .const c _ := f then
            match c with
            | .str _ "fst" => path := 0 :: path; cur := cur.appArg!
            | .str _ "snd" => path := 1 :: path; cur := cur.appArg!
            | .str _ "val" => path := 100 :: path; cur := cur.appArg!
            | _ => break
          else break
        else break
    pure path
  let goalPath := pathOf subj
  for id in ids.getElems do
    let n ← realizeGlobalConstNoOverload id
    -- the lemma's module: `M_co_wfᵢ` → `M` (generated naming scheme)
    let calleeStr := (n.toString.splitOn "_co_wf").head!
    let callee := calleeStr.toName
    if (subj.find? (·.isConstOf callee)).isSome then
      -- leg check: instantiate the lemma's conclusion subject path
      let info ← getConstInfo n
      let lemPath ← Meta.forallTelescopeReducing info.type
        fun _ concl => do
          let subj' ← do
            match concl with
            | .proj _ _ st => pure st
            | _ => pure (if concl.isApp then concl.appArg! else concl)
          pure (pathOf subj')
      unless lemPath == goalPath do continue
      let ok ← tryTactic <| evalTactic
        (← `(tactic| apply $(mkIdent n)))
      if ok then return
  throwError "hydro_wf_close: no callee wf lemma closed the goal"

/-- `hydro_pre_close [M.pre_h₁, …]`: close the goal by one of the
program's `ghost pre` theorems. Each is `∀ binders, let wires…; R → S`
(the fact at the denotation, over the module's binders with the
let-chain as `let`s); the binders become metavariables, the lets are
zeta-reduced, `S` is unified with the goal, and the clause `R` is
discharged by a hypothesis in scope (the enclosing lemma's `hreq`) or
is `True`. This is how a conditional fold's pairwise conjunct and a
callee's `requires` hypothesis get closed — by the proof the program
wrote before the site, never by the fold's refused-content branch.
Unconditional facts are accepted too (a knot's loop-wire clause at the
closed knot, `<def>.<w>.req` — the chain module's `hreq` conjunct at the
enclosing module's boundary). -/
syntax preCloseRw := " rw " "[" ident,* "]"

/-- A decision-record binder of a `ghost pre` theorem (`dec : D (Values L
mem) n`), as a constructor application over fresh field metavariables
(recursively for nested records): the goal spells the record's fields
through their derived decisions (`dec.batch ↦ batchDerive …`), and a
projection of a bare metavariable never unifies with a term — a
projection of a constructor reduces to the field's metavariable.
`none` for anything that is not a `Values`-parameterized structure. -/
private partial def decRecordMVar? (ty : Expr) : MetaM (Option Expr) := do
  let ty ← instantiateMVars ty
  let .const S us := ty.getAppFn | return none
  let env ← getEnv
  unless isStructure env S do return none
  unless (ty.find? (·.isAppOf ``Values)).isSome do return none
  let ctor := getStructureCtor env S
  let mut e := mkAppN (mkConst ctor.name us) (ty.getAppArgs.extract 0 ctor.numParams)
  let mut cty ← inferType e
  for _ in [0:ctor.numFields] do
    cty ← whnf cty
    unless cty.isForall do return none
    let d := cty.bindingDomain!
    let arg ← match ← decRecordMVar? d with
      | some a => pure a
      | none => mkFreshExprMVar d
    e := mkApp e arg
    cty := cty.bindingBody!.instantiate1 arg
  return some e

/-- Assign every remaining metavariable of `e` whose type is inhabited
(`Inhabited`/`Nonempty`, after `whnf` — a `Values` decision field is a
function type or `Nat`); others stay. -/
private def fillByInhabitant (e : Expr) : MetaM Expr := do
  let e ← instantiateMVars e
  let mvars := (e.collectMVars {}).result
  for m in mvars do
    if ← m.isAssigned then continue
    let ty ← whnf (← m.getType)
    let inst? ← (do
      try
        let i ← synthInstance (← mkAppM ``Inhabited #[ty])
        pure (some (← mkAppOptM ``Inhabited.default #[some ty, some i]))
      catch _ =>
        try
          let i ← synthInstance (← mkAppM ``Nonempty #[ty])
          pure (some (← mkAppOptM ``Classical.ofNonempty #[some ty, some i]))
        catch _ => pure none)
    if let some v := inst? then
      if ← isDefEq (mkMVar m) v then pure () else pure ()
  instantiateMVars e

/-- A fresh metavariable for a theorem binder — a constructor of field
metavariables for a decision record. -/
private def binderMVar (d : Expr) : MetaM Expr := do
  match ← decRecordMVar? d with
  | some e => pure e
  | none => mkFreshExprMVar d

/-- One attempt of `hydro_pre_close` on the main goal: try each fact in
turn; `true` when one closed the goal. -/
private def preCloseAttempt (names : Array Name) (errs : IO.Ref (Array MessageData))
    (reducibleOnly : Bool) :
    Elab.Tactic.TacticM Bool := open Elab.Tactic in do
  let goal ← getMainGoal
  let gt ← instantiateMVars (← goal.getType)
  for n in names do
    let ok ← (do
      let s ← saveState
      try
        (do
          let info ← getConstInfo n
          let lvls := info.levelParams.map mkLevelParam
          -- binders → mvars, lets zeta-reduced, down to the `PreFact R S` head
          let mut e := Lean.mkConst n lvls
          let mut ty ← instantiateMVars info.type
          let mut fuel := 256
          while fuel > 0 do
            fuel := fuel - 1
            ty ← whnfCore ty
            match ty with
            | .forallE _ d b _ =>
              let m ← binderMVar d
              e := mkApp e m
              ty := b.instantiate1 m
            | .letE _ _ v b _ => ty := b.instantiate1 v
            | _ => break
          -- a `ghost pre` theorem (`PreFact R S`), or an unconditional
          -- fact (a knot's loop-wire clause at the closed knot, `K.req`):
          -- for the latter the statement's own binders were consumed by
          -- the loop above — re-walk the theorem, unifying at every arity
          let isPre := ty.isAppOfArity ``PreFact 2
          let mut rTy := Lean.mkConst ``True
          if isPre then
            rTy := ty.getArg! 0
            -- reducible first (D71 Stage C): default transparency deltas
            -- the callee modules on both sides (a knot's `leader_election`
            -- inside a corner goal — the heartbeat bomb); the default
            -- pass is the last resort, on the `rr`-rewritten goal
            let hit ← if reducibleOnly then withReducible (isDefEq (ty.getArg! 1) gt)
              else isDefEq (ty.getArg! 1) gt
            unless hit do
              throwError "hydro_pre_close: {n} does not match the goal"
          else
            e := Lean.mkConst n lvls
            ty ← instantiateMVars info.type
            let mut matched := false
            fuel := 256
            while fuel > 0 do
              fuel := fuel - 1
              let s ← saveState
              -- reducible: the fact and the goal are both spelled at the
              -- denotation legs after the `rw`; default transparency here
              -- would delta whole knots (the `hydro_wf_close` lesson)
              if ← withReducible (isDefEq ty gt) then
                matched := true
                break
              s.restore
              ty ← whnfCore ty
              match ty with
              | .forallE _ d b _ =>
                let m ← binderMVar d
                e := mkApp e m
                ty := b.instantiate1 m
              | .letE _ _ v b _ => ty := b.instantiate1 v
              | _ => break
            unless matched do
              throwError "hydro_pre_close: {n} does not match the goal at any arity"
          -- the clause: by a hypothesis in scope, or trivially
          rTy ← instantiateMVars rTy
          let rPrf ← (do
            if rTy.isConstOf ``True then pure (some (Lean.mkConst ``True.intro))
            else
              let lctx ← getLCtx
              let mut found : Option Expr := none
              for d in lctx do
                if found.isNone && !d.isImplementationDetail then
                  if ← isDefEq (← instantiateMVars d.type) rTy then
                    found := some d.toExpr
              pure found)
          let some rPrf := rPrf
            | throwError "hydro_pre_close: the clause of {n} is not in scope"
          let prf ← instantiateMVars (if isPre then mkApp e rPrf else e)
          -- a decision record's unused fields (`dec.fuel` when the fact
          -- speaks of `dec.batch` only) stay undetermined: the theorem is
          -- parametric in them — any inhabitant instantiates it
          let prf ← fillByInhabitant prf
          if prf.hasExprMVar then
            let ms := (prf.collectMVars {}).result
            let tys ← ms.mapM fun m => do
              pure (m!"{← instantiateMVars (← m.getType)}")
            throwError "hydro_pre_close: {n} leaves binders undetermined: \
              {MessageData.joinSep tys.toList m!"; "}"
          goal.assign prf
          replaceMainGoal [])
        pure true
      catch e =>
        s.restore
        errs.modify (·.push (m!"{n}: " ++ e.toMessageData))
        pure false)
    if ok then return true
  return false

elab "hydro_pre_close" "[" ids:ident,* "]" rws:(preCloseRw)? : tactic =>
  open Elab.Tactic in withMainContext do
  let names ← ids.getElems.mapM fun id => realizeGlobalConstNoOverload id
  let errs ← IO.mkRef (#[] : Array MessageData)
  let gt ← instantiateMVars (← (← getMainGoal).getType)
  let diag := (← IO.getEnv "HYDROGEN_PRE_DIAG").isSome
  if diag then IO.eprintln s!"[hydro_pre_close] pass 1 (raw, reducible) on: {← ppExpr gt}"
  if ← preCloseAttempt names errs true then
    if diag then IO.eprintln "[hydro_pre_close] pass 1 closed"
    return
  -- the goal may sit at the corner (`(K corner args).rr` of a callee):
  -- the callees' `rr` namings move it to the denotation legs, where the
  -- facts speak — a second attempt on the rewritten goal (reducible,
  -- then at default transparency as the last resort)
  let mut rewritten := false
  if let some rws := rws then
    let rwNames ← rws.raw[2].getSepArgs.mapM fun id => realizeGlobalConstNoOverload id
    unless rwNames.isEmpty do
      let mut st : SimpTheorems := {}
      for n in rwNames do
        st ← st.addConst n
      let ctx ← Simp.mkContext (simpTheorems := #[st])
        (congrTheorems := ← getSimpCongrTheorems)
      let goal ← getMainGoal
      if diag then IO.eprintln "[hydro_pre_close] rewriting with the callees' rr lemmas"
      let (r, _) ← simpTarget goal ctx (mayCloseGoal := false)
      if let some g := r then
        if g != goal then
          replaceMainGoal [g]
          rewritten := true
          if diag then IO.eprintln s!"[hydro_pre_close] pass 2 (rewritten, reducible) on: {← ppExpr (← g.getType)}"
          if ← preCloseAttempt names errs true then
            if diag then IO.eprintln "[hydro_pre_close] pass 2 closed"
            return
  if (← IO.getEnv "HYDROGEN_PRE_DIAG").isSome then
    -- the reducible passes failed: say why before the (expensive) default pass
    IO.eprintln s!"[hydro_pre_close] reducible passes failed (rewritten := {rewritten}) on: {← ppExpr gt}"
    for e in (← errs.get) do IO.eprintln s!"[hydro_pre_close]   {← e.toString}"
  if ← preCloseAttempt names errs false then
    if diag then IO.eprintln "[hydro_pre_close] pass 3 (default transparency) closed"
    return
  let errs ← errs.get
  if (← IO.getEnv "HYDROGEN_PRE_DIAG").isSome then
    -- stderr: a failing `first` alternative's messages are discarded
    IO.eprintln s!"[hydro_pre_close] goal: {← ppExpr gt}"
    for e in errs do IO.eprintln s!"[hydro_pre_close]   {← e.toString}"
  throwError "hydro_pre_close: no `ghost pre` fact closes the goal:{indentExpr gt}\n{MessageData.joinSep errs.toList "\n"}"

/-- Per-leg wf-threading `M_co_wf₁ …`: the input wires' `wf` implies
each output leg's `wf`. Knot-free callee modules are unfolded into
the wf normalizer (their wf structure threads conjunctively);
knot-reaching callees stay folded and close by their wf lemmas. -/
def genWf (mc : ModCtx) : TermElabM (Array Name) := do
  -- wf hypotheses for wire args (`whnfR` normalizes the projection-fn
  -- spelling to the raw `.proj` node co_wf_simp leaves in goals, so
  -- `with_reducible assumption` matches syntactically)
  let mut hypDecls : Array (Name × BinderInfo × Expr) := #[]
  for i in [0:mc.args.size] do
    if let .wire _ := mc.kinds[i]! then
      hypDecls := hypDecls.push
        (Name.mkSimple s!"hw{i}", .default,
         ← whnfR (← mkProjection mc.args[i]! `wf))
  -- the module's own `requires` clause, at the inputs' denotation legs
  -- (the lemma's trailing hypothesis `hreq`; callers supply it)
  let reqHyp : Array (Name × BinderInfo × Expr) ← do
    match ← requiresFnAt mc.mName with
    | none => pure #[]
    | some fn =>
      pure #[(`hreq, .default, ← mc.requiresAt fn)]
  withLocalDecls ((hypDecls ++ reqHyp).map fun (n, bi, ty) =>
      (n, bi, fun _ => pure ty)) fun hyps => do
  let callees ← calleeInfos mc.mName
  -- EVERY callee stays folded: its wf closes by its own `_co_wf`
  -- lemmas (knots via `K_co_wf₁`, leaves and glue via their generated
  -- stacks). Unfolding a callee into the normalizer was the cost
  -- center (`sequence_payload` alone put `co_wf_simp` at ~17s) and
  -- forced transitive closer lists. A closer application exposes the
  -- callee's wire-wf hypotheses as goals over this module's ops, so
  -- the loop re-runs the normalizer as an alternative.
  let knotWfs := callees.foldl (init := #[]) fun acc (_, gi) =>
    acc ++ gi.wfLemmas
  -- the program's `ghost pre` facts: closers for the pairwise conjuncts
  -- of conditional folds and for callees' `requires` hypotheses
  let pres ← currentPres.get
  if (← IO.getEnv "HYDROGEN_WF_DIAG").isSome then
    logInfo m!"hydro_wf {mc.mName}: pre closers {pres}"
  let calleeRrs := callees.foldl (init := #[]) fun acc (_, gi) =>
    acc ++ gi.rrLemmas
  let preAlt := if pres.isEmpty then ""
    else s!" | (hydro_pre_close [{String.intercalate ", " (pres.toList.map toString)}]      {if calleeRrs.isEmpty then "" else s!" rw [{String.intercalate ", " (calleeRrs.toList.map toString)}]"})"
  let mut names := #[]
  for i in [0:mc.legs.size] do
    let leg := mc.legs[i]!
    let concl ← mkProjection (← mc.legAppAt mc.corner mc.args leg) `wf
    let stmt ← mkForallFVars (mc.allBinders ++ hyps) concl
    let name := mc.mName.appendAfter s!"_co_wf{subscript (i + 1)}"
    let modList := s!"{mc.mName}"
    -- the callees' wf lemmas, keyed (D72, `hydro_rule_apply`): a wf goal
    -- `(K …).2.2.1.wf` only ever matches K's lemma for THAT leg
    let closeList := String.intercalate ", "
      (knotWfs.toList.map toString)
    let knotAlts := if knotWfs.isEmpty then ""
      else s!" | (hydro_rule_apply [{closeList}])"
    let closeAlt := if knotWfs.isEmpty then ""
      else s!" | (hydro_wf_close [{closeList}])"
    let diag := (← IO.getEnv "HYDROGEN_WF_DIAG").isSome
    addThmByTac name stmt
      (if diag then
        s!"(intros; co_wf_simp [{modList}{rulesSuffix mc.extras}]; " ++
        s!"repeat' first | exact trivial \
           | (with_reducible assumption) \
           | (with_reducible refine And.intro ?_ ?_)\
           {knotAlts}{closeAlt}{preAlt} \
           | (co_wf_simp [{modList}{rulesSuffix mc.extras}]))"
      else
        s!"(intros; co_wf_simp [{modList}{rulesSuffix mc.extras}]; " ++
        -- `assumption`/`⟨⟩`-splitting at default transparency whnf
        -- through folded callee deltas (the heartbeat bomb): every
        -- alternative that unifies stays reducible
        s!"repeat' first | exact trivial \
           | (with_reducible assumption) \
           | (with_reducible refine And.intro ?_ ?_)\
           {knotAlts}{closeAlt}{preAlt} \
           | (co_wf_simp [{modList}{rulesSuffix mc.extras}]) \
           | (co_simp [{modList}{rulesSuffix mc.extras}]) " ++
        "| assumption | refine ⟨?_, ?_⟩)")
    names := names.push name
  pure names

/-! ## The mono phase: per-leg `Values` monotonicity (via `MonoRel`) -/

/-- The `Values`-side order for a wire/leg: grade-directed. Returns
the relation as a function of the two sides. -/
def valuesRelOf (cf : CarrierFam) (ty : Expr) (a b : Expr) :
    TermElabM Expr := do
  match cf with
  | .stream => do
    -- ∀ i, PoolLe ord ret (a i) (b i)
    let ord := ty.getArg! (ty.getAppNumArgs - 2)
    let ret := ty.getArg! (ty.getAppNumArgs - 1)
    let aS ← exprToSyntax a
    let bS ← exprToSyntax b
    let ordS ← exprToSyntax ord
    let retS ← exprToSyntax ret
    elabType (← `(∀ i, PoolLe $ordS $retS ($aS i) ($bS i)))
  | .atomic => do
    let aS ← exprToSyntax a
    let bS ← exprToSyntax b
    elabType (← `(∀ i, ($aS i) <+: ($bS i)))
  | .atomicStream => do
    let aS ← exprToSyntax a
    let bS ← exprToSyntax b
    elabType (← `(∀ i, ($aS i) <+: ($bS i)))
  | .sing => do
    -- read-prefix at every cut decision (`SingRel`'s shape);
    -- bound-directed (`Monotonic` singleton cells are subtypes)
    let aS ← exprToSyntax a
    let bS ← exprToSyntax b
    let bound ← whnfR (ty.getArg! (ty.getAppNumArgs - 1))
    if bound.isAppOf ``SingBound.monotonic then
      elabType (← `(∀ i d, (($aS i)).val d <+: (($bS i)).val d))
    else
      elabType (← `(∀ i d, ($aS i d) <+: ($bS i d)))
  | _ => throwError "HydroGen: mono relation for this carrier family \
      is not supported in phase 1"

/-- Per-leg `Values` monotonicity `M_mono₁ …`, by the `MonoRel`
instantiation (the module is `H`-generic — no per-program content). -/
def genMono (mc : ModCtx) : TermElabM (Array Name) := do
  let monoI := mkAppN (mkConst ``MonoRel) #[mc.L, mc.memE]
  -- Values-typed args; wires doubled (implicit) + order hyps; plain
  -- args shared
  let mut decls : Array (Name × BinderInfo × Expr) := #[]
  for i in [0:mc.args.size] do
    let ty := reinterp mc.corner mc.values (← inferType mc.args[i]!)
    let nm ← mc.args[i]!.fvarId!.getUserName
    match mc.kinds[i]! with
    | .wire _ =>
      decls := decls.push (nm, .implicit, ty)
      decls := decls.push (nm.appendAfter "'", .implicit, ty)
    | .plain => pure ()
    | _ => decls := decls.push (nm, .default, ty)
  withLocalDecls (decls.map fun (n, bi, ty) =>
      (n, bi, fun _ => pure ty)) fun xs => do
  let mut fstOf : Std.HashMap Nat Expr := {}
  let mut sndOf : Std.HashMap Nat Expr := {}
  let mut j := 0
  for i in [0:mc.args.size] do
    match mc.kinds[i]! with
    | .wire _ =>
      fstOf := fstOf.insert i xs[j]!
      sndOf := sndOf.insert i xs[j+1]!
      j := j + 2
    | .plain =>
      fstOf := fstOf.insert i mc.args[i]!
      sndOf := sndOf.insert i mc.args[i]!
    | _ =>
      fstOf := fstOf.insert i xs[j]!
      sndOf := sndOf.insert i xs[j]!
      j := j + 1
  let mut argBinders : Array Expr := #[]
  for i in [0:mc.args.size] do
    match mc.kinds[i]! with
    | .wire _ =>
      argBinders := argBinders.push (fstOf.get? i).get!
      argBinders := argBinders.push (sndOf.get? i).get!
    | .plain => argBinders := argBinders.push mc.args[i]!
    | _ => argBinders := argBinders.push (fstOf.get? i).get!
  let mut hypDecls : Array (Name × BinderInfo × Expr) := #[]
  for i in [0:mc.args.size] do
    if let .wire cf := mc.kinds[i]! then
      let ty := reinterp mc.corner mc.values (← inferType mc.args[i]!)
      let rel ← valuesRelOf cf ty (fstOf.get? i).get! (sndOf.get? i).get!
      hypDecls := hypDecls.push
        (Name.mkSimple s!"hm{i}", .default, rel)
  withLocalDecls (hypDecls.map fun (n, bi, ty) =>
      (n, bi, fun _ => pure ty)) fun hyps => do
  -- the MonoRel instantiation: wires packed as ⟨(a, b), h⟩
  let mut hypOf : Std.HashMap Nat Expr := {}
  let mut hj := 0
  for i in [0:mc.args.size] do
    if let .wire _ := mc.kinds[i]! then
      hypOf := hypOf.insert i hyps[hj]!
      hj := hj + 1
  let mut monoArgs := #[]
  for i in [0:mc.args.size] do
    let v ← match mc.kinds[i]! with
      | .wire _ =>
        let pair ← mkAppM ``Prod.mk
          #[(fstOf.get? i).get!, (sndOf.get? i).get!]
        -- the MonoRel carrier is a subtype; supply its predicate
        -- explicitly (inference cannot see through the instance)
        let monoTy ← whnf
          (reinterp mc.corner monoI (← inferType mc.args[i]!))
        unless monoTy.isAppOfArity ``Subtype 2 do
          throwError "HydroGen: {mc.mName}: MonoRel carrier for arg \
            {i} is not a subtype"
        mkAppOptM ``Subtype.mk
          #[monoTy.getArg! 0, monoTy.getArg! 1, pair,
            (hypOf.get? i).get!]
      | .decRec _ =>
        -- Values-typed record → MonoRel-typed record (field-wise
        -- identity repack; the decision vocabularies coincide)
        repackRec mc.values monoI (fun _ p => pure p)
          (fstOf.get? i).get!
      | _ => pure (fstOf.get? i).get!
    monoArgs := monoArgs.push v
  let fstAll := (Array.range mc.args.size).map fun i => (fstOf.get? i).get!
  let sndAll := (Array.range mc.args.size).map fun i => (sndOf.get? i).get!
  let mut names := #[]
  for i in [0:mc.legs.size] do
    let leg := mc.legs[i]!
    let legHead := mc.legTys[i]!.getAppFn
    let cf ← match legHead with
      | .const n _ =>
        match carrierFamOfHead n with
        | some cf => pure cf
        | none => throwError "HydroGen: {mc.mName}: mono leg {i} not \
            a carrier"
      | _ => throwError "HydroGen: {mc.mName}: mono leg {i} head"
    let legTyV := reinterp mc.corner mc.values mc.legTys[i]!
    let concl ← valuesRelOf cf legTyV
      (← mc.legAppAt mc.values fstAll leg)
      (← mc.legAppAt mc.values sndAll leg)
    let name := mc.mName.appendAfter s!"_mono{subscript (i + 1)}"
    withDeclDiag (toString name) do
      let prf ← mkProjection (← mc.legAppAt monoI monoArgs leg) `property
      let stmt ← mkForallFVars
        (mc.ctxArgs ++ argBinders ++ hyps) concl
      -- the MonoRel projection IS the proof (module-scale defeq)
      let val ← mkLambdaFVars (mc.ctxArgs ++ argBinders ++ hyps) prf
      addThm name stmt val
    names := names.push name
  pure names

/-! ## The commands -/

/-- `registerInfo`, term-elaboration side (the eager knot-stack drain
runs inside `fix` elaboration). -/
def registerInfoT (M : Name) (f : GenInfo → GenInfo) :
    Elab.TermElabM Unit := do
  let cur := (getGenInfo (← getEnv) M).getD {}
  modifyEnv fun env => genRegistry.addEntry env (M, f cur)

/-- Does the registered module (transitively) reach a knot through
its spec'd callee graph? Module-scale projections (`hydro_mono`'s
`MonoRel` route) are definitional only below the first fix. -/
def reachesKnotN (env : Environment) (c : Name) : Bool := Id.run do
  let mut seen : NameSet := {}
  let mut stack := #[c]
  while h : stack.size > 0 do
    let n := stack[stack.size - 1]
    stack := stack.pop
    if seen.contains n then continue
    seen := seen.insert n
    if let some gi := getGenInfo env n then
      if gi.isKnot || gi.hasFix then return true
      for d in gi.callees do
        stack := stack.push d
  return false

/-- Does the module's body contain a knot (`HydroSem.fix`/`fixTick`)?
Knot modules get machine namings from the knot phase; their
denotational namings and `wf` go through the structural knot route. -/
def hasKnot (M : Name) : MetaM Bool := do
  let some info := (← getEnv).find? M | return false
  let some v := info.value? | return false
  return v.getUsedConstants.any fun c =>
    c == ``HydroSem.fix || c == ``HydroSem.fixTick

/-- The optional extras bracket (a NAMED syntax kind — anonymous
optional groups do not survive elab-pattern antiquotation; the
phase-1 spelling silently dropped every extras list). -/
syntax hydroGenIds := " [" ident,* "]"

/-- Parse the optional hint bracket of `hydro [id, id, …] def M`. -/
def elabExtras (stx : Option Syntax) : CommandElabM (Array Name) := do
  match stx with
  | none => pure #[]
  | some s => do
    let mut out := #[]
    for a in s[1].getArgs do
      if a.isIdent then
        out := out.push (← liftCoreM <| realizeGlobalConstNoOverload a)
    pure out

/-- Extras merged with the module's recorded inline wrappers (the
spec's `inlines` must unfold wherever command-line extras would). -/
def extrasWithSpec (env : Environment) (M : Name)
    (extras : Array Name) : Array Name :=
  match getGenInfo env M with
  | some gi => extras ++ gi.inlines.filter (!extras.contains ·)
  | none => extras

/-- The naming engine for plain modules (`M_co_srᵢ`, and `M_co_rrᵢ`
without content decisions): the knot glue's simp + keyed-residual
chain (`HydroGenKnot.genNaming`), routed here through a hook (the
engine lives with the glue machinery it reuses). -/
initialize namingEngineHook :
    IO.Ref (Option (ModCtx → Bool → TermElabM (Array Name))) ← IO.mkRef none

/-- The couple phase, term-elaboration side, **idempotent**
(see `runModMonoT`; the eager knot-stack drain inside `fix`
elaboration runs it for couple-route chain bodies). -/
def runCoupleT (mName : Name) (extras : Array Name) :
    Elab.TermElabM Unit := do
  if let some gi := getGenInfo (← getEnv) mName then
    unless gi.srLemmas.isEmpty do
      return
  let extras := extrasWithSpec (← getEnv) mName extras
  let (srs, rrs, ex?, vdec?, pins) ← 
    withModCtx mName (extras := extras) fun mc => do
    let some eng ← namingEngineHook.get
      | throwError "hydro_couple: naming engine not installed           (import Hydro.HydroGenKnot)"
    let srs ← eng mc false
    if ← hasKnot mName then
      -- knot module: the denotational naming is the structural
      -- (per-knot) route — the knot phase
      pure (srs, #[], none, none, #[])
    else if mc.contentDecs.isEmpty then
      let rrs ← eng mc true
      pure (srs, rrs, none, none, #[])
    else
      let (ex, vdec, rrs, pins) ← genRrEx mc
      pure (srs, rrs, some ex, some vdec, pins)
  registerInfoT mName fun gi =>
    { gi with srLemmas := srs, rrLemmas := rrs,
              exLemma? := ex?, vdec? := vdec?, pinLemmas := pins }
  logInfo m!"hydro_couple {mName}: {srs.size} sr, {rrs.size} rr\
    {if ex?.isSome then " (+ex/vdec)" else ""}"

/-- The causal phase, term-elaboration side, **idempotent**
(see `runModMonoT`). -/
def runModCausalT (mName : Name) (extras : Array Name) :
    Elab.TermElabM Unit := do
  if let some gi := getGenInfo (← getEnv) mName then
    unless gi.causalLemmas.isEmpty do
      return
  let extras := extrasWithSpec (← getEnv) mName extras
  let names ← withModCtx mName (extras := extras) fun mc => genCausal mc
  registerInfoT mName fun gi => { gi with causalLemmas := names }
  logInfo m!"hydro_causal {mName}: {names.size} lemmas"

/-- The smono phase, term-elaboration side, **idempotent**
(see `runModMonoT`). -/
def runModSMonoT (mName : Name) (extras : Array Name) :
    Elab.TermElabM Unit := do
  if let some gi := getGenInfo (← getEnv) mName then
    unless gi.smonoLemmas.isEmpty do
      return
  let extras := extrasWithSpec (← getEnv) mName extras
  let names ← withModCtx mName (extras := extras) fun mc => genSMono mc
  registerInfoT mName fun gi => { gi with smonoLemmas := names }
  logInfo m!"hydro_smono {mName}: {names.size} lemmas"

/-- The wf phase, term-elaboration side, **idempotent**
(see `runModMonoT`). -/
def runModWfT (mName : Name) (extras : Array Name) :
    Elab.TermElabM Unit := do
  if let some gi := getGenInfo (← getEnv) mName then
    unless gi.wfLemmas.isEmpty do
      return
  let extras := extrasWithSpec (← getEnv) mName extras
  let names ← withModCtx mName (extras := extras) fun mc => genWf mc
  let hasReq := (← requiresStxFor mName).isSome
  registerInfoT mName fun gi => { gi with wfLemmas := names, hasRequires := hasReq }
  logInfo m!"hydro_wf {mName}: {names.size} lemmas{if hasReq then " (requires)" else ""}"

/-- Glue-module mono generation (compositional): implemented in
`HydroGenKnot` and routed here through a hook (the module-scale
`MonoRel` projection is not definitional through a callee's knots). -/
initialize glueMonoHook :
    IO.Ref (Option (ModCtx → TermElabM (Array Name))) ← IO.mkRef none


/-- The mono phase, term-elaboration side, **idempotent** (the
eager knot-stack drain inside `fix` elaboration runs it for chain
bodies; the post-def pipeline pass then skips them). Compositional
route only when a callee reaches a knot (the module-scale `MonoRel`
projection is definitional below fixes). -/
def runModMonoT (mName : Name) : Elab.TermElabM Unit := do
  let env ← getEnv
  if let some gi := getGenInfo env mName then
    unless gi.monoLemmas.isEmpty do
      return
  let glueRoute := match getGenInfo env mName with
    | some gi => gi.callees.any (reachesKnotN env)
    | none => false
  let names ← if glueRoute then do
    let some gen ← glueMonoHook.get
      | throwError "hydro_mono: glue hook not installed           (import Hydro.HydroGenKnot)"
    withModCtx mName fun mc => gen mc
  else
    withModCtx mName fun mc => genMono mc
  registerInfoT mName fun gi => { gi with monoLemmas := names }
  logInfo m!"hydro_mono {mName}:     {(names.filter (· != Name.anonymous)).size} lemmas"


end HydroGen

end Hydro
