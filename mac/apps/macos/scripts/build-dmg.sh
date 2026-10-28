#!/usr/bin/env bash
# Packs the internal release app into a disk image for the team (Mac phase M9, ADR-0030).
# Unsigned (ad hoc signature only) and not notarised: the team opens it once with
# Control-click > Open, or removes the quarantine flag (docs/testing/pilot-kit/mac-install.md).
#
# Usage: apps/macos/scripts/build-dmg.sh   (after build-release-app.sh)
# Output in release-out/mac/:
#   OpenFrame-Studio-<version>-internal-<commit>.dmg, .sha256, -sbom.json, -manifest.txt
set -euo pipefail
HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
MACOS="$(cd "$HERE/.." && pwd)"
ROOT="$(cd "$MACOS/../.." && pwd)"
OUT="$ROOT/release-out/mac"
APP="$OUT/OpenFrame Studio.app"
[[ -d "$APP" ]] || { echo "build the app first: apps/macos/scripts/build-release-app.sh" >&2; exit 2; }
codesign --verify --deep --strict "$APP"

VERSION="$(/usr/libexec/PlistBuddy -c 'Print :CFBundleShortVersionString' "$APP/Contents/Info.plist")"
COMMIT="$(git -C "$ROOT" rev-parse --short HEAD)"
NAME="OpenFrame-Studio-$VERSION-internal-$COMMIT"
DMG="$OUT/$NAME.dmg"

STAGE="$(mktemp -d "${TMPDIR:-/tmp}/openframe-dmg.XXXXXX")"
trap 'rm -rf "$STAGE"' EXIT
ditto "$APP" "$STAGE/OpenFrame Studio.app"
ln -s /Applications "$STAGE/Applications"
cat > "$STAGE/Read Me First.txt" <<'TXT'
OpenFrame Studio, internal team build

1. Drag OpenFrame Studio onto Applications.

2. Before the first start, open Terminal (Applications > Utilities > Terminal), paste this line
   and press Return:

   xattr -dr com.apple.quarantine "/Applications/OpenFrame Studio.app"

   Why: this team build is not signed with an Apple Developer ID yet (that comes with the store
   release). macOS marks every downloaded app, and for an app without that signature it says
   "OpenFrame Studio is damaged and can't be opened" or "Apple could not verify". The app is not
   damaged: the line above only removes the download mark from this one app. Check the SHA-256
   your owner sent first (shasum -a 256 <the .dmg file>) so you know the file is the right one.

3. Open OpenFrame Studio from Applications.

4. The offline AI is downloaded inside the app when you ask for it (Settings > Offline AI).
   Nothing else leaves this Mac.

Apple silicon Macs only, macOS 14 or later.
TXT

rm -f "$DMG"
hdiutil create -volname "OpenFrame Studio" -srcfolder "$STAGE" -fs APFS -format UDZO -imagekey zlib-level=9 -ov "$DMG" >/dev/null
hdiutil verify "$DMG" >/dev/null

# Checks on the image as the team receives it: mount read-only, check the signature, scan.
MNT="$(mktemp -d "${TMPDIR:-/tmp}/openframe-dmg-mnt.XXXXXX")"
hdiutil attach -readonly -nobrowse -mountpoint "$MNT" "$DMG" >/dev/null
cleanup_mount() { hdiutil detach "$MNT" -quiet >/dev/null 2>&1 || true; rm -rf "$STAGE" "$MNT"; }
trap cleanup_mount EXIT
codesign --verify --deep --strict "$MNT/OpenFrame Studio.app"
node "$ROOT/scripts/scan-installer.mjs" "$MNT" "$DMG"

( cd "$OUT" && shasum -a 256 "$NAME.dmg" > "$NAME.dmg.sha256" )
node "$HERE/mac-sbom.mjs" "$MNT/OpenFrame Studio.app" > "$OUT/$NAME-sbom.json"
{
  echo "file: $NAME.dmg"
  echo "bytes: $(stat -f %z "$DMG")"
  echo "sha256: $(cut -d' ' -f1 "$OUT/$NAME.dmg.sha256")"
  echo "version: $VERSION"
  echo "commit: $(git -C "$ROOT" rev-parse HEAD)"
  echo "built: $(date -u +%Y-%m-%dT%H:%M:%SZ)"
  echo "machine: $(sw_vers -productName) $(sw_vers -productVersion) $(uname -m)"
  echo "signature: ad hoc (not notarised)"
  echo "executable sha256: $(shasum -a 256 "$MNT/OpenFrame Studio.app/Contents/MacOS/OpenFrameStudio" | cut -d' ' -f1)"
  echo "app files:"
  ( cd "$MNT/OpenFrame Studio.app" && find . -type f | sort | while read -r f; do printf '  %s  %s\n' "$(shasum -a 256 "$f" | cut -c1-16)" "$f"; done )
} > "$OUT/$NAME-manifest.txt"
echo "$DMG"
cat "$OUT/$NAME.dmg.sha256"
