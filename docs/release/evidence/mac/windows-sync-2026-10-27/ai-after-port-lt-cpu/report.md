# AI evaluation report

- Cases: 274
- Gold folder: .build-cache/eval-new/gold
- Model files: the pinned Offline AI of packaging/ai/ai-assets.lock.json
- Build: release
- Search model: Automatic
- Engine place: off
- Sampling: deterministic (temperature 0, fixed seed)
- Wall time: 140.3 min

## English release bar (ADR-0028 decision 2)

| Measure | Result | Bar |
|---|---|---|
| Correct tool and arguments (tool and action cases) | 73.9% | 90% |
| Correct end state after apply (action cases) | 85.7% | 85% |

Bar met: **no**

## By kind

| kind | Cases | Setup errors | Right tool offered | Tool chosen | Tool and arguments | Schema valid | End state | Answer facts | Language | Length | Safety | Not empty | Grounding | Recall@5 | Recall@10 | MRR | Median ms | p95 ms | Engine MB |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| action | 7 | 0 | 100.0% (7/7) | 100.0% (7/7) | 85.7% (6/7) | 100.0% (7/7) | 85.7% (6/7) | none | 100.0% (7/7) | none | none | 100.0% (7/7) | 92.9% | none | none | none | 7 | 959 | 1644 |
| conversation | 251 | 0 | none | none | none | 100.0% (251/251) | none | 69.1% (94/136) | 100.0% (251/251) | none | none | 100.0% (251/251) | 61.7% | none | none | none | 5669 | 69055 | 4473 |
| tool | 16 | 0 | 100.0% (16/16) | 68.8% (11/16) | 68.8% (11/16) | 100.0% (16/16) | none | 100.0% (4/4) | 100.0% (16/16) | none | none | 100.0% (16/16) | 56.6% | none | none | none | 4 | 7387 | 1957 |

## By language

| language | Cases | Setup errors | Right tool offered | Tool chosen | Tool and arguments | Schema valid | End state | Answer facts | Language | Length | Safety | Not empty | Grounding | Recall@5 | Recall@10 | MRR | Median ms | p95 ms | Engine MB |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| en | 274 | 0 | 100.0% (23/23) | 78.3% (18/23) | 73.9% (17/23) | 100.0% (274/274) | 85.7% (6/7) | 70.0% (98/140) | 100.0% (274/274) | none | none | 100.0% (274/274) | 61.7% | none | none | none | 5155 | 60121 | 4473 |

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
| conversation | 170 | 33 | 48 | 251 |
| tool | 14 | 0 | 2 | 16 |
| all | 191 | 33 | 50 | 274 |

## Conversation bench

Deterministic checks only; the rubric and human labels come at development time. Invented names, wrong scenes and missing quotes are rates over all names, scene citations and quotes in the answers.

| Question type | Cases | Setup errors | Answer facts | Invented names | Wrong scene citations | Missing quotes | Within length | Correct abstention | Repetition | Median ms |
|---|---|---|---|---|---|---|---|---|---|---|
| all | 251 | 0 | 69.1% (94/136) | 37.5% (234/624) | 1.7% (1/59) | 43.1% (22/51) | none | 75.0% (15/20) | 0.4% (1/251) | 5669 |
| big | 5 | 0 | 100.0% (3/3) | 44.4% (4/9) | 0.0% (0/1) | 100.0% (2/2) | none | none | 0.0% (0/5) | 7422 |
| brain | 11 | 0 | none | 21.2% (7/33) | 0.0% (0/1) | 66.7% (2/3) | none | none | 0.0% (0/11) | 30717 |
| char | 22 | 0 | 100.0% (4/4) | 4.1% (4/97) | none | 0.0% (0/4) | none | none | 0.0% (0/22) | 5118 |
| export | 40 | 0 | 73.9% (17/23) | 38.4% (43/112) | 0.0% (0/12) | 25.0% (1/4) | none | none | 0.0% (0/40) | 5458 |
| help | 11 | 0 | 45.5% (5/11) | 70.8% (34/48) | none | 60.0% (3/5) | none | none | 0.0% (0/11) | 32716 |
| hop | 13 | 0 | 55.6% (5/9) | 40.0% (10/25) | 0.0% (0/9) | 0.0% (0/2) | none | none | 0.0% (0/13) | 3855 |
| hostile | 11 | 0 | none | 100.0% (5/5) | none | none | none | none | 0.0% (0/11) | 10479 |
| lang | 9 | 0 | 25.0% (1/4) | 30.0% (6/20) | 0.0% (0/1) | 33.3% (1/3) | none | 100.0% (1/1) | 0.0% (0/9) | 5174 |
| missing | 19 | 0 | 100.0% (1/1) | 59.4% (19/32) | 0.0% (0/3) | 50.0% (2/4) | none | 76.5% (13/17) | 0.0% (0/19) | 4651 |
| money | 27 | 0 | 79.2% (19/24) | 77.8% (49/63) | none | 60.0% (3/5) | none | none | 0.0% (0/27) | 7384 |
| prod | 30 | 0 | 77.8% (14/18) | 61.4% (35/57) | 0.0% (0/1) | 50.0% (3/6) | none | 0.0% (0/1) | 0.0% (0/30) | 4942 |
| repair | 11 | 0 | 100.0% (7/7) | 30.8% (8/26) | 0.0% (0/4) | 0.0% (0/1) | none | none | 0.0% (0/11) | 8449 |
| scene | 30 | 0 | 57.1% (12/21) | 13.9% (10/72) | 0.0% (0/17) | 50.0% (2/4) | none | 100.0% (1/1) | 3.3% (1/30) | 3954 |
| xref | 12 | 0 | 54.5% (6/11) | 0.0% (0/25) | 10.0% (1/10) | 37.5% (3/8) | none | none | 0.0% (0/12) | 10820 |

