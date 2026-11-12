---
sidebar_position: 10
title: Traps
---

# Traps

Each entry describes a failure the framework has hit: the symptom you see (the error text or the measurement), its cause, and the rule the framework follows because of it. In most of them Lean does something correct but unbounded (it unfolds, eta-expands or re-evaluates), and nothing reports it except time or memory. A slow proof is therefore diagnosed with `trace.profiler` or the per-declaration table and fixed at its cause, never given a larger budget. Where a trap is developed with its measurements on [Elaboration and Kernel Cost](./cost-model.md), the entry here gives the symptom and the rule, with a link.

The vocabulary used below:

- A **module** is one `hydro def` program; a **wire** is one of its `let`-bound collections; a **knot** is a `fix … complete` cycle ([knot invariants](../using/proving/knot-invariants.md)).
- The **corner** `CoupleSem` runs a module as two runs at once: the step machine's run (`sr`, the **machine run**) and the `Values` run (`rr`, the **denotational run**) ([the coupling corner](./corner/coupling.md)).
- A **ghost** fact is a proof-only `ghost have` inside a program ([Proofs over Values](../using/proving/proofs-over-values.md)).
- The **generator** is the code behind `hydro def` that emits each module's theorems ([generator overview](./generator/overview.md)); a **rule** is a callee's generated lemma that a generated proof applies.
- A **heartbeat** is Lean's unit of elaboration work, and the **kernel** is the checker every declaration finally passes ([cost model](./cost-model.md)).

## Elaboration

### Failing Unification on a Folded Callee

