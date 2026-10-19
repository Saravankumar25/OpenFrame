# Visual regression matrix (Windows)

> Written 2026-10-20. What automation checks, at which sizes, in which states, and where the evidence is. All runs use
> the debug build through `tests/e2e/run.mjs`; release runs add `--release=gated`.

| Check | Screens | Window sizes | AI panel | Themes | Data | Evidence |
|---|---|---|---|---|---|---|
| Layout geometry (spec 98) | Home, Idea Vault, Story, Screenplay, Breakdown, all 14 Production tabs, Call Sheets, Files, Settings | 1200 by 720, 1280 by 720, 1920 by 1080 | closed and open | default | seeded journey project "Night Bus" | `tests/e2e/artifacts/layout-geometry.json` |
| Visual baseline (spec 90) | 15 screens | runner default | closed | default light | seeded project | baseline PNGs in `docs/release/evidence/M0/M0.20-visual-baseline/`, diffs under `tests/e2e/artifacts/` |
| Appearance themes (spec 91) | the same 15 screens | runner default | closed | 17 built-in themes, light and dark | seeded project | spec log; paper stays white |
| Reachability (spec 95) | every route and production tab | runner default | open | default | seeded project | spec log |
| Keyboard and screen reader (spec 96) | every route | runner default | open and closed | default | seeded project | spec log |
| Journey render pass (spec 01) | every workspace and production tab | runner default | open | default | seeded project | console and unlabelled-control check in the spec log |

A changed visual baseline is regenerated only with `E2E_UPDATE_BASELINE=1`, after the new images are reviewed, and the
reason is written in the progress log.

## Data states

- Empty: a new project with no screenplay (specs 10, 30, 40).
- Seeded: the journey project with imported Fountain, a schedule day and a call sheet (spec 01 onward).
- Large: the performance spec (97) measures a long screenplay. The 100,000-document benchmark of the AI brief was not
  built (see `docs/engineering/ai-answer-paths.md`).

## Not automated

- Reading PDF exports: sample files are written to `release-out/pdf-samples/` for the owner; the PDF golden tests in
  `crates/openframe-import-export/tests/pdf_golden.rs` compare page images.
- Hand review of the installed build by the team (Chapter 1 exit criterion).
