#!/usr/bin/env bash
# Captures every screen of the real Mac app, in the default theme in Light mode and in Dark mode, and builds the
# MacHTML pages from the pictures.
#
# Usage: apps/macos/E2E/capture.sh <OpenFrame Studio.app | executable> [MacHTML output folder]
# Step 1 runs the capture tour (the zz-capture scenarios of the end-to-end driver) twice against the app with the
# usual runner, once per appearance. Each scenario starts by setting Settings, Appearance, Mode, then writes a
# picture and an accessibility list per screen. A build with the team gate shows the gate first and the tour types
# OPENFRAME_E2E_TEAM_PASSWORD (or OPENFRAME_TEAM_PASSWORD) to get past it; the gate is the first page.
# Step 2 builds MacHTML/light, MacHTML/dark, index.html and README.md (scripts/mac-html/build-mac-html.mjs).
# Step 3 renders every page in WebKit and compares it with its picture (scripts/mac-html/check-pages.swift).
# OPENFRAME_CAPTURE_SKIP_RUN=1 with OPENFRAME_CAPTURE_DIR=<an earlier capture folder> only rebuilds the pages from it.
# Needs: the driver library (swift build --product OpenFrameE2EDriver) and node.
set -uo pipefail
HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT="${OPENFRAME_ROOT:-$(cd "$HERE/../../.." && pwd)}"
APP="${1:?usage: capture.sh <app or executable> [MacHTML folder]}"
OUT="${2:-$ROOT/MacHTML}"
BASE="${OPENFRAME_CAPTURE_DIR:-$ROOT/release-out/capture/$(date +%Y%m%d-%H%M%S)}"
for MODE in light dark; do
  [[ "${OPENFRAME_CAPTURE_SKIP_RUN:-}" == 1 ]] && continue
  export OPENFRAME_CAPTURE_MODE="$MODE"
  export OPENFRAME_E2E_RUN_DIR="$BASE/$MODE"
  mkdir -p "$OPENFRAME_E2E_RUN_DIR"
  echo "Capturing the $MODE pass into $OPENFRAME_E2E_RUN_DIR"
  "$HERE/run.sh" "$APP" "zz-capture/" || echo "Some tour scenarios failed in the $MODE pass; the pages that were captured are still built."
done
COMMIT="$(git -C "$ROOT" rev-parse --short HEAD 2>/dev/null || echo unknown)"
node "$ROOT/scripts/mac-html/build-mac-html.mjs" --both "$BASE/light" "$BASE/dark" "$OUT" "$COMMIT" || exit 1
{
  echo
  echo "## Verification: pages against the pictures"
  echo
  echo "Every page was rendered in WebKit at the window's own size (one CSS pixel is one point) and compared with its capture picture. The number is the mean difference over all colour channels, in percent; a page over ${MACHTML_MAX_DIFF:-1.5}% fails."
} >> "$OUT/README.md"
for MODE in light dark; do
  if swift "$ROOT/scripts/mac-html/check-pages.swift" "$OUT/$MODE" "${MACHTML_MAX_DIFF:-1.5}" > "$OUT/$MODE/check-output.txt" 2>&1; then RES="passed"; else RES="has pages over the threshold, or did not run"; fi
  { echo; echo "### $MODE: $RES"; echo; cat "$OUT/$MODE/check-results.md" 2>/dev/null || echo "No table was produced."; } >> "$OUT/README.md"
  rm -f "$OUT/$MODE/check-output.txt" "$OUT/$MODE/check-results.md"
done
echo "Pages: $OUT/index.html"
