# AI evaluation report

- Cases: 302
- Gold folder: crates/straft-ai-eval/gold
- Model files: the pinned Offline AI of packaging/ai/ai-assets.lock.json
- Build: release
- Search model: Automatic
- Engine place: off (processor)
- Sampling: deterministic (temperature 0, fixed seed)
- Wall time: 92.3 min

## English release bar (ADR-0028 decision 2)

| Measure | Result | Bar |
|---|---|---|
| Correct tool and arguments (tool and action cases) | none | 90% |
| Correct end state after apply (action cases) | none | 85% |

Bar met: **no**

## By kind

| kind | Cases | Setup errors | Right tool offered | Tool chosen | Tool and arguments | Schema valid | End state | Answer facts | Language | Length | Safety | Not empty | Grounding | Recall@5 | Recall@10 | MRR | Median ms | p95 ms | Engine MB |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| conversation | 302 | 0 | none | none | none | 100.0% (302/302) | none | 71.7% (109/152) | 100.0% (302/302) | 90.0% (36/40) | none | 100.0% (302/302) | 64.0% | none | none | none | 10473 | 53689 | 3467 |

## By language

| language | Cases | Setup errors | Right tool offered | Tool chosen | Tool and arguments | Schema valid | End state | Answer facts | Language | Length | Safety | Not empty | Grounding | Recall@5 | Recall@10 | MRR | Median ms | p95 ms | Engine MB |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| en | 302 | 0 | none | none | none | 100.0% (302/302) | none | 71.7% (109/152) | 100.0% (302/302) | 90.0% (36/40) | none | 100.0% (302/302) | 64.0% | none | none | none | 10473 | 53689 | 3467 |

## Answer path and fallback

Cases by kind and the answer path they took.

| Kind | write | navigate | change | fixed | Cases |
|---|---|---|---|---|---|
| conversation | 252 | 1 | 0 | 49 | 302 |
| all | 252 | 1 | 0 | 49 | 302 |

Cases by kind and the fallback they used. `last_resort` is the last-resort answer; `abstained` is a could-not-find answer.

| Kind | none | last_resort | abstained | Cases |
|---|---|---|---|---|
| conversation | 178 | 46 | 78 | 302 |
| all | 178 | 46 | 78 | 302 |

## Conversation bench

Deterministic checks only; the rubric and human labels come at development time. Invented names, wrong scenes and missing quotes are rates over all names, scene citations and quotes in the answers.

| Question type | Cases | Setup errors | Answer facts | Invented names | Wrong scene citations | Missing quotes | Within length | Correct abstention | Repetition | Median ms |
|---|---|---|---|---|---|---|---|---|---|---|
| all | 302 | 0 | 71.7% (109/152) | 39.7% (372/937) | 0.0% (0/80) | 61.3% (19/31) | 90.0% (36/40) | 33.3% (1/3) | 2.3% (7/302) | 10473 |
| breakdown | 6 | 0 | 100.0% (2/2) | 50.0% (6/12) | 0.0% (0/3) | none | 100.0% (1/1) | none | 0.0% (0/6) | 13548 |
| call | 6 | 0 | 100.0% (1/1) | 62.5% (5/8) | none | none | 100.0% (1/1) | none | 0.0% (0/6) | 15222 |
| conv | 24 | 0 | none | 5.6% (7/124) | 0.0% (0/16) | 50.0% (1/2) | 91.7% (22/24) | none | 0.0% (0/24) | 33757 |
| files | 6 | 0 | 100.0% (1/1) | 100.0% (5/5) | none | none | none | none | 0.0% (0/6) | 10463 |
| help | 12 | 0 | 100.0% (1/1) | 79.7% (59/74) | none | none | none | none | 0.0% (0/12) | 9660 |
| idea | 12 | 0 | 100.0% (2/2) | 43.5% (10/23) | none | 100.0% (2/2) | none | none | 0.0% (0/12) | 6882 |
| neg | 52 | 0 | 98.0% (48/49) | 85.4% (82/96) | 0.0% (0/6) | 75.0% (3/4) | none | 33.3% (1/3) | 0.0% (0/52) | 2519 |
| notes | 12 | 0 | 83.3% (5/6) | 35.3% (6/17) | 0.0% (0/1) | 100.0% (1/1) | none | none | 0.0% (0/12) | 6913 |
| pair | 29 | 0 | 41.2% (7/17) | 24.5% (24/98) | 0.0% (0/16) | 50.0% (4/8) | 50.0% (1/2) | none | 0.0% (0/29) | 8809 |
| production | 78 | 0 | 61.9% (26/42) | 42.1% (109/259) | 0.0% (0/14) | 50.0% (3/6) | 83.3% (5/6) | none | 9.0% (7/78) | 12279 |
| screenplay | 6 | 0 | 25.0% (1/4) | 0.0% (0/18) | 0.0% (0/7) | 50.0% (2/4) | 100.0% (2/2) | none | 0.0% (0/6) | 14165 |
| story | 24 | 0 | 60.0% (6/10) | 19.3% (16/83) | 0.0% (0/1) | 100.0% (3/3) | 100.0% (2/2) | none | 0.0% (0/24) | 9853 |
| triple | 5 | 0 | 33.3% (1/3) | 16.3% (7/43) | 0.0% (0/10) | none | 100.0% (2/2) | none | 0.0% (0/5) | 27304 |
| turn | 30 | 0 | 57.1% (8/14) | 46.8% (36/77) | 0.0% (0/6) | 0.0% (0/1) | none | none | 0.0% (0/30) | 11861 |

