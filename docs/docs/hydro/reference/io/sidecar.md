---
sidebar_position: 1
---

# Sidecars
Inevitably, a non-trivial application will need to integrate with an existing library that is not written in Hydro. The `sidecar_bidi` API provides a low-level generalizable escape hatch which can be used to wrap existing code so that it can be ergonomically be used with Hydro's dataflow programming model.

The right way to think about the sidecar API is that it spawns a separate task that runs in the background. It has two channels, one into it and one out of it. You control what the background task does, how it processes the requests it receives, and what it sends on the output channel. The background task is persistent for the life of the Hydro process, this makes it the right place to put things like file handles, database handles, operating system resources, etc.

## The `sidecar_bidi` API
Both [`Process`](../locations/processes.md) and [`Cluster`](../locations/clusters.md) locations provide the `sidecar_bidi` method (it is defined on the `Location` trait). You call it as `process.sidecar_bidi::<InT, OutT, _>(q!(|| { ... }))`, passing a closure that is run once, when the location starts up, and returns a `(Stream<InT>, Sink<OutT>)` pair.

The two type parameters describe the boundary:
- `InT` is the type of items the sidecar pushes _into_ the dataflow. Hydro reads these from the `Stream` you return.
- `OutT` is the type of items the dataflow sends _out_ to the sidecar. Hydro writes these to the `Sink` you return.

`sidecar_bidi` returns two things:
- `inbound`, a `Stream<InT>` carrying the items your sidecar produced. You transform it like any other Hydro stream.
- `response_handle`, a [`ForwardHandle`](pathname:///rustdoc/hydro_lang/) that expects the outbound `Stream<OutT>`. Once you have computed the stream of responses, you hand it back with `response_handle.complete(...)`.

:::info

The closure runs on the deployed machine, not on your laptop during planning, so it is wrapped in `q!(...)` like other runtime code in Hydro. The `Stream` and `Sink` you return can be any concrete types that implement the `futures` `Stream` and `Sink` traits. You own all buffering, backpressure, and lifecycle inside the sidecar, and Hydro only sees the two ends.

:::

Here is a minimal sidecar that echoes items back into the dataflow. It uses a pair of `tokio` mpsc channels: one carries items into the dataflow, the other carries responses back out. We send a greeting out through the response handle, the sidecar echoes it back on `inbound`, and we transform it like any other Hydro stream:

```rust
# use hydro_lang::prelude::*;
# let mut flow = FlowBuilder::new();
# let process = flow.process::<()>();
let (inbound, response_handle) = process.sidecar_bidi::<String, String, _>(q!(|| {
    let (to_df_tx, to_df_rx) = tokio::sync::mpsc::channel::<String>(16);
    let (from_df_tx, mut from_df_rx) = tokio::sync::mpsc::channel::<String>(16);

    // The sidecar task: forward each item the dataflow sends out
    // back into the dataflow as new input.
    tokio::spawn(async move {
        while let Some(msg) = from_df_rx.recv().await {
            to_df_tx.send(msg).await.ok();
        }
    });

    // Hand the framework-facing ends back to Hydro.
    let stream = tokio_stream::wrappers::ReceiverStream::new(to_df_rx);
    let sink = tokio_util::sync::PollSender::new(from_df_tx);
    (stream, sink)
}));

// Send a greeting out to the sidecar via the response handle...
let greetings = process.source_stream(q!(futures::stream::iter(["hello".to_string()])));
response_handle.complete(greetings);

// ...the sidecar echoes it back on `inbound`, which we transform
// like any other Hydro stream.
let echoed = inbound.map(q!(|msg: String| format!("echo: {}", msg)));
# let _ = echoed;
# let _ = flow.with_default_optimize::<hydro_lang::deploy::HydroDeploy>();
```

:::info

Because the closure returns a plain `(Stream, Sink)` pair, the sidecar can also be _one-directional_ in practice: an ingest-only source can return a `Sink` that is never written to, and an output-only sink can return an empty `Stream`. The `bidi` API simply gives you both directions when you need them.

:::

## When to reach for a sidecar
Sidecars are the right tool whenever you need to connect Hydro to code that already exists and that you would rather not reimplement as a dataflow:

- **Databases and stateful stores**: SQLite, Postgres, Redis, or any driver.
- **Existing servers and protocols**: wrap a TCP/HTTP/gRPC endpoint so incoming requests become a stream and responses flow back out.
- **Third-party clients**: message-queue consumers, cloud SDK calls, or anything else exposed as an `async` interface.

If instead you are connecting Hydro programs to each other then you should instead use the networking APIs in [Locations and Networking](../locations/index.md).
