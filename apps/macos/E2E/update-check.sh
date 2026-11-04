#!/usr/bin/env bash
# The update check (package file 05, U2): a person's data written by the previous build opens in the
# new build. Phase 1 of each two-phase scenario runs on the OLD app, phase 2 on the NEW app, on the
# same data folder. Both apps run as private copies with an ad hoc signature, as apps/macos/E2E/run.sh
# does, because the hardened runtime refuses the driver.
#
# Usage: apps/macos/E2E/update-check.sh <old app> <old driver> <new app> <new driver>
# Output: release-out/update-check/run-<time>/ with one folder per scenario and summary.txt
set -uo pipefail
HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT="$(cd "$HERE/../../.." && pwd)"
OLD_APP="${1:?old app}"; OLD_DRIVER="${2:?old driver}"; NEW_APP="${3:?new app}"; NEW_DRIVER="${4:?new driver}"
WORK="$(mktemp -d "${TMPDIR:-/tmp}/openframe-update.XXXXXX")"
trap 'rm -rf "$WORK"' EXIT
RUN="$ROOT/release-out/update-check/run-$(date +%Y%m%d-%H%M%S)"
mkdir -p "$RUN"

private_copy() {
  ditto "$1" "$WORK/$2/$(basename "$1")"
  local app="$WORK/$2/$(basename "$1")"
  /usr/libexec/PlistBuddy -c "Set :CFBundleIdentifier app.openframestudio.updatecheck.$2" "$app/Contents/Info.plist"
  codesign --force --sign - --timestamp=none "$app" >/dev/null 2>&1
  echo "$app/Contents/MacOS/$(/usr/libexec/PlistBuddy -c 'Print :CFBundleExecutable' "$app/Contents/Info.plist")"
}
OLD_EXE="$(private_copy "$OLD_APP" old)"
NEW_EXE="$(private_copy "$NEW_APP" new)"
export OPENFRAME_E2E_FIXTURES="$ROOT/tests/fixtures"

run_phase() { # exe driver scenario phase data out end
  local exe="$1" driver="$2" name="$3" phase="$4" data="$5" out="$6" end="$7"
  OPENFRAME_DATA_ROOT="$data" OPENFRAME_E2E_WORK="$WORK" OPENFRAME_E2E_SCENARIO="$name" OPENFRAME_E2E_PHASE="$phase" \
    OPENFRAME_E2E_OUT="$out" DYLD_INSERT_LIBRARIES="$driver" "$exe" >"$out/app-$phase.log" 2>&1 &
  local pid=$! t=0
  while [[ ! -f "$out/result-$phase.json" ]] && kill -0 "$pid" 2>/dev/null && (( t < 1500 )); do sleep 0.2; t=$((t + 1)); done
  if [[ ! -f "$out/result-$phase.json" ]]; then
    kill -9 "$pid" 2>/dev/null; wait "$pid" 2>/dev/null
    echo "no result"; return
  fi
  if [[ "$end" == hold ]]; then
    sleep 0.3; kill -9 "$pid" 2>/dev/null; wait "$pid" 2>/dev/null
  else
    t=0; while kill -0 "$pid" 2>/dev/null && (( t < 150 )); do sleep 0.2; t=$((t + 1)); done
    kill -9 "$pid" 2>/dev/null; wait "$pid" 2>/dev/null
  fi
  /usr/bin/python3 -c 'import json,sys; d=json.load(open(sys.argv[1])); print(d["status"]+" "+d["message"][:200])' "$out/result-$phase.json"
}

pass=0; fail=0
check() { # scenario end-of-phase-1
  local name="$1" end="$2" safe="${1//\//__}"
  local out="$RUN/$safe" data="$WORK/data-$safe"
  mkdir -p "$out" "$data"
  local a b
  a="$(run_phase "$OLD_EXE" "$OLD_DRIVER" "$name" 1 "$data" "$out" "$end")"
  b="$(run_phase "$NEW_EXE" "$NEW_DRIVER" "$name" 2 "$data" "$out" quit)"
  local schema
  schema="$(sqlite3 "file:$data/app.sqlite?mode=ro" 'PRAGMA user_version' 2>/dev/null)"
  if [[ "$a" == pass* && "$b" == pass* ]]; then
    pass=$((pass + 1)); echo "PASS  $name (app schema now $schema)" | tee -a "$RUN/summary.txt"
  else
    fail=$((fail + 1)); printf "FAIL  %s\n  old: %s\n  new: %s\n" "$name" "$a" "$b" | tee -a "$RUN/summary.txt"
  fi
}

# Written by the old build and read back by the new one: Story page text, screenplay text (after a
# clean quit and after a crash), and the sheets on the way out.
check "03-story-page/type-restart-and-it-is-there" quit
check "26-forced-kill-keeps-work/saved-text-survives-sigkill" hold
check "98-quit/command-q-with-project-settings-open" quit
check "26-forced-kill-keeps-work/every-piece-of-work-survives-sigkill" hold

echo "$pass passed, $fail failed. Results: $RUN" | tee -a "$RUN/summary.txt"
[[ "$fail" == 0 ]]
