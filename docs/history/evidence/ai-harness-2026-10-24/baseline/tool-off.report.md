# AI evaluation report

- Cases: 81
- Gold folder: crates/openframe-ai-eval/gold
- Model files: the pinned Offline AI of packaging/ai/ai-assets.lock.json
- Build: release
- Search model: Automatic
- Routes: off (model path only)
- Engine place: auto
- Sampling: deterministic (temperature 0, fixed seed)
- Wall time: 26.4 min

## English release bar (ADR-0028 decision 2)

| Measure | Result | Bar |
|---|---|---|
| Correct tool and arguments (tool and action cases) | 27.2% | 90% |
| Correct end state after apply (action cases) | none | 85% |

Bar met: **no**

## By kind

| kind | Cases | Setup errors | Right tool offered | Tool chosen | Tool and arguments | Schema valid | End state | Answer facts | Language | Length | Safety | Not empty | Grounding | Recall@5 | Recall@10 | MRR | Median ms | p95 ms | Engine MB |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| tool | 81 | 0 | 97.5% (79/81) | 28.4% (23/81) | 27.2% (22/81) | 100.0% (81/81) | none | 37.0% (10/27) | 100.0% (81/81) | none | none | 97.5% (79/81) | 67.9% | none | none | none | 11272 | 25493 | 4065 |

## By language

| language | Cases | Setup errors | Right tool offered | Tool chosen | Tool and arguments | Schema valid | End state | Answer facts | Language | Length | Safety | Not empty | Grounding | Recall@5 | Recall@10 | MRR | Median ms | p95 ms | Engine MB |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| en | 81 | 0 | 97.5% (79/81) | 28.4% (23/81) | 27.2% (22/81) | 100.0% (81/81) | none | 37.0% (10/27) | 100.0% (81/81) | none | none | 97.5% (79/81) | 67.9% | none | none | none | 11272 | 25493 | 4065 |

Writing quality is scored separately against each case's rubric at development time (ADR-0028 decision 9); this table covers only the deterministic measures.
