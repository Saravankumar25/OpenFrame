# AI evaluation report

- Cases: 420
- Gold folder: crates/openframe-ai-eval/gold
- Model files: the pinned Offline AI of packaging/ai/ai-assets.lock.json
- Build: release
- Search model: Automatic
- Engine place: auto
- Sampling: deterministic (temperature 0, fixed seed)
- Wall time: 34.3 min

## English release bar (ADR-0028 decision 2)

| Measure | Result | Bar |
|---|---|---|
| Correct tool and arguments (tool and action cases) | 96.7% | 90% |
| Correct end state after apply (action cases) | 88.6% | 85% |

Bar met: **yes**

## By kind

| kind | Cases | Setup errors | Right tool offered | Tool chosen | Tool and arguments | Schema valid | End state | Answer facts | Language | Length | Safety | Not empty | Grounding | Recall@5 | Recall@10 | MRR | Median ms | p95 ms | Engine MB |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| action | 79 | 0 | 82.3% (65/79) | 94.9% (75/79) | 93.7% (74/79) | 100.0% (79/79) | 88.6% (70/79) | none | 100.0% (79/79) | none | none | 98.7% (78/79) | 77.7% | none | none | none | 31 | 10083 | 1988 |
| retrieval | 164 | 0 | none | none | none | none | none | 99.4% (163/164) | none | none | none | none | none | 99.4% (163/164) | 100.0% (164/164) | 0.933 | none | none | 0 |
| safety | 46 | 0 | 100.0% (2/2) | 0.0% (0/2) | none | 100.0% (46/46) | 100.0% (1/1) | 100.0% (7/7) | 100.0% (46/46) | none | 97.8% (45/46) | 91.3% (42/46) | 64.4% | none | none | none | 8304 | 18481 | 1817 |
| tool | 71 | 0 | 100.0% (71/71) | 100.0% (71/71) | 100.0% (71/71) | 100.0% (71/71) | none | 100.0% (18/18) | 100.0% (71/71) | none | none | 93.0% (66/71) | 77.3% | none | none | none | 33 | 10078 | 1628 |
| writing | 60 | 0 | 100.0% (60/60) | 45.0% (27/60) | none | 100.0% (60/60) | none | none | 100.0% (60/60) | 31.7% (19/60) | none | 93.3% (56/60) | 71.9% | none | none | none | 7640 | 14204 | 1896 |

## By language

| language | Cases | Setup errors | Right tool offered | Tool chosen | Tool and arguments | Schema valid | End state | Answer facts | Language | Length | Safety | Not empty | Grounding | Recall@5 | Recall@10 | MRR | Median ms | p95 ms | Engine MB |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| en | 420 | 0 | 93.4% (198/212) | 81.6% (173/212) | 96.7% (145/150) | 100.0% (256/256) | 88.8% (71/80) | 99.5% (188/189) | 100.0% (256/256) | 31.7% (19/60) | 97.8% (45/46) | 94.5% (242/256) | 73.0% | 99.4% (163/164) | 100.0% (164/164) | 0.933 | 3695 | 13189 | 1988 |

Writing quality is scored separately against each case's rubric at development time (ADR-0028 decision 9); this table covers only the deterministic measures.
