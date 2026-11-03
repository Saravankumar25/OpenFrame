# AI re-harness evidence

ADR-0041, measure first. Runs on the owner's M1 Mac, 2026-11-03, engine on the graphics card, deterministic sampling, release
build of `ai-eval`, the pinned Gemma 3 1B. No model judges an answer at runtime; these are development-time checks.

## R0: baseline, before any runtime change

Commit ea643476 (Track A), every English case of the existing gold sets, 844 cases, 106.9 minutes.
`baseline/report.md` and `baseline/summary.json`. Other builds compiled on the machine during the run, so its timings read high
and only compare loosely.

| Measure | Result | Bar |
|---|---|---|
| Correct tool and arguments | 59.0% | 90%, missed before this work (the open owner item in the handoff) |
| Correct end state | 89.5% | 85% |
| Conversation answer facts | 64.6% (124 of 192) | none |
| Conversation length | 71.7% (43 of 60) | none |
| Retrieval Recall@5 | 99.4% | none |
| Safety | 100% (46 of 46) | none |

## New test set

`crates/openframe-ai-eval/gold/sections.lowtide.en.jsonl`, 395 cases written at development time from the fixture's `FACTS.md`:
225 per leaf, 30 pairs, 10 triples, 60 out-of-section negatives, 10 look-alikes for false pointers, 40 conversational, 20 follow-ups.

**Smaller than the design asked.** The design says about 30 questions per leaf. This set has about 10 per leaf with data, and
2 for each of the seven leaves the fixture holds nothing for (`vault/global`, `story/page`, `story/outline`, `story/timeline`,
`story/episodes`, `production/sides`, `production/reports`). Reason: time overnight on one Mac. A person has not spot-checked
one in ten yet (ADR-0028 decision 9): that is the owner's task.
