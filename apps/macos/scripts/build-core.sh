#!/usr/bin/env bash
# Builds the Rust core as a Swift library for the Mac shell (Mac phase M2, ADR-0027):
#   1. the static library of crates/openframe-ffi for Apple silicon,
#   2. the Swift side of the UniFFI interface (Sources/OpenFrameCore/Generated/openframe_ffi.swift),
#   3. the XCFramework the Swift package links (Frameworks/OpenFrameFFI.xcframework),
#   4. the Swift operation list and argument types (scripts/gen-swift-types.mjs).
# Usage: apps/macos/scripts/build-core.sh [debug|release]   (default debug)
# Outputs in Frameworks/ and Generated/openframe_ffi.swift are build products and are not committed.
set -euo pipefail

PROFILE="${1:-debug}"
HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
MACOS="$(cd "$HERE/.." && pwd)"
ROOT="$(cd "$MACOS/../.." && pwd)"
TARGET=aarch64-apple-darwin
export PATH="$HOME/.cargo/bin:$PATH"
# .cargo/config.toml compiles through sccache; a Mac without it turns that off.
command -v sccache >/dev/null 2>&1 || export RUSTC_WRAPPER=""
export MACOSX_DEPLOYMENT_TARGET=14.0

cd "$ROOT"
CARGO_FLAGS=(-p openframe-ffi --lib --target "$TARGET")
if [[ "$PROFILE" == "release" ]]; then
  CARGO_FLAGS+=(--release)
fi
echo "Building openframe-ffi ($PROFILE, $TARGET)"
cargo build "${CARGO_FLAGS[@]}"
LIB="$ROOT/target/$TARGET/$PROFILE/libopenframe_ffi.a"
test -f "$LIB" || { echo "missing $LIB" >&2; exit 1; }

GEN_DIR="$(mktemp -d)"
trap 'rm -rf "$GEN_DIR"' EXIT
echo "Generating the Swift interface"
cargo run -q -p openframe-ffi --features bindgen --bin uniffi-bindgen -- \
  generate --library "$LIB" --language swift --out-dir "$GEN_DIR" >/dev/null

SWIFT_OUT="$MACOS/Sources/OpenFrameCore/Generated"
mkdir -p "$SWIFT_OUT"
cp "$GEN_DIR/openframe_ffi.swift" "$SWIFT_OUT/openframe_ffi.swift"

echo "Assembling the XCFramework"
XCF="$MACOS/Frameworks/OpenFrameFFI.xcframework"
rm -rf "$XCF"
HEADERS="$GEN_DIR/headers"
mkdir -p "$HEADERS"
cp "$GEN_DIR/openframe_ffiFFI.h" "$HEADERS/"
cp "$GEN_DIR/openframe_ffiFFI.modulemap" "$HEADERS/module.modulemap"
if xcodebuild -version >/dev/null 2>&1; then
  xcodebuild -create-xcframework -library "$LIB" -headers "$HEADERS" -output "$XCF" >/dev/null
else
  # Command Line Tools only: write the documented XCFramework layout by hand.
  SLICE="$XCF/macos-arm64"
  mkdir -p "$SLICE/Headers"
  # A hard link, not a copy: the debug library is several hundred megabytes.
  ln -f "$LIB" "$SLICE/libopenframe_ffi.a" 2>/dev/null || cp "$LIB" "$SLICE/libopenframe_ffi.a"
  cp "$HEADERS/"* "$SLICE/Headers/"
  cat > "$XCF/Info.plist" <<'PLIST'
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
	<key>AvailableLibraries</key>
	<array>
		<dict>
			<key>BinaryPath</key>
			<string>libopenframe_ffi.a</string>
			<key>HeadersPath</key>
			<string>Headers</string>
			<key>LibraryIdentifier</key>
			<string>macos-arm64</string>
			<key>LibraryPath</key>
			<string>libopenframe_ffi.a</string>
			<key>SupportedArchitectures</key>
			<array>
				<string>arm64</string>
			</array>
			<key>SupportedPlatform</key>
			<string>macos</string>
		</dict>
	</array>
	<key>CFBundlePackageType</key>
	<string>XFWK</string>
	<key>XCFrameworkFormatVersion</key>
	<string>1.0</string>
</dict>
</plist>
PLIST
fi

# The theme list the Windows app reads, shared by the Mac theme engine (single source of truth).
mkdir -p "$MACOS/Sources/OpenFrameUI/Resources"
cp "$ROOT/crates/openframe-application/src/modules/appearance/themes.json" "$MACOS/Sources/OpenFrameUI/Resources/themes.json"

echo "Generating the Swift operation list and types"
cargo run -q -p openframe-ffi --features bindgen --bin op-inventory > "$MACOS/Sources/OpenFrameCore/Generated/operations.json"
node "$ROOT/scripts/gen-swift-types.mjs"

du -sh "$XCF" | awk '{print "XCFramework: " $1}'
echo "Done"
