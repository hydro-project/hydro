---
sidebar_position: 0
---

# Semantics

A Hydro Lean program is a function of an instance of the type class `HydroSem`, and each instance is a semantics for it. `Values` is the denotational semantics, where you prove contracts. `SchedSem` is the operational semantics, a step machine that runs the program step by step with explicit pacing and delivery, and that the compiled graph and the processes are proved equal to. `MonoRel` is a proof instance at which running a program proves it monotone. The fourth instance, the coupling `CoupleSem` that relates `SchedSem` runs to `Values` runs, has its own section, [The Coupling Corner](../corner/index.md). [The Approach](../approach.md) explains why the levels exist; these pages give the constructions, starting with the quotient carriers that `Values` and the coupling are built on.

- [Quotient Carriers and Fold Obligations](./grades.md): the quotients a stream's content lives in, one per grade, and how each operator is lifted to them, including the fold obligation and the refusal of order-dependent folds.
- [The Denotational Semantics](./values.md): `Values` — traces, tick shapes, decisions as cuts, keyed pools, and the network as identity.
- [The Step Machine](./sched.md): `SchedSem` — flat concrete streams on a global clock, delivery floors, pacing, the ungated knot, and its causality and pacing laws.
- [Monotonicity by Instantiation](./monorel.md): `MonoRel`, the instance at which running a program proves it monotone.
