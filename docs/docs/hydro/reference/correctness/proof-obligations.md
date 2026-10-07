---
sidebar_position: 4
---

# Proof Obligations
Some Hydro APIs are only deterministic if the closure you give them has a certain **algebraic property**. For example, a `fold` over a `NoOrder` stream gives a deterministic result only if the closure is **commutative**, meaning the final accumulator does not depend on the order in which elements arrive. In the same way, a closure over an `AtLeastOnce` stream must be **idempotent** so that duplicates have no effect.

Hydro does not take these properties on faith. When the input stream has weaker guarantees, the API requires a **property annotation** inside `q!(...)`, next to the closure, and the code does not compile without one:

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

Every annotation needs a **proof**. A proof is evidence that the property holds. Hydro accepts two kinds:
- a **manual proof** ([`manual_proof!`](rust:hydro_lang::properties::manual_proof)): a written justification that a human reviewer checks
- a **Verus proof** (`verus_proof_commutative_*!`): a machine-checked proof that [Verus](https://verus-lang.github.io/verus/guide/) verifies against the closure body

The table below lists the properties and which proofs each one accepts:

| Property | Annotation | Required when | Accepted proofs |
|---|---|---|---|
| Commutativity | `commutative = ...` | the input is `NoOrder` | `manual_proof!`, `verus_proof_commutative_*!` |
| Idempotence | `idempotent = ...` | the input is `AtLeastOnce` | `manual_proof!` |
| Monotonicity | `monotone = ...` | opting in to a `Monotonic` result | `manual_proof!` |
| Order preservation | `order_preserving = ...` | keeping a singleton `Monotonic` through a `map` | `manual_proof!` |

The rules for when annotations are needed are covered in [Streams](../streaming-data/streams.md), [Keyed Streams](../streaming-data/keyed-streams.mdx), and [References and Mutations](../state-management/references-mutations.md). This page gives an overview of the two kinds of proof.

:::note

Neither kind of proof is trusted by the [Hydro simulator](../simulation/index.mdx). Even when a closure is annotated as commutative, the simulator still explores different element orders, so a [simulation test](../simulation/writing.mdx) can catch a wrong claim. For manual proofs, this is often the only mechanical check.

:::

## Manual Proofs
A manual proof is created with the [`manual_proof!`](rust:hydro_lang::properties::manual_proof) macro, which takes a doc comment explaining *why* the property holds:

```rust,no_run
# use hydro_lang::prelude::*;
# use hydro_lang::live_collections::stream::{AtLeastOnce, NoOrder};
# let mut flow = FlowBuilder::new();
# let process = flow.process::<()>();
# let checks = process.source_iter(q!(vec![true, false])).weaken_ordering::<NoOrder>().weaken_retries::<AtLeastOnce>();
let any_failed = checks.fold(
    q!(|| false),
    q!(
        |acc, failed| *acc |= failed,
        commutative = manual_proof!(/** boolean OR is commutative */),
        idempotent = manual_proof!(/** boolean OR is idempotent */)
    ),
);
```

Manual proofs work for every property and every closure, but **you are responsible for their correctness**. The compiler does not check the explanation, so a wrong claim (for example, calling a last-writer-wins update "commutative") quietly brings back the non-determinism the annotation was meant to rule out. Treat `manual_proof!` invocations the same way you treat [`nondet!` guards](./nondet.md): write a specific justification, and check it carefully in code review.

Use a Verus proof wherever one applies (currently commutativity), and keep manual proofs for the remaining cases.

## Verus Proofs
[Verus](https://verus-lang.github.io/verus/guide/) is a verifier for Rust code. For commutativity, the `verus_proof_commutative_*!` macros generate a proof obligation from the **actual closure body**, and Verus checks it: running the body on any two inputs, in either order, from the same starting state, must produce the same observable result. You only declare the types involved; you never write the obligation, so you cannot get it wrong or weaken it.

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

Under a normal build, the proof is erased, with no runtime cost and no dependency on Verus. It is only checked when you run `cargo verus verify`. Verus also proves that the closure cannot panic, which affects how some closures (such as overflowing arithmetic) have to be written.

See [Verus Proofs](./verus/index.md) for the details:
- [Verifying with Verus](./verus/index.md): setting up a crate, the macros for each closure shape, and declaring captures
- [Handling Panics](./verus/panics.md): why Verus requires panic-free closures, and how to use `verus_panic!`
- [Custom Proof Scripts](./verus/proof-scripts.md): giving the solver hints when it cannot prove a property on its own
