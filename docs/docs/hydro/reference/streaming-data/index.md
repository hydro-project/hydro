# Streaming Data
Streaming collections are the backbone of every Hydro service: requests arrive as streaming data, flow through transformations, and leave as streaming responses. This section covers the live collection types that represent data **in motion** — new elements continually arriving, with each element processed independently.

At first glance, this makes Hydro look like a streaming analytics framework such as Flink or Kafka Streams. But Hydro's streaming collections are designed to model the **request/response interactions of traditional services**. Rather than writing an RPC handler that is invoked once per request, you declare how the entire stream of requests is transformed into a stream of responses. The result is the same service behavior, but with a program structure that Hydro can type-check for distributed correctness properties like [determinism](../correctness/determinism.md) and deploy across many machines.

Hydro provides three streaming collection types, each capturing a different shape of data in motion:
- **[Streams](./streams.md)**: a single sequence of elements arriving over time, like requests from one client or a totally-ordered event log. The `Stream` type tracks whether elements have a deterministic order and whether they may be duplicated by retries.
- **[Keyed Streams](./keyed-streams.mdx)**: many independent streams, one per key, such as requests from many concurrent clients. Elements within each key preserve their order, while elements across keys can interleave arbitrarily.
- **[Keyed Singletons](./keyed-singletons.mdx)**: exactly one **immutable** value per key, with new keys arriving over time. This is the natural shape of request/response data: each request key maps to one request payload on the way in, and one response on the way out.

All three types can be sent between [locations](../locations/index.md) (processes and clusters), with the type system tracking how the network affects ordering and retries.

:::tip

Streaming data is about elements that are processed once and flow onward. When you need values that are **updated in place** — counters, caches, session state — you'll aggregate streaming data into state collections like `Singleton`. See [State Management](../state-management/index.md).

:::
