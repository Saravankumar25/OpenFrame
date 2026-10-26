# AI evaluation report

- Cases: 274
- Gold folder: crates/openframe-ai-eval/gold
- Model files: the pinned Offline AI of packaging/ai/ai-assets.lock.json
- Build: release
- Search model: Automatic
- Routes: on (regex direct routes active)
- Engine place: off (processor)
- Sampling: deterministic (temperature 0, fixed seed)
- Wall time: 121.5 min

## English release bar (ADR-0028 decision 2)

| Measure | Result | Bar |
|---|---|---|
| Correct tool and arguments (tool and action cases) | 91.3% | 90% |
| Correct end state after apply (action cases) | 85.7% | 85% |

Bar met: **no**

## By kind

| kind | Cases | Setup errors | Right tool offered | Tool chosen | Tool and arguments | Schema valid | End state | Answer facts | Language | Length | Safety | Not empty | Grounding | Recall@5 | Recall@10 | MRR | Median ms | p95 ms | Engine MB |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| action | 7 | 0 | 100.0% (7/7) | 100.0% (7/7) | 85.7% (6/7) | 100.0% (7/7) | 85.7% (6/7) | none | 100.0% (7/7) | none | none | 100.0% (7/7) | 92.9% | none | none | none | 95 | 119 | 1414 |
| conversation | 251 | 0 | none | none | none | 100.0% (251/251) | none | 17.6% (24/136) | 100.0% (251/251) | none | none | 97.6% (245/251) | 62.7% | none | none | none | 16671 | 30839 | 4112 |
| tool | 16 | 0 | 100.0% (16/16) | 100.0% (16/16) | 93.8% (15/16) | 100.0% (16/16) | none | 25.0% (1/4) | 100.0% (16/16) | none | none | 100.0% (16/16) | 66.2% | none | none | none | 79 | 11760 | 1884 |

## By language

| language | Cases | Setup errors | Right tool offered | Tool chosen | Tool and arguments | Schema valid | End state | Answer facts | Language | Length | Safety | Not empty | Grounding | Recall@5 | Recall@10 | MRR | Median ms | p95 ms | Engine MB |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| en | 274 | 0 | 100.0% (23/23) | 100.0% (23/23) | 91.3% (21/23) | 100.0% (274/274) | 85.7% (6/7) | 17.9% (25/140) | 100.0% (274/274) | none | none | 97.8% (268/274) | 62.9% | none | none | none | 16275 | 30128 | 4112 |

## Lanes and intents

Cases by kind and the lane they took.

| Kind | direct | chat | recipe | agent | action | Cases |
|---|---|---|---|---|---|---|
| action | 0 | 0 | 0 | 0 | 7 | 7 |
| conversation | 35 | 1 | 0 | 213 | 2 | 251 |
| tool | 11 | 0 | 0 | 5 | 0 | 16 |
| all | 46 | 1 | 0 | 218 | 9 | 274 |

Cases that had a closest intent, by its score. Matched: the intent's tool is a tool the case expects. Unjudged: the case expects no tool.

| Score | Matched | Missed | Unjudged | Matched of judged |
|---|---|---|---|---|
| 0.78 to 0.80 | 0 | 0 | 7 | none |
| 0.80 to 0.82 | 0 | 0 | 25 | none |
| 0.82 to 0.84 | 0 | 0 | 58 | none |
| 0.84 to 0.86 | 0 | 1 | 62 | 0.0% (0/1) |
| 0.86 to 0.88 | 0 | 2 | 43 | 0.0% (0/2) |
| 0.88 to 0.90 | 2 | 0 | 14 | 100.0% (2/2) |
| 0.90 to 0.92 | 0 | 0 | 7 | none |
| 0.92 to 0.94 | 0 | 0 | 3 | none |
| 0.94 to 0.96 | 0 | 0 | 1 | none |

## Conversation bench

Deterministic checks only; the rubric and human labels come at development time. Invented names, wrong scenes and missing quotes are rates over all names, scene citations and quotes in the answers.

