#!/usr/bin/env bash
# Deterministically demonstrates the host/dylib tokio feature divergence on
# any platform, without relying on the UB manifesting as a crash.
#
# Compares the tokio features resolved for:
#   1. the HOST test binary in the solo invocation    (cargo test -p sim_repro_staged)
#   2. the HOST test binary in the unified invocation (cargo test --workspace)
#   3. the sim DYLIB, as built by the simulator's trybuild workspace
#      (target/hydro_trybuild/sim_repro_staged/, with the exact feature flags
#      `compile_trybuild_example` passes: --no-default-features
#      --features hydro___test,hydro___feature_sim_runtime)
#
# Exits 1 if (2) != (3): the configuration under which the simulator shares
# tokio channel internals across the dlopen boundary is divergent -> UB.
set -euo pipefail
cd "$(dirname "$0")"

TRYBUILD_DYLIB="target/hydro_trybuild/sim_repro_staged/dylib"
if [ ! -d "$TRYBUILD_DYLIB" ]; then
    echo "Generating the sim trybuild workspace (running the sim test once)..."
    cargo test -p sim_repro_staged --quiet > /dev/null
fi

tokio_features() {
    # Prints "tokio vX.Y.Z features=..." for the given cargo-tree args.
    cargo tree -i tokio -f "{p} features={f}" --prefix none "$@" 2>/dev/null | head -1
}

host_solo=$(tokio_features -p sim_repro_staged -e normal,dev)
host_unified=$(tokio_features --workspace -e normal,dev)
dylib=$(cd "$TRYBUILD_DYLIB" && cargo tree --locked --no-default-features \
    --features hydro___test,hydro___feature_sim_runtime \
    -i tokio -f "{p} features={f}" --prefix none -e normal 2>/dev/null | head -1)

echo "HOST (cargo test -p sim_repro_staged):"
echo "  $host_solo"
echo "HOST (cargo test --workspace):"
echo "  $host_unified"
echo "DYLIB (target/hydro_trybuild, as built by the simulator):"
echo "  $dylib"
echo

if [ "$host_solo" != "$dylib" ]; then
    echo "UNEXPECTED: solo host diverges from dylib (repro assumptions broken)"
    exit 2
fi

if [ "$host_unified" != "$dylib" ]; then
    echo "DIVERGENCE: under 'cargo test --workspace', the host test binary and"
    echo "the sim dylib are built against differently-configured tokio crates,"
    echo "while sharing tokio channel data structures across the dlopen"
    echo "boundary. This is undefined behavior (observed as SIGSEGV on"
    echo "macOS/aarch64; may pass silently elsewhere)."
    exit 1
fi

echo "OK: host and dylib tokio features match."
