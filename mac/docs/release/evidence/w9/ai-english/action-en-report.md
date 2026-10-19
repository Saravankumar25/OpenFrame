# AI evaluation report

- Cases: 79
- Gold folder: crates/openframe-ai-eval/gold
- Model files: the pinned Offline AI of packaging/ai/ai-assets.lock.json
- Build: release
- Search model: Automatic
- Engine place: off (processor)
- Sampling: deterministic (temperature 0, fixed seed)
- Wall time: 18.5 min

## English release bar (ADR-0028 decision 2)

| Measure | Result | Bar |
|---|---|---|
| Correct tool and arguments (tool and action cases) | 93.7% | 90% |
| Correct end state after apply (action cases) | 88.6% | 85% |

Bar met: **yes**

## By kind

| kind | Cases | Setup errors | Right tool offered | Tool chosen | Tool and arguments | Schema valid | End state | Answer facts | Language | Length | Safety | Not empty | Grounding | Recall@5 | Recall@10 | MRR | Median ms | p95 ms | Engine MB |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| action | 79 | 0 | 82.3% (65/79) | 94.9% (75/79) | 93.7% (74/79) | 100.0% (79/79) | 88.6% (70/79) | none | 100.0% (79/79) | none | none | 100.0% (79/79) | 70.9% | none | none | none | 108 | 42102 | 3026 |

## By language

| language | Cases | Setup errors | Right tool offered | Tool chosen | Tool and arguments | Schema valid | End state | Answer facts | Language | Length | Safety | Not empty | Grounding | Recall@5 | Recall@10 | MRR | Median ms | p95 ms | Engine MB |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| en | 79 | 0 | 82.3% (65/79) | 94.9% (75/79) | 93.7% (74/79) | 100.0% (79/79) | 88.6% (70/79) | none | 100.0% (79/79) | none | none | 100.0% (79/79) | 70.9% | none | none | none | 108 | 42102 | 3026 |

Writing quality is scored separately against each case's rubric at development time (ADR-0028 decision 9); this table covers only the deterministic measures.
