# Handoff: where we are right now

> Rewritten (not appended) at the end of every session. One page. Last rewritten: 2026-10-25 (upgrade session: AI
> harness, PDF exports v2, screenplay v5, owner issues). The session stopped at the usage limit.

## State

1. **Branch `upgrade/harness-exports-screenplay`** (local, not pushed, not merged to `main`) holds the AI harness
   rebuild S1 to S8, the merged screenplay v5 and owner-issue work (`09265cbe`), the FDX suite CRLF fix, and docs
   (history moved to `docs/history/`, ADR-0035, ADR-0036, owner decisions of 2026-10-24, Mac port list).
2. **Exports are on two other branches**, not yet merged into the upgrade branch:
   - `exports/pdf-design-v2` (worktree `C:\OpenFrame-wt\exports`): shared renderer, Call Sheet, Daily View,
     Schedule, DOOD, Breakdown/Catalog, Budget. Agent C7 (Moodboard, Beat Board Outline and Card Board) was still
     running there when the session stopped: check its commits and `git status` first.
   - `exports/pdf-design-v2-b` (worktree `C:\OpenFrame-wt\screenplay`): Shot List, Storyboard, six production
     reports, plus a merge of the first branch at `fbe615dd`.
3. **AI measurements** (real model; baseline committed in `docs/release/evidence/ai-harness-2026-10-24/baseline/`):
   held-out paraphrases 20% to 35.6%; gold tools 100% to 98.8%; conversation within-length 55% to 87%, abstention
   0/4 to 3/4, answer facts 43% to about 48%; safety and poisoning 100%. A final run of the S8 binary was started in
   `.build-cache/ai-final/` (`all.log`, one `report.md` per run): read it and commit its reports as evidence.

## Next items, in order

1. Check C7 in `C:\OpenFrame-wt\exports`; merge `exports/pdf-design-v2` into `exports/pdf-design-v2-b`, run the
   import-export and production tests, then merge that branch into `upgrade/harness-exports-screenplay`.
2. Full verify on the upgrade branch (`npm run verify`; `scripts/tests/governance.test.mjs` fails on a missing
   `zz-capture` scenario file under `tests/e2e`, not yet investigated), then the one full E2E at the end.
3. Commit the final AI reports, update `feature-status.md` and the progress log, run `graphify update .`, remove
   both worktrees safely (unlink the `node_modules` junctions first) and delete their `target-*` build folders.

## Open blockers

- None. Paraphrase accuracy (35.6%) is far below the 90% bar; the gold sets need owner approval to count.

## Do not

- Do not push or build installers. Do not `rm -rf` a worktree folder. The held-out sets were used once for intent
  threshold calibration (S8): write a fresh held-out set before quoting a final honest number.
