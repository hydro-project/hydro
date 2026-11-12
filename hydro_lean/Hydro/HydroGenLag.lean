import Hydro.HydroGen
import Hydro.Graph.Causal

/-!
# Hydro · the graph lag analysis, computed by the generator and proved one node at a time

`Graph/Causal.lean` defines the lag arithmetic `Dag.lagOf` and the validity check
`Dag.valid` as structural recursions over the typed graph. They specify the analysis;
nothing evaluates them on a production graph, because a kernel evaluation of the whole
graph does not share: the environment-passing recursion re-runs a wire's producer at
every read of the wire's lag (exponential in the fan-out along a chain), and a `brecOn`
over a hundred-node graph rebuilds its `below` towers per projection, so a whole-graph
`rfl` exhausts memory.

The generator therefore walks the graph literal itself (`Expr`, constructor by
constructor), computes every lag as a `Nat`, and proves `Dag.lagOf d l = r` and
`Dag.valid d = true` by the per-node lemmas (`Dag.lagOf_letN_lit`, `Node.lagOf_call_lit`,
`Node.lagOf_fixN_lit`, `Dag.valid_*_lit`, `Node.valid_*_lit`) with every environment and
result a literal `HList` of numerals. Each kernel judgement is one node's rule on literal
inputs; a plain node's `Node.lagOf n l = ln` is `Eq.refl`, one constructor step and a
`get` on a literal. A callee graph (`N.dag kept`) is unfolded (delta) and walked; its
own generated `N.dag_valid` proves its validity. The cascade check decides a knot block's
validity (`cascadeValid`, `Graph/Lags.lean`): knot by knot, with the earlier knots closed
(`LagFun.next`), the body's lag functional (`Dag.knotLag`) is evaluated at the knot's own
probe. Each evaluation is one `Dag.lagOf` chain (`Dag.knotLag_lit`, `LagFun.next_lit`),
`2^i` of them for knot `i`, memoized per probe.

