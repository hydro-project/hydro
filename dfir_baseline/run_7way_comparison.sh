#!/bin/bash
# 7-way metastability comparison demo
set -e

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
WORKSPACE_ROOT="$(dirname "$SCRIPT_DIR")"

RATE="55:30,165:15,55:90"
THINK=10
TIMEOUT_MS=100
RETRIES=3
QUEUE=7

mkdir -p /tmp/dfir_comparison
rm -f /tmp/dfir_comparison/*.jsonl

# Kill any zombie processes from previous runs
"$SCRIPT_DIR/kill_servers.sh"

echo "=== 1/7: DFIR Multi-Stage ==="
QUIET=1 SERVER_ADDRESS=127.0.0.1:8080 THINK_TIME_MS=$THINK "$WORKSPACE_ROOT/target/release/server" &
S=$!; sleep 2
SERVER_ADDRESS=127.0.0.1:8080 RATE_SCHEDULE="$RATE" TIMEOUT_MS=$TIMEOUT_MS MAX_RETRIES=$RETRIES \
    METRICS_FILE=/tmp/dfir_comparison/dfir_multistage.jsonl "$WORKSPACE_ROOT/target/release/client_quiet" &
C=$!; sleep 137; kill $C $S 2>/dev/null || true
"$SCRIPT_DIR/kill_servers.sh"
echo "Events: $(wc -l < /tmp/dfir_comparison/dfir_multistage.jsonl)"

echo "=== 2/7: DFIR Single Unbounded ==="
QUIET=1 SERVER_ADDRESS=127.0.0.1:8081 THINK_TIME_MS=$THINK "$WORKSPACE_ROOT/target/release/server_unbounded" &
S=$!; sleep 2
SERVER_ADDRESS=127.0.0.1:8081 RATE_SCHEDULE="$RATE" TIMEOUT_MS=$TIMEOUT_MS MAX_RETRIES=$RETRIES \
    METRICS_FILE=/tmp/dfir_comparison/dfir_unbounded.jsonl "$WORKSPACE_ROOT/target/release/client_quiet" &
C=$!; sleep 137; kill $C $S 2>/dev/null || true
"$SCRIPT_DIR/kill_servers.sh"
echo "Events: $(wc -l < /tmp/dfir_comparison/dfir_unbounded.jsonl)"

echo "=== 3/7: TCP Blocking ==="
QUIET=1 SERVER_ADDRESS=127.0.0.1:8082 THINK_TIME_MS=$THINK "$WORKSPACE_ROOT/target/release/server_tcp_blocking" &
S=$!; sleep 2
SERVER_ADDRESS=127.0.0.1:8082 RATE_SCHEDULE="$RATE" TIMEOUT_MS=$TIMEOUT_MS MAX_RETRIES=$RETRIES \
    METRICS_FILE=/tmp/dfir_comparison/tcp_blocking.jsonl "$WORKSPACE_ROOT/target/release/client_quiet" &
C=$!; sleep 137; kill $C $S 2>/dev/null || true
"$SCRIPT_DIR/kill_servers.sh"
echo "Events: $(wc -l < /tmp/dfir_comparison/tcp_blocking.jsonl)"

echo "=== 4/7: DFIR Single + Admission Control (polite) ==="
QUIET=1 SERVER_ADDRESS=127.0.0.1:8083 MAX_QUEUE_DEPTH=$QUEUE THINK_TIME_MS=$THINK "$WORKSPACE_ROOT/target/release/server_single_stage" &
S=$!; sleep 2
SERVER_ADDRESS=127.0.0.1:8083 RATE_SCHEDULE="$RATE" TIMEOUT_MS=$TIMEOUT_MS MAX_RETRIES=$RETRIES \
    METRICS_FILE=/tmp/dfir_comparison/single_admission_polite.jsonl "$WORKSPACE_ROOT/target/release/client_quiet" &
C=$!; sleep 137; kill $C $S 2>/dev/null || true
"$SCRIPT_DIR/kill_servers.sh"
echo "Events: $(wc -l < /tmp/dfir_comparison/single_admission_polite.jsonl)"

echo "=== 5/7: DFIR Single + Admission Control (rude) ==="
QUIET=1 SERVER_ADDRESS=127.0.0.1:8083 MAX_QUEUE_DEPTH=$QUEUE THINK_TIME_MS=$THINK "$WORKSPACE_ROOT/target/release/server_single_stage" &
S=$!; sleep 2
SERVER_ADDRESS=127.0.0.1:8083 RATE_SCHEDULE="$RATE" TIMEOUT_MS=$TIMEOUT_MS MAX_RETRIES=$RETRIES \
    METRICS_FILE=/tmp/dfir_comparison/single_admission_rude.jsonl "$WORKSPACE_ROOT/target/release/client_rude" &
C=$!; sleep 137; kill $C $S 2>/dev/null || true
"$SCRIPT_DIR/kill_servers.sh"
echo "Events: $(wc -l < /tmp/dfir_comparison/single_admission_rude.jsonl)"

echo "=== 6/7: DFIR Multi + Admission Control (polite) ==="
QUIET=1 SERVER_ADDRESS=127.0.0.1:8084 MAX_QUEUE_DEPTH=$QUEUE THINK_TIME_MS=$THINK "$WORKSPACE_ROOT/target/release/server_multistage_admission" &
S=$!; sleep 2
SERVER_ADDRESS=127.0.0.1:8084 RATE_SCHEDULE="$RATE" TIMEOUT_MS=$TIMEOUT_MS MAX_RETRIES=$RETRIES \
    METRICS_FILE=/tmp/dfir_comparison/multi_admission_polite.jsonl "$WORKSPACE_ROOT/target/release/client_quiet" &
C=$!; sleep 137; kill $C $S 2>/dev/null || true
"$SCRIPT_DIR/kill_servers.sh"
echo "Events: $(wc -l < /tmp/dfir_comparison/multi_admission_polite.jsonl)"

echo "=== 7/7: DFIR Multi + Admission Control (rude) ==="
QUIET=1 SERVER_ADDRESS=127.0.0.1:8084 MAX_QUEUE_DEPTH=$QUEUE THINK_TIME_MS=$THINK "$WORKSPACE_ROOT/target/release/server_multistage_admission" &
S=$!; sleep 2
SERVER_ADDRESS=127.0.0.1:8084 RATE_SCHEDULE="$RATE" TIMEOUT_MS=$TIMEOUT_MS MAX_RETRIES=$RETRIES \
    METRICS_FILE=/tmp/dfir_comparison/multi_admission_rude.jsonl "$WORKSPACE_ROOT/target/release/client_rude" &
C=$!; sleep 137; kill $C $S 2>/dev/null || true
"$SCRIPT_DIR/kill_servers.sh"
echo "Events: $(wc -l < /tmp/dfir_comparison/multi_admission_rude.jsonl)"

echo ""
echo "=== Generating plots ==="
python3 "$SCRIPT_DIR/scripts/plot_comparison.py"
echo "Done. Plot: /tmp/dfir_metastability_comparison.png"
