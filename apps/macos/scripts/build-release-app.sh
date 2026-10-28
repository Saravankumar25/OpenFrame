#!/usr/bin/env bash
# Builds the internal release app bundle of OpenFrame Studio for Apple silicon (Mac phase M9).
#
#   1. the Rust core in the release profile, with the team password hashed in (ADR-0024, ADR-0030);
#   2. the Swift app in the release configuration, in a staging copy of apps/macos so a release
#      build never replaces the debug core that development and tests link;
#   3. OpenFrame Studio.app: executable, resources, the AI engine (when fetched), Info.plist,
#      privacy manifest and icon, signed ad hoc (internal build: no Developer ID, not notarised).
#
# Usage: OPENFRAME_TEAM_PASSWORD=... apps/macos/scripts/build-release-app.sh
# Output: release-out/mac/OpenFrame Studio.app
# The password is read from the environment only; it is never written to a file or a log.
set -euo pipefail
HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
MACOS="$(cd "$HERE/.." && pwd)"
ROOT="$(cd "$MACOS/../.." && pwd)"
TARGET=aarch64-apple-darwin
export PATH="$HOME/.cargo/bin:$PATH"
command -v sccache >/dev/null 2>&1 || export RUSTC_WRAPPER=""
export MACOSX_DEPLOYMENT_TARGET=14.0
export CARGO_INCREMENTAL=0

if [[ -z "${OPENFRAME_TEAM_PASSWORD:-}" ]]; then
  echo "set OPENFRAME_TEAM_PASSWORD (at least 8 characters) for the internal build" >&2
  exit 2
fi
[[ "$(uname -m)" == "arm64" ]] || { echo "the release app is built on an Apple silicon Mac" >&2; exit 2; }

# Outside the home folder: build paths end up in the binary and must not name the builder.
# A private per-user temporary folder (mode 700), not a shared fixed path another account could
# prepare in advance.
STAGE="$(getconf DARWIN_USER_TEMP_DIR)openframe-release-stage/macos"
OUT="$ROOT/release-out/mac"
APP="$OUT/OpenFrame Studio.app"
VERSION="$(sed -n 's/^version = "\(.*\)"/\1/p' "$ROOT/Cargo.toml" | head -1)"
COMMIT="$(git -C "$ROOT" rev-parse --short HEAD)"
# The store accepts at most three numbers in CFBundleVersion: the build number is the commit count.
BUILD="$(git -C "$ROOT" rev-list --count HEAD)"

echo "== 1. Rust core (release, team gate on)"
cd "$ROOT"
# Source paths in panic messages are remapped to neutral names, as in the Windows release build
# (scripts/with-msvc-linker.mjs), so the binary does not carry the builder's folders.
CARGO_HOME_DIR="${CARGO_HOME:-$HOME/.cargo}"
RUSTUP_HOME_DIR="${RUSTUP_HOME:-$HOME/.rustup}"
export RUSTFLAGS="--remap-path-prefix=$ROOT=openframe --remap-path-prefix=$CARGO_HOME_DIR=cargo --remap-path-prefix=$RUSTUP_HOME_DIR=rustup"
cargo build -p openframe-ffi --lib --release
LIB="$ROOT/target/release/libopenframe_ffi.a"
[[ -f "$LIB" ]] || { echo "missing $LIB" >&2; exit 1; }
# The internal build must carry the gate: the hash file exists only when the gate was embedded.
grep -aqF '$argon2id$' "$LIB" || { echo "the release core has no team gate hash" >&2; exit 1; }

echo "== 2. Swift app (release) in $STAGE"
mkdir -p "$STAGE"
rsync -a --delete --exclude .build --exclude Frameworks --exclude build "$MACOS/" "$STAGE/"
XCF="$STAGE/Frameworks/OpenFrameFFI.xcframework"
rm -rf "$STAGE/Frameworks"
mkdir -p "$XCF/macos-arm64/Headers"
cp "$LIB" "$XCF/macos-arm64/libopenframe_ffi.a"
cp "$MACOS/Frameworks/OpenFrameFFI.xcframework/macos-arm64/Headers/"* "$XCF/macos-arm64/Headers/"
cp "$MACOS/Frameworks/OpenFrameFFI.xcframework/Info.plist" "$XCF/Info.plist"
# The Swift interface is generated from this very library: one generated earlier from another
# build of the core fails its checksum check and the app stops at start (2026-10-27).
GEN_DIR="$(mktemp -d)"
(cd "$ROOT" && RUSTFLAGS= cargo run -q -p openframe-ffi --features bindgen --bin uniffi-bindgen -- \
  generate --library "$LIB" --language swift --out-dir "$GEN_DIR" >/dev/null)
