# AI evaluation report

- Cases: 60
- Gold folder: crates/straft-ai-eval/gold
- Model files: the pinned Straft AI of packaging/ai/ai-assets.lock.json
- Build: release
- Search model: Automatic
- Engine place: auto
- Sampling: deterministic (temperature 0, fixed seed)
- Wall time: 10.8 min

## English release bar (ADR-0028 decision 2)

| Measure | Result | Bar |
|---|---|---|
| Correct tool and arguments (tool and action cases) | none | 90% |
| Correct end state after apply (action cases) | none | 85% |

Bar met: **no**

## By kind

| kind | Cases | Setup errors | Right tool offered | Tool chosen | Tool and arguments | Schema valid | End state | Answer facts | Language | Length | Safety | Not empty | Grounding | Recall@5 | Recall@10 | MRR | Median ms | p95 ms | Engine MB |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| safety | 60 | 0 | none | none | none | 100.0% (60/60) | none | 100.0% (48/48) | 60.0% (36/60) | none | 100.0% (60/60) | 100.0% (60/60) | 48.5% | none | none | none | 3052 | 25666 | 2530 |

## By language

| language | Cases | Setup errors | Right tool offered | Tool chosen | Tool and arguments | Schema valid | End state | Answer facts | Language | Length | Safety | Not empty | Grounding | Recall@5 | Recall@10 | MRR | Median ms | p95 ms | Engine MB |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| de | 5 | 0 | none | none | none | 100.0% (5/5) | none | 100.0% (4/4) | 0.0% (0/5) | none | 100.0% (5/5) | 100.0% (5/5) | 50.0% | none | none | none | 2883 | 11872 | 2422 |
| en | 5 | 0 | none | none | none | 100.0% (5/5) | none | 100.0% (4/4) | 100.0% (5/5) | none | 100.0% (5/5) | 100.0% (5/5) | 77.1% | none | none | none | 5245 | 8180 | 2136 |
| es | 5 | 0 | none | none | none | 100.0% (5/5) | none | 100.0% (4/4) | 0.0% (0/5) | none | 100.0% (5/5) | 100.0% (5/5) | 45.8% | none | none | none | 2480 | 30903 | 2416 |
| fr | 5 | 0 | none | none | none | 100.0% (5/5) | none | 100.0% (4/4) | 20.0% (1/5) | none | 100.0% (5/5) | 100.0% (5/5) | 46.3% | none | none | none | 2433 | 12421 | 2422 |
| hi | 5 | 0 | none | none | none | 100.0% (5/5) | none | 100.0% (4/4) | 100.0% (5/5) | none | 100.0% (5/5) | 100.0% (5/5) | 100.0% | none | none | none | 2693 | 25666 | 2469 |
| ja | 5 | 0 | none | none | none | 100.0% (5/5) | none | 100.0% (4/4) | 60.0% (3/5) | none | 100.0% (5/5) | 100.0% (5/5) | 44.2% | none | none | none | 2800 | 21563 | 2349 |
| kn | 5 | 0 | none | none | none | 100.0% (5/5) | none | 100.0% (4/4) | 60.0% (3/5) | none | 100.0% (5/5) | 100.0% (5/5) | 44.7% | none | none | none | 11414 | 21389 | 2376 |
| ko | 5 | 0 | none | none | none | 100.0% (5/5) | none | 100.0% (4/4) | 60.0% (3/5) | none | 100.0% (5/5) | 100.0% (5/5) | 44.7% | none | none | none | 2575 | 21861 | 2382 |
| ml | 5 | 0 | none | none | none | 100.0% (5/5) | none | 100.0% (4/4) | 40.0% (2/5) | none | 100.0% (5/5) | 100.0% (5/5) | 45.7% | none | none | none | 22241 | 41505 | 2364 |
| ta | 5 | 0 | none | none | none | 100.0% (5/5) | none | 100.0% (4/4) | 80.0% (4/5) | none | 100.0% (5/5) | 100.0% (5/5) | 42.9% | none | none | none | 4792 | 38761 | 2530 |
| te | 5 | 0 | none | none | none | 100.0% (5/5) | none | 100.0% (4/4) | 100.0% (5/5) | none | 100.0% (5/5) | 100.0% (5/5) | 80.0% | none | none | none | 3074 | 7529 | 2506 |
| zh | 5 | 0 | none | none | none | 100.0% (5/5) | none | 100.0% (4/4) | 100.0% (5/5) | none | 100.0% (5/5) | 100.0% (5/5) | 78.9% | none | none | none | 2650 | 5207 | 2360 |

## Answer path and fallback

Cases by kind and the answer path they took.

| Kind | write | navigate | change | fixed | Cases |
|---|---|---|---|---|---|
| safety | 57 | 2 | 0 | 1 | 60 |
| all | 57 | 2 | 0 | 1 | 60 |

Cases by kind and the fallback they used. `last_resort` is the last-resort answer; `abstained` is a could-not-find answer.

| Kind | none | last_resort | abstained | Cases |
|---|---|---|---|---|
| safety | 45 | 0 | 15 | 60 |
| all | 45 | 0 | 15 | 60 |

Writing quality is scored separately against each case's rubric at development time (ADR-0028 decision 9); this table covers only the deterministic measures.

## Latency (spec section 7.5)

| Measure | Value | Bar |
|---|---|---|
| Median, all questions (60 asked) | 3.0 s | 15.0 s or lower: met |
| 95th percentile, all questions | 25.7 s | 30.0 s or lower: met |
| Median, questions that used the engine (57) | 3.1 s | |
| 95th percentile, same | 30.9 s | |
| First question after the project opened | 6.6 s | |
| First question that used the engine | 6.6 s | |
| Prompt tokens served from stored work | 40.7% (84247 of 206775) | |
| Completion tokens, engine calls | 17045, 135 | |

| Length class | Questions | Median | 95th percentile |
|---|---|---|---|
| brief | 1 | 1.7 s | 1.7 s |
| narrative | 9 | 18.8 s | 30.9 s |
| one_sentence | 28 | 2.4 s | 3.1 s |
| paragraph | 15 | 8.2 s | 25.7 s |
| short | 5 | 5.2 s | 8.7 s |
| story | 2 | 40.1 s | 41.5 s |

| Stage | Mean per question |
|---|---|
| dossier | 20 ms |
| engine_generation | 6694 ms |
| engine_prompt | 1977 ms |
| turn | 277 ms |

Stage times come from the AI trace and from what the request recorded; a stage nobody records is not listed. Engine prompt and generation are the engine's own figures and include the earlier turns a conversation case asks first, so they add up to more than the time of the last question.
