#!/usr/bin/env bash
#
# Benchmark the per-test "final compile" of trybuild-generated binaries, and experiment with
# cargo/rustc flags for it.
#
# Hydro tests (sim, maelstrom, localhost deploy) generate a Rust source file per test and
# compile it as a cargo `--example` against a prebuilt dylib of the trybuild project. With a
# fully warm cache (e.g. CI with `__CARGO_DEFAULT_LIB_METADATA=1`), the *final compile* of each
# generated example is the dominant remaining per-test cost. This script measures exactly that
# step so we can iterate on flags / linking strategies without rerunning the tests themselves.
#
# For a breakdown of *all* per-test phases (staged codegen, cargo metadata, prebuild, final
# build, sim execution, ...), use scripts/profile_test_phases.sh instead.
#
# How it works:
#   1. "Warm" phase: runs a couple of hydro_lang sim tests via nextest (twice: once to populate
#      caches / run prebuilds, once to capture logs with everything cached). The build phases in
#      `hydro_lang::compile::trybuild::generate` are instrumented with `tracing` spans (target
#      `hydro_build`); this script enables them via RUST_LOG and captures the exact final
#      `cargo rustc` command.
#   2. "Measure" phase: replays each captured final-compile command directly, N times, touching
#      the generated example sources before each run to force a recompile (this mimics CI, where
#      every commit produces fresh generated sources but the dependency cache is warm).
#
# Usage:
#   ./scripts/bench_trybuild.sh                 # warm (if needed) + measure
#   ./scripts/bench_trybuild.sh --rewarm        # force re-running the warm phase
#
# Env knobs:
#   ITERS=5                    number of measured iterations per test (default 5)
#   LIB_METADATA=0             unset __CARGO_DEFAULT_LIB_METADATA (defaults to 1, matching CI)
#   BENCH_TESTS='...'          space-separated test-name regexes for nextest (default: two sim tests)
#   EXTRA_RUSTC_FLAGS='...'    extra flags appended to the replayed command (after `--`, so they
#                              are passed to rustc for the example target, e.g. '-Cdebuginfo=0')
#   EXTRA_CARGO_FLAGS='...'    extra cargo flags inserted right after `rustc` (e.g. '--timings')
#
# Examples:
#   ITERS=3 ./scripts/bench_trybuild.sh
#   EXTRA_RUSTC_FLAGS='-Clink-arg=-fuse-ld=lld' ./scripts/bench_trybuild.sh
#   EXTRA_CARGO_FLAGS='--timings' ITERS=1 ./scripts/bench_trybuild.sh
set -euo pipefail

cd "$(dirname "$0")/.."

# Match CI (.github/workflows/ci.yml). Set LIB_METADATA=0 to benchmark without
# __CARGO_DEFAULT_LIB_METADATA (normal local-dev configuration).
if [[ "${LIB_METADATA:-1}" == 0 ]]; then
    unset __CARGO_DEFAULT_LIB_METADATA
else
    export __CARGO_DEFAULT_LIB_METADATA="${__CARGO_DEFAULT_LIB_METADATA:-1}"
fi

ITERS="${ITERS:-5}"
BENCH_TESTS="${BENCH_TESTS:-sim_collect_waits_for_all_ticks sim_cluster_e2m_m2e}"
EXTRA_RUSTC_FLAGS="${EXTRA_RUSTC_FLAGS:-}"
EXTRA_CARGO_FLAGS="${EXTRA_CARGO_FLAGS:-}"

TARGET_DIR="$(cargo metadata --format-version 1 --no-deps | python3 -c 'import json,sys; print(json.load(sys.stdin)["target_directory"])')"
BENCH_DIR="$TARGET_DIR/bench-trybuild"
WARM_LOG="$BENCH_DIR/warm.log"
mkdir -p "$BENCH_DIR"

REWARM=0
for arg in "$@"; do
    case "$arg" in
        --rewarm) REWARM=1 ;;
        *) echo "unknown argument: $arg" >&2; exit 1 ;;
    esac
done

filter_expr() {
    local expr="" t
    for t in $BENCH_TESTS; do
        if [[ -n "$expr" ]]; then expr+=" or "; fi
        expr+="test(/${t}\$/)"
    done
    echo "$expr"
}

run_tests() {
    # --no-capture: we need the `hydro_build` tracing spans on stderr; also serializes tests so
    # per-test timings are not polluted by concurrent builds. RUST_LOG enables the spans
    # (subscriber is installed by hydro_lang's test init ctor).
    RUST_LOG='error,hydro_build=debug' cargo nextest run -p hydro_lang --features sim,deploy \
        --no-capture -E "$(filter_expr)" 2>&1
}

