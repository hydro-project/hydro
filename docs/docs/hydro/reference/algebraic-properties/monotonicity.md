---
title: Monotonicity
sidebar_position: 1
---

# Monotonicity in `q!` Quoted Functions

Hydro's type system tracks **monotonicity** — the property that a value only grows over time — through `q!` quoted closures. This enables deterministic threshold-based coordination in distributed programs: detecting when a counter crosses a threshold fires exactly once, regardless of message ordering.

## Overview

A `Singleton` in Hydro has one of three boundedness markers:
- **`Bounded`** — the value is frozen and will never change.
- **`Unbounded`** — the value may change arbitrarily over time.
- **`Monotonic`** — the value only grows (according to `PartialOrd`) over time.

A `KeyedSingleton` has analogous markers including **`MonotonicValue`** (values per key only grow).

The `Monotonic` bound enables APIs like `threshold_greater_or_equal`, which deterministically detects when a value crosses a threshold. This would be non-deterministic on an `Unbounded` singleton, so the type system prevents it.

## Declaring Monotonicity

Monotonicity is declared via property annotations inside `q!()` closures passed to aggregation operators like `fold`:

```rust
stream.fold(
    q!(|| 0),
    q!(
        |acc, v| *acc += v,
        monotone = manual_proof!(/** integer addition with non-negative inputs is monotone */)
    ),
)
```

The `monotone = manual_proof!(/** ... */)` annotation tells the type system that applying the closure to any new input only increases (or maintains) the accumulator. The doc comment is your human-written justification for why this holds.

## Producing a Monotonic Singleton

When you annotate `fold` with a `monotone` proof and the input stream is `Unbounded`, the output singleton is promoted to `Monotonic`:

```rust
// Input: Stream<i32, Process, Unbounded>
// Output: Singleton<usize, _, Monotonic>
let count: Singleton<usize, _, Monotonic> = unbounded_stream.fold(
    q!(|| 0usize),
    q!(|acc, _| *acc += 1, monotone = manual_proof!(/** += 1 is monotone */)),
);
```

Without the `monotone` annotation, the same `fold` produces `Singleton<..., Unbounded>`.

## Preserving Monotonicity Through `.map()`

If you have a `Monotonic` singleton and apply `.map()`, the result is demoted to `Unbounded` by default (since the map function could reverse the ordering). To preserve monotonicity, annotate with `order_preserving`:

```rust
let still_monotonic: Singleton<usize, _, Monotonic> = monotonic_singleton.map(
    q!(|x| x * 2, order_preserving = manual_proof!(/** doubling preserves order */)),
);
```

Without `order_preserving`, using the result with `threshold_greater_or_equal` would fail to compile.

## Threshold Detection

Once you have a `Monotonic` singleton (or `Bounded`), you can use threshold APIs:

```rust
let a: Singleton<i32, _, Monotonic> = /* ... */;
let threshold = process.singleton(q!(100));
let crossed: Stream<i32, _, _> = a.threshold_greater_or_equal(threshold);
// Emits 100 exactly once, the first time `a` >= 100
```

If you try this on a non-monotonic singleton:

```rust
let no_longer_monotone = is_monotone.map(q!(|x| -x)); // no order_preserving proof
let _ = no_longer_monotone.threshold_greater_or_equal(threshold);
// ERROR: The input singleton must be monotonic (`Monotonic`) or bounded (`Bounded`)
```

## KeyedSingleton Monotonicity

For `KeyedSingleton`, the `MonotonicValue` bound indicates that the **value** for each key grows monotonically. This enables `threshold_greater_or_equal` and `threshold_greater_or_equal_uniform` on keyed data:

```rust
let counts: KeyedSingleton<u32, usize, _, MonotonicValue> = events.into_keyed()
    .fold(
        q!(|| 0),
        q!(|acc, _| *acc += 1, monotone = manual_proof!(/** +1 is monotone */)),
    );

// Per-key threshold
let thresholds: KeyedSingleton<u32, usize, _, BoundedValue> = /* ... */;
let crossed = counts.threshold_greater_or_equal(thresholds);

// Uniform threshold across all keys
let threshold = process.singleton(q!(5usize));
let crossed = counts.threshold_greater_or_equal_uniform(threshold);
```

## Available Annotations

For aggregation functions (`fold`, `reduce`):

| Annotation | Meaning | When to use |
|---|---|---|
| `commutative = ...` | Order of inputs doesn't matter | Input stream is unordered (`NoOrder`) |
| `idempotent = ...` | Duplicate inputs don't change result | Input stream has retries |
| `monotone = ...` | Accumulator only grows over time | You want `Monotonic` / `MonotonicValue` output |

For map functions on singletons:

| Annotation | Meaning | When to use |
|---|---|---|
| `order_preserving = ...` | If input grows, output also grows | You want to preserve `Monotonic` through `.map()` |

All annotations use `manual_proof!(/** reason */)` — a human attestation that the property holds. In the future, automated verification backends such as [Kani](https://model-checking.github.io/kani/) may be supported.

## Common Patterns

### Counter (always monotone)
```rust
stream.fold(
    q!(|| 0usize),
    q!(|count, _| *count += 1, monotone = manual_proof!(/** += 1 is monotone */)),
)
```

### Sum of non-negative values
```rust
// Only monotone if all inputs are non-negative!
stream.fold(
    q!(|| 0),
    q!(|acc, v| *acc += v, monotone = manual_proof!(/** sum of non-negative values */)),
)
```

### Set size via `.count()`
The built-in `.count()` method already includes a monotone proof internally:
```rust
// Returns Singleton<usize, _, Monotonic> when input is Unbounded
stream.count()
```

## Interaction with the Simulator

When using Hydro's simulator (`flow.sim()`), monotonicity proofs are checked at runtime. The simulator explores different orderings of inputs to verify that annotated properties actually hold, catching bugs where a `manual_proof!` claim is incorrect.

## Summary

| Concept | Where declared | Effect |
|---|---|---|
| Monotone aggregation | `q!(\|acc, v\| ..., monotone = ...)` in `fold` | Output becomes `Monotonic` / `MonotonicValue` |
| Order-preserving map | `q!(\|x\| ..., order_preserving = ...)` in `.map()` | Preserves `Monotonic` through the map |
| Threshold detection | `.threshold_greater_or_equal(...)` | Requires `Monotonic` or `Bounded` input |
| Manual proof | `manual_proof!(/** reason */)` | Human attestation that property holds |