Expectation flags (harness spec 7.4). Cases that set none of them show none. Flags the build cannot measure yet are listed as not counted, never as passed or failed.

| Question type | Required facts | Abstain names | No navigation | No data dump | No internal words | Length class | No re-ask | Answer type | Answer group | Not counted |
|---|---|---|---|---|---|---|---|---|---|---|
| all | 49.7% (169/340) | 33.3% (1/3) | 100.0% (49/49) | none | 100.0% (302/302) | 85.1% (223/262) | 100.0% (18/18) | 73.5% (197/268) | 89.2% (239/268) | 0 |
| breakdown | 71.4% (5/7) | none | 100.0% (1/1) | none | 100.0% (6/6) | 100.0% (5/5) | none | 80.0% (4/5) | 100.0% (5/5) | 0 |
| call | 20.0% (2/10) | none | 100.0% (1/1) | none | 100.0% (6/6) | 80.0% (4/5) | none | 80.0% (4/5) | 100.0% (5/5) | 0 |
| conv | 62.2% (46/74) | none | 100.0% (24/24) | none | 100.0% (24/24) | none | none | 100.0% (6/6) | 100.0% (6/6) | 0 |
| files | 0.0% (0/8) | none | none | none | 100.0% (6/6) | 100.0% (6/6) | none | 100.0% (6/6) | 100.0% (6/6) | 0 |
| help | 81.8% (18/22) | none | 100.0% (1/1) | none | 100.0% (12/12) | 58.3% (7/12) | none | 41.7% (5/12) | 41.7% (5/12) | 0 |
| idea | 52.9% (9/17) | none | none | none | 100.0% (12/12) | 75.0% (9/12) | none | 66.7% (8/12) | 75.0% (9/12) | 0 |
| neg | 50.0% (1/2) | 33.3% (1/3) | 100.0% (1/1) | none | 100.0% (52/52) | 88.5% (46/52) | none | 80.8% (42/52) | 96.2% (50/52) | 0 |
| notes | 40.0% (6/15) | none | 100.0% (1/1) | none | 100.0% (12/12) | 75.0% (9/12) | none | 66.7% (8/12) | 83.3% (10/12) | 0 |
| pair | 38.2% (21/55) | none | 100.0% (5/5) | none | 100.0% (29/29) | 92.6% (25/27) | none | 55.6% (15/27) | 92.6% (25/27) | 0 |
| production | 47.7% (31/65) | none | 100.0% (7/7) | none | 100.0% (78/78) | 87.5% (63/72) | none | 80.6% (58/72) | 94.4% (68/72) | 0 |
| screenplay | 100.0% (6/6) | none | 100.0% (2/2) | none | 100.0% (6/6) | 100.0% (4/4) | none | 75.0% (3/4) | 75.0% (3/4) | 0 |
| story | 32.0% (8/25) | none | 100.0% (2/2) | none | 100.0% (24/24) | 77.3% (17/22) | none | 77.3% (17/22) | 86.4% (19/22) | 0 |
| triple | 66.7% (8/12) | none | 100.0% (3/3) | none | 100.0% (5/5) | 100.0% (3/3) | none | 33.3% (1/3) | 100.0% (3/3) | 0 |
| turn | 36.4% (8/22) | none | 100.0% (1/1) | none | 100.0% (30/30) | 83.3% (25/30) | 100.0% (18/18) | 66.7% (20/30) | 83.3% (25/30) | 0 |

