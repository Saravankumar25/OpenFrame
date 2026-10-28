#!/usr/bin/env bash
# Runs the end-to-end scenarios against a real OpenFrame Studio app: the release .app (or the app
# inside the mounted .dmg). Each scenario gets its own empty data folder; scenarios that restart
# the app launch it again on the same folder, and "hold" phases end with SIGKILL (a crash).
#
# Usage: apps/macos/E2E/run.sh <OpenFrame Studio.app | executable> [name-filter]
# OPENFRAME_E2E_SHARD=k/n runs only every n-th scenario starting at the k-th (1-based), so n runners
# can share one machine; each scenario still has its own data folder and its own app process.
# OPENFRAME_E2E_FRESH_HOME=1 runs each scenario as a brand-new Mac user would: an empty home folder
# (HOME and CFFIXED_USER_HOME) and the app's real data locations, with no test data folder.
# Needs: the driver library (swift build --product OpenFrameE2EDriver -c release).
# Output: release-out/e2e/<timestamp>/ with one folder per scenario, results.tsv and summary.md.
set -uo pipefail
HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
MACOS="$(cd "$HERE/.." && pwd)"
ROOT="$(cd "$MACOS/../.." && pwd)"
APP="${1:?usage: run.sh <app or executable> [filter]}"
FILTER="${2:-}"
WORK="$(mktemp -d "${TMPDIR:-/tmp}/openframe-e2e.XXXXXX")"
trap 'rm -rf "$WORK"; [[ -n "${TEST_ID:-}" ]] && defaults delete "$TEST_ID" >/dev/null 2>&1; true' EXIT
if [[ -d "$APP" ]]; then
  # The release app has the hardened runtime, which refuses an injected library. Test a private
  # copy whose only difference is an ad hoc signature without it: same executable code, same
  # resources (the runner prints both hashes).
  # Read the flags first: a grep -q in a pipe would end codesign early, and pipefail would then
  # count the match as a failure.
  # The copy also gets its own bundle identifier ("<id>.e2e"): preferences are kept per
  # identifier, and the person's own use of the app (a hidden scene list, a zoom, a card view)
  # must neither reach the scenarios nor be changed by them. The test identifier's preferences
  # are cleared before the run.
  ditto "$APP" "$WORK/app/$(basename "$APP")"
  APP_UNDER_TEST="$WORK/app/$(basename "$APP")"
  PLIST="$APP_UNDER_TEST/Contents/Info.plist"
  # OPENFRAME_E2E_ID_SUFFIX keeps runners that are started separately (not as shards of one run) apart.
  TEST_ID="$(/usr/libexec/PlistBuddy -c 'Print :CFBundleIdentifier' "$PLIST").e2e${OPENFRAME_E2E_ID_SUFFIX:-}$(echo "${OPENFRAME_E2E_SHARD:-1/1}" | cut -d/ -f1)"
  /usr/libexec/PlistBuddy -c "Set :CFBundleIdentifier $TEST_ID" "$PLIST"
  codesign --force --sign - --timestamp=none "$APP_UNDER_TEST" >/dev/null 2>&1
  defaults delete "$TEST_ID" >/dev/null 2>&1 || true
  EXE="$APP_UNDER_TEST/Contents/MacOS/$(/usr/libexec/PlistBuddy -c 'Print :CFBundleExecutable' "$APP_UNDER_TEST/Contents/Info.plist")"
else
  EXE="$APP"
fi
[[ -x "$EXE" ]] || { echo "not an executable: $EXE" >&2; exit 2; }
DRIVER="${OPENFRAME_E2E_DRIVER:-}"
if [[ -z "$DRIVER" ]]; then
  for c in "$MACOS/.build/release/libOpenFrameE2EDriver.dylib" "$MACOS/.build/debug/libOpenFrameE2EDriver.dylib"; do
    [[ -f "$c" ]] && { DRIVER="$c"; break; }
  done
fi
[[ -f "$DRIVER" ]] || { echo "build the driver first: swift build --product OpenFrameE2EDriver -c release" >&2; exit 2; }
export OPENFRAME_E2E_FIXTURES="$ROOT/tests/fixtures"

