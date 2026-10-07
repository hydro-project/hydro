---
sidebar_position: 0
---

# Verifying with Verus
[Verus](https://verus-lang.github.io/verus/guide/) is a verifier for Rust code. Hydro uses it to machine-check **commutativity** [proof obligations](../proof-obligations.md), so you don't have to rely on a written `manual_proof!`.

The `verus_proof_commutative_*!` macros generate a proof obligation from the **actual closure body**. The obligation runs the body on two arbitrary inputs `x` and `y` in both orders (`x` then `y`, and `y` then `x`), starting from the same arbitrary state, and requires the observable results to be equal. You never write the obligation yourself, so you cannot get it wrong or weaken it; you only declare the types involved.

Under normal compilation (`cargo build`, `cargo test`), the proof is erased: the macro expands to a marker value, with no runtime cost and no dependency on Verus. The proof is only checked when the crate is built with the Verus driver (`cargo verus verify`).

## Setting Up a Crate
1. Install [Verus](https://github.com/verus-lang/verus/releases) and the Rust toolchain it pins, and make sure `cargo-verus` is on your `PATH`.
2. In your crate's `Cargo.toml`, opt in to verification, and add a `verus` feature that enables `vstd` (Verus's standard library) and `hydro_lang`'s own `verus` feature. Pin `vstd` to the exact version matching your installed Verus release; `hydro_lang` accepts any `vstd` from `0.0.0-2026-08-02-0125` onward, so your pin decides which version is used:

   ```toml
   [package.metadata.verus]
   verify = true

   [features]
   verus = ["dep:vstd", "hydro_lang/verus"]

   [dependencies]
   vstd = { version = "=0.0.0-2026-09-06-0133", optional = true }
   ```

   Under a normal build, neither feature does anything, and without them `vstd` is not part of the dependency graph at all. The `hydro_lang/verus` feature is **required** whenever you run `cargo verus`, because Verus also processes `hydro_lang` (to read the specifications it exports, such as [`verus_panic!`](./panics.md)). If you forget it, compiling `hydro_lang` fails with an error saying that "the `verus_builtin` crate was not imported".
3. Verify the crate:

   ```bash
   cargo verus verify -p my_crate --features verus
   ```

   Pass `--tests` as well to check proofs in `#[cfg(test)]` code. `cargo-verus` requires Verus flags such as `--features` to come before other flags.

A failed obligation is reported at the annotation, with the specific failing assertion inside the closure body (for example, the line that may overflow). We recommend running `cargo verus verify` in CI next to your regular tests, so that a change to a closure body cannot silently break its proof.

## Choosing a Macro
There is one macro per closure shape. Each one checks the definition of commutativity that fits that shape:

| Macro | Closure shape | Used with | Must be order-independent |
|---|---|---|---|
| [`verus_proof_commutative_fold!`](rust:hydro_lang::properties::verus_proof_commutative_fold) | `\|acc: &mut A, item: T\|` | `fold`, `reduce` | the final accumulator |
| [`verus_proof_commutative_map!`](rust:hydro_lang::properties::verus_proof_commutative_map) | `\|item: T\| -> U`, mutating captured state | `map` | the final captured state **and** the multiset of outputs |
| [`verus_proof_commutative_filter!`](rust:hydro_lang::properties::verus_proof_commutative_filter) | `\|item: &T\| -> bool`, mutating captured state | `filter` | the final captured state **and** the multiset of retained elements |
| [`verus_proof_commutative_effect!`](rust:hydro_lang::properties::verus_proof_commutative_effect) | `\|item: T\|` returning `()`, mutating captured state | `for_each`, `inspect` | the final captured state |

### Aggregations
For `fold` and `reduce`, declare the accumulator and item types:

```rust,no_run
# use hydro_lang::prelude::*;
# use hydro_lang::live_collections::stream::NoOrder;
# let mut flow = FlowBuilder::new();
# let process = flow.process::<()>();
# let numbers = process.source_iter(q!(vec![1usize, 2, 3])).weaken_ordering::<NoOrder>();
let largest = numbers.reduce(q!(
    |curr, new| {
        if new > *curr {
            *curr = new;
        }
    },
    commutative = verus_proof_commutative_fold!(acc = usize, item = usize)
));
```

Verus proves this automatically: whichever order the two elements arrive in, the accumulator ends up holding the larger one. Swap in a non-commutative body, such as `*curr = new` (last writer wins), and verification fails.

### Closures That Mutate State
When a `map`, `filter`, `for_each`, or `inspect` closure mutates a [`by_mut()`](../../state-management/references-mutations.md) reference on an unordered stream, the mutation must commute. Declare the mutable capture, using the type *behind* the reference, in a `captures_mut = |name: Type|` clause:

```rust,no_run
# use hydro_lang::prelude::*;
# use hydro_lang::live_collections::stream::NoOrder;
# let mut flow = FlowBuilder::new();
# let process = flow.process::<()>();
let count = process
    .source_iter(q!(0..5u32))
    .fold(q!(|| 0u32), q!(|acc: &mut u32, _x| *acc = acc.wrapping_add(1)));
let count_mut = count.by_mut();

let numbered = process
    .source_iter(q!(vec![10u32, 20, 30]))
    .weaken_ordering::<NoOrder>()
    .map(q!(
        |_x| {
            *count_mut = count_mut.wrapping_add(1);
            *count_mut
        },
        commutative = verus_proof_commutative_map!(item = u32, captures_mut = |count_mut: u32|)
    ));
```

For `map` and `filter`, commuting state updates are **not enough**, because the closure's output is also observable. The example above is accepted because the *set* of emitted counts (`s + 1` and `s + 2`) is the same in either order. But a `map` that returns a running total (`*total += x; *total`) emits values that reveal the processing order, so `verus_proof_commutative_map!` rejects it, even though its state update commutes. Likewise, a rate-limiting `filter` always uses up its budget the same way, but *which* element gets through depends on the order, so `verus_proof_commutative_filter!` rejects it. Hydro's generated obligations check the outputs too, so you don't have to reason about this case by hand.

The `item = ...` type must match exactly what the closure receives. `filter` and `inspect` receive a reference, so the type is written as, for example, `item = &u32`.

## Read-Only Captures
If the closure reads other variables from its environment, declare them, with their types, in a `captures = |...|` clause. The obligation is then proven for **every possible value** of those captures:

```rust,no_run
# use hydro_lang::prelude::*;
# use hydro_lang::live_collections::stream::NoOrder;
# let mut flow = FlowBuilder::new();
# let process = flow.process::<()>();
# let numbers = process.source_iter(q!(vec![1usize, 2, 3])).weaken_ordering::<NoOrder>();
let cap = 100usize;
let capped_max = numbers.reduce(q!(
    move |curr, new| {
        if new > *curr && new <= cap {
            *curr = new;
        }
    },
    commutative = verus_proof_commutative_fold!(
        acc = usize,
        item = usize,
        captures = |cap: usize|
    )
));
```

You cannot leave a capture out by accident: `q!` passes the closure's actual capture list to the macro, and an undeclared capture is a compile error, even under a normal `cargo build`.

## Types in Obligations
The obligation mentions the accumulator, item, and capture types, so Verus must be able to reason about them. Primitive types, tuples, and the standard library types that Verus supports (such as `Option`) work as they are. Your own structs and enums must be declared inside a [`verus!`](https://verus-lang.github.io/verus/guide/) block, which compiles to ordinary Rust under a normal build:

```rust,ignore
vstd::prelude::verus! {
    pub struct Totals {
        pub count: u64,
        pub sum: u64,
    }
}
```

The types do not need to implement `Copy` or `Clone`.

## Next Steps
- [Handling Panics](./panics.md): Verus also proves that the closure cannot panic, so closures with overflowing arithmetic (like `*acc += x`) need a small change.
- [Custom Proof Scripts](./proof-scripts.md): some properties, such as bitwise ones, need a hint for the solver.
