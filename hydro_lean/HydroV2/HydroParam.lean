import HydroV2.HydroRelLaws

/-!
# HydroV2 · `hydro_param` — per-module free theorems

The abstraction theorem for `HydroSem` programs, one module at a
time: `hydro_param M` generates, per output leg,

    M_paramₖ : ∀ …ctx… {I} [Preorder I] {H₁ H₂} (C : HRelC I H₁ H₂),
      HRel.Laws C → ∀ (i : I) …shared data args…
      …doubled wire/decision args… …relatedness hypotheses…,
      ∀ i' ≤ i, C.rel i' (legₖ (M H₁ args₁)) (legₖ (M H₂ args₂))

quantified over **every** relation whose op laws hold — so each
guarantee (eager agreement, monotonicity, causality, …, and any
future one) instantiates the same theorem; nothing is generated per
(module, guarantee) pair. All wire facts ride the uniform **strong
form** `∀ i' ≤ i, rel i' x y`, which is what lets nested knots lower
the index repeatedly with no downward-closure assumption on the
relation.

The proof is one structural walk of the module body (callees folded,
closed by their own `_param` theorems; knots closed by the
`fix_stream`/`fix_tick` laws — kernel defeq never crosses a knot or
module boundary, the D40 discipline).
-/

namespace HydroV2

namespace HydroGen

open Lean Elab Term Meta Command Tactic

/-- Generated `_param` lemmas per module (per output leg). -/
initialize paramRegistry :
    SimplePersistentEnvExtension (Name × Array Name)
      (NameMap (Array Name)) ←
  registerSimplePersistentEnvExtension {
    addImportedFn := fun as =>
      as.foldl (fun m es => es.foldl (fun m (n, i) => m.insert n i) m) {}
    addEntryFn := fun m (n, i) => m.insert n i
  }

def getParamLemmas (env : Environment) (n : Name) : Array Name :=
  ((paramRegistry.getState env).find? n).getD #[]

/-! ## The walker -/

/-- Strip projection applications (both native `.proj` and projection
functions like `Subtype.val`, `Prod.fst`, wire-bundle fields) to the
underlying head constant of a rel-goal subject. Beta-reduces first,
and unfolds reducible non-registered heads (named loop bodies like
`pcSeqF` presented by fix-law body premises): a blocked head would
otherwise defer to the un-pinned user rules — whose free ambient
index an eager `≤`-closure then pins to the OUTER bound, stranding
capture-crossing loop wires (the D46 lesson). -/
partial def subjectHead (e : Expr) : MetaM (Option Name) := do
  let env ← getEnv
  let e := e.headBeta
  match e.getAppFn with
  | .const n _ =>
    match env.getProjectionFnInfo? n with
    | some info =>
      -- signature ops are themselves projection functions of the
      -- `HydroSem` class — those ARE the heads we dispatch on
      if info.ctorName == ``HydroSem.mk then return some n
      else
        let args := e.getAppArgs
        if h : info.numParams < args.size then
          subjectHead args[info.numParams]
        else return some n
    | none =>
      if (getRelLaw env n).isSome ||
          !(getParamLemmas env n).isEmpty then
        return some n
      if (← getReducibilityStatus n) == .reducible then
        if let some e' ← unfoldDefinition? e then
          return ← subjectHead e'
      return some n
  | .proj _ _ s => subjectHead s
  | _ => return none

/-- One op-law step: on a goal `C.…Rel j x₁ x₂` whose subject head is
a signature op, apply the generated law accessor at the `HRel.Laws`
hypothesis. -/
elab "hydro_param_op" : tactic => do
  let g ← getMainGoal
  g.withContext do
    let t ← instantiateMVars (← g.getType)
    let .const relName _ := t.getAppFn
      | throwError "hydro_param_op: not a rel goal"
    unless relName.getPrefix == ``HRelC do
      throwError "hydro_param_op: not a rel goal"
    let args := t.getAppArgs
    if args.size < 2 then throwError "hydro_param_op: underapplied"
    let x₁ := args[args.size - 2]!
    let some head ← subjectHead x₁
      | throwError "hydro_param_op: subject head"
    let some acc := getRelLaw (← getEnv) head
      | throwError "hydro_param_op: no law for {head}"
    let e ← mkConstWithFreshMVarLevels acc
    let gs ← g.apply e
    replaceMainGoal gs

