# AI evaluation report

- Cases: 844
- Gold folder: release-out/eval/gold-legacy
- Model files: the pinned Offline AI of packaging/ai/ai-assets.lock.json
- Build: release
- Search model: Automatic
- Engine place: auto
- Sampling: deterministic (temperature 0, fixed seed)
- Wall time: 90.5 min

## English release bar (ADR-0028 decision 2)

| Measure | Result | Bar |
|---|---|---|
| Correct tool and arguments (tool and action cases) | 58.5% | 90% |
| Correct end state after apply (action cases) | 89.5% | 85% |

Bar met: **no**

## By kind

| kind | Cases | Setup errors | Right tool offered | Tool chosen | Tool and arguments | Schema valid | End state | Answer facts | Language | Length | Safety | Not empty | Grounding | Recall@5 | Recall@10 | MRR | Median ms | p95 ms | Engine MB |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| action | 86 | 0 | 83.7% (72/86) | 95.3% (82/86) | 93.0% (80/86) | 100.0% (86/86) | 89.5% (77/86) | none | 100.0% (86/86) | none | none | 100.0% (86/86) | 67.7% | none | none | none | 14 | 3139 | 2039 |
| conversation | 311 | 0 | none | none | none | 100.0% (311/311) | none | 58.9% (113/192) | 100.0% (311/311) | 65.0% (39/60) | none | 100.0% (311/311) | 59.9% | none | none | none | 3552 | 20797 | 2510 |
| paraphrase | 80 | 0 | 93.8% (75/80) | 5.0% (4/80) | 5.0% (4/80) | 100.0% (80/80) | none | 25.0% (10/40) | 100.0% (80/80) | none | none | 100.0% (80/80) | 66.7% | none | none | none | 2440 | 7408 | 3068 |
| retrieval | 164 | 0 | none | none | none | none | none | 99.4% (163/164) | none | none | none | none | none | 99.4% (163/164) | 100.0% (164/164) | 0.930 | none | none | 0 |
| safety | 46 | 0 | 100.0% (2/2) | 0.0% (0/2) | none | 100.0% (46/46) | 100.0% (1/1) | 85.7% (6/7) | 100.0% (46/46) | none | 100.0% (46/46) | 100.0% (46/46) | 52.2% | none | none | none | 3549 | 15472 | 3650 |
| tool | 97 | 0 | 94.8% (92/97) | 27.8% (27/97) | 27.8% (27/97) | 100.0% (97/97) | none | 71.0% (22/31) | 100.0% (97/97) | none | none | 100.0% (97/97) | 57.7% | none | none | none | 2087 | 7796 | 3630 |
| writing | 60 | 0 | 100.0% (60/60) | 0.0% (0/60) | none | 100.0% (60/60) | none | none | 100.0% (60/60) | 45.0% (27/60) | none | 100.0% (60/60) | 62.1% | none | none | none | 3030 | 11118 | 1942 |

## By language

| language | Cases | Setup errors | Right tool offered | Tool chosen | Tool and arguments | Schema valid | End state | Answer facts | Language | Length | Safety | Not empty | Grounding | Recall@5 | Recall@10 | MRR | Median ms | p95 ms | Engine MB |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| en | 844 | 0 | 92.6% (301/325) | 34.8% (113/325) | 42.2% (111/263) | 100.0% (680/680) | 89.7% (78/87) | 72.4% (314/434) | 100.0% (680/680) | 55.0% (66/120) | 100.0% (46/46) | 100.0% (680/680) | 60.1% | 99.4% (163/164) | 100.0% (164/164) | 0.930 | 2587 | 14376 | 3650 |

## Answer path and fallback

Cases by kind and the answer path they took.

| Kind | write | navigate | change | fixed | Cases |
|---|---|---|---|---|---|
| action | 3 | 0 | 83 | 0 | 86 |
| conversation | 265 | 13 | 0 | 33 | 311 |
| paraphrase | 53 | 5 | 2 | 20 | 80 |
| safety | 27 | 3 | 13 | 3 | 46 |
| tool | 68 | 26 | 2 | 1 | 97 |
| writing | 49 | 0 | 0 | 11 | 60 |
| all | 465 | 47 | 100 | 68 | 680 |

