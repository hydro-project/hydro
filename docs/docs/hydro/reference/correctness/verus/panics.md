---
sidebar_position: 1
---

# Handling Panics
A Verus proof establishes more than commutativity: Verus also proves that the closure body **can never panic**, for any input or state. That covers arithmetic overflow, division by zero, out-of-bounds indexing, `unwrap()` on `None`, and explicit `panic!` / `assert!` calls; each one is an obligation to prove it is unreachable. You cannot work around this with `std::panic::catch_unwind`, which Verus does not support.

As a result, some closures are rejected even though they are commutative in every run that finishes. The most common example is plain integer addition:

```rust,ignore
q!(
    |acc, x| *acc += x, // rejected: `*acc += x` may overflow
    commutative = verus_proof_commutative_fold!(acc = u32, item = u32)
)
```

Addition is commutative, but `*acc += x` panics on overflow (in debug builds), and Verus reports `possible arithmetic underflow/overflow`. That is stricter than Hydro needs: if a closure panics, the process crashes, so only runs that complete without panicking have to agree.

## `verus_panic!`
To tell Verus that panicking is acceptable, use the [`verus_panic!`](rust:hydro_lang::properties::verus_panic) macro instead of an implicit panic. It takes the same arguments as `panic!`, and Verus knows that execution does not continue past it, so the property only has to hold on paths that don't reach it. The usual pattern is a guard right before the line that could panic:

```rust
# use hydro_lang::prelude::*;
# use hydro_lang::live_collections::stream::NoOrder;
# use futures::StreamExt;
# tokio_test::block_on(hydro_lang::test_util::stream_transform_test(|process| {
# let numbers = process.source_iter(q!(vec![1u32, 2, 3])).weaken_ordering::<NoOrder>();
numbers.fold(
    q!(|| 0u32),
    q!(
        |acc, x| {
            if *acc > u32::MAX - x {
                verus_panic!("sum overflowed");
            }
            *acc += x; // accepted: Verus knows this cannot overflow here
        },
        commutative = verus_proof_commutative_fold!(acc = u32, item = u32)
    ),
)
# .into_stream()
# }, |mut stream| async move {
# assert_eq!(stream.next().await.unwrap(), 6);
# }));
```

This keeps the panicking behavior while still proving commutativity, and you cannot get it wrong in a way that breaks the guarantee:
- the guard really runs, and panics, at runtime, so a guard that is **stricter** than necessary is still correct (it just panics more often)
- a guard that is **too weak** (one that misses an overflowing case) is caught by Verus, which reports the line that can still overflow
- `verus_panic!` only removes the paths that panic, so a closure that is **not commutative** on the paths that continue is still rejected

The `verus_panic!` specification comes from `hydro_lang` itself, so it requires `hydro_lang`'s `verus` feature when verifying (see [Setting Up a Crate](./index.md#setting-up-a-crate)). Under a normal build, `verus_panic!` is an ordinary `panic!`.

## Avoiding Panics
If you'd rather not panic, rewrite the closure so that it cannot panic:
- **wrapping arithmetic**: `*acc = acc.wrapping_add(x)`. Wrapping addition is commutative, and this matches what `+=` already does in release builds, where overflow checks are off by default.
- **saturating arithmetic** (unsigned types): `*acc = acc.saturating_add(x)` caps the result at the maximum value, and is still commutative for unsigned integers.
- **explicit handling**: use `checked_add` and decide what an overflow should mean, for example by recording a sticky overflow flag in the accumulator.

## Unchecked Preconditions
If the closure relies on a precondition that the code doesn't check, such as "the counts never exceed `u32::MAX` in practice", Verus can't prove it. Either check it explicitly with a `verus_panic!` guard, or use a `manual_proof!` that states the assumption.
