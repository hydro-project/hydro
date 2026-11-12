---
sidebar_position: 3
title: Elaboration and Kernel Cost
---

# Elaboration and Kernel Cost

A `hydro def` at Paxos scale elaborates one module and then generates dozens of theorems about it, over terms that are large in two ways: module applications nest several knots, and the typed graph's terms carry their whole typing context at every node. Lean offers several ways to decide that two such terms are equal or to compute a value from one, and at this scale most of them are exponential or exhaust memory. The framework follows rules that avoid those paths; each section below gives one rule with the measurement that forced it. [Traps](./traps.md) lists the same failures by symptom, for diagnosing a new one. The [Architecture Overview](./index.md#vocabulary) defines the vocabulary (knot, corner, machine run, denotational run, graph, face, ghost, kernel judgement).

The page uses three Lean terms:

- a **heartbeat** is Lean's unit of elaboration work (`maxHeartbeats` counts thousands of them, default 200000);
- **`Meta.isDefEq`** is the elaborator's definitional-equality check, which `rfl`, `simp`, `apply` and `exact` run;
- the **kernel** is the independent checker every declaration finally passes; it also decides definitional equality, by its own algorithm.

## Heartbeat and Memory Budgets

Lean counts a command's heartbeats together, and one `hydro def` is one command: the def, its hoisted knots and every declaration the generator emits. The program files whose commands exceed the default carry the measured sum, with the reason written next to it (`Examples/Paxos/LeaderElection.lean`):

```lean
-- One `hydro def` command elaborates the def and every declaration it
-- generates, and Lean counts their heartbeats together. The override is
-- the measured sum for this command.
set_option maxHeartbeats 700000 in
```

`Examples/Paxos/PaxosCore.lean` carries 500000, and `Examples/Demo/BenchProfile.lean` 600000 for one line whose elaboration unifies types computed from the production graph. Each hoisted ghost fact runs under its own budget (the elaborator wraps it in `withCurrHeartbeats`, as for a top-level theorem), and so does each of `hydro_run`'s three phases. The engine files set no budget at all. A heartbeat count is a proxy for time, and time is the quantity to optimize: a new override, or a staging phase above 200 M raw heartbeats, is a blowup to diagnose, not a limit to raise ([doctrine](./doctrine.md) R8).

The per-declaration table measures the cost of each declaration:

```bash
#   usage: scripts/heartbeat_table.sh <file.lean> [N]
#   e.g.   scripts/heartbeat_table.sh Examples/Paxos/LeaderElection.lean 30
```

It elaborates the file with `HYDRO_PHASE_DIAG`, `HYDRO_GHOST_DIAG` and `HYDROGEN_HB_DIAG` set, and prints every generated theorem, hoisted ghost and knot-stack phase with its heartbeats and wall time, largest first, then the command's running total (`cmd-used=`) that the override must cover. The other diagnostic variables are listed on [Generator Overview](./generator/overview.md#diagnostics).

A kernel evaluation that should never happen needs gigabytes (up to 19 GB, below). When a file's build slows down, run it under a memory cap, `lake env lean -M 4500 <file>`: such an evaluation then fails at once with "(kernel) excessive memory consumption" instead of running for minutes.

## Definitional Equality Through Knots

The `sr` equation of a module, `(M (CoupleSem …)).sr = M (SchedSem …) …`, is true by unfolding, and the obvious proof is `rfl`. Measured on `leader_election`'s body, one `rfl` at default transparency:

| knots the term passes through | time |
|---|---|
| none | 9 ms |
| one | 7 ms |
| two nested | 21 ms |
| three nested (the real `leader_election`) | about 4 minutes; the `rr` equation more than 70 minutes |
| five nested (`leader_election`'s three inside `paxos_core`'s two) | more than 80 minutes, then out of budget at 51 M heartbeats |

Each outer knot's body re-closes the knots nested in it (a sequential closure of what is semantically one mutual fixpoint), and kernel substitution does not preserve sharing, so the term the check walks grows by a constant factor per nesting level. Module-scale and body-scale `rfl`s stay in milliseconds; only the knots cost.

