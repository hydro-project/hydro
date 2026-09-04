# Doctrine — the binding rules for programs and proofs

These are the rulings the tree is built to, in the words they were ruled in (the
D-number is where `FINDINGS.md` records the ruling and the example that forced it).
They are not style preferences: each one closed off a failure class that had actually
occurred. A change that regresses one of them is wrong even if it compiles.

## Programs

**P1 — Program text is a 1:1 mirror of the Rust.** One Rust function = one `hydro def`.
The `let`-chain is sacred: same operators, same order, same binder names, the Rust
line quoted in a comment above each Lean line, `paxos.rs:line` anchors in the
docstring. A `sliced!` body is a `tick` block whose per-tick let-chain is Rust's
in-tick dataflow over the b-ops; `use::state` registers are `(state …)` binders;
`forward_ref`/`complete_cycle` is `fix … complete`. Where the Lean has a line the Rust
does not (B2's recommit gate, `SequencePayload.lean`), the comment says so and the
line is written as the Rust it *would* be. (D51, D53, D58, D61.) A let-chain that is
only *semantically* equal to the Rust is a queued fidelity item, not an accepted state
(D66: the keyed `fold_early_stop`/`get_max_key` site). **Closures are functions**
(ruling F, D81): a Rust closure passed to a stage (`with_client`'s payload generator,
`bench_client`'s workload generator and protocol) is a `hydro def` of its own, named as
the `fn` Rust would write, taking the wires the closure captured and closed over the
stage by the caller's `fix … complete` — the cycle a `forward_ref`/`TickCycleHandle`
threads through it (`payloads_to_leader`, `tag_with_sender`, `paxos_protocol`). A
`forward_ref` whose closing a callee performs (`sequence_payloads`' `r_next_slot`) is a
wire argument of the callee and a knot of the caller — until `hydro def` is higher-order
(queued, D83: the stage takes the closures as module arguments and owns its cycle, as the
Rust does). Where the Lean's chain *order*
differs from the Rust's because of this (the measurement lines inside `paxos_bench`'s
knot body), the program text says so and the reason is an engine queue item, not a
style choice.

**P2 — Nondeterminism is a program input, split by who chooses.** Content
nondeterminism (`nondet!` sites: batch cuts, snapshot cuts, order selection, timing)
is the module's `…Dec` record; the adversary's freedom (delivery cursors, and since
D91 an interval timer's fire schedule — `source_interval` denotes the one `AtLeastOnce`
class of `()`, so when it fires changes nothing a proof can see) is its
`…Sched` bundle, `Unit` at `Values`. "Nondets are what proofs have to reason about,
not sched-dets." `#nondet_census` pins both counts to the Rust tally, per module,
build-enforced. (D54, D55.) **Decisions are made total by construction**: a decision
space never carries a legality side condition that a theorem must assume — an illegal
cut is a truncation (`batchCuts`), an illegal per-tick order BLOCKS the trace at that
tick (`realizeTicks`, D71). So a headline over "all decisions" is over all of them,
and the corner derives the realized decision from the machine run (the `…Derive`
functions), which is where "`Values` covers every real concurrent execution" is paid.
A per-tick decision needs the machine's tick trace to be monotone to be read off it;
that fact is recorded in the corner (`mono`), not in `Sched` — `Sched` stays the
minimal, auditable operational semantics; `Couple` is where proofs live (D71, (A)).

**P3 — The semantics mirrors Hydro, not a convenient abstraction of it.** No operator
exists in `HydroSem` without a Rust counterpart (the scan-across-ticks family was
deleted when this was noticed, D60–D61); a Rust grade is a Lean grade (`Monotonic`
singletons stay, D67); one tick's content is a plain list at the machine because the
runtime never erases order (D60). Adding an operator needs the Rust line that motivates
it and the rows for every interpretation plus its laws (`docs/09`). Where the Rust is
weaker than the protocol needs, the Lean takes the grade the proof needs and the
divergence is written down for upstream, never silently: the acceptor checkpoint is
`.monotonic` here (Rust's `Optional` has no `Monotonic` bound yet; a retreating
checkpoint lets an acceptor GC a slot its view does not cover — `SCHED_AUDIT.md`,
D71). Where the Rust is *wrong*, the Lean carries both (`PaxosVariant.faithful` /
`.guarded`), proves the fix and runs the bug (B1, B2, B4).

## Proofs

**R1 — Proofs are directly over the denotational semantics.** No state-machine
abstraction layer, no "stage system", no pure mirror of a program body: the proof reads
the program's own wires under `den` and the construct's readers. "It's not hydro-style
to do proofs over state-machine-looking abstractions." (D57 — the rejected `PCStageSys`;
D64 — every hand step-mirror deleted.)

**R2 — Proofs are decomposed along the program.** Facts live at the program point they
are about: `ghost have` after the line that establishes them, `ghost pre` before the
operator site whose `Pre` it discharges, `invariant` clauses on the loop they govern,
`prove field :=` for the contract's fields, `requires` for the premise the module
genuinely needs (and it is a premise of the module — `hreq` in its proofs — never a
conditional on the `ensures` face: a consumer discharges a callee's `requires` by
`ghost pre` at the call, D71). A monolithic trailing
tactic block is a design smell; a separate lemma/theory file for a module is forbidden
(`PaxosCoreLemmas`, `SequencePayloadLemmas`, `RecommitTheory`, `QuorumTheory`,
`PP1bLemmas` were all dissolved, D57–D66). Genuinely pure mathematics (list/multiset
algebra, keyed-fold closed forms) goes to `Trace.lean`/`Grades.lean`, not to a
module-named file.

**R3 — Ensures are over outputs, never internal wires; consumers consume ensures,
period.** A contract face mentions the module's outputs and inputs only. A consumer
cites `callee.ensures …` and the generated `callee_mono…`; it never unfolds a callee,
never re-derives a callee fact, never receives an exported internal `have`. "If I
output this, re-execution with larger inputs guarantees X" is `M_mono ∘ M.ensures`,
composed by the consumer — not a new face and not an internal wire. (D64: K4 restated
over log entries so `SPEmission`, a predicate on sequencing's *internal* sent traffic,
could be deleted along with the mirror it required.)

**R4 — Ghosts refer to program wires by their program names.** An unfolded wire
denotation in a proof, or a `ghost let` alias of a wire, is a bug in how the fact
reached that proof — fix the face or the binder, don't alias. (D63's E5 ruling: the
`foldAcrossTicksTrace … ap2QualBatches …` ×25 respell in `sequence_payload` was a
callee face stated in unfolded vocabulary; the fix was consumer-shaped `AP2Ensures`
fields — `log_entry_src`, `log_covers_mono`, `log_len_le_ck` — proven inside
`acceptor_p2` from its own `hlog_pool` ghost.)