Expectation flags (harness spec 7.4). Cases that set none of them show none. Flags the build cannot measure yet are listed as not counted, never as passed or failed.

| Question type | Required facts | Abstain names | No navigation | No data dump | No internal words | Length class | No re-ask | Answer type | Answer group | Not counted |
|---|---|---|---|---|---|---|---|---|---|---|
| all | 72.5% (137/189) | 75.0% (15/20) | 97.0% (65/67) | 95.2% (20/21) | 100.0% (251/251) | 92.0% (231/251) | 97.2% (69/71) | 84.1% (211/251) | 91.6% (230/251) | 0 |
| big | 100.0% (5/5) | none | none | none | 100.0% (5/5) | 100.0% (5/5) | none | 100.0% (5/5) | 100.0% (5/5) | 0 |
| brain | 66.7% (8/12) | none | 90.9% (10/11) | none | 100.0% (11/11) | 72.7% (8/11) | none | 72.7% (8/11) | 72.7% (8/11) | 0 |
| char | 82.9% (29/35) | none | 100.0% (11/11) | none | 100.0% (22/22) | 77.3% (17/22) | 100.0% (3/3) | 68.2% (15/22) | 72.7% (16/22) | 0 |
| export | 78.6% (22/28) | none | 100.0% (13/13) | 90.0% (9/10) | 100.0% (40/40) | 97.5% (39/40) | 97.4% (38/39) | 90.0% (36/40) | 92.5% (37/40) | 0 |
| help | none | none | none | none | 100.0% (11/11) | 81.8% (9/11) | none | 90.9% (10/11) | 90.9% (10/11) | 0 |
| hop | 33.3% (4/12) | none | 100.0% (2/2) | none | 100.0% (13/13) | 92.3% (12/13) | 100.0% (6/6) | 84.6% (11/13) | 100.0% (13/13) | 0 |
| hostile | none | none | none | 100.0% (11/11) | 100.0% (11/11) | 100.0% (11/11) | 100.0% (1/1) | 100.0% (11/11) | 100.0% (11/11) | 0 |
| lang | 83.3% (5/6) | 100.0% (1/1) | 100.0% (3/3) | none | 100.0% (9/9) | 100.0% (9/9) | none | 66.7% (6/9) | 100.0% (9/9) | 0 |
| missing | 100.0% (2/2) | 76.5% (13/17) | 0.0% (0/1) | none | 100.0% (19/19) | 100.0% (19/19) | none | 89.5% (17/19) | 94.7% (18/19) | 0 |
| money | 66.7% (10/15) | none | 100.0% (8/8) | none | 100.0% (27/27) | 92.6% (25/27) | 100.0% (1/1) | 77.8% (21/27) | 96.3% (26/27) | 0 |
| prod | 63.6% (14/22) | 0.0% (0/1) | none | none | 100.0% (30/30) | 100.0% (30/30) | none | 90.0% (27/30) | 96.7% (29/30) | 0 |
| repair | 66.7% (6/9) | none | 100.0% (6/6) | none | 100.0% (11/11) | 81.8% (9/11) | 100.0% (11/11) | 90.9% (10/11) | 90.9% (10/11) | 0 |
| scene | 72.2% (26/36) | 100.0% (1/1) | 100.0% (6/6) | none | 100.0% (30/30) | 93.3% (28/30) | 90.0% (9/10) | 80.0% (24/30) | 93.3% (28/30) | 0 |
| xref | 85.7% (6/7) | none | 100.0% (6/6) | none | 100.0% (12/12) | 83.3% (10/12) | none | 83.3% (10/12) | 83.3% (10/12) | 0 |

Writing quality is scored separately against each case's rubric at development time (ADR-0028 decision 9); this table covers only the deterministic measures.

## Latency (spec section 7.5)

| Measure | Value | Bar |
|---|---|---|
| Median, all questions (274 asked) | 5.1 s | 15.0 s or lower: met |
| 95th percentile, all questions | 62.7 s | 30.0 s or lower: not met |
| Median, questions that used the engine (246) | 6.0 s | |
| 95th percentile, same | 69.1 s | |
| First question after the project opened | 24.8 s | |
| First question that used the engine | 24.8 s | |
| Prompt tokens served from stored work | 65.2% (401877 of 616704) | |
| Completion tokens, engine calls | 54993, 832 | |

| Length class | Questions | Median | 95th percentile |
|---|---|---|---|
| brief | 16 | 6.9 s | 88.0 s |
| one_sentence | 150 | 2.5 s | 16.5 s |
| paragraph | 54 | 24.5 s | 183.0 s |
| short | 54 | 10.2 s | 59.9 s |

| Stage | Mean per question |
|---|---|
| dossier | 1 ms |
| engine_generation | 27178 ms |
| engine_prompt | 5138 ms |
| turn | 697 ms |

Stage times come from the AI trace and from what the request recorded; a stage nobody records is not listed. Engine prompt and generation are the engine's own figures and include the earlier turns a conversation case asks first, so they add up to more than the time of the last question.
