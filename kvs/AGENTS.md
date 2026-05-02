# Build & Test Notes

## Architecture

See `ARCHITECTURE.md` for the full system architecture, data model, request flow, deployment infrastructure, and testing strategy.

**Important:** `ARCHITECTURE.md` must always accurately reflect the current architecture of the demo app. When making changes that affect the system's structure, data flow, deployment topology, wire protocol, or component responsibilities, update `ARCHITECTURE.md` to match.

## Brazil Workspace Structure

This is a Brazil workspace with 6 packages that must be built in dependency order:

1. `HydroProjectStageleft` (no workspace deps)
2. `HydroProjectHydro` (depends on Stageleft)
3. `HydroProjectDemoApp` (depends on Hydro, Stageleft)

The other packages (`HydroProjectDemoAppCDK`, `HydroProjectDemoAppTests`, `HydroProjectDemoAppImageBuild`) are not needed for the docker test.

## Building

Run from each package's `src/<PackageName>/` directory.

```sh
cd src/HydroProjectStageleft && brazil-build quick
cd src/HydroProjectHydro && brazil-build quick
cd src/HydroProjectDemoApp && brazil-build quick
```

- Use `brazil-build quick` for fast compile-only builds (no tests, no linting).
- Use `brazil-build` (no argument) to compile and run tests.
- Use `brazil-build release` only when you need the full release build (tests + coverage + linting) — this is significantly slower.
- Use `brazil-build quick` instead of `brazil-build release` for HydroProjectHydro — the full release build runs very long trybuild snapshot tests.
- The DemoApp's custom build script (`cargo-brazil-hydro`) does NOT accept extra args like `--features`. It will fail with `unexpected argument '--features'`.
- The build script automatically runs a `export` step and compiles trybuild binaries for the Hydro staged compilation.
- For the CDK package, use `brazil-build` (not `brazil-build quick` — it doesn't have a `quick` script).

## Running the Docker KVS Test

Run cargo directly from the DemoApp package directory:

```sh
cd src/HydroProjectDemoApp
cargo test --frozen --example kvs -- test_kvs_docker
```

Do NOT set `CARGO_HOME` or run `cargo brazil configure` first — just run `cargo test` directly.

The test (`test_kvs_docker` in `examples/kvs.rs`):

- Cleans up stale docker containers/networks at the start and end of the test
- Spawns `cargo run --example kvs -- --mode docker` as a child process
- Deploys 3 routers + 7 storage nodes in Docker containers
- Connects to all 3 router endpoints and verifies each accepts connections
- Runs the full KVS test suite (puts, gets, overwrites, missing keys, 100-key distribution check)
- Takes ~1 minute

## Pre-commit Checks

Before committing Rust code changes, run:

```sh
cargo clippy --tests
cargo +nightly fmt
```

For the CDK package, also run:

```sh
npx tsc --noEmit
npx prettier --check '**/*.{js,ts,md}' '!build/**/*' '!dist/**/*'
```

The build fleet runs `prettier-check` on all `*.md` files in the CDK package, so markdown formatting issues will fail the build.

Fix any warnings or formatting issues before committing.

## Git

- Never amend commits that have been pushed to a remote branch.
- In general, prefer creating new commits over amending. Amending rewrites history and can cause problems if the commit has already been shared or auto-pushed by CRUX.
