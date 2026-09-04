import HydroV2.HydroGen

/-!
# HydroV2 · `HydroRel` — the generated relational signature
(parametricity for `HydroSem`, as data)

Lean has no parametricity plugin, so the fact that every program is
polymorphic over `H : HydroSem` — and therefore respects **any**
relation between two interpretations whose operations preserve it —
is not a theorem Lean gives us. This file is that plugin, scoped to
the `HydroSem` signature:

- `HRelC I H₁ H₂` (hand-written, mirrors the class's sixteen *type*
  fields): a relation family per carrier former and per decision
  family, indexed by a preordered `I` (the step/horizon index of
  graded guarantees; `Unit` for plain ones).
- `hydro_rel_laws` (**generated — nothing below hardcodes any
  correspondence**): for every *operation* field of `HydroSem`, the
  H-relative binary parametricity lift of its type — non-`H`
  data/closure/proof arguments shared, carrier/decision arguments
  doubled with relatedness premises at the ambient index, results
  related at the same index. Negative-position carrier functions (the
  `fix_stream`/`fix_tick` bodies — the only such positions) lift with
  **index lowering** (`∀ i' ≤ i`, the step-indexed logical-relation
  shape: exactly `causal_fix_stream`'s `hb` and `co_fix_cpl`'s
  `hcplj`). The op laws are bundled into one Prop (`HRel.Laws C`) with
  generated per-op accessors, so the per-module free theorems
  (`HydroParam.lean`) can quantify over **all** relations at once.

A new guarantee = one `HRelC` instance + one `HRel.Laws` proof whose
fields are (almost always) existing per-op lemmas — zero
metaprogramming, zero per-module work. A new signature op = rerun
`hydro_rel_laws`; every instance gets a compiler-error-guided new
obligation.
-/

namespace HydroV2

/-- The relation families of a (step-indexed) binary logical relation
between two interpretations: one per carrier former, one per decision
family — the parametricity translation of `HydroSem`'s *type* fields.
`I` is the guarantee's index (`Unit` when unindexed; `Nat` horizons
for causality-style guarantees); indices only interact through the
op laws' premises, so no monotonicity is assumed here. -/
structure HRelC (I : Type) [Preorder I] {L : Type} {mem : L → Nat}
    (H₁ H₂ : HydroSem L mem) where
  streamRel : ∀ {ℓ : L} {α : Type} [DecidableEq α] {ord : StrOrd}
    {ret : Retries}, I → H₁.Stream ℓ α ord ret →
    H₂.Stream ℓ α ord ret → Prop
  keyedRel : ∀ {p c : L} {α : Type} [DecidableEq α] {ord : StrOrd}
    {ret : Retries}, I → H₁.KeyedStream p c α ord ret →
    H₂.KeyedStream p c α ord ret → Prop
  singRel : ∀ {ℓ : L} {α σ : Type} [DecidableEq α] {ord : StrOrd}
    {ret : Retries} {b : SingBound σ}, I →
    H₁.Singleton ℓ α σ ord ret b → H₂.Singleton ℓ α σ ord ret b → Prop
  tickSingRel : ∀ {ℓ : L} {σ : Type} {b : SingBound σ}, I →
    H₁.TickSingleton ℓ σ b → H₂.TickSingleton ℓ σ b → Prop
  tickStreamRel : ∀ {ℓ : L} {α : Type} [DecidableEq α] {ord : StrOrd}
    {ret : Retries}, I → H₁.TickStream ℓ α ord ret →
    H₂.TickStream ℓ α ord ret → Prop
  transportRel : ∀ {p c : Nat}, I → H₁.TransportDec p c →
    H₂.TransportDec p c → Prop
  orderSelRel : ∀ {n : Nat} {α : Type}, I → H₁.OrderSelDec n α →
    H₂.OrderSelDec n α → Prop
  snapRel : ∀ {n : Nat} {α : Type} {ord : StrOrd}, I →
    H₁.SnapDec n α ord → H₂.SnapDec n α ord → Prop
  batchRel : ∀ {n : Nat} {α : Type}, I → H₁.BatchDec n α →
    H₂.BatchDec n α → Prop
  ordBatchRel : ∀ {n : Nat}, I → H₁.OrdBatchDec n →
    H₂.OrdBatchDec n → Prop
  batchOrdSelRel : ∀ {n : Nat} {α : Type}, I →
    H₁.BatchOrdSelDec n α → H₂.BatchOrdSelDec n α → Prop
  sampleRel : ∀ {n : Nat}, I → H₁.SampleDec n → H₂.SampleDec n → Prop
  timerRel : ∀ {n : Nat}, I → H₁.TimerDec n → H₂.TimerDec n → Prop
  pulseRel : ∀ {n : Nat}, I → H₁.PulseDec n → H₂.PulseDec n → Prop
  emitRel : ∀ {n : Nat} {α : Type}, I → H₁.EmitDec n α →
    H₂.EmitDec n α → Prop
  fixRel : I → H₁.FixDec → H₂.FixDec → Prop

