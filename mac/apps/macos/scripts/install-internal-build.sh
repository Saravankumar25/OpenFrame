#!/usr/bin/env bash
# Builds the internal release app, packs the disk image and installs the app from that disk image
# into /Applications on this Mac (owner decision 2026-10-27: an internal .dmg installed on the
# owner's own Mac; internal builds open without a team password since the same day).
#
# Usage (from the repository root, in Terminal): apps/macos/scripts/install-internal-build.sh
set -euo pipefail
HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT="$(cd "$HERE/../../.." && pwd)"
cd "$ROOT"

echo "== Building the release app (about 25 minutes)"
"$HERE/build-release-app.sh"

echo "== Packing the disk image"
"$HERE/build-dmg.sh"
BUILT="$(ls -t "$ROOT"/release-out/mac/*-internal-*.dmg | head -1)"
[[ -f "$BUILT" ]] || { echo "The disk image was not made." >&2; exit 1; }
# The team's name for this build, as on Windows (Straft-5.exe): Straft-<n>.dmg.
DMG="$ROOT/release-out/mac/${STRAFT_DMG_NAME:-Straft-12}.dmg"
cp "$BUILT" "$DMG"
(cd "$(dirname "$DMG")" && shasum -a 256 "$(basename "$DMG")" > "$(basename "$DMG").sha256")

echo "== Installing from the disk image"
# The core copies the earlier app's data folder on the first start, so that app must be closed too.
if pgrep -x StraftStudio >/dev/null || pgrep -x OpenFrameStudio >/dev/null; then # LEGACY: Open Frame name kept for migration
  echo "Straft Studio is open. Quit it, then run this script again. Nothing was installed." >&2
  exit 1
fi
MNT="$(mktemp -d)"
hdiutil attach "$DMG" -nobrowse -readonly -mountpoint "$MNT" >/dev/null
trap 'hdiutil detach "$MNT" -quiet >/dev/null 2>&1 || true; rmdir "$MNT" 2>/dev/null || true' EXIT
TARGET="/Applications/Straft Studio.app"
EARLIER="/Applications/OpenFrame Studio.app" # LEGACY: Open Frame name kept for migration
for copy in "$TARGET" "$EARLIER"; do
  if [[ -d "$copy" ]]; then
    # The earlier copy is moved to the Trash, never deleted, so it can be put back.
    osascript -e "tell application \"Finder\" to delete POSIX file \"$copy\"" >/dev/null
  fi
done
ditto "$MNT/Straft Studio.app" "$TARGET"
# Spotlight should find only the installed app: drop the development and test copies.
rm -rf "$ROOT/apps/macos/build/Straft Studio.app"
rm -rf "$ROOT/apps/macos/build/OpenFrame Studio.app" # LEGACY: Open Frame name kept for migration
echo "Installed: $TARGET"
echo "Disk image: $DMG"
echo "Your projects, Idea Vault and library are not touched by an install."
open "$TARGET"
