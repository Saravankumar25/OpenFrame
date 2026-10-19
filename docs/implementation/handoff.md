# Handoff: where we are right now

> Rewritten (not appended) at the end of every session. One page. Last rewritten: 2026-10-20, overnight run.

## State

1. **Branch `windows/final-polish`.** Owner (2026-10-20, going to sleep): finish everything autonomously, all
   permissions granted, build and verify a new internal .exe, run the E2E release suite, clean up, do not miss anything
   from `changes/`, `OpenFrameAI.docx`, `Downloads/OPENFRAME_WINDOWS_UX_AUDIT_AND_REDESIGN_CLAUDE_CODE_MASTER_PROMPT.md`,
   `Downloads/OPENFRAME_WINDOWS_AI_CONTEXT_ENGINEERING_CLAUDE_CODE_MASTER_PROMPT.md`, `Downloads/openframe-chnages.docx`.
   The Windows UI keeps its own design (owner, 2026-10-19). Do not edit `OpenFrameMac/`.
2. **Checklist with status:** [final-polish-plan.md](final-polish-plan.md), section "Overnight checklist".
3. **Done and committed:** change list items 1 to 11, 13 to 21; openframe-chnages.docx 1 (strip colours) and 2 (Story
   Board columns, story.css); AI fixes incl. scene facts, dialogue facts, follow-ups, product-help gating (0ced174).
   AI integration tests 110 of 110, AI unit tests 88 of 88, web tests 608 of 608 at last run.
4. **PDF redesign:** a Sonnet subagent is finishing it (files: `crates/openframe-import-export/**`,
   `modules/exports/**`, `tests/production/{exports,pdf_regression,pdf_samples,pdf_design,main}.rs`). Do not edit
   those files while it runs. Review its samples in `release-out/pdf-samples/` before committing.

## Next items, in order

1. Monochrome themes (Black on White, White on Black) added to `themes.json` and theme tests; Daily View export
   (owner item 12) after the PDF agent finishes.
2. UX geometry pass: an E2E spec that opens every route with the AI panel and measures overlap, page-level horizontal
   overflow and clipped controls at 1280x720 and 1920x1080; fix what it finds; `docs/ux/` audit notes.
3. Real-model AI evaluation (`ai-eval --only en`), then fmt, clippy, cargo test, web tests, governance, E2E suite,
   installer (`npm run installer:internal`), install and smoke, cleanup, progress log, local merge to main.

## Do not

- Do not push. Do not restyle Windows to look like the Mac. Do not edit `OpenFrameMac/`.
