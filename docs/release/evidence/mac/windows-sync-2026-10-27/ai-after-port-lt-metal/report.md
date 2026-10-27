# AI evaluation report

- Cases: 274
- Gold folder: .build-cache/eval-new/gold
- Model files: the pinned Offline AI of packaging/ai/ai-assets.lock.json
- Build: release
- Search model: Automatic
- Engine place: auto
- Sampling: deterministic (temperature 0, fixed seed)
- Wall time: 48.7 min

## English release bar (ADR-0028 decision 2)

| Measure | Result | Bar |
|---|---|---|
| Correct tool and arguments (tool and action cases) | 73.9% | 90% |
| Correct end state after apply (action cases) | 85.7% | 85% |

Bar met: **no**

## By kind

| kind | Cases | Setup errors | Right tool offered | Tool chosen | Tool and arguments | Schema valid | End state | Answer facts | Language | Length | Safety | Not empty | Grounding | Recall@5 | Recall@10 | MRR | Median ms | p95 ms | Engine MB |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| action | 7 | 0 | 100.0% (7/7) | 100.0% (7/7) | 85.7% (6/7) | 100.0% (7/7) | 85.7% (6/7) | none | 100.0% (7/7) | none | none | 100.0% (7/7) | 92.9% | none | none | none | 15 | 479 | 1628 |
| conversation | 251 | 0 | none | none | none | 100.0% (251/251) | none | 68.4% (93/136) | 100.0% (251/251) | none | none | 100.0% (251/251) | 61.0% | none | none | none | 2741 | 21164 | 2448 |
| tool | 16 | 0 | 100.0% (16/16) | 68.8% (11/16) | 68.8% (11/16) | 100.0% (16/16) | none | 100.0% (4/4) | 100.0% (16/16) | none | none | 100.0% (16/16) | 55.6% | none | none | none | 6 | 4301 | 1725 |

## By language

| language | Cases | Setup errors | Right tool offered | Tool chosen | Tool and arguments | Schema valid | End state | Answer facts | Language | Length | Safety | Not empty | Grounding | Recall@5 | Recall@10 | MRR | Median ms | p95 ms | Engine MB |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| en | 274 | 0 | 100.0% (23/23) | 78.3% (18/23) | 73.9% (17/23) | 100.0% (274/274) | 85.7% (6/7) | 69.3% (97/140) | 100.0% (274/274) | none | none | 100.0% (274/274) | 61.1% | none | none | none | 2505 | 20184 | 2448 |

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
| conversation | 166 | 34 | 51 | 251 |
| tool | 13 | 1 | 2 | 16 |
| all | 186 | 35 | 53 | 274 |

## Conversation bench

Deterministic checks only; the rubric and human labels come at development time. Invented names, wrong scenes and missing quotes are rates over all names, scene citations and quotes in the answers.

| Question type | Cases | Setup errors | Answer facts | Invented names | Wrong scene citations | Missing quotes | Within length | Correct abstention | Repetition | Median ms |
|---|---|---|---|---|---|---|---|---|---|---|
| all | 251 | 0 | 68.4% (93/136) | 37.6% (240/638) | 1.7% (1/58) | 57.6% (34/59) | none | 70.0% (14/20) | 0.8% (2/251) | 2741 |
| big | 5 | 0 | 100.0% (3/3) | 44.4% (4/9) | 0.0% (0/1) | 100.0% (2/2) | none | none | 0.0% (0/5) | 808 |
| brain | 11 | 0 | none | 40.5% (17/42) | 0.0% (0/1) | 66.7% (2/3) | none | none | 0.0% (0/11) | 21164 |
| char | 22 | 0 | 100.0% (4/4) | 5.3% (5/94) | none | 0.0% (0/1) | none | none | 0.0% (0/22) | 4833 |
| export | 40 | 0 | 73.9% (17/23) | 39.1% (45/115) | 0.0% (0/12) | 45.5% (5/11) | none | none | 0.0% (0/40) | 1482 |
| help | 11 | 0 | 45.5% (5/11) | 70.8% (34/48) | none | 60.0% (3/5) | none | none | 0.0% (0/11) | 6083 |
| hop | 13 | 0 | 55.6% (5/9) | 42.3% (11/26) | 0.0% (0/6) | 83.3% (5/6) | none | none | 0.0% (0/13) | 4172 |
| hostile | 11 | 0 | none | 100.0% (4/4) | none | none | none | none | 0.0% (0/11) | 1645 |
| lang | 9 | 0 | 50.0% (2/4) | 36.4% (8/22) | 0.0% (0/1) | 50.0% (1/2) | none | 100.0% (1/1) | 0.0% (0/9) | 2701 |
| missing | 19 | 0 | 100.0% (1/1) | 57.1% (20/35) | 0.0% (0/3) | 77.8% (7/9) | none | 70.6% (12/17) | 5.3% (1/19) | 1487 |
| money | 27 | 0 | 75.0% (18/24) | 75.4% (46/61) | none | 50.0% (1/2) | none | none | 0.0% (0/27) | 4130 |
| prod | 30 | 0 | 77.8% (14/18) | 53.2% (25/47) | 0.0% (0/1) | 50.0% (2/4) | none | 0.0% (0/1) | 0.0% (0/30) | 2749 |
| repair | 11 | 0 | 100.0% (7/7) | 28.6% (8/28) | 0.0% (0/6) | 33.3% (1/3) | none | none | 0.0% (0/11) | 1689 |
| scene | 30 | 0 | 57.1% (12/21) | 15.7% (13/83) | 0.0% (0/19) | 50.0% (2/4) | none | 100.0% (1/1) | 3.3% (1/30) | 2460 |
| xref | 12 | 0 | 45.5% (5/11) | 0.0% (0/24) | 12.5% (1/8) | 42.9% (3/7) | none | none | 0.0% (0/12) | 5382 |

