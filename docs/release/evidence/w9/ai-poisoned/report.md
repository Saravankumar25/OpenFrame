# AI evaluation report

- Cases: 60
- Gold folder: crates/openframe-ai-eval/gold
- Model files: the pinned Offline AI of packaging/ai/ai-assets.lock.json
- Build: release
- Search model: Automatic
- Engine place: off (processor)
- Sampling: deterministic (temperature 0, fixed seed)
- Wall time: 38.5 min

## English release bar (ADR-0028 decision 2)

| Measure | Result | Bar |
|---|---|---|
| Correct tool and arguments (tool and action cases) | none | 90% |
| Correct end state after apply (action cases) | none | 85% |

Bar met: **no**

## By kind

| kind | Cases | Setup errors | Right tool offered | Tool chosen | Tool and arguments | Schema valid | End state | Answer facts | Language | Length | Safety | Not empty | Grounding | Recall@5 | Recall@10 | MRR | Median ms | p95 ms | Engine MB |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| safety | 60 | 0 | none | none | none | 100.0% (60/60) | none | 100.0% (48/48) | 50.0% (30/60) | none | 100.0% (60/60) | 95.0% (57/60) | 63.2% | none | none | none | 34605 | 47865 | 3173 |

## By language

| language | Cases | Setup errors | Right tool offered | Tool chosen | Tool and arguments | Schema valid | End state | Answer facts | Language | Length | Safety | Not empty | Grounding | Recall@5 | Recall@10 | MRR | Median ms | p95 ms | Engine MB |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| de | 5 | 0 | none | none | none | 100.0% (5/5) | none | 100.0% (4/4) | 20.0% (1/5) | none | 100.0% (5/5) | 100.0% (5/5) | 57.8% | none | none | none | 30434 | 33715 | 2988 |
| en | 5 | 0 | none | none | none | 100.0% (5/5) | none | 100.0% (4/4) | 100.0% (5/5) | none | 100.0% (5/5) | 100.0% (5/5) | 67.6% | none | none | none | 39642 | 42814 | 1995 |
| es | 5 | 0 | none | none | none | 100.0% (5/5) | none | 100.0% (4/4) | 0.0% (0/5) | none | 100.0% (5/5) | 100.0% (5/5) | 49.4% | none | none | none | 34605 | 36302 | 2952 |
| fr | 5 | 0 | none | none | none | 100.0% (5/5) | none | 100.0% (4/4) | 100.0% (5/5) | none | 100.0% (5/5) | 100.0% (5/5) | 59.7% | none | none | none | 40498 | 43225 | 3027 |
| hi | 5 | 0 | none | none | none | 100.0% (5/5) | none | 100.0% (4/4) | 60.0% (3/5) | none | 100.0% (5/5) | 100.0% (5/5) | 76.6% | none | none | none | 35535 | 43673 | 2235 |
| ja | 5 | 0 | none | none | none | 100.0% (5/5) | none | 100.0% (4/4) | 80.0% (4/5) | none | 100.0% (5/5) | 100.0% (5/5) | 41.4% | none | none | none | 12643 | 21267 | 3173 |
| kn | 5 | 0 | none | none | none | 100.0% (5/5) | none | 100.0% (4/4) | 0.0% (0/5) | none | 100.0% (5/5) | 100.0% (5/5) | 70.5% | none | none | none | 39547 | 51492 | 2663 |
| ko | 5 | 0 | none | none | none | 100.0% (5/5) | none | 100.0% (4/4) | 80.0% (4/5) | none | 100.0% (5/5) | 100.0% (5/5) | 49.0% | none | none | none | 30102 | 37299 | 3159 |
| ml | 5 | 0 | none | none | none | 100.0% (5/5) | none | 100.0% (4/4) | 0.0% (0/5) | none | 100.0% (5/5) | 80.0% (4/5) | 74.6% | none | none | none | 39274 | 40864 | 2870 |
| ta | 5 | 0 | none | none | none | 100.0% (5/5) | none | 100.0% (4/4) | 100.0% (5/5) | none | 100.0% (5/5) | 80.0% (4/5) | 74.2% | none | none | none | 38534 | 61999 | 2641 |
| te | 5 | 0 | none | none | none | 100.0% (5/5) | none | 100.0% (4/4) | 40.0% (2/5) | none | 100.0% (5/5) | 80.0% (4/5) | 69.8% | none | none | none | 37226 | 54434 | 2334 |
| zh | 5 | 0 | none | none | none | 100.0% (5/5) | none | 100.0% (4/4) | 20.0% (1/5) | none | 100.0% (5/5) | 100.0% (5/5) | 40.7% | none | none | none | 20070 | 21394 | 3162 |

Writing quality is scored separately against each case's rubric at development time (ADR-0028 decision 9); this table covers only the deterministic measures.
