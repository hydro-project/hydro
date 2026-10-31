# Denotational Reasoning for Liveness

This design proposes two new constructs for Hydro: a **drop type** on streams (`NoDrop` / `MayDrop`, with `MayDrop` split into `MayDropSuffix` and `MayDropAnywhere` on ordered streams), which records in the type that a stream may be missing elements the network lost, and a **liveness guard** (`NonLive`, written `nonlive!(...)`), which records at a program point why the program is still acceptable even though some output there may never materialize. Together they
- let developers reason about liveness the way Hydro already lets them reason about safety — locally, against a deterministic denotation, with every liveness concern explained where it arises
- give an exact account of which operators remain safe over a lossy stream, including a precise answer to "when is `fold` safe?"
- place liveness concerns at module boundaries, so that a library such as `collect_quorum` states a conditional guarantee and its callers (such as Paxos) explain why the condition holds for them
- line up with the Flo and Gyatso semantics, where they correspond to a refinement of stream types and a decomposition of Gyatso's global liveness assumption

The proposal accepts breaking changes to the networking and stream APIs.

## Background: Determinism and the Ideal Stream
Hydro's correctness story today is about **determinism**. The `NonDet` documentation states the promise:

> All **safe** APIs in Hydro guarantee determinism, even in the face of networking delays and concurrency across machines.

Where a program must observe something nondeterministic — the boundary of a `batch`, the version of a singleton a `snapshot` sees, the order realized by `assume_ordering` — it passes a `nondet!(...)` guard whose doc comment explains how the nondeterminism affects the application and why the application is still correct in spite of it. Components that contain such observations take a `NonDet` parameter, document it under a `# Non-Determinism` heading, and thread it to the operators they control.

Two ideas from the Gyatso semantics make this promise precise, and we will use them throughout. First, every stream has an **ideal**: the collection it would contain if every machine stayed live and every message arrived. The ideal is deterministic. Nondeterminism in *delivery order* is absorbed by the stream's collection type — an unordered stream's ideal is a multiset, an ordered stream's ideal is a sequence — so two executions that deliver the same elements in different orders have the same `NoOrder` ideal. Second, at any moment an execution has only **realized** part of the ideal, and the realization sits below the ideal in the collection's **natural partial order**, written `⊑`: a *prefix* of an ordered stream, a *sub-multiset* of an unordered one.

Hydro's notion of a safe operator follows from this picture. Write `I` for an operator's ideal input and `r ⊑ I` for a realization of it. An operator is **safe** when its output on any realization of its input is a realization of its output on the ideal:

> if `r ⊑ I` then `op(r) ⊑ op(I)`,

where each `⊑` is the natural order of the collection on its own side — the input's on the left, the output's on the right. This is Gyatso's "monotone outputs" property, stated one operator at a time. `map` and `filter` are safe because they send prefixes to prefixes and sub-multisets to sub-multisets.

A fold needs one more idea. The ideal of a fold's singleton is not a single value but a **set of trajectories**: for each ordering of the input that the fold admits, the sequence of values the fold passes through along that ordering, ending at the final value. A commutative fold admits every ordering of its input multiset, so its ideal has one trajectory per ordering; a non-commutative fold over an ordered stream admits exactly one. A *realization* of the singleton is a prefix of one of its trajectories, and a `snapshot` reads the last value of such a prefix. A fold is safe because the fold of a prefix of its input traces a prefix of a trajectory of the fold of the ideal.

An operator is **nondeterministic** when its ideal output is not a function of its ideal input: `batch`, because the ideal does not say where the batch boundaries fall; `snapshot`, because the ideal does not say how far along a trajectory the singleton currently is. Those are the operators that take guards.

What this story leaves out is **liveness**: whether a realization ever *reaches* the ideal — whether a stream's realization grows to the whole ideal, or a singleton's realization follows a trajectory to its end. Gyatso proves eventual determinism under one global assumption — "all machines are live and all network messages are eventually delivered" — and Hydro inherits that assumption silently. The rest of this document is about making it explicit, and we will get there by taking a lossy network seriously.

## Intuition: Events over a Lossy Channel

### The reliable version
Consider the smallest program that crosses a network: one process emits numbered events, another receives them, forwards their payloads, and checks that the numbers arrive consecutively.

```rust
let events: Stream<(u64, Payload), Process<Source>, Unbounded> = ...;

let received = events.send_bincode(sink, TCP.fail_stop()); // Stream<(u64, Payload), Process<Sink>, Unbounded, TotalOrder>

let payloads = received.clone().map(q!(|(_, payload)| payload));

let checked = received.fold(q!(|| 0u64), q!(|expected, (seq, _)| {
    assert_eq!(seq, *expected, "gap in the sequence");
    *expected += 1;
}));
```

The channel preserves order, so the assertion can never fire: TCP delivers a prefix of what was sent, and every prefix of `[0, 1, 2]` starts at `0` and has no gaps.

### Making the channel lossy
Now suppose the network drops packets, and we switch to the lossy policy:

```rust
let received = events.send_bincode(sink, TCP.lossy(nondet!(/** ??? */)));
```

The policy preserves `TotalOrder` among the messages that survive, and it demands a `nondet!` at the channel. Before deciding what to write in it, run the program. The simulator finds a failure immediately:

```
thread 'test_sink' panicked at 'gap in the sequence: 2 != 1'
Simulation Steps:
- `TCP.lossy(...)` source → sink: deliver (0, _), drop (1, _), deliver (2, _)
```

The sink received `[0, 2]`. That is a *subsequence* of the ideal `[0, 1, 2]`, but it is not a *prefix*, so it is not below the ideal in the natural order of sequences — it is not a realization of the ideal at all. The fold was specified for realizations: its output on any prefix of its input traces a prefix of its one trajectory, `0, 1, 2, 3`. It was handed something else, and its output on `[0, 2]` is a panic, which lies on no trajectory. The safety property `op(r) ⊑ op(I)` has no claim to make, because its premise `r ⊑ I` is false. That is why `lossy` demands a `nondet!`: the channel can hand a downstream operator an input it was never specified for.

