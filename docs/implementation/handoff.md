# Handoff: where we are right now

> Rewritten (not appended) at the end of every session. One page. It exists so the next session starts from the facts
> instead of from a long history, which is how context rot is avoided. If this page is older than the newest progress log
> entry, trust the log and fix this page first. Last rewritten: 2026-10-27, Mac sync with Windows main `114dff50`.

## State

1. **Windows track:** source of truth on GitHub `main` `windows/` (local Windows `114dff50`): conversational assistant
   (ADR-0037), production PDF v2 (ADR-0036), screenplay v5, storage and library setup (ADR-0038), owner fixes.
2. **Mac track, branch `mac/windows-sync`** (from `mac/screening-room-theme`, local only, not pushed, not merged; 20
   commits). The Mac is level with Windows `114dff50`:
   - Shared Rust core merged three ways (change list and reasons:
     `docs/release/evidence/mac/windows-sync-2026-10-27/mac-only-changes.md`). Migrations byte for byte as Windows,
     0017 replaced by the Windows file (owner decision 2026-10-27). Mac keeps Metal placement, FFI, gold project.
   - Mac storage specifics: library suggested at `~/OpenFrame`; pointer at
     `~/Library/Application Support/OpenFrame Library/library-pointer.json`; Finder files skipped in moves; App Store
     build hides every Move, Locate and Change Folder (bookmarks not built).
   - Swift screens ported: assistant panel (progress, Cancel, Save, memory, See sources, navigation), Story page (A4,
     formatting, colours, zoom, Dark page, rich paste), Beat Board Vertical/Horizontal, screenplay Dark page, relationship
     maps, page setup first-page number, call sheet in PDF order, Sides selected cast, Daily View export, Home cover and
     Found on this Mac, first-start library setup, Settings > Storage, Make Them Mine, Add Them Back, Command+Q with a
     sheet open.
   - Bugs found and fixed beyond Windows: read-only `-wal`/`-shm` blocked a writable project on macOS (core,
     regression test); Story page saves refused after a move; older text could overwrite newer; a late AI answer broke
     the next question's Cancel; Project Settings hid its buttons' ids; quit ignored with a sheet open.
3. **Measured:** see the progress log entry of 2026-10-27 (sync) and
   `docs/release/evidence/mac/windows-sync-2026-10-27/` (baseline, after-port runs, PDF check, screens).

## Next three items

1. Owner at the Mac with the screen unlocked: rerun `72-ai-assistant/conversation-is-still-there-after-a-relaunch`
   and `72-ai-assistant/stop-offline-ai-frees-the-engines` (`.build-cache/run-ai-e2e-aifix.sh 72-ai-assistant`), the
   full Swift suite on a quiet machine, and look at the new screens in the dev app
   (`apps/macos/scripts/bundle-dev-app.sh`).
2. Owner decision: the AI tool bar (90%) is not met after the conversational rebuild on either platform (Windows
   69.6%, Mac 73.9% on Low Tide; read questions are answered from facts, not read tools). Keep, change or split the bar.
3. Repeat on Windows the Mac fixes listed under "Made during the merge" in the change list (ADR-0034), then merge
   `mac/windows-sync` into `mac/m1-toolchain` on the owner's word.

4. The early engine start test fails on the Mac: the free memory reading leaves out reclaimable memory. Fix the
   reading in the shared core (both platforms), then measure the early start (port orders phase 6).

## Open blockers

- Owner: full Xcode for XCUITest and StoreKit testing; security-scoped bookmarks for the store build (Chapter 3).
- Not tested here: App Store sandbox, bookmarks, a real reinstall, the store build.

## Do not

- Do not push, merge or build a `.dmg` or release app without the owner's word.
- Do not run E2E with the screen locked: keys and screenshots go nowhere (the 2026-10-27 screenshots of some states
  show inactive drawing or the opening screen; they prove the flow ran, not the look).
- Do not run two E2E runners at once: use the lock in `.build-cache/lanes/e2e-rules.md`.
- Do not let a SwiftUI view feed a measured size back into its own layout (the Scene Hub crash of 2026-10-19).
- Do not run the eval with relative engine or model folders on Windows: it measured "AI unavailable" on the Mac until
  fixed (`c54ecedd`).
