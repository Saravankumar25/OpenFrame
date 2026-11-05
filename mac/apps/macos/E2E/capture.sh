#!/usr/bin/env bash
# Captures every screen of the real Mac app, in the default theme in Light mode and in Dark mode, and builds the
# MacHTML pages from the pictures and the accessibility trees.
#
# Usage: apps/macos/E2E/capture.sh <Straft Studio.app | executable> [MacHTML output folder]
# Step 1 runs the capture tour (the zz-capture scenarios of the end-to-end driver) twice against the app, once per
# appearance, with run.sh, or with run-parallel.sh when STRAFT_E2E_PARALLEL is set (that many runners side by side;
# the run folder then holds shard-k folders). Each scenario starts by setting Settings, Appearance, Mode, then writes a
# picture and an accessibility list per screen.
# Step 2 builds MacHTML/light, MacHTML/dark, index.html and README.md (scripts/mac-html/build-mac-html.mjs).
# Step 3 renders every page in WebKit, compares it with its picture and looks for every label and value of the tree in
# the page (scripts/mac-html/check-pages.swift); the tables go at the end of README.md.
# STRAFT_CAPTURE_SKIP_RUN=1 with STRAFT_CAPTURE_DIR=<an earlier capture folder holding light/ and dark/> only
# rebuilds and checks the pages.
# The pages show a person's name and a person's folders: STRAFT_E2E_PERSON sets the name typed at the welcome (default
# Maya Rao), and with one runner the data lives in STRAFT_E2E_CLEAN_ROOT (default /Users/Shared/Straft Demo).
# Needs: the driver library (swift build --product StraftE2EDriver), node and the Swift compiler.
set -uo pipefail
HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT="${STRAFT_ROOT:-$(cd "$HERE/../../.." && pwd)}"
APP="${1:?usage: capture.sh <app or executable> [MacHTML folder]}"
OUT="${2:-$ROOT/MacHTML}"
BASE="${STRAFT_CAPTURE_DIR:-$ROOT/release-out/capture/$(date +%Y%m%d-%H%M%S)}"
RUNNER="$HERE/run.sh"
[[ -n "${STRAFT_E2E_PARALLEL:-}" ]] && RUNNER="$HERE/run-parallel.sh"
export STRAFT_E2E_PERSON="${STRAFT_E2E_PERSON:-Maya Rao}"
if [[ -z "${STRAFT_E2E_PARALLEL:-}" ]]; then export STRAFT_E2E_CLEAN_ROOT="${STRAFT_E2E_CLEAN_ROOT:-/Users/Shared/Straft Demo}"; fi
for MODE in light dark; do
  [[ "${STRAFT_CAPTURE_SKIP_RUN:-}" == 1 ]] && continue
  export STRAFT_CAPTURE_MODE="$MODE"
  export STRAFT_E2E_RUN_DIR="$BASE/$MODE"
  mkdir -p "$STRAFT_E2E_RUN_DIR"
  echo "Capturing the $MODE pass into $STRAFT_E2E_RUN_DIR with $(basename "$RUNNER")"
  "$RUNNER" "$APP" "zz-capture/" || echo "Some tour scenarios failed in the $MODE pass; the pages that were captured are still built."
done
COMMIT="${STRAFT_CAPTURE_COMMIT:-$(git -C "$ROOT" rev-parse --short HEAD 2>/dev/null || echo unknown)}"
node "$ROOT/scripts/mac-html/build-mac-html.mjs" --both "$BASE/light" "$BASE/dark" "$OUT" "$COMMIT" || exit 1

MAX="${MACHTML_MAX_DIFF:-0.5}"
CHANNEL="${MACHTML_CHANNEL:-24}"
TOOLS="$(mktemp -d "${TMPDIR:-/tmp}/machtml-check.XXXXXX")"
trap 'rm -rf "$TOOLS"' EXIT
CHECK=(swift "$ROOT/scripts/mac-html/check-pages.swift")
if swiftc -O "$ROOT/scripts/mac-html/check-pages.swift" -o "$TOOLS/check-pages" >/dev/null 2>&1; then CHECK=("$TOOLS/check-pages"); fi
{
  echo
  echo "## Verification: pages against the pictures"
  echo
  echo "Every page was rendered in WebKit in its normal view at the window's own size and at the picture's scale (2x), and"
  echo "the window part was compared with its capture picture. A pixel differs when any colour channel differs by more than"
  echo "$CHANNEL of 255; a page fails when more than $MAX percent of its pixels differ (the window's transparent rounded corners"
  echo "are not compared). The check also looks for every non-empty label and value of the accessibility tree in the page"
  echo "(as text, a control value or an aria-label); any miss fails the page."
} >> "$OUT/README.md"
STATUS=0
for MODE in light dark; do
  if "${CHECK[@]}" "$OUT/$MODE" "$MAX" "$CHANNEL" > "$OUT/$MODE/check-output.txt" 2>&1; then RES="every page passed"; else RES="some pages failed, or the check did not run"; STATUS=1; fi
  { echo; echo "### $MODE: $RES"; echo; cat "$OUT/$MODE/check-results.md" 2>/dev/null || echo "No table was produced."; } >> "$OUT/README.md"
  rm -f "$OUT/$MODE/check-output.txt" "$OUT/$MODE/check-results.md"
done
echo "Pages: $OUT/index.html"
exit "$STATUS"
