---
sidebar_position: 0
---

# The Coupling Corner

You prove a contract at the denotational semantics `Values`; safety must hold of every run of the step machine `SchedSem`. The coupling instance `CoupleSem` relates the two once, for every program: run at it, a program yields its machine run, its denotation at decisions derived from that run, and a proof that the first sits below the second.

- [The Coupling Instance](./coupling.md): the corner carrier, derived decisions, the per-operator couplings, lag and the network hop, knots and their residual, and ticks.
- [Projections and Tactics](./projections-and-tactics.md): the definitional projection lemmas, the `co_*` tactic macros that use them, and the rewrite loops that prove a knot's `sr` and `rr` equations without unfolding through it.