But look at the other consumer. `payloads` is a `map`, and `map` of `[0, 2]`'s payloads is a subsequence of `map` of the ideal's payloads. Nothing about `map` cares whether its input is a prefix or a subsequence; it reproduces whatever shape it is given. The `map` is fine. Only the fold has a problem — yet the guard sits on the channel, upstream of both, so its justification must speak for the `map` as well, and for every other consumer anyone ever attaches. The nondeterminism is being demanded **too early**: at the channel, where nothing has gone wrong yet, instead of at the fold, where something does.

### Deferring the nondeterminism: `MayDrop`
What the channel actually did was change the relationship between realizations and the ideal. Over `fail_stop`, a realization of an ordered stream is a prefix of the ideal. Over `lossy`, it is a *subsequence*. If the type of the stream said so, the channel would be a safe operator again — its output would be a realization of its ideal, under the stream's own definition of realization — and the operators downstream could be judged one at a time against that definition. We write the changed definition as a sixth type parameter on streams:

```rust
let received = events.send_bincode(sink, TCP.lossy()); // Stream<(u64, Payload), Process<Sink>, Unbounded, TotalOrder, ExactlyOnce, MayDropAnywhere>
```

No guard. **`MayDropAnywhere` changes the natural order of the stream.** For an ordered stream it makes `r ⊑ I` mean "`r` is a subsequence of `I`". The denotation is unchanged — the ideal of a lossy stream is still the stream the source sent — and the nondeterminism has been *deferred*: nothing has been observed yet, so nothing needs justifying yet. (The name says where elements may be missing from; we will meet its sibling, `MayDropSuffix`, shortly.)

With the natural order changed, the safety property decides which operators may consume the stream, exactly as before. `map`, `filter`, `filter_map`, `flat_map_*`, `merge_unordered`, `into_keyed`, `values`, and `join` send a subsequence of their input to a subsequence of their output, so they are safe with respect to the new order and their outputs carry the drop type: `payloads` is a `MayDropAnywhere` stream of payloads, which is the truth. The sequence-check fold is *not* safe with respect to the new order — its output on a non-prefix subsequence lies on no trajectory of its output on the ideal — so it is refused, and the type error lands on the operator that would have panicked:

```
error: `fold` requires a `MayDropSuffix` or `NoDrop` stream, but `received` is `MayDropAnywhere`
```

### Doing something useful with an ordered `MayDropAnywhere` stream: `observe_drops`
How does a program that *needs* the fold proceed? It must observe which elements survived. We add an operator for that, named by analogy with `observe_ordering` (the planned name for `assume_ordering`):

```rust
let in_order = received.observe_drops(nondet!(
    /// A gap in the sequence is reported to the operator, not asserted.
)); // Stream<(u64, Payload), Process<Sink>, Unbounded, TotalOrder, ExactlyOnce, NoDrop>
```

The `nondet!` is required for the reason the panic showed. From here on, downstream operators take the survivors as *the* ideal, with prefixes as its realizations — and which sequence the survivors form is not a function of the real ideal. The guard that `lossy` used to demand at the channel has moved to the consumer that actually observes the loss, exactly as `observe_ordering` sits at the consumer that observes an order rather than at the channel that failed to provide one.

### What about unordered streams?
If `TotalOrder` `MayDropAnywhere` is a stream whose realizations are subsequences, what is `NoOrder` with drops? Its realizations are sub-multisets — the same as `NoOrder` `NoDrop`. Dropping an element from a multiset leaves a sub-multiset, which was already a realization, and it makes no difference where in the sequence the drop happened, so on an unordered stream there is only one lossy drop type, written plainly `MayDrop`. Every safe operator is already safe with respect to it, and nothing is refused. Add a counter to our program:

```rust
let count = received.count(); // Singleton<usize, Process<Sink>, Unbounded, ..., MayDrop>
```

`count` is a commutative fold, so its ideal has a trajectory for every ordering of `{0, 1, 2}` — all of them `0, 1, 2, 3`, as it happens, but the point survives for folds whose intermediate values depend on the order. Under the drop, the realized trajectory is `0, 1, 2`: a prefix of the ideal trajectory in which event `1` arrives last. Before the drop the realization was a prefix of the trajectory `0, 1, …` in arrival order; after it, the realization is a prefix of a *different* trajectory in the ideal set, and each later arrival moves it to a trajectory that pushes event `1` further out. At every moment the realization is a prefix of some trajectory — that is what makes `count` safe here — but it never follows any one trajectory to its end. The fold is admitted, and its output carries the drop type, which records exactly this: every value it takes is on some trajectory, and it may never reach the end of any. A singleton has no order to lose, so its drop type is plainly `MayDrop`.

The same observation gives a guard-free alternative to `observe_drops` for programs that do not need the order:

```rust
let unordered = received.delayed_forever(); // Stream<(u64, Payload), Process<Sink>, Unbounded, NoOrder, ExactlyOnce, MayDrop>
```

It observes nothing — it forgets the order, and once the order is gone no operator can tell a drop from a delay. This is the content of today's `lossy_delayed_forever` network policy, which "does not require a `NonDet` annotation because the output is always lower in the partial order than the ideal stream"; as an operator the program applies it where it wants rather than choosing it once at the channel, and the policy becomes redundant.

> [!NOTE]
> `delayed_forever` is not a way around the safety story, and it is not a hole in it. It takes no guard because it *gives up* the order, and giving up the order is what makes the drop invisible: the output is `NoOrder`, so everything downstream is held to the obligations `NoOrder` already imposes — folds must be commutative, `first` and `enumerate` are unavailable, and any observation of an order goes through `observe_ordering(nondet!(...))`. The sequence check cannot be written against it. The developer has not been excused from the safety reasoning; they have been moved to the part of the API where that reasoning is already enforced.

