# 09 — Extending the engine

Most work never touches the engine: a new module is program text plus proofs
(chapter 08). You are in this chapter if the Rust uses an operator `HydroSem` lacks, or a
proof keeps paying for something every program would pay for. Both are ratified before
they are built (`../DOCTRINE.md` §process); both start with a toy.

## When a new operator is justified — and when it is not

An operator enters `HydroSem` (`Hydro/Sem.lean`) when a **mirrored Rust line** needs it
and no existing operator *is* that Rust operator. It does not enter for convenience:

- "I need a batch as a value to feed my tick block" was not an operator — the `tick`
  construct dispatches on the input's kind (`Atomic`/`AtomicStream`) and the Rust slice
  simply consumes its batched inputs (D60; a proposed `mapBatchUnordered` was rejected).
- A cross-tick fold that Rust writes as `use::state` is a `tick` register, not a
  `scan_across_ticks` operator — the whole scan family was deleted once this was seen
  (D60–D61).
- A pure helper over a wire's *value* (a keyed-fold closed form, a sum lemma) is a
  `Trace.lean`/`Grades.lean` lemma, not an operator.
- Conversely, a grade Rust has is a grade here even with one consumer:
  `SingBound.monotonic`/`fold_monotone` mirror `SingletonBound::Monotonic`
  (`hydro_lang/src/live_collections/singleton.rs:39–45`) and stay (D67).

The keyed fidelity item (D66) was of this kind: `paxos.rs:548–560` is
`into_keyed().assume_ordering().fold_early_stop().get_max_key().snapshot()`, and the Lean
let-chain modeled the keyed `fold_early_stop` as `H.fold` over an association list with
`get_max_key` applied after the snapshot — semantically equal, not operator-for-operator.
D70 added the operators (`into_keyed`, `kassume_ordering`, `kfold_early_stop`,
`kget_max_key`, `tinto_keyed`, `reduce_watermark`, the in-tick keyed API) through the
full checklist; D71 migrated PP1b/AP2/Recommit onto them under R3 (conditional `FoldOk
… Pre`) and added `tassume_ordering` (a per-tick order selection on a tick-located
unordered stream, decision `TickOrderSelDec`, denotation `realizeTicks` — an illegal
selection blocks the trace, so no legality premise) for the one Rust fold that is not
commutative at all.

- Conversely, a Rust *cast* is not an operator: `across_ticks(thunk) =
  thunk(all_ticks_atomic()).batched_atomic()` has no Lean counterpart because both casts
  are identities on the tick-located carrier — the thunk's operators are invoked directly
  on the atomic wire (`H.reduce_watermark (H.tinto_keyed x) …`), and the Rust quote
  above the line carries the `across_ticks` text (D70).

Shaped tick formers (`tick_scan`) need no per-arity rules: the projection of a former's
output along any `.1`/`.2` path is computed by one simproc per interpretation
(`eagTickScanProj`, `coTickScanProj`) — a definitional step, since `pack`/`mk` are
shape-structural. Do not add path tables (D70 removed them).

## What one operator costs: the `bmax` checklist

`bmax` (in-tick max of a bounded stream, Rust `.max()`) touches these files; every new
operator touches the same set:

