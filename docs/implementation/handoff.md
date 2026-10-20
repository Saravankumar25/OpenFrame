# Handoff: where we are right now

> Rewritten (not appended) at the end of every session. One page. Last rewritten: 2026-10-20, after the AI follow-ups.

## State

1. **The Windows final polish and the two AI follow-ups are merged to `main` locally (not pushed).** The AI now reads
   project notes directly for "Summarise my project notes" and keeps short writing (loglines, lines, titles,
   descriptions) to a word budget. Record: the top two entries of the [progress log](progress-log.md).
2. **Checks:** AI unit 93, AI integration 113, clippy and fmt clean. Real-model English evaluation: writing within
   length 100 percent (was 56.7) on the shipped build, safety 40 of 41. The release bar measure (tool 94.4, end state 87.3) was not rerun
   on all 430 cases; only the writing and safety sets changed.
3. **Internal installer:** `release-out/OpenFrame-3.exe` (built from 3fa5433), installed (owner's instruction,
   2026-10-20). E2E on the installed build: every spec file passes except spec 04, whose 100 ms helper precondition
   misses on the installed program (no text lost; same as 2026-10-18). Details in the progress log.

## Next items, in order

1. Owner and team hand test of the installed build (Chapter 1 exit for W10).
2. Fix safety-en-023 (a licence request makes a settings proposal) and make spec 04's 100 ms helper reliable on the
   installed program.
3. Refresh the user manual (.docx): it predates the top bar, rail, removals and AI changes.

## Open blockers

- None for the build. The 100,000-document AI benchmark of the context brief was not built.

## Do not

- Do not push. Do not restyle Windows to look like the Mac. Do not edit `OpenFrameMac/`. Do not commit `changes/`,
  `OpenFrameAI.docx` or `OpenFrameMac/`.

> Rewritten (not appended) at the end of every session. One page. It exists so the next session starts from the facts
> instead of from a long history, which is how context rot is avoided. If this page is older than the newest progress log
> entry, trust the log and fix this page first. Last rewritten: 2026-10-19, end of the Mac M6 to M10 session.

## State

1. **Windows track:** closed at W9 (installer 028d8b6, gate 0 Open); W10 is the team pilot. Unchanged this session.
2. **Mac track (branch `mac/m1-toolchain`):** M1 to M9 built and tested; M10 is the team pilot.
   - All thirteen workspaces, import and export in every format, the Metal AI engine (ADR-0033; English bar met: 96.7
     and 88.6 percent), StoreKit 2 (off in the internal build), Project Settings, shortcuts, custom themes, the
     first-launch welcome, centred and symmetric pages, and the window always inside the screen.
   - Internal disk image: `release-out/mac/OpenFrame-Studio-0.1.0-internal-<commit>.dmg` with its SHA-256, SBOM and
     manifest; the gate is [mac-internal-release-gate.md](../release/mac-internal-release-gate.md).
   - Real-app end-to-end: an injected driver runs about 200 scenarios on the app copied out of the disk image
     (`apps/macos/E2E/run-parallel.sh`); results in `release-out/e2e/`.
   - Gold reference project **Low Tide Radio** in `tests/gold-project/` (490 of 492 operations, exported, re-imported
     and compared).
   - Every Mac screen as HTML in Light and Dark: `release-out/MacHTML/` (regenerate with `apps/macos/E2E/capture.sh`).
3. **GitHub** (owner decision, ADR-0034): the repository holds two folders, `windows/` and `mac/`; GitHub main before
   the change is kept as `backup/main-before-two-folders`.

## Next three items

1. Give the disk image and [the Mac install note](../testing/pilot-kit/mac-install.md) to the team (M10). Checks only a
   person can do are listed in the pilot kit (voice recording, drags, Tab walks).
2. Triage pilot reports with `docs/testing/pilot-kit/triage.md`; fix, rebuild with
   `apps/macos/scripts/build-release-app.sh` and `build-dmg.sh`, rerun the end-to-end suite on the new image.
3. Gaps found by the gold project (dual dialogue editing, fixed breakdown and budget categories, the opening transition,
   weak 1B model answers) in `tests/gold-project/COVERAGE.md`.

## Open blockers

- Owner: install full Xcode for XCUITest and StoreKit testing; licence server address and response key (Chapter 3).

## Do not

- Do not push or merge without the owner's word. Do not keep two copies of the shared core drifting: a core change goes
  into both `windows/` and `mac/` on GitHub (ADR-0034).
- Do not let a SwiftUI view feed a measured size back into its own layout (the Scene Hub crash of 2026-10-19).
