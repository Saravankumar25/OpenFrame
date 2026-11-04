# AI evaluation report

- Cases: 302
- Gold folder: crates/straft-ai-eval/gold
- Model files: the pinned Offline AI of packaging/ai/ai-assets.lock.json
- Build: release
- Search model: Automatic
- Engine place: off (processor)
- Sampling: deterministic (temperature 0, fixed seed)
- Wall time: 48.4 min

## English release bar (ADR-0028 decision 2)

| Measure | Result | Bar |
|---|---|---|
| Correct tool and arguments (tool and action cases) | none | 90% |
| Correct end state after apply (action cases) | none | 85% |

Bar met: **no**

## By kind

| kind | Cases | Setup errors | Right tool offered | Tool chosen | Tool and arguments | Schema valid | End state | Answer facts | Language | Length | Safety | Not empty | Grounding | Recall@5 | Recall@10 | MRR | Median ms | p95 ms | Engine MB |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| conversation | 302 | 20 | none | none | none | 100.0% (282/282) | none | 47.1% (66/140) | 100.0% (282/282) | 87.5% (35/40) | none | 100.0% (282/282) | 67.4% | none | none | none | 6405 | 25210 | 2337 |

## By language

| language | Cases | Setup errors | Right tool offered | Tool chosen | Tool and arguments | Schema valid | End state | Answer facts | Language | Length | Safety | Not empty | Grounding | Recall@5 | Recall@10 | MRR | Median ms | p95 ms | Engine MB |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| en | 302 | 20 | none | none | none | 100.0% (282/282) | none | 47.1% (66/140) | 100.0% (282/282) | 87.5% (35/40) | none | 100.0% (282/282) | 67.4% | none | none | none | 6405 | 25210 | 2337 |

## Answer path and fallback

Cases by kind and the answer path they took.

| Kind | write | navigate | change | fixed | Cases |
|---|---|---|---|---|---|
| conversation | 268 | 2 | 2 | 10 | 282 |
| all | 268 | 2 | 2 | 10 | 282 |

Cases by kind and the fallback they used. `last_resort` is the last-resort answer; `abstained` is a could-not-find answer.

| Kind | none | last_resort | abstained | Cases |
|---|---|---|---|---|
| conversation | 188 | 25 | 69 | 282 |
| all | 188 | 25 | 69 | 282 |

## Conversation bench

Deterministic checks only; the rubric and human labels come at development time. Invented names, wrong scenes and missing quotes are rates over all names, scene citations and quotes in the answers.

| Question type | Cases | Setup errors | Answer facts | Invented names | Wrong scene citations | Missing quotes | Within length | Correct abstention | Repetition | Median ms |
|---|---|---|---|---|---|---|---|---|---|---|
| all | 302 | 20 | 47.1% (66/140) | 32.1% (235/733) | 0.0% (0/53) | 48.2% (27/56) | 87.5% (35/40) | 33.3% (1/3) | 0.7% (2/282) | 6405 |
| breakdown | 6 | 0 | 50.0% (1/2) | 6.2% (1/16) | 0.0% (0/4) | none | 100.0% (1/1) | none | 16.7% (1/6) | 6313 |
| call | 6 | 0 | 100.0% (1/1) | 80.0% (12/15) | none | 100.0% (1/1) | 100.0% (1/1) | none | 0.0% (0/6) | 10441 |
| conv | 24 | 0 | none | 6.9% (7/102) | 0.0% (0/3) | 50.0% (1/2) | 91.7% (22/24) | none | 0.0% (0/24) | 9728 |
| files | 6 | 0 | 100.0% (1/1) | 50.0% (1/2) | none | none | none | none | 0.0% (0/6) | 5234 |
| help | 12 | 0 | 0.0% (0/1) | 73.9% (51/69) | none | 0.0% (0/2) | none | none | 0.0% (0/12) | 7766 |
| idea | 12 | 6 | 0.0% (0/1) | 27.8% (5/18) | none | none | none | none | 0.0% (0/6) | 22847 |
| neg | 52 | 6 | 41.9% (18/43) | 37.4% (37/99) | 0.0% (0/16) | 20.0% (1/5) | none | 33.3% (1/3) | 0.0% (0/46) | 5646 |
| notes | 12 | 0 | 50.0% (3/6) | 60.0% (15/25) | 0.0% (0/1) | 100.0% (1/1) | none | none | 0.0% (0/12) | 6160 |
| pair | 29 | 0 | 35.3% (6/17) | 15.1% (14/93) | 0.0% (0/8) | 46.2% (6/13) | 50.0% (1/2) | none | 0.0% (0/29) | 7792 |
| production | 78 | 0 | 59.5% (25/42) | 33.1% (48/145) | 0.0% (0/8) | 58.3% (7/12) | 83.3% (5/6) | none | 1.3% (1/78) | 5775 |
| screenplay | 6 | 0 | 25.0% (1/4) | 7.7% (1/13) | 0.0% (0/7) | 50.0% (2/4) | 100.0% (2/2) | none | 0.0% (0/6) | 4101 |
| story | 24 | 6 | 50.0% (3/6) | 23.1% (12/52) | 0.0% (0/1) | 66.7% (2/3) | 100.0% (2/2) | none | 0.0% (0/18) | 6693 |
| triple | 5 | 0 | 33.3% (1/3) | 25.0% (5/20) | 0.0% (0/1) | none | 50.0% (1/2) | none | 0.0% (0/5) | 14665 |
| turn | 30 | 2 | 46.2% (6/13) | 40.6% (26/64) | 0.0% (0/4) | 46.2% (6/13) | none | none | 0.0% (0/28) | 6765 |

