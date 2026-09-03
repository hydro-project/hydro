---
sidebar_position: 4
sidebar_label: Core Macros API
---

# Core Macros API

Use this page to find the core macros used throughout Hydro programs. Follow the Hydro Guide links for explanations and examples, and use the Rust API links for exact definitions and details.

| Task | Macro | Guide |
| --- | --- | --- |
| Quote Rust code to run inside the dataflow | [`q!`](rust:hydro_lang::prelude::q) | [Stageleft](../reference/stageleft/index.mdx) |
| Reveal bounded batches or snapshots of live collections | [`sliced!`](rust:hydro_lang::live_collections::sliced::sliced) | [Slice Blocks](../reference/state-management/slices.mdx) |
| Document and pass a non-determinism guard | [`nondet!`](rust:hydro_lang::nondet::nondet) | [Non-Determinism and `nondet!`](../reference/correctness/nondet.md) |
| Supply a manually checked algebraic property | [`manual_proof!`](rust:hydro_lang::properties::manual_proof) | [Stream Ordering and Determinism](../reference/streaming-data/streams.md#stream-ordering-and-determinism) |
| Initialize a Hydro crate | [`setup!`](rust:hydro_lang::setup) | [Stageleft](../reference/stageleft/index.mdx) |
| Restrict a simulation to executions satisfying a precondition | [`continue_if!`](rust:hydro_lang::sim::continue_if) | [Restricting Explored Executions](../reference/simulation/writing.mdx#restricting-explored-executions-with-continue_if) |

Most applications import the [`hydro_lang::prelude`](rust:hydro_lang::prelude), which re-exports the commonly used macros alongside core collection and location types.
