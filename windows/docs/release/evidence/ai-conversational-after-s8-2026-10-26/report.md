# AI evaluation report

- Cases: 274
- Gold folder: crates/openframe-ai-eval/gold
- Model files: the pinned Offline AI of packaging/ai/ai-assets.lock.json
- Build: release
- Search model: Automatic
- Routes: on (regex direct routes active)
- Engine place: off (processor)
- Sampling: deterministic (temperature 0, fixed seed)
- Wall time: 71.0 min

## English release bar (ADR-0028 decision 2)

| Measure | Result | Bar |
|---|---|---|
| Correct tool and arguments (tool and action cases) | 69.6% | 90% |
| Correct end state after apply (action cases) | 85.7% | 85% |

Bar met: **no**

## By kind

| kind | Cases | Setup errors | Right tool offered | Tool chosen | Tool and arguments | Schema valid | End state | Answer facts | Language | Length | Safety | Not empty | Grounding | Recall@5 | Recall@10 | MRR | Median ms | p95 ms | Engine MB |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| action | 7 | 0 | 100.0% (7/7) | 100.0% (7/7) | 85.7% (6/7) | 100.0% (7/7) | 85.7% (6/7) | none | 100.0% (7/7) | none | none | 100.0% (7/7) | 92.9% | none | none | none | 54 | 2095 | 1704 |
| conversation | 251 | 0 | none | none | none | 100.0% (251/251) | none | 64.0% (87/136) | 100.0% (251/251) | none | none | 100.0% (251/251) | 64.2% | none | none | none | 4655 | 22932 | 2068 |
| tool | 16 | 0 | 100.0% (16/16) | 62.5% (10/16) | 62.5% (10/16) | 100.0% (16/16) | none | 75.0% (3/4) | 100.0% (16/16) | none | none | 100.0% (16/16) | 63.4% | none | none | none | 47 | 2286 | 1812 |

## By language

| language | Cases | Setup errors | Right tool offered | Tool chosen | Tool and arguments | Schema valid | End state | Answer facts | Language | Length | Safety | Not empty | Grounding | Recall@5 | Recall@10 | MRR | Median ms | p95 ms | Engine MB |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| en | 274 | 0 | 100.0% (23/23) | 73.9% (17/23) | 69.6% (16/23) | 100.0% (274/274) | 85.7% (6/7) | 64.3% (90/140) | 100.0% (274/274) | none | none | 100.0% (274/274) | 64.4% | none | none | none | 4390 | 22669 | 2068 |

## Lanes and intents

Cases by kind and the lane they took.

| Kind | direct | chat | recipe | agent | action | Cases |
|---|---|---|---|---|---|---|
| action | 0 | 0 | 0 | 0 | 0 | 7 |
| conversation | 0 | 0 | 0 | 0 | 0 | 251 |
| tool | 0 | 0 | 0 | 0 | 0 | 16 |
| all | 0 | 0 | 0 | 0 | 0 | 0 |

## Conversation bench

Deterministic checks only; the rubric and human labels come at development time. Invented names, wrong scenes and missing quotes are rates over all names, scene citations and quotes in the answers.

| Question type | Cases | Setup errors | Answer facts | Invented names | Wrong scene citations | Missing quotes | Within length | Correct abstention | Repetition | Median ms |
|---|---|---|---|---|---|---|---|---|---|---|
| all | 251 | 0 | 64.0% (87/136) | 35.8% (215/600) | 1.8% (1/55) | 45.5% (30/66) | none | 15.0% (3/20) | 0.4% (1/251) | 4655 |
| big | 5 | 0 | 100.0% (3/3) | 44.4% (4/9) | 0.0% (0/1) | 100.0% (1/1) | none | none | 0.0% (0/5) | 1069 |
| brain | 11 | 0 | none | 12.8% (5/39) | 0.0% (0/2) | 66.7% (2/3) | none | none | 0.0% (0/11) | 15727 |
| char | 22 | 0 | 100.0% (4/4) | 8.8% (8/91) | none | 0.0% (0/4) | none | none | 0.0% (0/22) | 6145 |
| export | 40 | 0 | 73.9% (17/23) | 38.4% (43/112) | 0.0% (0/13) | 44.4% (4/9) | none | none | 0.0% (0/40) | 4234 |
| help | 11 | 0 | 54.5% (6/11) | 80.5% (33/41) | none | 66.7% (4/6) | none | none | 0.0% (0/11) | 6023 |
| hop | 13 | 0 | 55.6% (5/9) | 36.0% (9/25) | 0.0% (0/6) | 40.0% (2/5) | none | none | 0.0% (0/13) | 3853 |
| hostile | 11 | 0 | none | 100.0% (3/3) | none | none | none | none | 0.0% (0/11) | 2187 |
| lang | 9 | 0 | 25.0% (1/4) | 4.3% (1/23) | 0.0% (0/1) | 25.0% (1/4) | none | 0.0% (0/1) | 0.0% (0/9) | 6607 |
| missing | 19 | 0 | 100.0% (1/1) | 45.5% (10/22) | 0.0% (0/4) | 50.0% (2/4) | none | 17.6% (3/17) | 0.0% (0/19) | 1940 |
| money | 27 | 0 | 70.8% (17/24) | 76.8% (43/56) | none | 50.0% (2/4) | none | none | 0.0% (0/27) | 7455 |
| prod | 30 | 0 | 72.2% (13/18) | 63.6% (35/55) | 0.0% (0/2) | 40.0% (2/5) | none | 0.0% (0/1) | 0.0% (0/30) | 5005 |
| repair | 11 | 0 | 85.7% (6/7) | 28.6% (8/28) | 0.0% (0/2) | 50.0% (1/2) | none | none | 0.0% (0/11) | 3439 |
| scene | 30 | 0 | 57.1% (12/21) | 17.3% (13/75) | 0.0% (0/16) | 40.0% (2/5) | none | 0.0% (0/1) | 3.3% (1/30) | 6218 |
| xref | 12 | 0 | 18.2% (2/11) | 0.0% (0/21) | 12.5% (1/8) | 50.0% (7/14) | none | none | 0.0% (0/12) | 2429 |

