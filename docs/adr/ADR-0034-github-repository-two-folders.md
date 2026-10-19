# ADR-0034: The GitHub repository holds two folders, windows and mac

- **Status:** Accepted 2026-10-19 (owner decision in the Mac M6 to M10 session).
- **Date:** 2026-10-19
- **Amends:** the "Layout and decisions" rule of `CLAUDE.md` (the repository root holds README, CONTRIBUTING and the
  other root files) for the GitHub repository `Saravankumar25/OpenFrame`. The local working checkout keeps its layout.

## Context

The owner's words (2026-10-19): "the thing is openframe github repo should only have 2 folder named windows and mac and
the 2 codebases must be seperately given in two folders, no compromises held. Make it safe and perfectly done without
losing anything."

Until then the GitHub repository held one tree at its root: the Windows release of Chapter 1 (merged W9 and the manual
testing guide). The Mac track of Chapter 1 was built on the local branch `mac/m1-toolchain`, cut from the same W9
commit, and adds the SwiftUI shell, the Mac AI engine, StoreKit, the real-app end-to-end driver, the internal disk image
and the HTML replica of every Mac screen.

## Decision

1. The root of the GitHub repository holds exactly two folders and nothing else: `windows/` and `mac/`.
2. `windows/` is the Windows codebase exactly as GitHub `main` held it before this change, moved with `git mv` so every
   file keeps its history.
3. `mac/` is the complete Mac codebase: the whole tree of `mac/m1-toolchain` at its final commit, joined with a subtree
   merge so its history stays reachable from `main`. Each folder builds on its own from inside the folder; the shared
   Rust core is therefore present in both. `mac/release/` holds the internal disk image with its SHA-256, SBOM and
   manifest, and `mac/MacHTML/` holds the HTML replica of every Mac screen in Light and Dark.
4. Nothing is lost: before the change, GitHub `main` is kept as the branch `backup/main-before-two-folders`, and the
   full Mac history is pushed as the branch `mac/m1-toolchain`.

## Consequences

- The two copies of the shared core can drift. A core change made for one platform must be made in the other folder in
  the same change, and recorded in both folders' progress logs.
- GitHub reads workflows only from the root `.github/`, so the CI workflows inside `windows/.github` and `mac/.github`
  do not run on GitHub until a root workflow calls them. The local checks (`cargo test`, `apps/macos/scripts/test.sh`,
  the end-to-end runner) are the gate in the meantime.
- Commands in the documents are written from inside a folder (`cd mac` or `cd windows` first).
