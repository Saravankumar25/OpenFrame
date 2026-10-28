# Handoff: where we are right now

> Rewritten (not appended) at the end of every session. One page. It exists so the next session starts from the facts
> instead of from a long history, which is how context rot is avoided. If this page is older than the newest progress log
> entry, trust the log and fix this page first. Last rewritten: 2026-10-28, end of the Mac sync and release session.

## State

1. **Branch `mac/windows-sync`** (local, not pushed) holds all Mac work: the sync with Windows `114dff50`, the Swift
   ports, the security review fixes, the team password removal (ADR-0039) and the E2E fixes. Every other Mac branch and
   local `main` are already ancestors of it (fast-forward possible). Unmerged and old: `claude/website-html-mockups-9ecfa7`
   (superseded by MacHTML; delete only on the owner's word).
2. **Installed app:** `/Applications/OpenFrame Studio.app`, from `release-out/mac/OpenFrame-5.dmg` (no team password).
   Rebuild and reinstall with `apps/macos/scripts/install-internal-build.sh`.
3. **Tests:** Swift 784 pass; Rust 2664 pass; web 735 pass; lint, links, writing, migrations clean. Full E2E on the
   release app (before the last fixes): 194 pass, 30 fail. The rerun after the fixes is in the progress log entry.

## Next session: MacHTML recapture (2 to 3 hours, owner order of 2026-10-27)

The prompt to paste is [next-session-prompt-machtml.md](next-session-prompt-machtml.md). Summary:

The last MacHTML was vague: windows not at full size, some screens not captured properly, the old theme. Recapture all
of it with the Screening Room theme (Light and Dark), consistently, with no mistakes:
1. Build the dev app (`apps/macos/scripts/bundle-dev-app.sh`) and the driver; run
   `apps/macos/E2E/capture.sh "apps/macos/build/OpenFrame Studio.app"` with ONE runner (no `OPENFRAME_E2E_PARALLEL`),
   the screen unlocked and nothing else on screen.
2. Before capturing, make the capture tour open the window at the full visible screen size (not minimised, not the
   default size) and keep it key and frontmost for every state; check `CaptureTour*.swift` and the window-fit helpers.
3. After the run, open every page and compare it with the real app: the whole window, the right screen, the active
   (not dimmed) look, the new theme in both modes. Recapture any state that is wrong; never leave a known-bad page.
4. Keep `MacHTML/README.md` honest (what each page is, the limits that remain).

## Next session: cleanup (owner order)

- Merge `mac/windows-sync` into `main` (fast-forward) and delete the merged branches; `HTML_Mockups/` and the old
  mockups branch only with the owner's word; remove finished builds and caches (`target/`, `.build-cache` app copies,
  `release-out/e2e`, the release staging folder in `$(getconf DARWIN_USER_TEMP_DIR)openframe-release-stage`).
- Dead code pass on the Swift app and the core; the two Word manuals still describe the team password.

## Open E2E items (from the release run; check which the rebuild fixed)

- 14 keyboard scenarios (96-*) need System Settings > Keyboard > Keyboard navigation on (owner setting, off here).
- 42 custom shortcut survives relaunch: hangs at the second launch on release builds only.
- 86 moodboard drag: tile bounds measured wrong; the three sketch Undo ones should pass with the driver's sheet fix.
- 91 sixteen themes: 322 screens reported hard to read; may be the Settings window behind being read, to settle.
- 01-journey 13 (search palette still open at the AI step), 75 (Cmd+J: now a menu command), 78 download (low disk
  at the time) and 78 remove: rerun.
- 95 window fit at the minimum size: rerun after the pane fixes.

## Owner decisions open

- AI tool bar (90%) missed on both platforms after the conversational rebuild (Mac 73.9%, Windows 69.6%).
- Identity in `library.json` is not authentication (doc 40 section 2): keep or change.
- Store items needing Xcode and the developer account: `docs/release/mac-app-store-readiness.md`.

## Do not

- Do not push, merge to a remote or build a store package without the owner's word.
- Do not run two E2E runners or two SwiftPM builds at once; build outside the E2E lock.
- Do not leave build caches behind: clear `target/` after each finished build step (owner decision 2026-10-27).
- Repeat on Windows every shared-core change listed in
  `docs/release/evidence/mac/windows-sync-2026-10-27/mac-only-changes.md` and the password removal (ADR-0034).