Expectation flags (harness spec 7.4). Cases that set none of them show none. Flags the build cannot measure yet are listed as not counted, never as passed or failed.

| Question type | Required facts | Abstain names | No navigation | No data dump | No internal words | Length class | No re-ask | Answer type | Answer group | Not counted |
|---|---|---|---|---|---|---|---|---|---|---|
| all | 71.4% (135/189) | 70.0% (14/20) | 97.0% (65/67) | 95.2% (20/21) | 100.0% (251/251) | 92.0% (231/251) | 97.2% (69/71) | 84.1% (211/251) | 91.6% (230/251) | 0 |
| big | 100.0% (5/5) | none | none | none | 100.0% (5/5) | 100.0% (5/5) | none | 100.0% (5/5) | 100.0% (5/5) | 0 |
| brain | 58.3% (7/12) | none | 90.9% (10/11) | none | 100.0% (11/11) | 81.8% (9/11) | none | 72.7% (8/11) | 72.7% (8/11) | 0 |
| char | 82.9% (29/35) | none | 100.0% (11/11) | none | 100.0% (22/22) | 77.3% (17/22) | 100.0% (3/3) | 68.2% (15/22) | 72.7% (16/22) | 0 |
| export | 78.6% (22/28) | none | 100.0% (13/13) | 90.0% (9/10) | 100.0% (40/40) | 97.5% (39/40) | 97.4% (38/39) | 90.0% (36/40) | 92.5% (37/40) | 0 |
| help | none | none | none | none | 100.0% (11/11) | 81.8% (9/11) | none | 90.9% (10/11) | 90.9% (10/11) | 0 |
| hop | 50.0% (6/12) | none | 100.0% (2/2) | none | 100.0% (13/13) | 92.3% (12/13) | 100.0% (6/6) | 84.6% (11/13) | 100.0% (13/13) | 0 |
| hostile | none | none | none | 100.0% (11/11) | 100.0% (11/11) | 100.0% (11/11) | 100.0% (1/1) | 100.0% (11/11) | 100.0% (11/11) | 0 |
| lang | 83.3% (5/6) | 100.0% (1/1) | 100.0% (3/3) | none | 100.0% (9/9) | 88.9% (8/9) | none | 77.8% (7/9) | 100.0% (9/9) | 0 |
| missing | 100.0% (2/2) | 70.6% (12/17) | 0.0% (0/1) | none | 100.0% (19/19) | 100.0% (19/19) | none | 89.5% (17/19) | 94.7% (18/19) | 0 |
| money | 60.0% (9/15) | none | 100.0% (8/8) | none | 100.0% (27/27) | 92.6% (25/27) | 100.0% (1/1) | 77.8% (21/27) | 96.3% (26/27) | 0 |
| prod | 63.6% (14/22) | 0.0% (0/1) | none | none | 100.0% (30/30) | 100.0% (30/30) | none | 86.7% (26/30) | 96.7% (29/30) | 0 |
| repair | 66.7% (6/9) | none | 100.0% (6/6) | none | 100.0% (11/11) | 90.9% (10/11) | 100.0% (11/11) | 90.9% (10/11) | 90.9% (10/11) | 0 |
| scene | 69.4% (25/36) | 100.0% (1/1) | 100.0% (6/6) | none | 100.0% (30/30) | 90.0% (27/30) | 90.0% (9/10) | 80.0% (24/30) | 93.3% (28/30) | 0 |
| xref | 71.4% (5/7) | none | 100.0% (6/6) | none | 100.0% (12/12) | 83.3% (10/12) | none | 83.3% (10/12) | 83.3% (10/12) | 0 |

Writing quality is scored separately against each case's rubric at development time (ADR-0028 decision 9); this table covers only the deterministic measures.

## Latency (spec section 7.5)

| Measure | Value | Bar |
|---|---|---|
| Median, all questions (274 asked) | 2.5 s | 15.0 s or lower: met |
| 95th percentile, all questions | 20.4 s | 30.0 s or lower: met |
| Median, questions that used the engine (246) | 2.8 s | |
| 95th percentile, same | 21.2 s | |
| First question after the project opened | 19.8 s | |
| First question that used the engine | 19.8 s | |
| Prompt tokens served from stored work | 67.3% (464955 of 690690) | |
| Completion tokens, engine calls | 64605, 891 | |

| Length class | Questions | Median | 95th percentile |
|---|---|---|---|
| brief | 16 | 1.6 s | 5.0 s |
| one_sentence | 149 | 1.2 s | 5.6 s |
| paragraph | 54 | 14.2 s | 31.4 s |
| short | 55 | 4.3 s | 15.9 s |

| Stage | Mean per question |
|---|---|
| dossier | 1 ms |
| engine_generation | 9240 ms |
| engine_prompt | 1290 ms |
| turn | 287 ms |

Stage times come from the AI trace and from what the request recorded; a stage nobody records is not listed. Engine prompt and generation are the engine's own figures and include the earlier turns a conversation case asks first, so they add up to more than the time of the last question.
