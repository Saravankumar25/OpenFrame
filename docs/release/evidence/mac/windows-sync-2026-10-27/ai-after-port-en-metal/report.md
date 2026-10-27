# AI evaluation report

- Cases: 844
- Gold folder: .build-cache/eval-new/gold
- Model files: the pinned Offline AI of packaging/ai/ai-assets.lock.json
- Build: release
- Search model: Automatic
- Engine place: auto
- Sampling: deterministic (temperature 0, fixed seed)
- Wall time: 77.0 min

## English release bar (ADR-0028 decision 2)

| Measure | Result | Bar |
|---|---|---|
| Correct tool and arguments (tool and action cases) | 59.0% | 90% |
| Correct end state after apply (action cases) | 89.5% | 85% |

Bar met: **no**

## By kind

| kind | Cases | Setup errors | Right tool offered | Tool chosen | Tool and arguments | Schema valid | End state | Answer facts | Language | Length | Safety | Not empty | Grounding | Recall@5 | Recall@10 | MRR | Median ms | p95 ms | Engine MB |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| action | 86 | 0 | 86.0% (74/86) | 96.5% (83/86) | 94.2% (81/86) | 100.0% (86/86) | 89.5% (77/86) | none | 100.0% (86/86) | none | none | 100.0% (86/86) | 68.9% | none | none | none | 13 | 3439 | 3182 |
| conversation | 311 | 0 | none | none | none | 100.0% (311/311) | none | 64.6% (124/192) | 100.0% (311/311) | 71.7% (43/60) | none | 100.0% (311/311) | 66.0% | none | none | none | 2486 | 16158 | 2048 |
| paraphrase | 80 | 0 | 95.0% (76/80) | 6.2% (5/80) | 6.2% (5/80) | 100.0% (80/80) | none | 40.0% (16/40) | 100.0% (80/80) | none | none | 100.0% (80/80) | 69.5% | none | none | none | 1585 | 12438 | 3312 |
| retrieval | 164 | 0 | none | none | none | none | none | 99.4% (163/164) | none | none | none | none | none | 99.4% (163/164) | 100.0% (164/164) | 0.933 | none | none | 0 |
| safety | 46 | 0 | 100.0% (2/2) | 0.0% (0/2) | none | 100.0% (46/46) | 100.0% (1/1) | 100.0% (7/7) | 100.0% (46/46) | none | 100.0% (46/46) | 97.8% (45/46) | 70.0% | none | none | none | 2408 | 15826 | 2273 |
| tool | 97 | 0 | 94.8% (92/97) | 27.8% (27/97) | 27.8% (27/97) | 100.0% (97/97) | none | 54.8% (17/31) | 100.0% (97/97) | none | none | 99.0% (96/97) | 67.0% | none | none | none | 1128 | 8035 | 2848 |
| writing | 60 | 0 | 100.0% (60/60) | 0.0% (0/60) | none | 100.0% (60/60) | none | none | 100.0% (60/60) | 46.7% (28/60) | none | 100.0% (60/60) | 86.1% | none | none | none | 3478 | 11767 | 3256 |

## By language

| language | Cases | Setup errors | Right tool offered | Tool chosen | Tool and arguments | Schema valid | End state | Answer facts | Language | Length | Safety | Not empty | Grounding | Recall@5 | Recall@10 | MRR | Median ms | p95 ms | Engine MB |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| en | 844 | 0 | 93.5% (304/325) | 35.4% (115/325) | 43.0% (113/263) | 100.0% (680/680) | 89.7% (78/87) | 75.3% (327/434) | 100.0% (680/680) | 59.2% (71/120) | 100.0% (46/46) | 99.7% (678/680) | 68.4% | 99.4% (163/164) | 100.0% (164/164) | 0.933 | 1945 | 13293 | 3312 |

## Answer path and fallback

Cases by kind and the answer path they took.

| Kind | write | navigate | change | fixed | Cases |
|---|---|---|---|---|---|
| action | 3 | 0 | 83 | 0 | 86 |
| conversation | 282 | 18 | 0 | 11 | 311 |
| paraphrase | 66 | 12 | 2 | 0 | 80 |
| safety | 24 | 6 | 13 | 3 | 46 |
| tool | 66 | 29 | 2 | 0 | 97 |
| writing | 60 | 0 | 0 | 0 | 60 |
| all | 501 | 65 | 100 | 14 | 680 |