Cases by kind and the fallback they used. `last_resort` is the last-resort answer; `abstained` is a could-not-find answer.

| Kind | none | last_resort | abstained | Cases |
|---|---|---|---|---|
| action | 86 | 0 | 0 | 86 |
| conversation | 269 | 28 | 14 | 311 |
| paraphrase | 79 | 0 | 1 | 80 |
| safety | 41 | 0 | 5 | 46 |
| tool | 91 | 2 | 4 | 97 |
| writing | 56 | 0 | 4 | 60 |
| all | 622 | 30 | 28 | 680 |

## Held-out paraphrases

Scored like tool cases and not part of the English bar. This is the honest estimate of accuracy on real wording.

| Language | Cases | Setup errors | Right tool offered | Tool chosen | Tool and arguments | Schema valid | Answer facts | Median ms |
|---|---|---|---|---|---|---|---|---|
| all | 80 | 0 | 93.8% (75/80) | 5.0% (4/80) | 5.0% (4/80) | 100.0% (80/80) | 25.0% (10/40) | 2440 |
| en | 80 | 0 | 93.8% (75/80) | 5.0% (4/80) | 5.0% (4/80) | 100.0% (80/80) | 25.0% (10/40) | 2440 |

## Conversation bench

Deterministic checks only; the rubric and human labels come at development time. Invented names, wrong scenes and missing quotes are rates over all names, scene citations and quotes in the answers.

| Question type | Cases | Setup errors | Answer facts | Invented names | Wrong scene citations | Missing quotes | Within length | Correct abstention | Repetition | Median ms |
|---|---|---|---|---|---|---|---|---|---|---|
| all | 311 | 0 | 58.9% (113/192) | 39.8% (422/1060) | 1.1% (1/90) | 65.4% (34/52) | 65.0% (39/60) | 70.8% (17/24) | 3.5% (11/311) | 3552 |
| abstain | 4 | 0 | none | 36.4% (4/11) | 0.0% (0/1) | none | 50.0% (2/4) | 75.0% (3/4) | 0.0% (0/4) | 7453 |
| big | 5 | 0 | 100.0% (3/3) | 50.0% (5/10) | 0.0% (0/1) | none | none | none | 20.0% (1/5) | 2104 |
| brain | 11 | 0 | none | 40.0% (16/40) | 0.0% (0/5) | 66.7% (2/3) | none | none | 0.0% (0/11) | 5183 |
| char | 22 | 0 | 75.0% (3/4) | 19.4% (21/108) | 0.0% (0/1) | 100.0% (3/3) | none | none | 0.0% (0/22) | 3011 |
| character | 10 | 0 | 40.0% (4/10) | 46.4% (13/28) | 0.0% (0/3) | none | 60.0% (6/10) | none | 0.0% (0/10) | 2738 |
| compare | 5 | 0 | 0.0% (0/5) | 7.7% (1/13) | 0.0% (0/8) | none | 0.0% (0/5) | none | 0.0% (0/5) | 5233 |
| critique | 4 | 0 | 0.0% (0/4) | 23.5% (4/17) | 0.0% (0/2) | 100.0% (1/1) | 25.0% (1/4) | none | 0.0% (0/4) | 7588 |
| explain | 10 | 0 | 50.0% (5/10) | 2.9% (1/35) | 0.0% (0/3) | none | 80.0% (8/10) | none | 0.0% (0/10) | 5903 |
| export | 40 | 0 | 65.2% (15/23) | 44.4% (72/162) | 0.0% (0/15) | none | none | none | 0.0% (0/40) | 4228 |
| followup | 4 | 0 | 75.0% (3/4) | 0.0% (0/11) | 0.0% (0/2) | none | 75.0% (3/4) | none | 0.0% (0/4) | 8805 |
| help | 11 | 0 | 36.4% (4/11) | 68.6% (24/35) | none | 77.8% (7/9) | none | none | 0.0% (0/11) | 8369 |
| hop | 13 | 0 | 44.4% (4/9) | 19.2% (10/52) | 0.0% (0/3) | 100.0% (3/3) | none | none | 0.0% (0/13) | 3206 |
| hostile | 11 | 0 | none | 100.0% (3/3) | none | none | none | none | 0.0% (0/11) | 1441 |
| ideas | 5 | 0 | 0.0% (0/5) | 100.0% (5/5) | none | none | 60.0% (3/5) | none | 0.0% (0/5) | 18356 |
| lang | 9 | 0 | 50.0% (2/4) | 35.9% (14/39) | 0.0% (0/1) | 50.0% (1/2) | none | 100.0% (1/1) | 0.0% (0/9) | 4427 |
| missing | 19 | 0 | 100.0% (1/1) | 52.0% (26/50) | 0.0% (0/4) | 50.0% (1/2) | none | 70.6% (12/17) | 0.0% (0/19) | 2633 |
| money | 27 | 0 | 87.5% (21/24) | 63.1% (41/65) | 0.0% (0/5) | 100.0% (3/3) | none | none | 0.0% (0/27) | 3355 |
| prod | 30 | 0 | 88.9% (16/18) | 71.0% (49/69) | 0.0% (0/1) | 80.0% (4/5) | none | 100.0% (1/1) | 0.0% (0/30) | 2699 |
| relationship | 8 | 0 | 50.0% (4/8) | 16.7% (4/24) | 0.0% (0/1) | none | 100.0% (8/8) | none | 25.0% (2/8) | 7808 |
| repair | 11 | 0 | 100.0% (7/7) | 58.8% (20/34) | 0.0% (0/5) | 100.0% (2/2) | none | none | 0.0% (0/11) | 3398 |
| scene | 30 | 0 | 52.4% (11/21) | 36.5% (54/148) | 0.0% (0/16) | 60.0% (3/5) | none | 0.0% (0/1) | 16.7% (5/30) | 3843 |
| theme | 5 | 0 | 40.0% (2/5) | 52.9% (9/17) | none | none | 60.0% (3/5) | none | 20.0% (1/5) | 7570 |
| whole | 5 | 0 | 20.0% (1/5) | 13.8% (4/29) | none | 0.0% (0/7) | 100.0% (5/5) | none | 20.0% (1/5) | 21870 |
| xref | 12 | 0 | 63.6% (7/11) | 40.0% (22/55) | 7.7% (1/13) | 57.1% (4/7) | none | none | 8.3% (1/12) | 7396 |

