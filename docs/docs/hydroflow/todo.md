---
sidebar_position: 8
---

# TODO

## Concepts
- p1 (Mingwei) Hydroflow and Rust: how do they go together?
    - State, control, scoping
- p1 State over time
    - lifetimes
    - explicit deletion
- p3 Coordination tricks?
    - End-of-stream to Distributed EOS?

## Docs
- p1 `hydroflow` struct and its methods
- p2 Review the ops docs

## Operators not discussed
    - dest_sink
    - identity
    - repeat_iter
    - unzip
    - p1 *fold* -- add to chapter on [state](./syntax/state.md)
    - p1 *reduce* -- add to chapter on [state](./syntax/state.md)
    - p1 *group_by* -- add to chapter on [state](./syntax/state.md)
    - p3 *sort_by* -- add to chapter on [state](./syntax/state.md)
    - p2 *next_stratum* -- add to chapter on [time](./concepts/life_and_times.md)
    - p2 *next_tick* -- add to chapter on [time](./concepts/life_and_times.md)
    - p2 *inspect* -- add to chapter on [debugging](./concepts/debugging.md)
    - p2 *null* -- add to chapter on [debugging](./concepts/debugging.md)

## How-Tos and Examples
- p1 Lamport clocks
- p2 Vector clocks
- p2 A partitioned Service
- p2 A replicated Service
- p2 Interfacing with external data
- p2 Interfacing with external services
- p1 Illustrate `'static` and `'tick` lifetimes (KVS)
- p3 Illustrate the `next_stratum` operator for atomicity (eg Bloom's upsert `<+-` operator)
- p3 Illustrate ordered streams (need `zip` operator ... what's the example?)
- p3 Actor model implementation (Borrow an Akka or Ray Actors example?)
- p3 Futures emulation? (Borrow a Ray example)
- p2 Illustrate external storage source and sink (e.g. for WAL of KVS)

## Odds and ends taken out of other chapters
- **Document the methods on the `hydroflow` struct** -- especially the run methods.
    -  The [`run_tick()`](https://hydro-project.github.io/hydroflow/doc/hydroflow/scheduled/graph/struct.Hydroflow.html#method.run_tick), [`run_stratum()`](https://hydro-project.github.io/hydroflow/doc/hydroflow/scheduled/graph/struct.Hydroflow.html#method.run_stratum), [`run()`](https://hydro-project.github.io/hydroflow/doc/hydroflow/scheduled/graph/struct.Hydroflow.html#method.run), and [`run_async()`](https://hydro-project.github.io/hydroflow/doc/hydroflow/scheduled/graph/struct.Hydroflow.html#method.run_async) methods provide other ways to control the graph execution.
    - Also `run_available()` `next_stratum()` and `recv_events` are important

- **Make sure `src/examples/echoserver` is the same as the template project** -- or better, find a way to do that via github actions or a github submodule

## What's covered in examples
- Concepts covered
    - cargo generate for templating
    - Hydroflow program specs embedded in Rust
    - Tokio Channels and how to use them in Hydroflow
        - Network sources and sinks (source_stream)
        - Built-in serde (source_stream_serde, dest_sink_serde)
    - Hydroflow syntax: operators, ->, variables, indexing multi-input/output operators
    - running Hydroflow via `run_available` and `run_async`
    - Recursion via cyclic dataflow
    - Fixpoints and Strata
    - Template structure: clap, message types
    - source_stdin
    - Messages and `demux`
    - broadcast pattern
    - gated buffer pattern
    - bootstrapping pipelines

- Operators covered
    - cross_join
    - demux
    - dest_sink_serde
    - difference
    - filter
    - filter_map
    - flatten
    - flat_map
    - for_each
    - join
    - map
    - merge
    - source_iter
    - source_stdin
    - source_stream
    - source_stream_serde
    - tee
    - unique