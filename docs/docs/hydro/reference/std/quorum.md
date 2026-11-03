---
sidebar_position: 0
---

# Quorums

Several replicas often reply to the same request. You use [`collect_quorum`](rust:hydro_std::quorum::collect_quorum) and [`collect_quorum_with_response`](rust:hydro_std::quorum::collect_quorum_with_response), both in `hydro_std::quorum`, to tell when enough of those replies succeeded.

You pass an unbounded [stream](../streaming-data/streams.md) of `(key, Result<_, _>)`. The key groups every reply for one request. For each key, both functions count `Ok` replies and `Err` replies separately, and return `(success_stream, error_stream)`:

- Successes are emitted once the `Ok` count reaches `min`.
- The error stream carries `(key, error)` for every `Err`. It is not limited by `min` or `max`: a key that never reaches `min` still reports its errors, and so does a key that already succeeded.

`max` is the total number of replies you expect for a key, so it should be at least `min`. Set `max == min` when every reply has to succeed, such as a coordinator waiting for every participant. Set `min` to the quorum size and `max` to the number of replicas you contacted when some failures are expected, for example `f + 1` successes out of `2 * f + 1` replies.

Keys must be `Clone + Eq + Hash`. Payloads and errors must be `Clone`.

## Counting successes

`collect_quorum` is the variant whose success payload is `()`. You pass `Result<(), E>`. The success stream contains each key that reached `min`, once, and it is unordered (`NoOrder`) even when the input is ordered. The error stream keeps the input's ordering guarantee.

```rust,no_run
# use hydro_lang::prelude::*;
use hydro_std::quorum::collect_quorum;

fn collect<'a>(
    votes: Stream<(u64, Result<(), i32>), Process<'a>, Unbounded>,
) -> (
    Stream<u64, Process<'a>, Unbounded, hydro_lang::live_collections::stream::NoOrder>,
    Stream<(u64, i32), Process<'a>, Unbounded>,
) {
    // Commit a key once 2 replies are Ok. 3 replies are expected in total.
    collect_quorum(votes, 2, 3)
}
```

For key `1` with replies `Ok`, `Ok`, `Err` and those thresholds, the success stream emits `1` once and the error stream emits `(1, error)`. One `Ok` and two `Err`s do not succeed, but both errors are still emitted.

## Keeping the `Ok` payload

`collect_quorum_with_response` takes `Result<V, E>` instead of `Result<(), E>`. When a key first reaches `min` successes, every `Ok` payload received for that key up through that batch is emitted as `(key, value)`. Both output streams keep the input's ordering guarantee, so on a totally ordered input each stream is in input order. Payloads from a later batch are not added to that emission.

```rust,no_run
# use hydro_lang::prelude::*;
use hydro_std::quorum::collect_quorum_with_response;

fn collect_with_payload<'a>(
    replies: Stream<(u64, Result<u32, i32>), Process<'a>, Unbounded>,
) -> (
    Stream<(u64, u32), Process<'a>, Unbounded>,
    Stream<(u64, i32), Process<'a>, Unbounded>,
) {
    collect_quorum_with_response(replies, 2, 3)
}
```

The same three replies, with payloads `Ok(10)`, `Ok(20)`, and `Err(7)`, emit `(1, 10)` and `(1, 20)` on the success stream and `(1, 7)` on the error stream. Use this variant when the `Ok` value is data you still need. Use `collect_quorum` when you only need the keys.

## `min`, `max`, and batches

A success is emitted as soon as `min` `Ok` replies have arrived, even if you have not yet seen `max` replies. Further replies for that key do not emit success again while the key is still below `max`.

When `max == min`, the key is dropped from the running count once it hits that threshold. When `max != min`, replies that arrive after `min` and before `max` are retained. An `Err` in that window is still reported on the error stream, and those extra replies do not become a second success. Once `max` replies have been counted, the key is forgotten, so a later batch is counted on its own.

Any key that has not yet reached this threshold is persisted. The `Ok` and `Err` counts are therefore the same no matter how the runtime splits that key's replies across batch boundaries.

If you still have metadata from the moment you sent the request, join it back onto the quorum keys with [`join_responses`](./request-response.md).