cp "$GEN_DIR/openframe_ffi.swift" "$STAGE/Sources/OpenFrameCore/Generated/openframe_ffi.swift"
cp "$GEN_DIR/openframe_ffiFFI.h" "$XCF/macos-arm64/Headers/openframe_ffiFFI.h"
rm -rf "$GEN_DIR"
cd "$STAGE"
SWIFT_FLAGS=(-c release -Xswiftc -file-prefix-map -Xswiftc "$STAGE=openframe")
swift build "${SWIFT_FLAGS[@]}" --product OpenFrameStudio
swift build "${SWIFT_FLAGS[@]}" --product OpenFrameE2EDriver
BIN="$(swift build "${SWIFT_FLAGS[@]}" --product OpenFrameStudio --show-bin-path)"

echo "== 3. App bundle"
rm -rf "$APP"
mkdir -p "$APP/Contents/MacOS" "$APP/Contents/Resources"
cp "$BIN/OpenFrameStudio" "$APP/Contents/MacOS/OpenFrameStudio"
strip -x "$APP/Contents/MacOS/OpenFrameStudio" 2>/dev/null || true
for b in "$BIN"/*.bundle; do [[ -e "$b" ]] && cp -R "$b" "$APP/Contents/Resources/"; done
cp "$MACOS/packaging/PrivacyInfo.xcprivacy" "$APP/Contents/Resources/PrivacyInfo.xcprivacy"
[[ -f "$MACOS/packaging/AppIcon.icns" ]] && cp "$MACOS/packaging/AppIcon.icns" "$APP/Contents/Resources/AppIcon.icns"
# The offline AI engine (llama.cpp with Metal), when packaging/ai has fetched it (ADR-0026).
if [[ -d "$ROOT/.build-cache/ai-runtime/macos-arm64" ]]; then
  mkdir -p "$APP/Contents/Resources/ai"
  cp -R "$ROOT/.build-cache/ai-runtime/macos-arm64/." "$APP/Contents/Resources/ai/"
fi
cat > "$APP/Contents/Info.plist" <<PLIST
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
	<key>CFBundleName</key><string>OpenFrame Studio</string>
	<key>CFBundleDisplayName</key><string>OpenFrame Studio</string>
	<key>CFBundleIdentifier</key><string>studio.openframe.mac</string>
	<key>CFBundleExecutable</key><string>OpenFrameStudio</string>
	<key>CFBundleIconFile</key><string>AppIcon</string>
	<key>CFBundlePackageType</key><string>APPL</string>
	<key>CFBundleShortVersionString</key><string>${VERSION}</string>
	<key>CFBundleVersion</key><string>${BUILD}</string>
	<key>LSMinimumSystemVersion</key><string>14.0</string>
	<key>LSArchitecturePriority</key><array><string>arm64</string></array>
	<key>LSRequiresNativeExecution</key><true/>
	<key>LSApplicationCategoryType</key><string>public.app-category.productivity</string>
	<key>NSHighResolutionCapable</key><true/>
	<key>NSHumanReadableCopyright</key><string>OpenFrame Studio internal build ${COMMIT}</string>
	<key>NSMicrophoneUsageDescription</key><string>OpenFrame Studio records voice notes into your Idea Vault only when you press Record.</string>
	<key>NSSupportsAutomaticTermination</key><false/>
	<key>NSSupportsSuddenTermination</key><false/>
	<key>LSFileQuarantineEnabled</key><true/>
</dict>
</plist>
PLIST
plutil -lint "$APP/Contents/Info.plist" >/dev/null

# Ad hoc signature: Apple silicon runs only signed code. The internal build has no Developer ID
# and is not notarised; the team opens it once with Control-click > Open (pilot kit).
# The hardened runtime is on: no DYLD_* environment variables, no unsigned code loaded into the
# process. The only exception the app needs is the microphone for voice notes. The end-to-end
# runner re-signs a private copy without it to inject its driver (apps/macos/E2E/run.sh).
ENT="$MACOS/packaging/OpenFrameStudio-internal.entitlements"
# The engine files keep the bytes pinned in packaging/ai/ai-assets.lock.json (the core checks
# their SHA-256 before every start), so they are sealed as resources and never re-signed.
codesign --force --sign - --timestamp=none --options runtime --entitlements "$ENT" "$APP"
codesign --verify --deep --strict "$APP"

echo "== 4. Secret and personal-path scan of the bundle"
OPENFRAME_TEAM_PASSWORD="$OPENFRAME_TEAM_PASSWORD" node "$ROOT/scripts/scan-installer.mjs" "$APP"

echo "== Done"
echo "$APP"
echo "version $VERSION, build $BUILD, commit $COMMIT"
shasum -a 256 "$APP/Contents/MacOS/OpenFrameStudio"
