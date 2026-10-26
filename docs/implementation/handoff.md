# Handoff: where we are right now

> Rewritten (not appended) at the end of every session. One page. Last rewritten: 2026-10-26 (conversational assistant:
> slices S0 to S8 built and measured on branch `ai/conversational-s0`, local, not merged, not pushed).

## State

1. **Branch `ai/conversational-s0`** holds the whole conversational assistant of
   [ADR-0037](../adr/ADR-0037-conversational-assistant-focus-and-dossiers.md): eval fixtures and 274 gold cases (S0), coverage
   (S2), project index, resolver and focus (S3), answer type read (S4), facts with guards (S5), memory and the Settings list
   (S6), one write and check path with the five lanes removed (S7), prompt caching and warm-up (S8). How a message runs now:
   [ai-answer-paths.md](../engineering/ai-answer-paths.md). Design and every measurement: [ai-conversational-harness.md](../specs/ai-conversational-harness.md)
   sections 5, 12, 13 and 14. `main` does not have it yet.
2. **Measured on the Low Tide fixtures** (release, processor, 274 cases;
   `docs/release/evidence/ai-conversational-after-s8-2026-10-26/` against `...-baseline-2026-10-26/`): answer facts 17.6% to
   64.0%, required facts 25.9% to 68.3%, length class 65.7% to 93.2%, no re-ask 83.1% to 97.2%, repetition 6.0% to 0.4%,
   median 16.7 s to 4.4 s, p95 30.8 s to 22.9 s. Answer type on 40 unseen messages: 87.5% exact, 92.5% by group.
3. **A fix round** after that run (screen navigation, "open" read as a verb, searches that asked which screenplay, the
   missing-data sentence, raw field names in fallbacks) and its re-run: see the first entry of the progress log for the
   result.
4. Owner decisions today: long-term memory on, visible and deletable; DeepSeek Harness not installed, ideas borrowed (spec
   section 14). Recorded in `owner-decisions.md`.

## Next items, in order

1. **Close the bars that are still open** (spec 7.5): required facts on explain and relation turns, correct abstention,
   the exact answer type (87.5% against 90%), tool and arguments on the navigation and change cases. Start from the
   weakest families in the latest report (character web, story hop, money).
2. **Human reading of 50 answers** (spec 7.6) on the Low Tide run, marked good or bad, before the numbers count as a release
   candidate. The gold sets of 2026-10-24 and today's still need the owner's approval.
3. **Merge `ai/conversational-s0` into `main` locally** once the owner agrees, then the 40 end-to-end specs with the real
   model and a new internal `.exe` only on the owner's word.

## Open blockers

- Owner: approve the bars of spec 7.5 and the gold sets; say whether to merge the branch.
- Not run: the 40 end-to-end UI specs on the new path; the Mac port of these slices (Mac track 1b).

## Do not

- Do not push without the owner's word. Do not `git pull` origin/main here. Do not edit `OpenFrameMac/`. Do not build an
  installer.
- Do not add a deterministic answer without an entry in spec section 4.1.
- Do not claim a result that was not measured. State what was run, on what data, how many times.
- Do not tune the answer type rules on `turn-sets/fresh-40.jsonl`; it is the held-out set. Write a new one if it is needed.
- Tool gates in this workspace ask for a short statement of facts before the first write or edit of a file; give it, then retry.
