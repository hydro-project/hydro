---
sidebar_position: 2
---

# Custom Proof Scripts
Verus checks proof obligations with an SMT solver. Arithmetic is usually handled automatically, as are comparisons and most control flow, but some properties need a hint. Bitwise reasoning, for example, needs Verus's `bit_vector` mode.

Every `verus_proof_commutative_*!` macro accepts an optional `proof = |state, x, y| { ... }` clause containing a **proof script**. The script has ghost bindings for the initial state (the accumulator, or the `captures_mut` value) and the two items, and can use any Verus [proof constructs](https://verus-lang.github.io/verus/guide/proof_functions.html), such as `assert(...)`, `assert(...) by (...)`, and calls to lemmas:

```rust,no_run
# use hydro_lang::prelude::*;
# use hydro_lang::live_collections::stream::NoOrder;
# let mut flow = FlowBuilder::new();
# let process = flow.process::<()>();
let flags = process
    .source_iter(q!(0..1u32))
    .fold(q!(|| 0u32), q!(|acc: &mut u32, x| *acc |= x));
let flags_mut = flags.by_mut();

process
    .source_iter(q!(vec![1u32, 2, 4]))
    .weaken_ordering::<NoOrder>()
    .for_each(q!(
        |x| {
            *flags_mut |= x;
        },
        commutative = verus_proof_commutative_effect!(
            item = u32,
            captures_mut = |flags_mut: u32|,
            proof = |s, x, y| {
                // A wrong hint fails verification; it can never make a
                // non-commutative closure pass (only `assume(...)` could).
                assert(((s | x) | y) == ((s | y) | x)) by (bit_vector);
            }
        )
    ));
```

Without the script, the solver cannot show on its own that bitwise OR commutes, and verification fails. The `by (bit_vector)` assertion proves that fact in bit-vector mode, and the solver then uses it to discharge the obligation.

## Proof Scripts Cannot Weaken the Guarantee
A proof script **does not change what is proven**. Hydro always generates the obligation itself, from the closure body. The script runs before the obligation's final assertion, as extra steps that Verus must also check. So even a wrong script cannot break the guarantee: a hint that is false, or that proves something irrelevant, makes verification fail or simply doesn't help, but it can never make a non-commutative closure verify. You can experiment with proof scripts freely without risking soundness.

## `assume` Is the Exception
`assume(...)` is Verus's explicit escape hatch. Verus accepts an `assume` without checking it, so a false `assume` *can* make a non-commutative closure verify. An `assume` in a proof script is an unchecked claim, just like a [`manual_proof!`](../proof-obligations.md#manual-proofs), and should get the same scrutiny in review.

If you find yourself reaching for `assume` to rule out a panic, use a [`verus_panic!` guard](./panics.md) instead: the guard is checked at runtime, so it stays sound.