**R5 — Loop invariants are the construct's.** A `tick` block's cross-tick fact is its
`invariant` clause in loop-invariant normal form — a predicate on (emissions so far,
registers, spectator inputs), no history indices — with `prove init`/`tick` as the
only obligations; the construct supplies the induction, the readers and the register.
Likewise a `fix` knot's cross-stage fact is its `invariant` with `base`/`step`. Nothing
outside the construct mentions stages or scan states. (D57, D59, D62.) A `fix`
`invariant` that speaks of the wire alone is a Verus loop invariant in the full sense
— assumable of the loop input, proven of the loop output (D71); one keyword, no
`assume`/`export` dialects.

**R6 — Any-body truths belong to the engine.** If a fact holds for every body of a
construct (the run face, the tick reader, the register abstraction, the Kleene chain,
the per-op coupling, tick-trace monotonicity), the elaborator or a generic
`Trace.lean`/`SchedMono.lean` lemma emits it; it is never re-proven per module. Engine extensions are any-body truths only and are ratified
before they are built, toys first (`HydroTickCheck.lean`, `HydroGenCheck.lean`). (D57,
D63, D64.)

**R7 — Every headline has an executable or structural witness.** `#guard_eval` smoke
tests per module at `Values`, the five executables — the `Values` programs themselves
and, for `wired`, the per-process `Dag.locStep` over the elaborator-compiled graph —
with nine compiled seams (`MemberMap.mk'` → `mkImpl`, the `memoBox*` tables, and the
D75 hash-backed keyed operators `dedupH`/`listFilterNotIn`/`listAntiJoin`/`listJoinH`/
`keyedFoldAssocH`/`reduceWatermarkEagerH` → their `…Impl`, each authorized by its
`@[csimp]` equation — a kernel-checked proof is the only thing that lets the compiler
substitute;
`implemented_by`, `extern`, `unsafe` and `native_decide` are not used in `Hydro/`)
tying what runs to what is proven — and the falsifiers showing the premises bite. A theorem whose
premises can silently conflict is vacuous until shown otherwise — the D37 lesson
(`paxos_safe_sched`'s `hsat` was unsatisfiable for nested knots; the theorem was
true and empty for a year). (D15, D37.)