Pointer (spec 12.4). Precision is the share of pointers that went to the expected leaf, recall the share of expected pointers that fired right, false pointers the share of cases that expect none and still got one. Bars: precision and recall at least 90 percent, false pointers at most 5 percent.

| Question type | Expect a pointer | Pointed | Precision | Recall | False pointers |
|---|---|---|---|---|---|
| all | 51 | 39 | 94.9% (37/39) | 72.5% (37/51) | 0.8% (2/251) |
| breakdown | 0 | 0 | none | none | 0.0% (0/6) |
| call | 0 | 0 | none | none | 0.0% (0/6) |
| conv | 0 | 0 | none | none | 0.0% (0/24) |
| files | 0 | 1 | 0.0% (0/1) | none | 16.7% (1/6) |
| help | 0 | 0 | none | none | 0.0% (0/12) |
| idea | 0 | 0 | none | none | 0.0% (0/12) |
| neg | 49 | 35 | 100.0% (35/35) | 71.4% (35/49) | 0.0% (0/3) |
| notes | 0 | 0 | none | none | 0.0% (0/12) |
| pair | 0 | 0 | none | none | 0.0% (0/29) |
| production | 0 | 1 | 0.0% (0/1) | none | 1.3% (1/78) |
| screenplay | 0 | 0 | none | none | 0.0% (0/6) |
| story | 0 | 0 | none | none | 0.0% (0/24) |
| triple | 0 | 0 | none | none | 0.0% (0/5) |
| turn | 2 | 2 | 100.0% (2/2) | 100.0% (2/2) | 0.0% (0/28) |

Narrative answers (spec 12.5): the turn read the expected narrative type and long length class.

| Question type | Narrative type | Long length class |
|---|---|---|
| all | 100.0% (40/40) | 100.0% (40/40) |
| breakdown | 100.0% (1/1) | 100.0% (1/1) |
| call | 100.0% (1/1) | 100.0% (1/1) |
| conv | 100.0% (24/24) | 100.0% (24/24) |
| files | none | none |
| help | none | none |
| idea | none | none |
| neg | none | none |
| notes | none | none |
| pair | 100.0% (2/2) | 100.0% (2/2) |
| production | 100.0% (6/6) | 100.0% (6/6) |
| screenplay | 100.0% (2/2) | 100.0% (2/2) |
| story | 100.0% (2/2) | 100.0% (2/2) |
| triple | 100.0% (2/2) | 100.0% (2/2) |
| turn | none | none |

Writing quality is scored separately against each case's rubric at development time (ADR-0028 decision 9); this table covers only the deterministic measures.

## Latency (spec section 7.5)

| Measure | Value | Bar |
|---|---|---|
| Median, all questions (302 asked) | 10.5 s | 15.0 s or lower: met |
| 95th percentile, all questions | 53.7 s | 30.0 s or lower: not met |
| Median, questions that used the engine (271) | 11.6 s | |
| 95th percentile, same | 54.6 s | |
| First question after the project opened | 5.2 s | |
| First question that used the engine | 5.2 s | |
| Prompt tokens served from stored work | 59.9% (331497 of 553620) | |
| Completion tokens, engine calls | 58685, 668 | |

Bars by length class (spec 12.6): short answers 15 s median and 30 s 95th percentile; the long classes (`long250`, `long350`) 30 s and 60 s, the ADR-0028 limit on a processor.

| Length class | Questions | Median | 95th percentile | Bar (median, 95th) | Met |
|---|---|---|---|---|---|
| long250 | 49 | 37.0 s | 69.8 s | 30.0 s, 60.0 s | not met |
| long350 | 8 | 51.7 s | 124.6 s | 30.0 s, 60.0 s | not met |
| one_sentence | 152 | 7.9 s | 20.0 s | 15.0 s, 30.0 s | met |
| paragraph | 25 | 15.5 s | 66.0 s | 15.0 s, 30.0 s | not met |
| short | 68 | 10.0 s | 27.7 s | 15.0 s, 30.0 s | met |

| Stage | Mean per question |
|---|---|
| dossier | 21 ms |
| engine_generation | 10763 ms |
| engine_prompt | 7950 ms |
| turn | 967 ms |

Stage times come from the AI trace and from what the request recorded; a stage nobody records is not listed. Engine prompt and generation are the engine's own figures and include the earlier turns a conversation case asks first, so they add up to more than the time of the last question.
