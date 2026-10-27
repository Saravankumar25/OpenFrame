# AI evaluation report

- Cases: 41
- Gold folder: crates/openframe-ai-eval/gold
- Model files: the pinned Offline AI of packaging/ai/ai-assets.lock.json
- Build: release
- Search model: Automatic
- Routes: on (regex direct routes active)
- Engine place: auto
- Sampling: deterministic (temperature 0, fixed seed)
- Wall time: 11.4 min

## English release bar (ADR-0028 decision 2)

| Measure | Result | Bar |
|---|---|---|
| Correct tool and arguments (tool and action cases) | none | 90% |
| Correct end state after apply (action cases) | none | 85% |

Bar met: **no**

## By kind

| kind | Cases | Setup errors | Right tool offered | Tool chosen | Tool and arguments | Schema valid | End state | Answer facts | Language | Length | Safety | Not empty | Grounding | Recall@5 | Recall@10 | MRR | Median ms | p95 ms | Engine MB |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| safety | 41 | 0 | 100.0% (2/2) | 100.0% (2/2) | none | 100.0% (41/41) | 100.0% (1/1) | 100.0% (3/3) | 100.0% (41/41) | none | 100.0% (41/41) | 97.6% (40/41) | 65.7% | none | none | none | 7730 | 20004 | 3009 |

## By language

| language | Cases | Setup errors | Right tool offered | Tool chosen | Tool and arguments | Schema valid | End state | Answer facts | Language | Length | Safety | Not empty | Grounding | Recall@5 | Recall@10 | MRR | Median ms | p95 ms | Engine MB |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| en | 41 | 0 | 100.0% (2/2) | 100.0% (2/2) | none | 100.0% (41/41) | 100.0% (1/1) | 100.0% (3/3) | 100.0% (41/41) | none | 100.0% (41/41) | 97.6% (40/41) | 65.7% | none | none | none | 7730 | 20004 | 3009 |

## Lanes and intents

Cases by kind and the lane they took.

| Kind | direct | chat | recipe | agent | action | Cases |
|---|---|---|---|---|---|---|
| safety | 12 | 3 | 1 | 19 | 6 | 41 |
| all | 12 | 3 | 1 | 19 | 6 | 41 |

Cases that had a closest intent, by its score. Matched: the intent's tool is a tool the case expects. Unjudged: the case expects no tool.

| Score | Matched | Missed | Unjudged | Matched of judged |
|---|---|---|---|---|
| 0.78 to 0.80 | 0 | 0 | 1 | none |
| 0.80 to 0.82 | 0 | 0 | 1 | none |
| 0.82 to 0.84 | 0 | 0 | 3 | none |
| 0.84 to 0.86 | 0 | 0 | 5 | none |
| 0.86 to 0.88 | 0 | 0 | 6 | none |
| 0.88 to 0.90 | 0 | 0 | 1 | none |
| 0.90 to 0.92 | 0 | 0 | 1 | none |

Writing quality is scored separately against each case's rubric at development time (ADR-0028 decision 9); this table covers only the deterministic measures.
