---
sidebar_position: 3
title: Elaborating hydro def
---

# Elaborating `hydro def`

A `hydro def` is written the way a proof-carrying program is easiest to read: the Rust function's lines in order, with the contract, the `fix` cycles, the `tick` blocks and the ghost proof steps inline, and the proofs referring to the program's own `let` names. That text has the wrong shape for everything downstream. The generator's phases need a program with no proof terms inside it and no dependent result type, or `simp` cannot rewrite under a callee application. No proof may unfold through a cycle, because definitional equality through nested knots is exponential in their depth. And a ghost fact replayed as tactic text in every consumer would be re-elaborated once per consumer, all on one command's heartbeat budget.

`hydro def` (`hydro_lean/Hydro/HydroDef.lean`) therefore splits the definition, before any generator phase runs, into four kinds of object:

- the plain program `M`, a function with no proofs inside and no dependent result type;
- the contract theorem `M.ensures`;
- one top-level `def` per cycle wire (a **knot**) and one per cycle body, so a proof about a cycle cites these `def`s and never unfolds through the cycle;
- one theorem per proof fact the body states (a **ghost** fact), proved once and cited by name.

This page shows how each split is made and what forces its details. A module here is a Lean function over an interpretation `H : HydroSem L mem`, whose arguments are wires and decisions (the [Vocabulary](../index.md#vocabulary) defines the terms); the routing that follows the split is on [Generator Overview](./overview.md#routing).

How you write the clauses it elaborates is on the user pages: [Proofs over Values](../../using/proving/proofs-over-values.md) (ghosts, `requires`, `ghost pre`), [Ticks and Cycles](../../using/programs/ticks-and-cycles.mdx) (`tick`, `fix`, `complete`), [Tick Blocks and Their Invariants](../../using/proving/tick-invariants.md) and [Knots and Loop Invariants](../../using/proving/knot-invariants.md) (`invariant`).

## Steps of the Command

The command elaborates the declaration under a renamed identifier, then derives, registers and generates:

```lean
    let specIdStx : Syntax :=
      mkIdentFrom declId[0] (declId[0].getId ++ `_spec)
    …
    liftTermElabM <| HydroGhost.deriveModule specName mName
    …
    -- the emitted knots first (emission order is innermost-first), then the
    -- module itself
    …
    for k in knots do
      …
      liftTermElabM <| registerSpec k
      …
      runPipeline k hintNames
    -- replay the theorems proven during elaboration (possibly in a speculative
    -- branch) into the persistent environment
    let invs ← HydroFix.pendingInvs.get
    …
    liftTermElabM <| registerSpec mName
    …
    runPipeline mName hintNames
```

1. The declaration elaborates as `M._spec`, whose type pairs the result with the contract. The `fix` blocks inside it hoist their knots and chain bodies as top-level constants during this elaboration, and generate their stacks there ([Eager generation](#eager-generation-inside-the-fix)).
2. `deriveModule` derives `M` and `M.ensures` from `M._spec`.
3. Each hoisted constant is registered and its pipeline run. Every phase is idempotent, so the stacks generated during step 1 are skipped.
4. The command re-adds the theorems proved during elaboration if they are missing. A definition body may elaborate in a speculative branch whose environment additions Lean discards (asynchronous and incremental elaboration), so the elaborator also records every `inv`, `ghost` and `pre` theorem as a closed `(name, statement, value)` triple in `pendingInvs`, and the command replays the triples here.
5. `M` itself is registered (the spec walk) and its pipeline run ([Generator Overview](./overview.md#routing)).

## The Contract Face

A **face** is a module's contract: a proposition about its outputs at `Values` that callers consume without looking at the body. `τ ensures out => P` in the result position elaborates to a subtype (`elabFace`):

```lean
    let face ← `({ $outId : $τStx //
      ∀ hv : $hId = Values $lStx $memStx,
        match $hId:term, hv, $[$deps:term],*, $outId:term with
        | _, rfl, $[$deps:term],*, $outId:term => $pStx })
```

The module is generic in `H`, but the contract is about `Values` only. The `∀ hv : H = Values L mem` makes the obligation vacuous at every other interpretation, and the `match` on `hv` transports every binder whose type mentions `H` (`deps`), so `P` is stated over `Values`-typed inputs. `value prove f₁ := e₁, …` closes such a body: it elaborates to `⟨value, by intro hv; subst hv; ‹replayed ghosts›; exact {f₁ := e₁, …}⟩` (`elabProveTerm`). After `subst hv` the interpretation variable is gone, and every computational `let` and every replayed ghost fact is in scope at `Values`. The generated `toy_relay._spec` is shown on [Generated Artifacts](./generated-artifacts.md#the-module-and-its-face).

### Deriving `M` from `M._spec`

`deriveModule` strips the pair from `M._spec`. `M`'s value is `M._spec`'s with the trailing `Subtype.mk … val prf` replaced by `val` (`stripPair`), and `M.ensures` is `(M._spec …).property rfl` with the `match` on `rfl` reduced by one `whnfCore` step. Its docstring gives the reason:

```lean
- `M : ∀ xs, τ`, the plain program: the pair's value with the `Subtype.mk`
  stripped. It is a non-dependent function, so a callee's arguments are
  ordinary rewrite positions for simp (a dependent `{out // … args …}` result
  type would make them fixed, and callee namings would not fire under a callee
  application);
```

The proof of every generated `sr` and `rr` equation of a caller (an equation identifying one output of a corner run with a run at another interpretation, [The Couple Phase](./couple-phase.md)) rewrites a callee's arguments in place: in `(callee (CoupleSem …) w …).rr`, `simp` rewrites `w` itself first. If the callee's result type mentioned its arguments, `simp`'s congruence lemma for that argument would have no motive, and the callee's equations would not fire. Callers therefore cite `M` and `M.ensures`; `M._spec` exists only to produce them.

## Hoisting the Knots

`fix (w₁ : τ₁) … (wₙ : τₙ) via (f₁, …, fₙ) := body` closes a cycle of wires, Rust's `forward_ref`. Each wire becomes a **knot constant**: a top-level constant whose value is `H.fix` (or `H.fixTick` when the wire is an `Atomic`, a per-tick trace) over a body that is its own top-level `def`. Hoisting is forced by cost. Definitional equality through `k` nested knots is exponential in `k`, because each outer knot's body re-closes the inner knots and kernel substitution loses sharing: on `leader_election`'s body one default-transparency `rfl` takes 21 ms inside two knots and about 4 minutes inside three ([the measurements](../cost-model.md#definitional-equality-through-knots)). With every knot and knot body its own top-level `def`, no proof unfolds through a knot: a proof about a knot cites the knot's and the body's `def`s and their generated lemmas.

The hoist builds the body as an instance-generic function of a fresh interpretation `H''`, the captured locals as a tuple, and the loop wire:

```lean
          let genBody ←
            withLocalDeclD `H'' hTy fun h'' => do
            let capsProd'' := (capsProd.abstract #[hExpr]).instantiate1 h''
            withLocalDeclD `caps capsProd'' fun capsV => do
            let τw'' := (τw.abstract #[hExpr]).instantiate1 h''
            withLocalDeclD `l τw'' fun wV => do
              …
              mkLambdaFVars #[h'', capsV, wV] bInst
```

and adds it as `<knot>.body`, marked `@[reducible]` and recorded in the inline registry, so that the generator's spec walks open it. The knot passes the capture family `Γ` explicitly:

```lean
          -- `H.fix` / `H.fixTick` with Γ passed explicitly, because higher-order
          -- unification would pick the non-abstracting Γ
          let knotBody ←
            if isTick then
              mkAppOptM ``HydroSem.fixTick
                #[none, none, some hExpr, some gamma, none, none,
                  some f, some capsTuple, some bodyApp]
```

The body is instance-generic because every proof re-instantiates it: at `MonoRel` for monotonicity, at `SchedSem` for causality, at `CoupleSem` for the coupling. For `toy_loop` (`Hydro/HydroGenToy.lean`) the knot's value is `H.fix tdec.fuel (inp, tdec, sched) (toy_loop.w.body ℓ)`, and the body is `fun ℓ H'' caps l => (toy_loop.body H'' ℓ caps.1 caps.2.1 caps.2.2 l).1`, a projection of the **chain module** `toy_loop.body`, the `fix` body hoisted as a registered module ([below](#single-source-blocks)); `.1` is the wire-closing output.

The captures are found by free-variable analysis of the elaborated body: the `H`-dependent locals it mentions become the tuple components. The knot's binders are the enclosing definition's data before `H`, then `H`, then everything after `H` that the body, the fuels or the wire types mention, in context order.

### The Bekić Decomposition

A block with `n` wires is a mutual fixpoint. The elaborator emits it as `n` nested single-wire knots (the Bekić decomposition): component `i`'s body is closed over the earlier components' knots, re-closed at the loop variable and the later wires:

```lean
private partial def closeWire (kApps ws : Array Expr) (i n j : Nat)
    (loopV : Expr) : MetaM Expr := do
  let mut wireArgs : Array Expr := #[]
  for k in [j+1:n] do
    if k == i then wireArgs := wireArgs.push loopV
    else if k > i then wireArgs := wireArgs.push ws[k]!
    else wireArgs := wireArgs.push (← closeWire kApps ws i n k loopV)
  pure (mkAppN kApps[j]! wireArgs)
```

Knot `j` takes the later wires `w_{j+1} … w_{n-1}` as trailing parameters, its **Bekić parameters**. The elaborator records their count (`knotParams`) in the knot's `GenInfo`, because the knot's strictness lemma `K_strict₁` assumes those parameters at lag `1` and the external inputs at lag `0` (a wire's **lag** is how many steps short of the coupling's step bound its coupling holds; [The Knot Stack](./knot-stack.md#strictness)).

## Single-Source Blocks

The Rust writes a cycle as one chain with the wire closed in the middle (`complete_cycle.complete(…)`), and the code after the closing reads the chain's values at the closed knot. The single-source form mirrors it: the body carries a `complete (e₁, …)` marker and the continuation follows inline. The body is elaborated twice, and a stack of pass flags (`fixPassMode`) tells `complete` which pass it is in:

- **Pass 1.** `complete` returns the wire-closing tuple and the continuation is not elaborated. `extendChain` then walks the elaborated `let` spine and extends the closing tuple with every chain `let` the continuation mentions (found syntactically by `collectIdents` on the continuation), flattened to product leaves because each flat wire is one output for the generator. Two kinds of `let` are skipped and keep their inline values: subtype-valued lets (a module call's contract fetch point) and lets only the pre-`complete` ghosts mention. The extended chain is hoisted as the registered module `<M>.body`, the knots are hoisted over projections of it, and their stacks are generated.
- **Pass 2.** The body is re-elaborated with the wires bound to the closed knots. `complete` skips to its continuation, so the chain's `let`s stay in scope for the continuation under their source names. `swapChainLets` then replaces each tupled `let`'s value by the matching projection of `<M>.body` at the closed knots, and `dropDeadLets` removes a skipped `let` that nothing after it mentions (otherwise the graph would carry a dead second application of its callee).

The swap makes the module value compute through the registered chain constant. The generator and the graph walker see that constant as the boundary, while the source shows the chain once. `toy_loop` after both passes (`#print`):

```lean
fun {L} {mem} H ℓ inp tdec sched =>
  have w := toy_loop.w H ℓ inp tdec sched;
  have out := (toy_loop.body H ℓ inp tdec sched w).2;
  out
```

and the hoisted chain, which exports `out` as its second component for the continuation:

```lean
fun {L} {mem} H ℓ inp tdec sched w =>
  let out := toy_step H ℓ w inp tdec.relay sched;
  (out, out)
```

The module value contains no `fix` any more, so its spec walk records `fix := false` with `toy_loop.w` and `toy_loop.body` as callees, and the module itself takes the glue route ([Routing](./overview.md#routing)). During pass 1 the body's ghost clauses queue as usual (a `ghost pre` inside the body needs the earlier facts for its own theorem), and the queue is restored afterwards so that nothing leaks out of the pass. Nested single-source blocks name their chains `<M>.<wire>.body` so the two chains do not collide.

### Eager Generation Inside the `fix`

The hoisted constants' pipelines run inside the `fix` elaboration, not after the command:

```lean
        -- eager knot-stack generation: the hoisted chain's and knots' generated
        -- vocabulary (couple/glue namings, `K_co_*`, `K_mono₁`, `K.stages*`) is
        -- available to the ghost clauses and prove legs of the enclosing
        -- `hydro def`, so the knot induction can cite the any-body Kleene-chain
        -- lemmas from inside the program.
```

The order is fixed: the chain module first (its couple or glue phase, step monotonicity, `wf`, monotonicity and its graph), then each knot's stack. The chain's graph must exist before its knots, because a knot's strictness follows from the chain graph's lag analysis at the knot wires ([Generator Overview](./overview.md#the-order-of-the-phases)). `HYDRO_PHASE_DIAG=1` on `Hydro/HydroGenToy.lean` shows the effect: the whole stack runs inside the definition's elaboration, and the post-command passes over the same constants find their artifacts and return.

```text
[phase] toy_loop._spec def-elab: 1494ms
…
[phase] Hydro.toy_loop.body couple: 0ms
…
[phase] Hydro.toy_loop.w knot-stack: 0ms
```

## The `invariant` Clause

A `fix` may carry a loop invariant over the knot's **Kleene stages**, the iterates `stages 0 = ⊥`, `stages (m + 1) = body (stages m)` of the knot body at `Values`, whose value at the fuel equals the knot (`K.stages_fix`, by `rfl`). What you write and what each obligation receives is on [Knots](../../using/proving/knot-invariants.md#the-invariant-clause). The elaborator states the obligations so that a proof never respells a module application, composes them into theorems over the stages, and chooses between an exported and an assumed invariant.

**Timing.** `elabFixInvariantClause` elaborates the clause from a hook (`loopInvHook`) that the knot stack calls after it has generated `K.stages` and before `K_co_wf₁`: the stages must exist, and the `wf` may need the result.

**Obligations over opaque binders.** `mkObligation` states each obligation over binders named after the runs: each chain `let` the clause lists, at the current stage, at the one-step image and at the closed knot, plus the faces `C.ensures …` of those that are module applications. It replaces each run's value by its binder in decreasing size order, so a containing value is replaced before the subterms it contains, and the binders stay transparent `let`s, so the callees' monotonicity lemmas and faces apply through them by definitional unfolding. The user's proof is elaborated against that type.

**Composition.** `iterate_invariant` composes the two obligations over the stages, with `K.stages_mono` supplying the chain-order premises (`w ⊑ w'`, `body w ⊑ w'`, `w ⊑ body w`, and `htop : w' ⊑ body w'`, all relative to the fuel) at each stage, and `K.stages_fix` identifying the closed knot with the stage at the fuel. The results are `<M>.<w>.inv` (the invariant at the closed knot) and `<M>.<w>.inv_stages` (at every stage up to the fuel), and the elaborator queues a ghost `have h<w>_inv := <M>.<w>.inv …` for the field proofs of the `prove` clause, applied at the definition's binders by name.

**Assumable invariants.** When the predicate mentions the wire alone (no listed `let`, not `w'`), the body may assume it of its loop input, as a Verus loop invariant does. The test (`fixInvAssumable`) is syntactic and runs before the body elaborates, because it changes how the body elaborates: the predicate is conjoined into the chain module's `requires` clause, so the body's `ghost pre` proofs see it as `h<w>_inv`. In the generator, the induction is unbounded (`iterate_invariant_all`), because the knot's `wf` probes the body at arbitrary stages, so the fuel-relative premises are not offered. The knot records `<M>.<w>.inv_stages` (`knotLoopInv`), and its `wf` feeds it to the body's `requires` at the stage it probes; and `<M>.<w>.inv` is registered as a `ghost pre` fact of the enclosing module, so the enclosing `_co_wf` closes the chain's invariant conjunct by `hydro_pre_close`.

**Dead free variables.** A misnamed hypothesis inside an obligation error-recovers into a term with dead free variables, which only the asynchronous kernel would reject, far from the cause. The elaborator collects the free variables of the packaged statement and value before adding them and fails at the clause:

```text
fix invariant: residual free variables in the packaged induction {nm} (an obligation proof likely references an unknown name): val …, stmt …
```

## The Ghost Layer

`ghost have h : S := prf` states a fact at a program point. The fact belongs to the proof component of `M._spec` only: it is stripped from the computational value and replayed after `intro hv; subst hv`, so it is stated at `Values` with every earlier `let` in scope. `ghost obtain`, `ghost let`, `ghost witness`, `ghost intro` and `ghost subst` queue tactics the same way.

### Hoisting Each Fact to a Theorem

Replaying a fact as tactic text in every consumer would re-elaborate the whole preceding ghost chain once per consumer, and every consumer shares one command's heartbeat budget. `elabGhostHave` therefore proves each fact once, as the theorem `<M>.ghost.<h>`:

1. it replays the pending ghost tactics on a scratch goal to obtain the ghost context;
2. it elaborates the statement and proof there, under `withCurrHeartbeats`, so each fact gets a declaration's budget;
3. it closes over only the context the fact needs, through binder types and let-values (module binders, let-wires, destructured witnesses, earlier ghost facts), and adds the theorem, keeping the module's `let`s as `let`s and the earlier facts it uses as premises; a fact that depends on an unnamed hypothesis cannot be closed over and is an error (`… depends on the inaccessible hypothesis …; name it`);
4. it queues `hydro_ghost_have h := <M>.ghost.<h> n` in place of the tactic, with `n` the context arity.

A hoisted statement, with the earlier facts it uses as premises, is shown on [Proofs over Values](../../using/proving/proofs-over-values.md#ghost-facts).

### Replay by Binder Name

`hydro_ghost_have` applies the theorem to the consumer's context by name. A `∀` binder is filled by the local of the same name (an instance binder by synthesis); a `let` binder is instantiated by the current let-local of that name, not by its value:

```lean
      | .letE nm _ _ b _ =>
        match lctx.findFromUserName? nm with
        | some decl => ty := b.instantiate1 decl.toExpr
        | none => throwError "hydro_ghost_have: the wire `{nm}` (a let of {thmName}) is not in scope"
```

The replayed fact therefore mentions the wire (`bumped i`), never its unfolded definition (`tick_scan …`), and `rw [h i]` finds the wire in the goal. The value is the plain application; the stated type and the application's type agree by zeta. Because a `fix` body elaborates twice and a wire's binder status may differ between the passes, the arity is read off the theorem (`ghostArity`), not recomputed. After a goal-shaped clause (`ghost witness`, `ghost intro`, `ghost subst`) the fact's context is the refined `ensures` goal's, which a closed theorem cannot capture, so such facts stay replayed tactics.

### Irreducible Callees

A ghost reasons about a callee through its face `C.ensures`, never through its body. The elaborator enforces this by making every callee irreducible while the fact elaborates:

```lean
            -- Registered modules are opaque to the fact's elaboration: a
            -- default-transparency `whnf` of a hypothesis about a wire
            -- (`elabAppArgs` on `hmem : p2a ∈ Multiset.map f (List.sum
            -- (payloads_to_send r))`) zeta-deltas the wire and then
            -- delta-unfolds every callee module it reaches, the whole program
            -- down to its tick formers, before getting stuck.
            …
            for m in mods do setReducibilityStatus m .irreducible
            let (S, prf) ← try HydroGen.withDeclDiag s!"ghost {thmName}" (withCurrHeartbeats elabFact)
              finally for m in mods do setReducibilityStatus m .semireducible
```

`mods` is every registered module whose name the current module's does not prefix, so the module's own knots and chain bodies stay open and a ghost may read its own structure. The measured cost of the failure this prevents is on [Proofs over Values](../../using/proving/proofs-over-values.md#callee-contracts). The generator is unaffected, because it unfolds modules by their equation lemmas (`simp only [M]`), not by `whnf`.

### `ghost pre` and `requires`

A fold over an unordered stream resolves the stream's deferred nondeterminism only if its closure gives one result on every history the stream captures; the fold's commutativity argument certifies that ([Deferred Nondeterminism](../approach.md#deferred-nondeterminism), [Grades](../semantics/grades.md)). A **conditional fold**, whose closure commutes only under a premise about the content, leaves that premise as a pairwise fact in the module's residual `wf` (the proposition under which the corner couples `sr` and `rr`, [The Coupling Corner](../corner/coupling.md)), and a callee with a `requires R` clause leaves `R` as a hypothesis of its `_co_wf` lemmas. With `ghost pre h : S := prf` the program proves such a fact at `Values`, before the site; [Proofs over Values](../../using/proving/proofs-over-values.md#requires-and-ghost-pre) explains why the premise belongs to the coupling and not to the face.

`elabGhostPre` makes it the theorem `<M>.pre.<h>`, stated as `∀ binders, let wires…; PreFact R S`, with `R` the module's own `requires` conjoined with the assumed invariants of the enclosing `fix` blocks (`PreFact R S` is `R → S`):

```lean
@toy_pre.pre.hpre : ∀ {L : Type} {mem : L → ℕ} (ℓ : L)
  (inp : HydroSem.Stream mem ℓ (ℕ × ℕ × ℕ) StrOrd.noOrder Retries.exactlyOnce)
  (dec : HydroSem.BatchDec mem (mem ℓ) (ℕ × ℕ × ℕ) StrOrd.noOrder Retries.exactlyOnce),
  let batched := HydroSem.batch inp dec;
  let keyed := HydroSem.tinto_keyed batched;
  PreFact (∀ (i : Fin (mem ℓ)), ∀ a ∈ MemberMap.get inp i, ∀ b ∈ MemberMap.get inp i, a.1 = b.1 → ToyTie a.2 b.2)
    (∀ (i : Fin (mem ℓ)), KeyedPool.Pairwise ToyTie (KeyedPool.accum (MemberMap.get keyed i)))
```

The theorem closes over only the context `R → S` mentions, so every binder occurs in its statement and the `wf` phase's closer `hydro_pre_close` determines them all by unification against a goal ([`hydro_pre_close`](./wf-smono-mono.md#content-facts-and-hydro_pre_close)). Its name is recorded for that phase (`currentPres`). Inside the proof the clause is available under its parts' names, `hreq` for the module's own clause and `h<w>_inv` for each enclosing assumable invariant, destructured from the left-nested conjunction. The `requires` clause itself is elaborated at once in the local context with `H` replaced by `Values` (`withValuesLCtx`), so an unknown name in it is reported at the definition.

## The Decision Census

`#nondet_census M (nondets := a) (scheds := b) (fuels := c)`, the line after each module, is a separate command in the same file. It walks `M`'s signature through its decision records and checks three counts: content decisions, schedule sites (every network edge and every interval timer) and cycle fuels. The expected counts are read off the Rust function but are not its `nondet!` tally; the definition and the per-module counts are on [Nondeterminism](../../using/programs/nondeterminism.md#the-census).
