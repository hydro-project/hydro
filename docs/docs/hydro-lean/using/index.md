---
sidebar_position: 0
---

# Introduction

Hydro Lean uses the same core primitives as Hydro in Rust, including [live collections](../../hydro/reference/introduction/live-collections.md) and [location-oriented programming](../../hydro/reference/introduction/location-oriented-programming.md). But unlike Rust, which is limited to [deterministic simulation](../../hydro/reference/simulation/index.mdx), Hydro Lean offers mechanisms that enable proving arbitrary safety properties over _unbounded_ executions. A safety theorem about a Hydro Lean program holds for:

1. every input, at every size;
2. every resolution of every `nondet!` in the program (each batch boundary, snapshot point, order assumption and clock reading);
3. every execution of the program as a concurrent system: a model in which each cluster member advances on its own on a global step clock, messages arrive after any delay, and any member may sit idle for any number of steps;
4. every byte-level trace of the program running as separate processes over a network: who ran at each step, what it was handed, and how many bytes it read off each connection. This level needs a theorem about the program's compiled processes, for example Paxos's `paxos_safe_wired`.

In a traditional model checker the state machine is a model, separate from the code that runs. A Hydro Lean program is itself executable, and Hydro Lean transfers its proven properties to the executable code. For example, when `lake exe wired` runs Paxos as five processes over local TCP, each process calls `paxosBody`, and the main safety theorem `paxos_safe_wired` quantifies over `paxosBody`. When `lake exe falsify` shows that the Rust Paxos as written can commit two values at one slot, it runs the same Lean term that the safety theorem is about, with one flag changed.

Hydro Lean comes with several examples in the `hydro_lean/` directory of this repository, each ported from the Rust line for line: the quorum stage of `hydro_std/src/quorum.rs`, Paxos from `hydro_test/src/cluster/paxos.rs`, two-phase commit from `two_pc.rs`, and the Paxos benchmark with its clients and replicas from `paxos_bench.rs`.

## Unbounded Proofs with Live Collections

State-machine reasoning in the style of TLA+ treats the network as part of the system state, a bag of in-flight messages, and proves invariants over every interleaving of sends, deliveries and local steps. Proofs in Hydro Lean use a fundamentally different structure. Because a Hydro function encapsulates the network as just another operator, you implement an entire distributed system and _prove_ it with the same techniques as a plain top-down function.

This enables cleaner composition of proofs: for example, the leader election and sequencing components of Paxos can be separately proven and composed into the larger protocol. It also dramatically simplifies proofs, by leveraging the Hydro type system to automatically prove properties that would otherwise require manual invariants.

In Hydro Lean, you prove properties using a _denotational semantics_ where distributed streams are captured using mathematical objects like sets. The object for a stream captures every history the stream could have had: every arrival order of an unordered stream, every retry count of an at-least-once one. The nondeterminism is deferred, carried unresolved inside the value, until an operator observes it. An operator whose result is the same for every history the stream captures, such as a fold whose closure carries a commutativity proof, needs no choice. Only an operator that observes one particular history (where a tick's batch ends, a fold whose result depends on the order) takes its `nondet!`, and in Lean that `nondet!` is an explicit argument. The network's delay and reordering are encoded the same way, in the types of the streams a receiver gets, so the network nondeterminism is deferred too, and resolved only where an operator observes it.

Programs in Hydro Lean are also executable, with a compilation architecture that yields high-performance binaries. Once you have proven a property in the denotational semantics, Hydro Lean _automatically_ transfers it to _real_ concurrent executions of these compiled programs, so you can deploy to production with confidence.

[Grades](./programs/signature.md#grades) shows what this means for the collection types you write, [The Approach](../internals/approach.md#quotient-denotations) gives what it costs, and [Quotient Carriers](../internals/semantics/grades.md) gives the construction.

## How to Read These Docs

**Using Hydro Lean** is for writing a program and proving it. **Framework Architecture & Internals** is for extending the engine (the Lean metaprograms that turn a program into its theorems and its executable), or for judging what a theorem means.

- To write and prove a program, read [Setup](./setup.md), then work through the [Quickstart](./quickstart/), which writes, proves and runs two small programs from scratch and then ports a real Rust function. The pages under [Writing Programs](./programs/) and [Proving Programs](./proving/proofs-over-values.md) are the reference you return to. [Adding a Module](https://github.com/hydro-project/hydro/blob/main/hydro_lean/internal/ADDING_A_MODULE.md) and [the gate](https://github.com/hydro-project/hydro/blob/main/hydro_lean/internal/GATE.md) cover landing a change.
- To judge the theorems, read [Transferring to the Machine and the Network](./proving/transferring-to-the-machine.md), then [Soundness](../internals/soundness.md) and [Fidelity to the Rust](https://github.com/hydro-project/hydro/blob/main/hydro_lean/internal/FIDELITY.md).
- To weigh Hydro Lean against model-based tools, read [Hydro Lean, TLA+ and Veil](./comparison.mdx), which verifies the Quickstart's quorum in all three and compares what each one checks, automates and costs.
- To extend the engine, read the [Architecture Overview](../internals/index.md) and [The Approach](../internals/approach.md), then [How Proofs Are Generated](../internals/proof-generation.md) and [Elaboration and Kernel Cost](../internals/cost-model.md) before you touch a generator phase. Read [Traps](../internals/traps.md) before you debug one.
