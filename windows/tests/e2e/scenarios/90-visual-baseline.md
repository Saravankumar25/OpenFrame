# Visual baseline and visual comparison

Spec: `tests/e2e/specs/90-visual-baseline.test.mjs`

Tags: none (shared journey data).

Before: `01-journey` and `02-restart` have run on the shared journey data. The app starts on that data. The window is made 1440 by 900 pixels. The appearance is set to Light mode with the OpenFrame Default theme, through the same operations Settings, Appearance uses, so every run captures the same look whatever the computer's own setting is. At the end the appearance is set back to System mode with OpenFrame Default.

"Capture S" means:

1. Move focus off any field.
2. Wait about 1.2 seconds.
3. Check: no visible text says "Saving…" within 15 seconds.
4. Click Dismiss on every toast, and wait about 0.4 seconds.
5. Take a screenshot and compare it with the committed baseline image S in `docs/release/evidence/M0/M0.20-visual-baseline/`.
6. Check: the screenshot has the same size as the baseline.
7. Check: at most 0.1 percent of the compared pixels differ. A pixel differs when its red, green or blue value differs by more than 24 out of 255. The parts that change from run to run by design are left out of the comparison (see Notes).

## The 15 representative screens

1. Check: the screen shows "Your projects".
2. Capture 01-application-home.
3. Click the project row "Open Night Bus".
4. Check: the Workspaces navigation shows.
5. If an enabled Keep Saved Version button shows within 4 seconds, click it and check it goes away.
6. Press Ctrl+K, type 'there that night' and click the result containing "INT. POLICE STATION".
7. Check: the workspace shows "Draft 1 from Story Board" and the cursor sits on "INT. POLICE STATION [em dash] NIGHT".
8. Wait about 3 seconds, then check no visible text says "Saving…" (so the remembered place is saved the same way on every run).
9. Click Home. Capture 02-project-home.
10. Click Idea Vault. Capture 03-idea-vault.
11. Click Story. Capture 04-story-board.
12. Click the Outline tab. Capture 05-story-outline.
13. Click Screenplay. Capture 06-screenplay-editor.
14. Click Breakdown. Capture 07-breakdown.
15. Click Production. Capture 08-production-overview.
16. Click the Catalog tab in the Production navigation. Capture 09-production-catalog.
17. Click the Schedule tab in the Production navigation. Capture 10-production-schedule.
18. Click Call Sheets. Capture 11-call-sheets.
19. Click Files. Capture 12-files.
20. Click the project button "Project: Night Bus. Switch or manage projects", choose Notes & Tasks. Capture 13-notes-and-tasks.
21. Open the same menu and choose Recently Deleted. Capture 14-recently-deleted.
22. Open the same menu and choose Project settings. Capture 15-project-settings.
23. Check: 15 screens were compared (or updated), and none failed.

## Notes

- On the Mac, Ctrl+K uses Cmd instead.
- Parts left out of the comparison: relative and clock times ("just now", "N minutes ago", "today", "yesterday", "hh:mm"), the "Last modified", "Last opened", "Created" and "Last changed" lines, the project card preview pattern on Application Home (picked from the project's random id), the data folder path on Project settings, and the toast band at the bottom of screens 02 to 04.
- Runner settings change the mode: `E2E_UPDATE_BASELINE=1` writes a new baseline, `E2E_VISUAL_UPDATE=<names>` writes only the named screens, `E2E_VISUAL_BASELINE=<folder>` captures into another folder, and `E2E_VISUAL_PERTURB_CSS` and `E2E_VISUAL_IGNORE` are review aids. A failure writes the screenshot and a difference image to `tests/e2e/artifacts/visual/`.

## Platform notes

- The committed baseline is 2160 by 1350 pixels: 1440 by 900 at 150 percent Windows display scaling, drawn by WebView2. Mac: the SwiftUI shell draws its own screens, so it needs its own baseline at a Retina scale; to decide in phase M-track.