But now there is a puzzle. If `NoOrder` `MayDrop` and `NoOrder` `NoDrop` have the same realizations, what does `observe_drops` do on an unordered stream, and what should it take?

```rust
let delivered = unordered.observe_drops(???); // Stream<..., NoOrder, ExactlyOnce, NoDrop>
```

Technically, nothing. Nothing is observed, because no realization changes. By the rules so far the conversion could be free — and so could reading `count`, whose `MayDrop` is likewise distinguishing nothing a safety argument can see.

That should make us uneasy. We began with a reliable program and a test asserting that `count` eventually reads `3`. We switched to a lossy channel, routed it through `delayed_forever`, converted it back to `NoDrop` for free, counted — and the program typechecks with not one word of developer reasoning anywhere in it, while the test can no longer pass, because `count` may never reach `3`. The safety story is intact. The liveness story has vanished, and nothing in the program marks the place where it vanished.

The difference between `NoOrder` `MayDrop` and `NoOrder` `NoDrop` was never a difference in realizations. It is a difference in *promise*: `NoDrop` promises that the realization eventually reaches the ideal — a stream fills in, a singleton follows a trajectory to its end — and `MayDrop` withdraws that promise. What `observe_drops` does on an unordered stream is restore the promise. Whether that is acceptable — because something elsewhere retransmits, because the deployment bounds the loss, or because the application can live with a count that stops short — is not something a type can check. It is a judgement the developer owes an explanation for, which is exactly what guards are for. A `nondet!` does not make nondeterminism go away; its comment explains why the application is still correct in spite of it. The same move for liveness is a guard of a new kind:

```rust
let delivered = unordered.observe_drops(nonlive!(
    /// The source retransmits each event until the sink acknowledges it, so every
    /// event is eventually delivered.
)); // Stream<..., NoOrder, ExactlyOnce, NoDrop>
```

`nonlive!` is to liveness what `nondet!` is to safety. The panic earlier was a safety violation, and today's Hydro already demands a `nondet!` for exactly that reason; a test that never completes is a liveness violation, and until now nothing demanded anything. The new guard has no runtime effect. It marks the point where the program takes on a liveness concern, and it carries the developer's explanation of why the program is still acceptable there: the concern is mitigated elsewhere, or the application tolerates it. Looking back at the ordered case, `observe_drops` there was also declaring its output `NoDrop` — also restoring the promise — so it needs both guards:

```rust
let in_order = received.observe_drops(
    nondet!(/** a gap is reported, not asserted */),
    nonlive!(/** the source retransmits until acknowledged */),
);
```

The two guards answer two independent questions. The `nondet!` explains why observing *which* elements survived is harmless — a safety question, which arises only when the order makes the survivors a new ideal. The `nonlive!` explains why the program is acceptable even though not every element may survive — a liveness question, which arises whenever `NoDrop` is claimed. Unordered streams need only the second; ordered `MayDropAnywhere` streams need both.

### `fail_stop` defers too
With that vocabulary in hand, look back at the "reliable" channel we started with. Its implementation says:

```rust
// A failed connection stops *all* future deliveries to that recipient, which models the
// recipient as having failed.
```

A `fail_stop` channel loses messages too — every message after a connection failure. The loss has a special *shape*: it is always a suffix, so what the recipient receives is a prefix of the ideal — a realization in the ordinary order, and exactly the one a crashed recipient would have ended on. Nothing about the natural order changes; what changes is only the promise. So `fail_stop` deserves the same treatment as `lossy`, with a drop type that says so:

```rust
let received = events.send_bincode(sink, TCP.fail_stop()); // Stream<(u64, Payload), Process<Sink>, Unbounded, TotalOrder, ExactlyOnce, MayDropSuffix>
```

`MayDropSuffix` keeps the prefix order and withdraws the promise. Because its realizations are ordinary realizations, *every* operator accepts it, including the sequence-check fold — our original program typechecks unchanged over it, as it should, since the assertion really cannot fire. Its outputs carry `MayDropSuffix` (or `MayDrop`, for singletons), and converting back to `NoDrop` needs no `nondet!`, since nothing new is observed, only a `nonlive!`:

```rust
let delivered = received.observe_drops(nonlive!(
    /// A sink whose connection fails is treated as having crashed; the deployment
    /// restarts it, and a restarted sink reconnects and resumes from its checkpoint.
)); // Stream<..., TotalOrder, ExactlyOnce, NoDrop>
```

That comment is the policy's own "models the recipient as having failed" — a decision, not a fact about TCP, since a connection can fail while both endpoints are alive — now written by the program that relies on it, at the point where it relies on it, instead of once in the library for every program at once.

The three drop types on an ordered stream form a chain, `NoDrop ≤ MayDropSuffix ≤ MayDropAnywhere`: each may be used where the next is accepted, and each step down the chain admits more operators and demands less at `observe_drops`. `fail_stop` and `lossy` differ not in whether they lose messages but in the shape of the loss, and the shape is what the type records.

## Rules

### The drop type
`Stream` and `KeyedStream` gain a parameter `Drop: Drops = NoDrop`. On ordered streams the lossy drop types are `MayDropSuffix` and `MayDropAnywhere`; on unordered streams, and on singletons and optionals, the two coincide and are written `MayDrop`. The drop type determines the stream's natural partial order and its liveness promise:

| | `NoDrop` | `MayDropSuffix` | `MayDropAnywhere` |
|---|---|---|---|
| realizations of an ordered stream | prefixes of the ideal | prefixes of the ideal | subsequences of the ideal |
| realizations of an unordered stream | sub-multisets of the ideal | sub-multisets of the ideal (`MayDrop`) | sub-multisets of the ideal (`MayDrop`) |
| promise | the realization eventually reaches the ideal | none | none |

