# Handoff: where we are right now

> Rewritten (not appended) at the end of every session. One page. It exists so the next session starts from the facts
> instead of from a long history, which is how context rot is avoided. If this page is older than the newest progress log
> entry, trust the log and fix this page first. Last rewritten: 2026-10-16, late evening (W5 baseline running).

## State

1. **Branch `windows/w8-download`** holds W4 to W8 (W1 to W3 are on `main`). Nothing pushed. Old phase branches were
   deleted after a check that each is contained in `main` or this branch (owner's clean-up decision 2026-10-16).
2. **Owner decisions 2026-10-16** (all in `owner-decisions.md`): models straight from Hugging Face at pinned commits
   (ADR-0032); model set Gemma 3 1B Q8_0 + BGE small English + multilingual E5 small; optimise only computers that can be
   tested (no borrowed laptops); licence compliance is the owner's (never raise it); GSD and `AGENTS.md` removed
   (moved to `.build-cache/removed-2026-10-16/`), claude-mem kept; gold sets approved as they are.
3. **Release run 2026-10-16** (gated release exe): everything passed except specs 04, 72, 76, 78, 97. Fixed since:
   a real download bug (a first graphics check during a new set's health check threw the download away; health check
   now decides placement first; test `a_download_checked_while_the_app_warms_up_keeps_the_new_set`), specs 72 and 76
   (stale bundled-era text), spec 04 helper (PowerShell progress noise, foreground retries). Spec 97 (typing 23 ms p95
   against 16) ran while the PC was busy: needs the clean rerun.
4. **W5 measured** ([evidence](../release/evidence/w5/README.md)): 86 percent of a request is prompt reading; Intel Arc
   7.1 s against 27.2 s per request; 12 threads beat 14; schema and Q4 not worth changing; E5 232 of 252 retrieval cases
   against BGE 202. Changes made: 12 threads on chips with 12+ cores, card probe version 2 (realistic prompt, whole-call
   speed, one key `placement::probe_key`), E5 as third pinned model with `query:`/`passage:` prefixes and
   `SearchChoice::Automatic` = E5. Tools: `ai-eval` (`--search`, `--graphics auto`, `requests.jsonl`, `engine`/`index`
   per result), `engine-bench` (threads, card, reuse, schema, quant, embed).
5. **Running at this rewrite:** the full W5 baseline (`ai-eval --graphics auto`, out `.build-cache/ai-eval/baseline-2026-10-16`).

## Next three items

1. Read the baseline summary; copy `summary.json` and `report.md` to `docs/release/evidence/w5/baseline/`; compare with
   the ADR-0028 bar (90 percent tool, 85 percent end state) and write the figures into the W5 evidence README.
2. Commit, then the final gated release rerun with the PC left alone and on mains power:
   `npm run test:e2e -- --release=gated` (about 2 hours with the build). Fix anything red and rerun only that spec.
3. Evidence and bookkeeping: W4 evidence file, plan boxes with `(ev: ...)`, feature rows, progress log entry, `graphify
   update .`, commit.

## Do not

- Do not raise licensing or ask about the model set or borrowed laptops again (decided).
- Do not build an installer; do not push.
- Do not run measurements while the release suite runs (it times typing and idle CPU).
