---
sidebar_position: 1
title: Overview
---

# Generator Overview

Every `hydro def` needs a stack of theorems about itself, and none of them can be a library theorem: a program is a Lean function generic over its interpretation, not a data structure a theorem could quantify over ([The Approach](../approach.md#per-program-proof-generation)). `hydro def` therefore generates the stack for each module, from the per-operator lemmas of `Hydro/CoupleProj.lean`, `Hydro/KnotTactics.lean` and `Hydro/WfTactics.lean`; the generator adds no proof principle of its own.

A **module** is one `hydro def`: a Lean function of **wires** (live collections at a location) and decisions, generic over an instance `H : HydroSem L mem`, its **interpretation**. A **callee** is a registered module that another module applies. The generator states facts about four interpretations: the denotational semantics `Values`, the operational semantics `SchedSem` (the step machine), the monotonicity instance `MonoRel`, and the corner `CoupleSem`, which runs a module at `SchedSem` and `Values` at once (the [Vocabulary](../index.md#vocabulary) defines the corner's terms).

The stack consists of the module's **run equations**, which say which run each output of its corner run equals (the `sr` equation says that the machine run `sr` equals the `SchedSem` run; the `rr` equation says that the denotational run `rr` equals the `Values` run at a decision record derived from the machine run), the **lags** of its outputs, its residual `wf`, its step monotonicity at `SchedSem`, its monotonicity at `Values`, and the theorem that its compiled graph agrees with the module at `SchedSem`.

Two constraints shape every phase. A proof must never unfold a callee, because a callee may contain knots and definitional unfolding through nested knots is exponential in their depth ([Elaboration and Kernel Cost](../cost-model.md#definitional-equality-through-knots)); a caller's proof therefore consumes its callees' generated lemmas by name. And every proof must be produced without a human, for every module shape the grammar allows, so each one is a fixed script or a fixed term construction whose only per-module content is names. This page covers the machinery all phases share for that: the registry of generated names, the spec walk, the routing, the module telescope, the proof entry points, the order of the phases and the diagnostics. [How Proofs Are Generated](../proof-generation.md) follows `toy_relay` and a knot through every phase with the statements and scripts they produce; the phases themselves are [the couple phase](./couple-phase.md), [the glue phase](./glue-phase.md), [the knot stack](./knot-stack.md), [`wf`, step monotonicity and monotonicity](./wf-smono-mono.md) and [the project phase](./project-phase.md).

## The Registry

A caller's proof cites its callees' lemmas, so it has to know their names, their number of outputs, which knots they reach and which of their binders their graphs keep. Recomputing that from the callee's body would mean opening the body. Instead, what a phase generates for a module is recorded in `GenInfo`, a persistent environment extension keyed by module name, which the later phases of every caller read:

```lean
structure GenInfo where
  srLemmas : Array Name := #[]
  rrLemmas : Array Name := #[]
  …
  srcLemmas : Array Name := #[]
  …
  lagLemmas : Array Name := #[]
  exLemma? : Option Name := none
  vdec? : Option Name := none
  /-- A knot's `K_strict₁`: strict in its Bekić parameters, causal in its externals. -/
  strictLemma? : Option Name := none
  …
  knotOutLag : Nat := 0
  …
  smonoLemmas : Array Name := #[]
  wfLemmas : Array Name := #[]
  monoLemmas : Array Name := #[]
  …
  pinLemmas : Array Name := #[]
  /-- Knot modules (the knot phase). Consumers keep them folded: their `wf` goes through
  `K_co_wf₁`, never through `co_wf_simp`. -/
  isKnot : Bool := false
  …
  knotParams : Nat := 0
  …
  callees : Array Name := #[]
  inlines : Array Name := #[]
  hasFix : Bool := false
  hasSpec : Bool := false
  …
  hasRequires : Bool := false
  …
  dagKeep : Array Bool := #[]
  …
  schedIdxs : Array Nat := #[]
  …
  dagRecKeep : Array Bool := #[]
```

A caller's scripts are assembled from these names. `calleeNamingRules M` concatenates every callee's `srLemmas`, `rrLemmas`, `srcLemmas` and `lagLemmas`; the `wf` phase reads the callees' `wfLemmas`; the knot stack reads its body module's `vdec?` and `pinLemmas` and its inner knots' `strictLemma?`; the project phase reads the callees' `dagKeep` and `schedIdxs` masks to apply `C.dag` and `C.recordOf` at the right arguments. Because the extension persists across imports, a module in one file is a callee in another with no re-generation. A caller's proof never unfolds a callee; it uses the callee through these names, folded.

## The Spec Walk

Routing a module and assembling its scripts needs its composition: which callees it applies, which inline wrappers it opens, and whether it ties a knot. `hydro def` records that once, before any phase runs (`registerSpec`). `specWalk` walks the elaborated value in the module grammar and collects the three:

```lean
      | .const c _ => do
        let env ← getEnv
        if c == ``HydroSem.fix || c == ``HydroSem.fixTick then
          modify fun s => { s with hasFix := true }
          for a in args do specWalk inlineSet interps a
        else if c.getPrefix == ``HydroSem then
          -- a core operator or carrier type: structure field applications
          for a in args do specWalk inlineSet interps a
        else if (getGenInfo env c).isSome then
          modify fun s => { s with callees := s.callees.insert c }
          …
        else if inlineSet.contains c then
          …
        else
          throwError "hydro def: unrecognized invocation head `{c}` in\
            {indentExpr e}\n(register it with `hydro_register`/\
            `hydro def`, or mark it `hydro_inline`)"
```

Relevance is type-based: a subterm is walked only if its type mentions `HydroSem` or an interpretation variable, so closures over plain data (`q!` bodies) and proofs are skipped wholesale. `hydro def` rejects anything else that touches the interpretation at the definition. Because of the walk, no phase scans a body transitively (the callee list is authoritative), and a module outside the grammar fails at its own definition, not later as a generator error in some downstream module.

## Routing

`runPipeline` (`Hydro/HydroDef.lean`) chooses the phase set from the recorded spec:

```lean
  if gi.hasFix then
    -- a knot: the structural route
    phase "knot-stack" <| runKnotStackT mName hints
  else if gi.callees.any (reachesKnotN env) then
    -- a glue module: simp-discovery namings over folded callees
    phase "glue" <| runGlueT mName hints
    phase "smono" <| runModSMonoT mName hints
    phase "wf" <| runModWfT mName hints
    phase "mono" <| runModMonoT mName
  else
    -- a knot-free content module: the choice-route namings
    phase "couple" <| runCoupleT mName hints
    phase "smono" <| runModSMonoT mName hints
    phase "wf" <| runModWfT mName hints
    phase "mono" <| runModMonoT mName
  -- the production graph and its denotation lemma
  phase "project" <| Project.genProject mName
```

| route | modules | `rr` equation | why this route |
|---|---|---|---|
| knot stack | the hoisted knot constants `<M>.<wire>` | `K_co_rr₁` by `co_knot_rr₂` | a knot is closed per knot, by rewriting its body with the body's own run equations, never by unfolding it |
| glue | modules with a callee that reaches a knot, transitively | discovered: both sides normalized until they meet syntactically, the decision record read off the result | the choice route closes each output's equation by a definitional check of two normal forms; through a knot the two sides agree only up to the knots' **fuels** (the iteration count of a knot's denotation), which come out of a `Classical.choose` and equal `Td + 1` (`Td` the coupling instance's derive horizon) propositionally, not definitionally |
| couple | every other module | the choice triple `M_co_rr_ex`, `M_vdec := Classical.choose …`, `M_co_rrᵢ` | knot-free, so the closing definitional check is module-sized |

A module whose source contains a `fix` does not take the knot route itself. After hoisting, its value applies the knot constants, so its spec walk records `fix := false` with the knots among its callees, and it takes the glue route; the knot constants are registered separately and take the knot route (for `toy_loop`: `callees [Hydro.toy_loop.w, Hydro.toy_loop.body], inlines [], fix := false`).

The monotonicity phase routes on the same predicate a second time: a module-scale `MonoRel` instantiation is definitional only below the first `fix`, so a module over knot-reaching callees composes its callees' `mono` lemmas instead (`genGlueMono`). Both are described on [wf, smono and mono](./wf-smono-mono.md).

The engines live in two files, and `Hydro/HydroGen.lean` cannot import `Hydro/HydroGenKnot.lean` (which imports it). The engine that proves the run equations of plain modules is the glue engine, so `HydroGen.lean` declares hooks (`namingEngineHook`, `srcNamingEngineHook`, `lagNamingEngineHook`, `glueMonoHook`) that `HydroGenKnot.lean` fills in its `initialize` block.

## The Module Telescope

Every phase opens the module's type at the coupling instance with fresh implicit locals for its parameters: the **pacing** (which members of each location are active at each step) and the couple and derive horizons `Tc ≤ Td`, the steps up to which the coupling is claimed and up to which the machine's history is read (`withModCtx`):

```lean
    withLocalDecl `pacing .implicit pacingTy fun pacing => do
    withLocalDecl `Tc .implicit (mkConst ``Nat) fun Tc => do
    withLocalDecl `Td .implicit (mkConst ``Nat) fun Td => do
    withLocalDecl `hjT .implicit (← mkAppM ``Nat.le #[Tc, Td]) fun hjT => do
    let corner := mkAppN (mkConst ``CoupleSem) #[L, memE, pacing, Tc, Td, hjT]
```

and classifies each argument after `H` from its type with `H` visible: a wire (its type has a carrier family head such as `HydroSem.Stream`), a primitive **decision** (a decision family head such as `HydroSem.BatchDec`), a **decision record** (a structure parameterized by the interpretation whose fields are decisions or records, flattened to a field tree), or plain data. The output type is peeled into outputs: `Subtype` to `.val`, products to components, structures of carriers to fields.

`DecFam` classifies each decision family twice: whether its `Values` side is content the coupling instance derives, and whether the machine reads it:

```lean
def DecFam.isContent : DecFam → Bool
  | .orderSel | .tickOrderSel | .snap | .batch | .ticks
  | .clock => true
  | _ => false

def DecFam.schedPass : DecFam → Bool
  | .transport | .interval | .clock => true
  | _ => false
```

A **content** decision has `Values`-side data the coupling instance derives from the machine run (a batch's cuts, where each tick's batch ends; a snapshot's cut; an order selection); the existential `M_co_rr_ex` quantifies over these decisions. A **pass-through** decision is machine data: the delivery cursors and timing scripts the machine reads. The two statement directions map the coupling-instance arguments accordingly (`ModCtx.srArgs`, `ModCtx.rrArgsDirect`):

| argument | to the machine (`srArgs`) | to the denotation (`rrArgsDirect`) |
|---|---|---|
| wire `w` | `w.sr` | `w.rr` |
| pass-through decision `d` | `d` | `()` |
| other decision | `()` | `()` |
| `FixDec` fuel | `()` | `Td + 1` |
| decision record | repacked field-wise by the rules above | repacked field-wise |
| plain data | itself | itself |

`toy_relay`'s `sr` and `rr` equations show the mapping in their right-hand sides: the machine side gets `{ batch := (), gate := () } { interval := sched.interval }` and the denotation side `{ interval := () }` with the content record replaced by the derived one. `genRrEx` pins a content field the body never projects (a record carried only for an enclosed construct's own decision) to a default inhabitant, so its witness hole does not stay unassigned; a field reaching a callee whole counts as used (`recordFlowsWhole`).

## The Proof Entry Points

The generator builds every statement as an `Expr` from the module telescope, and has to prove it with no human in the loop. Three kinds of proof serve that, and which one a declaration uses depends on how large the terms are and whether the equation is true by unfolding: a tactic script, assembled as a string from the module's names, for statements that need rewriting; a bare `Eq.refl` handed to the kernel, for equations that are definitionally true but too large for the elaborator; and a proof term assembled directly from lemma applications, where the right lemma chain is known in advance. The four entry points:

| entry point | what it does | used for |
|---|---|---|
| `proveByTac goal tac` | parses the string `tac` as a tactic in the current environment and runs it on a closed goal in an empty local context, with error recovery off | every `addThmByTac` statement: the couple, smono and `wf` phases, `K_co_sr₁` |
| `proveByTacOpen goal tac` | the same with the ambient locals visible | the knot facts' scripts (`K_co_rr₁`, `K_co_wf₁`, the lag side condition), whose installed facts are local hypotheses |
| `addThmByKernelRfl name stmt` | `Eq.refl lhs` at the closed `∀ …, lhs = rhs`, judged by the kernel alone; `false` on refusal | `M.dag_denote` of a module without callees, `M.dag_denote_graphs`, `M.schedOf_recordOf`, `hydro_run`'s `eq_impl` |
| direct terms | an `Expr` assembled from lemma applications and `congrArg`/`Eq.trans` chains, given to `addThm` | plain and glue run equations, the lag equations, `K_co_lag₁`, `K_strict₁`, `K_smono₁`, `K_mono₁`, the Kleene-chain lemmas `K.stages*`, `M_monoᵢ`, `M.dag_valid`, `M.dag_agreeL`, hop 2 of `M.dag_denote` |

All four end in `addThm`, which hands the declaration to the kernel and turns a refusal into ``HydroGen: the kernel refused `name`: …``.

### `proveByTac`

`proveByTac` (`Hydro/HydroGen.lean`), with its comments:

```lean
def proveByTac (goal : Expr) (tac : String) (label : String := "") : TermElabM Expr :=
  withDeclDiag (if label == "" then s!"script {tac.take 60}" else label) do
  let env ← getEnv
  let stx ← match Parser.runParserCategory env `tactic tac with
    | .ok s => pure s
    | .error e => throwError "HydroGen: tactic parse error: {e}\n{tac}"
  …
  -- the goal is closed, so an empty local context: the script's `intro` names cannot be
  -- shadowed by the generator's open telescopes
  -- recovery off: a failing alternative is an exception, never an admitted goal
  -- the generated simp sets are over-approximate (one list per phase serves every module
  -- shape), so the unused-simp-arg linter would fire on every proof; it has two entry
  -- points (the tactic-time option read and the command-level info-tree pass), so the
  -- option is disabled and the info tree skipped
  let val ← withLCtx {} {} <| Term.withoutErrToSorry <| withEnableInfoTree false <|
      withOptions (fun o => Lean.Elab.Tactic.linter.unusedSimpArgs.set o false) do
    let mvar ← mkFreshExprSyntheticOpaqueMVar goal
    let remaining ← Tactic.run mvar.mvarId! <| Tactic.withoutRecover do
      Tactic.evalTactic stx; Tactic.pruneSolvedGoals
    …
    unless remaining.isEmpty do
      throwError "HydroGen: unsolved goals ({remaining.length}) under\n{tac}\n{goalsToMessageData remaining}"
```

Error recovery is off because the scripts are `first | … | …` chains: with recovery on, a failing alternative can become an admitted `sorry` instead of an exception, and the next alternative never runs. `addThmByTac name stmt tac` is `proveByTac` followed by `addThm`, and is where `HYDROGEN_TAC_DIAG` prints the statement and the script. Why the scripts are strings at all is on [How Proofs Are Generated](../proof-generation.md#proof-by-script-string).

### `addThmByKernelRfl`

Where an equation is definitionally true and its sides are large, the generator gives the kernel the proof term directly and runs no elaborator-side check (`Hydro/HydroGen.lean`):

```lean
def addThmByKernelRfl (name : Name) (stmt : Expr) : TermElabM Bool := do
  let stmt ← instantiateMVars stmt
  let some prf ← forallTelescope stmt fun xs body => do
      let some (ty, lhs, _) := body.eq? | return none
      return some (← mkLambdaFVars xs (mkApp2 (mkConst ``Eq.refl [← getLevel ty]) ty lhs))
    | return false
  -- one kernel judgement; a `Kernel.isDefEqGuarded` pre-check would double the kernel's
  -- work on the production-graph lemmas
  try
    addThm name stmt prf
    return true
  catch e =>
    if (← IO.getEnv "HYDRO_PHASE_DIAG").isSome then
      logInfo m!"[kernel rfl refused] {name}: {e.toMessageData}"
    return false
```

A refusal leaves the environment unchanged and returns `false`. There is no fallback: no caller retries with a script or a `Meta`-side `rfl`. A refused `Eq.refl` means the two sides, as the generator built them, are not definitionally equal, so the cause is a generator bug or a program outside the grammar; a retry through `Meta.isDefEq`, up to 50 times slower on these terms, would only fail later. Every caller throws instead, with a message that says what to compare:

```text
project: the kernel refused `M.dag_denote` (set HYDRO_PHASE_DIAG=1 and compare `M.dag` against `M`)
project: the kernel refused `M.dag_denote_graphs` (set HYDRO_PHASE_DIAG=1 and compare `M.dag` against the opened chain of `M`)
project: the kernel refused `M.schedOf_recordOf`: …
hydro_run: the kernel refused `name.eq_impl`
```

For a refused `dag_denote_graphs`, `HYDRO_PHASE_DIAG` re-judges each output component separately and prints which one fails ([The Project Phase](./project-phase.md#proof-of-dag_denote)). Why these statements go to the kernel and the limits on what the kernel may be asked are on [How Proofs Are Generated](../proof-generation.md#kernel-judged-terms) and [Elaboration and Kernel Cost](../cost-model.md#kernel-judgements).

## The Order of the Phases

Within one `hydro def` the order is: the hoisted constants innermost first (each knot block's **chain module**, the `fix` body hoisted as a registered module, then the block's knots), then the module itself. Within a knot's stack: `sr`, `lag`, `rr`, `strict`, `smono`, `mono`, `stages`, the `invariant` hook, `wf`. Each step consumes earlier ones:

| artifact | needs |
|---|---|
| a caller's run equations | the callees' run equations (`srLemmas`, `rrLemmas`, `srcLemmas`, `lagLemmas`) |
| `K_co_rr₁` | `K_co_sr₁` (it bridges the machine diagonal inside the `Values` iteration) and the body module's `vdec` and fuel pins |
| `K_strict₁`, the `wf`'s strictness fact | the chain module's `dag_agreeL`, so the chain's project phase runs before its knots |
| `K_co_wf₁` | the body's run equations, `wf`, `mono` and lag equation, the inner knots' stacks, `K.stages` and, for an assumed invariant, `inv_stages` |
| a caller's `M_co_wfᵢ` | the callees' `wfLemmas` and the module's `ghost pre` theorems (facts the program proves before a conditional fold, [Elaborating `hydro def`](./elaborating-hydro-def.md#ghost-pre-and-requires)) |
| `M.dag`, `M.dag_denote` | the callees' `C.dag`, `C.dag_denote` and keep masks |
| `M.dag_valid` | the callees' `C.dag_valid` |

The dependencies are enforced by errors that name the missing step:

```text
HydroGen: M.body has no graph lag analysis (`M.body.dag_agreeL`): the chain module's project phase must run before its knots
HydroGen: inner knot K is not registered (generate it first)
project: callee C has no `C.dag` (phase order)
```

Every `run…T` begins by checking whether its first artifact exists and returns if so. This check makes the phases idempotent. The eager generation inside `fix` elaboration relies on idempotence: the knot stacks run while the enclosing definition elaborates, so the proofs written inside it (its `ghost` facts and its `prove` clause) can cite them, and the post-command pass skips them ([Elaborating `hydro def`](./elaborating-hydro-def.md#eager-generation-inside-the-fix)).

## Cost Accounting

Lean counts a command's heartbeats once, so every declaration a `hydro def` generates is charged to that one command. The engine files therefore carry no heartbeat budget of their own; a heavy module raises `maxHeartbeats` where it is defined, and the number it needs is the sum over its generated declarations, not the cost of any single one. Hoisted ghost facts are the exception: each elaborates under `withCurrHeartbeats` and gets a declaration's budget. Where time and memory go is on [Elaboration and Kernel Cost](../cost-model.md).

## Diagnostics

The messages a failing generated proof produces, and where to look for each:

| message | meaning | look with |
|---|---|---|
| `HydroGen: unsolved goals (n) under <script> <goals>` | a script's closers did not finish | `HYDROGEN_TAC_DIAG`, then the phase's own variable below |
| `HydroGen: M: <leg> does not meet; first divergence: corner node (…): … module node (…): …` | the glue engine's two normal forms differ; the printed pair is the first redex no rule rewrote | `HYDROGEN_SR_DIAG`, `HYDROGEN_GLUE_TICK` |
| ``HydroGen: the kernel refused `name`: …`` | `addThm` rejected a direct term or a script's proof | the term's construction in the phase |
| `project: the kernel refused …` | an `addThmByKernelRfl` statement was refused ([above](#addthmbykernelrfl)) | `HYDRO_PHASE_DIAG` |
| ``hydro_pre_close: no `ghost pre` fact closes the goal:`` | a conditional fold's or callee's content premise has no matching `ghost pre` | `HYDROGEN_PRE_DIAG` |
| `HydroGen: a lag-2 agreement cannot be read at lag 1`, `HydroGen: knot K: the body's output lag is …` (knot stack); `HydroGen: knot {i} of a knot block is not strict in its own wire …` (project phase) | a cycle without a network hop or `defer_tick` | the program; [Strictness](./knot-stack.md#strictness) |

Each phase reads its own environment variable at the point it applies. All print to standard error, except where noted:

| variable | prints |
|---|---|
| `HYDRO_PHASE_DIAG` | each phase's wall time per module (`[phase] M couple: 232ms`), the project phase's per-step times, which output component the kernel refused when `M.dag_denote_graphs` fails, and (as a log message) every `addThmByKernelRfl` refusal |
| `HYDROGEN_HB_DIAG` | heartbeats and time per generated declaration (`[hb] <label>: <k> <ms>`), per `hb_block` of a knot `wf` script, and the knot generator's own cost between proofs |
| `HYDROGEN_TAC_DIAG` | the statement and the script of every `addThmByTac` declaration, as `[tac] <name>`, `stmt: …`, `tac: …` (`1`, or a suffix of the declaration name to select one) |
| `HYDROGEN_SR_DIAG` | the goal of `M_co_rr_ex` with its witness holes, left unsolved (`genRrEx` drops the closer); the glue chain's per-step heartbeats; a meet failure as `meetError` instead of the default-transparency fallback |
| `HYDROGEN_GLUE_TICK` | the glue chain's per-step heartbeats and term sizes |
| `HYDROGEN_RULE_DIAG` | `hydro_rule_apply`'s goal keys, each rule's keys, and each rule's outcome: skipped by key, applied, or failed (with its time) |
| `HYDROGEN_WF_DIAG` | the `wf` phase's `ghost pre` closers (as a log message); the script without its default-transparency fallbacks, so the failing goal prints in reducible form |
| `HYDROGEN_PRE_DIAG` | each `hydro_pre_close` pass and every failing fact's error, which a failing `first` alternative would otherwise discard |
| `HYDROGEN_LAG_DIAG` | a knot chain's input and output lags, and the lag facts of the knot `wf`'s coupling fact |
| `HYDROGEN_MONO_DIAG` | every candidate the glue monotonicity resolver tries, with its unification outcome and cost |
| `HYDRO_GHOST_DIAG` | the command's heartbeats used after each hoisted ghost fact |
| `HYDRO_DEF_DBG` | every `fix` obligation's statement (as a log message), and the pipeline steps the `fix` elaborator runs |

Symptoms with their causes are collected on [Traps](../traps.md).