| Question type | Cases | Setup errors | Answer facts | Invented names | Wrong scene citations | Missing quotes | Within length | Correct abstention | Repetition | Median ms |
|---|---|---|---|---|---|---|---|---|---|---|
| all | 251 | 0 | 17.6% (24/136) | 47.4% (525/1108) | 0.0% (0/211) | 50.0% (61/122) | none | 15.0% (3/20) | 6.0% (15/251) | 16671 |
| big | 5 | 0 | 66.7% (2/3) | 53.8% (7/13) | 0.0% (0/1) | 100.0% (1/1) | none | none | 0.0% (0/5) | 108 |
| brain | 11 | 0 | none | 52.4% (22/42) | 0.0% (0/3) | 50.0% (2/4) | none | none | 0.0% (0/11) | 18451 |
| char | 22 | 0 | 25.0% (1/4) | 31.8% (42/132) | 0.0% (0/48) | 50.0% (6/12) | none | none | 13.6% (3/22) | 12262 |
| export | 40 | 0 | 13.0% (3/23) | 38.3% (80/209) | 0.0% (0/35) | 45.8% (11/24) | none | none | 2.5% (1/40) | 19732 |
| help | 11 | 0 | 45.5% (5/11) | 60.0% (18/30) | 0.0% (0/2) | 50.0% (1/2) | none | none | 0.0% (0/11) | 14506 |
| hop | 13 | 0 | 22.2% (2/9) | 45.5% (25/55) | 0.0% (0/16) | 50.0% (3/6) | none | none | 7.7% (1/13) | 20002 |
| hostile | 11 | 0 | none | 50.0% (3/6) | none | 66.7% (2/3) | none | none | 0.0% (0/11) | 12455 |
| lang | 9 | 0 | 0.0% (0/4) | 37.6% (32/85) | 0.0% (0/28) | 50.0% (2/4) | none | 0.0% (0/1) | 22.2% (2/9) | 17298 |
| missing | 19 | 0 | 0.0% (0/1) | 45.2% (28/62) | 0.0% (0/4) | 50.0% (2/4) | none | 11.8% (2/17) | 5.3% (1/19) | 13709 |
| money | 27 | 0 | 8.3% (2/24) | 70.4% (88/125) | none | 50.0% (7/14) | none | none | 11.1% (3/27) | 15146 |
| prod | 30 | 0 | 5.6% (1/18) | 57.5% (73/127) | 0.0% (0/29) | 46.2% (6/13) | none | 0.0% (0/1) | 6.7% (2/30) | 16492 |
| repair | 11 | 0 | 14.3% (1/7) | 54.5% (18/33) | 0.0% (0/8) | 60.0% (3/5) | none | none | 9.1% (1/11) | 28097 |
| scene | 30 | 0 | 19.0% (4/21) | 49.4% (42/85) | 0.0% (0/11) | 52.2% (12/23) | none | 100.0% (1/1) | 0.0% (0/30) | 14597 |
| xref | 12 | 0 | 27.3% (3/11) | 45.2% (47/104) | 0.0% (0/26) | 42.9% (3/7) | none | none | 8.3% (1/12) | 22676 |

Expectation flags (harness spec 7.4). Cases that set none of them show none. Flags the build cannot measure yet are listed as not counted, never as passed or failed.

| Question type | Required facts | Abstain names | No navigation | No data dump | No internal words | Length class | No re-ask | Not counted |
|---|---|---|---|---|---|---|---|---|
| all | 25.9% (49/189) | 0.0% (0/20) | 85.1% (57/67) | 85.7% (18/21) | 99.6% (250/251) | 65.7% (165/251) | 83.1% (59/71) | 251 |
| big | 100.0% (5/5) | none | none | none | 100.0% (5/5) | 100.0% (5/5) | none | 5 |
| brain | 41.7% (5/12) | none | 100.0% (11/11) | none | 100.0% (11/11) | 36.4% (4/11) | none | 11 |
| char | 31.4% (11/35) | none | 81.8% (9/11) | none | 100.0% (22/22) | 50.0% (11/22) | 33.3% (1/3) | 22 |
| export | 17.9% (5/28) | none | 69.2% (9/13) | 70.0% (7/10) | 97.5% (39/40) | 67.5% (27/40) | 87.2% (34/39) | 40 |
| help | none | none | none | none | 100.0% (11/11) | 45.5% (5/11) | none | 11 |
| hop | 25.0% (3/12) | none | 50.0% (1/2) | none | 100.0% (13/13) | 76.9% (10/13) | 66.7% (4/6) | 13 |
| hostile | none | none | none | 100.0% (11/11) | 100.0% (11/11) | 100.0% (11/11) | 100.0% (1/1) | 11 |
| lang | 50.0% (3/6) | 0.0% (0/1) | 100.0% (3/3) | none | 100.0% (9/9) | 22.2% (2/9) | none | 9 |
| missing | 100.0% (2/2) | 0.0% (0/17) | 0.0% (0/1) | none | 100.0% (19/19) | 89.5% (17/19) | none | 19 |
| money | 6.7% (1/15) | none | 100.0% (8/8) | none | 100.0% (27/27) | 66.7% (18/27) | 100.0% (1/1) | 27 |
| prod | 27.3% (6/22) | 0.0% (0/1) | none | none | 100.0% (30/30) | 70.0% (21/30) | none | 30 |
| repair | 11.1% (1/9) | none | 83.3% (5/6) | none | 100.0% (11/11) | 54.5% (6/11) | 90.9% (10/11) | 11 |
| scene | 8.3% (3/36) | 0.0% (0/1) | 83.3% (5/6) | none | 100.0% (30/30) | 63.3% (19/30) | 80.0% (8/10) | 30 |
| xref | 57.1% (4/7) | none | 100.0% (6/6) | none | 100.0% (12/12) | 75.0% (9/12) | none | 12 |

Writing quality is scored separately against each case's rubric at development time (ADR-0028 decision 9); this table covers only the deterministic measures.
