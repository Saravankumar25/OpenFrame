# Handoff: where we are right now

> Rewritten (not appended) at the end of every session. One page. It exists so the next session starts from the facts
> instead of from a long history, which is how context rot is avoided. If this page is older than the newest progress log
> entry, trust the log and fix this page first. Last rewritten: 2026-10-18, after the Mac M1 to M5 session.

## State

1. **Windows track:** unchanged since the W9 endgame (branch `windows/w9-endgame`, not merged; installer and gate as in
   [windows-internal-release-gate.md](../release/windows-internal-release-gate.md)). W10 pilot waits on the owner.
2. **Mac track, branch `mac/m1-toolchain`** (cut from `windows/w9-endgame`, not merged, nothing pushed), on the owner's
   Apple M1 Mac (macOS 26.5.2, Rust 1.98.1, Swift 6.3.3 from the Command Line Tools, **no full Xcode**):
   - M1 done: core tests pass on Apple silicon; Tauri Mac shell retired; parity matrix
     ([mac-parity-matrix.md](../roadmap/mac-parity-matrix.md)); `mac-core.yml` CI. Evidence:
     [m1-m2](../release/evidence/mac/m1-m2-core-and-interface.md).
   - M2 done: `crates/openframe-ffi` (UniFFI), XCFramework, 819 generated Swift types and 492 typed operations.
   - M3, M4, M5 built: SwiftUI shell, Home, team gate, 16 themes, Settings; the native TextKit 2 editor with drafts,
     compare, review, comments, scene hub, page setup; Story, Idea Vault (project and global), Breakdown.
     **229 Swift tests pass.** Evidence: [m3-m5](../release/evidence/mac/m3-m5-shell-editor-workspaces.md),
     [editor spike](../release/evidence/mac/m4-editor-spike.md).
3. **Editor engine choice** (ADR-0027 open item 1): built on TextKit 2 **provisionally**, under the owner's instruction of
   2026-10-17 to decide while away. Waiting for the owner to confirm (owner-decisions.md).

## Next three items

1. Owner: confirm the TextKit 2 editor choice; install full Xcode (needed for window-level UI tests and the `.dmg`).
2. Close the "Partly built" rows of M3 to M5 in the parity matrix (project duplicate, archive, delete and locate; command
   overlay and global search; undo toasts; focus mode and Writing Room; title page preview; About window).
3. Start M6 (production workspaces) or M8 (Mac AI); M5 to M7 may run in any order (phase page section 2).

## Open blockers

- Full Xcode is not installed: no XCUITest, no `xcodebuild`, no signing. Swift Testing runs without it
  (`apps/macos/scripts/test.sh`).
- Owner decisions S7, S11, S12, S13 and the W10 pilot (Windows), unchanged.

## Do not

- Do not push or merge either branch without the owner's word. Do not build the `.dmg` (M9, owner's word).
- Do not change the Tauri adapter or other Windows-only code: the Windows build evidence stays valid only while the app
  code it was built from is unchanged. Mac-only fixes in shared code go behind `cfg(unix)` or into the Mac crate.
- Do not run Swift test suites in parallel (`--no-parallel` is in the script for a reason).
- Keep disk use low: about 39 GB free; `target/` about 11 GB, `apps/macos/.build` about 2 GB. Never `rm -rf` a worktree.
