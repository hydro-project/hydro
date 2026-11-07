#!/bin/bash
# Per-declaration heartbeat table (D72): elaborate one file with the
# generator's diagnostics on and print every generated theorem / hoisted
# ghost fact / knot-stack phase with its cost in `maxHeartbeats` units
# (raw heartbeats / 1000) and wall time, largest first. Heartbeats are a
# proxy; proof TIME is the goal (D72) — this table says which declaration
# is slow. The two program overrides (`LeaderElection` 600k, `PaxosCore`
# 250k) are the measured sums of everything one `hydro def` command
# elaborates; `cmd-used=` below is that running sum.
#   usage: scripts/heartbeat_table.sh <file.lean> [N]
#   e.g.   scripts/heartbeat_table.sh Hydro/Paxos/LeaderElection.lean 30
set -u
f="$1"; n="${2:-30}"
log="$TMPDIR/hb_$(basename "$f").log"
HYDRO_PHASE_DIAG=1 HYDRO_GHOST_DIAG=1 HYDROGEN_HB_DIAG=1 \
  lake env lean "$f" > "$log" 2>&1
rc=$?
echo "== $f (lean exit $rc) — top $n declarations by wall time =="
grep -E '^\[hb\] ' "$log" | sed -E 's/^\[hb\] (.*): ([0-9]+)k ([0-9]+)ms.*/\3 \2 \1/' \
  | sort -rn | head -"$n" | awk '{printf "%7d ms %9d k  %s\n", $1, $2, substr($0, index($0,$3))}'
echo "== command-level (generator-side) usage, max per hydro def =="
grep -oE 'cmd-used=[0-9]+k' "$log" | sed 's/cmd-used=//; s/k//' | sort -rn | head -1 \
  | awk '{printf "max running command total %d k (the def'"'"'s override must cover this)\n", $1}'
echo "== phases =="
grep -E '^\[phase\]' "$log"