Consumers: `HydroGenProject.genProject` (`M.dag_valid`) and `HydroGenKnot.composedStrict`
(the output lags of a knot's chain at a lag vector), through `proveDagLag` and
`proveDagValid`.
-/

namespace Hydro

namespace HydroGen

open Lean Elab Term Meta

/-! ## Literals -/

/-- The kinds of a context literal (a `List.cons` chain; `whnf` opens `List.map` and `++`
spellings). -/
partial def ctxKinds (Γ : Expr) : MetaM (Array Expr) := do
  let mut ks : Array Expr := #[]
  let mut cur := Γ
  while true do
    let cur' ← whnf cur
    if cur'.isAppOfArity ``List.cons 3 then
      ks := ks.push cur'.getAppArgs[1]!
      cur := cur'.getAppArgs[2]!
    else if cur'.isAppOfArity ``List.nil 1 then
      break
    else
      throwError "HydroGen: context is not a list literal:{indentExpr cur'}"
  pure ks

/-- An `HList f Γ` literal over a context literal. -/
def hlistLitOf (L mem f : Expr) (Γ : Expr) (vals : Array Expr) : MetaM Expr := do
  let ks ← ctxKinds Γ
  unless ks.size == vals.size do
    throwError "HydroGen: hlistLitOf: {ks.size} kinds vs {vals.size} values"
  let kindTy := mkApp2 (mkConst ``Hydro.Project.Kind) L mem
  let mut acc := mkApp3 (mkConst ``Hydro.Project.HList.nil [Level.zero]) L mem f
  for i in [0:ks.size] do
    let j := ks.size - 1 - i
    let restKs := (ks.extract (j + 1) ks.size).foldr
      (fun k a => mkApp3 (mkConst ``List.cons [Level.one]) kindTy k a)
      (mkApp (mkConst ``List.nil [Level.one]) kindTy)
    acc := mkAppN (mkConst ``Hydro.Project.HList.cons [Level.zero]) #[L, mem, f, ks[j]!, restKs, vals[j]!, acc]
  pure acc

/-- The lag element function `fun _ : Kind L mem => Nat`. -/
def lagsFn (L mem : Expr) : Expr :=
  Expr.lam `κ (mkApp2 (mkConst ``Hydro.Project.Kind) L mem) (mkConst ``Nat) .default

/-- A `Lags Γ` literal. -/
def lagsLit (L mem Γ : Expr) (vals : Array Nat) : MetaM Expr :=
  hlistLitOf L mem (lagsFn L mem) Γ (vals.map mkNatLit)

/-- The index of a `Var` literal (`Var.succ^n Var.zero`). -/
partial def varIdx (v : Expr) : MetaM Nat := do
  let v ← whnf v
  if v.isAppOf ``Hydro.Project.Var.zero then pure 0
  else if v.isAppOf ``Hydro.Project.Var.succ then
    pure ((← varIdx v.appArg!) + 1)
  else throwError "HydroGen: not a variable literal:{indentExpr v}"

/-- The variables of a shaped or tupled variable term (`VarsOf`: `Prod.mk` nests of
`Var`s); `none` for anything else (a data payload, which may itself be a product). The
inferred type of a tuple term is a product, never the constructor binder's `VarsOf`, so
the term is inspected. -/
partial def collectVars? (e : Expr) : MetaM (Option (Array Nat)) := do
  let e := e.consumeMData
  let ty ← whnf (← inferType e)
  if ty.isAppOf ``Hydro.Project.Var then return some #[← varIdx e]
  let e' ← whnf e
  if e'.isAppOfArity ``Prod.mk 4 then
    let some a ← collectVars? e'.getAppArgs[2]! | return none
    let some b ← collectVars? e'.getAppArgs[3]! | return none
    return some (a ++ b)
  return none

partial def collectVars (e : Expr) : MetaM (Array Nat) := do
  let some vs ← collectVars? e | throwError "HydroGen: not a variable term:{indentExpr e}"
  pure vs

/-- The variables of an `HList (Var Γ) κs` literal (a substitution, or the returned
wires). -/
partial def hlistVars (xs : Expr) : MetaM (Array Nat) := do
  let xs' ← whnf xs
  if xs'.isAppOfArity ``Hydro.Project.HList.cons 7 then
    let a := xs'.getAppArgs
    return #[← varIdx a[5]!] ++ (← hlistVars a[6]!)
  if xs'.isAppOf ``Hydro.Project.HList.nil then return #[]
  if xs'.isAppOfArity ``Prod.mk 4 then
    let a := xs'.getAppArgs
    return #[← varIdx a[2]!] ++ (← hlistVars a[3]!)
  if xs'.isAppOf ``PUnit.unit then return #[]
  throwError "HydroGen: not a variable list literal:{indentExpr xs'}"

/-- The length of a list literal. -/
partial def listLen (l : Expr) : MetaM Nat := do
  let l' ← whnf l
  if l'.isAppOfArity ``List.cons 3 then pure ((← listLen l'.getAppArgs[2]!) + 1)
  else if l'.isAppOfArity ``List.nil 1 then pure 0
  else throwError "HydroGen: not a list literal:{indentExpr l'}"

/-! ## The walk -/

/-- The strict node heads: a network hop or a `defer_tick` reads its input one step back.
-/
def strictHeads : List Name :=
  [``Hydro.Project.Node.broadcast_closed, ``Hydro.Project.Node.demux, ``Hydro.Project.Node.defer_tick]

/-- The analysis memo. A graph at an environment is walked once; a wire read many times
reuses its result and its proof. -/
structure LagSt where
  /-- `(graph, input lags) ↦ (output lags, proof of `Dag.lagOf d l = r`)`. -/
  memo : Std.HashMap (Expr × List Nat) (Array Nat × Expr) := {}
  /-- `graph ↦ proof of `Dag.valid d = true``. -/
  validMemo : Std.HashMap Expr Expr := {}

abbrev LagM := StateRefT LagSt TermElabM

/-- `L`, `mem`, `Γ`, `outs` of a graph term. -/
def dagParams (d : Expr) : MetaM (Expr × Expr × Expr × Expr) := do
  let ty ← whnf (← inferType d)
  unless ty.isAppOfArity ``Hydro.Project.Dag 4 do
    throwError "HydroGen: not a graph:{indentExpr ty}"
  let a := ty.getAppArgs
  pure (a[0]!, a[1]!, a[2]!, a[3]!)

def nodeParams (n : Expr) : MetaM (Expr × Expr × Expr × Expr) := do
  let ty ← whnf (← inferType n)
  unless ty.isAppOfArity ``Hydro.Project.Node 4 do
    throwError "HydroGen: not a node:{indentExpr ty}"
  let a := ty.getAppArgs
  pure (a[0]!, a[1]!, a[2]!, a[3]!)

def maxArr (xs : Array Nat) : Nat := xs.foldl max 0

/-- `Eq.refl r` ascribed `lhs = r`; the kernel evaluates `lhs`. -/
def rflEq (lhs r : Expr) : MetaM Expr := do
  mkExpectedTypeHint (← mkEqRefl r) (← mkEq lhs r)

mutual

/-- The lags of a node's outputs at a literal environment, with its proof
(`Node.lagOf n lLit = lnLit`). A `call` recurses into the callee graph at the argument
lags; a `fixN` evaluates the body with every knot wire at the maximum input lag; a plain
node takes the maximum over its variable arguments, one less at a strict head. -/
partial def nodeLag (n : Expr) (env : Array Nat) (lLit : Expr) : LagM (Array Nat × Expr) := do
  let n := n.consumeMData
  let (L, mem, _Γ, κs) ← nodeParams n
  let .const c _ := n.getAppFn | throwError "HydroGen: node head is not a constructor:{indentExpr n}"
  let args := n.getAppArgs
  if c == ``Hydro.Project.Node.call then
    -- `call {Γ} {Γ'} {outs} N args`
    let N := args[args.size - 2]!
    let sub := args[args.size - 1]!
    let vars ← hlistVars sub
    let argLags := vars.map fun v => env[v]!
    let (_, _, Γ', _) ← dagParams N
    let l'Lit ← lagsLit L mem Γ' argLags
    let (r, prfN) ← dagLag N argLags l'Lit
    let rLit ← lagsLit L mem κs r
    let f := lagsFn L mem
    let getFn ← withLocalDecl `κ .implicit (mkApp2 (mkConst ``Hydro.Project.Kind) L mem) fun κE =>
      withLocalDeclD `v (mkAppN (mkConst ``Hydro.Project.Var) #[L, mem, _Γ, κE]) fun vE => do
        mkLambdaFVars #[κE, vE] (← mkAppOptM ``Hydro.Project.HList.get #[L, mem, f, _Γ, κE, lLit, vE])
    let mapped ← mkAppM ``Hydro.Project.HList.map #[getFn, sub]
    let hargs ← rflEq mapped l'Lit
    let prf ← mkAppM ``Hydro.Project.Node.lagOf_call_lit #[N, sub, lLit, l'Lit, rLit, hargs, prfN]
    pure (r, prf)
  else if c == ``Hydro.Project.Node.fixN then
    -- `fixN {Γ} ks {outs} body`
    let ks := args[args.size - 3]!
    let body := args[args.size - 1]!
    let nks ← listLen ks
    let Lg := maxArr env
    let (_, _, Γb, outsB) ← dagParams body
    let envB := (Array.replicate nks Lg) ++ env
    let l'Lit ← lagsLit L mem Γb envB
    let (r, prfB) ← dagLag body envB l'Lit
    let rLit ← lagsLit L mem outsB r
    let res := (Array.replicate nks Lg) ++ r.extract nks r.size
    let resLit ← lagsLit L mem κs res
    let LgE := mkNatLit Lg
    let ksKinds ← mkAppM ``List.map #[mkApp2 (mkConst ``Hydro.Project.KnotKind.toKind) L mem, ks]
    let hLg ← rflEq (← mkAppM ``Hydro.Project.Lags.maxAll #[lLit]) LgE
    let constE ← mkAppM ``Hydro.Project.Lags.const #[ksKinds, LgE]
    let henv ← rflEq (← mkAppM ``Hydro.Project.HList.append #[constE, lLit]) l'Lit
    let dropE ← mkAppOptM ``Hydro.Project.HList.drop #[L, mem, lagsFn L mem, ksKinds, none, rLit]
    let hres ← rflEq (← mkAppM ``Hydro.Project.HList.append #[constE, dropE]) resLit
    let prf ← mkAppM ``Hydro.Project.Node.lagOf_fixN_lit
      #[ks, body, lLit, LgE, l'Lit, rLit, resLit, hLg, henv, prfB, hres]
    pure (res, prf)
  else
    -- a plain node: the max over its variable arguments, one off at a strict head
    let mut vars : Array Nat := #[]
    for a in args do
      if let some vs ← collectVars? a then vars := vars ++ vs
    let base := if vars.isEmpty then 0 else maxArr (vars.map fun v => env[v]!)
    let lag := if strictHeads.contains c then base - 1 else base
    let nOut ← listLen κs
    let res := Array.replicate nOut lag
    let resLit ← lagsLit L mem κs res
    let prf ← rflEq (← mkAppM ``Hydro.Project.Node.lagOf #[n, lLit]) resLit
    pure (res, prf)

/-- The lags of a graph's outputs at a literal environment, with its proof
(`Dag.lagOf d lLit = rLit`), memoized on `(graph, environment)`. -/
partial def dagLag (d : Expr) (env : Array Nat) (lLit : Expr) : LagM (Array Nat × Expr) := do
  let d := d.consumeMData
  if let some hit := (← get).memo.get? (d, env.toList) then return hit
  let (L, mem, Γ, outs) ← dagParams d
  let res ← match d with
    | .letE nm ty v b _ =>
      withLetDecl nm ty v fun x => do
        let (r, prf) ← dagLag (b.instantiate1 x) env lLit
        pure (r, ← mkLetFVars #[x] prf)
    | _ =>
      let .const c _ := d.getAppFn
        | throwError "HydroGen: graph head is not a constructor or constant:{indentExpr d}"
      let args := d.getAppArgs
      if c == ``Hydro.Project.Dag.letN then
        let n := args[args.size - 2]!
        let rest := args[args.size - 1]!
        let (ln, p1) ← nodeLag n env lLit
        let (_, _, Γr, _) ← dagParams rest
        let env' := ln ++ env
        let l'Lit ← lagsLit L mem Γr env'
        let (_, _, _, κs) ← nodeParams n
        let lnLit ← lagsLit L mem κs ln
        let h0 ← rflEq (← mkAppM ``Hydro.Project.HList.append #[lnLit, lLit]) l'Lit
        let (r, p2) ← dagLag rest env' l'Lit
        let rLit ← lagsLit L mem outs r
        let prf ← mkAppM ``Hydro.Project.Dag.lagOf_letN_lit #[n, rest, lLit, lnLit, l'Lit, rLit, p1, h0, p2]
        pure (r, prf)
      else if c == ``Hydro.Project.Dag.ret then
        let xs := args[args.size - 1]!
        let vars ← hlistVars xs
        let r := vars.map fun v => env[v]!
        let rLit ← lagsLit L mem outs r
        let f := lagsFn L mem
        let getFn ← withLocalDecl `κ .implicit (mkApp2 (mkConst ``Hydro.Project.Kind) L mem) fun κE =>
          withLocalDeclD `v (mkAppN (mkConst ``Hydro.Project.Var) #[L, mem, Γ, κE]) fun vE => do
            mkLambdaFVars #[κE, vE] (← mkAppOptM ``Hydro.Project.HList.get #[L, mem, f, Γ, κE, lLit, vE])
        let mapped ← mkAppM ``Hydro.Project.HList.map #[getFn, xs]
        let h ← rflEq mapped rLit
        let prf ← mkAppM ``Hydro.Project.Dag.lagOf_ret_lit #[xs, lLit, rLit, h]
        pure (r, prf)
      else
        -- a callee graph constant (`N.dag kept`): unfold and walk
        let some d' ← unfoldDefinition? d
          | throwError "HydroGen: cannot unfold graph constant {c}"
        let (r, prf) ← dagLag d'.headBeta env lLit
        let rLit ← lagsLit L mem outs r
        let prf ← mkExpectedTypeHint prf (← mkEq (← mkAppM ``Hydro.Project.Dag.lagOf #[d, lLit]) rLit)
        pure (r, prf)
  modify fun st => { st with memo := st.memo.insert (d, env.toList) res }
  pure res

end

mutual

/-- A proof of `Node.valid n = true`. A `call` is valid by the callee's validity; a `fixN`
by the cascade check over its knots and the body's validity; a plain node by `Eq.refl`. -/
partial def nodeValid (n : Expr) : LagM Expr := do
  let n := n.consumeMData
  let (L, mem, Γ, _κs) ← nodeParams n
  let .const c _ := n.getAppFn | throwError "HydroGen: node head is not a constructor:{indentExpr n}"
  let args := n.getAppArgs
  if c == ``Hydro.Project.Node.call then
    let N := args[args.size - 2]!
    let sub := args[args.size - 1]!
    mkAppM ``Hydro.Project.Node.valid_call_lit #[N, sub, ← dagValid N]
  else if c == ``Hydro.Project.Node.fixN then
    -- the cascade check: knot by knot, the earlier knots closed, strict in its own wire
    let ks := args[args.size - 3]!
    let body := args[args.size - 1]!
    let nks ← listLen ks
    let nΓ := (← ctxKinds Γ).size
    let (_, _, Γb, outsB) ← dagParams body
    let ksKinds ← mkAppM ``List.map #[mkApp2 (mkConst ``Hydro.Project.KnotKind.toKind) L mem, ks]
    let l0 ← mkAppM ``Hydro.Project.Lags.const #[Γ, mkNatLit 0]
    let Φ0 ← mkAppM ``Hydro.Project.Dag.knotLag #[ks, body, l0]
    -- `evalPhi Φ p`: the knot-output lags of the lag functional `Φ` (a `Dag.knotLag` or a
    -- `LagFun.next` of one) at the literal knot lags `p`, with the proof `Φ pLit = rLit`
    let rec evalPhi (Φ : Expr) (K : Expr) (p : Array Nat) : LagM (Array Nat × Expr) := do
      let pLit ← lagsLit L mem K p
      let Φ' := Φ.consumeMData
      if Φ'.isAppOfArity ``Hydro.Project.Dag.knotLag 7 then
        -- `knotLag ks body l p = take (lagOf body (p ++ l))`
        let x := p ++ (Array.replicate nΓ 0)
        let xLit ← lagsLit L mem Γb x
        let hx ← rflEq (← mkAppM ``Hydro.Project.HList.append #[pLit, l0]) xLit
        let (y, hy) ← dagLag body x xLit
        let yLit ← lagsLit L mem outsB y
        let r := y.extract 0 nks
        let rLit ← lagsLit L mem K r
        let takeE ← mkAppOptM ``Hydro.Project.HList.take #[L, mem, lagsFn L mem, K, none, yLit]
        let hr ← rflEq takeE rLit
        let prf ← mkAppM ``Hydro.Project.Dag.knotLag_lit #[ks, body, l0, pLit, xLit, yLit, rLit, hx, hy, hr]
        pure (r, prf)
      else if Φ'.isAppOfArity ``Hydro.Project.LagFun.next 5 then
        -- `next Φ₁ p = (Φ₁ ((Φ₁ (0 :: p)).1 :: p)).2`
        let Φ₁ := Φ'.appArg!
        let κ := Φ'.getAppArgs[2]!
        let κs := Φ'.getAppArgs[3]!
        let K₁ ← mkAppM ``List.cons #[κ, κs]
        let (a, ha) ← evalPhi Φ₁ K₁ (#[0] ++ p)
        let aLit ← lagsLit L mem K₁ a
        let q := #[a[0]!] ++ p
        let qLit ← lagsLit L mem K₁ q
        let hq ← rflEq (← mkAppOptM ``Hydro.Project.HList.cons #[L, mem, lagsFn L mem, κ, κs, ← mkAppM ``Prod.fst #[aLit], pLit]) qLit
        let (b, hb) ← evalPhi Φ₁ K₁ q
        let bLit ← lagsLit L mem K₁ b
        let r := b.extract 1 b.size
        let rLit ← lagsLit L mem K r
        let hr ← rflEq (← mkAppM ``Prod.snd #[bLit]) rLit
        let prf ← mkAppM ``Hydro.Project.LagFun.next_lit #[Φ₁, pLit, aLit, qLit, bLit, rLit, ha, hq, hb, hr]
        pure (r, prf)
      else throwError "HydroGen: not a knot lag functional:{indentExpr Φ'}"
    -- `cascadeValid K Φ = true`, one knot at a time
    let rec cascade (Φ : Expr) (K : Expr) (i : Nat) : LagM Expr := do
      let K' ← whnf K
      if K'.isAppOfArity ``List.nil 1 then
        mkAppM ``Hydro.Project.cascadeValid_nil #[Φ]
      else
        let κ := K'.getAppArgs[1]!
        let κs := K'.getAppArgs[2]!
        let n := nks - i
        let probe := #[1] ++ Array.replicate (n - 1) 0
        let pLit ← lagsLit L mem K' probe
        let hp ← rflEq (← mkAppOptM ``Hydro.Project.HList.cons
          #[L, mem, lagsFn L mem, κ, κs, mkNatLit 1, ← mkAppM ``Hydro.Project.Lags.const #[κs, mkNatLit 0]]) pLit
        let (r, hr) ← evalPhi Φ K' probe
        unless r[0]! == 0 do
          throwError "HydroGen: knot {i} of a knot block is not strict in its own wire once the earlier \
            knots are closed (knot output lags {r} at its own wire at lag 1, the rest at 0) — a cycle \
            without a network hop or `defer_tick`:{indentExpr body}"
        let rLit ← lagsLit L mem K' r
        let h0 ← rflEq (← mkAppM ``Prod.fst #[rLit]) (mkNatLit 0)
        let Φnext ← mkAppM ``Hydro.Project.LagFun.next #[Φ]
        let hrest ← cascade Φnext κs (i + 1)
        mkAppM ``Hydro.Project.cascadeValid_cons_lit #[Φ, pLit, rLit, hp, hr, h0, hrest]
    let hc ← cascade Φ0 ksKinds 0
    let hv ← dagValid body
    mkAppM ``Hydro.Project.Node.valid_fixN_lit #[ks, body, hc, hv]
  else
    rflEq (← mkAppM ``Hydro.Project.Node.valid #[n]) (mkConst ``Bool.true)

/-- A proof of `Dag.valid d = true`, memoized per graph. A callee constant's proof is its
own generated `N.dag_valid` when it exists; otherwise the constant is unfolded and
walked. -/
partial def dagValid (d : Expr) : LagM Expr := do
  let d := d.consumeMData
  if let some hit := (← get).validMemo.get? d then return hit
  let res ← match d with
    | .letE nm ty v b _ =>
      withLetDecl nm ty v fun x => do
        let prf ← dagValid (b.instantiate1 x)
        mkLetFVars #[x] prf
    | _ =>
      let .const c _ := d.getAppFn
        | throwError "HydroGen: graph head is not a constructor or constant:{indentExpr d}"
      let args := d.getAppArgs
      if c == ``Hydro.Project.Dag.letN then
        let n := args[args.size - 2]!
        let rest := args[args.size - 1]!
        mkAppM ``Hydro.Project.Dag.valid_letN_lit #[n, rest, ← nodeValid n, ← dagValid rest]
      else if c == ``Hydro.Project.Dag.ret then
        mkAppM ``Hydro.Project.Dag.valid_ret_lit #[args[args.size - 1]!]
      else
        -- `N.dag kept`: the callee's own validity theorem
        let validName := match c with
          | .str p "dag" => p ++ `dag_valid
          | _ => c ++ `dag_valid
        if (← getEnv).contains validName then
          pure (mkAppN (mkConst validName) args)
        else
          let some d' ← unfoldDefinition? d
            | throwError "HydroGen: cannot unfold graph constant {c}"
          let prf ← dagValid d'.headBeta
          mkExpectedTypeHint prf (← mkEq (← mkAppM ``Hydro.Project.Dag.valid #[d]) (mkConst ``Bool.true))
  modify fun st => { st with validMemo := st.validMemo.insert d res }
  pure res

end

/-- `Dag.lagOf d lLit = rLit` for a graph at literal input lags `env`: the result lags and
the proof. -/
def proveDagLag (d : Expr) (env : Array Nat) : TermElabM (Array Nat × Expr) := do
  let (L, mem, Γ, _) ← dagParams d
  let lLit ← lagsLit L mem Γ env
  (dagLag d env lLit).run' {}

/-- A proof of `Dag.valid d = true`. -/
def proveDagValid (d : Expr) : TermElabM Expr :=
  (dagValid d).run' {}

end HydroGen

end Hydro
