# Handoff: where we are right now

> Rewritten (not appended) at the end of every session. One page. It exists so the next session starts from the facts
> instead of from a long history, which is how context rot is avoided. If this page is older than the newest progress log
> entry, trust the log and fix this page first. Last rewritten: 2026-11-05, after build 259 and the push of `mac/`.

## State

1. **The product is Straft Studio, its assistant Straft AI** (ADR-0042). Mac bundle id `app.straftstudio.studio`, products `app.straftstudio.studio.trial7` and `.lifetime`. The `.of*` package extensions stay, owner's decision.
2. **Build 259 (`Straft-12.dmg`, commit `0d048fd3`) is installed in `/Applications`.** It adds the eight official accounts to the Welcome window. The complete end-to-end run is on build 256 (221 passed, 0 failed, 3 blocked, 9 skipped); by owner's instruction build 259 was tested only where it changed: Swift 817 of 817, `99-store` 7 of 7, first start 2 of 2, 58 of 58 before the repeat run was stopped. Evidence: `docs/release/evidence/mac/e2e-targeted-build-259-2026-11-05.md`.
3. **Old data carries over** by the Windows rules: app data copied, models hard linked, library and project folders renamed, paths updated in one transaction, nothing deleted (`legacy_migration.rs`, `legacy_rename.rs`). First start on a copy of real data passes: `apps/macos/E2E/first-launch-old-data.sh`. The owner's real data was migrated on 2026-11-04 by an earlier build that moved the models; the code keeps working with that.
4. **Store build limit, tested with a probe:** a sandboxed build sees only its container, so it starts empty and cannot read internal build data ([evidence](../release/evidence/mac/store-sandbox-old-data-2026-11-04.md)). The way across is a package restored in the store build.
5. **MacHTML** has 771 screens from installed build 259, sample name and folders (`apps/macos/E2E/capture.sh`). Capture from the installed app: the development build has no AI engine, so 19 pages per mode wrongly showed Straft AI as missing. Library setup alone needs the debug executable.
6. **Team password:** none on the Mac (owner decision 2026-10-27). The Windows tree on GitHub still has `team_gate`. Owner to say which is meant.

## Next items

1. **Owner, first:** unlock the Mac and allow Documents access. Build 259 is open on real data and waits at "Opening Straft Studio" for that macOS prompt, which a new ad hoc signature asks again. A test run never touches Documents, so build 259 has not yet been seen on real data. The cause is read from the blocked `opendir` and the prompt process, not seen on screen: if the app stays stuck after allowing, treat it as a bug.
2. **Owner:** the three drag scenarios of `86-visual-planning/` passed by hand on build 259 but have never passed automatically. To automate them, allow computer control of `app.straftstudio.studio.e2e1` or turn on Accessibility for the app that starts the run.
3. **Owner:** should the eight accounts also appear in the Help menu? Today they show only in the Welcome window, after purchase.
4. **Owner:** the store steps in `docs/release/mac-store-test.md`, then the AI bar decision below.

## Owner decisions open

- Team password on internal builds: the Mac has none, the owner's last message describes one.
- Does the Mac tree keep `apps/desktop` (the build and two tests read from it), `HTML_Mockups/` (only documents cite it), and the untracked `straft-history.bundle`?
- Removing Straft AI frees no disk while the old app data folder still holds its hard links, and nothing in the app cleans that folder.
- AI tool bar on the older sets missed (58.5% against 90%), and whether those sets still count. The current section set passes 324 of 336 and the injection set 60 of 60 after the name change.
- Identity in `library.json` is not authentication: keep or change.

## Do not

- Do not write the old product name anywhere except behind the `LEGACY` marker.
- Do not stage `Straft_Release_Package/` or `release-out/`.
- Do not run the MacHTML capture beside anything heavy, and do not run two SwiftPM builds at once.
- Do not repeat tests that passed on unchanged code: diff against the last clean build and test what changed (owner, 2026-11-05).
- Do not point a test at a home copy without rewriting every stored path: the database holds absolute paths, and the script's checks exist because a run reached the real library once.
- Do not leave build caches behind: clear `target/` after each finished build step (owner decision 2026-10-27).