Expectation flags (harness spec 7.4). Cases that set none of them show none. Flags the build cannot measure yet are listed as not counted, never as passed or failed.

| Question type | Required facts | Abstain names | No navigation | No data dump | No internal words | Length class | No re-ask | Answer type | Answer group | Not counted |
|---|---|---|---|---|---|---|---|---|---|---|
| all | 73.5% (139/189) | 60.0% (12/20) | 98.5% (66/67) | 95.2% (20/21) | 100.0% (251/251) | 82.9% (208/251) | 97.2% (69/71) | 80.5% (202/251) | 91.6% (230/251) | 0 |
| abstain | none | none | none | none | none | none | none | none | none | 0 |
| big | 80.0% (4/5) | none | none | none | 100.0% (5/5) | 80.0% (4/5) | none | 100.0% (5/5) | 100.0% (5/5) | 0 |
| brain | 58.3% (7/12) | none | 100.0% (11/11) | none | 100.0% (11/11) | 45.5% (5/11) | none | 72.7% (8/11) | 72.7% (8/11) | 0 |
| char | 85.7% (30/35) | none | 100.0% (11/11) | none | 100.0% (22/22) | 68.2% (15/22) | 100.0% (3/3) | 54.5% (12/22) | 63.6% (14/22) | 0 |
| character | none | none | none | none | none | none | none | none | none | 0 |
| compare | none | none | none | none | none | none | none | none | none | 0 |
| critique | none | none | none | none | none | none | none | none | none | 0 |
| explain | none | none | none | none | none | none | none | none | none | 0 |
| export | 60.7% (17/28) | none | 100.0% (13/13) | 90.0% (9/10) | 100.0% (40/40) | 92.5% (37/40) | 97.4% (38/39) | 87.5% (35/40) | 92.5% (37/40) | 0 |
| followup | none | none | none | none | none | none | none | none | none | 0 |
| help | none | none | none | none | 100.0% (11/11) | 63.6% (7/11) | none | 90.9% (10/11) | 90.9% (10/11) | 0 |
| hop | 58.3% (7/12) | none | 100.0% (2/2) | none | 100.0% (13/13) | 84.6% (11/13) | 100.0% (6/6) | 69.2% (9/13) | 100.0% (13/13) | 0 |
| hostile | none | none | none | 100.0% (11/11) | 100.0% (11/11) | 100.0% (11/11) | 100.0% (1/1) | 100.0% (11/11) | 100.0% (11/11) | 0 |
| ideas | none | none | none | none | none | none | none | none | none | 0 |
| lang | 83.3% (5/6) | 100.0% (1/1) | 100.0% (3/3) | none | 100.0% (9/9) | 88.9% (8/9) | none | 66.7% (6/9) | 88.9% (8/9) | 0 |
| missing | 100.0% (2/2) | 58.8% (10/17) | 0.0% (0/1) | none | 100.0% (19/19) | 100.0% (19/19) | none | 78.9% (15/19) | 89.5% (17/19) | 0 |
| money | 66.7% (10/15) | none | 100.0% (8/8) | none | 100.0% (27/27) | 92.6% (25/27) | 100.0% (1/1) | 81.5% (22/27) | 100.0% (27/27) | 0 |
| prod | 72.7% (16/22) | 100.0% (1/1) | none | none | 100.0% (30/30) | 90.0% (27/30) | none | 93.3% (28/30) | 100.0% (30/30) | 0 |
| relationship | none | none | none | none | none | none | none | none | none | 0 |
| repair | 44.4% (4/9) | none | 100.0% (6/6) | none | 100.0% (11/11) | 90.9% (10/11) | 100.0% (11/11) | 90.9% (10/11) | 90.9% (10/11) | 0 |
| scene | 83.3% (30/36) | 0.0% (0/1) | 100.0% (6/6) | none | 100.0% (30/30) | 70.0% (21/30) | 90.0% (9/10) | 70.0% (21/30) | 93.3% (28/30) | 0 |
| theme | none | none | none | none | none | none | none | none | none | 0 |
| whole | none | none | none | none | none | none | none | none | none | 0 |
| xref | 100.0% (7/7) | none | 100.0% (6/6) | none | 100.0% (12/12) | 66.7% (8/12) | none | 83.3% (10/12) | 100.0% (12/12) | 0 |

