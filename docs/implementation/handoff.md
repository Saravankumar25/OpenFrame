# Handoff: where we are right now

> Rewritten (not appended) at the end of every session. One page. Last rewritten: 2026-10-25 (upgrade session: AI
> harness, PDF exports v2, screenplay v5, owner issues, OpenFrame-4.exe).

## State

1. **`main` holds the whole upgrade** (merge 94b1942b, local, not pushed): AI harness rebuild (lanes, Chat answers
   from code-built briefings, code verifiers and sanitizer, typo-tolerant and intent routing, whole-script recipe,
   Cancel and progress lines, Save conversation for review), production PDF design v2 (20 designs, logo footer),
   screenplay v5 (36 pt scene gaps, keep rules, continuation markers, first-page number setting, title-page gutter),
   Sides fixes, and the owner's five issues (relationship maps, side panels, Dark page, Sides selected cast, cover
   photos). Docs: `docs/history/` is the single history folder; ADR-0035 and ADR-0036; owner decisions of 2026-10-24.
2. **Installer:** `release-out/OpenFrame-4.exe` (SHA-256 de953fd9...e5a27b), installed on this PC; gated end-to-end on
   the installed build passed 40 spec files. Details and every number in the top entry of `progress-log.md`.
3. **AI quality** (real model; evidence in `docs/release/evidence/ai-harness-2026-10-24/`): held-out paraphrases
   20% to 46.7% correct tool, conversation invented names 13.1% to 7.7%, answer facts 42.9% to 46.4%, safety 100%.
   Still far below the 90% tool bar on reworded requests.

## Next items, in order

1. Owner hand test of `OpenFrame-4.exe` (AI panel answers and Cancel, relationship maps, Dark page, Sides selected cast,
   cover photo, a few PDF exports).
2. Write a fresh held-out paraphrase set (the current one was used for threshold calibration), then re-measure on the
   current build, including poisoning, writing and routes-off, which the final pass did not run.
3. Push to GitHub `windows/` by the subtree method only on the owner's word; then the Mac port list
   (`docs/roadmap/mac-port-list-2026-10-24.md`).

## Open blockers

- None. The gold sets need the owner's approval before the AI numbers count as official.

## Do not

- Do not push without the owner's word. Do not `git pull` origin/main here. Do not edit `OpenFrameMac/`.
