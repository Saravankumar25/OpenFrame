# Handoff: where we are right now

> Rewritten (not appended) at the end of every session. One page. It exists so the next session starts from the facts
> instead of from a long history, which is how context rot is avoided. If this page is older than the newest progress log
> entry, trust the log and fix this page first. Last rewritten: 2026-10-20, late night, end of the MacHTML rebuild.

## State

1. **Windows track:** closed at W9; the final polish, the AI repair and the PDF redesign are on GitHub `windows/`
   (commit `324cd963`). W10 is the team hand test of `OpenFrame-3.exe`.
2. **Mac track, branch `mac/windows-parity`** (from `mac/m1-toolchain`, not merged, not pushed):
   - The Windows code is merged in (`af5e41f`), so the shared Rust core is one core again: the AI repair (exact counts,
     project questions never answered with product help), the PDF redesign, the 17 themes with Black and White and
     OpenFrame Default's own dark palette, and the removed exchange packages.
   - The Mac has the Windows structure in Mac styling: flat sidebar (Home, Idea Vault, Story, Screenplay, Breakdown,
     Production, Call Sheets, Files), Production with the 13 Windows tabs and a new Overview, Back and Forward,
     Settings with the seven Windows pages (new Writing page), plain AI answers with a closed "See sources", Tab round
     every element, screenplay zoom, resizable AI panel, shot list Grid, whole Beat Board cards, the relationship map
     layout, Breakdown Export, Compare named Scene-only and Text-only.
   - Owner requests of the afternoon, both apps: "Story Board" is "Beat Board" everywhere; a free Story page left of
     Board (table `story_page`, migration 0017; TXT, PDF and DOCX export). Windows TypeScript type check, lint and the
     611 web unit tests pass on the Mac (`npm ci`, `tsc`, `eslint`, `vitest`); the Tauri E2E needs the Windows machine.
   - New in the shared core: `schedule.export_daily` ("Shoot Day N - Daily view", PDF, CSV, XLSX), used by the Daily
     View on both apps. The AI disclosure is "See sources" on both apps (owner decision 2026-10-20).
   - Requirement inventory and status: [mac-windows-structure-parity.md](../roadmap/mac-windows-structure-parity.md).

   - Evening round: every Mac button named; Home cards and every page header as on Windows; the screenplay's controls
     in a page bar (they no longer fit the window toolbar); Breakdown Hide scenes and Hide panel; Settings buttons
     fixed (`openSettings`); eight deterministic AI routes (Mac evaluation 98.1 percent tool, 89.9 percent end state).
   - The E2E runner tests a copy with its own bundle identifier and clears its preferences per scenario: the owner's
     own use of the dev app otherwise leaks into the scenarios.

3. **MacHTML rebuilt, branch `mac/machtml-rebuild`** (from `mac/windows-parity`, not merged, not pushed): 692 states of
   the Mac app, each in Light and Dark, as standalone HTML pages in `MacHTML/` (git-ignored, 583 MB; regenerate with
   `apps/macos/E2E/capture.sh "apps/macos/build/OpenFrame Studio.app"`, `OPENFRAME_E2E_PARALLEL=4` for speed). Each page
   is the 2x window picture cut into its containers plus real HTML for every accessibility element. 692 of 692 pages
   pass the WebKit check (0.5 percent pixels, channel over 24) in both modes with 0 HTML misses. The old 179-screen
   `release-out/MacHTML/` is deleted. Limits are in `MacHTML/README.md` (window heights vary by a few points, some
   windows drawn inactive, no AI answers, no images).

## Next three items

1. Merge `mac/windows-parity` (and `mac/machtml-rebuild` on top of it) into `mac/m1-toolchain` after the owner looks at
   the Mac app (dev build: `apps/macos/scripts/bundle-dev-app.sh`). Then the internal disk image only on the owner's
   word. Owner to decide whether MacHTML pictures are committed (583 MB) or kept generated.
2. Windows machine: `npm run verify` (needs `cargo-deny` and `cargo-audit`, not installed on the Mac) and the Tauri
   E2E suite for the Story page, Beat Board, See sources and the Daily View export.
3. Remaining AI misses (comment on the current draft, pin a note, sequence without an act): measure, then route.

## Open blockers

- Owner: install full Xcode for XCUITest and StoreKit testing; licence server address and response key (Chapter 3).
- The AI end-to-end scenarios need the model download fixture; they were not run this session.

## Do not

- Do not run the capture tour with more runners than the display can show frontmost if inactive-looking windows matter;
  app bugs the tour found are listed in the progress log entry of 2026-10-20 (late night).

- Do not push or merge without the owner's word. A core change goes into both `windows/` and `mac/` on GitHub
  (ADR-0034); the Daily View export is such a change.
- Do not let a SwiftUI view feed a measured size back into its own layout (the Scene Hub crash of 2026-10-19).
- Do not put a hub page inside another accessibility container when it is shown as one place: SwiftUI folds the page's
  own identifier away (`hubContainer` in ProjectNavigation.swift).