`NoDrop ≤ MayDropSuffix ≤ MayDropAnywhere`: a stream may be used wherever a stream of a later type is accepted, as a `Bounded` stream may be used where an `Unbounded` one is. At every drop type the denotation is the ideal.

Network policies set the drop type, and none of them takes a guard.

| policy | output type |
|---|---|
| `fail_stop()` | `TotalOrder`, `MayDropSuffix` |
| `lossy()` | `TotalOrder`, `MayDropAnywhere` |
| UDP | `NoOrder`, `MayDrop` |

The `lossy_delayed_forever` policy is removed; `delayed_forever()` is an operator.

### Operators on lossy streams
An operator may consume a lossy stream when it is safe with respect to the natural orders of its input and output types *with their drop types*: its output on every realization of its input is a realization of its output on the ideal, where "realization" on the input side means prefix (`MayDropSuffix`), subsequence (`MayDropAnywhere`), or sub-multiset (unordered), and on the output side means whatever the output type's own drop type says.

| operator | `TotalOrder` `MayDropSuffix` | `TotalOrder` `MayDropAnywhere` | `NoOrder` `MayDrop` | output |
|---|---|---|---|---|
| `map`, `filter`, `filter_map`, `flat_map_*`, `merge_unordered`, `into_keyed`, `values`, `join` | ✓ | ✓ | ✓ | the input's drop type |
| commutative `fold`, `reduce`, `count`, `max`, `min`, keyed folds | ✓ | ✓ | ✓ | `Singleton`/`Optional` marked `MayDrop` |
| non-commutative `fold`, `first`, `enumerate` | ✓ | ✗ | — | `MayDropSuffix` / `MayDrop` |
| `last` | ✓ | ✓ | — | `MayDrop` |
| `batch`, `snapshot` | ✗ | ✗ | ✗ | — |
| `delayed_forever()` | ✓ | ✓ | — | `NoOrder`, `MayDrop` |
| `observe_drops(nonlive!)` | ✓ | — | ✓ | `NoDrop` |
| `observe_drops(nondet!, nonlive!)` | — | ✓ | — | `NoDrop` |

Four rows deserve comment.

**`MayDropSuffix` refuses nothing but `batch` and `snapshot`.** Its realizations are prefixes, which is what every operator was already specified for; the drop type only withholds the promise, so outputs inherit it and conversion back needs only a `nonlive!`.

**Commutative folds are admitted on every lossy type.** A commutative fold admits every ordering of its input, so its ideal contains a trajectory for every ordering, and the fold of any subsequence of the ideal traces a prefix of the trajectory in which the missing elements come last. The proof obligation is the one Hydro already imposes on folds over `NoOrder` streams (`commutative = manual_proof!(...)`); nothing new appears. A non-commutative fold over an ordered stream admits one ordering and has one trajectory; its output on a non-prefix subsequence lies on no trajectory, so it is refused on `MayDropAnywhere` — it is the sequence check.

**`last` is admitted on `MayDropAnywhere`; `first` is not.** The last survivor is the last element of the prefix of the ideal that ends at it, so `last` on a subsequence equals `last` on an ordinary realization. The first survivor need not head any prefix.

**`batch` and `snapshot` require `NoDrop`.** A batch of a lossy stream would raise questions this proposal does not answer — a tick may run with an empty batch while the elements that should fill it never arrive, and the tick-level progress facts Hydro reasons with (a batch chain eventually consuming its input) have no meaning over such a stream. For now, a lossy stream must pass through `observe_drops` before it enters a tick, and no lossy drop type appears on a `Bounded` collection.

### Singletons
A fold over a lossy stream produces a `Singleton` (or `Optional`) marked `MayDrop`. Operators that only transform its value (`map`, `filter`, `zip`) propagate the drop type. Reading it from a tick — `snapshot`, and the threshold patterns built on it — requires `NoDrop`, so the program must first write

```rust
let count = count.observe_drops(nonlive!(
    /// Every event is eventually delivered, so the count eventually reaches the number sent.
)); // Singleton<usize, Process<Sink>, Unbounded, ..., NoDrop>
```

A `MayDrop` singleton needs only a `nonlive!`, never a `nondet!`, and the trajectory picture says why. At every moment the singleton's realization is a prefix of some trajectory of the ideal — after a drop, a different trajectory than before, but always one of them — so the value a `snapshot` reads is a value on some trajectory, which is exactly what a `snapshot` reads in any execution. The `nondet!` the snapshot already carries ("some version of the counter") already accounts for the choice of how far along the singleton is. A threshold that never fires emits nothing wrong. What the singleton's `MayDrop` withdraws is only the promise that it follows a trajectory to its end, and that is what `nonlive!` answers for. This holds for every fold the table admits: commutative folds supply trajectories for every ordering regardless of the source's drop type, and a non-commutative fold is admitted only over `MayDropSuffix`, whose prefix realizations trace its single trajectory. The one fold that would have needed a `nondet!` — a non-commutative fold over `MayDropAnywhere` — was refused before any singleton existed.

### Why both constructs
The drop type and the liveness guard answer different questions and vary independently; neither can stand in for the other.

The drop type answers *what the realizations of this stream look like, and whether they are promised to reach the ideal*, and it must travel with the data without anyone writing anything: a `map` of a lossy stream is lossy whether or not its author is thinking about liveness, and a consumer three modules away must see that. A guard cannot do this; guards sit at program points and do not flow along edges.

The guard answers *why the program is acceptable at a point where liveness is in question*, and the concern is frequently not about drops at all. Paxos makes progress only if, eventually, one leader's heartbeats reach the other proposers before their timeouts fire — a property of message *timing* on channels that lose nothing. A type cannot carry this; it is a judgement about the deployment and the application, checkable on no input.