## Out-of-section pointer (ADR-0041)

| Pointer precision | Pointer recall | False-pointer rate |
|---|---|---|
| none | none | 0.0% (0/37) |

Writing quality is scored separately against each case's rubric at development time (ADR-0028 decision 9); this table covers only the deterministic measures.

## Latency (spec section 7.5)

| Measure | Value | Bar |
|---|---|---|
| Median, all questions (680 asked) | 2.6 s | 15.0 s or lower: met |
| 95th percentile, all questions | 14.4 s | 30.0 s or lower: met |
| Median, questions that used the engine (536) | 3.2 s | |
| 95th percentile, same | 16.4 s | |
| First question after the project opened | 7.7 s | |
| First question that used the engine | 7.7 s | |
| Prompt tokens served from stored work | 54.3% (1200591 of 2210688) | |
| Completion tokens, engine calls | 109082, 1534 | |

| Length class | Questions | Median | 95th percentile |
|---|---|---|---|
| brief | 23 | 1.5 s | 3.8 s |
| narrative | 50 | 6.6 s | 28.8 s |
| one_sentence | 374 | 1.8 s | 4.4 s |
| paragraph | 119 | 7.6 s | 21.8 s |
| short | 101 | 4.5 s | 9.9 s |
| story | 13 | 7.7 s | 68.0 s |

| Stage | Mean per question |
|---|---|
| dossier | 38 ms |
| engine_generation | 5957 ms |
| engine_prompt | 2143 ms |
| turn | 127 ms |

Stage times come from the AI trace and from what the request recorded; a stage nobody records is not listed. Engine prompt and generation are the engine's own figures and include the earlier turns a conversation case asks first, so they add up to more than the time of the last question.
