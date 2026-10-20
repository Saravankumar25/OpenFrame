# Handoff: where we are right now

> Rewritten (not appended) at the end of every session. One page. Last rewritten: 2026-10-20, after the AI follow-ups.

## State

1. **The Windows final polish and the two AI follow-ups are merged to `main` locally (not pushed).** The AI now reads
   project notes directly for "Summarise my project notes" and keeps short writing (loglines, lines, titles,
   descriptions) to a word budget. Record: the top two entries of the [progress log](progress-log.md).
2. **Checks:** AI unit 93, AI integration 113, clippy and fmt clean. Real-model English evaluation: writing within
   length 100 percent (was 56.7) on the shipped build, safety 40 of 41. The release bar measure (tool 94.4, end state 87.3) was not rerun
   on all 430 cases; only the writing and safety sets changed.
3. **Internal installer:** `release-out/OpenFrame-Studio-0.1.0-internal-3fa5433.exe`, installed (owner's instruction,
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
