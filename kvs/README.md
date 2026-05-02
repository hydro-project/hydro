# HydroProjectDemoApp

An end-to-end demonstration of running a non-trivial stateful service on top of [Hydro](https://hydro.run/) — a distributed key-value store with multiple client-facing protocol ingresses, deployable via Docker locally and via CDK on ECS Fargate.

The goal of this project is to be a **worked reference** for builders who want to put a new service together on Hydro. It exercises Hydro's external-port and sidecar primitives, gRPC and WebSocket ingresses on the same dataflow, AppConfig-driven feature flags, and Docker/ECS deployment. Every pattern here is production-shaped; the code is intended to be copied and adapted.

## What the service does

- **Distributed key-value store** with rendezvous-hash replication across a cluster of storage nodes, a router tier in front, and a two-phase read-before-write protocol backed by vector-clocked multi-value CRDT registers.
- **Two client-facing ingresses in the same process**: a tonic/gRPC server and a tokio-tungstenite WebSocket server, both feeding the same `distributed_kvs` dataflow and sharing the same per-key state.
- **Runtime-toggleable feature flags** via AWS AppConfig, exposed to the dataflow as an `Unbounded` `Singleton<bool>`.
- **CDK-driven deployment** to ECS Fargate behind an NLB, with Docker-based local development and integration tests.

## Architecture

See **`ARCHITECTURE.md`** for the full system design — data model, request flow, wire protocols, the `bidi_external_sidecar` primitive, deployment topology, and testing strategy. `ARCHITECTURE.md` must always reflect the current architecture; update it alongside any change to system structure, dataflow, deployment, wire protocol, or component responsibilities.

See **`PLAN.md`** for the design journal covering how the Hydro integration evolved (options considered, decisions made, what was tried and rejected).

## Package layout (Brazil workspace)

The workspace contains several Brazil packages. Packages must be built in dependency order:

1. `HydroProjectStageleft` — the staged-programming framework the Hydro macros are built on. No intra-workspace deps.
2. `HydroProjectHydro` — Hydro itself. Depends on `HydroProjectStageleft`.
3. `HydroProjectDemoApp` — the Rust dataflow + both sidecars + local examples. The main development target.
4. `HydroProjectDemoAppImageBuild` — packages the release binary into a Docker image.
5. `HydroProjectDemoAppCDK` — CDK stack for the ECS/NLB/AppConfig infrastructure.
6. `HydroProjectDemoAppTests` — Hydra post-deploy integration test harness.


## Building

Run from each package's `src/<PackageName>/` directory.

```sh
cd src/HydroProjectStageleft && brazil-build quick
cd src/HydroProjectHydro && brazil-build quick
cd src/HydroProjectDemoApp && brazil-build quick
```

- `brazil-build quick` — fast compile-only build (no tests, no linting).
- `brazil-build` — compile and run tests.
- `brazil-build release` — full build with tests + coverage + linting. Significantly slower; avoid in the inner loop.
- For `HydroProjectHydro`, always prefer `brazil-build quick` — the release build runs long trybuild snapshot tests.
- For the CDK package, use plain `brazil-build` (no `quick` script exists).
- The DemoApp's custom build script (`cargo-brazil-hydro`) does **not** accept extra args like `--features`. It will fail with `unexpected argument '--features'`.
- The build script automatically runs a `export` step and compiles the stageleft trybuild binaries.

### Brazil Rust workspaces — do not commit `Cargo.lock`

`cargo brazil` manages inter-package dependency resolution via each package's Brazil `Config` file. Only `Cargo.toml` matters for declaring dependencies; `Cargo.lock` is generated at build time and must not be checked in.

### Adding crates.io dependencies — just edit `Cargo.toml`

Amazon mirrors the public crates.io registry internally as `third-party-crates-io`. CargoBrazil resolves third-party Rust dependencies through this mirror automatically. **To add a new crates.io dependency, just add it to `Cargo.toml` like you would on any normal Cargo project** — do not search the version set, do not run `brazil workspace merge`, do not touch `Config`. Pin the version (e.g., `tokio-tungstenite = "0.21"`) and go. This applies to any crate published on crates.io, including transitive deps.

The only time the Brazil `Config` file matters for Rust dependencies is when depending on *another Brazil package* in the same workspace (an intra-workspace dep), not a crates.io crate.

## Running the distributed KVS

### Localhost mode (fastest, no Docker)

```sh
cd src/HydroProjectDemoApp
cargo run --frozen --no-default-features --example kvs -- --mode localhost
```

Spawns the router + cluster nodes as local OS processes.

### Docker mode

```sh
docker rm -f $(docker ps -aq) 2>/dev/null
docker network prune -f 2>/dev/null
cd src/HydroProjectDemoApp
cargo run --frozen --features docker --example kvs -- --mode docker
```

Builds a container image per Hydro binary and runs the full distributed system in Docker. Closest local approximation to ECS.

### ECS CDK manifest

```sh
cargo run --frozen --features ecs --example kvs -- --mode export --output ./hydro-assets
```

Produces `hydro-assets/hydro-manifest.json` and the compiled binaries. `brazil-build release` does this automatically.


## Testing

### Sim tests

The Hydro simulator exhaustively explores non-deterministic interleavings of the dataflow. All `sim_*` tests in `src/tests.rs` cover the core dataflow: storage layer, quorum merging, routing, and pure helper functions.

```sh
cd src/HydroProjectDemoApp
cargo test --frozen sim_
```

### Fuzz test

`fuzz_distributed_kvs` drives `distributed_kvs` (the pure-dataflow function without the gRPC/WS ingresses) under randomized sim exploration. 50,000 iterations.

### Docker end-to-end test

```sh
cd src/HydroProjectDemoApp
cargo test --frozen --example kvs -- test_kvs_docker
```

Do NOT set `CARGO_HOME` or run `cargo brazil configure` first — just `cargo test` directly.

The test (defined in `examples/kvs.rs`):

1. Cleans up any stale `hy-*` Docker containers and `kvs_*` networks.
2. Spawns `cargo run --example kvs -- --mode docker-e2e-test` as a child process.
3. Deploys 3 routers + 7 storage nodes as Docker containers.
4. Runs the full KVS test suite against the **gRPC** ingress on every router (puts, gets, overwrites, missing keys, 100-key distribution check).
5. Runs the **WebSocket** parity suite on every router: writes via WS, reads back via WS, and — crucially — reads the same key via the gRPC endpoint on the same router. This cross-protocol read-back confirms that both ingresses feed a single `distributed_kvs` state, rather than running as two independent stacks.
6. Waits for the `docker e2e test completed successfully` sentinel on the child's stdout, then tears the cluster down.

Typical run: ~90 seconds after warm builds.

### Debugging spinning sim tests

When a sim test hangs (spins forever):

1. Run with a timeout and debug logging:
   ```sh
   cd src/HydroProjectDemoApp
   timeout 90 env RUST_LOG=debug cargo test --frozen <test_name> -- --ignored --nocapture > /tmp/sim_test_output.log 2>&1
   ```
2. Check the tail of the log. A spinning dataflow shows two or more `run-subgraph` entries alternating endlessly. Their `sg_name` fields list operator ids like `persist#133`, `cross_singleton#133`, etc.
3. Map operator ids back to source. The test writes `/tmp/source_map.json` via `built.source_map()`:
   ```python
   import json
   data = json.load(open('/tmp/source_map.json'))
   for e in data:
       if e['stmt_id'] in [133, 134, 135]:
           print(e['stmt_id'])
           for f in e['backtrace']:
               print(f"  {f['fn_name']}  {f['filename']}:{f['line']}")
   ```
4. Identify the feedback loop. A common cause is an `Unbounded` singleton (from `source_iter` + `fold`) feeding a `cross_singleton`: the persist backing the singleton re-emits every tick, triggering the downstream operator, which re-triggers the persist — even when there's no real data to process.


## Hydro coding conventions

Things that have bitten us repeatedly and are worth internalizing before you write new Hydro code.

### `q!()` closures should always be `move`

The `q!` macro rewrites free variables (like `CLUSTER_SELF_ID`) into shadow locals for type-checking. A non-`move` closure tries to borrow those shadows, producing `cannot return reference to temporary value` errors. `move` forces the shadows to be captured by value, which is what you want. Inside `sliced!` blocks this is especially important — without `move`, the macro captures a reference to a temporary and you get `E0515`.

### Constants and const generics inside `q!()`

- Plain constants (`REPLICATION_FACTOR`) are fine.
- **Const generic parameters cannot be used directly** inside `q!()` — the macro doesn't recognize them as capturable identifiers, so the trybuild binary fails with "cannot find value". Bind the const generic to a local at the top of the `q!()` closure:

  ```rust
  fn my_function<'a, const N: usize>(...) {
      .map(q!(move |x| {
          let n = N; // bind const generic so q!() captures it as a local
          // ... use `n` instead of `N`
      }))
  }
  ```

### Calling local functions inside `q!()` needs `self::`

Bare function names don't resolve through stageleft's `FreeVariableWithContext` machinery. Always use `self::`:

```rust
// ❌ Won't compile
.map(q!(|x| helper(x)))

// ✅ Works
.map(q!(|x| self::helper(x)))
```

This applies to functions defined in the same module (or re-exported into scope). It does NOT apply to methods, trait methods, or functions from external crates.

### Extract pure functions for unit testability

Logic inside `q!()` closures only runs under the Hydro dataflow runtime — it's awkward to unit-test. Extract the core computation into a regular `pub fn`, call it from the `q!()` closure via `self::`, and write standard `#[test]` unit tests against the extracted function.

```rust
pub fn build_response(cmd: Command, store: &Store) -> Response { ... }

.map(q!(|(cmd, store)| self::build_response(cmd, &store)))

#[test]
fn test_build_response() {
    assert_eq!(build_response(some_cmd, &some_store), expected);
}
```

### How `q!()` actually works (stageleft internals)

`q!()` is a staged-programming construct. It captures code as tokens for code generation AND type-checks the code locally for IDE support, simultaneously.

When you write `q!(|x| x + CLUSTER_SELF_ID.clone())`:

1. The macro walks the AST and identifies free variables (identifiers not bound by `let`, closure param, or function arg). `FreeVariableVisitor` renames each free var `FOO` to `FOO__free`.
2. It generates a `move` outer closure that, for each free variable, calls `FreeVariableWithContext::uninitialized(&FOO, ctx)` to create a shadow local (`FOO__free`) with the correct output type, and `FreeVariableWithContext::to_tokens(FOO, ctx)` to capture tokens for code generation. The captured tokens go into a `QuotedOutput`; the outer closure returns `MaybeUninit::uninit().assume_init()` (unreachable at runtime).
3. The user's rewritten expression goes in an `#[allow(unreachable_code)]` block at the bottom, purely for type-checking / Rust Analyzer support. This is why inner closures in the block must be `move` — the shadow locals are owned by the outer closure and a non-`move` inner closure can't borrow them.
4. Free variables implement `FreeVariableWithContextWithProps<L, Props>` with `type O` (the output type), `to_tokens()`, and `uninitialized()`.

Key source files:

- `HydroProjectStageleft/stageleft_macro/src/quote_impl/mod.rs` — the `q_impl` function.
- `HydroProjectStageleft/stageleft_macro/src/quote_impl/free_variable/mod.rs` — `FreeVariableVisitor`.
- `HydroProjectHydro/hydro_lang/src/location/cluster.rs` — `CLUSTER_SELF_ID` and `ClusterSelfId` impl.


## Pre-commit checks

Before committing Rust changes:

```sh
cd src/HydroProjectDemoApp
cargo clippy --tests
cargo +nightly fmt
```

For CDK changes:

```sh
cd src/HydroProjectDemoAppCDK
npx tsc --noEmit
npx prettier --check '**/*.{js,ts,md}' '!build/**/*' '!dist/**/*'
```

The build fleet runs `prettier-check` on all `*.md` files in the CDK package, so markdown formatting issues will fail the build. Fix warnings and formatting issues before committing.

## Git etiquette

- **Never amend commits that have been pushed.** Once shared, a commit is immutable; fix forward.
- Prefer new commits over `--amend` even for local-only history. CRUX auto-merge may push without explicit user action, so what looks local may already be upstream.
- Always `git fetch` before assuming commits are local-only.
- Do not use destructive commands (`git push --force`, `git reset --hard`, `git clean -fd`) without a clear, ack'd reason.

## Things that bit us (and how we avoided them)

A few hard-won lessons worth calling out explicitly:

### External ports must model actual external processes

HTTP clients talking to the KVS are **external** in the Hydro sense — they live outside the compiled dataflow graph and communicate over TCP. That means they belong behind an `External<HttpClients>` handle with a `bidi_external_*` port, not behind an ad-hoc sidecar that binds its own socket. We initially built a `local_sidecar_bidi` primitive that didn't take an `External`, which broke Docker/ECS port exposure because Hydro Deploy had no awareness of the bound port. The fix was `Location::bidi_external_sidecar`, which fuses port-exposure (via the deploy backend's `e2m_listener_bind` hook) with the typed-channel sidecar bridge. See `PLAN.md` for the full progression.