Cases by kind and the fallback they used. `last_resort` is the last-resort answer; `abstained` is a could-not-find answer.

| Kind | none | last_resort | abstained | Cases |
|---|---|---|---|---|
| action | 85 | 0 | 1 | 86 |
| conversation | 206 | 33 | 72 | 311 |
| paraphrase | 56 | 0 | 24 | 80 |
| safety | 30 | 1 | 15 | 46 |
| tool | 73 | 5 | 19 | 97 |
| writing | 41 | 0 | 19 | 60 |
| all | 491 | 39 | 150 | 680 |

## Held-out paraphrases

Scored like tool cases and not part of the English bar. This is the honest estimate of accuracy on real wording.

| Language | Cases | Setup errors | Right tool offered | Tool chosen | Tool and arguments | Schema valid | Answer facts | Median ms |
|---|---|---|---|---|---|---|---|---|
| all | 80 | 0 | 95.0% (76/80) | 6.2% (5/80) | 6.2% (5/80) | 100.0% (80/80) | 40.0% (16/40) | 1585 |
| en | 80 | 0 | 95.0% (76/80) | 6.2% (5/80) | 6.2% (5/80) | 100.0% (80/80) | 40.0% (16/40) | 1585 |

## Conversation bench

Deterministic checks only; the rubric and human labels come at development time. Invented names, wrong scenes and missing quotes are rates over all names, scene citations and quotes in the answers.

