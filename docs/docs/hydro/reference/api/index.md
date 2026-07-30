# Live Collections API

Live collections are the core APIs for building distributed applications with Hydro. This page helps you choose a collection and understand the guarantees represented by its type parameters. Follow the API links for exact signatures, methods, and trait implementations in rustdoc.

:::tip Guide or API?

Use the linked guides to learn when and why to use each collection. Use the Rust API reference when you need exact type signatures, trait bounds, or the complete set of available methods.

:::

## Choose a Live Collection

| Collection | Data model | Typical uses | Guide | Rust API |
| --- | --- | --- | --- | --- |
| `Stream<T>` | A sequence of values arriving over time | Requests, events, and logs | [Streams](../streaming-data/streams.md) | [`Stream`](rust:hydro_lang::live_collections::Stream) |
| `KeyedStream<K, V>` | An independently ordered stream for each key | Concurrent clients, grouped events, and partitioned processing | [Keyed Streams](../streaming-data/keyed-streams.mdx) | [`KeyedStream`](rust:hydro_lang::live_collections::KeyedStream) |
| `KeyedSingleton<K, V>` | One live value for each key | Request/response data and mutable per-key state | [Streaming Keyed Singletons](../streaming-data/keyed-singletons.mdx) and [Keyed State](../state-management/keyed-state.md) | [`KeyedSingleton`](rust:hydro_lang::live_collections::KeyedSingleton) |
| `Singleton<T>` | One value that may change over time | Counters, aggregates, and snapshots | [Singletons and Optionals](../state-management/singletons-optionals.md) | [`Singleton`](rust:hydro_lang::live_collections::Singleton) |
| `Optional<T>` | Zero or one value that may change over time | Filtered state and aggregations that may have no result | [Singletons and Optionals](../state-management/singletons-optionals.md) | [`Optional`](rust:hydro_lang::live_collections::Optional) |

For a conceptual introduction to how these types relate, see [Live Collections](../introduction/live-collections.md).

## Guarantees Encoded in Types

Live Collection type parameters record distributed-systems guarantees. They determine which operations are safe and therefore which methods are available.

| Guarantee | Marker types | Learn more |
| --- | --- | --- |
| Whether a collection is final or may continue changing | [`Bounded`](rust:hydro_lang::live_collections::boundedness::Bounded), [`Unbounded`](rust:hydro_lang::live_collections::boundedness::Unbounded) | [Bounded and Unbounded Types](../correctness/bounded-unbounded.md) |
| Whether stream order is deterministic | [`TotalOrder`](rust:hydro_lang::live_collections::stream::TotalOrder), [`NoOrder`](rust:hydro_lang::live_collections::stream::NoOrder) | [Stream Ordering and Determinism](../streaming-data/streams.md#stream-ordering-and-determinism) |
| Whether stream elements may be retried | [`ExactlyOnce`](rust:hydro_lang::live_collections::stream::ExactlyOnce), [`AtLeastOnce`](rust:hydro_lang::live_collections::stream::AtLeastOnce) | [Streams](../streaming-data/streams.md) |
| Whether a singleton evolves monotonically | [`Monotonic`](rust:hydro_lang::live_collections::singleton::Monotonic) | [Singleton Boundedness and Monotonicity](../state-management/singletons-optionals.md#boundedness-and-monotonicity) |
| How keys and values in keyed state may change | [`BoundedValue`](rust:hydro_lang::live_collections::keyed_singleton::BoundedValue), [`MonotonicValue`](rust:hydro_lang::live_collections::keyed_singleton::MonotonicValue), [`MonotonicKeys`](rust:hydro_lang::live_collections::keyed_singleton::MonotonicKeys) | [Streaming Keyed Singletons](../streaming-data/keyed-singletons.mdx) |

## Slices and Atomicity

The [`sliced`](rust:hydro_lang::live_collections::sliced) APIs reveal bounded batches or snapshots of live collections so they can be safely observed and combined. Start with [Slice Blocks](../state-management/slices.mdx), then see [Atomic Collections](../atomic-collections.mdx) when a service requires read-after-write consistency.

## Complete Rust API

Browse the complete [`hydro_lang::live_collections`](rust:hydro_lang::live_collections) module for supporting modules and every public item, or open the full [`hydro_lang`](rust:hydro_lang) crate documentation.

The API links on this site are generated from the same source revision as the Hydro guide. Documentation for published crate versions is also available on [docs.rs](https://docs.rs/hydro_lang).
