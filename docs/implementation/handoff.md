# Handoff: where we are right now

> Rewritten (not appended) at the end of every session. One page. It exists so the next session starts from the facts
> instead of from a long history, which is how context rot is avoided. If this page is older than the newest progress log
> entry, trust the log and fix this page first. Last rewritten: 2026-10-20, end of the Mac and Windows parity session.

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
     every element, screenplay zoom, resizable AI panel, shot list Grid, whole Story Board cards, the relationship map
     layout, Breakdown Export, Compare named Scene-only and Text-only.
   - New in the shared core: `schedule.export_daily` ("Shoot Day N - Daily view", PDF, CSV, XLSX). Windows does not
     call it yet.
   - Requirement inventory and status: [mac-windows-structure-parity.md](../roadmap/mac-windows-structure-parity.md).

## Next three items

1. Merge `mac/windows-parity` into `mac/m1-toolchain` after the owner looks at the Mac app (dev build:
   `apps/macos/scripts/bundle-dev-app.sh`). Then the internal disk image only on the owner's word.
2. Open items of the parity page: a visible search button, the "+" create menu, Breakdown hide scenes and panel, the
   Catalog filter row at the smallest window with the AI panel open, the screenplay overlap at 900 points with the
   sidebar and AI open (known since 5355dc7).
3. Windows: call `schedule.export_daily` from the Daily View, and choose one name for the AI disclosure ("See sources"
   on the Mac, "Details" on Windows).

## Open blockers

- Owner: install full Xcode for XCUITest and StoreKit testing; licence server address and response key (Chapter 3).
- The AI end-to-end scenarios need the model download fixture; they were not run this session.

## Do not

- Do not push or merge without the owner's word. A core change goes into both `windows/` and `mac/` on GitHub
  (ADR-0034); the Daily View export is such a change.
- Do not let a SwiftUI view feed a measured size back into its own layout (the Scene Hub crash of 2026-10-19).
- Do not put a hub page inside another accessibility container when it is shown as one place: SwiftUI folds the page's
  own identifier away (`hubContainer` in ProjectNavigation.swift).
