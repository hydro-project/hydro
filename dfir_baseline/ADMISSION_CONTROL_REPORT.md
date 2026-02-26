# Admission Control Implementation Report

## Overview

This report documents the admission control fix implemented in `server_single_stage.rs` that prevents metastable collapse, contrasted with the vulnerable unbounded approach in `server_unbounded.rs`.

## The Problem: Unbounded Channels Enable Metastable Collapse

DFIR's canonical pattern for external communication uses `dfir_rs::util::unbounded_channel()`:

```rust
// From dfir_rs/src/util/mod.rs (lines 57-64)
pub fn unbounded_channel<T>() -> (
    tokio::sync::mpsc::UnboundedSender<T>,
    tokio_stream::wrappers::UnboundedReceiverStream<T>,
) {
    let (send, recv) = tokio::sync::mpsc::unbounded_channel();
    let recv = tokio_stream::wrappers::UnboundedReceiverStream::new(recv);
    (send, recv)
}
```

This is used throughout DFIR for:
- Handoff buffers between strata (see `hydro_lang/src/sim/builder.rs`)
- External input to dataflows (see `dfir_rs/examples/`)
- Inter-process communication

The problem: **unbounded channels accept work without backpressure**. During overload:
1. Requests pile up in the channel faster than they can be processed
2. By the time requests are processed, clients have already timed out
3. Responses go to disconnected clients ("stale responses")
4. Clients retry, adding MORE requests to the already-overloaded queue
5. Positive feedback loop → metastable collapse

## Reference Code: DFIR TCP Networking Pattern

Our implementation follows the pattern from `dfir_rs/src/util/tcp.rs`:

```rust
// From dfir_rs/src/util/tcp.rs - bind_tcp() function (lines 77-140)
spawn_local(async move {
    let mut peers_send = HashMap::new();
    let mut peers_recv = StreamMap::<SocketAddr, FramedRead<...>>::new();

    loop {
        select! {
            biased;
            // Send outgoing messages
            msg_send = recv_egress.next() => { ... }
            // Receive incoming messages  
            msg_recv = peers_recv.next(), if !peers_recv.is_empty() => { ... }
            // Accept new clients
            new_peer = listener.accept() => { ... }
        }
    }
});
```

Key patterns adopted:
- `select!` with `biased` for priority ordering
- `StreamMap` for multiplexed peer connections
- `HashMap` for peer write halves
- Non-blocking, event-driven I/O

## Side-by-Side Comparison

### Vulnerable: server_unbounded.rs

```rust
// NO admission control - always accept
let _ = request_tx.send(request);
```

The unbounded server blindly forwards every request to the DFIR pipeline. Under load:
- Channel depth grows without bound
- Processing latency increases proportionally to queue depth
- All responses become stale (clients timed out)
- System cannot recover even after load returns to baseline

### Fixed: server_single_stage.rs

```rust
// Check queue depth before accepting request
let sent = requests_sent.load(Ordering::Relaxed);
let received = requests_received.load(Ordering::Relaxed);
let depth = sent.saturating_sub(received);

if depth >= max_queue_depth {
    // Server overloaded - reject request immediately
    let rejection = Response::rejected(request.id);
    write_half.write_all(&rejection_bytes).await?;
    rejected_requests.fetch_add(1, Ordering::Relaxed);
    continue;
}

// Only accept if queue has capacity
request_to_peer.insert(request.id, peer_addr);
requests_sent.fetch_add(1, Ordering::Relaxed);
request_tx.send(request)?;
```

The admission control server:
1. Tracks queue depth via atomic counters (sent - received)
2. Rejects requests when depth exceeds `MAX_QUEUE_DEPTH`
3. Sends immediate rejection response to client
4. Only accepts requests when there's processing capacity

## How Bounded Queue Prevents Collapse

```
                    UNBOUNDED                          BOUNDED (Admission Control)
                    
Load Burst:         Requests pile up                   Excess requests rejected
                    Queue: [1,2,3,4,5,6,7,8,9,10...]   Queue: [1,2,3,4,5] (max 5)
                    
Processing:         All requests stale                 Fresh requests processed
                    Latency: unbounded                 Latency: bounded by queue depth
                    
Client Behavior:    Timeout → Retry → More load       Rejection → Retry elsewhere
                                                      OR client backs off
                    
Recovery:           Cannot recover (queue too deep)   Immediate recovery
                    Positive feedback loop            No feedback loop
```

The key insight: **rejection is better than delayed acceptance**. A rejected request:
- Gets immediate feedback to the client
- Doesn't consume server resources
- Doesn't contribute to queue depth
- Allows the client to make informed retry decisions

## Metrics Tracking

The admission control server tracks:
- `requests_sent`: Requests accepted into the queue
- `requests_received`: Requests dequeued by DFIR pipeline
- `stale_responses`: Responses for disconnected clients
- `rejected_requests`: Requests rejected due to overload

These are logged periodically (not per-request) to avoid output spam.

## Configuration

```bash
MAX_QUEUE_DEPTH=10  # Default: reject when >10 requests queued
```

The optimal queue depth depends on:
- Think time (processing latency)
- Client timeout threshold
- Acceptable latency variance

Rule of thumb: `MAX_QUEUE_DEPTH ≈ (client_timeout / think_time) - 1`

## Conclusion

The admission control pattern transforms an unbounded, collapse-prone system into a bounded, stable one. By rejecting excess load at the edge rather than accepting it into an unbounded queue, we break the positive feedback loop that causes metastable failures.

This is a general pattern applicable to any system using DFIR's unbounded channels for external input.
