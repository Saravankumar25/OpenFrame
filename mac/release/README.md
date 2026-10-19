# OpenFrame Studio for the Mac

This folder (`mac/release/`) holds the internal disk image of the Mac track of Chapter 1. Next to it, `mac/MacHTML/`
holds the HTML replica of every Mac screen, and the rest of `mac/` is the complete Mac codebase. Commands below run
from inside `mac/`.

## What is in this folder

| Path | What it is |
|---|---|
| `OpenFrame-Studio-<version>-internal-<commit>.dmg` | The internal disk image: Apple silicon, macOS 14 or later, team password gate, ad hoc signed with the hardened runtime, not notarised. |
| `*.sha256`, `*-sbom.json`, `*-manifest.txt` | SHA-256 of the image, the software bill of materials (CycloneDX), and the file list with hashes. |
| [`../MacHTML/index.html`](../MacHTML/index.html) | Every screen of the Mac app as an HTML page, in the default theme in Light and Dark (a switch on every page). |

## Install it (team)

Read [the Mac install note](../docs/testing/pilot-kit/mac-install.md). In short: check the SHA-256, open the disk image, drag the app to
Applications, open it once with Control-click > Open (or System Settings > Privacy & Security > Open Anyway), and enter
the team password.

## Where the code is

| Part | Path |
|---|---|
| The Mac app (SwiftUI and AppKit) | `apps/macos/Sources/OpenFrameUI` |
| The typed Swift interface to the core | `apps/macos/Sources/OpenFrameCore` (operations and types are generated) |
| The shared Rust core | `crates/` (the same core as Windows) |
| The core as a Swift library (UniFFI) | `crates/openframe-ffi` |
| Model-level tests | `apps/macos/Tests` |
| Real-app end-to-end driver and scenarios | `apps/macos/Sources/OpenFrameE2EDriver`, runner `apps/macos/E2E/run.sh` |
| Release build, disk image, SBOM | `apps/macos/scripts/build-release-app.sh`, `build-dmg.sh`, `mac-sbom.mjs` |
| HTML replica tooling | `apps/macos/E2E/capture.sh`, `scripts/mac-html/` |
| Offline AI engine for the Mac | `packaging/ai/ai-assets.lock.json`, `packaging/ai/fetch-macos-runtime.sh`, ADR-0033 |

## Build it

```bash
apps/macos/scripts/build-core.sh debug
apps/macos/scripts/test.sh
OPENFRAME_TEAM_PASSWORD=<team password> apps/macos/scripts/build-release-app.sh
apps/macos/scripts/build-dmg.sh
```

Then run the end-to-end suite on the app inside the image:

```bash
apps/macos/E2E/run.sh "/path/to/OpenFrame Studio.app"
```

## Evidence

- Release gate for the Mac: [mac-internal-release-gate.md](../docs/release/mac-internal-release-gate.md)
- Evidence folder: `../docs/release/evidence/mac/` (M1 to M9, AI evaluation, security review, end-to-end runs)
- Parity with Windows: [mac-parity-matrix.md](../docs/roadmap/mac-parity-matrix.md)