/-- One callee step: on a rel goal whose subject is (a leg of) a
registered module application, apply the module's `_param` lemma with
its ambient index pinned to the **goal's** index (the strong-form
hypotheses then discharge at any lower bound via `le_trans`; leaving
the index to unification lets an eager `assumption` pick the outer
bound and strand capture-crossing loop wires). -/
elab "hydro_param_callee" : tactic => do
  let g ← getMainGoal
  g.withContext do
    let t ← instantiateMVars (← g.getType)
    let .const relName _ := t.getAppFn
      | throwError "hydro_param_callee: not a rel goal"
    unless relName.getPrefix == ``HRelC do
      throwError "hydro_param_callee: not a rel goal"
    let args := t.getAppArgs
    if args.size < 3 then throwError "hydro_param_callee: underapplied"
    let jv := args[args.size - 3]!
    let x₁ := args[args.size - 2]!
    let some head ← subjectHead x₁
      | throwError "hydro_param_callee: subject head"
    let rules := getParamLemmas (← getEnv) head
    if rules.isEmpty then
      throwError "hydro_param_callee: no _param lemmas for {head}"
    for rule in rules do
      let ok ← try
        let e ← mkConstWithFreshMVarLevels rule
        let gs ← withReducible <| g.apply e
        -- pin the rule's ambient index: close the `j ≤ ?i` side goal
        -- by reflexivity, assigning ?i := the goal's index
        let mut remaining : List MVarId := []
        for g' in gs do
          if ← g'.isAssigned then continue
          let t' ← instantiateMVars (← g'.getType)
          if t'.isAppOfArity ``LE.le 4 then
            let lhs := t'.getArg! 2
            let rhs := t'.getArg! 3
            if rhs.isMVar && (← withReducible <| isDefEq lhs jv) then
              let prf ← mkAppM ``le_refl #[jv]
              -- defeq-check the assignment (pins the rule's ambient
              -- index mvar to the goal's index)
              if ← isDefEq (← g'.getType) (← inferType prf) then
                g'.assign prf
                continue
          remaining := remaining ++ [g']
        replaceMainGoal remaining
        pure true
      catch _ => pure false
      if ok then return
    throwError "hydro_param_callee: no rule applies"

/-- One leaf step: close a rel goal from a strong-form hypothesis
`∀ j ≤ _, rel j x₁ x₂`, leaving its `≤` side goal. -/
elab "hydro_param_leaf" : tactic => do
  let g ← getMainGoal
  g.withContext do
    let t ← instantiateMVars (← g.getType)
    let .const relName _ := t.getAppFn
      | throwError "hydro_param_leaf: not a rel goal"
    unless relName.getPrefix == ``HRelC do
      throwError "hydro_param_leaf: not a rel goal"
    let lctx ← getLCtx
    for d in lctx do
      if !d.isImplementationDetail then
        let ty := d.type
        if ty.isForall && ty.bindingBody!.isForall then
          let concl := ty.bindingBody!.bindingBody!
          if concl.getAppFn.isConstOf relName then
            let ok ← try
              let gs ← withReducible <| g.apply d.toExpr
              replaceMainGoal gs
              pure true
            catch _ => pure false
            if ok then return
    throwError "hydro_param_leaf: no hypothesis applies"

/-- Deterministic `≤`-chain closer: follow `lhs ≤ mid` hypotheses
forward via `le_trans` until the goal's bound is reached (the index
chains the walk produces are linear, so no search is needed). -/
partial def leChain (g : MVarId) (fuel : Nat := 12) :
    MetaM Unit := do
  if fuel == 0 then throwError "hydro_param_le: chain too deep"
  g.withContext do
    let t ← instantiateMVars (← g.getType)
    unless t.isAppOfArity ``LE.le 4 do
      throwError "hydro_param_le: not a ≤ goal"
    let lhs := t.getArg! 2
    -- direct hypothesis
    for d in ← getLCtx do
      if !d.isImplementationDetail then
        if ← withReducible (isDefEq d.type t) then
          g.assign d.toExpr
          return
    -- reflexivity
    if ← withReducible (isDefEq (t.getArg! 2) (t.getArg! 3)) then
      g.assign (← mkAppM ``le_refl #[lhs])
      return
    -- forward chain through `lhs ≤ mid` (backtracking: an index may
    -- carry several upper bounds)
    for d in ← getLCtx do
      if !d.isImplementationDetail then
        let dt ← instantiateMVars d.type
        if dt.isAppOfArity ``LE.le 4 then
          if (← withReducible (isDefEq (dt.getArg! 2) lhs)) &&
              !(← withReducible (isDefEq (dt.getArg! 3) lhs)) then
            let s ← saveState
            try
              let gs ← g.apply
                (← mkConstWithFreshMVarLevels ``le_trans)
              -- assign the first premise to `d`, recurse on the rest
              let mut rest : List MVarId := []
              let mut used := false
              for g' in gs do
                if ← g'.isAssigned then continue
                if !used && (← withReducible <| isDefEq (← g'.getType) dt) then
                  g'.assign d.toExpr
                  used := true
                else
                  rest := rest ++ [g']
              unless used do throwError "le_trans premise mismatch"
              for g' in rest do
                if ← g'.isAssigned then continue
                leChain g' (fuel - 1)
              return
            catch _ =>
              s.restore
    throwError "hydro_param_le: stuck"

/-- Close `≤` side conditions (index chains through nested lowering)
— shape-guarded so nothing here ever touches rel goals. -/
elab "hydro_param_le" : tactic => do
  let g ← getMainGoal
  liftMetaM <| leChain g
  replaceMainGoal []

/-- The free-theorem walk: intro, close leaves from hypotheses,
dispatch op laws, fold callees by their `_param` rules, close index
side conditions, normalize capture-tuple projections. -/
macro "hydro_param_walk" "[" rules:term,* "]" : tactic => do
  let userRws ← rules.getElems.mapM fun r =>
    `(tactic| with_reducible apply $r:term)
  let pre : Array (Lean.TSyntax `tactic) := #[
    ← `(tactic| intro _),
    ← `(tactic| assumption),
    ← `(tactic| exact le_refl _),
    ← `(tactic| hydro_param_leaf)]
  let post : Array (Lean.TSyntax `tactic) := #[
    ← `(tactic| hydro_param_op),
    ← `(tactic| hydro_param_callee),
    ← `(tactic| hydro_param_le)]
  let alts := pre ++ post ++ userRws ++ #[← `(tactic| dsimp only [])]
  let failTac ← `(tactic| fail "hydro_param_walk: no rule applies")
  let alt ← alts.foldrM (init := failTac) fun t acc =>
    `(tactic| first | $t:tactic | $acc:tactic)
  `(tactic| repeat' $alt:tactic)

/-! ## The generator -/

/-- Lockstep walk of a module's argument telescope at the two
interpretation sides: shared binders for `H`-free domains, doubled
binders plus strong-form relatedness hypotheses for carriers,
primitive decisions and decision records; ends in the conjunction of
the per-leg strong conclusions. -/
partial def paramTele (M : Name) (ctxArgs : Array Expr)
    (H1 H2 Cv Iv iv : Expr) (T₁ T₂ : Expr)
    (args₁ args₂ : Array Expr) : TermElabM Expr := do
  let strongRel := fun (fld : Name) (a₁ a₂ : Expr) => do
    withLocalDecl `j .default Iv fun jv => do
      let leTy ← mkAppM ``LE.le #[jv, iv]
      withLocalDecl `hj .default leTy fun hjv => do
        let r ← mkAppM fld #[Cv, jv, a₁, a₂]
        mkForallFVars #[jv, hjv] r
  match T₁ with
  | .forallE n τ₁ _ bi =>
    let τ₂ := T₂.bindingDomain!
    if τ₁ == τ₂ then
      withLocalDecl n bi τ₁ fun x => do
        let r ← paramTele M ctxArgs H1 H2 Cv Iv iv
          (T₁.bindingBody!.instantiate1 x)
          (T₂.bindingBody!.instantiate1 x)
          (args₁.push x) (args₂.push x)
        mkForallFVars #[x] r
    else
      withLocalDecl n .default τ₁ fun x₁ =>
      withLocalDecl (n.appendAfter "'") .default τ₂ fun x₂ => do
        let mut prems : Array Expr := #[]
        match τ₁.getAppFn with
        | .const h _ =>
          if let some fld := relFieldOfHead h then
            prems := prems.push (← strongRel fld x₁ x₂)
          else if isStructure (← getEnv) h
              && τ₁.getAppArgs.any (· == H1) then
            let tree ← decFieldTreeOf H1 x₁
            for (path, df) in flattenTree tree do
              let p₁ ← applyFieldPath x₁ path
              let p₂ ← applyFieldPath x₂ path
              let fld := `HydroV2.HRelC ++ relFieldOfDec df
              prems := prems.push (← strongRel fld p₁ p₂)
          else
            throwError "hydro_param: {M}: doubled arg {τ₁} unrecognized"
        | _ => throwError "hydro_param: {M}: doubled arg head"
        let hypDecls : Array (Name × BinderInfo ×
            (Array Expr → TermElabM Expr)) :=
          prems.mapIdx fun k p =>
            (Name.mkSimple s!"hrel_{args₁.size}_{k}",
             BinderInfo.default, fun _ => pure p)
        let body ← withLocalDecls hypDecls fun hs => do
          let r ← paramTele M ctxArgs H1 H2 Cv Iv iv
            (T₁.bindingBody!.instantiate1 x₁)
            (T₂.bindingBody!.instantiate1 x₂)
            (args₁.push x₁) (args₂.push x₂)
          mkForallFVars hs r
        mkForallFVars #[x₁, x₂] body
  | _ => do
    let app₁ := mkAppN (mkAppN (mkConst M) (ctxArgs.push H1)) args₁
    let app₂ := mkAppN (mkAppN (mkConst M) (ctxArgs.push H2)) args₂
    let legs ← peelLegsTy T₁ []
    let legs₂ ← peelLegsTy T₂ []
    let mut concls : Array Expr := #[]
    for k in [0:legs.size] do
      let (leg, legTy) := legs[k]!
      let legTy₂ := legs₂[k]!.2
      if legTy.isForall then
        -- function leg (applied `.val` shape): double the wire
        -- binders with strong-form premises, then peel the applied
        -- result into sub-legs (one conjunct each, own telescope)
        let resTy ← forallTelescope legTy fun _ r => pure r
        let subLegs ← peelLegsTy resTy []
        for (subPath, subTy) in subLegs do
          let fld ← match subTy.getAppFn with
            | .const h _ =>
              match relFieldOfHead h with
              | some f => pure f
              | none => throwError "hydro_param: {M}: fn sub-leg head {h}"
            | _ => throwError "hydro_param: {M}: fn sub-leg head"
          let g₁ ← applyLeg app₁ leg
          let g₂ ← applyLeg app₂ leg
          let rec walkPi (τ₁ τ₂ g₁ g₂ : Expr) (idx : Nat) :
              TermElabM Expr := do
            match τ₁ with
            | .forallE n dom₁ body₁ _ =>
              let dom₂ := τ₂.bindingDomain!
              let wn := if n.isAnonymous || n.hasMacroScopes then
                Name.mkSimple s!"w{idx}" else n
              withLocalDecl wn .default dom₁ fun w₁ =>
              withLocalDecl (wn.appendAfter "'") .default dom₂ fun w₂ => do
                let domFld ← match dom₁.getAppFn with
                  | .const h _ =>
                    match relFieldOfHead h with
                    | some f => pure f
                    | none => throwError
                        "hydro_param: {M}: fn wire head {h}"
                  | _ => throwError "hydro_param: {M}: fn wire head"
                let prem ← strongRel domFld w₁ w₂
                withLocalDecl (Name.mkSimple s!"hw{idx}") .default prem
                  fun hw => do
                  let r ← walkPi (body₁.instantiate1 w₁)
                    (τ₂.bindingBody!.instantiate1 w₂)
                    (mkApp g₁ w₁) (mkApp g₂ w₂) (idx + 1)
                  mkForallFVars #[w₁, w₂, hw] r
            | _ => do
              let l₁ ← applyLeg g₁ subPath
              let l₂ ← applyLeg g₂ subPath
              strongRel fld l₁ l₂
          concls := concls.push (← walkPi legTy legTy₂ g₁ g₂ 0)
      else
        let fld ← match legTy.getAppFn with
          | .const h _ =>
            match relFieldOfHead h with
            | some f => pure f
            | none => throwError "hydro_param: {M}: leg head {h}"
          | _ => throwError "hydro_param: {M}: leg head"
        let l₁ ← applyLeg app₁ leg
        let l₂ ← applyLeg app₂ leg
        concls := concls.push (← strongRel fld l₁ l₂)
    mkAndN concls

/-- Top-level And count. -/
partial def countAnd (e : Expr) (acc : Nat := 0) : Nat :=
  if e.isAppOfArity ``And 2 then countAnd (e.getArg! 1) (acc + 1)
  else acc + 1

/-- The `k`-th top-level conjunct. -/
partial def conjunctAt (e : Expr) (k n : Nat) : Expr :=
  if n == 1 then e
  else if k == 0 then e.getArg! 0
  else conjunctAt (e.getArg! 1) (k - 1) (n - 1)

def genParam (M : Name) (extras : Array Name) :
    TermElabM (Array Name) := do
  let info ← getConstInfo M
  let hIdx ← forallTelescope info.type fun xs _ => do
    let mut r := none
    for h : k in [0:xs.size] do
      if r.isNone then
        let t ← inferType xs[k]
        if t.getAppFn.isConstOf ``HydroSem then r := some k
    match r with
    | some k => pure k
    | none => throwError "hydro_param: {M} has no `HydroSem` binder"
  forallBoundedTelescope info.type (some hIdx) fun ctxArgs tyH => do
  let hTy := tyH.bindingDomain!
  withLocalDecl `I .implicit (mkSort 1) fun Iv => do
  let preTy ← mkAppM ``Preorder #[Iv]
  withLocalDecl `ipre .instImplicit preTy fun ipv => do
  withLocalDecl `H₁ .implicit hTy fun H1 => do
  withLocalDecl `H₂ .implicit hTy fun H2 => do
  let cTy ← mkAppM ``HRelC #[Iv, H1, H2]
  withLocalDecl `C .default cTy fun Cv => do
  let lawsTy ← mkAppM `HydroV2.HRel.Laws #[Cv]
  withLocalDecl `laws .default lawsTy fun lawsv => do
  withLocalDecl `i .default Iv fun iv => do
  let T₁ := tyH.bindingBody!.instantiate1 H1
  let T₂ := tyH.bindingBody!.instantiate1 H2
  let bundled ← paramTele M ctxArgs H1 H2 Cv Iv iv T₁ T₂ #[] #[]
  let outer := ctxArgs ++ #[Iv, ipv, H1, H2, Cv, lawsv, iv]
  let stmt ← mkForallFVars outer bundled
  -- the And of leg conclusions sits under the argument telescope
  let nLegs ← forallTelescope bundled fun _ body => pure (countAnd body)
  -- proof script for the bundle
  let gi := (getGenInfo (← getEnv) M).getD {}
  let mut rules : Array Name := #[]
  for c in gi.callees do
    rules := rules ++ getParamLemmas (← getEnv) c
  let unfolds := (#[M] ++ gi.inlines ++ extras).map toString
  let unfoldList := String.intercalate ", "
    (unfolds.toList ++
     ["HydroV2.HydroSem.fix", "HydroV2.HydroSem.fixTick"])
  let ruleList := String.intercalate ", " (rules.toList.map toString)
  let allName := M.appendAfter "_param_all"
  addThmByTac allName stmt
    s!"(intros; (repeat' apply And.intro) <;> ((try intros); \
       simp only [{unfoldList}]; hydro_param_walk [{ruleList}]))"
  -- per-leg projections (statements are the individual conjuncts,
  -- under the same telescopes)
  let mut names : Array Name := #[]
  for k in [0:nLegs] do
    let name :=
      if nLegs == 1 then M.appendAfter "_param"
      else M.appendAfter s!"_param{subscript (k + 1)}"
    let (stmtK, valK) ← forallTelescope bundled fun xs body => do
      let stmtK ← mkForallFVars (outer ++ xs) (conjunctAt body k nLegs)
      let inner ← andProj
        (mkAppN (mkConst allName) (outer ++ xs)) k nLegs
      let valK ← mkLambdaFVars (outer ++ xs) inner
      pure (stmtK, valK)
    addThm name stmtK valK
    names := names.push name
  modifyEnv fun env => paramRegistry.addEntry env (M, names)
  pure names

elab "hydro_param " M:ident ids:(hydroGenIds)? : command => do
  let mName ← liftCoreM <| realizeGlobalConstNoOverload M
  let extras ← elabExtras (ids.map (·.raw))
  let names ← liftTermElabM <| genParam mName extras
  logInfo m!"hydro_param {mName}: {names.size} lemmas"

end HydroGen

end HydroV2
