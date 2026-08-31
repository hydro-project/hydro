# Finite round-trip deadlock in Hydro's deployed scheduler

A finite `A -> B -> A` network round trip deadlocks once enough bytes are in
flight; the identical one-way `A -> B` never does. Root-caused to the deployed
DFIR scheduler awaiting a process's subgraphs **sequentially within one tick**:
a subgraph blocked on a backpressured output sink suspends the whole tick,
starving the sibling subgraph that drains the peer's input — so two processes
that each must read to unblock the other wedge permanently.

Witness: `hydro_test/src/local/finite_round_trip.rs` (study copy:
`FINITE_ROUNDTRIP_STALL_WITNESS.rs`).

## Symptom

```text
A -> B -> A        result   terminal sent/recv (of N)
156 x 32 B         pass     156 / 156
156 x 192 B        pass     156 / 156
156 x 224 B        DEADLOCK 134 / 0
156 x 1024 B       DEADLOCK ~28 / 0     (deterministic)
2048 x 32 B        DEADLOCK 713 / 45

A -> B (one-way)
156 x 1024 B       pass     156 / 156
2048 x 32 B        pass     2048 / 2048
```

Trigger is **bytes in flight** (~32 KB, ≈ one socket buffer), not item count.
The return leg is required — one-way never deadlocks at any volume tested.

## Proof it is a deadlock, and where it wedges

The witness (`round_trip_probed`) has `inspect` probes on each leg. Driving the
hanging config and reading the deployed processes' `[PROBE]` stdout for **60 s**:

```text
t=1s  .. t=59s   AtoB=17  atB=8  backAtA=0     (identical every second)
```

- Items leave A (`AtoB=17`) and arrive at B (`atB=8`), but the **return leg
  produces zero** (`backAtA=0`) — and every count is byte-for-byte frozen for
  60 s. Not slow, not throttled: no progress at all.
- This is measured *inside the flow* (probes are dataflow `inspect`s), so it is
  not an artifact of the external test receiver.
- Ruled out: single-thread executor starvation (reproduces on
  `#[tokio::test(flavor = "multi_thread")]`); benign backpressure (the forward
  leg delivers to B while the return is pinned at 0 forever — backpressure
  throttles a producer, it cannot pin a live return path to zero).

## Root cause (traced through the generated code)

Process A's flow compiles to **three subgraphs** run **sequentially in one tick
future**. From the generated `main`
(`target/hydro_trybuild/hydro_test/dylib-examples/examples/_process_loc2v1_*.rs`):

```rust
let __dfir_inline_tick = async move |df| {
    let sgid_1v1 = async { ... (pivot_run_sg_1v1)(op_1v1, op_2v1).await; ... };
    InstrumentSubgraph::new(sgid_1v1, ..).await;   // sg1: external -> [AtoB] -> send A->B
    let sgid_2v1 = async { ... };
    InstrumentSubgraph::new(sgid_2v1, ..).await;   // sg2
    let sgid_3v1 = async { ... };
    InstrumentSubgraph::new(sgid_3v1, ..).await;   // sg3: recv B->A -> [backAtA] -> external
    ...
};
```

`subgraph_toposort = [1, 2, 3]`; sg1 (the A->B send) is awaited **before** sg3
(the only reader of the B->A socket). No `join!`/`spawn`/concurrent poll across
subgraphs — one linear future per tick.

Each subgraph is a `SendPush` future. In `dfir_pipes/src/pull/send_push.rs:60-67`
it calls `push.poll_ready()` before pulling and, on `Pending`, suspends the
whole subgraph future:

```rust
match this.push.as_mut().poll_ready(..) {
    PushStep::Done => {}
    PushStep::Pending(_) => return Poll::Pending,   // <-- suspends the tick
}
```

The push is the socket sink (`dfir_pipes/src/push/sink.rs:39-44`): a full send
buffer yields `Poll::Pending` -> `PushStep::Pending`.

Chain:

1. A->B socket send buffer fills.
2. sg1's `poll_ready` returns `Pending`, so `__dfir_inline_tick` returns
   `Pending` **before sg3 is ever polled**.
3. sg3 is the only subgraph that reads the B->A socket, so A stops reading B->A
   whenever A is blocked writing A->B.
4. Symmetric on B. A is blocked writing A->B and won't read B->A; B is blocked
   writing B->A and won't read A->B. Neither buffer can drain because each
   process's only reader is sequenced behind its blocked writer in a
   single-threaded tick. **Cyclic wait -> permanent deadlock.**

Below ~32 KB a buffer never fully fills, so at least one write completes, the
tick finishes, and the reader subgraph runs — which is why small payloads pass.

## Transport note

`TCP.fail_stop().bincode()` selects framing + a failure policy, not the wire. On
this unix localhost the legs run over Unix domain sockets (both processes share
one `LocalhostHost`; `Auto` prefers the Unix strategy). `unix_bytes`/`tcp_bytes`
use the same `Framed<_, LengthDelimitedCodec>` sink, so the deadlock is not
TCP-specific.

## Reproduce

```bash
# controls (pass)
cargo test -p hydro_test local::finite_round_trip::tests:: -- --test-threads=1

# witness: deadlocks; the 20 s internal timeout turns it into a panic so it
# can't wedge the runner
cargo test -p hydro_test \
  local::finite_round_trip::tests::round_trip_156x1kib_stalls \
  -- --exact --ignored --nocapture

# in-flow probe showing where it wedges (AtoB/atB climb, backAtA stays 0)
cargo test -p hydro_test \
  local::finite_round_trip::tests::diag_probe \
  -- --exact --ignored --nocapture
```

## Scope / caveats

- The **failure and its mechanism are proven** (behavioral 60 s-flat probe +
  the generated sequential-subgraph tick + the `poll_ready -> Pending -> tick
  suspends` code path).
- The **fix is not obvious** and is the Hydro team's call: poll a process's
  subgraphs concurrently, or make a sink-`Pending` yield the tick (reschedule)
  rather than suspend it, so sibling input subgraphs still run. Each has
  tradeoffs (ordering, busy-polling, buffering).
- Whether an external-driven finite round trip is a "supported" pattern is also
  their call; the same sequential-tick structure applies to any process whose
  outbound subgraph is toposorted ahead of an inbound one it depends on
  (cluster protocols included), so this is not specific to the external harness.

## Raft

A separate attempt to exhibit this in Raft did **not** cleanly confirm it: large
payloads collapsed commit throughput and a replica died with
`LengthDelimitedCodecError` (a max-frame-size violation, plausibly from
`AppendEntries` packing a growing log suffix) — a different failure, possibly an
artifact of the workload chosen. Not presented as a result.

## Environment

Repo `hydro-clean`, sandbox commit `29a3e64c0d`; Rust 1.96.0 aarch64-apple-darwin;
macOS. No Hydro runtime or consensus code was modified.
