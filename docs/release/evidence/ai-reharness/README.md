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

## After tuning the section answers

Three real-model runs of the section set (`--only sec-`), same fixture, same model, 393 cases, deterministic sampling, about 29
minutes each, on the owner's M1 with an E2E run and other builds sharing the machine, so timings only compare loosely. Run r6 was
made before slice R7. The fix list came from reading every failing case of r6 (`release-out/eval/r6-causes.md`, not tracked).
Final run: `after-tuning/report.md` and `after-tuning/summary.json`.

| Measure | r6 | tune 1 | tune 2 | Final (tune 3) |
|---|---|---|---|---|
| Answer facts | 64.0% (215 of 336) | 91.4% (307) | 95.5% (321) | **96.4% (324 of 336)** |
| Length | 100% (40 of 40) | 97.5% (39) | 100% (40) | 100% (40 of 40) |
| Not empty | 100% | 99.7% | 100% | 100% |
| Pointer precision | 100% (57 of 57) | 100% (59 of 59) | 100% (59 of 59) | 100% (59 of 59) |
| Pointer recall | 95.0% (57 of 60) | 98.3% | 98.3% | 98.3% (59 of 60) |
| False pointers | 0 of 333 | 0 | 0 | 0 of 333 |
| Correct abstention | 8 of 15 | 10 | 10 | 10 of 15 |
| Invented names, rate over names in answers | 37.7% | 38.1% | 37.3% | 37.4% |
| Wrong scene citations | 0 of 38 | 0 of 33 | 0 of 34 | 0 of 39 |
| Repeated four-word run | 5 of 393 | 7 | 7 | 8 of 393 |
| Last resort / abstained | 40 / 57 | 20 / 4 | 21 / 4 | 22 / 4 |
| Median, all questions | 2.0 s | 2.3 s | 2.1 s | 2.3 s |
| 95th percentile, all questions | 21.7 s | 15.5 s | 12.4 s | 16.0 s |
| Median one sentence / narrative / story | 1.6 / 7.4 / 26.4 s | 1.6 / 6.3 / 31.7 s | 1.6 / 5.6 / 13.7 s | 1.6 / 7.3 / 19.0 s |

Runs compare with these limits:

- **The evaluator changed once.** R7 made a Scenes pick with no chosen screenplay ask which one, so on this two-screenplay fixture
  the run would have measured that question. The evaluator now chooses the longest screenplay for a Scenes pick that names none,
  as a person does in the picker. Tune 1 and tune 2 still measured the question on 9 cases; the final run does not.
- **Six gold lines were corrected** where `FACTS.md` or the script proves them wrong (the list is in the commit that holds them).
- **Repetition rose by 3.** A kept answer with a code sentence can say a run of words twice. It is a measure here with no bar.

The new `scene_ref` check fired once in each run, on "scenes 1, 2, 3" for a day that holds 6, 13, 24 and 42. That is a true alarm: no
false alarm was seen. `held_back` fired on 8 cases in tune 2; each attempt had said "could not find" while a row held the answer. The
relationship and mood checks stay off: nothing here measures them.

Still failing, 12 cases: the writer gets the facts and tells them wrongly on 9 (scenes-007, guide-001, pair-01-1, pair-03-2, pair-08-1,
pair-08-2, narr-brief-002, 003 and 004, of which the three briefs invent detail the checks do not catch), and follow-013 keeps to one
sentence and drops the fish stall. triple-1-2 needs a join from a scene's speakers to their actors and their availability on the day,
which no fetcher makes. triple-2-1 tells the scene without the word roof.

## The older gold sets on the tuned build

Same 844 English cases as the baseline, on the tuned binary from before the last review fixes and the rename (the rename changed the words of
the prompts, so these numbers could move a little), 90.5 minutes with builds and E2E sharing the machine. They run through `ai_eval_all_sections`, which reads every
section, a road no person can take: the app always has picks. `legacy-after-tuning/report.md` and `summary.json`.

| Measure | Baseline | Tuned | Bar |
|---|---|---|---|
| Correct tool and arguments | 59.0% (113) | 58.5% (111) | 90%, **still missed**, as before this work |
| Correct end state | 89.5% (77 of 86) | 89.5% (77 of 86) | 85%, met |
| Conversation answer facts | 64.6% (124 of 192) | 58.9% (113 of 192) | none, **went down** |
| Paraphrase answer facts | 40.0% (16 of 40) | 25.0% (10 of 40) | none, **went down** |
| Tool kind answer facts | 54.8% (17 of 31) | 71.0% (22 of 31) | none, went up |
| Conversation length | 71.7% (43 of 60) | 65.0% (39 of 60) | none, went down |
| Writing length | 46.7% (28 of 60) | 45.0% (27 of 60) | none |
| Repeated four-word run, conversation | 2 | 11 | none, **worse** |
| Retrieval Recall@5 | 99.4% | 99.4% | none |
| Safety | 100% (46 of 46) | 100% (46 of 46) | none |

Reading it: no bar was lowered. The tool bar was missed before this work and this work did not address it, so it stays an owner item.
A likely cause of the fall in answer facts, not proven: every section now feeds one budget, with more grounding per answer (3,568 against
2,040 in conversation), and the rules were tuned for picked sections. The section set is the road the app uses and scores 96.4%, but it is a smaller set
written at development time and no person has spot-checked it. Neither number proves the other wrong: the owner should read both.

## The section set on the final code

After the review fixes and the rename, which changed the words of the prompts: 20.1 minutes, same fixture and model. `final-build/report.md` and `summary.json`.

| Measure | Tune 3 | Final code |
|---|---|---|
| Answer facts | 96.4% (324 of 336) | **95.8% (322 of 336)** |
| Length | 100% (40 of 40) | 97.5% (39 of 40) |
| Pointer precision and recall | 59 of 59, 59 of 60 | 59 of 59, 59 of 60 |
| False pointers | 0 of 333 | 0 of 333 |
| Repeated four-word run | 8 of 393 | 7 of 393 |

Eleven answers moved, both ways, with no change to the rules that read the project: the model's wording shifts when its prompt shifts. Examples a
person would see: `sec-triple-4-1` says "Babu's Boat Yard charges 9,00,000 rupees for shoot days", which joins two real figures wrongly;
`sec-narr-story-002` says it found no plot when the facts hold one; `sec-leaf-settings-001` answers with the raw line of the guide. These stay open.

