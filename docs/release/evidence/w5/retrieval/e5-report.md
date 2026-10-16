# AI evaluation report

- Cases: 252
- Gold folder: crates/openframe-ai-eval/gold
- Model files: the pinned Offline AI of packaging/ai/ai-assets.lock.json
- Build: release
- Search model: Multilingual
- Sampling: deterministic (temperature 0, fixed seed)
- Wall time: 1.8 min

## English release bar (ADR-0028 decision 2)

| Measure | Result | Bar |
|---|---|---|
| Correct tool and arguments (tool and action cases) | none | 90% |
| Correct end state after apply (action cases) | none | 85% |

Bar met: **no**

## By kind

| kind | Cases | Setup errors | Right tool offered | Tool chosen | Tool and arguments | Schema valid | End state | Answer facts | Language | Length | Safety | Not empty | Grounding | Recall@5 | Recall@10 | MRR | Median ms | p95 ms | Engine MB |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| retrieval | 252 | 0 | none | none | none | none | none | 92.1% (232/252) | none | none | none | none | none | 92.1% (232/252) | 96.8% (244/252) | 0.790 | none | none | 0 |

## By language

| language | Cases | Setup errors | Right tool offered | Tool chosen | Tool and arguments | Schema valid | End state | Answer facts | Language | Length | Safety | Not empty | Grounding | Recall@5 | Recall@10 | MRR | Median ms | p95 ms | Engine MB |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| de | 8 | 0 | none | none | none | none | none | 100.0% (8/8) | none | none | none | none | none | 100.0% (8/8) | 100.0% (8/8) | 0.677 | none | none | 0 |
| en | 164 | 0 | none | none | none | none | none | 99.4% (163/164) | none | none | none | none | none | 99.4% (163/164) | 100.0% (164/164) | 0.930 | none | none | 0 |
| es | 8 | 0 | none | none | none | none | none | 87.5% (7/8) | none | none | none | none | none | 87.5% (7/8) | 87.5% (7/8) | 0.650 | none | none | 0 |
| fr | 8 | 0 | none | none | none | none | none | 50.0% (4/8) | none | none | none | none | none | 50.0% (4/8) | 87.5% (7/8) | 0.376 | none | none | 0 |
| hi | 8 | 0 | none | none | none | none | none | 87.5% (7/8) | none | none | none | none | none | 87.5% (7/8) | 100.0% (8/8) | 0.271 | none | none | 0 |
| ja | 8 | 0 | none | none | none | none | none | 50.0% (4/8) | none | none | none | none | none | 50.0% (4/8) | 75.0% (6/8) | 0.297 | none | none | 0 |
| kn | 8 | 0 | none | none | none | none | none | 87.5% (7/8) | none | none | none | none | none | 87.5% (7/8) | 100.0% (8/8) | 0.667 | none | none | 0 |
| ko | 8 | 0 | none | none | none | none | none | 62.5% (5/8) | none | none | none | none | none | 62.5% (5/8) | 75.0% (6/8) | 0.537 | none | none | 0 |
| ml | 8 | 0 | none | none | none | none | none | 100.0% (8/8) | none | none | none | none | none | 100.0% (8/8) | 100.0% (8/8) | 0.792 | none | none | 0 |
| ta | 8 | 0 | none | none | none | none | none | 100.0% (8/8) | none | none | none | none | none | 100.0% (8/8) | 100.0% (8/8) | 0.625 | none | none | 0 |
| te | 8 | 0 | none | none | none | none | none | 87.5% (7/8) | none | none | none | none | none | 87.5% (7/8) | 100.0% (8/8) | 0.620 | none | none | 0 |
| zh | 8 | 0 | none | none | none | none | none | 50.0% (4/8) | none | none | none | none | none | 50.0% (4/8) | 75.0% (6/8) | 0.297 | none | none | 0 |

Writing quality is scored separately against each case's rubric at development time (ADR-0028 decision 9); this table covers only the deterministic measures.
