# Baseline of the current assistant on the Low Tide gold sets, 2026-10-26

Build: commit 0d13cdb9 (before slices S2 and S3), release, processor engine, deterministic sampling. Cases: the 251
conversation and 23 tool cases on `low-tide-two-screenplays`, `thin-project` and `big-project` (`--only lt-`). Wall time
121.5 min; other builds ran on the same machine, so times are pessimistic.

| Measure | Result |
|---|---|
| Answer facts (`answer_has`) | 17.6% (24 of 136) |
| Required facts | 25.9% (49 of 189) |
| Correct abstention | 15.0% (3 of 20); abstain names 0 of 20 |
| Length class respected | 65.7% |
| No re-ask after the screenplay was named | 83.1% (59 of 71) |
| No navigation on explain turns | 85.1% |
| No data dump to an upset user | 85.7% |
| Answers that asked "which screenplay" | 30 of 251 (38 with `ai.ambiguous`) |
| Lane taken | agent 213, direct 35, action 2, chat 1 |
| Median and p95 time, conversation | 16.7 s, 30.8 s |
| Tool and arguments, tool and action cases | 91.3% (21 of 23); end state 85.7% |

The invented-name rate of the old bench (47.4%) is not comparable: it checks names against the screenplay text only, and
these fixtures hold names in cast, crew and budget data. Files: `report.md`, `summary.json`, `results.jsonl` (the
11 MB request log stays in `.build-cache`).
