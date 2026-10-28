# Handoff: where we are right now

> Rewritten (not appended) at the end of every session. One page. It exists so the next session starts from the facts
> instead of from a long history, which is how context rot is avoided. If this page is older than the newest progress log
> entry, trust the log and fix this page first. Last rewritten: 2026-10-28, end of the MacHTML recapture session.

## State

1. **Branch `mac/windows-sync`** (local, not pushed) holds all Mac work: the sync with Windows `114dff50`, the Swift
   ports, the security review fixes, the team password removal (ADR-0039), the E2E fixes, three Mac UI fixes and the
   recapture driver fixes, all committed locally. Every other Mac branch and local `main` are already
   ancestors of it (fast-forward possible). Unmerged and old: `claude/website-html-mockups-9ecfa7` (superseded by
   MacHTML; delete only on the owner's word).
2. **Installed app:** `/Applications/OpenFrame Studio.app`, from `release-out/mac/OpenFrame-5.dmg` (no team password).
   Unchanged this session. Rebuild and reinstall with `apps/macos/scripts/install-internal-build.sh`.
3. **Tests:** Swift 784 pass; Rust 2664 pass; web 735 pass; lint, links, writing, migrations clean (Swift rerun on 2026-10-28 after the UI fixes, 784 pass; the rest from the
   previous session). Full E2E on the release app (before the last fixes):
   194 pass, 30 fail.

## Done this session (MacHTML recapture)

- **Three Mac UI fixes** from the owner screenshots of 2026-10-28, each confirmed in real captures: budget row Edit and
  Delete are icon buttons (pencil, trash) with tooltips and accessibility labels (`BudgetWorkspace.swift`); the Global
  Idea Vault sheet opened from Home has a Done button in its header and Escape closes it (`IdeaVaultWorkspace.swift`,
  `HomeView.swift`); the Story page placeholder disappears as soon as the first character is typed
  (`StoryPageTextView.swift`).
- **MacHTML rebuilt** from the real development build: window at the full visible screen (1280 x 628 points on this
  Mac), frontmost and active for every picture, Screening Room theme in Light and Dark, 760 screens (1520 pages), every page passing the pixel and text check in both modes.
  README: `MacHTML/README.md`. The tour also covers the screens added on 2026-10-27 (library setup on a new Mac,
  Settings > Storage with a project move, Story page typing, dark sheet and zoom, Beat Board rows, screenplay dark
  page, cover photo, Found on this Mac). Capture folders are in `release-out/capture/` (git-ignored); the final run is
  `release-out/capture/full-20261028-030147` with the reruns merged in.
- **Driver fixes found during the capture:** pop-up menus are opened and closed with AppKit's menu tracking
  notifications (an open menu blocked quitting and 9 scenarios failed at quit); `UI.click` posts mouse events to the
  queue instead of sending them straight to the window (a table tracking the mouse deadlocked the vault scenario); the
  project sidebar is hidden with a checked helper (`capHideSidebar`) because a press on the toolbar button did not
  always take; `ProductionScenarios.menuButton` reuses the capture helper.

## Next items

1. **Rerun the full regular E2E** (one runner) on the dev or release app. Not verified: the regular suite was not rerun
   after the driver changes to `click`, `menuButton` and the pop-up helpers, so do not trust E2E results until it is.
2. **App bug found by the capture:** right after Save Snapshot, a reports or sides snapshot sometimes stays on its
   spinner (2 of 3 runs; `SidesModel.save` calls `openSnapshot(id)` the same way). The tour works around it for reports
   by opening the saved row again; the MacHTML page `schedule-sides-snapshot` still shows the spinner. Fix in the app,
   then recapture `06-production-days`.
3. **A new disk image** with this session's three UI fixes, on the owner's word (`mac/release/OpenFrame-5.dmg` on GitHub
   is build 183, made before them).
4. **Cleanup left:** `HTML_Mockups/` and the old mockups branch only after asking; remove finished builds and caches
   (`target/`, `.build-cache` app copies, `release-out/e2e`, the release staging folder in
   `$(getconf DARWIN_USER_TEMP_DIR)openframe-release-stage`). Dead code pass on the Swift app and the core; the two Word
   manuals still describe the team password. Never captured by design in MacHTML: real AI answers and Assistant memory
   (the dev build has no AI engine), system Open and Save panels, drag and drop, microphone, print panel.

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
