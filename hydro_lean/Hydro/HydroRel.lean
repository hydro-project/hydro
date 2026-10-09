import Hydro.HydroGen

/-!
# Hydro · `HydroRel` — the generated relational signature
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

namespace Hydro

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
  tickedRel : ∀ {ℓ : L} {σ : Type}, I →
    H₁.Ticked ℓ σ → H₂.Ticked ℓ σ → Prop
  /-- One tick's bounded stream (the in-tick carrier; `List`-vs-quotient
  coupling between the step machine and the denotation). -/
  boundedRel : ∀ {α : Type} [DecidableEq α] {ord : StrOrd}
    {ret : Retries}, I → H₁.BoundedStream α ord ret →
    H₂.BoundedStream α ord ret → Prop
  /-- This tick's singleton / optional (in-tick values; a coupled pair
  at the corner, equal legs). -/
  bsingRel : ∀ {σ : Type}, I → H₁.BoundedSingleton σ →
    H₂.BoundedSingleton σ → Prop
  /-- A ticked trace of per-tick bounded streams (the in-tick emission
  before it leaves the slice): `Ticked`'s element type is itself
  interpretation-dependent here, so the relation is its own field
  (the one nested-carrier shape in the signature). -/
  tickedBoundedRel : ∀ {ℓ : L} {α : Type} [DecidableEq α] {ord : StrOrd}
    {ret : Retries}, I →
    H₁.Ticked ℓ (H₁.BoundedStream α ord ret) →
    H₂.Ticked ℓ (H₂.BoundedStream α ord ret) → Prop
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
  sampleRel : ∀ {n : Nat}, I → H₁.SampleDec n → H₂.SampleDec n → Prop
  timerRel : ∀ {n : Nat}, I → H₁.TimerDec n → H₂.TimerDec n → Prop
  pulseRel : ∀ {n : Nat}, I → H₁.PulseDec n → H₂.PulseDec n → Prop
  fixRel : I → H₁.FixDec → H₂.FixDec → Prop

/-! ### Shaped tick carriers: the leafwise lifts (the `tick_scan`
former's input/output wires and the body's bounded tuples) -/

namespace HRelC
variable {I : Type} [Preorder I] {L : Type} {mem : L → Nat}
  {H₁ H₂ : HydroSem L mem}

/-- Leafwise relatedness of two shaped wire tuples. -/
@[reducible] def tickedOfRel (C : HRelC I H₁ H₂) (i : I) {ℓ : L} :
    (sh : TickShape) → TickedOf H₁.Ticked H₁.TickStream ℓ sh →
    TickedOf H₂.Ticked H₂.TickStream ℓ sh → Prop
  | .sing _, x, y => C.tickedRel i x y
  | .stream _ _ _ _, x, y => C.tickStreamRel i x y
  | .pair a b, x, y => tickedOfRel C i a x.1 y.1 ∧ tickedOfRel C i b x.2 y.2

/-- Leafwise relatedness of two shaped in-tick tuples. -/
@[reducible] def boundedOfRel (C : HRelC I H₁ H₂) (i : I) :
    (sh : TickShape) → BoundedOf H₁.BoundedSingleton H₁.BoundedStream sh →
    BoundedOf H₂.BoundedSingleton H₂.BoundedStream sh → Prop
  | .sing _, x, y => C.bsingRel i x y
  | .stream _ _ _ _, x, y => C.boundedRel i x y
  | .pair a b, x, y => boundedOfRel C i a x.1 y.1 ∧ boundedOfRel C i b x.2 y.2

theorem tickedOfRel_fst (C : HRelC I H₁ H₂) (i : I) {ℓ : L} (a b : TickShape)
    {x : TickedOf H₁.Ticked H₁.TickStream ℓ (.pair a b)}
    {y : TickedOf H₂.Ticked H₂.TickStream ℓ (.pair a b)}
    (h : tickedOfRel C i (.pair a b) x y) : tickedOfRel C i a x.1 y.1 := h.1
theorem tickedOfRel_snd (C : HRelC I H₁ H₂) (i : I) {ℓ : L} (a b : TickShape)
    {x : TickedOf H₁.Ticked H₁.TickStream ℓ (.pair a b)}
    {y : TickedOf H₂.Ticked H₂.TickStream ℓ (.pair a b)}
    (h : tickedOfRel C i (.pair a b) x y) : tickedOfRel C i b x.2 y.2 := h.2
