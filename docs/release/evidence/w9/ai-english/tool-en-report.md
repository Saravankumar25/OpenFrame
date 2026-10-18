# AI evaluation report

- Cases: 71
- Gold folder: crates/openframe-ai-eval/gold
- Model files: the pinned Offline AI of packaging/ai/ai-assets.lock.json
- Build: release
- Search model: Automatic
- Engine place: off (processor)
- Sampling: deterministic (temperature 0, fixed seed)
- Wall time: 14.3 min

## English release bar (ADR-0028 decision 2)

| Measure | Result | Bar |
|---|---|---|
| Correct tool and arguments (tool and action cases) | 100.0% | 90% |
| Correct end state after apply (action cases) | none | 85% |

Bar met: **no**

## By kind

| kind | Cases | Setup errors | Right tool offered | Tool chosen | Tool and arguments | Schema valid | End state | Answer facts | Language | Length | Safety | Not empty | Grounding | Recall@5 | Recall@10 | MRR | Median ms | p95 ms | Engine MB |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| tool | 71 | 0 | 100.0% (71/71) | 100.0% (71/71) | 100.0% (71/71) | 100.0% (71/71) | none | 100.0% (18/18) | 100.0% (71/71) | none | none | 93.0% (66/71) | 77.1% | none | none | none | 81 | 41638 | 2367 |

## By language

| language | Cases | Setup errors | Right tool offered | Tool chosen | Tool and arguments | Schema valid | End state | Answer facts | Language | Length | Safety | Not empty | Grounding | Recall@5 | Recall@10 | MRR | Median ms | p95 ms | Engine MB |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| en | 71 | 0 | 100.0% (71/71) | 100.0% (71/71) | 100.0% (71/71) | 100.0% (71/71) | none | 100.0% (18/18) | 100.0% (71/71) | none | none | 93.0% (66/71) | 77.1% | none | none | none | 81 | 41638 | 2367 |

Writing quality is scored separately against each case's rubric at development time (ADR-0028 decision 9); this table covers only the deterministic measures.