| Question type | Cases | Setup errors | Answer facts | Invented names | Wrong scene citations | Missing quotes | Within length | Correct abstention | Repetition | Median ms |
|---|---|---|---|---|---|---|---|---|---|---|
| all | 311 | 0 | 64.6% (124/192) | 33.1% (269/813) | 1.1% (1/90) | 58.7% (37/63) | 71.7% (43/60) | 62.5% (15/24) | 0.6% (2/311) | 2486 |
| abstain | 4 | 0 | none | 75.0% (3/4) | none | none | 100.0% (4/4) | 25.0% (1/4) | 0.0% (0/4) | 12043 |
| big | 5 | 0 | 100.0% (3/3) | 44.4% (4/9) | 0.0% (0/1) | 100.0% (2/2) | none | none | 0.0% (0/5) | 904 |
| brain | 11 | 0 | none | 40.5% (17/42) | 0.0% (0/1) | 66.7% (2/3) | none | none | 0.0% (0/11) | 12128 |
| char | 22 | 0 | 100.0% (4/4) | 4.4% (4/90) | none | 0.0% (0/1) | none | none | 0.0% (0/22) | 2833 |
| character | 10 | 0 | 30.0% (3/10) | 56.5% (13/23) | none | none | 70.0% (7/10) | none | 0.0% (0/10) | 4956 |
| compare | 5 | 0 | 60.0% (3/5) | 0.0% (0/13) | 0.0% (0/10) | 0.0% (0/1) | 0.0% (0/5) | none | 0.0% (0/5) | 3039 |
| critique | 4 | 0 | 75.0% (3/4) | 0.0% (0/16) | 0.0% (0/3) | none | 75.0% (3/4) | none | 0.0% (0/4) | 4473 |
| explain | 10 | 0 | 80.0% (8/10) | 10.5% (4/38) | 0.0% (0/7) | 100.0% (1/1) | 90.0% (9/10) | none | 0.0% (0/10) | 3969 |
| export | 40 | 0 | 73.9% (17/23) | 39.1% (45/115) | 0.0% (0/12) | 45.5% (5/11) | none | none | 0.0% (0/40) | 1424 |
| followup | 4 | 0 | 50.0% (2/4) | 10.0% (1/10) | 0.0% (0/2) | 100.0% (1/1) | 75.0% (3/4) | none | 0.0% (0/4) | 3569 |
| help | 11 | 0 | 45.5% (5/11) | 70.8% (34/48) | none | 60.0% (3/5) | none | none | 0.0% (0/11) | 3162 |
| hop | 13 | 0 | 55.6% (5/9) | 42.3% (11/26) | 0.0% (0/6) | 83.3% (5/6) | none | none | 0.0% (0/13) | 2067 |
| hostile | 11 | 0 | none | 100.0% (4/4) | none | none | none | none | 0.0% (0/11) | 1316 |
| ideas | 5 | 0 | 100.0% (5/5) | 0.0% (0/17) | 0.0% (0/3) | none | 100.0% (5/5) | none | 0.0% (0/5) | 12971 |
| lang | 9 | 0 | 50.0% (2/4) | 36.4% (8/22) | 0.0% (0/1) | 50.0% (1/2) | none | 100.0% (1/1) | 0.0% (0/9) | 3282 |
| missing | 19 | 0 | 100.0% (1/1) | 57.1% (20/35) | 0.0% (0/3) | 77.8% (7/9) | none | 70.6% (12/17) | 5.3% (1/19) | 1304 |
| money | 27 | 0 | 75.0% (18/24) | 75.4% (46/61) | none | 50.0% (1/2) | none | none | 0.0% (0/27) | 2139 |
| prod | 30 | 0 | 77.8% (14/18) | 53.2% (25/47) | 0.0% (0/1) | 50.0% (2/4) | none | 0.0% (0/1) | 0.0% (0/30) | 1952 |
| relationship | 8 | 0 | 62.5% (5/8) | 15.8% (6/38) | 0.0% (0/5) | none | 100.0% (8/8) | none | 0.0% (0/8) | 6157 |
| repair | 11 | 0 | 100.0% (7/7) | 28.6% (8/28) | 0.0% (0/6) | 33.3% (1/3) | none | none | 0.0% (0/11) | 1278 |
| scene | 30 | 0 | 57.1% (12/21) | 15.7% (13/83) | 0.0% (0/19) | 50.0% (2/4) | none | 100.0% (1/1) | 3.3% (1/30) | 2042 |
| theme | 5 | 0 | 20.0% (1/5) | 11.1% (1/9) | 0.0% (0/1) | 100.0% (1/1) | 60.0% (3/5) | none | 0.0% (0/5) | 5402 |
| whole | 5 | 0 | 20.0% (1/5) | 18.2% (2/11) | 0.0% (0/1) | none | 20.0% (1/5) | none | 0.0% (0/5) | 12643 |
| xref | 12 | 0 | 45.5% (5/11) | 0.0% (0/24) | 12.5% (1/8) | 42.9% (3/7) | none | none | 0.0% (0/12) | 4080 |

Expectation flags (harness spec 7.4). Cases that set none of them show none. Flags the build cannot measure yet are listed as not counted, never as passed or failed.

