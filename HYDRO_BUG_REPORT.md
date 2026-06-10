# Bug Report: DemuxMap panics surviving nodes when a cluster peer dies

## Summary

When a node in a Hydro cluster dies (process crash, network partition, EC2 instance stop), all surviving nodes that were sending messages to the dead peer panic with `BrokenPipe`. This makes it impossible to build fault-tolerant protocols using Hydro's cluster primitives without patching the `sinktools` crate.

## Steps to Reproduce

1. Create a 3-node cluster where nodes broadcast messages to each other via `.broadcast()` or `.demux()`
2. Deploy locally or on EC2
3. Kill one node (e.g. `Service::stop()`)
4. Observe: surviving nodes panic within seconds

Minimal test: `tests/demux_crash_repro.rs` in this repo.

## Expected Behavior

Surviving nodes should continue operating. Messages to the dead peer should be silently dropped. This is what `TCP.fail_stop()` semantics imply — "a prefix of messages is delivered, then the channel stops."

## Actual Behavior

```
thread 'main' panicked at 'BrokenPipe: ...'
```

The surviving nodes crash, taking down the entire cluster.

## Root Cause

Three components interact to create this behavior:

### 1. `sinktools/src/demux_map.rs` — Error propagation

```rust
// Upstream poll_ready (simplified):
fn poll_ready(...) -> Poll<Result<(), Self::Error>> {
    self.sinks.values_mut().try_fold(Poll::Ready(()), |poll, sink| {
        ready_both!(poll, Pin::new(sink).poll_ready(cx)?); // <-- `?` propagates errors
        Poll::Ready(Ok(()))
    })
}
```

When ANY sink returns `Err(BrokenPipe)`, the `?` operator propagates it to the caller.

### 2. `dfir_lang/src/graph/ops/dest_sink.rs` — Panic on error

```rust
fn sink_guard<Sink, Item>(sink: Sink) -> ...
where Sink::Error: Debug,
{
    sink.sink_map_err(|e| panic!("{:?}", e))  // <-- Any error panics the process
}
```

`dest_sink` wraps all output sinks with a panic-on-error guard. Any error from DemuxMap becomes a process crash.

### 3. The inconsistency with `MultiConnectionSink`

`hydro_deploy_integration/src/multi_connection.rs` already handles this correctly:

```rust
self.connection_sinks.retain(|_, sink| match sink.as_mut().poll_ready(cx) {
    Poll::Ready(Ok(())) => true,
    Poll::Ready(Err(_)) => false,  // <-- gracefully drops dead connections
    Poll::Pending => { any_pending = true; true }
});
```

The TCP-level multiplexer drops dead connections gracefully, but the dataflow-level multiplexer (`DemuxMap`) propagates errors fatally. The design intent (crash-tolerant) exists at the lower layer but isn't carried through to the higher layer.

## Proposed Fix

`DemuxMap` should use `retain()` to drop errored sinks, matching `MultiConnectionSink`'s pattern:

```rust
fn poll_ready(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
    let me = self.get_mut();
    me.sinks.retain(|_key, sink| {
        match Pin::new(sink).poll_ready(cx) {
            Poll::Ready(Ok(())) => true,
            Poll::Ready(Err(_)) => false,  // drop dead sinks
            Poll::Pending => true,          // keep but don't block
        }
    });
    Poll::Ready(Ok(()))
}

fn start_send(self: Pin<&mut Self>, item: (Key, Item)) -> Result<(), Self::Error> {
    if let Some(sink) = self.get_mut().sinks.get_mut(&item.0) {
        Pin::new(sink).start_send(item.1)
    } else {
        Ok(()) // silently drop for removed sinks
    }
}

fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
    let me = self.get_mut();
    me.sinks.retain(|_key, sink| {
        match Pin::new(sink).poll_flush(cx) {
            Poll::Ready(Ok(())) => true,
            Poll::Ready(Err(_)) => false,
            Poll::Pending => true,
        }
    });
    Poll::Ready(Ok(()))
}
```

Additionally, `start_send` should not panic on missing keys — it should silently drop, since the key may have been removed by a prior `poll_ready`/`poll_flush` due to a dead peer.

## Additional Consideration: `dest_sink` panic behavior

Even with the DemuxMap fix, `dest_sink`'s `sink_map_err(|e| panic!(...))` is aggressive. For `fail_stop` channels, errors should arguably be swallowed (the channel "stops") rather than panicking the process. However, fixing DemuxMap alone is sufficient since the error never reaches dest_sink if DemuxMap drops the dead sink first.

## Impact

- **All cluster protocols** using `.broadcast()` or `.demux()` with `TCP.fail_stop()` are affected
- **Paxos, Chain Replication, Primary-Backup** — any protocol that expects `f` failures to be tolerated will instead crash the entire cluster on the first failure
- **hydro_test examples** (paxos_bench, etc.) are not affected when run locally because the test framework doesn't kill individual cluster members during operation

## Environment

- Hydro commit: `5aa43241a5` (pinned in lego-replicate)
- Rust: 1.92.0
- Platform: aarch64-apple-darwin (also reproduced on x86_64-unknown-linux-gnu via EC2)
- Affected crate: `sinktools` (DemuxMap), triggered via `dfir_lang` (dest_sink)

## Workaround

Patch `sinktools/src/demux_map.rs` as described above. We have been running with this patch in production (EC2 failover tests passing with node kills).
