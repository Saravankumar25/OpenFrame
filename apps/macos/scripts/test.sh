#!/usr/bin/env bash
# Builds and tests the Swift package (apps/macos). Run apps/macos/scripts/build-core.sh first.
#
# With full Xcode, `swift test` finds the test frameworks itself. With the Command Line Tools
# only, Swift Testing lives in a framework folder SwiftPM does not search, so this script passes
# that folder to the compiler and the linker. XCTest is not part of the Command Line Tools; the
# package uses Swift Testing only.
# Usage: apps/macos/scripts/test.sh [extra swift test arguments, e.g. --filter CoreTests]
set -euo pipefail
HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
MACOS="$(cd "$HERE/.." && pwd)"
cd "$MACOS"

if [[ ! -d Frameworks/OpenFrameFFI.xcframework ]]; then
  echo "Frameworks/OpenFrameFFI.xcframework is missing: run apps/macos/scripts/build-core.sh first" >&2
  exit 1
fi

FLAGS=()
if ! xcodebuild -version >/dev/null 2>&1; then
  DEV="$(xcode-select -p)"
  FW="$DEV/Library/Developer/Frameworks"
  LIB="$DEV/Library/Developer/usr/lib"
  if [[ -d "$FW/Testing.framework" ]]; then
    FLAGS+=(-Xswiftc -F -Xswiftc "$FW" -Xlinker -F -Xlinker "$FW"
            -Xlinker -rpath -Xlinker "$FW" -Xlinker -rpath -Xlinker "$LIB")
  else
    echo "Swift Testing was not found in $FW; install Xcode or current Command Line Tools" >&2
    exit 1
  fi
fi
swift build "${FLAGS[@]+"${FLAGS[@]}"}"
# Serial: the screen tests share process-wide notifications (workspace switches, Build
# Screenplay targets) and debounce timers, so suites running side by side can see each other's
# notices or miss a timer under load. Each test still uses its own core and data folder.
swift test --no-parallel "${FLAGS[@]+"${FLAGS[@]}"}" "$@"