theorem boundedOfRel_fst (C : HRelC I H₁ H₂) (i : I) (a b : TickShape)
    {x : BoundedOf H₁.BoundedSingleton H₁.BoundedStream (.pair a b)}
    {y : BoundedOf H₂.BoundedSingleton H₂.BoundedStream (.pair a b)}
    (h : boundedOfRel C i (.pair a b) x y) : boundedOfRel C i a x.1 y.1 := h.1
theorem boundedOfRel_snd (C : HRelC I H₁ H₂) (i : I) (a b : TickShape)
    {x : BoundedOf H₁.BoundedSingleton H₁.BoundedStream (.pair a b)}
    {y : BoundedOf H₂.BoundedSingleton H₂.BoundedStream (.pair a b)}
    (h : boundedOfRel C i (.pair a b) x y) : boundedOfRel C i b x.2 y.2 := h.2

end HRelC

namespace HydroGen

open Lean Elab Term Meta Command

/-- The `HRelC` field for a carrier family. -/
def relFieldOfCarrier : CarrierFam → Name
  | .stream => `streamRel
  | .keyed => `keyedRel
  | .sing => `singRel
  | .ticked => `tickedRel
  | .tickStream => `tickStreamRel
  | .bounded => `boundedRel
  | .bsing => `bsingRel
  | .bopt => `bsingRel