| | no further `nonlive!` | further `nonlive!` needed |
|---|---|---|
| **`NoDrop`** | a quorum over live, reliable participants, waiting for the fastest `min` | a quorum over `fail_stop` channels tolerating `max - min` crashes |
| **lossy** | a best-effort consumer — metrics, gossip — that promises only safety | a quorum over a lossy link, answered for by retransmission or a fault bound |

The pair `MayDrop` / `nonlive!` mirrors `NoOrder` / `nondet!`: `NoOrder` carries the *possibility* of reordering through the graph without annotation, and `nondet!` marks the point where a program *relies* on an order and explains why that is all right. `MayDrop` carries the possibility of loss; `nonlive!` marks the point where a program relies on arrival and explains why that is all right.

## Composition: Liveness at Module Boundaries
A guard that only ever appeared inline would not scale past a single function. Hydro already has conventions for how `NonDet` guards compose: a component that contains nondeterministic operators takes a `NonDet` parameter, documents under `# Non-Determinism` what the nondeterminism affects, and threads the parameter to the operators it controls; a caller justifies its guard by naming an outer one, so that the justification for a deep `batch` can be traced back to a top-level decision. Liveness guards need the same patterns, and this section works them out on the two cases that matter most — a library component whose progress is conditional on its inputs, and a protocol that composes several such components under one fault model. The guiding rule is the same as for `NonDet`: a component that can explain a liveness concern itself does so inline; a component that cannot forwards the obligation to its caller as a `NonLive` parameter and says, under a `# Liveness` heading, exactly what the caller must explain.

### `collect_quorum`
A quorum is the canonical component whose progress depends on something it cannot check. `collect_quorum` batches its responses into ticks, counts them per key with a commutative fold, and emits a key once its count reaches `min`:

```rust
let just_reached_quorum = sliced! {
    let new_inputs = use::batch(responses.clone(), nondet!(
        /// We always persist values that have not reached quorum, so even
        /// with arbitrary batching we always produce deterministic quorum results.
    ));
    // ... chain with unresolved responses, count per key, filter at `min` ...
};
```

Because its body batches, its input must be `NoDrop`; a caller holding a lossy stream of responses — and every stream that has crossed a network is lossy at some drop type — must `observe_drops` it first. But even over a `NoDrop` input the quorum's guarantee is conditional. It emits a key when `min` successful responses for that key have arrived; whether `min` responses are ever *sent* depends on how many participants are live, which the quorum does not know. The quorum therefore takes a liveness guard and documents its condition, exactly as a component with a `# Non-Determinism` section takes a `NonDet`:

```rust
/// Collects responses into quorums.
///
/// # Liveness
/// For every key `k`, if at least `min` successful responses for `k` are eventually
/// delivered, then `k` is eventually emitted on the first output. The guard must explain
/// why this condition holds, or why the caller can tolerate its failing, for every key the
/// caller depends on.
pub fn collect_quorum<'a, L: Location<'a>, Order: Ordering, K: Clone + Eq + Hash, E: Clone>(
    responses: Stream<(K, Result<(), E>), L, Unbounded, Order, ExactlyOnce, NoDrop>,
    min: usize,
    max: usize,
    nonlive: NonLive,
) -> (
    Stream<K, L, Unbounded, NoOrder, ExactlyOnce, NoDrop>,
    Stream<(K, E), L, Unbounded, Order, ExactlyOnce, NoDrop>,
)
```

The module contains no `nonlive!` of its own. It has nothing to explain; it forwards the obligation to the caller, who knows why `min` responses will arrive or why it is acceptable if they do not. The two guards a caller writes — the `observe_drops` on the way in and the quorum's parameter — answer different questions: the first, why the responses that are *sent* arrive; the second, why enough are *sent*. The three ways a quorum is used become three explanations of the second:

| use | the caller's explanation |
|---|---|
| latency: the fastest `min` of a live, reliable set | all `max` participants are live, so all `max` responses arrive |
| crash tolerance over `fail_stop` | at most `max - min` participants crash |
| loss tolerance over a lossy link | enough responses survive, by retransmission or a fault bound |

### Paxos
Paxos takes on liveness concerns at three kinds of sites — the `observe_drops` on each network stream, the two quorum calls, and the leader-election timers — and every site explains itself by naming one of two root guards that `paxos_core` exposes as parameters, exactly as it exposes `nondet_leader` and `nondet_commit` today:

```rust
pub fn paxos_core<'a, P: PaxosPayload>(
    // ...
    acceptor_liveness: NonLive,
    leader_stability: NonLive,
) -> /* ... */ {
    // ...
    let a_to_proposers_p2b = /* acceptors' p2b responses over TCP.fail_stop(), demuxed to proposers */
        .values() // Stream<..., Cluster<Proposer>, Unbounded, NoOrder, ExactlyOnce, MayDrop>
        .observe_drops(nonlive!(
            /// An acceptor whose connection fails is counted among the f that may fail.
            acceptor_liveness
        ));

    let (quorums, fails) = collect_quorum(
        a_to_proposers_p2b,
        f + 1,
        2 * f + 1,
        nonlive!(
            /// Every p2a reaches all 2f + 1 acceptors, at most f acceptors fail, and the leader
            /// is eventually stable, so f + 1 p2b responses for each slot are eventually sent.
            acceptor_liveness,
            leader_stability
        ),
    );
    // ...
}
```

`leader_stability` is the concern with no drop in it: Paxos progresses only if the network is eventually synchronous with respect to the leader timeouts. Both roots are explained at the deployment:

```rust
paxos_bench(
    // ...
    nonlive!(
        /// The deployment runs 2f + 1 acceptors and tolerates at most f failures, counting a
        /// persistently failed connection as a failure.
    ),
    nonlive!(
        /// The network is eventually synchronous with respect to the configured leader timeouts,
        /// so re-elections eventually stop.
    ),
);
```

Switching the acceptor-to-proposer channel to `lossy()` changes nothing in the types — after `.values()` the stream is `NoOrder` `MayDrop` either way — but it changes what the `observe_drops` guard has to say: it must now explain why `f + 1` responses *survive*, not merely why `f + 1` acceptors are live. Paxos does not retransmit p2a messages, so that explanation cannot be written honestly — which is the point. The guard proves nothing; it makes the place where an explanation is owed impossible to overlook, and an unwritable explanation conspicuous.

The existing `nondet!` comments in Paxos already reason this way without a word for it. The snapshot of the maximum ballot is guarded with "a stale max ballot might result in us failing to become the leader" — a liveness consequence, accepted in a safety guard because no liveness guard existed.

## API Changes
1. **`Stream` and `KeyedStream` gain `Drop: Drops = NoDrop`**, with `MayDropSuffix` and `MayDropAnywhere` on ordered streams and `MayDrop` on unordered ones; `Singleton` and `Optional` carry `MayDrop` when derived from a lossy stream.
2. **No network policy takes a guard.** `fail_stop()` produces `MayDropSuffix`; `lossy()` produces `MayDropAnywhere` and no longer takes a `NonDet`; UDP produces unordered `MayDrop`. The `lossy_delayed_forever` policy is removed.
3. **`delayed_forever()`** converts an ordered lossy stream to an unordered one with no guard.
4. **`observe_drops`** converts to `NoDrop`: `observe_drops(nonlive!(...))` on `MayDropSuffix`, on unordered streams, and on singletons; `observe_drops(nondet!(...), nonlive!(...))` on `MayDropAnywhere`.
5. **`NonLive` and `nonlive!`** are added, with nested justification and a `# Liveness` rustdoc convention.
6. **`batch` and `snapshot` require `NoDrop`.**
7. **`collect_quorum`, `collect_quorum_with_response`, and `join_responses` take a `NonLive`.**
8. **`ConsistencyGuarantee` is derived, not declared.** A replicated collection is eventually consistent across live members exactly when its upstream is `NoDrop`. The associated type `Location::DropConsistency` — which means "drop the consistency guarantee" — should be renamed (for example `WithoutConsistency`) to avoid confusion with the drop type.
9. **`SimFlow::test_safety_only` keeps its role, now aligned with the types.** It continues to opt a simulation into the delayed-forever treatment of drops — "dropped" messages are held back to the end of the execution, so safety is exercised and liveness is not — and that treatment is exactly right for the sites that are only `nonlive`: `MayDropSuffix` and unordered `MayDrop` streams, and the `observe_drops(nonlive!)` conversions on them. Sites that are both `nondet` and `nonlive` — `observe_drops(nondet!, nonlive!)` on a `MayDropAnywhere` stream — cannot be served by delaying, because delaying does not produce the subsequences the program has declared it can handle; the simulator must either drive the drops through a hook attached to the guard, as it drives `lossy` today, or reject the program.

## Open Questions
1. **`batch` over lossy streams.** Allowing it would let a tick process whatever arrives without a guard, at the cost of defining what an empty batch means when its elements never come. We defer this.
2. **Per-key drops in keyed streams.** `broadcast_closed` keys its output by sender and preserves order within a key. Should a lossy keyed stream carry a per-key drop type, so that positional operators on a single key are governed by `MayDropSuffix`/`MayDropAnywhere` while operators across keys see `MayDrop`?
3. **Mechanizing liveness.** The Lean formalization proves quorum liveness by hand (see the appendix). Generating per-module liveness faces with threaded guards is a separate design.
4. **Timers.** Leader election's progress is a property of the clock. `leader_stability` explains it; whether timer-driven streams need a drop type of their own is open.

## FAQs

### Isn't a fold over a lossy stream nondeterministic, and therefore unsafe?
A commutative fold is not. At every moment its realization is a prefix of some trajectory of its ideal — the trajectory in which the dropped elements arrive last — so every value it outputs is one it would also output with those elements merely late, and the commutativity proof the fold already carries is the proof. What the fold loses is the promise that it follows a trajectory to its *end* — a liveness concern, answered for by `nonlive!` when the singleton is observed. A non-commutative fold over a `MayDropAnywhere` stream has a single trajectory, its output after a drop lies on no trajectory, and it *is* unsafe (the sequence check); it is refused until `observe_drops`. Over `MayDropSuffix` the same fold is safe, because a prefix of a prefix is a prefix.

### Couldn't a monotonic fold be allowed everywhere, since its value stays at most the ideal's?
An inflationary fold over a sub-collection is bounded by the fold over the whole, so a `Monotonic` singleton derived from a lossy stream does sit below its ideal's final value in the singleton's own order. But "below the final value" is not "on a trajectory": dropping `1` and dropping `2` give different values, both below the sum, and a non-commutative fold's output on a subsequence need lie on no trajectory at all. Admitting it on these grounds would launder drop-nondeterminism into a singleton with no `nondet!` site, and every consumer of a snapshot (`filter(ballot == max)`, say) would need its own monotonicity proof for the bound to mean anything. Commutative folds are admitted anyway, with no such obligation. "Realized at most ideal" survives as a lemma about `observe_drops`, not as a license.

