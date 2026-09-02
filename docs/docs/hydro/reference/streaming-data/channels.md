---
sidebar_position: 4
---

# Channels
Hydro programs are declarative: each live collection is defined in terms of its inputs. But sometimes a collection needs to be used **before its source is defined**. This comes up in mutually recursive dataflow, and especially in **feedback loops** where a location's output eventually feeds back into its own input (such as gossip protocols or leader election). **Channels** make these patterns possible.

Calling [`Location::channel`](rust:hydro_lang::location::Location::channel) creates a channel, returning a `(sender, receiver)` pair:
- The **receiver** is an ordinary live collection (such as a `Stream`) that can immediately be used as the input to other computations, even though no data has been wired into it yet.
- The **sender** is a [`ChannelSender`](rust:hydro_lang::channel::ChannelSender), whose `send` method is used to wire in the collection(s) that supply the data.

```rust
# use hydro_lang::prelude::*;
# use hydro_lang::live_collections::stream::NoOrder;
# use futures::StreamExt;
# tokio_test::block_on(hydro_lang::test_util::stream_transform_test(|process| {
// Create a channel; the receiver can be used before any data is sent
let (sender, forward_stream) = process.channel::<Stream<i32, _, _, NoOrder>>();

// Use the receiver as input to another computation
let output = forward_stream.map(q!(|x| x * 2));

// Later, wire in the actual source of the data
let source: Stream<_, _, Unbounded> = process.source_iter(q!([1, 2, 3])).into();
sender.send(source);
# output
# }, |mut stream| async move {
// output: 2, 4, 6
# assert_eq!(stream.next().await.unwrap(), 2);
# assert_eq!(stream.next().await.unwrap(), 4);
# assert_eq!(stream.next().await.unwrap(), 6);
# }));
```

Every live collection type can be the target of a channel, including `Stream`, `KeyedStream`, `Singleton`, `Optional`, and `KeyedSingleton`. For most targets, sending is optional: if the sender is dropped without sending anything, the channel acts as a **null input**, and the receiver never produces any elements. The exception is `Singleton` targets, which must always have a value and so cannot act as a null input; their sender panics if dropped without sending.

## Ordered and Unordered Channels
Whether a channel can accept **multiple** sends depends on the ordering of the target collection, which determines the signature of `send`:

- If the target is **ordered** (such as a `Stream` with [`TotalOrder`](rust:hydro_lang::live_collections::stream::TotalOrder)), `send` takes the sender **by value**. Exactly one collection can be sent, since merging several streams would introduce a non-deterministic ordering of their elements.
- If the target is **unordered** (such as a `Stream` with [`NoOrder`](rust:hydro_lang::live_collections::stream::NoOrder)), `send` takes the sender **by reference** and can be called several times. The sent collections are merged as unordered streams, just like [`Stream::merge_unordered`](rust:hydro_lang::live_collections::Stream::merge_unordered).

```rust
# use hydro_lang::prelude::*;
# use hydro_lang::live_collections::stream::NoOrder;
# use futures::StreamExt;
# tokio_test::block_on(hydro_lang::test_util::stream_transform_test(|process| {
let (sender, merged) = process.channel::<Stream<i32, _, _, NoOrder>>();

// The channel is unordered, so several streams can be sent into it
sender.send(process.source_iter(q!([1, 2])));
sender.send(process.source_iter(q!([3, 4])));
# merged
# }, |mut stream| async move {
// merged: 1, 2, 3, 4 in non-deterministic order
# let mut results = Vec::new();
# for _ in 0..4 {
#     results.push(stream.next().await.unwrap());
# }
# results.sort();
# assert_eq!(results, vec![1, 2, 3, 4]);
# }));
```

Attempting to send twice into an ordered channel is a compile-time error, since the first `send` consumes the sender:

```compile_fail
# use hydro_lang::prelude::*;
# let mut flow = FlowBuilder::new();
# let process = flow.process::<()>();
// TotalOrder is the default ordering for streams
let (sender, ordered) = process.channel::<Stream<i32, _, Unbounded>>();

sender.send(process.source_iter(q!([1, 2])));
sender.send(process.source_iter(q!([3, 4])));
// ^ error: use of moved value: `sender`
```

Single-value collections like `Singleton`, `Optional`, and `KeyedSingleton` behave like ordered channels: only a single collection can supply the value, so `send` consumes the sender. For `KeyedStream` targets, it is the ordering of the values within each group that determines whether multiple sends are allowed.

## Feedback Loops
The most common use of channels is closing a **feedback loop**: using the data that arrives back at a location as an input to the computation that produces it. Because unordered channels accept multiple sends, you can wire in a bootstrap source and the feedback path separately:

```rust,no_run
# use hydro_lang::prelude::*;
# use hydro_lang::live_collections::stream::NoOrder;
# let mut flow = FlowBuilder::new();
let p1 = flow.process::<()>();
let p2 = flow.process::<()>();

// Messages arriving at p1, used before the feedback path is defined
let (arrivals_sender, arrivals) = p1.channel::<Stream<u32, _, _, NoOrder>>();

// Bootstrap the loop with an initial element
let init: Stream<u32, _, Unbounded> = p1.source_iter(q!([0])).into();
arrivals_sender.send(init);

// p2 increments each value and sends it back to p1, closing the loop
let incremented = arrivals
    .send(&p2, TCP.fail_stop().bincode())
    .map(q!(|x| x + 1));
arrivals_sender.send(incremented.send(&p1, TCP.fail_stop().bincode()));
```

:::caution

The collections sent into a channel must not depend **synchronously** (within the same [tick](../state-management/slices.mdx)) on the channel's receiver, since a value would then depend on itself; Hydro will panic when compiling such a program. Feedback must pass through an *asynchronous* boundary, such as a network `send` (as in the example above), which breaks the synchronous cycle. For feedback *within* a single location's ticks, use [`Tick::cycle`](rust:hydro_lang::location::tick::Tick::cycle) instead, which defers the fed-back value to the next tick.

:::
