# Algebraic Properties

Hydro's type system tracks algebraic properties of operations — such as monotonicity, commutativity, and idempotence — through annotations on `q!` quoted closures. These properties enable the compiler to guarantee correctness of distributed coordination patterns like threshold detection and unordered aggregation.

- **[Monotonicity](./monotonicity.md)**: a value only grows over time, enabling deterministic threshold-based coordination