### The dataflow envelope must carry everything needed for demux — and the sidecar's id is sacred

When we added a second ingress (gRPC), we initially stole high bits of the sidecar-owned `u64` `client_id` to tag which ingress produced a command, planning to strip the bit on the way back. That turned out to be a layering violation *and* a bug.

- **The layering violation**: the sidecar owns its request ids. The ingress-demux layer has no business rewriting them — if the sidecar receives back a different `u64` than it sent out, any correlation the sidecar does against its own state (e.g. a `HashMap<u64, oneshot::Sender>`) silently breaks.
- **The bug**: `distributed_kvs` uses a `READ_PHASE_OFFSET = 1_000_000_000` convention internally, where `client_id < READ_PHASE_OFFSET` means "this is a client response" and `client_id >= READ_PHASE_OFFSET` means "this is a read-phase response that should trigger the write phase" (see `classify_merged_response`). Our first attempt packed the ingress tag into bit 63, which made every gRPC id larger than 1 billion, so every gRPC Get got misclassified as a read-phase response, routed back into the write-phase cycle instead of the client, and the gRPC handler's oneshot never fired — so Gets hung forever. Puts "worked" only because `PutOk` matches the first arm of `classify_merged_response` unconditionally.

The correct fix was to change the envelope type. `distributed_kvs` now takes `KeyedStream<(Ingress, u64), KvsCommand>` and returns `Stream<((Ingress, u64), KvsResponse)>`. Each sidecar is tagged with its own `Ingress` variant (`Grpc`, `Ws`, plus `Test` for sim/fuzz) on the way in, and the ingress demux in `complete_distributed_kvs` matches on the variant on the way out. The sidecar-owned `u64` passes through untouched.

