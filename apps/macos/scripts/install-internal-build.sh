#!/usr/bin/env bash
# Builds the internal release app with the team password, packs the disk image and installs the
# app from that disk image into /Applications on this Mac (owner decision 2026-10-27: an internal
# .dmg installed on the owner's own Mac). The password is read without echo, passed to the build
# through the environment only, and never written to a file or a log.
#
# Usage (from the repository root, in Terminal): apps/macos/scripts/install-internal-build.sh
set -euo pipefail
HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT="$(cd "$HERE/../../.." && pwd)"
cd "$ROOT"

read -r -s -p "Team password for this internal build (at least 8 characters): " PASSWORD
echo
if [[ ${#PASSWORD} -lt 8 ]]; then
  echo "The team password must have at least 8 characters. Nothing was built." >&2
  exit 2
fi

echo "== Building the release app (about 25 minutes)"
OPENFRAME_TEAM_PASSWORD="$PASSWORD" "$HERE/build-release-app.sh"
unset PASSWORD

echo "== Packing the disk image"
"$HERE/build-dmg.sh"
DMG="$(ls -t "$ROOT"/release-out/mac/*.dmg | head -1)"
[[ -f "$DMG" ]] || { echo "The disk image was not made." >&2; exit 1; }

echo "== Installing from the disk image"
if pgrep -x OpenFrameStudio >/dev/null; then
  echo "OpenFrame Studio is open. Quit it, then run this script again. Nothing was installed." >&2
  exit 1
fi
MNT="$(mktemp -d)"
hdiutil attach "$DMG" -nobrowse -readonly -mountpoint "$MNT" >/dev/null
trap 'hdiutil detach "$MNT" -quiet >/dev/null 2>&1 || true; rmdir "$MNT" 2>/dev/null || true' EXIT
TARGET="/Applications/OpenFrame Studio.app"
if [[ -d "$TARGET" ]]; then
  # The earlier copy is moved to the Trash, never deleted, so it can be put back.
  osascript -e "tell application \"Finder\" to delete POSIX file \"$TARGET\"" >/dev/null
fi
ditto "$MNT/OpenFrame Studio.app" "$TARGET"
echo "Installed: $TARGET"
echo "Disk image: $DMG"
echo "Your projects, Idea Vault and library are not touched by an install."
open "$TARGET"