| Question type | Required facts | Abstain names | No navigation | No data dump | No internal words | Length class | No re-ask | Answer type | Answer group | Not counted |
|---|---|---|---|---|---|---|---|---|---|---|
| all | 72.5% (137/189) | 70.0% (14/20) | 97.0% (65/67) | 95.2% (20/21) | 100.0% (251/251) | 92.0% (231/251) | 97.2% (69/71) | 84.1% (211/251) | 91.6% (230/251) | 0 |
| abstain | none | none | none | none | none | none | none | none | none | 0 |
| big | 100.0% (5/5) | none | none | none | 100.0% (5/5) | 100.0% (5/5) | none | 100.0% (5/5) | 100.0% (5/5) | 0 |
| brain | 58.3% (7/12) | none | 90.9% (10/11) | none | 100.0% (11/11) | 81.8% (9/11) | none | 72.7% (8/11) | 72.7% (8/11) | 0 |
| char | 88.6% (31/35) | none | 100.0% (11/11) | none | 100.0% (22/22) | 77.3% (17/22) | 100.0% (3/3) | 68.2% (15/22) | 72.7% (16/22) | 0 |
| character | none | none | none | none | none | none | none | none | none | 0 |
| compare | none | none | none | none | none | none | none | none | none | 0 |
| critique | none | none | none | none | none | none | none | none | none | 0 |
| explain | none | none | none | none | none | none | none | none | none | 0 |
| export | 78.6% (22/28) | none | 100.0% (13/13) | 90.0% (9/10) | 100.0% (40/40) | 97.5% (39/40) | 97.4% (38/39) | 90.0% (36/40) | 92.5% (37/40) | 0 |
| followup | none | none | none | none | none | none | none | none | none | 0 |
| help | none | none | none | none | 100.0% (11/11) | 81.8% (9/11) | none | 90.9% (10/11) | 90.9% (10/11) | 0 |
| hop | 50.0% (6/12) | none | 100.0% (2/2) | none | 100.0% (13/13) | 92.3% (12/13) | 100.0% (6/6) | 84.6% (11/13) | 100.0% (13/13) | 0 |
| hostile | none | none | none | 100.0% (11/11) | 100.0% (11/11) | 100.0% (11/11) | 100.0% (1/1) | 100.0% (11/11) | 100.0% (11/11) | 0 |
| ideas | none | none | none | none | none | none | none | none | none | 0 |
| lang | 83.3% (5/6) | 100.0% (1/1) | 100.0% (3/3) | none | 100.0% (9/9) | 88.9% (8/9) | none | 77.8% (7/9) | 100.0% (9/9) | 0 |
| missing | 100.0% (2/2) | 70.6% (12/17) | 0.0% (0/1) | none | 100.0% (19/19) | 100.0% (19/19) | none | 89.5% (17/19) | 94.7% (18/19) | 0 |
| money | 60.0% (9/15) | none | 100.0% (8/8) | none | 100.0% (27/27) | 92.6% (25/27) | 100.0% (1/1) | 77.8% (21/27) | 96.3% (26/27) | 0 |
| prod | 63.6% (14/22) | 0.0% (0/1) | none | none | 100.0% (30/30) | 100.0% (30/30) | none | 86.7% (26/30) | 96.7% (29/30) | 0 |
| relationship | none | none | none | none | none | none | none | none | none | 0 |
| repair | 66.7% (6/9) | none | 100.0% (6/6) | none | 100.0% (11/11) | 90.9% (10/11) | 100.0% (11/11) | 90.9% (10/11) | 90.9% (10/11) | 0 |
| scene | 69.4% (25/36) | 100.0% (1/1) | 100.0% (6/6) | none | 100.0% (30/30) | 90.0% (27/30) | 90.0% (9/10) | 80.0% (24/30) | 93.3% (28/30) | 0 |
| theme | none | none | none | none | none | none | none | none | none | 0 |
| whole | none | none | none | none | none | none | none | none | none | 0 |
| xref | 71.4% (5/7) | none | 100.0% (6/6) | none | 100.0% (12/12) | 83.3% (10/12) | none | 83.3% (10/12) | 83.3% (10/12) | 0 |

Writing quality is scored separately against each case's rubric at development time (ADR-0028 decision 9); this table covers only the deterministic measures.

## Latency (spec section 7.5)

| Measure | Value | Bar |
|---|---|---|
| Median, all questions (680 asked) | 1.9 s | 15.0 s or lower: met |
| 95th percentile, all questions | 13.3 s | 30.0 s or lower: met |
| Median, questions that used the engine (561) | 2.6 s | |
| 95th percentile, same | 14.0 s | |
| First question after the project opened | 17.0 s | |
| First question that used the engine | 17.0 s | |
| Prompt tokens served from stored work | 62.4% (680478 of 1089720) | |
| Completion tokens, engine calls | 111342, 1554 | |

| Length class | Questions | Median | 95th percentile |
|---|---|---|---|
| brief | 23 | 1.1 s | 3.4 s |
| one_sentence | 369 | 0.8 s | 3.4 s |
| paragraph | 186 | 7.0 s | 18.9 s |
| short | 102 | 3.5 s | 11.0 s |

| Stage | Mean per question |
|---|---|
| dossier | 0 ms |
| engine_generation | 5467 ms |
| engine_prompt | 812 ms |
| turn | 180 ms |

Stage times come from the AI trace and from what the request recorded; a stage nobody records is not listed. Engine prompt and generation are the engine's own figures and include the earlier turns a conversation case asks first, so they add up to more than the time of the last question.