**Rule.** No proof decides an equality by unfolding through a knot. `hydro def` hoists every knot body to its own top-level `def` (`M.w.body`), and the proof of a knot's `sr`/`rr` equations is a rewrite loop (`co_body_rw`) that folds the body with the body's own `sr`/`rr` equations and the inner knots' lemmas, all used folded. The cost becomes linear in the number of knots: Paxos's whole-program `sr` and `rr` equations, `paxos_co_sr` and `paxos_co_rr`, take about 3 seconds together. [How Proofs Are Generated](./proof-generation.md#rewrite-loops-instead-of-definitional-unfolding) shows the loop and why it uses `rw` rather than `simp`.

## Kernel Evaluation of Graphs, Lags and Knot Wires

The graph's facts `Dag.valid` and `Dag.lagOf` are structural recursions over a `Dag` literal, and `M.dag_valid : (M.dag …).valid = true` looks like a `decide`. Each of the following kernel evaluations ran to gigabytes (up to 19 GB):

- `Dag.valid`/`Dag.lagOf` of `leader_election`'s graph by `rfl`. The environment-passing recursion is call-by-name in the kernel, so every read of a wire's lag re-runs its producer, and a `brecOn` over a hundred-node graph rebuilds its `below` towers per projection.
- A knot's lag equation whose proof compared `.lag` of two differently headed module applications, which became a full `whnf` of the knot's body module.
- Hop 1 of `paxos_core.body`'s `dag_denote` (`dag_denote_graphs`, the graph against the module's `let`-chain with the callees opened), refused at 12 GB. The trigger was a knot-bearing callee's output feeding another callee with three or more unrelated nodes between the two calls. A kernel definitional check whose optimistic sub-comparison fails unfolds both sides to normal form before concluding, and those normal forms apply knot wires at symbolic steps.

**Rule for the graph.** The generator computes lags and validity itself, walking the graph literal as an `Expr`, and proves the result one node at a time with lemmas whose inputs are literals (`Hydro/Graph/Causal.lean`):

```lean
theorem Dag.valid_letN_lit {Γ : Ctx L mem} {κs outs : List (Kind L mem)} (n : Node Γ κs) (rest : Dag (κs ++ Γ) outs)
    (h1 : Node.valid n = true) (h2 : Dag.valid rest = true) : Dag.valid (.letN n rest) = true := by
  show (Node.valid n && Dag.valid rest) = true
  rw [h1, h2]; rfl
```

Each kernel judgement is one node's rule on literal `HList`s of numerals; a callee's validity is its own `N.dag_valid`; a knot block's cascade check (each knot, the earlier ones closed, is strict in its own wire) evaluates the body's lag function once per lag vector the check probes, with memoization (`Hydro/HydroGenLag.lean`). A knot's lag is opened by generic instance lemmas (`co_hfix_stream_fixLag`/`co_hfix_tick_fixLag`, `Hydro/CoupleProj.lean`) instead of being compared.

**Rule for the knot wire.** The third failure is caused by the semantics. A tick knot's machine wire at step `t` is `iterate body ⊥ (t + 1) i t`, and `t + 1` is `Nat.succ t`, so every application at a symbolic step unfolds one iteration and evaluates the knot's whole body. `SchedSem` puts the diagonal behind a gate the kernel cannot open (`Hydro/Sched.lean`):

```lean
def stepGate {α : Sort u} (f : Nat → α) : Nat → α
  | 0 => f 0
  | t + 1 => f (t + 1)

@[simp] theorem stepGate_eq {α : Sort u} (f : Nat → α) (t : Nat) : stepGate f t = f t := by
  cases t <;> rfl
```

`stepGate f t` is stuck on a variable `t` because its `Nat.casesOn` is, and proofs that need the value rewrite by `stepGate_eq`. With the gate in place the refused hop 1 takes seconds. The stream knot needs no gate, because its diagonal (`famFreeze`) already recurses on the step.

## Kernel Judgements

