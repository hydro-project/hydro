import HydroV2.CoupleProj
import HydroV2.KnotTactics
import HydroV2.WfTactics

/-!
# HydroV2 · `HydroGen` — the program-machinery generator

Given a program module written in the house style (one Rust fn = one
`def`, generic over `H : HydroSem`, colocated contract, explicit
decision arguments), the `HydroGen` commands generate the mechanical
artifacts that previously lived in hand-written files
(`CoupleStd.lean`, `Paxos/CoupleModules.lean`, `Paxos/CoupleKnots.lean`,
most of `Paxos/CoupleWf.lean`, `Paxos/CoupleDec.lean`):

- `hydro_couple M` — the corner naming stack: per-leg machine namings
  `M_co_srᵢ` (the corner run's machine leg IS the `SchedSem` run) and
  denotational namings. Modules with a **content** decision get the
  choice triple `M_co_rr_ex` / `M_vdec` / `M_co_rrᵢ` — the derived
  decision is *named by choice* from the ∃-lemma, no wire replay
  (resolving `CoupleDec.lean`'s TEMP note by deletion); modules whose
  decisions are all machine data get plain equations.
- `hydro_causal M` — per-leg step-causality congruences over the
  `SchedSem` legs (head-dispatched per-op walker).
- `hydro_wf M` — corner wf-threading (inputs' `wf` ⟹ outputs' `wf`).
- `hydro_mono M` — per-leg `Values` monotonicity via the `MonoRel`
  instantiation (grade-directed relations).
- `hydro_knot K` — the knot stack for a `HydroSem.fix`/`fixTick`
  wrapper (naming, causality, chain, `wf` triple).

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

namespace HydroV2

open Lean Elab Term Meta Command

namespace HydroGen

/-! ## Vocabulary classification -/

/-- The signature decision families. -/
inductive DecFam where
  | transport | orderSel | snap | batch | ordBatch | batchOrdSel
  | sample | timer | pulse | emit | fixd
  deriving BEq, Repr, Inhabited

def decFamOfHead : Name → Option DecFam
  | ``HydroSem.TransportDec => some .transport
  | ``HydroSem.OrderSelDec => some .orderSel
  | ``HydroSem.SnapDec => some .snap
  | ``HydroSem.BatchDec => some .batch
  | ``HydroSem.OrdBatchDec => some .ordBatch
  | ``HydroSem.BatchOrdSelDec => some .batchOrdSel
  | ``HydroSem.SampleDec => some .sample
  | ``HydroSem.TimerDec => some .timer
  | ``HydroSem.PulseDec => some .pulse
  | ``HydroSem.EmitDec => some .emit
  | ``HydroSem.FixDec => some .fixd
  | _ => none

/-- Content decisions: `Values`-side data the corner derives from its
machine leg (`Unit` at the machine). `fixd` is content but *pinned*
(the corner's fuel is `Td + 1` by construction), so it is not
∃-quantified. -/
def DecFam.isContent : DecFam → Bool
  | .orderSel | .snap | .batch | .ordBatch => true
  | _ => false

/-- Machine-side pass-through (the corner's decision type = the
machine's). -/
def DecFam.schedPass : DecFam → Bool
  | .transport | .sample | .timer | .pulse | .emit | .batchOrdSel => true
  | _ => false

/-- `Values`-side pass-through (data in both worlds). -/
def DecFam.valuesPass : DecFam → Bool
  | .sample | .timer | .pulse | .batchOrdSel => true
  | _ => false

/-- The five carrier families. -/
inductive CarrierFam where
  | stream | keyed | sing | tickSing | tickStream
  deriving BEq, Repr, Inhabited

def carrierFamOfHead : Name → Option CarrierFam
  | ``HydroSem.Stream => some .stream
  | ``HydroSem.KeyedStream => some .keyed
  | ``HydroSem.Singleton => some .sing
  | ``HydroSem.TickSingleton => some .tickSing
  | ``HydroSem.TickStream => some .tickStream
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
  bundles like `LEWires`). -/
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
  wfLemmas : Array Name := #[]
  monoLemmas : Array Name := #[]
  /-- Named fuel-pin projections of the choice spec (`(M_vdec …).fuel
  = Td + 1` per `fixd` field) — the knot route rewrites the `Values`
  fix's fuel leg with these. -/
  pinLemmas : Array Name := #[]
  /-- Knot modules (`hydro_knot`): consumers keep them folded (their
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

/-- Wrapper defs (`hydro_inline C`) opened during spec walks and
generation (e.g. `leCore` — an `H`-generic abbreviation that is not a
module of its own). -/
initialize inlineRegistry :
    SimplePersistentEnvExtension Name NameSet ←
  registerSimplePersistentEnvExtension {
    addImportedFn := fun as =>
      as.foldl (fun m es => es.foldl (fun m n => m.insert n) m) {}
    addEntryFn := fun m n => m.insert n
  }

/-! ## Proof-by-script -/

/-- Prove `goal` by a tactic script given as a string (generated
scripts reference only global names). -/
def proveByTac (goal : Expr) (tac : String) : TermElabM Expr := do
  let env ← getEnv
  let stx ← match Parser.runParserCategory env `tactic tac with
    | .ok s => pure s
    | .error e => throwError "HydroGen: tactic parse error: {e}\n{tac}"
  let byStx ← `(by $(⟨stx⟩):tactic)
  -- the goal is closed; elaborate in an empty local context so the
  -- script's `intro` names cannot be shadowed by the generator's
  -- open telescopes
  -- NOTE: no heartbeat bump — generated proofs must stay module-sized
  -- (a timeout here is a diagnostic failure of the script structure,
  -- not a budget problem)
  let val ← withLCtx {} {} <| Term.withoutErrToSorry do
    let mvar ← mkFreshExprSyntheticOpaqueMVar goal
    let (remaining, _) ← Lean.Elab.runTactic mvar.mvarId! (⟨stx⟩ : TSyntax `tactic)
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

def addThmByTac (name : Name) (stmt : Expr) (tac : String) :
    TermElabM Unit := do
  let stmt ← instantiateMVars stmt
  let val ← proveByTac stmt tac
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
    (← `((ℓ : $Lstx) → Fin ($memStx ℓ) → Nat → Bool))
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
back to a transitive scan through unregistered `HydroV2` glue defs
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
        else if depth < 3 && (`HydroV2).isPrefixOf c then
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
def registerSpec (M : Name) : MetaM Unit := do
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
instances the module's matches scrutinize), supplied on the command
line: `hydro_couple M [p1bPairDecEq]`. -/
structure GenExtras where
  extras : Array Name := #[]

/-! ## `hydro_couple`: the naming stack -/

/-- Per-leg machine namings `M_co_srᵢ`. -/
def genSr (mc : ModCtx) : TermElabM (Array Name) := do
  let srArgs ← mc.srArgs
  let rules ← calleeNamingRules mc.mName
  let mut names := #[]
  for i in [0:mc.legs.size] do
    let leg := mc.legs[i]!
    let lhs ← mkProjection (← mc.legAppAt mc.corner mc.args leg) `sr
    let rhs ← mc.legAppAt mc.sched srArgs leg
    let stmt ← mkForallFVars mc.allBinders (← mkEq lhs rhs)
    let name := mc.mName.appendAfter s!"_co_sr{subscript (i + 1)}"
    addThmByTac name stmt
      s!"(intros; co_transfer [{mc.mName}{rulesSuffix rules}{rulesSuffix mc.extras}])"
    names := names.push name
  pure names

/-- Per-leg denotational namings, no content decisions. -/
def genRrDirect (mc : ModCtx) : TermElabM (Array Name) := do
  let rrArgs ← mc.rrArgsDirect
  let rules ← calleeNamingRules mc.mName
  let mut names := #[]
  for i in [0:mc.legs.size] do
    let leg := mc.legs[i]!
    let lhs ← mkProjection (← mc.legAppAt mc.corner mc.args leg) `rr
    let rhs ← mc.legAppAt mc.values rrArgs leg
    let stmt ← mkForallFVars mc.allBinders (← mkEq lhs rhs)
    let name := mc.mName.appendAfter s!"_co_rr{subscript (i + 1)}"
    addThmByTac name stmt
      s!"(intros; co_transfer [{mc.mName}{rulesSuffix rules}{rulesSuffix mc.extras}])"
    names := names.push name
  pure names

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
    | .wire _ =>
      outerReqs := outerReqs.push
        (i, [], Name.mkSimple s!"x{i}",
         reinterp mc.corner mc.sched (← inferType mc.args[i]!))
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
      | .wire _ => mkProjection (innerArg i) `sr
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
  -- callee namings interleaved with the FULL projection simp set:
  -- each callee rewrite exposes fresh `rr`/`sr` projections that must
  -- be pushed before the next callee application matches (the
  -- hand-written `sp_co_rr_ex` shape); the closing `rfl` then only
  -- pays leaf-local defeq
  let calleeRws :=
    if rules.isEmpty then ""
    else "(repeat first " ++ String.join (rules.toList.map fun r =>
      s!"| rw [{r}] ") ++
      s!"| co_simp [{(rulesSuffix rules ++ rulesSuffix mc.extras).drop 2}]); "
  -- the ∃-witness skeleton: plain hole for a primitive decision,
  -- nested constructor skeleton for a record (projection constraints
  -- are not unifiable — the skeleton exposes per-field holes)
  let witness := match mc.kinds[cIdx]! with
    | .decRec _ => skeletonOf (mc.trees[cIdx]!).get!
    | _ => "_"
  let script :=
    s!"(intros; apply Exists.intro (({witness})); intro {introNames}; {refinePart}" ++
    s!"(try co_simp [{mc.mName}{rulesSuffix rules}{rulesSuffix mc.extras}]); " ++
    s!"{calleeRws}{pinRws}(try exact rfl)))"
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
      | .wire _ => mkProjection mc.args[i]! `sr
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

/-! ## `hydro_causal`: per-leg step causality over the machine legs -/

/-- The agreement relation for a carrier family (the `SchedCausal`
vocabulary). -/
def agreeRelOf : CarrierFam → Name
  | .stream => ``SAgree
  | .keyed => ``KAgree
  | .sing => ``FAgree
  | .tickSing => ``TAgree
  | .tickStream => ``TAgree

/-- The `co_causal_walk` recipe (public copy of the house pattern):
refl/assumption/mono leaves, caller rules, then the per-op head
dispatch. -/
macro "hydro_causal_walk" "[" rules:term,* "]" : tactic => do
  let userRws ← rules.getElems.mapM fun r =>
    `(tactic| with_reducible apply $r:term)
  let base : Array (Lean.TSyntax `tactic) := #[
    ← `(tactic| with_reducible assumption),
    ← `(tactic| with_reducible exact TAgree.refl _ _),
    ← `(tactic| with_reducible exact SAgree.refl _ _),
    ← `(tactic| with_reducible exact KAgree.refl _ _),
    ← `(tactic| with_reducible exact FAgree.refl _ _),
    ← `(tactic| with_reducible exact VAgree.refl _ _)]
  let steps : Array (Lean.TSyntax `tactic) := #[
    ← `(tactic| co_causal_step)]
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

/-! ## `hydro_wf`: corner wf threading -/

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
  withLocalDecls (hypDecls.map fun (n, bi, ty) =>
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
  let mut names := #[]
  for i in [0:mc.legs.size] do
    let leg := mc.legs[i]!
    let concl ← mkProjection (← mc.legAppAt mc.corner mc.args leg) `wf
    let stmt ← mkForallFVars (mc.allBinders ++ hyps) concl
    let name := mc.mName.appendAfter s!"_co_wf{subscript (i + 1)}"
    let modList := s!"{mc.mName}"
    let knotAlts := String.join (knotWfs.toList.map fun w =>
      s!" | (with_reducible apply {w})")
    let closeList := String.intercalate ", "
      (knotWfs.toList.map toString)
    let closeAlt := if knotWfs.isEmpty then ""
      else s!" | (hydro_wf_close [{closeList}])"
    let diag := (← IO.getEnv "HYDROGEN_WF_DIAG").isSome
    addThmByTac name stmt
      (if diag then
        s!"(intros; co_wf_simp [{modList}{rulesSuffix mc.extras}]; " ++
        s!"repeat' first | exact trivial \
           | (with_reducible assumption) \
           | (with_reducible refine And.intro ?_ ?_)\
           {knotAlts}{closeAlt} \
           | (co_wf_simp [{modList}{rulesSuffix mc.extras}]))"
      else
        s!"(intros; co_wf_simp [{modList}{rulesSuffix mc.extras}]; " ++
        -- `assumption`/`⟨⟩`-splitting at default transparency whnf
        -- through folded callee deltas (the heartbeat bomb): every
        -- alternative that unifies stays reducible
        s!"repeat' first | exact trivial \
           | (with_reducible assumption) \
           | (with_reducible refine And.intro ?_ ?_)\
           {knotAlts}{closeAlt} \
           | (co_wf_simp [{modList}{rulesSuffix mc.extras}]) " ++
        "| assumption | refine ⟨?_, ?_⟩)")
    names := names.push name
  pure names

/-! ## `hydro_mono`: per-leg `Values` monotonicity (via `MonoRel`) -/

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
  | .tickSing => do
    let aS ← exprToSyntax a
    let bS ← exprToSyntax b
    -- bound-directed: `.monotonic vo` carriers are `MonoTrace`s (the
    -- prefix order lives on `.vals`)
    let bound ← whnfR (ty.getArg! (ty.getAppNumArgs - 1))
    if bound.isAppOf ``SingBound.monotonic then
      elabType (← `(∀ i, (($aS i)).vals <+: (($bS i)).vals))
    else
      elabType (← `(∀ i, ($aS i) <+: ($bS i)))
  | .tickStream => do
    let aS ← exprToSyntax a
    let bS ← exprToSyntax b
    elabType (← `(∀ i, ($aS i) <+: ($bS i)))
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
    let prf ← mkProjection (← mc.legAppAt monoI monoArgs leg) `property
    let stmt ← mkForallFVars
      (mc.ctxArgs ++ argBinders ++ hyps) concl
    let name := mc.mName.appendAfter s!"_mono{subscript (i + 1)}"
    -- the MonoRel projection IS the proof (module-scale defeq)
    let val ← mkLambdaFVars (mc.ctxArgs ++ argBinders ++ hyps) prf
    addThm name stmt val
    names := names.push name
  pure names

/-! ## The commands -/

def registerInfo (M : Name) (f : GenInfo → GenInfo) : CommandElabM Unit := do
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
Knot modules get machine namings from `hydro_couple`; their
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

/-- Parse optional extras: `hydro_couple M [id, id, …]`. -/
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

/-- `hydro_inline C` — mark a wrapper def (an `H`-generic abbreviation
that is not a module of its own, e.g. `leCore`) to be opened during
spec walks and generation. -/
elab "hydro_inline " M:ident : command => do
  let mName ← liftCoreM <| realizeGlobalConstNoOverload M
  modifyEnv fun env => inlineRegistry.addEntry env mName

/-- `hydro_register M` — record the composition spec for an existing
definition (the post-hoc form of `hydro def`; the staged migration
uses it so program files change last). -/
elab "hydro_register " M:ident : command => do
  let mName ← liftCoreM <| realizeGlobalConstNoOverload M
  liftTermElabM <| registerSpec mName
  let gi := (getGenInfo (← getEnv) mName).getD {}
  logInfo m!"hydro_register {mName}: callees {gi.callees}, \
    inlines {gi.inlines}, fix := {gi.hasFix}"

elab "hydro_couple " M:ident ids:(hydroGenIds)? : command => do
  let mName ← liftCoreM <| realizeGlobalConstNoOverload M
  let extras ← elabExtras (ids.map (·.raw))
  let extras := extrasWithSpec (← getEnv) mName extras
  let (srs, rrs, ex?, vdec?, pins) ← liftTermElabM <|
    withModCtx mName (extras := extras) fun mc => do
    let srs ← genSr mc
    if ← hasKnot mName then
      -- knot module: the denotational naming is the structural
      -- (per-knot) route — `hydro_knot`
      pure (srs, #[], none, none, #[])
    else if mc.contentDecs.isEmpty then
      let rrs ← genRrDirect mc
      pure (srs, rrs, none, none, #[])
    else
      let (ex, vdec, rrs, pins) ← genRrEx mc
      pure (srs, rrs, some ex, some vdec, pins)
  registerInfo mName fun gi =>
    { gi with srLemmas := srs, rrLemmas := rrs,
              exLemma? := ex?, vdec? := vdec?, pinLemmas := pins }
  logInfo m!"hydro_couple {mName}: {srs.size} sr, {rrs.size} rr\
    {if ex?.isSome then " (+ex/vdec)" else ""}"

elab "hydro_causal " M:ident ids:(hydroGenIds)? : command => do
  let mName ← liftCoreM <| realizeGlobalConstNoOverload M
  let extras ← elabExtras (ids.map (·.raw))
  let extras := extrasWithSpec (← getEnv) mName extras
  let names ← liftTermElabM <|
    withModCtx mName (extras := extras) fun mc => genCausal mc
  registerInfo mName fun gi => { gi with causalLemmas := names }
  logInfo m!"hydro_causal {mName}: {names.size} lemmas"

elab "hydro_wf " M:ident ids:(hydroGenIds)? : command => do
  let mName ← liftCoreM <| realizeGlobalConstNoOverload M
  let extras ← elabExtras (ids.map (·.raw))
  let extras := extrasWithSpec (← getEnv) mName extras
  let names ← liftTermElabM <|
    withModCtx mName (extras := extras) fun mc => genWf mc
  registerInfo mName fun gi => { gi with wfLemmas := names }
  logInfo m!"hydro_wf {mName}: {names.size} lemmas"

/-- Glue-module mono generation (compositional): implemented in
`HydroGenKnot` and routed here through a hook (the module-scale
`MonoRel` projection is not definitional through a callee's knots). -/
initialize glueMonoHook :
    IO.Ref (Option (ModCtx → TermElabM (Array Name))) ← IO.mkRef none

elab "hydro_mono " M:ident : command => do
  let mName ← liftCoreM <| realizeGlobalConstNoOverload M
  -- compositional route only when a callee reaches a knot (the
  -- module-scale MonoRel projection is definitional below fixes)
  let env ← getEnv
  let glueRoute := match getGenInfo env mName with
    | some gi => gi.callees.any (reachesKnotN env)
    | none => false
  let names ← if glueRoute then do
    let some gen ← glueMonoHook.get
      | throwError "hydro_mono: glue hook not installed           (import HydroV2.HydroGenKnot)"
    liftTermElabM <| withModCtx mName fun mc => gen mc
  else
    liftTermElabM <| withModCtx mName fun mc => genMono mc
  registerInfo mName fun gi => { gi with monoLemmas := names }
  logInfo m!"hydro_mono {mName}:     {(names.filter (· != Name.anonymous)).size} lemmas"


end HydroGen

end HydroV2