warm() {
    echo "==> warm phase: building test binary + prebuilds (this can take a while on a cold cache)"
    run_tests > "$BENCH_DIR/warm-pass1.log" || {
        tail -50 "$BENCH_DIR/warm-pass1.log"
        echo "warm pass 1 failed; see $BENCH_DIR/warm-pass1.log" >&2
        exit 1
    }
    echo "==> warm phase: capturing final-compile commands with a hot cache"
    # Touch generated examples so the final compiles re-run and get logged.
    touch_examples
    run_tests > "$WARM_LOG" || {
        tail -50 "$WARM_LOG"
        echo "warm pass 2 failed; see $WARM_LOG" >&2
        exit 1
    }
}

touch_examples() {
    local d
    for d in "$TARGET_DIR"/hydro_trybuild/*/dylib-examples/examples "$TARGET_DIR"/hydro_trybuild/*/examples; do
        [[ -d "$d" ]] && find "$d" -name '*.rs' -exec touch {} + || true
    done
}

strip_ansi() {
    sed 's/\x1b\[[0-9;]*m//g'
}

stats() {
    # stdin: one duration-in-ms per line
    awk '{ a[NR]=$1; s+=$1 } END {
        if (NR==0) { print "no data"; exit 1 }
        asort_n = NR; for (i=1;i<asort_n;i++) for (j=i+1;j<=asort_n;j++) if (a[j]<a[i]) { t=a[i]; a[i]=a[j]; a[j]=t }
        med = (NR%2==1) ? a[(NR+1)/2] : (a[NR/2]+a[NR/2+1])/2
        printf "n=%d  mean=%.0fms  median=%.0fms  min=%.0fms  max=%.0fms\n", NR, s/NR, med, a[1], a[NR]
    }'
}

if [[ "$REWARM" == 1 || ! -s "$WARM_LOG" ]] || ! grep -q "final build command" "$WARM_LOG" 2>/dev/null; then
    warm
fi

# --- measure: replay the captured final-compile commands ---
# Extract the captured final-compile commands from the `hydro_build` tracing events. Each
# event's message looks like:
#   final build command (cwd=/path/to/dylib-examples): VAR="1" ... "cargo" "rustc" ...
mapfile -t CMD_LINES < <(strip_ansi < "$WARM_LOG" | grep -o 'final build command (cwd=[^)]*): .*' | sort -u)
if [[ ${#CMD_LINES[@]} -eq 0 ]]; then
    echo "no final-compile commands captured in $WARM_LOG; try --rewarm" >&2
    exit 1
fi

echo "==> replaying ${#CMD_LINES[@]} final-compile command(s), $ITERS iterations each"
[[ -n "$EXTRA_RUSTC_FLAGS" ]] && echo "    EXTRA_RUSTC_FLAGS: $EXTRA_RUSTC_FLAGS"
[[ -n "$EXTRA_CARGO_FLAGS" ]] && echo "    EXTRA_CARGO_FLAGS: $EXTRA_CARGO_FLAGS"

overall_times="$BENCH_DIR/replay-times.txt"
: > "$overall_times"

idx=0
for line in "${CMD_LINES[@]}"; do
    idx=$((idx+1))
    cwd="${line#final build command (cwd=}"
    cwd="${cwd%%)*}"
    cmd="${line#*): }"
    # The Command debug output is already shell-compatible: env assignments like KEY="val"
    # followed by quoted program+args. Prefix with `env` so the assignments are accepted.
    if [[ -n "$EXTRA_CARGO_FLAGS" ]]; then
        cmd="${cmd/\"rustc\"/\"rustc\" $EXTRA_CARGO_FLAGS}"
    fi
    if [[ -n "$EXTRA_RUSTC_FLAGS" ]]; then
        cmd="$cmd $EXTRA_RUSTC_FLAGS"
    fi

    # Show which example this is (TRYBUILD_LIB_NAME for sim, --example otherwise)
    label="$(echo "$line" | grep -o 'TRYBUILD_LIB_NAME="[^"]*"' | head -1 || true)"
    [[ -z "$label" ]] && label="$(echo "$line" | grep -o '"--example" "[^"]*"' | head -1 || true)"
    echo "--- command $idx ($label) in $cwd"

    times_file="$BENCH_DIR/replay-cmd$idx.txt"
    : > "$times_file"
    for ((i=1; i<=ITERS; i++)); do
        touch_examples
        start=$(date +%s%N)
        if ! (cd "$cwd" && eval "env $cmd") > "$BENCH_DIR/replay-cmd$idx-run$i.out" 2> "$BENCH_DIR/replay-cmd$idx-run$i.err"; then
            echo "replay failed; stderr:" >&2
            cat "$BENCH_DIR/replay-cmd$idx-run$i.err" >&2
            exit 1
        fi
        end=$(date +%s%N)
        ms=$(( (end - start) / 1000000 ))
        echo "$ms" | tee -a "$times_file" >> "$overall_times"
        echo "    iter $i: ${ms}ms"
    done
    echo -n "    stats: "; stats < "$times_file"
done

echo "==> overall stats (all commands pooled):"
stats < "$overall_times"
