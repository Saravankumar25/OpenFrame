# AI evaluation report

- Cases: 252
- Gold folder: crates/openframe-ai-eval/gold
- Model files: the pinned Offline AI of packaging/ai/ai-assets.lock.json
- Build: release
- Search model: English
- Sampling: deterministic (temperature 0, fixed seed)
- Wall time: 1.6 min

## English release bar (ADR-0028 decision 2)

| Measure | Result | Bar |
|---|---|---|
| Correct tool and arguments (tool and action cases) | none | 90% |
| Correct end state after apply (action cases) | none | 85% |

Bar met: **no**

## By kind

| kind | Cases | Setup errors | Right tool offered | Tool chosen | Tool and arguments | Schema valid | End state | Answer facts | Language | Length | Safety | Not empty | Grounding | Recall@5 | Recall@10 | MRR | Median ms | p95 ms | Engine MB |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| retrieval | 252 | 0 | none | none | none | none | none | 80.2% (202/252) | none | none | none | none | none | 80.2% (202/252) | 88.9% (224/252) | 0.676 | none | none | 0 |

## By language

| language | Cases | Setup errors | Right tool offered | Tool chosen | Tool and arguments | Schema valid | End state | Answer facts | Language | Length | Safety | Not empty | Grounding | Recall@5 | Recall@10 | MRR | Median ms | p95 ms | Engine MB |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| de | 8 | 0 | none | none | none | none | none | 62.5% (5/8) | none | none | none | none | none | 62.5% (5/8) | 87.5% (7/8) | 0.407 | none | none | 0 |
| en | 164 | 0 | none | none | none | none | none | 98.2% (161/164) | none | none | none | none | none | 98.2% (161/164) | 99.4% (163/164) | 0.879 | none | none | 0 |
| es | 8 | 0 | none | none | none | none | none | 75.0% (6/8) | none | none | none | none | none | 75.0% (6/8) | 87.5% (7/8) | 0.594 | none | none | 0 |
| fr | 8 | 0 | none | none | none | none | none | 50.0% (4/8) | none | none | none | none | none | 50.0% (4/8) | 100.0% (8/8) | 0.407 | none | none | 0 |
| hi | 8 | 0 | none | none | none | none | none | 25.0% (2/8) | none | none | none | none | none | 25.0% (2/8) | 37.5% (3/8) | 0.266 | none | none | 0 |
| ja | 8 | 0 | none | none | none | none | none | 50.0% (4/8) | none | none | none | none | none | 50.0% (4/8) | 75.0% (6/8) | 0.297 | none | none | 0 |
| kn | 8 | 0 | none | none | none | none | none | 25.0% (2/8) | none | none | none | none | none | 25.0% (2/8) | 50.0% (4/8) | 0.095 | none | none | 0 |
| ko | 8 | 0 | none | none | none | none | none | 62.5% (5/8) | none | none | none | none | none | 62.5% (5/8) | 62.5% (5/8) | 0.300 | none | none | 0 |
| ml | 8 | 0 | none | none | none | none | none | 25.0% (2/8) | none | none | none | none | none | 25.0% (2/8) | 62.5% (5/8) | 0.109 | none | none | 0 |
| ta | 8 | 0 | none | none | none | none | none | 25.0% (2/8) | none | none | none | none | none | 25.0% (2/8) | 50.0% (4/8) | 0.098 | none | none | 0 |
| te | 8 | 0 | none | none | none | none | none | 62.5% (5/8) | none | none | none | none | none | 62.5% (5/8) | 75.0% (6/8) | 0.408 | none | none | 0 |
| zh | 8 | 0 | none | none | none | none | none | 50.0% (4/8) | none | none | none | none | none | 50.0% (4/8) | 75.0% (6/8) | 0.297 | none | none | 0 |

Writing quality is scored separately against each case's rubric at development time (ADR-0028 decision 9); this table covers only the deterministic measures.