/-- The `HRelC` field for a decision family. -/
def relFieldOfDec : DecFam → Name
  | .transport => `transportRel
  | .orderSel => `orderSelRel
  | .snap => `snapRel
  | .batch => `batchRel
  | .ordBatch => `ordBatchRel
  | .sample => `sampleRel
  | .timer => `timerRel
  | .pulse => `pulseRel
  | .fixd => `fixRel

/-- The `HRelC` relation field for a carrier- or decision-headed type,
by its head constant. -/
def relFieldOfHead (n : Name) : Option Name :=
  match carrierFamOfHead n with
  | some cf => some (`Hydro.HRelC ++ relFieldOfCarrier cf)
  | none =>
    match decFamOfHead n with
    | some df => some (`Hydro.HRelC ++ relFieldOfDec df)
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

/-- The relation between two values of a pair of (side-1 / side-2) types,
by the types' structure: carrier/decision heads by their `HRelC`
field, the shaped tick carriers (`TickedOf`/`BoundedOf`) by their
leafwise lifts, products componentwise, non-dependent functions
pointwise (shared domain) or by relatedness-preservation (doubled
domain), and `H`-free types by equality. `none` when the shape is not
recognized. -/
partial def relOfTypes (Cv iv : Expr) (τ₁ τ₂ : Expr) (a₁ a₂ : Expr) :
    TermElabM (Option Expr) := do
  if τ₁ == τ₂ then
    return some (← mkEq a₁ a₂)
  match τ₁.getAppFn with
  | .const h _ =>
    if h == ``HydroSem.Ticked
        && (τ₁.getAppArgs.back!.getAppFn.constName?
            == some ``HydroSem.BoundedStream) then
      return some (← mkAppM ``HRelC.tickedBoundedRel #[Cv, iv, a₁, a₂])
    if let some fld := relFieldOfHead h then
      return some (← mkAppM fld #[Cv, iv, a₁, a₂])
    if h == ``TickedOf then
      let sh := τ₁.getAppArgs.back!
      return some (← mkAppM ``HRelC.tickedOfRel #[Cv, iv, sh, a₁, a₂])
    if h == ``BoundedOf then
      let sh := τ₁.getAppArgs.back!
      return some (← mkAppM ``HRelC.boundedOfRel #[Cv, iv, sh, a₁, a₂])
    if h == ``Prod && τ₁.getAppNumArgs == 2 && τ₂.getAppNumArgs == 2 then
      let some r₁ ← relOfTypes Cv iv (τ₁.getArg! 0) (τ₂.getArg! 0)
        (← mkAppM ``Prod.fst #[a₁]) (← mkAppM ``Prod.fst #[a₂]) | return none
      let some r₂ ← relOfTypes Cv iv (τ₁.getArg! 1) (τ₂.getArg! 1)
        (← mkAppM ``Prod.snd #[a₁]) (← mkAppM ``Prod.snd #[a₂]) | return none
      return some (mkApp2 (mkConst ``And) r₁ r₂)
    return none
  | .forallE n dom₁ cod₁ _ =>
    if cod₁.hasLooseBVars then return none
    let dom₂ := τ₂.bindingDomain!
    let cod₂ := τ₂.bindingBody!
    if dom₁ == dom₂ then
      let r ← withLocalDecl n .default dom₁ fun y => do
        let some r ← relOfTypes Cv iv cod₁ cod₂ (mkApp a₁ y) (mkApp a₂ y)
          | return none
        return some (← mkForallFVars #[y] r)
      return r
    else
      let r ← withLocalDecl n .default dom₁ fun y₁ =>
        withLocalDecl (n.appendAfter "'") .default dom₂ fun y₂ => do
          let some hy ← relOfTypes Cv iv dom₁ dom₂ y₁ y₂ | return none
          withLocalDecl (n.appendAfter "_rel") .default hy fun hyv => do
            let some r ← relOfTypes Cv iv cod₁ cod₂ (mkApp a₁ y₁) (mkApp a₂ y₂)
              | return none
            return some (← mkForallFVars #[y₁, y₂, hyv] r)
      return r
  | _ => return none

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
    (mkResult : Array Expr → Array Expr → Expr → Expr → TermElabM Expr) :
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
            if h == ``HydroSem.Ticked
                && (τ₁.getAppArgs.back!.getAppFn.constName?
                    == some ``HydroSem.BoundedStream) then
              -- the nested carrier: a ticked trace of per-tick bounded
              -- streams (interpretation-dependent element type)
              pure (some (← mkAppM ``HRelC.tickedBoundedRel
                #[Cv, iv, x₁, x₂]))
            else if let some fld := relFieldOfHead h then
              pure (some (← mkAppM fld #[Cv, iv, x₁, x₂]))
            else if let some p ← relOfTypes Cv iv τ₁ τ₂ x₁ x₂ then
              -- shaped tick carriers (`TickedOf`/`BoundedOf`)
              pure (some p)
            else
              throwError "hydro_rel_laws: doubled binder {τ₁} has \
                unrecognized head {h}"
          else if τ₁.isForall && !τ₁.bindingBody!.hasLooseBVars
              && τ₁.bindingDomain! == τ₂.bindingDomain!
              && (match τ₁.bindingBody!.getAppFn with
                  | .const h _ => (carrierFamOfHead h).isSome
                  | _ => false) then
            -- a plain-domain closure returning a carrier (in-tick
            -- `flat_map_unordered`'s per-element bounded streams):
            -- pointwise relatedness at the ambient index
            let dom := τ₁.bindingDomain!
            let cod₁ := τ₁.bindingBody!
            let relFld ← do
              match cod₁.getAppFn with
              | .const h _ =>
                match relFieldOfHead h with
                | some f => pure f
                | none => throwError "hydro_rel_laws: closure codomain \
                    head unrecognized"
              | _ => throwError "hydro_rel_laws: closure codomain"
            let p ← withLocalDecl `y .default dom fun y => do
              let concl ← mkAppM relFld #[Cv, iv, mkApp x₁ y, mkApp x₂ y]
              mkForallFVars #[y] concl
            pure (some p)
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
          else if let some p ← relOfTypes Cv iv τ₁ τ₂ x₁ x₂ then
            -- the general structural lift (shaped tick carriers, tick
            -- bodies: multi-argument functions into products)
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
    -- result: relate the two op applications at `i` (the result types
    -- name the relation)
    mkResult args₁ args₂ T₁ T₂

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
      let lifted ← liftOpType Cv iv T₁ T₂ #[] #[] fun a₁ a₂ rTy₁ rTy₂ => do
        let app₁ := mkAppN (mkApp3 (mkConst projName) Lv memv H1) a₁
        let app₂ := mkAppN (mkApp3 (mkConst projName) Lv memv H2) a₂
        match ← relOfTypes Cv iv rTy₁ rTy₂ app₁ app₂ with
        | some r => pure r
        | none =>
          -- a plain result type (in-tick `count`/`fold`/`first`: a
          -- value, not a carrier) — related by equality
          mkEq app₁ app₂
      mkForallFVars #[iv] lifted
    let lawName := `Hydro.HRel ++ (`law).appendAfter s!"_{f}"
    addDefn lawName (← mkForallFVars outer (mkSort 0))
      (← mkLambdaFVars outer body)
    setReducibilityStatus lawName .reducible
    laws := laws.push (projName, lawName)
  -- the bundle
  let conjs := laws.map fun (_, ln) => mkAppN (mkConst ln) outer
  let andAll ← mkAndN conjs
  let lawsName := `Hydro.HRel.Laws
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

end Hydro
