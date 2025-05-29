---
sidebar_position: 3
---

# Safety and Correctness
Much like Rust's type system helps ensure memory safety, Hydro helps ensure **distributed safety**. Hydro's type system helps you avoid many kinds of distributed systems bugs, including:
- Non-determinism due to message delays (which affect arrival order), interleaving across streams (which affect order of handling) or retries (which result in duplicates)
  - See [Live Collections / Eventual Determinism](./live-collections/determinism)
- Using mismatched serialization and deserialization formats across services
  - See [Locations and Networking](./locations/index)
- Misusing node identifiers across logically independent clusters of machines
  - See [Locations / Clusters](./locations/clusters)
- Relying on non-determinstic clocks for batching events
  - See [Ticks and Atomicity / Batching and Emitting Streams](./ticks-atomicity/batching-and-emitting)

These safety guarantees are surfaced through the Rust type system, so you can catch these bugs at compile time rather than in production. And when it is necessary to bypass these checks for advanced distributed logic, you can use the same `unsafe` keyword as in Rust as an escape hatch.