- Symptom: a generated walk spends 31–45 s per rule that does not match, for example a rule about `(leader_election ?args…).2.1 ?i` tried against the goal `(leader_election args…).2.2.1 i`; a rule tried against a subject headed by a `HydroSem` operator cost 403 M heartbeats to fail.
- Cause: every argument unifies, and refuting `.2.1 =?= .2.2.1` then delta-unfolds the callee's body through its knots.
- Rule: compare keys (head constant and projection path) before unifying: `conclKeyMismatch` in the monotonicity resolver, `hydro_rule_apply` in the other walks ([a failing unification costs more than a succeeding one](./cost-model.md#failing-unification)).

### Failing `apply` on a Deep Projection Spine

- Symptom: `with_reducible apply` of a rule about the fourteenth output of `leader_election`'s body, against a goal fourteen `Prod.snd` deep that it does not match, takes 7.08 s to fail; 2.44 s at depth 12, 0.83 s at depth 10.
- Rule: every generated walk applies rules through `hydro_rule_apply`, which reads the keys with `subjKeyOfM` (through `Prod.fst/snd`, `Subtype.val`, `.proj` and reducible heads) before `with_reducible apply` ([cost model](./cost-model.md#failing-unification)).
- Sub-trap: the key test must open the rule with the non-reducing `forallMetaTelescope`. `forallMetaTelescopeReducing` also unfolds the conclusion (`TMono x`, the tick-monotonicity relation, becomes its `List.IsPrefix` body), the keys then never match the goal's, and the test never fires. Nothing reports that.

### Default Transparency in the Generator

- Symptom: one `kabstract` in the outer knot's `wf` generator spends 419 M heartbeats (36 s), because every same-headed `.rr` subterm becomes an `isDefEq` that `whnf`s two modules. A callee `wf` lemma applied wherever its callee occurs in the subject costs 3.6 G heartbeats (318 s) for one output.
- Rule: generator rewrites abstract at reducible transparency (`withReducible <| kabstract`), and `hydro_wf_close` applies a callee's lemma only when that callee heads the subject and the lemma's projection path equals the goal's. The two measurements become 3.5 s and 0.8 s.

### Ghost Facts and Whole-Program `whnf`

- Symptom: a ghost `obtain` on `hmem : p2a ∈ Multiset.map f (List.sum (payloads_to_send r))` takes 2.7 s and 161 M heartbeats.
- Cause: `elabAppArgs` runs a default-transparency `whnf`, which zeta-deltas the wire and delta-unfolds every callee down to its tick formers (`tick_scan`, what a `tick` block compiles to).
- Rule: while a ghost fact elaborates, every registered module other than the current one is `[irreducible]`; a ghost reasons about callees through their contracts. The fact drops to 4.85 M heartbeats. Blocking zeta instead is slower, because the unifier then delta-unfolds the callees ([callees stay folded](./cost-model.md#folded-callees)).

### Argument-Dependent Result Types

- Symptom: rewrites under a callee's arguments silently do nothing, and `exact rfl` then closes the goal by a whole-program `whnf`.
- Cause: if a module's result is a subtype that mentions its arguments (the contract carried in the type), `simp`'s auto-congruence marks those argument positions fixed.
- Does not work: `@[congr]` never fires, because its type check runs at `implicit` transparency and cannot see through `SchedSem`; setting `backward.isDefEq.respectTransparency.types false` unfolds the whole interpretation (`Values` ×144k).
- Rule: a module's result type never depends on its arguments. The contract lives on `M._spec`, and `M` and `M.ensures` are derived from it ([the contract face](./generator/elaborating-hydro-def.md#the-contract-face)). `paxos_core`'s budget falls from 12.8 M to 3.2 M heartbeats.

### `rfl` and `show` Across a Boundary Operator

- Symptom: `fun j => rfl` across an operator that moves content into or out of a tick costs 4.5 s per check, and the ghost replay runs it three times, past the default 200 k heartbeat limit. A `show` of a zipped tick former reduces the shaped former (`TickShape.rec` ×167k).
- Cause: lazy delta reduction unfolds the side with the lower head first: `Multiset.map` → `Quot.liftOn` → `Quot.lift`, four levels deep.
- Rule: read wires one operator at a time with the `den` readers and `simp only` ([`den`](../using/proving/proofs-over-values.md#reading-wires)). The 13 s check takes milliseconds.

### Structure Projections and the Same-Head Heuristic

- Symptom: one `exact` in a knot `wf` costs 34 M heartbeats.
- Cause: with `MemberMap.get` as the structure projection, `get A i =?= get A' i` skips the unifier's same-head argument comparison, and a stuck projection makes `isDefEq` `whnf` the whole program upstream of the wire.
- Rule: `get` is a regular `@[reducible] def` over the `cells` field (`Hydro/MemberMap.lean`).

### Instance Projections at Reducible Transparency

- Symptom: `s i` on a `(Values L mem).Stream …` does not find the `MemberMap` coercion, and a lemma keyed on `MemberMap` never fires on it.
- Cause: instance lookup and the discrimination tree work at reducible transparency, where a class field of the `Values` instance stays in canonical form, so the carrier is keyed as `(Values L mem).Stream ℓ α ord ret`, not as its `MemberMap`.
- Does not work: marking the fields `@[reducible]` breaks the `tick` elaborator's wire classification for an abstract `H`.
- Rule: one `CoeFun` instance per member-indexed family, keyed on the projected spelling (`instCoeFunValuesStream` and its siblings in `Hydro/Values.lean`).

### Per-Command Heartbeat Counting

- Symptom: "timeout at `simp at htj`" points where the counter crossed the limit, not at what was expensive. A module with about 60 ghost facts crosses the default budget by growth alone.
- Rule: every top-level ghost fact is hoisted to its own declaration (`M.ghost.<h>`) under `withCurrHeartbeats`, and an override on a program is set to the measured sum, read off `scripts/heartbeat_table.sh` ([where the budget is counted](./cost-model.md#heartbeat-and-memory-budgets)). A trick that moves the number without moving the time is not a fix.

### Unparenthesized `try`

- Symptom: for a knot with no wire inputs, the rest of a generated script silently does not run.
- Cause: `try rw [a]; exact h` parses as `try (rw [a]; exact h)`.
- Rule: every `try` in a generated script is parenthesized (`(try rw [{n}]); (try simp only [{n}]); …`). The same holds for `by` inside `prove` fields: `⟨…, fun h => by cases h, …⟩` takes the following `, …` as tactic arguments ("multiple induction targets"); write `(fun h => by cases h)`.

### Clearing an Existential Witness with `-`

- Symptom: "unknown identifier `hp1a_at`", several lines after the real cause.
- Cause: `ghost obtain ⟨-, …⟩` with `-` on the witness clears every hypothesis whose type mentions it.
- Rule: name the witness (`⟨_ls, -, hp1a_at, …⟩`).

### Dead Obligations and Dead Free Variables

- Symptom 1: a `tick` invariant in a `hydro def` without `ensures` compiles with `by done` in the proof of one field; the only sign is Lean's "this tactic is never executed" warning.
- Cause 1: ghosts replay only in the field proofs, and the field proofs exist only under `ensures`.
- Symptom 2: an error-recovered tactic inside a packaged induction produces a term with dead free variables; `addThm` accepts it, and the asynchronous kernel rejects it later, behind cascading `whnf` timeouts.
- Rule: every invariant toy carries an `ensures`, and the `fix` invariant elaborator checks the term for residual free variables and fails at the clause ("fix invariant: residual free variables in the …").

### Smaller Elaboration Traps

- A `DecidableEq` on a nested product (`Ballot nP × Except (Option (Ballot nP)) (ALog P nP)`) exceeds the default instance-synthesis depth although every component instance exists. It is assembled by hand (`p1bPairDecEq`) and passed to the generator as an unfold hint: `hydro [p1bPairDecEq] def paxos_core`.
- `instDecidableEqUnit` is not a constant: `autoImplicit` turns it into an implicit binder of the constructor that mentions it, and every projection stops reducing. Write `instDecidableEqPUnit` (`Node.source_interval` in `Hydro/Graph/IR.lean`).
- `M.dag_denote` stated over a literal `HList.cons` environment puts `Kind.sched`-reduced carrier types into the discrimination-tree keys, and the simp lemma never matches. It is stated over an arbitrary `env`.
- Building the corner's simp set at every `co_simp` call (about 300 lemmas) makes every new `HydroSem` field a per-module cost. The sets are simp attributes (`co_simp_rules`, `co_lag_rules`, `co_wf_simp_rules`); the full build takes 2m51s instead of 4m50s.
- A thousand-line `do` block in one `TermElab` exceeds the compile budget of LCNF (the compiler's intermediate form). Split it into helpers.
- A 2-space indentation slip in a `do` block re-scopes code under a dead `if` with no error; an invariant can vanish everywhere, and only a `#check` guard on the generated name shows it.
- `lake env lean File.lean` reads stale `.olean`s of edited imports; `lake build` them first.

## The Kernel and Knots

### Definitional Equality Through Nested Knots

- Symptom: proving the `sr` equation of `leader_election`'s body by `rfl` takes 21 ms through two nested knots, about 4 minutes through three, and more than 80 minutes (51 M heartbeats) through five.
- Cause: each outer knot's body re-closes the inner knots, and kernel substitution does not preserve sharing.
- Rule: definitional equality never crosses a knot. `hydro def` hoists each knot body to its own top-level `@[reducible]` `def` (`M.w.body`), the proof of each knot's `sr`/`rr` equations is a rewrite loop over the body's lemma and the inner knots' lemmas, and inlining the composed body brings the cost back ([cost model](./cost-model.md#definitional-equality-through-knots), [knot stack](./generator/knot-stack.md)).

### Kernel Evaluation of Lags, Module Applications and Knot Wires

A wire's **lag** is how many steps its value trails its inputs: a network hop and a `defer_tick` add one, and a valid graph has one of them on every knot cycle ([graph IR](./execution/graph-ir-causality.md)).

- Symptom: kernel evaluation runs to gigabytes (up to 19 GB) on `Dag.valid`/`Dag.lagOf` of `leader_election`'s graph by `rfl`, on a knot's lag equation whose proof compares `.lag` of two differently headed module applications, and on hop 1 of `paxos_core.body`'s `dag_denote` (the kernel judgement of the graph against the module's `let`-chain), refused at 12 GB although its text is unchanged. The last is triggered by a knot-bearing callee's output feeding another callee with three or more unrelated nodes between the two calls.
- Cause: the kernel's evaluation is call-by-name (every read of a lag re-runs its producer); and a definitional check that fails an optimistic sub-comparison unfolds both sides to normal form, applying the tick knot's ungated diagonal `iterate body ⊥ (t + 1) i t` at symbolic steps, where `t + 1` is `Nat.succ t` and every application unfolds the body.
- Rule: the generator computes lags itself and proves validity one node at a time with the `*_lit` lemmas (`Hydro/HydroGenLag.lean`); knot lags go through the generic `co_hfix_stream_fixLag`/`co_hfix_tick_fixLag`; the tick knot's diagonal sits behind `stepGate`, which is semantically an identity operator and stuck for the kernel on a variable step ([cost model](./cost-model.md#kernel-evaluation-of-graphs-lags-and-knot-wires)).

### Unfolded Lag Arithmetic

- Symptom: `omega` case-splits every `max`: 105 M heartbeats on `p1b_fail`, no end on `i_am_leader`. Raw `max` trees grow multiplicatively through nested knots; one cannot be pretty-printed, and AC `simp` on them is super-linear.
- Cause: a knot's lag is its body's output lag at a probe (`fixLag`), and `Nat` defeq compares a horizon bound `j + lag ≤ T` that contains it by evaluating the whole knot body.
- Rule: `fixLag` is `@[irreducible]` (`Hydro/Couple.lean`). Lags are put in a sorted normal form (`lagCanon`) and proved leaf-wise (`co_lag_le`, `lagFactAt`). A module application is never `simp`'d to extract one projection.

### `Meta.isDefEq` on Big Terms

- Symptom: a `Meta` `rfl` on the `leader_election` wrapper's `dag_denote` takes 108 s; in the stager, `Meta.isDefEq` takes 46 s on a comparison the kernel judges in 78 ms.
- Rule: the generator states the equation and hands the kernel `Eq.refl` (`addThmByKernelRfl`), once, with no `Kernel.isDefEqGuarded` pre-check (it would double the kernel's work). A refusal is not retried another way: the caller reports an error ([judge by the kernel](./cost-model.md#kernel-judgements)).

### `#guard` as Kernel Evaluation

- Symptom: a `#guard` on a nested-knot whole-program run takes 36 minutes.
- Cause: `#guard` decides its proposition by kernel reduction, and the `Values` reads of keyed state (`Finset.max`, legal views) are cheap to execute and very slow to `whnf`.
- Rule: `#guard_eval` runs the compiled evaluator in milliseconds and fails the build on `false` (`Hydro/HydroDef.lean`).

### The Memory Cap

- Symptom: "(kernel) excessive memory consumption" on `paxosRun` under `-M 4000`, with unchanged heartbeats; the terms are about 7 % larger, because every `#[]` is `List.toArray []`.
- Rule: when debugging a blowup, run the file under `lake env lean -M 4500`. A kernel that evaluates a knot or a module application needs gigabytes, so the cap separates a representation cost from an evaluation the kernel should not be doing ([where the budget is counted](./cost-model.md#heartbeat-and-memory-budgets)).

## The Compiler and the Runtime

### One-Field Structures and Eta-Expansion

- Symptom: `paxos_core` at `Values` with 1 proposer and 1 acceptor runs in 107 ms and with 2 and 1 does not finish in 60 s; `leader_election` builds 852, 14k, 344k and 3.9 M member maps at one to four proposers; one output reads its input 4,775,952 times (48.6 s).
- Cause: the compiler eta-expands every declaration to its type's arity, and it erases a structure with exactly one relevant field to that field (`hasTrivialStructure?`), after which local lambdas returning it are eta-expanded and a `let`-bound map is merged into its read. A member map that is a function at runtime is call-by-name, and a `fix_stream` re-runs its Kleene iteration per read.
- Rule: `MemberMap` carries a second field, `boxed : Unit := ()`, and `MemoBox` a `tag` field, for this reason alone. The constructions are a constant 694, and `paxos_core` with 2 and 1 runs in 17 ms. A `@[noinline] get` fixes straight-line programs but not knots ([compiled code](./cost-model.md#sharing-in-compiled-code)).

### Eta-Expanded Local Functions and `match` Arms

- Symptom: in a wired run the sixth activation never finishes.
- Cause, three ways: `fun i => memoFin (s i)` becomes `fun i j => (build table) j`, rebuilding the inner table per read; a one-field `MemoBox` is erased; a `match` arm whose type reduces to a function type, `| .fold .., s => memo s`, becomes `fun s i => (memoBoxImpl s).fn i`.
- Rule: `memoBox2` builds the inner table as the stored value's content; `Node.memoStateBox`'s arms return a two-field `StateBox` projected outside the match; and because the `@[csimp]` replacement runs after `macro_inline`, `memo` is a `@[macro_inline]` projection of `memoBox` ([processes](./execution/processes.md)).

### What a Memo Table Captures

- Symptom: `bench 500 10 6000` reaches 30.5 GB maximum RSS, then 55 GB at 12 s, and is OOM-killed at 20 s on a 61 GB machine: about 0.5 MB retained per activation.
- Cause: the lookup captures the tabulated `f` for a `none` fallback that never fires, and `f` has the shape `fun i => s i ++ Δ.get x i`, so every cell retains every past state of everything it reads.
- Rule: `lookupHosted` is `@[noinline]` and captures only the optional value and its totality proof (`Hydro/Project/Wired.lean`); the run stays at 325 MB. The totality proof is needed: `Option.get` on `none` compiles to a field projection of a scalar, which spins instead of panicking.

### Finding a Sharing Leak

Exponential time at constant memory indicates lost sharing. Put a `dbgTrace` on the forced value inside `mkImpl`, run with the member count doubled, and count the traces with `sort | uniq -c`: a linear count means the value is shared, a `2^depth` count that it is rebuilt. The trace is itself subject to all of the above: wrapped around the map in a proof-relevant reducible definition, it reintroduces the laziness (`leader_election` from 74 s to more than 9 minutes; one elaboration ran 11 minutes and 9 GB). To locate a stall, read the compiler's IR (`.lake/build/ir/…`), not a stopwatch.

### Runtime Traps

- Lean's task manager has one condition variable for all blocked `Task.get`s, so every task completion wakes every blocked waiter: about two cores per process. No harness thread blocks on a socket; reads are `Async` continuation chains, and writes go to one writer thread per process (`Hydro/Project/Sockets.lean`).
- An array behind an `IO.Ref` that another thread created is marked multi-threaded, so every `push` copies it whole: O(n) per activation. Such state travels as loop arguments (`Examples/Demo/BenchDemo.lean`).
- Core `instDecidableEqProd` is a closed function taking the components' instances as dictionaries, a `lean_apply` with boxed arguments per hash-bucket probe (9.9 % of the profile). `instDecidableEqProdInline` (`Hydro/Order.lean`) is `@[inline]` at high priority. The same trick for `Hashable` does nothing: the keyed operators are generic in `K`, so the dictionary never reaches the compiler as a constant.
- `Array.filterMap` and `Array.foldl` carry `stop := as.size`, a second occurrence of the batch. The kernel compares the staged code to the interpreter term for term, and a dependent rewrite under them hits only the `stop` occurrence. Use `filterMapB` and `foldlB` (`Hydro/Project/Step.lean`).

## Staging

`hydro_run` evaluates the interpreter on a closed graph at elaboration time and emits straight-line code ([staging](./execution/staging.md)). It exists because a `@[csimp]` equation replaces one constant by another everywhere and cannot specialize `Dag.iterateBytes` at one graph. The rules below are in the header of `Hydro/HydroStage.lean`, and [the cost model](./cost-model.md#staging-cost) has their measurements.

- **Partial evaluation by `simp` does not scale.** On a four-node toy graph `simp` exceeds 4 GB, because a conditional on structure it cannot decide duplicates its continuation. The stager drives the sweep's control flow itself in `MetaM` and emits each data step as a `let`.
- **Projections are reduced by the stager's own procedure, never by δ.** `whnfCore`'s default projection mode runs a full `whnf` on `(Node.memoStateBox m n st).1` with `st` a chain of about a hundred `Prod.snd`s, 0.1–0.7 s a node. `reduceHead` reduces it, and the cells' and codecs' projections are primitive `Expr.proj`s.
- **Smart unfolding reduces its discriminants again.** An inbox nested in an enqueue nested in a memo is reduced three times. Each is bound to a `let` before anything unfolds over it.
- **The compiler's front end re-infers uncached types.** `toLCNF` infers and `whnf`s the type of every subterm, and a type over a free variable (`Dag.LocState m (D cfg)`) is never cached: 41 of 46 s of compilation. The type-level functions (`typeLevel`: `Node.LocState`, `Dag.LocState`, `Dag.Codecs`, … `Kind.locDelta`) are unfolded wherever they occur, and compilation takes 6 s.
- **What the stager never reduces.** `let`-bound variables (`whnfNZ` runs with `zetaDelta := false`: the lets make up the program, and unfolding one re-evaluates it symbolically, without end on a condition over symbolic inputs); `ite`/`dite`, which are decided through their instance (`decideIte`) because `Decidable.rec` does not compile; `memo`/`memo2`/`memoBox`, the sharing seams; and the messages' right-nested `List.append` chain, which the kernel bridge needs in the same shape because `List.append` is not definitionally associative on symbolic lists.
- **Measure synchronously.** Under the default asynchronous elaboration the kernel bridge prints as 1 ms while it takes 20 s; measure with `-DElab.async=false`.

## Models

### Uninhabited Premises

- Symptom: a machine-safety theorem that assumes its decision environment is satisfiable compiles with zero `sorry`s and a green gate, and the premise is false for every decision, every schedule and every horizon `T ≥ 1` once knots are nested.
- Cause: the bridge for the inner knot requires its two readers to agree at stage 0 (the first Kleene iterate), and they do not.
- Rule: derive the decision from the machine run instead of postulating it ([coupling](./corner/coupling.md)). Every quantified side condition on a trusted statement ships with a witness that inhabits it: the `example` beside `cq_live` ([liveness](./liveness.md)), the `#guard` scenarios beside `paxos_safe_wired`.

### Deadlocked Knots

- Symptom: the `a_log` cycle's Kleene chain (its iterates from the empty wire) stays at the empty run, every stream is `[]` at every fuel, and the safety headline is vacuously true.
- Rule: every headline needs an executable witness whose premises are inhabited by a run that does something.

### Undischarged Hypotheses

- Symptom: a wired theorem needs hypotheses that the environment is idle and the run settled at its start; the settledness hypothesis is false at `Dag.init`, because `defer_tick`'s cell starts unfired, and nothing in the framework discharges it.
- Cause: delivery, knot feedback and `defer_tick` move content at members that are not active.
- Rule: a member that is not active is inert. Each member's pace at a step is `idle`, `stutter` or `tick`, and only a tick runs tick bodies ([sched](./semantics/sched.md)); the wired headline carries no such hypotheses.

### One Pass per Activation Under DFIR's Wake Rule

- Symptom: under the schedule in which a process wakes only for bytes or a timer, a run stalls at step 5, and `bench` collapses from 54k to 0 requests per second, because stutters do not tick.
- Cause: a knot whose feedback takes effect only in a later pass leaves work for a pass that nothing triggers.
- Rule: an activation is one tick in which each knot wire is its body's output of the same tick, computed by one sweep. That output is a fixpoint because every cycle passes a hop or a `defer_tick` ([activation](./execution/activation.md)).

### Strictness on Every Edge

- Symptom: `leader_election`'s knot output lags are `[0, 0, 1]`, not all positive: `p_is_leader` reads `i_am_leader` in the same tick.
- Rule: cascade validity (`cascadeValid`): knot by knot, with the earlier knots closed, the check evaluates the body's lag function at the knot's own probe. On a module's graph the generator reports a violation as `HydroGen: knot {i} of a knot block is not strict in its own wire once the earlier knots are closed (…)`; the generator rejects a single knot without a hop earlier, while its `fix` elaborates, with `HydroGen: a lag-2 agreement cannot be read at lag 1` ([The Knot Stack](./generator/knot-stack.md#strictness)).

### Commutative Models and Order Dependence

- Symptom: with `f = 1` and log entries `X = (1, v)`, `Z = (2, w)`, `Y = (3, v)`, Paxos's recommit fold gives a count of 2 under `X·Y·Z` and skips the slot, but 1 under `X·Z·Y` and recommits it. A model that pools the entries into a multiset with a count is commutative and cannot show this.
- Rule: the Rust takes `assume_ordering::<TotalOrder>`, and the Lean takes `tassume_ordering` with the literal step, so every order is a theorem; an illegal order blocks the trace. A legality hypothesis would put a decision premise on the headline, and a fallback order would invent one the Rust does not have (`Examples/Paxos/Recommit.lean` has the witness).

### What the Model Must Admit

- A delivery premise (`fifo`) on traces is violated by the shell's own trace. Rule: a `WireTrace` has no proof fields, and every quadruple (`who`, `inp`, `readings`, `reads`) is a run.
- Decisions baked into the compiled graph put the quantifier on the wrong side ("for every run there exists a reproducing schedule"). Rule: `Dag.Sched` is a field of the `Interleaving`, and `schedOf_recordOf` covers every schedule.
- A step that appends a process's sends to its own queue, with the activation dropping them again, is a proof device on the executable path. Rule: the model describes `hydro_lang` and DFIR, and devices live in the proof layer.
- A timing model that admits a tick without a clock reading admits a run Rust cannot produce; the wired demo stops getting heartbeat verdicts after 64 ticks. Rule: timing is a total per-tick script (`TickScript`).
- An operator whose coupling is stated as `True` is not a soundness hole, but it can be wrong silently. Every operator's coupling is its real inclusion.
