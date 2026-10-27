# AI evaluation report

- Cases: 430
- Gold folder: .build-cache/baseline/gold
- Model files: the pinned Offline AI of packaging/ai/ai-assets.lock.json
- Build: release
- Search model: Automatic
- Engine place: auto
- Sampling: deterministic (temperature 0, fixed seed)
- Wall time: 67.8 min

## English release bar (ADR-0028 decision 2)

| Measure | Result | Bar |
|---|---|---|
| Correct tool and arguments (tool and action cases) | 98.1% | 90% |
| Correct end state after apply (action cases) | 89.9% | 85% |

Bar met: **yes**

## By kind

| kind | Cases | Setup errors | Right tool offered | Tool chosen | Tool and arguments | Schema valid | End state | Answer facts | Language | Length | Safety | Not empty | Grounding | Recall@5 | Recall@10 | MRR | Median ms | p95 ms | Engine MB |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| action | 79 | 0 | 82.3% (65/79) | 96.2% (76/79) | 96.2% (76/79) | 100.0% (79/79) | 89.9% (71/79) | none | 100.0% (79/79) | none | none | 100.0% (79/79) | 73.6% | none | none | none | 71 | 51188 | 1569 |
| retrieval | 164 | 0 | none | none | none | none | none | 99.4% (163/164) | none | none | none | none | none | 99.4% (163/164) | 100.0% (164/164) | 0.933 | none | none | 0 |
| safety | 46 | 0 | 100.0% (2/2) | 0.0% (0/2) | none | 100.0% (46/46) | 100.0% (1/1) | 100.0% (7/7) | 100.0% (46/46) | none | 100.0% (46/46) | 97.8% (45/46) | 68.2% | none | none | none | 11391 | 29142 | 2197 |
| tool | 81 | 0 | 97.5% (79/81) | 100.0% (81/81) | 100.0% (81/81) | 100.0% (81/81) | none | 96.3% (26/27) | 100.0% (81/81) | none | none | 93.8% (76/81) | 77.2% | none | none | none | 35 | 12372 | 1860 |
| writing | 60 | 0 | 100.0% (60/60) | 46.7% (28/60) | none | 100.0% (60/60) | none | none | 100.0% (60/60) | 98.3% (59/60) | none | 100.0% (60/60) | 68.6% | none | none | none | 1693 | 13901 | 1075 |

## By language

| language | Cases | Setup errors | Right tool offered | Tool chosen | Tool and arguments | Schema valid | End state | Answer facts | Language | Length | Safety | Not empty | Grounding | Recall@5 | Recall@10 | MRR | Median ms | p95 ms | Engine MB |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| en | 430 | 0 | 92.8% (206/222) | 83.3% (185/222) | 98.1% (157/160) | 100.0% (266/266) | 90.0% (72/80) | 99.0% (196/198) | 100.0% (266/266) | 98.3% (59/60) | 100.0% (46/46) | 97.7% (260/266) | 74.1% | 99.4% (163/164) | 100.0% (164/164) | 0.933 | 771 | 26420 | 2197 |

Writing quality is scored separately against each case's rubric at development time (ADR-0028 decision 9); this table covers only the deterministic measures.
