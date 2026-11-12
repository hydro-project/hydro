---
sidebar_position: 0
---

# Writing Programs

A Hydro Lean program is a `hydro def`, a Lean function written against the operator signature `HydroSem`. The signature mirrors the `hydro_lang` API, so a program reads like Hydro code in Rust. You prove properties of this function, and `hydro def` compiles the same function into the dataflow graph that the deployed processes run. Writing a program involves the following choices.

- **Where the data lives and how it moves.** A program is parameterized by its locations, describes every member of a cluster at once, and sends collections between locations with network operators that return sender-keyed streams. [Locations and Networking](./distributed-programs.mdx) covers locations, member ids, `broadcast_closed` and `demux`, and the schedule record that holds each network edge's delivery timing.
- **What state persists.** A Rust `sliced!` block becomes a `tick` block, and a `forward_ref` cycle becomes a `fix … complete` block. [Ticks and Cycles](./ticks-and-cycles.mdx) shows the syntax of both.
- **Which collection types the program uses.** Each live collection has a carrier and a grade (its ordering and retry markers), and the grade decides which operators are available without a decision. [Carriers, Grades and Operators](./signature.md) maps the Rust types and operators to their Lean forms, including in-tick operators and closures.
- **Where the program observes a choice.** Every `nondet!` in the Rust becomes an explicit decision argument, bundled in the module's `…Dec` record, while network delivery and timers go in a separate `…Sched` record. [Nondeterminism and Decisions](./nondeterminism.md) covers the decision families, why every decision is legal by construction, and the census that counts each module's decision sites and fails the build when the count changes.
- **What the program promises.** A contract (`ensures`) is stated over the program's inputs and output at `Values`, proved inline with `prove`, and used by callers as a theorem `M.ensures`. [Contracts](./contracts.md) shows how to write a contract callers can use, and how `two_pc` consumes `collect_quorum`'s.

The [Quickstart](../quickstart/) writes two programs from scratch and then [ports an existing Rust function](../quickstart/porting-from-rust.mdx).
