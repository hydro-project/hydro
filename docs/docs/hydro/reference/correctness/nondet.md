---
sidebar_position: 3
---

# Non-Determinism and `nondet!`
Every **safe** API in Hydro guarantees [eventual determinism](./determinism.md): no matter how messages are delayed or interleaved at runtime, the outputs of your program will eventually settle to the same final value. But real distributed systems often need behavior that is _inherently_ non-deterministic:
- processing an input stream in **batches** whose boundaries depend on arrival timing
- **sampling** state or generating events on a wall-clock timer
- **assuming an order** for messages that arrive from concurrent senders
- tolerating **retries** that may deliver the same message more than once

Hydro does not forbid these behaviors; instead, it makes them impossible to introduce _silently_. Every non-deterministic API takes an extra parameter called a **non-determinism guard** (of type `NonDet`), which can only be created by invoking the `nondet!` macro with a written explanation. This has a powerful consequence: **all non-determinism in a Hydro program originates at a `nondet!` invocation**. If your program misbehaves in a way that depends on timing, batching, or message ordering, the root cause is at one of these explicitly marked points.

## Non-Determinism Guards
A non-determinism guard is passed by invoking `nondet!()` with a doc comment explaining how the non-determinism affects the application:

```rust,no_run
# use hydro_lang::prelude::*;
use std::time::Duration;

fn singleton_with_delay<T, L>(
  singleton: Singleton<T, Process<L>, Unbounded>
) -> Optional<T, Process<L>, Unbounded> {
  singleton
    .sample_every(q!(Duration::from_secs(1)), nondet!(
      /// which intermediate values are sampled is non-deterministic, but `.last()`
      /// eventually resolves to the final value of the input singleton
    ))
    .last()
}
```

The doc comment is **mandatory**; `nondet!` will not compile without one. This is intentional: the explanation is not decoration, it is the reasoning that a reviewer (or your future self) will use to check that the non-determinism is acceptable.

When writing a function that uses non-deterministic APIs, there are two distinct situations, which determine how you should invoke `nondet!`:
1. The non-determinism is **resolved locally**: it cannot be observed in the function's outputs.
2. The non-determinism is **exposed to callers**: the function's outputs may differ across executions.

## Locally-Resolved Non-Determinism
Often, a function uses a non-deterministic API internally, but its outputs are still deterministic. In this case, you invoke `nondet!` with only an explanation, and the function does not need to declare any non-determinism to its callers.

In `singleton_with_delay` above, sampling on a timer is non-deterministic, but the `.last()` at the end means the output _eventually_ settles to the input's final value, restoring eventual determinism.

Another common pattern is tolerating retries by making downstream processing insensitive to duplicates. Consider counting the number of unique requests in a stream that may contain retried deliveries:

```rust,no_run
# use hydro_lang::prelude::*;
use hydro_lang::live_collections::stream::{AtLeastOnce, ExactlyOnce, TotalOrder};

fn count_unique<'a, L>(
    requests: Stream<u64, Process<'a, L>, Unbounded, TotalOrder, AtLeastOnce>,
) -> Singleton<usize, Process<'a, L>, Unbounded> {
    requests
        .assume_retries::<ExactlyOnce>(nondet!(
            /// retried requests carry the same ID, and we only count each unique ID
            /// once (via `first()`), so duplicate deliveries cannot affect the count
        ))
        .map(q!(|id| (id, ())))
        .into_keyed()
        .first()
        .key_count()
}
```

Both explanations follow the same pattern: they read like a short, informal **line of a proof**. They state _why_ the output remains deterministic, naming the specific mechanism that resolves the non-determinism (`.last()` eventually converging, deduplication via `first()`). If the code changes in a way that invalidates the reasoning — say, someone removes the `first()` — the stale comment gives a reviewer a fighting chance to catch the bug.

## Forwarding Non-Determinism to Callers
If the outputs of your function _are_ affected by non-determinism, you must surface this in the function's signature by taking parameters of type `NonDet`. By convention, these parameters are named with a `nondet_` prefix, and each one is documented in a `# Non-Determinism` section of the function's Rustdoc.

Inside the function, when you invoke a non-deterministic API whose effects propagate to the outputs, you pass `nondet!` an explanation **plus** the guard parameter(s) that capture the effect:

```rust
# use hydro_lang::prelude::*;
use hydro_lang::live_collections::stream::ExactlyOnce;

use std::fmt::Debug;
use std::time::Duration;

/// Prints a sample of elements from the stream, at most one per second.
///
/// # Non-Determinism
/// - `nondet_samples`: this function will non-deterministically print elements
///   from the stream according to a timer
fn print_samples<T: Debug, L>(
  stream: Stream<T, Process<L>, Unbounded>,
  nondet_samples: NonDet
) {
  stream
    .sample_every(q!(Duration::from_secs(1)), nondet!(
      /// non-deterministic timing means arbitrary elements are picked for printing,
      /// which is captured by the caller's declared sampling non-determinism
      nondet_samples
    ))
    .assume_retries::<ExactlyOnce>(nondet!(
      /// the same element may be sampled and printed more than once, which is
      /// still within the declared "samples are arbitrary" non-determinism
      nondet_samples
    ))
    .for_each(q!(|v| println!("Sample: {:?}", v)))
}
```

Forwarding a guard is _also_ a proof obligation, just in the other direction: the explanation should state how the inner non-determinism **maps onto** the form of non-determinism the parameter declares. Here, both the timer and the potential duplicates fold into the single caller-visible fact "which elements get printed is arbitrary", so both call sites forward `nondet_samples`.