RUN="${OPENFRAME_E2E_RUN_DIR:-$ROOT/release-out/e2e/$(date +%Y%m%d-%H%M%S)}"
mkdir -p "$RUN"
# The scenario list comes from the driver itself.
OPENFRAME_DATA_ROOT="$WORK/list-data" OPENFRAME_E2E_SCENARIO=__list__ OPENFRAME_E2E_OUT="$WORK/list" \
  DYLD_INSERT_LIBRARIES="$DRIVER" "$EXE" >/dev/null 2>&1 &
LISTPID=$!
# The driver writes the list and ends the app at once; an app that ignored the driver is stopped.
for _ in $(seq 1 150); do kill -0 "$LISTPID" 2>/dev/null || break; sleep 0.2; done
kill -9 "$LISTPID" 2>/dev/null; wait "$LISTPID" 2>/dev/null
LIST="$WORK/list/scenarios.json"
[[ -f "$LIST" ]] || { echo "the driver did not load into the app (hardened runtime or a wrong driver path?)" >&2; exit 3; }

printf "scenario\tresult\tseconds\tmessage\n" > "$RUN/results.tsv"
pass=0; fail=0; skip=0
# A development build (bundle-dev-app.sh) honours the debug-only test switches; a release build
# ignores them on purpose, so the scenarios that need one are skipped there, never failed.
DEV_BUILD=false
[[ -n "${PLIST:-}" ]] && [[ "$(/usr/libexec/PlistBuddy -c 'Print :OpenFrameDevelopmentBuild' "$PLIST" 2>/dev/null)" == true ]] && DEV_BUILD=true
SHARD_K=1; SHARD_N=1
if [[ -n "${OPENFRAME_E2E_SHARD:-}" ]]; then SHARD_K="${OPENFRAME_E2E_SHARD%/*}"; SHARD_N="${OPENFRAME_E2E_SHARD#*/}"; fi
INDEX=0
while IFS=$'\t' read -r name phases timeout ends; do
  [[ -n "$FILTER" && "$name" != *"$FILTER"* ]] && continue
  # The capture tour (MacHTML) runs only when asked for by name.
  [[ "$name" == zz-capture/* && "$FILTER" != zz-capture* ]] && continue
  INDEX=$((INDEX + 1))
  (( (INDEX - SHARD_K) % SHARD_N == 0 )) || continue
  if [[ ( "$name" == 71-storage/first-start* || "$name" == zz-capture/*first-start* ) && "$DEV_BUILD" != true ]]; then
    printf "%s\tskip\t0\tneeds a development build (OPENFRAME_LIBRARY_SETUP is debug only)\n" "$name" >> "$RUN/results.tsv"
    skip=$((skip + 1)); echo "SKIP  $name (needs a development build)"; continue
  fi
  safe="${name//\//__}"
  OUT="$RUN/$safe"; DATA="$WORK/data-$safe"; mkdir -p "$OUT" "$DATA"
  # Each scenario starts with no preferences (its own phases keep theirs).
  [[ -n "${TEST_ID:-}" ]] && { defaults delete "$TEST_ID" >/dev/null 2>&1 || true; }
  status=pass; msg=""; start=$(date +%s)
  IFS=',' read -r -a endlist <<< "$ends"
  for ((p = 1; p <= phases; p++)); do
    end="${endlist[$((p - 1))]}"
    if [[ "${OPENFRAME_E2E_FRESH_HOME:-}" == 1 ]]; then
      mkdir -p "$DATA/home/Documents" "$DATA/home/Library"
      HOME="$DATA/home" CFFIXED_USER_HOME="$DATA/home" OPENFRAME_E2E_WORK="$WORK" OPENFRAME_E2E_SCENARIO="$name" \
        OPENFRAME_E2E_PHASE="$p" OPENFRAME_E2E_OUT="$OUT" DYLD_INSERT_LIBRARIES="$DRIVER" "$EXE" >"$OUT/app-$p.log" 2>&1 &
    else
      # The first-start scenarios (and the capture tour's library setup) begin like a new computer whose
      # library is not chosen yet.
      LIB=""; [[ "$name" == 71-storage/first-start* || "$name" == zz-capture/*first-start* ]] && LIB=1
      OPENFRAME_DATA_ROOT="$DATA" OPENFRAME_LIBRARY_SETUP="$LIB" OPENFRAME_E2E_WORK="$WORK" OPENFRAME_E2E_SCENARIO="$name" \
        OPENFRAME_E2E_PHASE="$p" OPENFRAME_E2E_OUT="$OUT" DYLD_INSERT_LIBRARIES="$DRIVER" "$EXE" >"$OUT/app-$p.log" 2>&1 &
    fi
    pid=$!
    limit=$(( ${timeout%.*} + 60 )); t=0
    while [[ ! -f "$OUT/result-$p.json" ]] && kill -0 "$pid" 2>/dev/null && (( t < limit * 5 )); do sleep 0.2; t=$((t + 1)); done
    if [[ ! -f "$OUT/result-$p.json" ]]; then
      kill -9 "$pid" 2>/dev/null; wait "$pid" 2>/dev/null
      status=fail; msg="phase $p: no result (app exited or hung); see app-$p.log"; break
    fi
    r=$(/usr/bin/python3 -c 'import json,sys; d=json.load(open(sys.argv[1])); print(d["status"]+"\t"+d["message"].replace("\t"," ").replace("\n"," "))' "$OUT/result-$p.json")
    rs="${r%%$'\t'*}"; rm_="${r#*$'\t'}"
    if [[ "$end" == "hold" && "$rs" == "pass" ]]; then
      sleep 0.3; kill -9 "$pid" 2>/dev/null; wait "$pid" 2>/dev/null
    else
      t=0; while kill -0 "$pid" 2>/dev/null && (( t < 150 )); do sleep 0.2; t=$((t + 1)); done
      if kill -0 "$pid" 2>/dev/null; then
        kill -9 "$pid"; wait "$pid" 2>/dev/null
        [[ "$rs" == "pass" ]] && { rs=fail; rm_="phase $p: the app did not quit within 30 s"; }
      else
        wait "$pid"; code=$?
        if [[ "$rs" == "pass" && $code -ne 0 ]]; then rs=fail; rm_="phase $p: the app exited with code $code on quit"; fi
      fi
    fi
    if [[ "$rs" != "pass" ]]; then status=fail; msg="phase $p: $rm_"; break; fi
  done
  secs=$(( $(date +%s) - start ))
  printf "%s\t%s\t%s\t%s\n" "$name" "$status" "$secs" "$msg" >> "$RUN/results.tsv"
  if [[ "$status" == pass ]]; then pass=$((pass + 1)); echo "PASS  $name (${secs}s)"; else fail=$((fail + 1)); echo "FAIL  $name: $msg"; fi
done < <(/usr/bin/python3 -c 'import json,sys
for s in json.load(open(sys.argv[1])): print("\t".join([s["name"], str(s["phases"]), str(s["timeout"]), ",".join(s["ends"])]))' "$LIST")

{
  echo "# End-to-end run on the real app"
  echo
  echo "- App: \`$(basename "$APP")\`, executable under test $(shasum -a 256 "$EXE" | cut -c1-16)..."
  [[ -d "$APP" ]] && echo "- Code directory hash of the shipped app: $(codesign -dvvv "$APP" 2>&1 | sed -n 's/^CDHash=//p'), of the copy under test: $(codesign -dvvv "${APP_UNDER_TEST:-$APP}" 2>&1 | sed -n 's/^CDHash=//p')"
  echo "- Machine: $(sw_vers -productName) $(sw_vers -productVersion), $(uname -m)"
  echo "- Date: $(date -u +%Y-%m-%dT%H:%M:%SZ)"
  echo "- Result: $pass passed, $fail failed"
  echo
  echo "| Scenario | Result | Seconds | Message |"
  echo "|---|---|---|---|"
  tail -n +2 "$RUN/results.tsv" | awk -F'\t' '{printf "| %s | %s | %s | %s |\n", $1, $2, $3, $4}'
} > "$RUN/summary.md"
echo "$pass passed, $fail failed, $skip skipped. Results: $RUN"
[[ $fail -eq 0 ]]
