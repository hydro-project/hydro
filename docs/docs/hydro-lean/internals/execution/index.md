---
sidebar_position: 0
---

# Graph and Execution

The machine theorems (`paxos_safe_sched`, `cq_safe_sched`) are about `SchedSem`, the step machine, and `SchedSem` is a type-class instance: it defines each operator on whole runs, sees operator applications rather than the program's `let`-sharing, and applies a `fix` body to a whole run rather than stepping it. It says what a run is; it cannot be executed one step at a time. What a deployed process runs is a step function: one cell of state per operator, this step's arrivals in, this step's outputs and sends out, each operator evaluated once. This category covers the gap between the two and the chain of theorems that closes it.

The generator compiles each module to a typed dataflow graph, `M.dag`, and proves `M.dag_denote`: the graph read with `SchedSem`'s definitions (`Dag.denote`) equals the module's run at `SchedSem` ([The Project Phase](../generator/project-phase.md)). Everything after that is proved once, for every graph, and each link is an equation, so a safety property moves down the chain unchanged:

| Page | Link | Theorem |
|---|---|---|
| [The Graph IR and Its Causality Theorems](./graph-ir-causality.md) | why a graph is needed at all, its representation, the lag analysis and cascade validity (every cycle passes a hop or a `defer_tick`), and the causality theorems the machine proofs use | `Dag.denote_causal`, `Dag.denote_agreeL`, `M.dag_valid` |
| [The Lock-Step Machine](./lockstep-machine.md) | the graph run one global step at a time against per-node cells, with every member of every location stepping, and a knot found as a fixpoint inside each step | `Dag.run_spec` |
| [Processes and the Compiled Step](./processes.md) | the lock-step step cut to one process, with only its own cells, and lifted to any interleaving of processes; the compiler behaviours that force how per-member tables are written | `Dag.step_wired`, `Interleaving.outputs` |
| [The Activation](./activation.md) | what a process runs when it wakes: one sweep of its graph that evaluates each node once, instead of re-running knot bodies per round, and the fused sweep the binary runs | `Dag.iterate_eq_pass`, `Dag.iterateBytes_eq_impl` |
| [Wire Codecs and Byte-Level Traces](./wire-and-traces.md) | bytes, frames and partial reads against typed messages, and the schedule a byte-level trace induces, so the theorem quantifies over every byte history | `WireTrace.streamOutputsAt_eq` |
| [Staging the Activation](./staging.md) | `hydro_run`, a symbolic executor that runs the sweep's control flow at elaboration time and emits straight-line code, checked equal to the interpreter by the kernel | `paxosRun.eq_impl`, `paxosRun.eq` |

The pages read in that order: each one starts from the theorem the previous one ends with. [Soundness](../soundness.md) walks the same chain from the contract down, link by link, with the hypotheses each link adds and the trust base.
