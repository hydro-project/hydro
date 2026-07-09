---
sidebar_position: 0
---

# Safety and Correctness
Much like Rust's type system helps ensure memory safety, Hydro helps ensure **distributed safety**. Hydro's type system helps you avoid many kinds of distributed systems bugs, including:
- Non-determinism due to message delays (which affect arrival order), interleaving across streams (which affect order of handling) or retries (which result in duplicates)
  - See [Eventual Determinism](./determinism.md)
- Observing a collection that is still asynchronously changing as if it were a final result
  - See [Bounded and Unbounded Types](./bounded-unbounded.md)
- Using mismatched serialization and deserialization formats across services
  - See [Locations and Networking](../locations/index.md)
- Misusing node identifiers across logically independent clusters of machines
  - See [Locations / Clusters](../locations/clusters.md)
- Relying on non-deterministic clocks for batching events
  - See [State Management / Slice Blocks](../state-management/slices.mdx)

These safety guarantees are surfaced through the Rust type system, so you can catch these bugs at compile time rather than in production. And when it is necessary to bypass these checks for advanced distributed logic, Hydro requires you to attach [non-determinism guards](./nondet.md) that explain the effects of the non-determinism, clearly marking the code that should be carefully reviewed.
