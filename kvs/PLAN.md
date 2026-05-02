# Plan / Design Journal: Smithy HTTP + Hydro Integration

This document walks through the design decisions behind the Smithy HTTP
ingress in this project, from the initial hand-rolled approach through
the current `bidi_external_sidecar` primitive. It's written as a design
journal so future readers can follow the reasoning — including what was
tried and rejected.

## Status (current)

- **Phase 1 (shipped, later superseded)**: extracted a reusable
  `smithy_http_port` module that factored the hand-rolled HTTP parser
  out of `distributed_kvs`. Still layered on top of
  `bidi_external_many_bytes`; the Smithy contract governed only the
  response body shape.
- **Phase 2 (shipped, later superseded)**: added a generic
  `Location::local_sidecar_bidi` primitive to `hydro_lang` that gave
  users an mpsc-bridged typed channel to a sidecar, and rewrote
  `smithy_http_port::kvs_sidecar` as a real tower-smithy service behind
  hyper. Broke Hydro Deploy's port-exposure because the primitive had no
  relationship with `External`.
- **Phase 2.5 (current, shipped)**: replaced `local_sidecar_bidi` with
  `Location::bidi_external_sidecar`, a fused primitive that both exposes
  an external-facing TCP port via Hydro Deploy *and* hands the bound
  `TcpListener` to a user-supplied sidecar, with an mpsc bridge to the
  dataflow. `External` correctly models HTTP clients. `smithy_http_port`
  is now a Smithy-specific sidecar implementation, not a generic helper.

## Context: what the Smithy SDK gives us

The `HydroProjectDemoAppServerSdk` package emits a generated Rust server
SDK (`amzn-hydro-kvs-protocol`) from the Smithy model. Public entry
points include:

- `input::{PutKeyInput, GetKeyInput}` with `FromRequest<RestJson1, B>`
  impls (async, enforces `@required` / `@httpLabel` / `@httpPayload`).
- `output::{PutKeyOutput, GetKeyOutput}` with `::builder()` + `IntoResponse<RestJson1>` impls.
- `error::{PutKeyError, GetKeyError}` with restJson1-compliant response
  generation (sets `x-amzn-errortype`, chooses correct status codes).
- `HydroKvs::builder(config).put_key(h).get_key(h).build()` — produces a
  tower `Service<http::Request<Body>>` that can be handed to hyper.

The lower-level `protocol_serde::*` codegen functions are
`pub(crate)` and not directly callable; we use the public trait impls.

The smithy-rs codegen currently emits references to
`aws-smithy-legacy-http-server` (http-0.2-based). A migration path to
`amzn-aws-smithy-http-server` with `http-1x` exists and should be taken
in a follow-up.

## Three positions we considered

### Option A: Shallow integration

Use the SDK only for output-struct construction + restJson1 body
serialization (`PutKeyOutput::builder()` + `SerializeConfigured`). Keep
request parsing and HTTP framing hand-rolled.

**Pros.** Minimal dependencies. Small glue code.

**Cons.** Request shape isn't contract-bound — `@required` etc. not
enforced. Content-Type checking, error responses, status codes all
hand-written. Errors aren't restJson1-compliant.

### Option B: Tower Service in-process, request parse via `FromRequest`

Keep Hydro owning the socket but reach into the SDK's public
`FromRequest` and `IntoResponse` trait impls: parse each request via
`<PutKeyInput as FromRequest<RestJson1, _>>::from_request(req).await`,
serialize the response via `output.into_response()`.

**Pros.** Inputs, outputs, and error shapes all governed by Smithy.

**Cons.** We still need something to turn a byte stream into an
`http::Request<Body>`. `FromRequest` is async, but the public path goes
through hyper anyway; we might as well use hyper and get a tower
service.

### Option C: Full tower service alongside the dataflow ← **what we implemented**

Run the smithy-rs-generated tower service on each router. The handlers
bridge each request into the Hydro dataflow via a per-request oneshot
correlation channel, keyed by a monotonically-increasing request id.
The dataflow sees typed `(u64, KvsCommand)` / `(u64, KvsResponse)`
streams. Hydro Deploy binds the TCP listener and exposes the port
through Docker/ECS port mapping.

**Pros.** Full Smithy contract coverage (requests, responses, errors,
restJson1 compliance). Matches the canonical smithy-rs server pattern.
External clients are modeled as `External` in the Hydro graph, which
matches Hydro's data model.

**Cons.** Requires a new Hydro primitive (`bidi_external_sidecar`)
that fuses port exposure with sidecar plumbing. Adds a new IR root
(`HydroRoot::ExposeExternalListener`) and a new `Deploy` trait method
(`e2m_listener_bind`).