namespace HydroGen

open Lean Elab Term Meta Command

/-- The `HRelC` field for a carrier family. -/
def relFieldOfCarrier : CarrierFam → Name
  | .stream => `streamRel
  | .keyed => `keyedRel
  | .sing => `singRel
  | .tickSing => `tickSingRel
  | .tickStream => `tickStreamRel

/-- The `HRelC` field for a decision family. -/
def relFieldOfDec : DecFam → Name
  | .transport => `transportRel
  | .orderSel => `orderSelRel
  | .snap => `snapRel
  | .batch => `batchRel
  | .ordBatch => `ordBatchRel
  | .batchOrdSel => `batchOrdSelRel
  | .sample => `sampleRel
  | .timer => `timerRel
  | .pulse => `pulseRel
  | .emit => `emitRel
  | .fixd => `fixRel

/-- The `HRelC` relation field for a carrier- or decision-headed type,
by its head constant. -/
def relFieldOfHead (n : Name) : Option Name :=
  match carrierFamOfHead n with
  | some cf => some (`HydroV2.HRelC ++ relFieldOfCarrier cf)
  | none =>
    match decFamOfHead n with
    | some df => some (`HydroV2.HRelC ++ relFieldOfDec df)
    | none => none

/-- Ops (`HydroSem.<op>` projection) → generated `HRel.Laws` accessor
name; populated by `hydro_rel_laws`, consumed by the `hydro_param`
walker. -/
initialize relLawRegistry :
    SimplePersistentEnvExtension (Name × Name) (NameMap Name) ←
  registerSimplePersistentEnvExtension {
    addImportedFn := fun as =>
      as.foldl (fun m es => es.foldl (fun m (n, i) => m.insert n i) m) {}
    addEntryFn := fun m (n, i) => m.insert n i
  }

def getRelLaw (env : Environment) (n : Name) : Option Name :=
  (relLawRegistry.getState env).find? n

/-- Is `τ` the type of a negative-position carrier function (a fix
body: non-dependent arrow between carrier-headed types)? -/
def isCarrierFun (τ : Expr) : Bool :=
  match τ with
  | .forallE _ dom cod _ =>
    !cod.hasLooseBVars &&
    (match dom.getAppFn with
     | .const n _ => (carrierFamOfHead n).isSome
     | _ => false) &&
    (match cod.getAppFn with
     | .const n _ => (carrierFamOfHead n).isSome
     | _ => false)
  | _ => false