| layer | file:line | what |
|---|---|---|
| signature | `Sem.lean:491` | the field, with Rust's grade constraints in its type (`[LinearOrder α]`) |
| `Values` | `Values.lean:331`, `:548` | the quotient implementation (`poolMax`) **and** its `@[den]` `rfl` reader |
| `SchedSem` | `Sched.lean:416` | the plain-list implementation (`b.foldl maxStep none`) |
| `MonoRel` | `MonoRel.lean:379` | delegate to `Values` |
| `CoupleSem` | `Couple.lean:1529–` | the paired implementation **with the coupling proof** (`cpl`: list result vs quotient result agree up to the grade's relation) |
| `CoupleProj` | `CoupleProj.lean:868–873` + the rule lists at `:1647`/`:1723` | `co_bmax_rr/_sr/_wf` `rfl` projections, registered for `co_transfer`/`co_wf_simp` |
| `HRel` laws | `MonoHRel.lean:461`, `CausalHRel.lean:203` | one law per `Laws` instance — the instances are anonymous constructors **ordered by field**, so the `by …_law_tac, -- bmax` line goes in the matching position (`../ENGINE_NOTES.md`) |
| walkers | `SchedCausal.lean` (per-op causality lemma) if the op is stream-level; `SchedMono.lean` (per-op `TMono` lemma) if it produces a tick-located wire | the knot `wf` kit's head-dispatch tables (`causalDispatch`/`coCausalDispatch`, `coSMonoDispatch`) |
| corner `mono` | `Couple.lean` | a tick carrier's `mono : wf → TMono sr` field — the machine leg's tick trace is prefix-monotone along steps (D71); the per-tick decision derivations (`tkOrdSelDerive`) read the realized tick through it |
| toy | `HydroTickCheck.lean` / `HydroGenCheck.lean` | a toy program using it, with `ensures` (an invariant toy without `ensures` has dead prove legs — D65) and `#print axioms` |

`hydro_rel_laws` (`HydroRel.lean:381`) regenerates the `HRel.Laws` bundle from the class
declaration, so the parametricity lift needs no hand edit — but each `Laws` *instance*
needs its law. Expect ~10 files and a compiler-error-guided walk; the b-ops landed in
D60–D61 are the precedent (13 fields in one pass), D81's 30 (21 in-tick, 9 stream-level)
the largest. Since D81 the projection set is a registered simp attribute
(`coSimpRules`/`coWfSimpRules` in `CoupleProj.lean`, loaded once by `registerCoRules`):
a new op is one entry in each list, and a stream-level op that a knot body may derive a
wire from also wants its `vmono_<op>` law found by `resolveRel` (the name is the key).
A `Project` node ctor per op (`StepOps*.lean`, `Wired*.lean`) completes the walk; an op
that produces an Atomic wire also owes `tpaced_<op>` (`SchedPaced.lean`): at most one
entry per step and none at an idle step, the fact that makes its production increment
a single slot (`Option`, D85). **In-tick ops stop earlier** (D89: `boOr`, `bisEmpty`,
`boFilterIf` were ~8 files each): they live inside `tick_scan`'s body, so no `Project`
node, no `SchedCausal`/`SchedMono`/`SchedPaced` lemma, no dispatch-table entry — the
signature, the four interpretations, the three `co_` projections, the two `HRel` law
lines, and a toy. **Before adding either kind, ask whether the Rust function is a program
over existing ops** — `timeout` and `sample_every` were primitives for six Ds and are now
`hydro def`s in `Std/Timers.lean` with `ensures` of their own; the question to put to a
candidate primitive is whether every interpretation would implement it by the same
composition. A primitive that takes a **data-dependent proof** (`latest_monotone`'s
ascent) cannot be handed one from a generic body today (`requires`/`ghost pre` facts live
in proof legs, not the term); the design that fixes this — interpretation-indexed `Prop`
fields lifted through the generated `_co_rr`/`_param` naturality — is queued (D89).

## Extending a construct

`tick` (`HydroTick.lean`) and `fix` (`HydroDef.lean`) are syntax transforms whose
emitted facts are **any-body truths** (`../DOCTRINE.md` R6). Extending one means:

1. Name the fact every body would otherwise prove by hand (D63's taxonomy is the model:
   "every module writes a 30-line register abstraction" → `h<out>_reg`).
2. Prove it once, generically, in `Trace.lean` (`scanAcrossTicks_invariant`,
   `scanAcrossTicksTrace_getElem?`, `prefix_ext_invariant`, …).
3. Emit it from the elaborator as a named ghost with the block's binder names.
4. A toy in `HydroTickCheck.lean` consuming it through a `prove` leg.
5. Migrate one real client; measure elaboration time before/after
   (`set_option trace.profiler true in`).

What the construct must not do: analyze the body's operators to decide semantics (it is
a syntax transform — the only type-directed step is reading each input's kind), or
emit a fact that is true only for some bodies.

## Generators and resolvers

If you touch `HydroGen.lean`/`HydroGenKnot.lean`: everything is **per module**; a
generated statement is closed and elaborated in an empty local context; candidate
resolution keys on head constant + projection path before any `isDefEq` (a failing
unification between two projections of one folded module application is a
whole-program whnf — D56); `instantiateMVars` before dispatching on heads; the
`co_simp`/`co_wf_simp` rule sets are `Array Name`s. `../ENGINE_NOTES.md` has the rest;
`HydroGenCheck.lean` pins every artifact class by `#check` so a regression is a build
error, not a silent degradation.

## Measuring

- `set_option trace.profiler true in` on the `hydro def` — attributes time to source
  positions, including inside generated scripts.
- `scripts/heartbeat_table.sh <file> [N]` — elaborate a file with the generator's
  diagnostics on (`HYDROGEN_HB_DIAG`, `HYDRO_GHOST_DIAG`, `HYDRO_PHASE_DIAG`) and list
  every generated theorem / hoisted ghost fact / knot-stack phase with its cost in
  `maxHeartbeats` units and wall time, largest first, plus the command-level usage and
  the per-phase wall clock. Heartbeats are a proxy, proof time is the goal (D72): the
  two program overrides (`LeaderElection` 600k, `PaxosCore` 300k) are the measured sums
  of everything one `hydro def` command elaborates; the table says which declaration
  is slow, and a slow declaration is an engine bug.
  `HYDROGEN_TAC_DIAG=<suffix>` prints a generated lemma's statement and script;
  `HYDROGEN_RULE_DIAG=1` prints `hydro_rule_apply`'s per-rule key decisions.
- Full build ≈ 5 min on 16 cores; `PaxosBench` (≈ 140 s, the deepest nesting — half the
  production-graph lemmas) and `LeaderElection` (three knots in one `fix`, ~77 s) are the
  long poles. A change that moves them is reported with the profile, not just the number.
- Two more cost classes (D81): a **default-transparency `apply`/`kabstract` against a
  module application that does not head the subject** — `hydro_wf_close`'s old `find?`
  rule (3.6 G heartbeats on one leg) and `rwRhs`'s `kabstract` (419 M) — select by HEAD,
  abstract at reducible transparency; and **per-call simp-set construction** (the
  class-size tax) — register the set once as a simp attribute.
- Two cost classes to know (D72): a **failing `apply`/unification against a deep
  shared projection spine** costs ~1.7× per level (the unifier retries each `Prod.snd`
  level in its `.proj` form on the way back up; failures with metavariables are not
  cached) — never try callee rules by unification, select them by key
  (`hydro_rule_apply`, `conclKeyMismatch`); and a **`#guard_eval` of a `Values` run
  must share** — a call-by-name carrier is interpreted at the denotation's speed (207 s
  for `leader_election`'s three guards, D72); since D73 the carriers are `MemberMap`s
  with a forcing compiled constructor and the guards take milliseconds.
- **The running system** (D86–D87, D92) is measured with the `bench` harness's faces:
  `HYDRO_BENCH_PROFILE=1` (an activation in three stages), `HYDRO_BENCH_NODES=1` (the step
  per operator, `Project/Profile.lean`), `HYDRO_BENCH_TRACE=1`, `bench idle P N` (an empty
  activation stage by stage) and `scripts/bench_perf_role.sh P` (`perf` with DWARF call
  graphs on one role, the machine thread separated from the socket threads; the inclusive
  view names operators, the flat view names runtime leaves — reference counting and
  allocation). A change is kept on a before/after measurement, never on the profile alone
  (D92 round 7 removed 7 % of self time and moved the throughput nowhere). The cost model
  to think in is fixed-per-activation (graph walks — one since D92), per-in-flight
  (`sliced!` blocks re-folding their state each tick, where DFIR's `'static` operators are
  incremental) and per-request (hash-map operations at Lean's ~100 ns constant); the
  closed-loop benchmark's throughput is in-flight over latency, and at the CPU wall more
  in-flight is only more latency. `ENGINE_NOTES.md` "The production path's costs" and
  "Profiling the benchmark" list what was found.
