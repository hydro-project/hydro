---
sidebar_position: 6
---

# The Glue Phase

For each output `i` of a module, the generator proves two **run equations** that identify output `i` of the module's corner run with a plain run: the `sr` equation `M_co_srᵢ` says `(M (CoupleSem …) args).ᵢ.sr` is `M (SchedSem …)` at the machine arguments, and the `rr` equation `M_co_rrᵢ` says `.rr` is `M (Values …)` at a derived decision record ([The Couple Phase](./couple-phase.md)). Most modules call other modules, and a **callee** already has its own run equations. The generator has to prove the caller's run equations from them, with every callee kept **folded**: applied by name, never unfolded into its body.

The generator never unfolds a callee, because a callee may contain a knot, and definitional unfolding through nested knots is exponential in their depth ([the measurements](../corner/projections-and-tactics.md#rewriting-through-knots)). Rewriting with folded callees has two failure modes of its own. `simp` cannot rewrite a callee redex in every position it occurs: an argument whose type a later argument's type mentions has no congruence motive. And any unification that fails against a folded callee makes Lean's unifier try to unfold it, so a near miss enters the exponential walk again.

The **glue engine** (`hydro_lean/Hydro/HydroGenKnot.lean`) addresses both. It normalizes each side of a run equation independently, by zeta, `simp` and a keyed residual pass that rewrites what `simp` cannot reach by `kabstract` and `congrArg`, until the two normal forms meet syntactically. Over knot-reaching callees it never falls back to a definitional check when they do not meet; it reports the first divergent node instead. It proves the run equations of every module that `runPipeline` (`hydro_lean/Hydro/HydroDef.lean`) routes away from the knot stack. For a **glue module**, a module with a callee that reaches a knot, transitively ([Routing](./overview.md#routing)), it also yields the derived decision record, read off the callees' `rr` equations. The last part of the page is about the phases that apply callee lemmas instead of rewriting with them, and the keyed matching (`hydro_rule_apply`, `conclKeyMismatch`) that rejects a rule that cannot match before any unification.

## Callers of the Engine

The engine serves five callers:

| Caller | Proves | Route |
|---|---|---|
| `genGlue` | `M_co_srᵢ`, `M_vdec`, `M_co_rrᵢ` | glue modules: some callee reaches a knot (`toy_loop` over its knot constant `toy_loop.w`, `paxos_core` over `leader_election` and its own knot `paxos_core.a_log`) |
| `genNaming` | `M_co_srᵢ`, and `M_co_rrᵢ` when the module has no content decision | the couple route: no callee reaches a knot (`toy_relay`, `toy_step`) |
| `genSrcNamingImpl` | `M_co_srcᵢ`, a singleton output's remembered machine source | the couple route |
| `genLagNamingImpl` | `M_co_lagᵢ`, an output's lag over the inputs' lags | the couple and glue routes (a knot's lag comes from the knot stack) |
| `co_residuals` (tactic) | the left side of `genRrEx`'s per-output goals | content modules ([The Couple Phase](./couple-phase.md#the-script)) |

An `initialize` block in `HydroGenKnot.lean` installs the engine into `HydroGen.lean`'s hooks (`namingEngineHook`, `srcNamingEngineHook`, `lagNamingEngineHook`, `glueMonoHook`), because the engine lives in the file that imports the knot machinery.

## The Corner Chain

The engine proves a run equation `lhs = rhs` by normalizing each side independently and checking that the normal forms coincide. The normalizer is `glueCornerChain`:

```lean
def glueCornerChain (lhs bodyC : Expr) (simpC : Array Name)
    (resLems : Array (Name × LemKey)) (cache : MatchCache) :
    TermElabM (Expr × Expr) := do
  …
  let bodyC ← zetaAll bodyC
  …
  let (tC0, prfC?) ← simpTerm bodyC simpC
  …
  let step ← mkExpectedTypeHint (← mkEqRefl bodyC) (← mkEq lhs bodyC)
  let step ← match prfC? with
    | some p => mkEqTrans step p
    | none => pure step
  let mut st ← RwSt.ofProof step lhs tC0
  …
  for _ in [0:8] do
    let rhs0 := st.rhs
    st ← st.rwResiduals resLems cache
    …
    let (t', prf?) ← simpTerm st.rhs simpC
    …
    match prf? with
    | some p => st := { st with rhs := t', prf := ← mkEqTrans st.prf p }
    | none =>
      -- a purely definitional pass (`rfl`-lemmas only; the op projections are all `rfl`)
      -- comes back without a proof, and the normal form still advances: the chain's proof
      -- is checked against the tracked `rhs` at the end, by defeq
      st := { st with rhs := t' }
    if st.rhs == rhs0 then break
  pure (st.prf, st.rhs)
```

`bodyC` is the module's value applied at the corner arguments. It is `lhs` after one delta step, hence the `Eq.refl` first step. The steps after it:

1. **`zetaAll`** substitutes every `let` and `have`. Glue bodies chain `have le := (callee …) …;`, and a projection applied to such a chain blocks `simp`'s own reduction.
2. **`simpTerm`** runs `simp` with a rule list: some of the callees' run equations, the per-operator projection set (the `co_simp_rules` attribute: every `co_<op>_rr`/`_sr`/`_src` lemma, all `rfl`) when the list asks for it, the `coTickScanProj` simproc, and the module's extras as unfold targets. The projection set is a prebuilt attribute, so a call adds only the extras.
3. **The residual pass** (`rwResiduals`) rewrites, one targeted rewrite at a time, what `simp` left behind, alternated with `simp` until the term stops changing (at most eight rounds).

The two main callers split the rules differently. `genNaming` puts the projection set and the callee run equations into `simp` and gives the residual pass the callee run equations only: the projection lemmas are `rfl`, so `simp` applies them even in positions it cannot otherwise rewrite, and every residual candidate under a lemma's key costs one unification, so the pass must not retry the whole operator set. `genGlue` restricts `simp` to the callee run equations and the boundary-constructor collapses (`co_probeC_sr`, `co_schedC_sr`, …: `rfl` lemmas for the corner values the knot machinery builds) and gives the residual pass the full per-operator set as well, because its redexes include combinator trees wrapping callee outputs.

The residual pass exists because `simp`'s congruence does not reach every argument position. An argument whose type a later argument's type mentions, as with a subtype-typed output such as that of a hoisted chain body (the module that holds the hoisted text of a `fix` block; see [The Knot Stack](./knot-stack.md)), has no congruence motive. `simp` treats the application as atomic there, although the same lemma instance is definitionally equal at reducible transparency. The redex is still rewritable at the level of the whole equation: abstract it out of the tracked right side with `kabstract`, then rebuild the equation with `congrArg` and `Eq.trans`.

```lean
def RwSt.rwStep (st : RwSt) (eqPrf : Expr) :
    TermElabM (Option RwSt) := do
  let eTy ← whnfR (← inferType eqPrf)
  let some (γ, c, d) := eTy.eq? | return none
  -- reducible: the pattern is syntactically present (found by the reducible matcher), and
  -- default-transparency near-miss comparisons against module-sized subterms whnf whole
  -- modules
  let rhsAbs ← withReducible <| kabstract st.rhs c
  unless rhsAbs.hasLooseBVars do return none
  let motive := Expr.lam `w γ rhsAbs .default
  let uγ ← getLevel γ
  let step := mkApp6 (.const ``congrArg [uγ, st.u]) γ st.α c d
    motive eqPrf
  let rhs' := rhsAbs.instantiate1 d
  let prf' := mkApp6 (.const ``Eq.trans [st.u]) st.α st.lhs st.rhs
    rhs' st.prf step
  pure (some { st with prf := prf', rhs := rhs' })
```

Candidates are found by one scan per round that buckets the closed subterms by a **pattern key** (projection flag, head constant, arity; `LemKey`, `collectKeyed`), so each lemma tries only the subterms under its own key. `matchLemma` matches a candidate against the lemma's left side at reducible transparency, so a near-miss never delta-unfolds a module body. `MatchCache` memoizes match results per `(lemma, candidate)` across rounds and outputs, because the same wire projections recur in every output. The state `RwSt` tracks the right side explicitly, so no step re-infers the type of the growing proof.

## Comparing the Normal Forms

Both sides run the same chain. The corner side carries corner-shaped applications that the callee run equations rewrite; the target side (`M sched …` or `M values …`) has none, but going through the same normalizers keeps the two normal forms confluent. They must then **meet**: be syntactically equal, or equal at reducible transparency, which tolerates residual differences in type arguments (matcher applications inside subtype predicates that `simp` cannot reach). In `genGlue` any other gap is reported, not explored:

```lean
def meetError (mName : Name) (what : String) (tC tR : Expr) :
    TermElabM Expr := do
  match exprDiff tC tR with
  | some (dA, dB) =>
    throwError "HydroGen: {mName}: {what} does not meet; first \
      divergence:\ncorner node ({dA.ctorName}): {dA}\nmodule node \
      ({dB.ctorName}): {dB}"
  | none =>
    throwError "HydroGen: {mName}: {what} does not meet (no \
      structural diff?!)"
```

`exprDiff` walks the two terms in parallel and returns the first divergent pair, reporting whole applications when the heads differ, since bare heads do not say which redex is stuck. The pretty-printer hides instances, universes and metadata, so two terms that print identically can still diverge, and the constructor names (`dA.ctorName`) disambiguate.

`genGlue` never falls back to `isDefEq` at default transparency, because such a check between two module-sized terms whose callees are folded unfolds the callees through their knots. When `genNaming` uses the engine for a plain module's run equations, a residual mismatch does fall through to a default-transparency `rfl`, because a knot-free module's callees are knot-free. With `HYDROGEN_SR_DIAG` set, that fallback becomes the same `meetError` report.

The proof that reaches the kernel is `Eq.trans stepC (Eq.symm stepR)`, built from one-delta `rfl`s, `simp` proofs and `congrArg`/`Eq.trans` steps. In `genGlue` the kernel is never asked a definitional equality that crosses a folded callee.

## Derived Decisions of Glue Modules

A couple module derives its decision record by solving an existential whose closing step is a definitional check of the two normalized sides ([The Couple Phase](./couple-phase.md#the-script)). Through a knot callee that check fails: the two sides agree only up to the knots' fuels, which equal `Td + 1` propositionally (by the fuel pins `M_vdec_fuelᵢ`), not definitionally ([Routing](./overview.md#routing)), and a failing definitional check against a folded knot callee unfolds it. The glue route therefore reads the record off instead of solving for it. A glue module's callees consume its content decisions (those whose `Values`-side data the corner derives): `toy_loop` passes its whole record to the knot `toy_loop.w`, and `paxos_core`'s chain body slices `dec.le` and `dec.sp` into `leader_election` and `sequence_payload`. After the corner chain has rewritten every callee by its `C_co_rrᵢ`, the `Values` side of each callee application already carries that callee's derived decision, and `extractGlueVdec` reads the module's record off them:

- **Whole-record passthrough**: if a callee consumes a record of the module's own record type at `Values`, that argument is the record.
- **Field-wise**: otherwise `extractGlueVdec` finds each field among the callee decision arguments by type, a `fixd` (fuel) field is `Td + 1`, and machine-only fields are `()`. A content field that no callee consumes is an error (`no callee consumes content decision field …`): a glue module cannot resolve a `nondet!` site with an operator of its own.

`genGlue` then abstracts the record over the module's machine data by `kabstract`, in the binder convention of `genRrEx` (`x{i}`, `x{i}_src`, `d{i}_<path>`), and adds it as the definition `M_vdec`. An occurrence of a corner binder that survives the abstraction is an error (`vdec extraction left a corner binder (…) unabstracted`), since the decision may depend only on machine data. For `toy_loop` the knot's `rr` equation supplies the record:

```lean
@toy_loop.w_co_rr₁ : ∀ {L : Type} {mem : L → ℕ} {pacing : Pacing L mem} {Tc Td : ℕ} {hjT : Tc.le Td} (ℓ : L)
  (inp : HydroSem.Stream mem ℓ ℕ StrOrd.noOrder Retries.exactlyOnce)
  (tdec : ToyLoopDec (CoupleSem L mem pacing Tc Td hjT) (mem ℓ))
  (sched : ToyStepSched (CoupleSem L mem pacing Tc Td hjT) (mem ℓ)),
  (toy_loop.w (CoupleSem L mem pacing Tc Td hjT) ℓ inp tdec sched).rr =
    toy_loop.w (Values L mem) ℓ inp.rr
      (toy_loop.body_vdec ℓ inp.sr sched.relay.interval sched.net
        (toy_loop.w (SchedSem L mem pacing) ℓ inp.sr { relay := { batch := (), gate := () }, fuel := () }
          { relay := { interval := sched.relay.interval }, net := sched.net }))
      { relay := { interval := () }, net := () }
```

The third argument of the right side is a `ToyLoopDec (Values L mem)`, the module's own record type, so `genGlue` takes it whole and abstracts it over `inp.sr`, `sched.relay.interval` and `sched.net`. The module's `rr` equation then reads:

```lean
@toy_loop_co_rr₁ : ∀ {L : Type} {mem : L → ℕ} {pacing : Pacing L mem} {Tc Td : ℕ} {hjT : Tc.le Td} (ℓ : L)
  (inp : HydroSem.Stream mem ℓ ℕ StrOrd.noOrder Retries.exactlyOnce)
  (tdec : ToyLoopDec (CoupleSem L mem pacing Tc Td hjT) (mem ℓ))
  (sched : ToyStepSched (CoupleSem L mem pacing Tc Td hjT) (mem ℓ)),
  (toy_loop (CoupleSem L mem pacing Tc Td hjT) ℓ inp tdec sched).rr =
    toy_loop (Values L mem) ℓ inp.rr (toy_loop_vdec ℓ inp.sr sched.relay.interval sched.net)
      { relay := { interval := () }, net := () }
```

To prove `M_co_rrᵢ`, `genGlue` adds `M_vdec` to the target side's simp set, so the target side unfolds the folded `M_vdec` and both sides meet at the discovered spelling. Discovery runs on every output before extraction, because a record slice may reach only some outputs: `paxos_core`'s `sp` slice appears only in the payload outputs.

A glue module whose body contains a `tick` block also takes the full projection set into `simp`. The tick former's structural projection is a simproc, and the rewrites inside a tick body sit under binders, so neither is reachable by the keyed residual pass.

### The `co_residuals` Tactic

`genRrEx`'s per-output goals have the shape `lhs = rhs[?holes]`, where the right side carries the existential's witness holes. `co_residuals [rules]` runs the same residual-and-simp loop on `lhs` only and replaces the goal by `lhs' = rhs`. A `rw` loop over the whole goal would make `kabstract` unify every candidate against the holes, and its trailing reducible `rfl` would explore the mismatched pair after every step.

## Keyed Rule Application

Other phases close goals by applying callee lemmas instead of rewriting with them. The wf phase closes a goal `(K …).2.2.1.wf` by K's `_co_wf` lemma for that output, and the tick-monotonicity walk closes `TMono (C …).2^n.1` (`TMono`: a machine tick trace never changes a realized tick) by C's `_smono` lemma (see [The wf, smono and mono Phases](./wf-smono-mono.md)). The obvious loop is `first | with_reducible apply r₁ | … | apply rₙ` over every callee rule, and its cost comes from the failed unifications against module outputs.

A module with many outputs returns a right-nested tuple, a **projection spine** of `Prod.snd` applications over one shared application, and a rule for one output tried against a goal about another fails only at the bottom of the spine, after the unifier has retried every level: on `leader_election`'s body, 7.08 s for a failing `with_reducible apply` fourteen levels deep, about 1.7× per level ([A failing `apply` on a projection spine](../corner/projections-and-tactics.md#a-failing-apply-on-a-projection-spine)). A walk over every rule in order pays this for each wrong rule before reaching the right one.

`hydro_rule_apply [rules]` (`hydro_lean/Hydro/HydroGen.lean`) tests whether a rule can match before unifying. It reads the goal's relation at reducible transparency, so `TAgreeOf h (.stream …) x y` is seen as `TAgree h x y`, and computes for each argument a **subject key**: the terminal head constant and the literal projection path above it.

```lean
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
```

The key crosses `Subtype.val` (the coercion on a callee with an `ensures` contract) and the member read `MemberMap.get m i` (a `Values` value is a per-member map), so `(K …).2.1 i` keys as `K` at path `[1, 0]`. The monadic variant `subjKeyOfM` also unfolds `@[reducible]` heads. A knot's hoisted body constant `<K>.body` is reducible so that the `fix` laws see through it, and `with_reducible apply` sees through it too, so it must key as the chain-module output it unfolds to. The test itself:

```lean
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
```

A rule is skipped only on a known mismatch: same relation head and arity, and two known keys that differ. A differing head, an unknown key (metavariable, free variable, lambda head) or anything else uncertain falls through to `apply`, so a matching rule is never skipped. The test is linear in the spine depth. The rule's conclusion is opened with `forallMetaTelescope`, not the reducing variant, which would unfold `TMono x` into its `List.IsPrefix` body and make the test never fire. With `forallMetaTelescope`, `leader_election_smono₃` takes 16 ms instead of 11.4 s. `HYDROGEN_RULE_DIAG` prints each rule's keys and its outcome.

### `conclKeyMismatch` in the Monotonicity Resolver

`resolveRel` proves the glue module's `Values` monotonicity (its denotation is monotone in its inputs; `genGlueMono`): it unifies callee mono lemmas against the goal and resolves their hypotheses recursively. It has the same exposure and the same pre-filter, with one more rule:

```lean
def conclKeyMismatch (callees : Array Name) (lemConcl goal : Expr) :
    Bool := Id.run do
  let (fl, fg) := (lemConcl.getAppFn, goal.getAppFn)
  let (.const rl _, .const rg _) := (fl, fg) | return false
  unless rl == rg do return false
  let (asl, asg) := (lemConcl.getAppArgs, goal.getAppArgs)
  unless asl.size == asg.size do return false
  for al in asl, ag in asg do
    if let (some (hl, pl), some (hg, pg)) :=
        (subjKeyOf al, subjKeyOf ag) then
      if callees.contains hl && callees.contains hg
          && (hl != hg || pl != pg) then
        return true
      -- a callee-leg lemma against an operator-headed subject (a derived wire
      -- `H.mapTick …`/`H.zipTick …` of the normalized goal): a match would need the callee
      -- unfolded through its knots
      if (callees.contains hl && (`Hydro.HydroSem).isPrefixOf hg)
          || (callees.contains hg && (`Hydro.HydroSem).isPrefixOf hl) then
        return true
  return false
```

The operator rule covers a lemma about a callee output tried against a subject built from an operator over that output, such as `mapTick (zipTick le.1 …)` in `paxos_core.body`'s seventh output. On such a near-miss `isDefEq` assigns every argument metavariable and then refutes the match by reducing the whole-program knot chain. Without the operator rule, one candidate, `leader_election_mono₂` against that output, costs 403M heartbeats to fail; with it, `paxos_core`'s definition elaborates in 17 s instead of 50 s. Operator-headed subjects are resolved by the operator's own `vmono_<op>` law instead.

## Guarantees and Limits

Every run equation over folded callees is proved by `simp`, targeted `congrArg` rewrites and one syntactic comparison, so the generator's cost per module tracks the module's own text, not the text of what it calls. A failed meet is an error that shows the first divergent node, not a timeout. Rule application costs one key comparison per wrong rule.

The engine relies on two invariants it does not check. Callees stay folded, so a genuine match always agrees on head and projection path. And every operator's projection lemmas are `rfl`, so a definitional `simp` pass that returns no proof still yields a correct normal form, and the kernel checks the tracked right side against the goal at the end.
