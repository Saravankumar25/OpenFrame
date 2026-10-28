#!/usr/bin/env bash
# Wraps the debug build of the Mac app in an app bundle so it runs like a Mac app (menu bar,
# Dock, Settings window, its own bundle identifier). Development only: unsigned, not optimised,
# never handed to anyone. The release disk image is built in phase M9, only on the owner's word.
# Usage: apps/macos/scripts/bundle-dev-app.sh   ->  apps/macos/build/OpenFrame Studio.app
set -euo pipefail
HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
MACOS="$(cd "$HERE/.." && pwd)"
cd "$MACOS"
swift build --product OpenFrameStudio
BIN="$(swift build --product OpenFrameStudio --show-bin-path)"
APP="$MACOS/build/OpenFrame Studio.app"
rm -rf "$APP"
mkdir -p "$APP/Contents/MacOS" "$APP/Contents/Resources"
cp "$BIN/OpenFrameStudio" "$APP/Contents/MacOS/OpenFrameStudio"
# SwiftPM resource bundles (themes.json and friends) sit next to the executable.
for b in "$BIN"/*.bundle; do [[ -e "$b" ]] && cp -R "$b" "$APP/Contents/Resources/"; done
VERSION="$(sed -n 's/^version = "\(.*\)"/\1/p' ../../Cargo.toml | head -1)"
cat > "$APP/Contents/Info.plist" <<PLIST
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
	<key>CFBundleName</key><string>OpenFrame Studio</string>
	<key>CFBundleDisplayName</key><string>OpenFrame Studio</string>
	<key>CFBundleIdentifier</key><string>studio.openframe.mac.dev</string>
	<key>CFBundleExecutable</key><string>OpenFrameStudio</string>
	<key>CFBundlePackageType</key><string>APPL</string>
	<key>CFBundleShortVersionString</key><string>${VERSION}</string>
	<key>CFBundleVersion</key><string>$(git -C "$MACOS" rev-list --count HEAD)</string>
	<key>LSMinimumSystemVersion</key><string>14.0</string>
	<key>OpenFrameDevelopmentBuild</key><true/>
	<key>LSApplicationCategoryType</key><string>public.app-category.productivity</string>
	<key>NSHighResolutionCapable</key><true/>
	<key>NSMicrophoneUsageDescription</key><string>OpenFrame Studio records voice notes into your Idea Vault only when you press Record.</string>
	<key>NSSupportsAutomaticTermination</key><false/>
	<key>NSSupportsSuddenTermination</key><false/>
</dict>
</plist>
PLIST
echo "$APP"
