#!/usr/bin/env bash
#
# Profile the per-test phase timings of hydro trybuild-based tests (sim, maelstrom, deploy).
#
# Runs the selected tests serially under nextest with the `hydro_build` tracing spans enabled
# (see hydro_lang/src/compile/trybuild/generate.rs, sim/graph.rs, sim/flow.rs), then parses the
# span-close events to produce a per-test breakdown of where the time goes:
#
#   flow_build          building DFIR graphs from the Hydro IR (in the test process)
#   sim_codegen         generating the example source (DFIR codegen + quoting)
#   gen_staged          __staged.rs generation; cached via a stamp of the current test
#                       executable (path + mtime), so only the first test per binary pays
#                       the syn+prettyplease cost
#   unparse_source      prettyplease of the generated example
#   create_trybuild     setting up the trybuild project dir (includes cargo_metadata)
#   cargo_metadata      `cargo metadata --no-deps` subprocess
#   write_project_files writing manifests/lib.rs into the trybuild project
#   update_lockfile     Cargo.lock regen (only when manifests change)
#   write_generated_sources  writing the example + __staged.rs
#   prebuild            freshness check (+ dep build when stale)
#   populate_job_dir    symlinking the per-job build dir
#   final_build         the final `cargo rustc` of the generated example
#   load_dylib          dlopen of the built cdylib
#   sim_compiled        total of all of the above (sim only)
#   sim_exhaustive      running the exhaustive simulation itself
#
# Usage:
#   ./scripts/profile_test_phases.sh                            # default: a few hydro_lang sim tests
#   FILTER='test(/sim_/)' ./scripts/profile_test_phases.sh      # any nextest filter expression
#   PACKAGE=hydro_test FEATURES=deploy ./scripts/profile_test_phases.sh
#
# Env knobs:
#   PACKAGE=hydro_lang         cargo package to test
#   FEATURES=sim,deploy        cargo features
#   FILTER='...'               nextest filter expression (-E)
#   LIB_METADATA=1             match CI's __CARGO_DEFAULT_LIB_METADATA=1 (set 0 to disable)
set -euo pipefail

cd "$(dirname "$0")/.."

if [[ "${LIB_METADATA:-1}" == 0 ]]; then
    unset __CARGO_DEFAULT_LIB_METADATA
else
    export __CARGO_DEFAULT_LIB_METADATA="${__CARGO_DEFAULT_LIB_METADATA:-1}"
fi

PACKAGE="${PACKAGE:-hydro_lang}"
FEATURES="${FEATURES:-sim,deploy}"
FILTER="${FILTER:-test(/sim_collect_waits_for_all_ticks$/) or test(/sim_cluster_e2m_m2e$/) or test(/sim_batch_cross_singleton$/) or test(/tick_batch$/)}"

TARGET_DIR="$(cargo metadata --format-version 1 --no-deps | python3 -c 'import json,sys; print(json.load(sys.stdin)["target_directory"])')"
OUT_DIR="$TARGET_DIR/profile-test-phases"
mkdir -p "$OUT_DIR"
LOG="$OUT_DIR/run.log"

echo "==> running tests (serially, --no-capture) with hydro_build span tracing"
RUST_LOG='error,hydro_build=debug' cargo nextest run -p "$PACKAGE" --features "$FEATURES" \
    --no-capture -E "$FILTER" > "$LOG" 2>&1 || {
    tail -50 "$LOG"
    echo "test run failed; see $LOG" >&2
    exit 1
}

python3 - "$LOG" <<'EOF'
import re, sys, collections

log_path = sys.argv[1]
ansi = re.compile(r'\x1b\[[0-9;]*m')

def parse_dur(s):
    m = re.match(r'^([0-9.]+)(ns|µs|us|ms|s|m)$', s)
    if not m:
        return None
    v = float(m.group(1))
    unit = m.group(2)
    return v * {'ns': 1e-6, 'µs': 1e-3, 'us': 1e-3, 'ms': 1.0, 's': 1000.0, 'm': 60000.0}[unit]

# span close lines look like (after ANSI stripping):
#   <ts> DEBUG hydro_build: parent:child{fields}: close time.busy=1.23s time.idle=456µs
# and nextest end-of-test lines like:
#   PASS [   5.123s] hydro_lang tests::foo
close_re = re.compile(r'hydro_build .*?:\d+: (?:[A-Za-z0-9_]+(?:\{[^}]*\})?: )*?(?P<name>[A-Za-z0-9_]+)(?:\{[^}]*\})?: close\s+time\.busy=(?P<busy>\S+)\s+time\.idle=(?P<idle>\S+)')
result_re = re.compile(r'^\s*(PASS|FAIL|TIMEOUT|ABORT)\s+\[\s*([0-9.]+)s\]\s+(?:\(\d+/\d+\)\s+)?\S+\s+(\S+)')

pending = []            # span closes since the last test boundary
tests = []              # (name, status, total_s, {phase: ms})
phase_order = []

with open(log_path, errors='replace') as f:
    for line in f:
        line = ansi.sub('', line.rstrip('\n'))
        m = close_re.search(line)
        if m:
            busy = parse_dur(m.group('busy'))
            if busy is not None:
                pending.append((m.group('name'), busy))
                if m.group('name') not in phase_order:
                    phase_order.append(m.group('name'))
            continue
        m = result_re.match(line)
        if m:
            status, total, name = m.group(1), float(m.group(2)), m.group(3)
            phases = collections.defaultdict(float)
            for pname, busy in pending:
                phases[pname] += busy
            pending = []
            tests.append((name, status, total, dict(phases)))

if not tests:
    print("no test results parsed; check the log")
    sys.exit(1)

def fmt_ms(v):
    return f'{v:9.0f}' if v else f'{"-":>9}'

print()
for name, status, total, phases in tests:
    print(f'=== {name} [{status}] total={total:.2f}s')
    accounted = 0.0
    for p in phase_order:
        if p in phases:
            print(f'    {p:26s} {phases[p]:8.0f} ms')
    # spans that nest inside others shouldn't be double counted; report umbrella separately
    top = sum(v for k, v in phases.items() if k in ('sim_compiled', 'sim_exhaustive'))
    other = total * 1000 - top
    print(f'    {"(sim_compiled+sim_exhaustive)":26s} {top:8.0f} ms   (unaccounted vs total: {other:.0f} ms)')

print()
print('=== aggregate (mean ms per test) ===')
for p in phase_order:
    vals = [phases.get(p, 0.0) for _, _, _, phases in tests]
    n = sum(1 for v in vals if v)
    if n:
        print(f'    {p:26s} mean={sum(vals)/len(tests):8.0f} ms   (present in {n}/{len(tests)} tests)')
totals = [t for _, _, t, _ in tests]
print(f'    {"TOTAL (nextest)":26s} mean={1000*sum(totals)/len(totals):8.0f} ms')
EOF

echo
echo "full log: $LOG"
