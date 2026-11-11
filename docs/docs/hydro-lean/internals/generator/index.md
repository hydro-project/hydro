---
sidebar_position: 0
---

# The Generator

A contract proved at `Values` reaches the step machine and the processes only through a dozen facts about the module that carries it: what the machine run `sr` and the denotational run `rr` of its corner run are, which decision record the denotational run uses, its output lags, its residual `wf`, its step monotonicity, its monotonicity, and that its compiled graph agrees with the module. None of them can be a library theorem, because a program is a Lean function generic over its instance and has no syntax a theorem could induct over ([The Approach](../approach.md#per-program-proof-generation)). `hydro def` therefore states every one of them over the module's own signature and proves it while the module elaborates, so a user never writes or repairs one.

The obvious proof of each fact is to unfold the module and its operators and compare. That works on a toy and fails on real programs: definitional equality through nested knots is exponential in the nesting depth, `Meta.isDefEq` is up to 50 times slower than the kernel on graph terms, and a failing unification against a deep projection spine costs about 1.7 times more per level. The generator is built so that no proof unfolds a callee or a knot. Each module's facts are proved from its callees' facts, which the generator generated earlier and keeps by name, and from per-operator lemmas that are `rfl`; the generator adds no proof principle of its own.

[How Proofs Are Generated](../proof-generation.md) follows one program through every phase and collects the cross-cutting techniques. The pages here go into each phase in depth, in the order a module meets them:

| Page | What it explains |
|---|---|
| [Overview](./overview.md) | what every phase shares: the registry of generated names that lets callers keep callees folded, the spec walk, the routing of a module to the knot stack, the glue phase or the couple phase, the four proof entry points (tactic-script strings, kernel `Eq.refl`, direct terms), the diagnostics |
| [Generated Artifacts](./generated-artifacts.md) | every declaration a `hydro def` adds, with its statement for `toy_relay` or `toy_loop` and who consumes it |
| [Elaborating `hydro def`](./elaborating-hydro-def.md) | how the surface definition is split into a plain program, its contract, hoisted knot constants and one theorem per ghost fact, so that no later proof unfolds through a cycle or re-elaborates a ghost |
| [The `tick` Elaborator](./tick-elaborator.md) | why a tick block is one structural fold `tick_scan` with its shape at the type level, not a knot, and the reader facts it emits so proofs never unfold the fold |
| [The Couple Phase](./couple-phase.md) | how the unifier finds the decision record of the denotational run, which has no closed form, inside an existential, and how `genRrEx` defines it by `Classical.choose` |
| [The Glue Phase](./glue-phase.md) | how the glue engine proves the run equations of a module over knot-reaching callees by rewriting both sides to a common normal form with every callee folded, and keyed rule matching that rejects a wrong rule before unification |
| [The Knot Stack](./knot-stack.md) | how each knot gets its `sr`, `rr` and lag equations, strictness, Kleene stages, invariant and residual one knot at a time, by rewrite loops and terms that never cross a knot by definitional equality |
| [The `wf`, `smono` and `mono` Phases](./wf-smono-mono.md) | the residual script and its closers, step monotonicity by a dispatching walk, and monotonicity by instantiating the program at `MonoRel` |
| [The Project Phase](./project-phase.md) | compiling the module to a graph and proving `dag_denote` in two kernel hops with callee graphs folded, and validity one node at a time |

The terms these pages use (corner, machine run, denotational run, lag, residual, knot, decision) are defined in the [Vocabulary](../index.md#vocabulary) of the Architecture Overview.