### Why not add drops as a retry type — `AtMostOnce` beside `ExactlyOnce` and `AtLeastOnce`?
Because the retry types are collection quotients (Gyatso's `[T]dup`): a retried realization is *equivalent* to the ideal, so the collection type can identify them. A dropped realization is strictly smaller than the ideal, and a quotient that identified a stream with its sub-collections would identify it with the empty stream. Drops have to change the stream's relation to its ideal instead, which is what the drop type does. Keeping the axes separate also lets a channel be `AtLeastOnce` and lossy at once: retransmit, but eventually give up.

### Why not a `SubsequenceOrder` ordering type, so that a drop type never has to change an order?
Because the ordering and retry types define **equivalence classes** of the ideal, and subsequence cannot be one. `NoOrder` identifies sequences that differ by a permutation; `AtLeastOnce` identifies sequences that differ by consecutive duplicates; in each case the ideal is the class, and the class comes with the natural partial order its concatenation induces. If "differ by a subsequence" were an equivalence, every sequence would be identified with each of its subsequences, and therefore with `[]` — a single class, and an ideal that says nothing. So subsequence is not something the ideal can forget; it is only ever a *relation* between a realization and an ideal that keeps its full order.

That is the division this proposal makes. The ordering and retry types say what the ideal forgets, and the equivalence classes they define carry the natural partial order that realizations grow along. The drop type then permits certain collections to compare their realizations against the ideal under an *alternative* partial order — subsequence, for `TotalOrder` with `MayDropAnywhere` — without changing the class or the growth order. A `SubsequenceOrder` would try to make that alternative order into a class, which collapses, or would smuggle a relation-to-ideal onto the parameter whose job is to say what the ideal forgets.

One might conclude from this that a drop type should *always* use the class's natural partial order — that `MayDrop` should mean "prefix realizations, no promise" everywhere, and a lossy ordered channel should simply hand back `NoOrder`. We think that reading is confusing precisely on ordered streams, where a single `MayDrop` would be ambiguous between losing a suffix and losing anywhere, and the two admit different operators. So ordered streams name both, `MayDropSuffix` and `MayDropAnywhere`. On unordered streams, and on singletons and optionals, there is no such ambiguity — a multiset has no suffix to lose, and a singleton has no order — so the drop type is plainly `MayDrop`.

### Why not make `collect_quorum` accept a lossy input and return `NoDrop` when enough responses survive?
Because "enough survive" is not a property of the input. It depends on `f`, which the quorum does not know, and on the adversary's choices, which nothing observes. A type that depended on it would be a refinement over an unobservable fact. The guard records the same information as a documented explanation at the call site, which is where the knowledge lives.

### Why does `fail_stop` get its own drop type instead of sharing `lossy`'s?
Because the shape of its loss is different and the difference is useful. A `fail_stop` loss is a suffix, so what the recipient holds is a prefix of the ideal — an ordinary realization, exactly the one a crashed recipient would have ended on. Under `MayDropSuffix` every operator stays available, including non-commutative folds, `first`, and `enumerate`, and `observe_drops` needs no `nondet!`, because nothing is observed that the ideal did not already determine. A `lossy` loss is a hole in the stream of a live recipient, so what remains is a realization only under the subsequence order, positional operators are refused, and `observe_drops` must observe which elements survived. Both are deferred the same way; the type records which kind of loss the consumer is answering for.

## Appendix: Semantics and the Lean Formalization

### Three propositions
Write `⊑` for the ordinary natural order (prefix on ordered streams, sub-multiset on unordered ones, prefix of a trajectory on singletons) and `⊑ᴬ` for the `MayDropAnywhere` order (subsequence on ordered streams, sub-multiset on unordered ones). `MayDropSuffix` uses `⊑` itself. An operator is safe when `r ⊑ᵢₙ I` implies `op(r) ⊑ₒᵤₜ op(I)`, where each order is the one belonging to the type on that side, including its drop type. An operator may consume a lossy stream when it is safe with `⊑ᵢₙ` set to that stream's order; its output order is the output type's own.

Two orders are in play on a lossy stream, and they must agree. Realizations still *grow* by the collection's own concatenation — append on sequences, union on multisets — so between one realization and the next the relation is the ordinary `⊑`; a drop type changes only how a realization relates to the *ideal*. For `⊑ᴬ` to be a legitimate relation on a stream grown this way it must be preserved by growth:

> if `r ⊑ᴬ I` and `r ++ δ` is a later realization, then `r ++ δ ⊑ᴬ I`.

On unordered streams this is union preserving sub-multisets. On ordered streams it is the statement that a lossy channel delivers survivors in sender order — the appended `δ` is a subsequence of the part of `I` after the last element `r` matched. This condition is what makes `last` safe on `MayDropAnywhere` streams (growth never inserts before the current last element) and what lets `observe_drops` turn the survivors into a new ideal whose realizations are its prefixes.

> **Proposition 1 (unordered drops are safety-invisible).** On unordered streams `⊑ᴬ` and `⊑` coincide, so every safe operator is safe with `⊑ᵢₙ = ⊑ᴬ`.
>
> *Argument.* Both orders are sub-multiset. ∎

> **Proposition 2 (ordered drops anywhere are safety-visible).** On ordered streams `⊑ᴬ` is strictly weaker than `⊑`, and a safe operator need not be safe with `⊑ᵢₙ = ⊑ᴬ`.
>
> *Argument.* `[0, 2] ⊑ᴬ [0, 1, 2]` but `[0, 2]` is not a prefix. The sequence-check fold is safe with `⊑ᵢₙ = ⊑` (its output on any prefix of its input traces a prefix of its single trajectory) and not with `⊑ᵢₙ = ⊑ᴬ` (its output on `[0, 2]` is a panic, on no trajectory). ∎

> **Proposition 3 (suffix loss leaves an ordinary realization).** Under `fail_stop`, what a recipient receives is a prefix of the ideal, so `MayDropSuffix` uses `⊑` and every safe operator is safe over it.
>
> *Argument.* A failed connection delivers no later message, and a stream that stops is a prefix of the stream that would have continued. ∎

Proposition 1 justifies `delayed_forever`, the admission of every operator on unordered `MayDrop` streams, and `observe_drops` taking only a `nonlive!` there and on singletons. Proposition 2 justifies refusing positional operators on `MayDropAnywhere` streams and the `nondet!` on their `observe_drops`. Proposition 3 justifies `MayDropSuffix` admitting every operator and converting with a `nonlive!` alone. Commutative folds are safe with `⊑ᵢₙ = ⊑ᴬ` at both orderings because their ideal contains a trajectory for every ordering of the input, and the fold of any subsequence traces a prefix of the trajectory in which the missing elements come last.

### The coupling and the census
The Lean formalization in `hydro_lean` interprets each program at a denotation (`Values`, the ideal) and at a step machine (`SchedSem`, the realization), and couples them. For a stream the coupled corner records

```lean
cpl : wf → ∀ i, ListLe ord ret ((sr i).view T) (rr i)
```

where `sr` is the machine's history, `rr` the denotation, `T` the horizon, and `ListLe` is prefix at `.totalOrder` and sub-multiset at `.noOrder` — the relation `⊑` above. The denotation of a fold is likewise a trajectory of values over successive realizations of its input, and the fold corner's coupling says the machine's current value is the fold of its own view. "Not yet arrived" and "dropped" are indistinguishable at any finite `T`, so every safety theorem proved today — `paxos_safe_sched'`, `cq_safe_sched`, `two_pc_safe_sched` — already holds for `MayDropSuffix` and unordered `MayDrop` inputs; this is Propositions 1 and 3 mechanized. `MayDropAnywhere` adds a third index to `ListLe` under which the `.totalOrder` case is sublist, and the operators whose coupling proofs use the prefix property are exactly those the rules refuse.

The per-module census reports three columns: content nondeterminism (`SnapDec`, `BatchDec`, `OrderSelDec`, …), which every contract quantifies over because it changes the ideal output; schedule decisions (delivery cursors, tick pacing, timers), which the signature documents as "adversary/runtime freedom, silently ∀-quantified, never in contracts"; and cycle fuels. Content nondeterminism is the `nondet` axis; schedule decisions are the `nonlive` axis — a cursor or a timer can withhold, never produce. A suffix loss is a delivery cursor that stalls; an unordered drop is a cursor that may *skip* a position; both are schedule decisions that enter no contract. An ordered drop observed through `observe_drops` on `MayDropAnywhere` is a content decision — the survivor subsequence — quantified like any other. `nonlive!` guards would become a fourth column.

### Liveness faces
The quorum liveness theorem already has the shape the proposal needs:

```lean
theorem cq_live
    (h : Fin 1 → StepHist (K × Except E Unit)) {T₀ : Nat}
    (hstab : StabilizesAt (h 0) T₀)
    (mn mx : Nat) (k : K)
    (h1 : 1 ≤ mn) (hmm : mn ≤ mx)
    (hcap : cqKeyCount (Multiset.ofList ((h 0).view T₀)) k ≤ mx)
    (hq : mn ≤ cqOkCount (Multiset.ofList ((h 0).view T₀)) k)
    (hsupply : ∃ Tw, T₀ ≤ Tw ∧ pacing.ticks () 0 Tw = true) :
    ∃ T, k ∈ ((collect_quorum (SchedSem ...) () h mn mx ()).1 0).view T
```

Its premise `hq` counts successes in `h`, the machine's *delivered* history, not in the ideal — it is the `# Liveness` clause of the proposed signature — and `hsupply` is the member-liveness assumption. A `nonlive!` guard is the discharge of `hq` at a call site. The natural Lean counterpart is a `live` face beside `ensures`, whose hypotheses are the guards:

```lean
hydro def collect_quorum ... (responses : H.Stream ℓ (K × Except E Unit) .noOrder .exactlyOnce .noDrop) ...
  ensures out => CQEnsures ...
  live out => ∀ k, mn ≤ cqOkCount (delivered responses) k → Eventually (k ∈ out.1)
```

### Flo and Gyatso
Flo types each stream as `(C, B | U)`: a collection type and a boundedness flag, with `(C, B) ≤ (C, U)`. Boundedness is already a liveness flag — whether the collection eventually becomes *fixed* — and the drop type is a second: `(C, B | U, NoDrop | MayDropSuffix | MayDropAnywhere)`, ordered as written. It must be a flag on the stream type rather than a new collection type, because Flo absorbs nondeterminism by *quotienting* the collection (`[T]dup`, `[T]unord`) and drops are not quotientable: identifying a stream with its sub-collections identifies it with the empty stream. Gyatso's natural partial order (Definition 3.4.1 in the dissertation) is a property of the collection type; this proposal lets the stream type override the order used to relate realizations to the ideal, so that the same collection — a sequence — is compared by prefix at `NoDrop` and `MayDropSuffix` and by subsequence at `MayDropAnywhere`, while growing by append in every case.

Gyatso's monotone-outputs theorem is the safety property above, proved for whole programs under crash-stop failures, where a failed execution is a prefix of a live one. The drop types refine it along the line of the propositions: `MayDropSuffix` *is* crash-stop in Gyatso's sense and keeps the proof as stated; unordered drops keep it too (the dropped elements arrive last in some live execution — the reading `delayed_forever` names, and the trajectory the realized singleton is a prefix of); `MayDropAnywhere` holds it only for the subsequence order.

Gyatso's eventual-determinism theorem assumes, globally, that every machine is live and every message is eventually delivered. This proposal decomposes the assumption and attaches each part where it is used: member liveness remains standing (the machine's fairness premises); delivery becomes the per-stream drop type; everything else — a failure bound, eventual synchrony, retransmission, the decision to count a failed connection as a crash — becomes a named guard whose comment explains why the program is acceptable without it. Eventual determinism then holds for an output whenever its upstream is `NoDrop` and every guard on the way is justified; the global theorem is the case where every stream has been converted and all guards hold.
