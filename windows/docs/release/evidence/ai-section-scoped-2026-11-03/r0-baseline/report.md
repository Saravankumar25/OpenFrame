# AI evaluation report

- Cases: 274
- Gold folder: crates/straft-ai-eval/gold
- Model files: the pinned Offline AI of packaging/ai/ai-assets.lock.json
- Build: release
- Search model: Automatic
- Engine place: off (processor)
- Sampling: deterministic (temperature 0, fixed seed)
- Wall time: 82.0 min

## English release bar (ADR-0028 decision 2)

| Measure | Result | Bar |
|---|---|---|
| Correct tool and arguments (tool and action cases) | 73.9% | 90% |
| Correct end state after apply (action cases) | 85.7% | 85% |

Bar met: **no**

## By kind

| kind | Cases | Setup errors | Right tool offered | Tool chosen | Tool and arguments | Schema valid | End state | Answer facts | Language | Length | Safety | Not empty | Grounding | Recall@5 | Recall@10 | MRR | Median ms | p95 ms | Engine MB |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| action | 7 | 0 | 100.0% (7/7) | 100.0% (7/7) | 85.7% (6/7) | 100.0% (7/7) | 85.7% (6/7) | none | 100.0% (7/7) | none | none | 100.0% (7/7) | 92.9% | none | none | none | 38 | 2403 | 1786 |
| conversation | 251 | 0 | none | none | none | 100.0% (251/251) | none | 69.1% (94/136) | 100.0% (251/251) | none | none | 100.0% (251/251) | 63.9% | none | none | none | 5640 | 28118 | 2191 |
| tool | 16 | 0 | 100.0% (16/16) | 68.8% (11/16) | 68.8% (11/16) | 100.0% (16/16) | none | 100.0% (4/4) | 100.0% (16/16) | none | none | 100.0% (16/16) | 55.6% | none | none | none | 20 | 14397 | 1820 |

## By language

| language | Cases | Setup errors | Right tool offered | Tool chosen | Tool and arguments | Schema valid | End state | Answer facts | Language | Length | Safety | Not empty | Grounding | Recall@5 | Recall@10 | MRR | Median ms | p95 ms | Engine MB |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| en | 274 | 0 | 100.0% (23/23) | 78.3% (18/23) | 73.9% (17/23) | 100.0% (274/274) | 85.7% (6/7) | 70.0% (98/140) | 100.0% (274/274) | none | none | 100.0% (274/274) | 63.8% | none | none | none | 5353 | 27706 | 2191 |

## Answer path and fallback

Cases by kind and the answer path they took.

| Kind | write | navigate | change | fixed | Cases |
|---|---|---|---|---|---|
| action | 0 | 0 | 7 | 0 | 7 |
| conversation | 222 | 18 | 0 | 11 | 251 |
| tool | 5 | 11 | 0 | 0 | 16 |
| all | 227 | 29 | 7 | 11 | 274 |

Cases by kind and the fallback they used. `last_resort` is the last-resort answer; `abstained` is a could-not-find answer.

| Kind | none | last_resort | abstained | Cases |
|---|---|---|---|---|
| action | 7 | 0 | 0 | 7 |
| conversation | 171 | 31 | 49 | 251 |
| tool | 13 | 1 | 2 | 16 |
| all | 191 | 32 | 51 | 274 |

## Conversation bench

Deterministic checks only; the rubric and human labels come at development time. Invented names, wrong scenes and missing quotes are rates over all names, scene citations and quotes in the answers.

| Question type | Cases | Setup errors | Answer facts | Invented names | Wrong scene citations | Missing quotes | Within length | Correct abstention | Repetition | Median ms |
|---|---|---|---|---|---|---|---|---|---|---|
| all | 251 | 0 | 69.1% (94/136) | 34.0% (217/639) | 1.5% (1/66) | 56.8% (25/44) | none | 70.0% (14/20) | 0.4% (1/251) | 5640 |
| big | 5 | 0 | 100.0% (3/3) | 44.4% (4/9) | 0.0% (0/1) | 100.0% (2/2) | none | none | 0.0% (0/5) | 2020 |
| brain | 11 | 0 | none | 12.8% (5/39) | 0.0% (0/2) | 66.7% (2/3) | none | none | 0.0% (0/11) | 18544 |
| char | 22 | 0 | 100.0% (4/4) | 5.2% (5/96) | none | 0.0% (0/1) | none | none | 0.0% (0/22) | 8269 |
| export | 40 | 0 | 73.9% (17/23) | 38.1% (40/105) | 0.0% (0/13) | 0.0% (0/2) | none | none | 0.0% (0/40) | 5220 |
| help | 11 | 0 | 54.5% (6/11) | 66.7% (30/45) | none | 60.0% (3/5) | none | none | 0.0% (0/11) | 6748 |
| hop | 13 | 0 | 55.6% (5/9) | 27.5% (11/40) | 0.0% (0/12) | 83.3% (5/6) | none | none | 0.0% (0/13) | 8061 |
| hostile | 11 | 0 | none | 100.0% (4/4) | none | none | none | none | 0.0% (0/11) | 2749 |
| lang | 9 | 0 | 25.0% (1/4) | 25.9% (7/27) | 0.0% (0/1) | 50.0% (1/2) | none | 100.0% (1/1) | 0.0% (0/9) | 7272 |
| missing | 19 | 0 | 100.0% (1/1) | 53.1% (17/32) | 0.0% (0/3) | 60.0% (3/5) | none | 70.6% (12/17) | 0.0% (0/19) | 3108 |
| money | 27 | 0 | 70.8% (17/24) | 76.7% (46/60) | none | 66.7% (2/3) | none | none | 0.0% (0/27) | 5625 |
| prod | 30 | 0 | 77.8% (14/18) | 56.0% (28/50) | 0.0% (0/1) | 66.7% (2/3) | none | 0.0% (0/1) | 0.0% (0/30) | 6634 |
| repair | 11 | 0 | 100.0% (7/7) | 29.6% (8/27) | 0.0% (0/4) | 0.0% (0/1) | none | none | 0.0% (0/11) | 3707 |
| scene | 30 | 0 | 57.1% (12/21) | 15.4% (12/78) | 0.0% (0/16) | 50.0% (2/4) | none | 100.0% (1/1) | 3.3% (1/30) | 5369 |
| xref | 12 | 0 | 63.6% (7/11) | 0.0% (0/27) | 7.7% (1/13) | 42.9% (3/7) | none | none | 0.0% (0/12) | 4830 |

