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
5. **In progress by a subagent:** the PDF redesign (change specs 01 to 05 plus moodboard, storyboard, shot list,
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