Expectation flags (harness spec 7.4). Cases that set none of them show none. Flags the build cannot measure yet are listed as not counted, never as passed or failed.

| Question type | Required facts | Abstain names | No navigation | No data dump | No internal words | Length class | No re-ask | Answer type | Answer group | Not counted |
|---|---|---|---|---|---|---|---|---|---|---|
| all | 68.3% (129/189) | 15.0% (3/20) | 97.0% (65/67) | 95.2% (20/21) | 100.0% (251/251) | 93.2% (234/251) | 97.2% (69/71) | 82.1% (206/251) | 90.4% (227/251) | 0 |
| big | 100.0% (5/5) | none | none | none | 100.0% (5/5) | 100.0% (5/5) | none | 100.0% (5/5) | 100.0% (5/5) | 0 |
| brain | 75.0% (9/12) | none | 90.9% (10/11) | none | 100.0% (11/11) | 81.8% (9/11) | none | 72.7% (8/11) | 72.7% (8/11) | 0 |
| char | 65.7% (23/35) | none | 100.0% (11/11) | none | 100.0% (22/22) | 72.7% (16/22) | 100.0% (3/3) | 63.6% (14/22) | 68.2% (15/22) | 0 |
| export | 85.7% (24/28) | none | 100.0% (13/13) | 90.0% (9/10) | 100.0% (40/40) | 97.5% (39/40) | 97.4% (38/39) | 90.0% (36/40) | 92.5% (37/40) | 0 |
| help | none | none | none | none | 100.0% (11/11) | 81.8% (9/11) | none | 90.9% (10/11) | 90.9% (10/11) | 0 |
| hop | 33.3% (4/12) | none | 100.0% (2/2) | none | 100.0% (13/13) | 92.3% (12/13) | 100.0% (6/6) | 84.6% (11/13) | 100.0% (13/13) | 0 |
| hostile | none | none | none | 100.0% (11/11) | 100.0% (11/11) | 100.0% (11/11) | 100.0% (1/1) | 100.0% (11/11) | 100.0% (11/11) | 0 |
| lang | 83.3% (5/6) | 0.0% (0/1) | 100.0% (3/3) | none | 100.0% (9/9) | 100.0% (9/9) | none | 66.7% (6/9) | 100.0% (9/9) | 0 |
| missing | 100.0% (2/2) | 17.6% (3/17) | 0.0% (0/1) | none | 100.0% (19/19) | 100.0% (19/19) | none | 84.2% (16/19) | 89.5% (17/19) | 0 |
| money | 46.7% (7/15) | none | 100.0% (8/8) | none | 100.0% (27/27) | 100.0% (27/27) | 100.0% (1/1) | 74.1% (20/27) | 92.6% (25/27) | 0 |
| prod | 68.2% (15/22) | 0.0% (0/1) | none | none | 100.0% (30/30) | 100.0% (30/30) | none | 86.7% (26/30) | 96.7% (29/30) | 0 |
| repair | 66.7% (6/9) | none | 100.0% (6/6) | none | 100.0% (11/11) | 90.9% (10/11) | 100.0% (11/11) | 90.9% (10/11) | 90.9% (10/11) | 0 |
| scene | 66.7% (24/36) | 0.0% (0/1) | 100.0% (6/6) | none | 100.0% (30/30) | 90.0% (27/30) | 90.0% (9/10) | 76.7% (23/30) | 93.3% (28/30) | 0 |
| xref | 71.4% (5/7) | none | 100.0% (6/6) | none | 100.0% (12/12) | 91.7% (11/12) | none | 83.3% (10/12) | 83.3% (10/12) | 0 |

Writing quality is scored separately against each case's rubric at development time (ADR-0028 decision 9); this table covers only the deterministic measures.

## Latency (spec section 7.5)

| Measure | Value | Bar |
|---|---|---|
| Median, all questions (274 asked) | 4.4 s | 15.0 s or lower: met |
| 95th percentile, all questions | 22.9 s | 30.0 s or lower: met |
| Median, questions that used the engine (236) | 5.1 s | |
| 95th percentile, same | 23.6 s | |
| First question after the project opened | 14.3 s | |
| First question that used the engine | 14.3 s | |
| Prompt tokens served from stored work | 64.9% (393277 of 606248) | |
| Completion tokens, engine calls | 56242, 821 | |

| Length class | Questions | Median | 95th percentile |
|---|---|---|---|
| brief | 17 | 4.2 s | 12.0 s |
| one_sentence | 148 | 2.7 s | 11.2 s |
| paragraph | 55 | 10.0 s | 28.9 s |
| short | 54 | 5.4 s | 22.9 s |

| Stage | Mean per question |
|---|---|
| dossier | 3 ms |
| engine_generation | 8012 ms |
| engine_prompt | 6715 ms |
| turn | 884 ms |

Stage times come from the AI trace and from what the request recorded; a stage nobody records is not listed. Engine prompt and generation are the engine's own figures and include the earlier turns a conversation case asks first, so they add up to more than the time of the last question.