Expectation flags (harness spec 7.4). Cases that set none of them show none. Flags the build cannot measure yet are listed as not counted, never as passed or failed.

| Question type | Required facts | Abstain names | No navigation | No data dump | No internal words | Length class | No re-ask | Answer type | Answer group | Not counted |
|---|---|---|---|---|---|---|---|---|---|---|
| all | 75.7% (143/189) | 70.0% (14/20) | 97.0% (65/67) | 95.2% (20/21) | 100.0% (251/251) | 91.6% (230/251) | 97.2% (69/71) | 82.9% (208/251) | 91.2% (229/251) | 0 |
| big | 100.0% (5/5) | none | none | none | 100.0% (5/5) | 100.0% (5/5) | none | 100.0% (5/5) | 100.0% (5/5) | 0 |
| brain | 75.0% (9/12) | none | 90.9% (10/11) | none | 100.0% (11/11) | 81.8% (9/11) | none | 72.7% (8/11) | 72.7% (8/11) | 0 |
| char | 85.7% (30/35) | none | 100.0% (11/11) | none | 100.0% (22/22) | 72.7% (16/22) | 100.0% (3/3) | 63.6% (14/22) | 68.2% (15/22) | 0 |
| export | 78.6% (22/28) | none | 100.0% (13/13) | 90.0% (9/10) | 100.0% (40/40) | 97.5% (39/40) | 97.4% (38/39) | 90.0% (36/40) | 92.5% (37/40) | 0 |
| help | none | none | none | none | 100.0% (11/11) | 81.8% (9/11) | none | 90.9% (10/11) | 90.9% (10/11) | 0 |
| hop | 75.0% (9/12) | none | 100.0% (2/2) | none | 100.0% (13/13) | 92.3% (12/13) | 100.0% (6/6) | 84.6% (11/13) | 100.0% (13/13) | 0 |
| hostile | none | none | none | 100.0% (11/11) | 100.0% (11/11) | 100.0% (11/11) | 100.0% (1/1) | 100.0% (11/11) | 100.0% (11/11) | 0 |
| lang | 83.3% (5/6) | 100.0% (1/1) | 100.0% (3/3) | none | 100.0% (9/9) | 88.9% (8/9) | none | 66.7% (6/9) | 100.0% (9/9) | 0 |
| missing | 100.0% (2/2) | 70.6% (12/17) | 0.0% (0/1) | none | 100.0% (19/19) | 100.0% (19/19) | none | 89.5% (17/19) | 94.7% (18/19) | 0 |
| money | 53.3% (8/15) | none | 100.0% (8/8) | none | 100.0% (27/27) | 92.6% (25/27) | 100.0% (1/1) | 77.8% (21/27) | 96.3% (26/27) | 0 |
| prod | 63.6% (14/22) | 0.0% (0/1) | none | none | 100.0% (30/30) | 100.0% (30/30) | none | 86.7% (26/30) | 96.7% (29/30) | 0 |
| repair | 66.7% (6/9) | none | 100.0% (6/6) | none | 100.0% (11/11) | 90.9% (10/11) | 100.0% (11/11) | 90.9% (10/11) | 90.9% (10/11) | 0 |
| scene | 75.0% (27/36) | 100.0% (1/1) | 100.0% (6/6) | none | 100.0% (30/30) | 90.0% (27/30) | 90.0% (9/10) | 76.7% (23/30) | 93.3% (28/30) | 0 |
| xref | 85.7% (6/7) | none | 100.0% (6/6) | none | 100.0% (12/12) | 83.3% (10/12) | none | 83.3% (10/12) | 83.3% (10/12) | 0 |

Writing quality is scored separately against each case's rubric at development time (ADR-0028 decision 9); this table covers only the deterministic measures.

## Latency (spec section 7.5)

| Measure | Value | Bar |
|---|---|---|
| Median, all questions (274 asked) | 5.3 s | 15.0 s or lower: met |
| 95th percentile, all questions | 28.1 s | 30.0 s or lower: met |
| Median, questions that used the engine (246) | 5.8 s | |
| 95th percentile, same | 28.1 s | |
| First question after the project opened | 16.1 s | |
| First question that used the engine | 16.1 s | |
| Prompt tokens served from stored work | 67.0% (448439 of 669486) | |
| Completion tokens, engine calls | 60731, 872 | |

| Length class | Questions | Median | 95th percentile |
|---|---|---|---|
| brief | 16 | 4.9 s | 6.5 s |
| one_sentence | 147 | 3.3 s | 11.4 s |
| paragraph | 55 | 16.1 s | 40.0 s |
| short | 56 | 6.2 s | 26.8 s |

| Stage | Mean per question |
|---|---|
| dossier | 3 ms |
| engine_generation | 9067 ms |
| engine_prompt | 8096 ms |
| turn | 880 ms |

Stage times come from the AI trace and from what the request recorded; a stage nobody records is not listed. Engine prompt and generation are the engine's own figures and include the earlier turns a conversation case asks first, so they add up to more than the time of the last question.
