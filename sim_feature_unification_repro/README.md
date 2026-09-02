# Sim host/dylib feature-unification unsoundness repro

Minimal reproduction of a soundness bug in Hydro's simulator: the sim dylib
is compiled with **different dependency features** than the host test binary
whenever a *sibling workspace crate* unifies extra features into the host
build, while the two share Rust data structures across the `dlopen` boundary.

This directory is a standalone cargo workspace (excluded from the root hydro
workspace) with two members:

- **`staged/`** (`sim_repro_staged`) — the smallest possible staged dataflow
  crate (`map(q!(|x| x + 1))`) with one sim test using `sim_input` /
  `sim_output` + `exhaustive()`. Deliberately does **not** carry the
  `tokio = { features = ["full"] }` dev-dependency that `hydro_test` and the
  project template have (which is copied into the trybuild manifest and masks
  this bug for hydro's own test crates).
- **`sibling/`** (`sim_repro_sibling`) — an empty crate whose only purpose is
  `tokio = { version = "1", features = ["full"] }`.

## Reproduction

```bash
cd sim_feature_unification_repro

# 1. Solo invocation: host and dylib resolve identical tokio features -> OK
cargo test -p sim_repro_staged

# 2. Unified invocation: cargo feature unification gives the HOST test binary
#    a tokio compiled with "full" (rt-multi-thread, parking_lot, ...), while
#    the sim dylib generated under target/hydro_trybuild/ is still built
#    against minimal-feature tokio.
#    -> `host_and_dylib_agree_on_tokio_layout` FAILS deterministically on any
#       platform (the two sides disagree on size_of::<tokio::runtime::Handle>:
#       host 16 vs dylib 8);
#    -> additionally UB for every sim test (observed as SIGSEGV on
#       macOS/aarch64)
cargo test --workspace
```

## Mechanism

`create_trybuild()` in `hydro_lang/src/compile/trybuild/generate.rs`
synthesizes the sim dylib's cargo workspace from **the staged crate's
`Cargo.toml` alone** (`dependencies::get_manifest(&source_dir)` +
`Runner::make_manifest`, which merges just that crate's `[dependencies]` and
`[dev-dependencies]`). Defenses exist for *version* skew — the workspace
`Cargo.lock` is copied and `cargo update -w` is run — but the trybuild
workspace re-runs **feature resolution** over its own small graph, and
`features::find()` only forwards the staged crate's *own package features*
(from the test binary's fingerprint JSON), not transitive dependency
features.

Meanwhile the host test binary is built by the user's cargo invocation, where
features unify across *every crate in the invocation*. Any feature a sibling
crate enables on a shared dependency (tokio here) makes the host's copy of
that dependency cfg-diverge from the dylib's copy.

The two copies then share data structures: the generated dylib code creates
`tokio::sync::mpsc::unbounded_channel::<Bytes>()` and hands the
`UnboundedReceiver` across the `dlopen` boundary for the host's `SimReceiver`
to poll. Rust has no stable ABI, so differently-configured compilations
sharing internals is undefined behavior. Note that `hydro_lang`'s own `sim`
feature already pins `dfir_rs/tokio` + `dfir_rs/meta` with the comment
"affects layout of DFIR, thus the ABI" — this repro shows the same hazard via
transitive features that no manifest pin currently covers.

## Observed behavior

| Invocation | Linux x86_64 | macOS aarch64 |
|---|---|---|
| `cargo test -p sim_repro_staged` | pass | pass |
| `cargo test --workspace` | **`host_and_dylib_agree_on_tokio_layout` fails** (other sim tests pass with silent UB) | **`host_and_dylib_agree_on_tokio_layout` fails**; sim tests **SIGSEGV** (observed in the originating project) |

`host_and_dylib_agree_on_tokio_layout` streams
`size_of::<tokio::runtime::Handle>()`, evaluated *inside the dylib* by a
`q!()` closure, out through a sim port and compares it against the host's
value — a direct, platform-independent witness that the two sides disagree
about the layout of types they share.

The macOS crash (from the originating report, hydro-project/infinity#112, an
equivalent workspace shape): host polls a dylib-created
`UnboundedReceiver<Bytes>` and dies in
`tokio::sync::mpsc::block::Block::<Bytes>::is_at_index` with `self == NULL`
(fault address `0x400`), via `SimReceiver::next` →
`UnboundedReceiverStream::poll_next` → `Rx::pop` → `try_advancing_head`.
A green run of the *sim* tests on Linux is *not* evidence of soundness — the
divergence is identical there (the layout test proves it); the transport
misbehavior is just currently silent on that target.

The unified-invocation tokio feature delta observed with this repro
(tokio v1.53.1): host gains `full, parking_lot, process, rt-multi-thread,
signal, signal-hook-registry` over the dylib. Size-probing tokio's public
types under each single extra feature shows `rt-multi-thread` **alone**
changes public type layouts (`runtime::Handle` 8 → 16, `runtime::Runtime`
64 → 80, `time::Sleep` 96 → 112, `net::TcpListener` 32 → 40, because the
runtime scheduler enum gains a multi-thread variant), while `parking_lot`,
`process`, and `signal` individually change none of the probed public types.
Private internals (like the mpsc structures implicated in the macOS crash)
may of course shift under any of them; *any* host-only feature constitutes
the divergence.

## Possible fix directions

- Resolve the trybuild workspace's features from the *host invocation's*
  resolved feature set (e.g. via `cargo metadata`/unit-graph of the running
  build) rather than from the staged crate's manifest alone.
- Or detect the divergence at sim-compile time (compare resolved features of
  shared boundary crates — tokio, bytes, dfir_rs — between host and trybuild)
  and fail with an actionable error instead of UB.
