---
sidebar_position: 2
---

# Streams
Streams are the most common type of live collection in Hydro; they can be used to model streaming data collections or a feed of API requests. A `Stream` represents a sequence of elements, with new elements being asynchronously appended to the end of the sequence. Streams can be transformed using APIs like `map` and `filter`, based on Rust [iterators](https://doc.rust-lang.org/beta/std/iter/trait.Iterator.html). You can view the full API documentation for Streams [here](pathname:///rustdoc/hydro_lang/live_collections/struct.Stream).

Streams have several type parameters:
- `T`: the type of elements in the stream
- `L`: the location the stream is on (see [Locations](../locations/index.md))
- `B`: indicates whether the stream is [bounded or unbounded](./bounded-unbounded.md)
- `Order`: indicates whether the elements in the stream have a deterministic order or not
  - This type parameter is _optional_; by default the order is deterministic

## Creating a Stream
The simplest way to create a stream is to use [`Location::source_iter`](https://hydro.run/rustdoc/hydro_lang/location/trait.Location#method.source_iter), which creates a stream from any Rust type that can be converted into an [`Iterator`](https://doc.rust-lang.org/beta/std/iter/trait.Iterator.html) (via [`IntoIterator`](https://doc.rust-lang.org/std/iter/trait.IntoIterator.html)). For example, we can create a stream of integers on a [process](../locations/processes.md) and transform it:

```rust
# use hydro_lang::prelude::*;
# use futures::StreamExt;
# tokio_test::block_on(hydro_lang::test_util::multi_location_test(|flow, p_out| {
let process = flow.process::<()>();
let numbers: Stream<_, Process<_>, Unbounded> = process
    .source_iter(q!(vec![1, 2, 3]))
    .map(q!(|x| x + 1));
// 2, 3, 4
# numbers.send(&p_out, TCP.bincode())
# }, |mut stream| async move {
# for w in 2..=4 {
#     assert_eq!(stream.next().await, Some(w));
# }
# }));
```

Streams also can be sent over the network to participate in distributed programs. Under the hood, sending a stream sets up an RPC handler at the target location that will receive the stream elements. For example, we can send a stream of integers from one process to another with [bincode](https://docs.rs/bincode/latest/bincode/) serialization:

```rust
# use hydro_lang::prelude::*;
# use futures::StreamExt;
# tokio_test::block_on(hydro_lang::test_util::multi_location_test(|flow, p_out| {
let p1 = flow.process::<()>();
let numbers: Stream<_, Process<_>, Unbounded> = p1.source_iter(q!(vec![1, 2, 3]));
let p2 = flow.process::<()>();
let on_p2: Stream<_, Process<_>, Unbounded> = numbers.send(&p2, TCP.bincode());
// 1, 2, 3
# on_p2.send(&p_out, TCP.bincode())
# }, |mut stream| async move {
# for w in 1..=3 {
#     assert_eq!(stream.next().await, Some(w));
# }
# }));
```

## Stream Ordering and Determinism
When sending a stream over the network, there are certain situations in which the order of messages will not be deterministic for the receiver. For example, when sending streams from a cluster to a process, delays will cause messages from different cluster members to be interleaved in a non-deterministic order.

To track this behavior, stream have an `Order` type parameter that indicates whether the elements in the stream will have a deterministic order ([`TotalOrder`](pathname:///rustdoc/hydro_lang/live_collections/stream/enum.TotalOrder)) or not ([`NoOrder`](pathname:///rustdoc/hydro_lang/live_collections/stream/enum.NoOrder)). When the type parameter is omitted, it defaults to `TotalOrder` for brevity.

If we send a stream from a cluster to a process and flatten the values across senders (with `.values()`), the return type will be a stream with `NoOrder`:

```rust,no_run
# use hydro_lang::prelude::*;
# let flow = FlowBuilder::new();
use hydro_lang::live_collections::stream::{NoOrder, TotalOrder};
let workers: Cluster<()> = flow.cluster::<()>();
let numbers: Stream<_, Cluster<_>, Unbounded, TotalOrder> =
    workers.source_iter(q!(vec![1, 2, 3]));
let process: Process<()> = flow.process::<()>();
let on_p2: Stream<_, Process<_>, Unbounded, NoOrder> =
    numbers.send(&process, TCP.bincode()).values();
```

The ordering of a stream determines which APIs are available on it. For example, `map` and `filter` are available on all streams, but `last` is only available on streams with `TotalOrder`. This ensures that even when the network introduces non-determinism, the program will not compile if it tries to use an API that requires a deterministic order.

A particularly common API that faces this restriction is [`fold`](pathname:///rustdoc/hydro_lang/live_collections/struct.Stream#method.fold) (and [`reduce`](pathname:///rustdoc/hydro_lang/live_collections/struct.Stream#method.reduce)). These APIs require the stream to have a deterministic order, since the result may depend on the order of elements. For example, the following code will not compile because `fold` is not available on `NoOrder` streams (note that the error is a bit misleading due to the Rust compiler attempting to apply `Iterator` methods):

```compile_fail
# use hydro_lang::prelude::*;
# let flow = FlowBuilder::new();
let workers: Cluster<()> = flow.cluster::<()>();
let process: Process<()> = flow.process::<()>();
let all_words: Stream<_, Process<_>, Unbounded, NoOrder> = workers
    .source_iter(q!(vec!["hello", "world"]))
    .map(q!(|x| x.to_string()))
    .send(&process, TCP.bincode())
    .values();

let words_concat = all_words
    .fold(q!(|| "".to_string()), q!(|acc, x| acc += x));
//   ^^^^ error: `hydro_lang::Stream<String, hydro_lang::Process<'_>, hydro_lang::Unbounded, NoOrder>` is not an iterator
```

:::tip

We use `values()` here to drop the member IDs which are included in `send`. See [Clusters](../locations/clusters.md) for more details.

Running an aggregation (`fold`, `reduce`) converts a `Stream` into a `Singleton`, as we see in the type signature here. The `Singleton` type is still "live" in the sense of a [Live Collection](./index.md), so updates to the `Stream` input cause updates to the `Singleton` output. See [Singletons and Optionals](./singletons-optionals.md) for more information.

:::

To perform an aggregation with an unordered stream, you must use [`fold_commutative`](pathname:///rustdoc/hydro_lang/live_collections/struct.Stream#method.fold_commutative), which requires the provided closure to be commutative (and therefore immune to non-deterministic ordering):

```rust,no_run
# use hydro_lang::prelude::*;
# use futures::StreamExt;
# let flow = FlowBuilder::new();
# let workers = flow.cluster::<()>();
# let process = flow.process::<()>();
# let all_words: Stream<_, Process<_>, _, hydro_lang::live_collections::stream::NoOrder> = workers
#     .source_iter(q!(vec!["hello", "world"]))
#     .map(q!(|x| x.to_string()))
#     .send(&process, TCP.bincode())
#     .values();
let words_count = all_words
    .fold_commutative(q!(|| 0), q!(|acc, x| *acc += 1));
```

:::danger

Developers are responsible for the commutativity of the closure they pass into `*_commutative` methods. In the future, commutativity checks will be automatically provided by the compiler (via tools like [Kani](https://github.com/model-checking/kani)).

:::

## Bounded and Unbounded Streams

:::caution

The Hydro documentation is currently under active development! This is a placeholder for future content.

:::
