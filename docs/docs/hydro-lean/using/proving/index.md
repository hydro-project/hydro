---
sidebar_position: 0
---

# Proving Programs

A Hydro Lean module carries its contract, its **face**, and the proof of that face sits in the module next to the code ([Contracts](../programs/contracts.md)). You write the proof against the denotational semantics `Values`, where a stream's type encodes the histories it could have had, including every delay and reordering the network could cause, and every `nondet!` that observes one history is an argument ([The Network at `Values`](../programs/distributed-programs.mdx#the-network-at-values)). A protocol proof there is equational reasoning over the program's own collections: there is no scheduler, no set of in-flight messages and no reachable-state invariant over interleavings. Once the face is proved, you carry it to the operational semantics `SchedSem` and then to the processes exchanging bytes with a short, mechanical proof that cites lemmas `hydro def` generates for every module.

The examples are two-phase commit, Paxos and the `collect_quorum` stage both use, and every proof snippet carries `--` comments that say what each fact states and what each step does.

- [Proofs over Values](./proofs-over-values.md): stating facts between the program's lines with `ghost have`, reading a collection with `den`, using a callee's face instead of its body, closing the face with `prove`, and passing preconditions between modules with `requires` and `ghost pre`. Two-phase commit's unanimity proof runs through the page.
- [Tick Blocks and Their Invariants](./tick-invariants.md): a `sliced!` block with state is a loop over ticks. The page shows the facts each block generates about one tick and how to state and prove a loop invariant over its registers, with `collect_quorum`'s exactly-once emission as the example.
- [Knots and Loop Invariants](./knot-invariants.md): a `forward_ref` cycle is a fixpoint, which at `Values` you reason about by induction over its Kleene iterates. The page covers the `fix` block's `invariant` clause and the Paxos acceptor-log induction behind slot agreement.
- [Transferring to the Machine and the Network](./transferring-to-the-machine.md): the four moves that turn a face into a theorem about every run of the step machine, the causality argument that removes the remaining premise, and the proof that carries Paxos's theorem down to byte-level traces.