/-- The parametricity lift of one op's (self-instantiated) type pair.
In scope: `C : HRelC I H₁ H₂` and the ambient index `i : I`. Walks the
two telescopes in lockstep: syntactically equal binder domains are
**shared** (one binder feeding both sides); differing domains are
doubled, with a relatedness premise when the head is a carrier or
decision family, and the index-lowered pointwise premise when it is a
carrier function (fix bodies). Ends in relatedness of the two results
at `i`. -/
partial def liftOpType (Cv iv : Expr) (T₁ T₂ : Expr)
    (args₁ args₂ : Array Expr)
    (mkResult : Array Expr → Array Expr → Expr → TermElabM Expr) :
    TermElabM Expr := do
  match T₁ with
  | .forallE n τ₁ _ bi =>
    let τ₂ := T₂.bindingDomain!
    if τ₁ == τ₂ then
      withLocalDecl n bi τ₁ fun x => do
        let r ← liftOpType Cv iv (T₁.bindingBody!.instantiate1 x)
          (T₂.bindingBody!.instantiate1 x)
          (args₁.push x) (args₂.push x) mkResult
        mkForallFVars #[x] r
    else
      let n₁ := n
      let n₂ := n.appendAfter "'"
      withLocalDecl n₁ .default τ₁ fun x₁ =>
      withLocalDecl n₂ .default τ₂ fun x₂ => do
        -- the relatedness premise for this doubled binder
        let prem? ← do
          if let .const h _ := τ₁.getAppFn then
            if let some fld := relFieldOfHead h then
              pure (some (← mkAppM fld #[Cv, iv, x₁, x₂]))
            else
              throwError "hydro_rel_laws: doubled binder {τ₁} has \
                unrecognized head {h}"
          else if isCarrierFun τ₁ then
            -- index-lowered pointwise preservation, with the loop
            -- wire's relatedness in STRONG form (`∀ i'' ≤ i'`): the
            -- uniform invariant every wire fact carries, so
            -- capture-crossing inner knots re-lower without any
            -- downward-closure assumption on the relation
            let dom₁ := τ₁.bindingDomain!
            let dom₂ := τ₂.bindingDomain!
            let iTy ← inferType iv
            let p ← withLocalDecl `i' .default iTy fun i' => do
              let leTy ← mkAppM ``LE.le #[i', iv]
              withLocalDecl `hle .default leTy fun hle => do
              withLocalDecl `y .default dom₁ fun y₁ => do
              withLocalDecl `y' .default dom₂ fun y₂ => do
                let relFld ← do
                  match dom₁.getAppFn with
                  | .const h _ =>
                    match relFieldOfHead h with
                    | some f => pure f
                    | none => throwError "hydro_rel_laws: fix body \
                        domain head unrecognized"
                  | _ => throwError "hydro_rel_laws: fix body domain"
                let hy ← withLocalDecl `i'' .default iTy fun i'' => do
                  let leTy' ← mkAppM ``LE.le #[i'', i']
                  withLocalDecl `hle' .default leTy' fun hle' => do
                    let r ← mkAppM relFld #[Cv, i'', y₁, y₂]
                    mkForallFVars #[i'', hle'] r
                withLocalDecl `hy .default hy fun hyv => do
                  let concl ← mkAppM relFld
                    #[Cv, i', mkApp x₁ y₁, mkApp x₂ y₂]
                  mkForallFVars #[i', hle, y₁, y₂, hyv] concl
            pure (some p)
          else
            throwError "hydro_rel_laws: doubled binder {τ₁} is neither \
              carrier/decision-headed nor a carrier function"
        let body ← do
          match prem? with
          | some prem =>
            withLocalDecl (n₁.appendAfter "_rel") .default prem fun hp => do
              let r ← liftOpType Cv iv (T₁.bindingBody!.instantiate1 x₁)
                (T₂.bindingBody!.instantiate1 x₂)
                (args₁.push x₁) (args₂.push x₂) mkResult
              mkForallFVars #[hp] r
          | none =>
            liftOpType Cv iv (T₁.bindingBody!.instantiate1 x₁)
              (T₂.bindingBody!.instantiate1 x₂)
              (args₁.push x₁) (args₂.push x₂) mkResult
        mkForallFVars #[x₁, x₂] body
  | _ =>
    -- result: relate the two op applications at `i` (T₁ is the
    -- side-1 result type — its head names the relation family)
    mkResult args₁ args₂ T₁

/-- Generate the op laws: for every operation field of `HydroSem`, a
reducible def `HRel.law_<op> … (C : HRelC I H₁ H₂) : Prop` (the
parametricity lift), the bundle `HRel.Laws C : Prop` (their
conjunction), and per-op accessors `HRel.Laws.<op>` — plus the
op-head → accessor registry the `hydro_param` walker dispatches on. -/
elab "hydro_rel_laws" : command => Command.runTermElabM fun _ => do
  let fields := getStructureFields (← getEnv) ``HydroSem
  withLocalDecl `L .implicit (mkSort 1) fun Lv => do
  withLocalDecl `mem .implicit (← mkArrow Lv (mkConst ``Nat)) fun memv => do
  withLocalDecl `I .implicit (mkSort 1) fun Iv => do
  let preTy ← mkAppM ``Preorder #[Iv]
  withLocalDecl `ipre .instImplicit preTy fun _ipv => do
  let hTy := mkApp2 (mkConst ``HydroSem) Lv memv
  withLocalDecl `H₁ .implicit hTy fun H1 => do
  withLocalDecl `H₂ .implicit hTy fun H2 => do
  let cTy ← mkAppM ``HRelC #[Iv, H1, H2]
  withLocalDecl `C .default cTy fun Cv => do
  let outer := #[Lv, memv, Iv, _ipv, H1, H2, Cv]
  let mut laws : Array (Name × Name) := #[]
  for f in fields do
    let projName := ``HydroSem ++ f
    let info ← getConstInfo projName
    let isType ← forallTelescopeReducing info.type fun _ r =>
      pure r.isSort
    if isType then continue
    let instAt := fun (h : Expr) =>
      forallBoundedTelescope info.type (some 3) fun xs body =>
        pure (body.replaceFVars xs #[Lv, memv, h])
    let T₁ ← instAt H1
    let T₂ ← instAt H2
    let body ← withLocalDecl `i .default Iv fun iv => do
      let lifted ← liftOpType Cv iv T₁ T₂ #[] #[] fun a₁ a₂ rTy => do
        let fld ← match rTy.getAppFn with
          | .const h _ =>
            match relFieldOfHead h with
            | some fl => pure fl
            | none => throwError "hydro_rel_laws: {projName} result \
                head {h} unrecognized"
          | _ => throwError "hydro_rel_laws: {projName} result head \
              is not a constant"
        let app₁ := mkAppN (mkApp3 (mkConst projName) Lv memv H1) a₁
        let app₂ := mkAppN (mkApp3 (mkConst projName) Lv memv H2) a₂
        mkAppM fld #[Cv, iv, app₁, app₂]
      mkForallFVars #[iv] lifted
    let lawName := `HydroV2.HRel ++ (`law).appendAfter s!"_{f}"
    addDefn lawName (← mkForallFVars outer (mkSort 0))
      (← mkLambdaFVars outer body)
    setReducibilityStatus lawName .reducible
    laws := laws.push (projName, lawName)
  -- the bundle
  let conjs := laws.map fun (_, ln) => mkAppN (mkConst ln) outer
  let andAll ← mkAndN conjs
  let lawsName := `HydroV2.HRel.Laws
  addDefn lawsName (← mkForallFVars outer (mkSort 0))
    (← mkLambdaFVars outer andAll)
  setReducibilityStatus lawsName .reducible
  -- accessors + registry
  let lawsApp := mkAppN (mkConst lawsName) outer
  for k in [0:laws.size] do
    let (projN, ln) := laws[k]!
    let acc ← withLocalDecl `h .default lawsApp fun hv => do
      let prf ← andProj hv k laws.size
      let accTy ← mkForallFVars (outer.push hv)
        (mkAppN (mkConst ln) outer)
      let accVal ← mkLambdaFVars (outer.push hv) prf
      pure (accTy, accVal)
    let accName := lawsName ++ projN.componentsRev.head!
    addThm accName acc.1 acc.2
    modifyEnv fun env => relLawRegistry.addEntry env (projN, accName)

end HydroGen

end HydroV2