This structure is compositional. Your caller now faces the same choice you did: it can resolve `nondet_samples` locally (if the printing is unobservable to _its_ callers), or forward it into its own `nondet_` parameters.

### Discharging Guards at the Application Root
At the top level of an application, there is no caller left to forward to. There, you discharge guards with an explanation that references the **service-level guarantees** — the documented, user-visible contract of the system:

```rust,ignore
print_samples(
    requests.clone(),
    nondet!(
        /// sampled logging is best-effort and explicitly excluded from the
        /// service contract; log content may vary across runs
    ),
);
```

A well-factored Hydro application thus has a clean "chain of custody" for every source of non-determinism: from the API that introduces it, through `nondet_` parameters that name it, to a root-level explanation of why the overall service tolerates it.

## Multiple Sources of Non-Determinism
A function should take **one `NonDet` parameter for each independently-observable form of non-determinism** in its outputs. This lets callers reason about — and discharge — each form separately.

The Paxos implementation in `hydro_test` is a good example. Its core entrypoint declares two distinct forms of non-determinism, corresponding to its two outputs:

```rust,ignore
/// Implements the core Paxos algorithm, which uses a cluster of proposers and acceptors
/// to sequence payloads being sent to the proposers.
///
/// # Non-Determinism
/// When the leader is stable, the algorithm will commit incoming payloads to the leader
/// in deterministic order. However, when the leader is changing, payloads may be
/// non-deterministically dropped. The stream of ballots is also non-deterministic because
/// leaders are elected in a non-deterministic process.
pub fn paxos_core<'a, P: PaxosPayload>(
    proposers: &Cluster<'a, Proposer>,
    acceptors: &Cluster<'a, Acceptor>,
    // ...
    nondet_leader: NonDet,
    nondet_commit: NonDet,
) -> (
    Stream<Ballot, Cluster<'a, Proposer>, Unbounded>,
    Stream<(usize, Option<P>), Cluster<'a, Proposer>, Unbounded, NoOrder>,
) {
    // ...
}
```

- `nondet_leader` captures _which proposer becomes the leader_ (affecting the emitted ballots)
- `nondet_commit` captures _which in-flight payloads may be dropped during a leader change_

These are genuinely independent: a caller building a replicated log might discharge `nondet_commit` by having clients retry and deduplicate dropped payloads, while `nondet_leader` remains visible in its own outputs (e.g., which node clients should contact). Collapsing both into one parameter would force callers to reason about them monolithically.

Internally, `paxos_core` forwards each guard at the call sites whose non-determinism it captures:

```rust,ignore
nondet!(
    /// The primary non-determinism exposed by leader election lies in which leader
    /// is elected, which affects both the ballot at each proposer and the leader flag.
    /// But using a stale ballot or leader flag will only lead to failure in sequencing
    /// rather than committing the wrong value.
    nondet_leader
)
```

Note how the explanation does double duty: it maps the inner non-determinism (stale ballots) onto the declared parameter (leader choice), **and** it records the invariant that limits the blast radius (a stale ballot can fail to sequence, but can never commit a wrong value).

## Writing Good Explanations
The explanation inside `nondet!` should be a short piece of an informal proof, naming the invariants it relies on. A useful test: could a reviewer, reading only the comment and the surrounding code, check whether the claim actually holds?

Compare:

```rust,ignore
// ❌ restates that non-determinism exists, but justifies nothing
let batch = use(requests, nondet!(/** batching is non-deterministic, that's fine */));

// ✅ states why the result is insensitive to the non-deterministic choice
let batch = use(requests, nondet!(
    /// each request is answered using only its own contents and the current
    /// snapshot, so batch boundaries are never observable in the responses
));
```

Guidelines for writing these explanations:
- **Locally-resolved guards**: state why the outputs remain (eventually) deterministic. Name the mechanism: deduplication, commutativity of an aggregation, convergence via `.last()`, quorum overlap, monotonicity, etc.
- **Forwarded guards**: state how the inner non-determinism is captured by the declared parameter, and any invariants that bound its effects.
- **Root-level guards**: reference the service-level contract that tolerates the behavior.
- If the justification depends on an invariant maintained elsewhere (e.g., "payloads are always logged before being acknowledged"), _name that invariant_ so reviewers know what else to check.

During code review, `nondet!` invocations are precisely the places that deserve extra scrutiny — everything else is guaranteed deterministic by the type system. Reviewers should treat each explanation as a claim to be verified, not a formality.

## Testing Non-Deterministic Code
Explanations are informal proofs, and informal proofs can be wrong. The [Hydro simulator](../simulation/index.mdx) mechanically explores the non-deterministic choices your program admits — batch boundaries, snapshot timing, message orderings — precisely at the points marked by `nondet!`. Writing [exhaustive simulation tests](../simulation/writing.mdx) for code that involves non-determinism guards lets you check that your outputs are insensitive to those choices (or vary only in the ways you claimed).

## Non-Determinism in User-Defined Functions
Another source of potential non-determinism comes from user-defined functions or closures, such as those provided to `map` or `filter`. Hydro allows arbitrary Rust code inside these closures, so it is possible to introduce non-determinism (random number generators, wall-clock time, thread IDs, iteration over hash maps) that will **not** be checked by the compiler or marked by a guard.

In general, avoid such APIs inside transformation functions unless the non-determinism is explicitly documented somewhere, following the same conventions as `nondet!` explanations.

:::info

To help avoid such bugs, we are working on ways to use formal verification tools (such as [Kani](https://model-checking.github.io/kani/)) to check arbitrary Rust code for properties such as determinism and more. This remains active research for now and is not yet available.

:::
