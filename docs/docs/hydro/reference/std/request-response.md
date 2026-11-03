---
sidebar_position: 1
---

# Request and Response

When you send a request, you often know something the response will not repeat: the payload you proposed, which client is waiting, or which ballot you used. [`join_responses`](rust:hydro_std::request_response::join_responses) stores that metadata until the matching response arrives.

You pass two streams that share a key type (`Clone + Eq + Hash`). Metadata and response values must be `Clone`.

- `responses` is an unbounded, unordered stream of `(key, value)`, one element per key.
- `metadata` is a tick-bounded, unordered stream of `(key, metadata)`, one element per key. Produce it on the same tick you send the request, or on an earlier tick.

The output is an unbounded, unordered stream of `(key, (metadata, value))`.

```rust,no_run
# use hydro_lang::live_collections::stream::NoOrder;
# use hydro_lang::prelude::*;
use hydro_std::request_response::join_responses;

let mut flow = FlowBuilder::new();
let process = flow.process::<()>();
let tick = process.tick();

// Unbounded replies. `join_responses` requires `NoOrder`.
let responses = process
    .source_iter(q!(vec![(1u64, 7u32)]))
    .batch(&tick, nondet!(/** joining does not depend on batch boundaries */))
    .all_ticks()
    .weaken_ordering::<NoOrder>();

// What you knew at request time, batched onto the process tick.
let metadata = process
    .source_iter(q!(vec![(1u64, "east")]))
    .batch(&tick, nondet!(/** metadata is captured on the sending tick */))
    .weaken_ordering::<NoOrder>();

// `(1, ("east", 7))`
let joined = join_responses(responses, metadata);
```

The `batch` and `all_ticks` calls above only give the example a stream of the right shape. In a service, `responses` is the unbounded reply stream (often what came back over the network) and `metadata` is the tick-bounded record you kept when you sent.

Metadata stays in memory across ticks until a response with the same key arrives. A response with no stored metadata produces nothing. Because the metadata is persisted, the joined pairs do not depend on how responses are split across batches.

:::note

Send one metadata element and one response per key. After a key is joined, its metadata is removed, so a second response for that key is dropped.

:::

## After a quorum

[`collect_quorum`](./quorum.md) emits bare keys. Map each key to a pair before you join it with the metadata you saved at send time. The quorum success stream is already unordered:

```rust,no_run
# use hydro_lang::live_collections::stream::NoOrder;
# use hydro_lang::prelude::*;
use hydro_std::quorum::collect_quorum;
use hydro_std::request_response::join_responses;

fn with_metadata<'a>(
    votes: Stream<(u64, Result<(), i32>), Process<'a>, Unbounded>,
    metadata: Stream<(u64, &'static str), Tick<Process<'a>>, Bounded, NoOrder>,
) -> Stream<(u64, (&'static str, ())), Process<'a>, Unbounded, NoOrder> {
    let (committed, _failed) = collect_quorum(votes, 2, 3);
    join_responses(committed.map(q!(|key| (key, ()))), metadata)
}
```
