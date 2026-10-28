# AI evaluation report

- Cases: 81
- Gold folder: crates/openframe-ai-eval/gold
- Model files: the pinned Offline AI of packaging/ai/ai-assets.lock.json
- Build: release
- Search model: Automatic
- Routes: on (regex direct routes active)
- Engine place: auto
- Sampling: deterministic (temperature 0, fixed seed)
- Wall time: 11.7 min

## English release bar (ADR-0028 decision 2)

| Measure | Result | Bar |
|---|---|---|
| Correct tool and arguments (tool and action cases) | 97.5% | 90% |
| Correct end state after apply (action cases) | none | 85% |

Bar met: **no**

## By kind

| kind | Cases | Setup errors | Right tool offered | Tool chosen | Tool and arguments | Schema valid | End state | Answer facts | Language | Length | Safety | Not empty | Grounding | Recall@5 | Recall@10 | MRR | Median ms | p95 ms | Engine MB |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| tool | 81 | 0 | 97.5% (79/81) | 97.5% (79/81) | 97.5% (79/81) | 100.0% (81/81) | none | 96.3% (26/27) | 100.0% (81/81) | none | none | 93.8% (76/81) | 78.4% | none | none | none | 90 | 25045 | 1809 |

## By language

| language | Cases | Setup errors | Right tool offered | Tool chosen | Tool and arguments | Schema valid | End state | Answer facts | Language | Length | Safety | Not empty | Grounding | Recall@5 | Recall@10 | MRR | Median ms | p95 ms | Engine MB |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| en | 81 | 0 | 97.5% (79/81) | 97.5% (79/81) | 97.5% (79/81) | 100.0% (81/81) | none | 96.3% (26/27) | 100.0% (81/81) | none | none | 93.8% (76/81) | 78.4% | none | none | none | 90 | 25045 | 1809 |

## Lanes and intents

Cases by kind and the lane they took.

| Kind | direct | chat | recipe | agent | action | Cases |
|---|---|---|---|---|---|---|
| tool | 76 | 0 | 0 | 5 | 0 | 81 |
| all | 76 | 0 | 0 | 5 | 0 | 81 |

Cases that had a closest intent, by its score. Matched: the intent's tool is a tool the case expects. Unjudged: the case expects no tool.

| Score | Matched | Missed | Unjudged | Matched of judged |
|---|---|---|---|---|
| 0.84 to 0.86 | 1 | 1 | 0 | 50.0% (1/2) |
| 0.90 to 0.92 | 1 | 0 | 0 | 100.0% (1/1) |

Writing quality is scored separately against each case's rubric at development time (ADR-0028 decision 9); this table covers only the deterministic measures.
