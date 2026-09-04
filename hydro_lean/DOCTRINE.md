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
(D66: the keyed `fold_early_stop`/`get_max_key` site).

**P2 — Nondeterminism is a program input, split by who chooses.** Content
nondeterminism (`nondet!` sites: batch cuts, snapshot cuts, order selection, timing)
is the module's `…Dec` record; the adversary's freedom (delivery cursors) is its
`…Sched` bundle, `Unit` at `Values`. "Nondets are what proofs have to reason about,
not sched-dets." `#nondet_census` pins both counts to the Rust tally, per module,
build-enforced. (D54, D55.)

**P3 — The semantics mirrors Hydro, not a convenient abstraction of it.** No operator
exists in `HydroSem` without a Rust counterpart (the scan-across-ticks family was
deleted when this was noticed, D60–D61); a Rust grade is a Lean grade (`Monotonic`
singletons stay, D67); one tick's content is a plain list at the machine because the
runtime never erases order (D60). Adding an operator needs the Rust line that motivates
it and the rows for every interpretation plus its laws (`docs/09`).

## Proofs

**R1 — Proofs are directly over the denotational semantics.** No state-machine
abstraction layer, no "stage system", no pure mirror of a program body: the proof reads
the program's own wires under `den` and the construct's readers. "It's not hydro-style
to do proofs over state-machine-looking abstractions." (D57 — the rejected `PCStageSys`;
D64 — every hand step-mirror deleted.)

**R2 — Proofs are decomposed along the program.** Facts live at the program point they
are about: `ghost have` after the line that establishes them, `invariant` clauses on
the loop they govern, `prove field :=` for the contract's fields. A monolithic trailing
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
outside the construct mentions stages or scan states. (D57, D59, D62.)

**R6 — Any-body truths belong to the engine.** If a fact holds for every body of a
construct (the run face, the tick reader, the register abstraction, the Kleene chain,
the per-op coupling), the elaborator or a generic `Trace.lean` lemma emits it; it is
never re-proven per module. Engine extensions are any-body truths only and are ratified
before they are built, toys first (`HydroTickCheck.lean`, `HydroGenCheck.lean`). (D57,
D63, D64.)

**R7 — Every headline has an executable or structural witness.** `#guard` smoke tests
per module at `Values`/`Eager`, the four executables, `paxos_eager_den` tying what
runs to what is proven, the falsifiers showing the premises bite. A theorem whose
premises can silently conflict is vacuous until shown otherwise — the D37 lesson
(`paxos_safe_sched`'s `hsat` was unsatisfiable for nested knots; the theorem was
true and empty for a year). (D15, D37.)

**R8 — Zero sorries; standard axioms; no engine budgets.** `propext`,
`Classical.choice`, `Quot.sound` only (`AxCheck.lean`). A `set_option maxHeartbeats`
is a call-site fact measured by `scripts/budget_probe.sh`, never an engine default; a
timeout is a script bug to diagnose with `trace.profiler` (D56 found six 40-second
failed `isDefEq`s this way), not a budget to raise. Module-sized kernel costs only:
whole-program defeq is the recurring failure class (D40, D44).

**R9 — Headline statements are meaning-identical across refactors.**
`paxos_safe_sched'`, `cq_safe_sched'`, `paxos_eager_den*`, `cq_live*` and the exported
`…Ensures` faces that other modules consume keep their statements byte-identical
unless a change is ratified at a checkpoint with the consumers listed. Internal faces
may be reshaped (consumer-shaped faces are encouraged, R3).

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
| `ghost have h := …` at a program point | `assert(…) by { … }` / `proof { … }` at that point |
| `ghost obtain ⟨x, hx⟩ := …` | `let ghost x = choose|x| …;` |
| `callee.ensures …` cited by a consumer | the callee's `ensures`, available at the call site |
| `tick … (invariant (out r …) => P)` | `invariant P(out, r, …)` on the `sliced!` loop |
| `prove init := …, tick := …` | the loop's entry assertion and the body's inductive step (the per-tick obligation **is** the Verus loop body proof) |
| `fix … invariant w (…) => P, base, step` | a loop invariant on the `forward_ref` cycle; `base`/`step` its obligations; `h<w>_inv` the post-loop assert |
| `M_mono` ∘ `M.ensures` for "re-execution with larger inputs" | a `proof fn` lemma about the spec fn's monotonicity in its inputs |
| `den` readers / `_at` readers | the spec fn's definitional unfolding at a tick |
| `#nondet_census` | the `nondet!` sites, counted |
| pure `Trace.lean`/`Grades.lean` lemmas | `proof fn` library lemmas |

"Flo monotonicity" in this tree means: a program's outputs grow (in the grade's
`PoolLe` order) when its inputs grow — the property the dissertation's Flo chapter
proves for its operator algebra, and which here is generated per module as
`<M>_mono…` by instantiating the module at `MonoRel`/`MonoHRel`.
