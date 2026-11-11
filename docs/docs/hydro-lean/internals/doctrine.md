---
sidebar_position: 11
title: "Design Rules"
---

# Design Rules

Every change to the framework is reviewed against the rules below. A change that breaks one is wrong even if it compiles and the [gate](https://github.com/hydro-project/hydro/blob/main/hydro_lean/internal/GATE.md) passes. Each rule comes with a one-line reason and a link to the page that develops it. The mechanical lessons (what Lean does to a term, with the measurements) are on [Traps](./traps.md) and [Elaboration and Kernel Cost](./cost-model.md).

The rules use the framework's vocabulary:

- A **module** is one `hydro def`, the Lean counterpart of one Rust function, and a **wire** is a `let`-bound or input collection inside it.
- A **face** is a field of a module's contract structure (`…Ensures`), stated over its outputs and inputs ([Contracts](../using/programs/contracts.md)).
- A **ghost** is a proof-only fact placed at a program point ([Proofs over Values](../using/proving/proofs-over-values.md)).
- The **denotation** `Values` is the denotational semantics: a stream captures all the histories it could have had, and the nondeterminism stays unresolved in the stream's value until an operator or an explicit **decision** observes it ([The Approach](./approach.md#quotient-denotations)).
- `SchedSem` is the operational semantics, a step machine with pacing, queues and delivery cursors, and running a program at the **coupling corner** `CoupleSem` proves that its `SchedSem` runs are bounded by the denotation ([The Coupling Corner](./corner/coupling.md)).
- A **knot** is a `fix … complete` cycle (Rust's `forward_ref`), and its **stages** are the iterates of its body from the empty wire.

## Programs

**P1. The program text mirrors the Rust line for line.** One Rust function is one `hydro def`, with the same operators in the same order and the same binder names, and the Rust line quoted above each Lean line. A `sliced!` body is a `tick` block, `use::state` registers are `(state …)` binders, and `forward_ref`/`complete_cycle` is `fix … complete`. A line the Rust lacks says so in its comment. A Rust closure handed to a stage is a `hydro def` of its own, named as the `fn` Rust would write.

Reason: the Rust-to-Lean correspondence is part of the trust base, and the reader checks it by reading the two texts side by side ([Soundness](./soundness.md#the-trust-base)). That check works only if the texts align. A chain that is semantically equal to the Rust but shaped differently asks the reader to trust an unproved equation, and fusing two stages can hide two `nondet!` sites behind one decision. Such a chain is a fidelity item to fix ([Fidelity](https://github.com/hydro-project/hydro/blob/main/hydro_lean/internal/FIDELITY.md)).

**P2. Nondeterminism is a program input, split by who chooses.** Content choices (batch cuts, snapshot cuts, order selections, clock readings: the `nondet!` sites) go in the module's `…Dec` record. Schedule choices (delivery cursors, timer fire scripts) go in its `…Sched` bundle, which is `Unit` at `Values`. Proofs reason about the `…Dec` record and never about the `…Sched` bundle. `#nondet_census` counts both kinds of site and the cycles, and the build fails when the counts differ from the declared ones ([Nondeterminism](../using/programs/nondeterminism.md)).

Reason: at `Values` the network is an identity operator, so delivery has nothing to denote there, and "for every decision, safe" means "for every content choice the program observes" ([The Approach](./approach.md#decisions-as-arguments)). The split also shows which modules are deterministic given delivery: `acceptor_p1` has `#nondet_census acceptor_p1 (nondets := 0) (scheds := 1) (fuels := 0)`.

**P3. Decision spaces are total.** An illegal cut truncates the run, and an illegal per-tick order blocks it, so a theorem over all decisions carries no legality premise.

Reason: a legality premise on a headline is a premise nobody constructs, and such a premise can be false ([Traps](./traps.md#uninhabited-premises)); totality is also what lets the corner derive the realized decision from a machine run. A fallback order would invent an order the Rust does not have ([The Approach](./approach.md#decisions-as-arguments)).

**P4. The semantics mirrors Hydro, not an abstraction of it.** Every `HydroSem` operator has a Rust counterpart, and a Rust grade (ordering and retry guarantee) is a Lean grade. One tick's content is a plain array at the machine, because the runtime never erases order. Where the Rust is weaker than the protocol needs, the Lean takes the grade the proof needs and records the divergence. Where the Rust is wrong, the Lean carries both variants (`PaxosVariant.faithful`/`.guarded`), proves the fix and runs the bug ([Fidelity](https://github.com/hydro-project/hydro/blob/main/hydro_lean/internal/FIDELITY.md)).

Reason: a grade or operator the Rust does not have makes theorems about a program nobody runs.

## Proofs

**R1. Proofs are directly over the denotation.** There is no state-machine abstraction and no pure mirror of a program body: a proof reads the program's own wires with the `den` rewrite set ([Proofs over Values](../using/proving/proofs-over-values.md)), and a liveness argument is a `Values` argument that transfers ([Liveness](./liveness.md)).

Reason: at `Values` the network is an identity operator, so a protocol proof never reasons about messages in flight or interleavings of deliveries; delay exists only in the step machine, and the corner bounds it once for every program. A state-machine proof would reintroduce what the denotation removes, and a mirror duplicates the program and drifts from it ([The Approach](./approach.md#the-network-in-the-denotation)).

**R2. A proof decomposes along the program.** A fact lives at the program point it is about:

- `ghost have` goes after the line that establishes it;
- `ghost pre` goes before the operator whose prerequisite it discharges;
- `invariant` goes on the loop it governs;
- `prove field :=` proves the face's fields;
- `requires` states a premise of the module. The clause becomes the hypothesis `hreq` of the module's proofs, never a conditional inside the face.

There is no separate lemma file per module. Pure mathematics about lists, multisets and keyed folds goes to `Trace.lean`, `Grades.lean` and `Keyed.lean`.

Reason: a fact at its program point is stated in that point's vocabulary (the wires in scope), and a per-module lemma file is where mirrors of the program accumulate.

**R3. Faces are over outputs and inputs, and consumers consume faces.** A consumer cites `callee.ensures …` and the generated `callee_monoᵢ`. It never unfolds a callee and never re-derives a callee fact. "If I output this, a larger input gives more" is `callee_mono ∘ callee.ensures`, composed by the consumer.

Reason: a face stated in the callee's internal vocabulary forces every caller to restate its own wires in that vocabulary, and unfolding a callee inside a consumer's proof is a whole-program `whnf` ([Traps](./traps.md#default-transparency-in-the-generator)).

**R4. Ghosts refer to wires by their program names.** An unfolded wire denotation in a proof, or a `ghost let` alias of a wire, means a fact reached the proof in the wrong vocabulary. Fix the face or the binder.

Reason: the same as R3, one level down.

**R5. Loop invariants belong to the construct.** A `tick` block states its cross-tick fact as its `invariant` clause: a predicate on the emissions so far, the registers and the spectators (inputs it reads but does not consume), with no history index and the two obligations `prove init` and `tick` ([Tick Invariants](../using/proving/tick-invariants.md)). A knot states its fact about each stage as its `invariant` with `base` and `step` ([Knot Invariants](../using/proving/knot-invariants.md)). Nothing outside the construct mentions stages or scan states.

Reason: an invariant over history indices restates the construct's own induction by hand and is far more expensive to elaborate; the construct does no operator analysis, so the same rules port to Verus ([below](#the-verus-translation)).

**R6. Facts true of every body belong to the engine.** The run face, the tick readers, the register abstraction, the Kleene chain, the per-operator coupling and tick-trace monotonicity hold for every body of their construct. They are emitted by the elaborator or proved once in `Trace.lean`/`SchedMono.lean`, never per module. An engine extension is agreed before it is built, and it is built on the toys (`Hydro/HydroGenToy.lean`) first.

Reason: a per-module proof of a per-construct fact is duplicated work whose cost scales with the program ([How Proofs Are Generated](./proof-generation.md)).

**R7. Every headline has an executable or structural witness.** Each module has `#guard_eval` smoke tests at `Values`. The executables run the `Values` programs and, for `wired` and `bench`, the activation over the compiled graph. The falsifiers (`lake exe falsify`, which runs the faithful and guarded variants on the bug witnesses) show that the premises bite. The `@[csimp]` equations transform the code reasoned about in proofs to the executable code ([Soundness](./soundness.md#the-trust-base)).

Reason: a sorry-free, gate-green build can still carry a vacuous headline, through a premise no run satisfies or a knot that makes every stream empty; a theorem whose premises can conflict silently is vacuous until a witness inhabits them ([Traps](./traps.md#deadlocked-knots)).

**R8. Zero sorries, standard axioms, no engine budgets.** The gate accepts only `propext`, `Classical.choice` and `Quot.sound` in the axiom reports of `Hydro/AxCheck.lean` and `Examples/AxCheck.lean`. The engine sets no heartbeat budget, and a `set_option maxHeartbeats` on a program definition is a measured sum, reported with its per-declaration table. Kernel work stays module-sized: no proof checks a whole-program definitional equality.

Reason: heartbeats are a proxy for proof time, and the goal is to reduce proof time; a generated proof that times out has a cliff in its script, which a larger budget hides, and definitional equality through nested knots is exponential in their depth ([Elaboration and Kernel Cost](./cost-model.md)).

**R9. Headline statements stay identical across changes.** The headlines, and the faces other modules consume, keep their statements byte-identical unless a change is agreed with the consumers listed. Internal faces may be reshaped toward the consumer's vocabulary.

Reason: a statement is what a reader checks once. Even an innocent-looking change, such as reordering `paxos_core`'s `fix` binders, changes the Bekić nesting (the order in which mutually recursive knots are closed one at a time) and every downstream definitional equality.

**R10. Processes are a compilation of the program, not an interpretation of it.** The elaborator compiles each module body into a typed graph (`M.dag`), `M.dag_denote` relates the graph to the program, and the distributed executable runs the graph. The network headline is proved by implication through the lock-step machine, and the `Values` side is untouched.

Reason: a `HydroSem` instance sees operator applications, not the body's `let`-sharing, so it would compute a shared wire once per consumer, and it cannot step a `fix` body once per global step ([The Approach](./approach.md#the-dataflow-graph)).

**R11. Type-class bounds sit where the Rust puts them.** `[Hashable K]` is on the operators that need it (the keyed folds, `join`, `anti_join`, `filter_not_in`, `reduce_watermark`) and on the programs that call them generically, never on carriers or the IR. The machine's semantics is stated over lists, and the hash map is a `@[csimp]`-checked implementation of it. A Rust `HashMap` carried as a value is `HMap`, a structure over `Std.ExtHashMap`.

Reason: `Std.HashMap` is not extensional (its bucket layout leaks into `=`), and a list-only keyed fold is O(keys × pool): the hash-backed operators run Paxos at 31k commits per second against 2.3k for the list-only ones, with no proof changed.

**R12. A member that is not active is inert, and an active member ticks or stutters.** A member's pace at a step is `idle`, `stutter` or `tick` ([The Step Machine](./semantics/sched.md)). The machine moves content across steps on its own in only two places, network delivery and `defer_tick`, and both are gated by the receiver's activity. Tick bodies fire only at ticks. A knot wire at a step equals its body's output at that step (`fix_stream_eq`), and this fixpoint equation holds because every cycle passes a hop or a `defer_tick` (`K_strict₁`, `M.dag_valid`). One DFIR tick is one tick at one member, and one activation of a process is one sweep of its graph. The test for a model change: one model tick has exactly the consequences of one DFIR tick, and the model never needs a tick that nothing in production triggers.

Reason: content that moves at an inactive member forces settling and quiescence hypotheses onto the wired theorem, and such hypotheses can be false ([Traps](./traps.md#undischarged-hypotheses)); with the rule, the correspondence holds for every interleaving ([Soundness](./soundness.md#link-5-interleaved-processes)).

**R13. The model follows production, never the proof.** The execution model (`Sched.lean`, the IR, `Node.step`/`locStep`, the activation) describes what `hydro_lang` and DFIR do. A change to it is admissible only when it cites the production behaviour it mirrors, never because a theorem would be easier. Devices the proofs need live in the proof layer, off the executable path, and facts such as tick-trace monotonicity are recorded in the corner, not in the machine.

Reason: the machine is part of what the reader audits; a proof device on the executable path is behaviour the binary runs and production does not ([Traps](./traps.md#what-the-model-must-admit)).

## Layout

A module file has this order:

1. Before the `hydro def`, only what the face's type needs:
   1. the `…Dec`/`…Sched` records;
   2. the vocabulary the face is stated in, with its decode lemmas;
   3. the loop-invariant predicate for the module's registers, with its `init`/`step` lemmas;
   4. the `…Ensures` structure.
2. The program.
3. The smoke tests and the census.

A theorem placed before the program that exists for one consumer belongs in that consumer as a ghost (R2).

## Process

The order is design, agreement, build, then the [gate](https://github.com/hydro-project/hydro/blob/main/hydro_lean/internal/GATE.md). Changes to a statement's shape, changes to `HydroSem` and engine extensions are agreed explicitly before they are built. Causes are measured, not hypothesized: a toy reproduction for a wrong answer, and the profiler for a slow one.

## The Verus Translation

The proofs are shaped so that the same structure ports to Verus in the Rust code. Only the denotational layer ports; the transfer to the machine is a Lean-side concern. The rows follow the Rust annotations of [Proof Obligations](../../hydro/reference/correctness/proof-obligations.md).

| here | Verus |
|---|---|
| `ensures out => MEnsures … out` | the `ensures` clause, a `spec fn` per field |
| `prove field := …` | one `proof fn` per field |
| `requires R` | the `requires` clause |
| `ghost pre h : S := …` before a fold or call | the `assert` that discharges the callee's `requires` / the conditional combiner's `manual_proof!(requires: …)` |
| `FoldOkP ord ret Pre g` | `commutative = manual_proof!(requires: Pre)`, the algebraic half proved once of the closure |
| `ghost have h := …` at a program point | `assert(…) by { … }` / `proof { … }` at that point |
| `ghost obtain ⟨x, hx⟩ := …` | `let ghost x = choose\|x\| …;` |
| `callee.ensures …` cited by a consumer | the callee's `ensures` at the call site |
| `tick … (invariant (out r …) => P)` | `invariant P(out, r, …)` on the `sliced!` loop |
| `prove init := …, tick := …` | the loop's entry assertion and the body's inductive step |
| `fix … invariant w (…) => P, base, step` | a loop invariant on the `forward_ref` cycle; a wire-only predicate is also the body's `requires` at the loop head |
| `tassume_ordering x dec.order` | `assume_ordering::<TotalOrder>(nondet!(…))`; the proof is over every order, the `nondet!` text states the argument why that is safe |
| `callee_mono ∘ callee.ensures` | a `proof fn` about the spec fn's monotonicity in its inputs |
| the `den` and `_at` readers | the spec fn's unfolding at a tick |
| `#nondet_census` | the module's `nondet!` sites, network edges, timers and `forward_ref` cycles, counted against declared numbers |
| `Trace.lean`/`Grades.lean` lemmas | `proof fn` library lemmas |

**Flo monotonicity** here means that a program's outputs grow in the grade's `PoolLe` order when its inputs grow. The Flo paper proves this property for its operator algebra. Here it is generated per module as `<M>_monoᵢ` by instantiating the module at `MonoRel` ([Monotonicity by Instantiation](./semantics/monorel.md)).