Where an equation is definitionally true and its sides are large, the generator does not run `rfl`. It builds the proof term `Eq.refl lhs` itself and hands it to the kernel with `addThm` (`addThmByKernelRfl`, `Hydro/HydroGen.lean`; [The Generator](./generator/overview.md#addthmbykernelrfl) shows the code and its callers). The kernel checks every proof anyway, so this makes the kernel's check the only one.

The elaborator's own check, `Meta.isDefEq`, re-traverses unshared subterms, and every `Node Γ κs` in a graph term carries its context, so on these terms it is up to 50 times slower than the kernel: a `Meta`-side `rfl` on the `leader_election` wrapper's `dag_denote` took 108 s. On the stager's probe toy, `Meta.isDefEq` took 46 s on a structural mask the kernel judged in 78 ms. `simp` as a partial evaluator fares worse: on a four-node toy it exceeded 4 GB, because each conditional on structure it cannot decide duplicates its continuation. A `Kernel.isDefEqGuarded` pre-check before `addThm` would double the kernel's work on the graph lemmas, so there is none.

The kernel judges `dag_denote` of leaf modules, hop 1 (`dag_denote_graphs`) of the others, `schedOf_recordOf`, and the staged activation's `name.eq_impl`. A refusal is an error naming the refused declaration, with no fallback proof.

## Failing Unification

The generator's walks discharge a goal by trying callee lemmas in order, so every lemma that does not match costs a failed unification, and a failed unification is slow.

**Projection spines.** A goal about a deep output of a module is `(body …).2.2.….1`, a spine of `Prod.snd`. Reproduced in isolation, `with_reducible apply` of the step-monotonicity rule for output 14 of `leader_election`'s body, against a goal fourteen `Prod.snd` deep that it does not match, took 7.08 s to fail; at depth 12, 2.44 s; at depth 10, 0.83 s: about 1.7× per level. The unifier learns the mismatch at the bottom of the shared spine, and on the way back up retries each level in its unfolded `.proj` form (`Prod.snd` is reducible); with metavariables on the rule side the failures are not cached. A walk that tried every rule in order paid this for every shallower rule before reaching the right one.

**Near misses on folded callees.** In the monotonicity resolver, a candidate lemma about the same callee at a different output, `(↑(leader_election ?args…)).2.1 ?i` against `(↑(leader_election args…)).2.2.1 i`, assigns every argument and then must refute `.2.1 =?= .2.2.1` of one folded application, which it can do only by unfolding `leader_election.body` through its knots: 31 to 45 s per candidate, six of them 251 s of one 258 s phase. The same mechanism against an operator-headed subject cost 403 M heartbeats on one failing candidate.

**Rule.** Match by key before unifying. `hydro_rule_apply [rules]` and the resolver's `conclKeyMismatch` read each argument's terminal key (head constant and literal projection path, through `Prod.fst/snd`, `Subtype.val` and `@[reducible]` heads) and skip a rule whose keys disagree with the goal's, in time linear in the spine depth; anything uncertain falls through to `apply`. `leader_election`'s third step-monotonicity lemma, the generated declaration that paid the most for failed candidates, went from 11.4 s to 16 ms. The same trap is why `MemberMap.get` is a `@[reducible] def` over the field rather than the structure's projection ([traps](./traps.md#structure-projections-and-the-same-head-heuristic)).

## Folded Callees

A consumer reasons about a callee through its face and its generated lemmas, never through its body ([doctrine](./doctrine.md) R3), and the elaborator enforces it where a default-transparency step would otherwise unfold the program. While a ghost fact elaborates, every registered module other than the current module's own knots and hoisted knot bodies is made `[irreducible]` (`Hydro/HydroDef.lean`):

```lean
            -- Registered modules are opaque to the fact's elaboration: a
            -- default-transparency `whnf` of a hypothesis about a wire
            -- (`elabAppArgs` on `hmem : p2a ∈ Multiset.map f (List.sum
            -- (payloads_to_send r))`) zeta-deltas the wire and then
            -- delta-unfolds every callee module it reaches, the whole program
            -- down to its tick formers, before getting stuck.
```

Inside the generator, rewriting and matching run at reducible transparency, and when the two sides of an `sr`/`rr` equation fail to meet there, the generator reports both rather than retrying at default transparency. The residual rewriter, which rewrites leftover callee applications by the callees' `sr`/`rr` equations, memoizes per lemma and candidate; for `leader_election` (a module whose callees contain knots) the memo brings the phase from 70 s to 3.9 s.

## Staging Cost

`hydro_run` (`Hydro/HydroStage.lean`) executes the activation symbolically in `MetaM` and emits one `let` per data step ([staging](./execution/staging.md)). On `paxos_bench`, a stager that uses Lean's reduction as is pays three costs, and each is a rule in the file's header:

- `whnfCore`'s default projection mode unfolds anything to reach a constructor. On `(Node.memoStateBox m n st).1` with `st` a 100-deep chain it ran a full `whnf`, 0.1 to 0.7 s a node, growing with the node's position. The stager reduces projections with its own `reduceHead`, never by unfolding a definition, and stores cells and codecs as primitive `Expr.proj`s; binding intermediates to `let`s before unfolding over them stops smart unfolding from reducing a nested inbox three times. With both, staging takes 7.7 s instead of 17 s.
- Compiling the result took 46 s, 41 s of it in `toLCNF`, the compiler front end, which infers and `whnf`s the type of every subterm. A type over a free variable, such as `Dag.LocState m (D cfg)` behind every projection of the state, is never cached. Unfolding the type-level functions wherever they occur brings compilation to 6 s.
- `ShareCommon`, Lean's hash-consing of a term, cuts the final term from 71k to 24k objects.

The kernel bridge `name.eq_impl` for `paxos_bench` takes about 20 s: the kernel evaluates the interpreter on the whole graph and zeta-reduces 668 nested `let`s. For `paxosRun` the three phases take 4.9 s, 3.3 s and 12.4 s. These figures are meaningful only under `-DElab.async=false`; asynchronously the kernel phase prints as 1 ms.

## Sharing in Compiled Code

The `Values` programs and the processes are executables, and Lean's compiler rewrites them in two ways that destroy sharing without changing meaning. It eta-expands every declaration to its type's arity, so a value of function type is never pre-forced; and it erases a structure with exactly one relevant field to that field, after which local lambdas returning it are eta-expanded and a `let`-bound map in head position is merged into its read. A member-indexed carrier that is a function at runtime is therefore call-by-name, and every read re-runs its producer. Measured on such carriers: one output of a `leader_election` smoke-test run read its input 4,775,952 times (48.6 s), and `leader_election`'s member-map constructions grew 852, 14k, 344k, 3.9M at one to four proposers, roughly a factor of 16 per added member (16 is the number of Kleene-chain evaluations in its knots), because the knot solution was re-run per read.

**Rule.** A carrier that must be shared is a structure with a second relevant field, and its compiled form is substituted only through a kernel-checked equation. `MemberMap` carries the second field for this reason alone (`Hydro/MemberMap.lean`):

```lean
structure MemberMap (n : Nat) (C : Type u) where
  /-- The cells, as a function. -/
  cells : Fin n → C
  /-- Representation pin (see above); always `()`. -/
  boxed : Unit := ()
```

With it the constructions are flat (694 at two, three and four proposers). The compiled constructor `mkImpl` forces the cells into a `Vector` once, and the compiler may substitute it for the denotational `mk'` only through a `@[csimp]` equation, part of the trust base ([soundness](./soundness.md#the-trust-base)):

```lean
@[csimp] theorem mk'_eq_mkImpl : @mk' = @mkImpl := by
  funext n C f
  exact (mkImpl_eq f).symm
```

The process memo tables follow the same pattern (`MemoBox` with a `tag` field, `@[noinline]` lookups; [processes](./execution/processes.md)). A `@[csimp]` equation replaces one constant by another at every occurrence and inspects no arguments, so it cannot specialize `Dag.iterateBytes` at one graph, and the staged activation is therefore a separate definition with a theorem, not a seam. [Traps](./traps.md#finding-a-sharing-leak) shows how to find a sharing leak and why a trace placed around the map itself reintroduces the laziness.