## Implementation: Phase 2.5 / Option C

### In `hydro_lang` (the HydroProjectHydro workspace)

**New IR root**: `HydroRoot::ExposeExternalListener { external_key,
port_id, cluster_key, op_metadata }` — a no-DFIR root that participates
in `compile_network` purely to drive port exposure + TcpListener bind.

**New Deploy trait method**: `Deploy::e2m_listener_bind(extra_stmts,
c2, c2_port, shared_handle) -> syn::Expr`. Default: `todo!()`.
Overridden on `DockerDeploy` and `EcsDeploy` to push the port into
`exposed_ports` (triggering Docker `--publish` / ECS port mapping) and
emit an extra_stmt of shape:

```rust
let __hydro_deploy_many_{ext}_{port}_listener =
    ::std::cell::RefCell::new(Some(
        tokio::net::TcpListener::bind("0.0.0.0:{port}").await.unwrap()
    ));
```

**New Location method**: `Location::bidi_external_sidecar<Ext, InT, OutT, F>(&External<Ext>, sidecar: impl QuotedWithContext<...>) -> (ExternalBytesPort<Many>, Stream<(u64, InT)>, ForwardHandle<Stream<(u64, OutT)>>)`.
Pushes `ExposeExternalListener` onto the IR, emits mpsc setup extra_stmts,
registers the user's sidecar future, and returns the port + typed streams.

**Removed**: `local_sidecar_bidi` (Phase 2's standalone sidecar primitive).

### In HydroProjectDemoApp

- `src/protocol.rs` — typed `KvsCommand`, `NodeCommand`, `KvsResponse`
  (unchanged from Phase 1).
- `src/proto_codec.rs` — prost/protobuf codec for inter-node traffic
  (unchanged).
- `src/smithy_http_port.rs` — `kvs_sidecar(listener, cmds_tx, resp_rx)`.
  Builds a tower `HydroKvs` service, spawns a drain task for responses,
  hands the pre-bound `TcpListener` to `hyper::server::conn::AddrIncoming::from_listener`.
- `src/lib.rs`:
  - `distributed_kvs(commands, routers, nodes) -> responses` — pure
    dataflow, sim-testable via `fuzz_distributed_kvs`.
  - `complete_distributed_kvs(&external, &routers, &nodes) -> ExternalBytesPort<Many>`
    — production wrapper that calls `routers.bidi_external_sidecar(...)`
    with `kvs_sidecar` and feeds its outputs into `distributed_kvs`.
- `examples/kvs.rs` — each deploy path (`docker_deploy`, `export_manifest`,
  `docker_rebalance`) constructs a `builder.external()`, passes it to
  `setup_kvs`, threads the returned `bidi_port` into
  `get_all_tcp_endpoints` for host-side endpoint discovery.
- `src/smithy_json.rs` — **deleted**. Output construction happens
  directly inside the sidecar handlers via `PutKeyOutput::builder()` +
  `IntoResponse`; no separate JSON helper needed.

## Future work

### http 1.x migration

The codegen currently emits `aws-smithy-legacy-http-server`
(http-0.2-based). Per the smithy-rs migration guide, the new path is
`amzn-aws-smithy-http-server` with the `http-1x` feature. When we
regenerate against a newer smithy-rs, the `FromRequest` / `IntoResponse`
trait paths move. Should be a mechanical update; the Phase 2.5
architecture is otherwise unaffected.

### Upstreaming `bidi_external_sidecar` to Hydro

The primitive is designed to be upstreamable: it has no Smithy-specific
coupling (smithy-rs lives entirely in the user's crate). The changes
in `hydro_lang` are a single new IR root, one new Deploy trait method,
and one new Location method. Proposing these as an RFC to the Hydro
team is a natural follow-up — this repository can serve as the worked
reference implementation.

### Input validation error paths

Today the sidecar handlers panic on unexpected response kinds (which
should be unreachable). If we add constraint traits to the Smithy model
(`@length`, `@pattern`, etc.), the generated `FromRequest` will reject
invalid requests with a `ValidationException` and smithy-rs will render
a restJson1 error response automatically — no handler changes required.
This is a genuine "Smithy contract for free" win we get by using the
tower service path.

### Deeper simulation coverage

`fuzz_distributed_kvs` exercises the pure dataflow. It does not
exercise the sidecar's oneshot routing or the hyper framing. A
follow-up could add a sim harness that drives
`bidi_external_sidecar`'s mpsc channels directly (skipping hyper) to
cover the handler + oneshot paths.
