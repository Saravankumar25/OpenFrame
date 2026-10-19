# Handoff: where we are right now

> Rewritten (not appended) at the end of every session. One page. It exists so the next session starts from the facts
> instead of from a long history, which is how context rot is avoided. If this page is older than the newest progress log
> entry, trust the log and fix this page first. Last rewritten: 2026-10-19, during the Windows final polish session.

## State

1. **Branch `windows/final-polish`** (from `docs/user-manual-windows-build`, which is one commit ahead of `main`). Work
   plan: [final-polish-plan.md](final-polish-plan.md). Owner sources are in the git-ignored `changes/`,
   `OpenFrameAI.docx` and `OpenFrameMac/` (reference only, never commit them; `.git/info/exclude` lists them).
2. **Owner decision 2026-10-19:** the Windows UI keeps its own design; it is not unified with the Mac app. Only the
   owner's listed changes are made, in the Windows design.
3. **Done and committed (checkpoint 1114758):** the owner change list items 1 to 11 and 13 to 21 (see the plan), the AI
   repair in the core and the chat view, removal of Review, four package types and new Episodic or Series projects.
   Web tests 608 of 608 and AI integration tests 108 of 108 passed at that point.
4. **Done, not yet committed:** Breakdown scene list toggle (`workspaces/breakdown/index.tsx`).
5. **Paused at the owner's request (2026-10-19, laptop shutdown).** Last commit holds everything, including
   unfinished work marked WIP. Not verified after the last edits: the product-help gating
   (`commands.rs` `may_be_product_question`, `toolbox.rs`) was written but its unit tests did not finish.
6. **AI evaluation on the real model (English, 420 cases, before the gating change):** tool and arguments 94.0% (bar
   90%), end state 87.3% (bar 85%), bar met. Report: `.build-cache/ai-eval-after/report.md` (copy it into
   `docs/release/evidence/` when the run is final). Case poison-en-001 showed product help answering a story question,
   which the gating change fixes; rerun the evaluation after it.
7. **PDF redesign subagent stopped mid-way** (it was rewriting the screenplay renderer in
   `crates/openframe-import-export/src/pdf_export.rs` for change spec 01). That crate may not compile until it is
   finished. Its other work: `report.rs`, `report/` folder, `modules/exports/**` including `style.rs`, and
   `tests/production/{pdf_samples,main}.rs`. Before it, the call sheet, schedule, DOOD, breakdown and catalog changes
   were in progress; none has been reviewed. It also left two em dashes in `modules/exports/schedule.rs` (writing
   check fails by two). Resume: run `node scripts/cargo-queued.mjs check -p openframe-application --tests`, read
   `git diff HEAD~1 -- crates/openframe-import-export crates/openframe-application/src/modules/exports`, then continue
   the PDF work (owner wants sample PDFs in `release-out/pdf-samples/` to review)
8. **Earlier note, superseded by item 7:** the PDF redesign (change specs 01 to 05 plus moodboard, storyboard, shot list,
   reports, budget). Its files: `crates/openframe-import-export/**`, `modules/exports/**`,
   `tests/production/{exports,pdf_regression,pdf_samples,main}.rs`. Samples go to `release-out/pdf-samples/`.

## Next items, in order

1. Review the PDF subagent's report and sample pages; commit its work.
2. Daily View export (owner item 12): a day PDF in the new design (new exporter in `modules/exports/`, an operation,
   and an Export button in `workspaces/production/tabs/daily.tsx`).
3. `cargo fmt`, clippy with `-D warnings`, `cargo test --workspace`, `npm run check:governance`, `npm run check:writing`.
4. End-to-end specs: update `tests/e2e/lib/flows.mjs` `undoLabel` (Undo moved to the project menu), the screenplay
   Review and removed package steps, then run the suite.
5. AI evaluation harness on the real model (ADR-0028) before and after; real-app walkthrough with `npm run dev`.
6. Owner asked in this session for the final installer: `npm run installer:internal`, install, verify.
7. Documents: feature status rows, UX notes (Tab now cycles every element), progress log, reconciliation log.

## Do not

- Do not push and do not merge to `main` without the owner's word.
- Do not edit or delete anything in `OpenFrameMac/` (the shared core change for the Mac is noted, not made).
- Do not restyle the Windows UI to look like the Mac.