Expectation flags (harness spec 7.4). Cases that set none of them show none. Flags the build cannot measure yet are listed as not counted, never as passed or failed.

| Question type | Required facts | Abstain names | No navigation | No data dump | No internal words | Length class | No re-ask | Answer type | Answer group | Not counted |
|---|---|---|---|---|---|---|---|---|---|---|
| all | 43.3% (142/328) | 33.3% (1/3) | 95.9% (47/49) | none | 99.3% (280/282) | 85.1% (206/242) | 100.0% (17/17) | 74.2% (184/248) | 87.1% (216/248) | 0 |
| breakdown | 85.7% (6/7) | none | 100.0% (1/1) | none | 100.0% (6/6) | 80.0% (4/5) | none | 80.0% (4/5) | 100.0% (5/5) | 0 |
| call | 10.0% (1/10) | none | 0.0% (0/1) | none | 100.0% (6/6) | 80.0% (4/5) | none | 80.0% (4/5) | 100.0% (5/5) | 0 |
| conv | 68.9% (51/74) | none | 100.0% (24/24) | none | 100.0% (24/24) | none | none | 83.3% (5/6) | 83.3% (5/6) | 0 |
| files | 25.0% (2/8) | none | none | none | 100.0% (6/6) | 100.0% (6/6) | none | 100.0% (6/6) | 100.0% (6/6) | 0 |
| help | 54.5% (12/22) | none | 100.0% (1/1) | none | 100.0% (12/12) | 75.0% (9/12) | none | 41.7% (5/12) | 41.7% (5/12) | 0 |
| idea | 12.5% (1/8) | none | none | none | 100.0% (6/6) | 33.3% (2/6) | none | 33.3% (2/6) | 33.3% (2/6) | 0 |
| neg | 50.0% (1/2) | 33.3% (1/3) | 100.0% (1/1) | none | 95.7% (44/46) | 80.4% (37/46) | none | 80.4% (37/46) | 91.3% (42/46) | 0 |
| notes | 6.7% (1/15) | none | 100.0% (1/1) | none | 100.0% (12/12) | 75.0% (9/12) | none | 75.0% (9/12) | 83.3% (10/12) | 0 |
| pair | 43.6% (24/55) | none | 100.0% (5/5) | none | 100.0% (29/29) | 96.3% (26/27) | none | 63.0% (17/27) | 92.6% (25/27) | 0 |
| production | 32.3% (21/65) | none | 85.7% (6/7) | none | 100.0% (78/78) | 93.1% (67/72) | none | 83.3% (60/72) | 95.8% (69/72) | 0 |
| screenplay | 100.0% (6/6) | none | 100.0% (2/2) | none | 100.0% (6/6) | 100.0% (4/4) | none | 75.0% (3/4) | 75.0% (3/4) | 0 |
| story | 30.4% (7/23) | none | 100.0% (2/2) | none | 100.0% (18/18) | 75.0% (12/16) | none | 68.8% (11/16) | 81.2% (13/16) | 0 |
| triple | 33.3% (4/12) | none | 100.0% (3/3) | none | 100.0% (5/5) | 100.0% (3/3) | none | 66.7% (2/3) | 100.0% (3/3) | 0 |
| turn | 23.8% (5/21) | none | 100.0% (1/1) | none | 100.0% (28/28) | 82.1% (23/28) | 100.0% (17/17) | 67.9% (19/28) | 82.1% (23/28) | 0 |

Writing quality is scored separately against each case's rubric at development time (ADR-0028 decision 9); this table covers only the deterministic measures.

## Latency (spec section 7.5)

| Measure | Value | Bar |
|---|---|---|
| Median, all questions (282 asked) | 6.4 s | 15.0 s or lower: met |
| 95th percentile, all questions | 25.2 s | 30.0 s or lower: met |
| Median, questions that used the engine (274) | 6.5 s | |
| 95th percentile, same | 25.3 s | |
| First question after the project opened | 7.8 s | |
| First question that used the engine | 7.8 s | |
| Prompt tokens served from stored work | 64.6% (266335 of 412365) | |
| Completion tokens, engine calls | 36720, 625 | |

| Length class | Questions | Median | 95th percentile |
|---|---|---|---|
| brief | 2 | 5.4 s | 8.2 s |
| one_sentence | 143 | 5.1 s | 10.4 s |
| paragraph | 68 | 17.4 s | 37.8 s |
| short | 69 | 7.3 s | 19.4 s |

| Stage | Mean per question |
|---|---|
| dossier | 3 ms |
| engine_generation | 4876 ms |
| engine_prompt | 4846 ms |
| turn | 1189 ms |

Stage times come from the AI trace and from what the request recorded; a stage nobody records is not listed. Engine prompt and generation are the engine's own figures and include the earlier turns a conversation case asks first, so they add up to more than the time of the last question.
