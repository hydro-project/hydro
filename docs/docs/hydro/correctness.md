---
sidebar_position: 3
---

# Safety and Correctness
Just like Rust's type system helps you avoid memory safety bugs, Hydro helps you ensure **distributed safety**. Hydro's type systems helps you avoid many kinds of distributed systems bugs, including:
- Non-determinism due to message delays (which reorder arrival) or retries (which result in duplicates)
  - See [Live Collections / Eventual Determinism](./live-collections/determinism.md)
- Using mismatched serialization and deserialization formats across services
  - See [Locations and Networking](./locations/index.md)
- Misusing node identifiers across logically independent clusters of machines
  - See [Locations / Clusters](./locations/clusters.md)
- Relying on non-determinstic clocks for batching events
  - See [Ticks and Atomicity / Batching and Emitting Streams](./ticks-atomicity/batching-and-emitting.md)

These safety guarantees are surfaced through the Rust type system, so you can catch these bugs at compile time rather than in production. And when it is necessary to bypass these checks for advanced distributed logic, you can use the same `unsafe` keyword as in Rust as an escape hatch.