**R8 — Zero sorries; standard axioms; no engine budgets.** `propext`,
`Classical.choice`, `Quot.sound` only (`AxCheck.lean`). Heartbeats are a proxy; proof
TIME is the goal (D72). The engine carries no budget; a `set_option maxHeartbeats` on a
program def is a measured call-site fact (one command elaborates the def and every
declaration it generates), reported with its per-declaration table
(`scripts/heartbeat_table.sh`), never an engine default. A slow proof is an engine bug to
diagnose with `trace.profiler` (D56 found six 40-second failed `isDefEq`s this way; D72
a failing `apply` costing 1.7× per shared projection level, and a witness interpreted
at the denotation's speed) — not a budget to raise. Module-sized kernel costs only:
whole-program defeq is the recurring failure class (D40, D44).

**R9 — Headline statements are meaning-identical across refactors.**
`paxos_safe_sched'`, `cq_safe_sched'`, `cq_live*` and the exported
`…Ensures` faces that other modules consume keep their statements byte-identical
unless a change is ratified at a checkpoint with the consumers listed. Internal faces
may be reshaped (consumer-shaped faces are encouraged, R3).

**R11 — Type-class bounds sit where the Rust puts them (D75).** `K: Hash` is demanded by
`fold_keyed`/`join`/`anti_join`/`filter_not_in`/`reduce_watermark`, not by `KeyedStream`;
so `[Hashable K]` is on those `HydroSem` methods and on the programs that call them
generically, never on the keyed carriers or the IR kinds. The machine's semantics stays
the list; the hash map is a `@[csimp]`-checked implementation of the per-tick batch.
A Rust `HashMap` carried as a *value* is `Std.ExtHashMap` (`Hydro.HMap`), the extensional
one — never a sorted list standing in for it.

**R12 — A member that is not active is inert; an active member ticks or stutters
(D76).** A member's pace at a step is `idle`, `stutter` or `tick` (`Pace`, `Pacing`).
`SchedSem` moves content across steps on its own in exactly three places — network
delivery (`StepHist.deliver`), the knot feedback wire (`StepHist.shift`, `fix_tick`'s
implicit defer) and `defer_tick`'s seed — and each is gated by the receiving member's
*activity* (`gateView`, `pacing.active`): an active step sees the source as of the
previous step (the `+1` floor stays), an idle step repeats. Tick bodies (`batch`,
`snapshot`, timers, `defer_tick`) fire only at `tick` steps (`pacing.ticks`). Every
other operator is pointwise in its inputs' views and inherits inertness. A **stutter**
is an active step without a tick: the member's asynchronous dataflow progresses —
deliveries land, knot feedback propagates, stream operators fire — and no `Tick`
location fires. The model may never need a tick nothing in production triggers: a
forward_ref knot closed through the network needs the feedback to propagate one step,
and DFIR does that inside one tick (stratum order); the model does it as a stutter. **One
DFIR tick = stutters, then a tick** at one member (every `Tick` location of that member at
once — DFIR runs them on one counter). A process that is not running is an idle member
of the lock-step machine, and the wired correspondence (`Dag.step_wired`,
`Interleaving.outputs`) holds for **every**
interleaving — no settling, quiescence, feasibility, cursor-stillness or initial-prefix
hypothesis. Idleness is irrelevant to safety; a semantics in which it mattered was an
artifact. The only state condition left, `Dag.Drained` (multi-input tick cells hold no
complete item), is true of every state the machine produces and is a theorem, not a
premise.

**R13 — The model follows production, never the proof (D78).** The execution model —
`Sched.lean`, the IR, `Node.step`/`locStep`, the loop body `Dag.iterate` — is a
description of what `hydro_lang`/DFIR do. A change to it is admissible only with the
production behaviour it mirrors named (`dfir_rs`/`hydro_lang` file and line, entered in
`SCHED_AUDIT.md`'s ground-truth table), and never because a theorem would be easier to
state or prove. Devices the proofs need — projections (`arrivedOnly`), derived oracles
(`cursorsAt`), bundle conversions (`recordOf`) — live in the proof layer; they must not
appear on the executable path, and when one does it is a recorded deviation with a
fix queued. The risk this guards against is silent divergence: a model that drifts from
production to fit the theorems proves something about a system nobody runs.

**R10 — Processes are a compilation of the program, not an interpretation of it
(D74).** The distributed executable is the elaborator's compilation of each module body
into a typed dataflow graph (`M.dag`), related to the program by a per-module
denotation theorem (`M.dag_denote`, over `SchedSem`) — never a fifth tagless-final
instance: an instance sees applications, not the body's `let`-sharing, and cannot step
a `fix` body once per global step (observable sharing). The graph is stepped per
process at its own members only (`Dag.locStep`), and the headline for a network of
processes (`paxos_safe_wired`) is proved by implication through the lock-step machine
(`Dag.step_wired` → `Interleaving.outputs` → `Dag.run_spec` → `M.dag_denote` →
`paxos_safe_sched'`), the `Values` side untouched. The execution model inherits no
scheduling constraint: a member that is not active is inert (R12), so a process that is
not running leaves the lock-step machine's cells exactly where it left them.

## Layout (`E10`, D64)

Before the `hydro def`, **only what the contract face's type needs**, in this order,
under one header `Prerequisites for the contract face — the program starts at
hydro def M`: the `…Dec`/`…Sched` records → the vocabulary the face is stated in and
its decode lemmas → the loop-invariant vocabulary for the module's `tick` registers
(the one allowed pre-def proof budget — `CQRegInv` with `init`/`step`, `OnceInv`) →
the `…Ensures` structure. Then the program. Then smoke tests and the census. Nothing
else precedes the program.

## Process

Checkpoints before building (design → user ratification → build); statement-shape
changes, `HydroSem` changes and engine extensions are ratified explicitly; every pass
ends with the gate (`GATE.md`) and a `FINDINGS.md` entry recording the cause, the fix,
the measurements and the gotchas. Measured causes over hypotheses: toy-repro for
wrong-answer bugs, profiler for slow-answer bugs (D56, D63).

## The Verus translation table

The Lean proofs are shaped so that the *same proof structure* ports to Verus in the
Rust tree. Only the denotational layer ports (Verus has no step machine to couple to;
the transfer property is a Lean-side concern).

| here | Verus |
|---|---|
| `ensures out => MEnsures … out` (the contract face, over outputs) | `ensures` clause with a `spec fn` per field |
| `prove field := …` | one `proof fn` per field |
| `requires R` | the `requires` clause (a premise of the function, not of a field) |
| `ghost pre h : S := …` before a `fold`/call site | the `assert` that discharges the callee's `requires` / the `manual_proof!(requires: …)` of a conditional combiner |
| `FoldOk ord ret Pre g` | `commutative = manual_proof!(requires: Pre)` — the algebraic half, proven once of the closure |
| `ghost have h := …` at a program point | `assert(…) by { … }` / `proof { … }` at that point |
| `ghost obtain ⟨x, hx⟩ := …` | `let ghost x = choose|x| …;` |
| `callee.ensures …` cited by a consumer | the callee's `ensures`, available at the call site |
| `tick … (invariant (out r …) => P)` | `invariant P(out, r, …)` on the `sliced!` loop |
| `prove init := …, tick := …` | the loop's entry assertion and the body's inductive step (the per-tick obligation **is** the Verus loop body proof) |
| `fix … invariant w (…) => P, base, step` | a loop invariant on the `forward_ref` cycle; `base`/`step` its obligations; `h<w>_inv` the post-loop assert; a wire-only predicate is also the body's `requires` at the loop head |
| `tassume_ordering x dec.order` | `assume_ordering::<TotalOrder>(nondet!(…))` — the proof is over every order, the `nondet!` text is the argument why that is safe |
| `M_mono` ∘ `M.ensures` for "re-execution with larger inputs" | a `proof fn` lemma about the spec fn's monotonicity in its inputs |
| `den` readers / `_at` readers | the spec fn's definitional unfolding at a tick |
| `#nondet_census` | the `nondet!` sites, counted |
| pure `Trace.lean`/`Grades.lean` lemmas | `proof fn` library lemmas |

"Flo monotonicity" in this tree means: a program's outputs grow (in the grade's
`PoolLe` order) when its inputs grow — the property the dissertation's Flo chapter
proves for its operator algebra, and which here is generated per module as
`<M>_mono…` by instantiating the module at `MonoRel`/`MonoHRel`.
