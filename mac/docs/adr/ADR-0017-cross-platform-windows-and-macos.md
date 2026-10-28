# ADR-0017: One codebase, two platform adapters, three release files

- **Amended 2026-10-15 by [ADR-0027](ADR-0027-rust-core-two-ui-shells.md):** "one codebase, two platform adapters" becomes one Rust core with two shells (Tauri on Windows, SwiftUI on the Mac). Decisions 2 (Windows 11 only) and 3 (Apple silicon only) stand; the Intel MacBook is a development machine. Decision 3's "AI engine built with Metal" is read together with [ADR-0028](ADR-0028-ai-quality-program-and-harness.md) (Apple's model as an opt-in provider).
- **Status:** Accepted 2026-10-06 by the owner. Amends the Engineering Package Index locked decision "Initial OS" and supersedes the earlier proposal in this file (Windows now, macOS later).
- **Date:** 2026-10-05, accepted 2026-10-06
- **Related:** [ADR-0018](ADR-0018-distribution-licensing-commerce.md), `docs/history/2026-10-05-cross-platform-compatibility-security-plan.md` (history), `docs/engineering/29-desktop-hld-lld.md`, `docs/implementation/masterplan.md` (areas "Platform adapter layer for Windows and Mac", "Microsoft Store package", "Windows installer", "Mac App Store package")

## Context

The Engineering Package Index locked "Windows 11 first; preserve macOS-ready boundaries". The earlier version of this record
proposed shipping Windows first and macOS later, outside the first release. Repository facts when it was written: CI ran on
`windows-latest` only; no platform crate existed; no `target_os = "macos"` code existed; the Store manifest targeted Windows 10
1809 while the README said Windows 11.

On 2026-10-06 the owner decided: build two platform adapters in the same codebase, one for Windows and one for the Mac, and
deliver three separate files: a Microsoft Store package, a Windows installer and a Mac App Store package.

## Decision

1. **Release files.** Every release produces three files from one codebase:
   - a Microsoft Store `.msix` package,
   - a signed Windows installer `.exe` for direct sales, **amended 2026-10-07 by [ADR-0023](ADR-0023-store-trial-and-internal-installer.md): internal build for the owner's team, never sold; a fourth file, an internal Mac `.dmg`, added by [ADR-0024](ADR-0024-internal-team-builds.md),**
   - a Mac App Store `.pkg` package.
2. **Windows 11 only.** The package manifest `MinVersion` is `10.0.22000.0`. Windows 10 is not supported and is not tested.
3. **Apple silicon only.** No Intel Mac build. The AI engine is built with Metal for Apple silicon.
4. **Two adapters behind one boundary.** Feature code never contains `cfg(windows)`, hard-coded Windows paths, or any operating
   system call. Paths, open and reveal, process start and stop, atomic file replace, power state, secure storage, licence
   checks and the updater sit behind a platform boundary with a Windows adapter and a macOS adapter. An architecture lint
   rule enforces this. The refactor changes no behaviour.
5. **Mac App Store rules shape the design.** The Mac build runs in the App Sandbox. It cannot download and run an executable,
   so the AI engine ships inside the package and only model weights are downloaded. It has no self-updater. Project folders
   are reached through security-scoped bookmarks. The AI engine needs the Mac equivalent of the Windows Job Object before the
   assistant ships there.
6. **Security properties are identical across systems** (see `docs/history/2026-10-05-cross-platform-compatibility-security-plan.md` (history)).
7. **Each package passes its own lifecycle tests:** install, upgrade, uninstall and reinstall, offline use, crash recovery and an
   end-to-end run on a clean Windows 11 machine or an Apple silicon Mac.

## Consequences

- The macOS work is on the release path, not after it. The release cannot be called ready until all three files pass their checks.
- Mac builds and tests need Mac hardware and an Apple Developer account (owner tasks in `docs/implementation/owner-decisions.md`).
- The earlier plan to keep macOS outside the release gate no longer applies and is kept here only as history.

## Alternatives rejected

| Alternative | Reason |
|---|---|
| Windows only, macOS later | The owner wants both stores in the first release. |
| Intel and Apple silicon Macs | The owner chose Apple silicon only, which halves the Mac test matrix. |
| Windows 10 and 11 | The owner chose Windows 11 only, which halves the Windows test matrix. |
| Separate codebases per platform | Doubles the work and lets the products drift apart. |

## Implementation notes

- The platform boundary is the crate `crates/openframe-platform` (Windows adapter and macOS adapter, selected at compile time). Module list, adapter state and open work: [37-platform-boundary.md](../engineering/37-platform-boundary.md).
- Lint rule R6 in `scripts/architecture-lint.mjs` forbids operating-system code outside that crate and the reviewed `OS_ALLOW` files.
- The macOS adapter compiles but is unverified until an Apple silicon CI runner exists. CI runs on Windows only today.
