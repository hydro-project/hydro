#!/bin/bash
# Budget probe: elaborate one file with its `set_option maxHeartbeats N in`
# lines replaced by a given value; report pass/fail + wall time.
# usage: budget_probe.sh <file> <value>   (value 0 = delete the option line)
set -u
f="$1"; v="$2"
tmp="$TMPDIR/probe_$(basename "$f")"
if [ "$v" = "0" ]; then
  sed -E '/^set_option maxHeartbeats [0-9]+ in$/d' "$f" > "$tmp"
else
  sed -E "s/^set_option maxHeartbeats [0-9]+ in$/set_option maxHeartbeats $v in/" "$f" > "$tmp"
fi
# keep the module path for imports: copy into place temporarily
cp "$f" "$f.orig"
cp "$tmp" "$f"
start=$(date +%s)
out=$(lake env lean "$f" 2>&1)
rc=$?
end=$(date +%s)
cp "$f.orig" "$f"; rm -f "$f.orig"
errs=$(echo "$out" | grep -c "^$f.*error\|error:")
hb=$(echo "$out" | grep -c "maximum number of heartbeats")
echo "$f budget=$v rc=$rc errors=$errs heartbeat_errors=$hb time=$((end-start))s"