The lesson: **if the dataflow envelope needs to carry transport-level metadata alongside the sidecar's id, make the envelope key a tuple; don't bit-pack into the id**. It's a bigger change — every internal `HashMap<u64, _>`, every inline `(u64, NodeCommand)` type annotation, every sim-test literal — but the resulting layering is clean and adding a new ingress is "add an enum variant," not "pick another free bit and hope it doesn't collide with any existing numeric convention."

### `bidi_external_sidecar` is protocol-agnostic by construction

The primitive gives the user-supplied closure a `tokio::net::TcpListener` and a pair of `(u64, T)` mpsc channels. Anything that can accept a `TcpListener` is a valid sidecar. We've proven this with two protocols that differ in every relevant dimension:

- **gRPC/protobuf** over HTTP/2: `tonic::transport::Server::builder().serve_with_incoming(TcpListenerStream::new(listener))`.
- **WebSocket** over HTTP/1.1: `tokio_tungstenite::accept_async(stream)` per connection.

Both run in the same binary, merge through the same `distributed_kvs` dataflow, and share the same state. If you add a third protocol, the shape of the change is: write another sidecar function, add another `bidi_external_sidecar` call with its own `Ingress` tag, extend the merge + demux. Nothing about `distributed_kvs` or the existing ingresses needs to change.


## Useful links

- [Code Browser](https://code.amazon.com/packages/HydroProjectDemoApp/)
- [Hydro project documentation](https://hydro.run/)
- The `docs/` folder inside `HydroProjectHydro/` — live collections, sliced, reference/stageleft/errors (the errors doc is especially useful when you hit errors inside a `q!()`).

## Related documents in this package

- `ARCHITECTURE.md` — full system design; keep in sync with any structural change.
- `PLAN.md` — design journal for the Hydro integration work.
- `CONTRIBUTING.md` — contribution guidelines.
- `ISSUES.md` — tracked issues, gotchas, and workarounds.
- `AGENTS.md` — concise build/test notes for AI assistants operating in this package.
