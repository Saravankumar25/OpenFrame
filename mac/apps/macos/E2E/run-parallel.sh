#!/usr/bin/env bash
# Runs the end-to-end suite as N runners side by side (default 4) and joins their results.
# Usage: apps/macos/E2E/run-parallel.sh <app> [filter]   (OPENFRAME_E2E_PARALLEL=n to change n)
# Every scenario still runs in its own app process with its own data folder; the runners only
# share the machine. Output: release-out/e2e/<timestamp>/ with shard-k folders, results.tsv and summary.md.
set -uo pipefail
HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT="$(cd "$HERE/../../.." && pwd)"
APP="${1:?usage: run-parallel.sh <app> [filter]}"
FILTER="${2:-}"
N="${OPENFRAME_E2E_PARALLEL:-4}"
RUN="${OPENFRAME_E2E_RUN_DIR:-$ROOT/release-out/e2e/$(date +%Y%m%d-%H%M%S)}"
mkdir -p "$RUN"
pids=()
for k in $(seq 1 "$N"); do
  OPENFRAME_E2E_SHARD="$k/$N" OPENFRAME_E2E_RUN_DIR="$RUN/shard-$k" "$HERE/run.sh" "$APP" "$FILTER" > "$RUN/shard-$k.log" 2>&1 &
  pids+=($!)
  sleep 3
done
for p in "${pids[@]}"; do wait "$p"; done
printf "scenario\tresult\tseconds\tmessage\n" > "$RUN/results.tsv"
for k in $(seq 1 "$N"); do tail -n +2 "$RUN/shard-$k/results.tsv" 2>/dev/null >> "$RUN/results.tsv"; done
pass=$(awk -F'\t' 'NR>1 && $2=="pass"' "$RUN/results.tsv" | wc -l | tr -d ' ')
fail=$(awk -F'\t' 'NR>1 && $2!="pass"' "$RUN/results.tsv" | wc -l | tr -d ' ')
{
  echo "# End-to-end run on the real app ($N runners side by side)"
  echo
  sed -n '3,6p' "$RUN/shard-1/summary.md" 2>/dev/null
  echo "- Result: $pass passed, $fail failed"
  echo
  echo "| Scenario | Result | Seconds | Message |"
  echo "|---|---|---|---|"
  tail -n +2 "$RUN/results.tsv" | sort | awk -F'\t' '{printf "| %s | %s | %s | %s |\n", $1, $2, $3, $4}'
} > "$RUN/summary.md"
echo "$pass passed, $fail failed. Results: $RUN"
[[ $fail -eq 0 ]]
