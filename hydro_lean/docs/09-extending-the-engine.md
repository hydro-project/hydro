# 09 — Extending the engine

Most work never touches the engine: a new module is program text plus proofs
(chapter 08). You are in this chapter if the Rust uses an operator `HydroSem` lacks, or a
proof keeps paying for something every program would pay for. Both are ratified before
they are built (`../DOCTRINE.md` §process); both start with a toy.

## When a new operator is justified — and when it is not

An operator enters `HydroSem` (`Hydro/Sem.lean`) when a **mirrored Rust line** needs it
and no existing operator *is* that Rust operator. It does not enter for convenience:

- "I need a batch as a value to feed my tick block" was not an operator — the `tick`
  construct dispatches on the input's kind (`Ticked`/`TickStream`) and the Rust slice
  simply consumes its batched inputs (D60; a proposed `mapBatchUnordered` was rejected).
- A cross-tick fold that Rust writes as `use::state` is a `tick` register, not a
  `scan_across_ticks` operator — the whole scan family was deleted once this was seen
  (D60–D61).
- A pure helper over a wire's *value* (a keyed-fold closed form, a sum lemma) is a
  `Trace.lean`/`Grades.lean` lemma, not an operator.
- Conversely, a grade Rust has is a grade here even with one consumer:
  `SingBound.monotonic`/`fold_monotone` mirror `SingletonBound::Monotonic`
  (`hydro_lang/src/live_collections/singleton.rs:39–45`) and stay (D67).

The one queued fidelity item is of this kind: `paxos.rs:548–560` is
`into_keyed().assume_ordering().fold_early_stop().get_max_key().snapshot()`, and the Lean
let-chain currently models the keyed `fold_early_stop` as `H.fold` over an association
list with `get_max_key` applied after the snapshot — semantically equal, not
operator-for-operator. Making it literal means two stream-level operators (keyed
`fold_early_stop`, `get_max_key`) through the full checklist below (D66).

## What one operator costs: the `bmax` checklist

`bmax` (in-tick max of a bounded stream, Rust `.max()`) touches these files; every new
operator touches the same set:

| layer | file:line | what |
|---|---|---|
| signature | `Sem.lean:491` | the field, with Rust's grade constraints in its type (`[LinearOrder α]`) |
| `Values` | `Values.lean:331`, `:548` | the quotient implementation (`poolMax`) **and** its `@[den]` `rfl` reader |
| `SchedSem` | `Sched.lean:416` | the plain-list implementation (`b.foldl maxStep none`) |
| `Eager` | `Eager.lean:406` | delegate to `Values` |
| `MonoRel` | `MonoRel.lean:379` | delegate to `Values` |
| `CoupleSem` | `Couple.lean:1529–` | the paired implementation **with the coupling proof** (`cpl`: list result vs quotient result agree up to the grade's relation) |
| `CoupleProj` | `CoupleProj.lean:868–873` + the rule lists at `:1647`/`:1723` | `co_bmax_rr/_sr/_wf` `rfl` projections, registered for `co_transfer`/`co_wf_simp` |
| `HRel` laws | `MonoHRel.lean:461`, `CausalHRel.lean:203`, `EagerRel.lean:175` | one law per `Laws` instance — the instances are anonymous constructors **ordered by field**, so the `by …_law_tac, -- bmax` line goes in the matching position (`../ENGINE_NOTES.md`) |
| walkers | `SchedCausal.lean` (per-op causality lemma) if the op is stream-level | the knot `wf` kit's head-dispatch table |
| toy | `HydroTickCheck.lean` / `HydroGenCheck.lean` | a toy program using it, with `ensures` (an invariant toy without `ensures` has dead prove legs — D65) and `#print axioms` |

`hydro_rel_laws` (`HydroRel.lean:381`) regenerates the `HRel.Laws` bundle from the class
declaration, so the parametricity lift needs no hand edit — but each `Laws` *instance*
needs its law. Expect ~10 files and a compiler-error-guided walk; the b-ops landed in
D60–D61 are the precedent (13 fields in one pass).

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
- `scripts/budget_probe.sh <file> <value>` — elaborate a file with its `maxHeartbeats`
  line replaced; `0` deletes it. The two remaining budgets (`LeaderElection` 1.6M,
  `PaxosCore` 400k) are the measured minima at power-of-two.
- Full build ≈ 6 min; `LeaderElection` (three knots in one `fix`, ~250 s) is the long
  pole. A change that moves it is reported with the profile, not just the number.
