#!/usr/bin/env bash
# perf-profile one role of the running benchmark (D92).
#
#   scripts/bench_perf_role.sh <proc> [seconds] [outfile]
#
# Starts `bench 500 10 <long enough>` in the background, finds the OS process
# `bench --proc <proc>` by reading /proc/*/cmdline (never `pgrep -f`, which
# matches the caller's own shell), attaches `perf record --call-graph dwarf`
# for <seconds>, and prints the flat top-30 self symbols and the top-40
# inclusive symbols with the Lean name mangling stripped, then the machine thread alone
# (the busiest thread) with the share of its samples that pass through each hot spot.
#
# Roles (BenchDemo.memberOf): 0,1 proposers (0 is the leader); 2,3,4 acceptors;
# 5,6 clients; 7 aggregator; 8,9 replicas.
set -euo pipefail
PROC="${1:?proc index}"
SECS="${2:-4}"
OUT="${3:-${TMPDIR:-/tmp}/role${PROC}.data}"
HERE="$(cd "$(dirname "$0")/.." && pwd)"
BENCH="$HERE/.lake/build/bin/bench"
DUR=$(( (SECS + 6) * 1000 ))
"$BENCH" 500 10 "$DUR" > "${OUT%.data}.run.log" 2>&1 &
sleep 2.5
PID=""
for d in /proc/[0-9]*; do
  if tr '\0' ' ' < "$d/cmdline" 2>/dev/null | grep -q "bin/bench --proc $PROC "; then
    PID="${d#/proc/}"; break
  fi
done
[ -n "$PID" ] || { echo "no process for --proc $PROC"; exit 1; }
perf record -F 499 --call-graph dwarf,8192 -o "$OUT" -p "$PID" -- sleep "$SECS" > /dev/null 2>&1
wait
demangle() { sed 's/lp_hydro__lean_//g; s/___at___00/@/g; s/Hydro_Project_//g; s/_spec__[0-9]*//g; s/___redArg//g; s/___lam__/·λ/g; s/___boxed//g'; }
echo "== role $PROC: self (top 30) =="
perf report -i "$OUT" --no-children --sort symbol --stdio -g none 2>/dev/null | grep -v '^#' | grep -v '^$' \
  | awk '{printf "%s ", $1; for(i=3;i<=NF;i++) printf "%s ", $i; print ""}' | demangle | cut -c1-150 | head -30
echo "== role $PROC: inclusive (top 40, our code) =="
perf report -i "$OUT" --children --sort symbol --stdio -g none 2>/dev/null | grep -v '^#' | grep -v '^$' \
  | awk '{printf "%s %s ", $1, $2; for(i=4;i<=NF;i++) printf "%s ", $i; print ""}' | demangle \
  | grep -E '^[0-9.]+% [0-9.]+% (Dag_|Node_|HList|memo|parseFrames|Hydro_|wireForTable|frameMsgs|scanRun|BenchDemo|l_Std|l_List|lean_)' | cut -c1-160 | head -40
echo "== role $PROC: the machine thread alone (busiest thread; share of its samples whose stack contains …) =="
# `perf record -p` samples every thread of the process; the reader and writer threads
# (condition-variable hand-offs through the event loop) are ~60 % of the process's user
# cycles and none of its throughput. The busiest thread is the machine (one activation after
# another); "stack contains" shares are inclusive and overlap.
perf script -i "$OUT" -F tid,ip,sym 2>/dev/null | awk '
BEGIN { RS=""; FS="\n" }
{ tid=$1; gsub(/ /,"",tid); n[tid]++; rec[NR]=$0; t[NR]=tid }
END {
  best=""; for (k in n) if (best=="" || n[k]>n[best]) best=k
  split("Dag_activateFused Node_locStep sequence__payload_dag collect__quorum_dag join__responses_dag p__leader__heartbeat Node_enqueueInbox Hydro_Project_decodeAt parseFrames bucketArray Node_wireEncode frameMsgs routeSends Node_memoState HList_map Dag_wireReceivers keyedFoldAssocImpl listAntiJoinImpl listJoinImpl listFilterNotInImpl lean_dec_ref_cold mi_malloc mi_free", P, " ")
  for (i=1;i<=NR;i++) if (t[i]==best) { tot++; for (j in P) if (index(rec[i], P[j])) c[P[j]]++ }
  printf "machine thread tid %s: %d samples of %d\n", best, tot, NR
  for (k in c) printf "%5.1f%% %s\n", 100*c[k]/tot, k
}' | sort -rn
