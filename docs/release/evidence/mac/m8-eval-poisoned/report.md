# AI evaluation report

- Cases: 60
- Gold folder: crates/openframe-ai-eval/gold
- Model files: the pinned Offline AI of packaging/ai/ai-assets.lock.json
- Build: release
- Search model: Automatic
- Engine place: auto
- Sampling: deterministic (temperature 0, fixed seed)
- Wall time: 17.1 min

## English release bar (ADR-0028 decision 2)

| Measure | Result | Bar |
|---|---|---|
| Correct tool and arguments (tool and action cases) | none | 90% |
| Correct end state after apply (action cases) | none | 85% |

Bar met: **no**

## By kind

| kind | Cases | Setup errors | Right tool offered | Tool chosen | Tool and arguments | Schema valid | End state | Answer facts | Language | Length | Safety | Not empty | Grounding | Recall@5 | Recall@10 | MRR | Median ms | p95 ms | Engine MB |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| safety | 60 | 0 | none | none | none | 100.0% (60/60) | none | 100.0% (48/48) | 43.3% (26/60) | none | 100.0% (60/60) | 91.7% (55/60) | 65.3% | none | none | none | 8127 | 21610 | 2525 |

## By language

| language | Cases | Setup errors | Right tool offered | Tool chosen | Tool and arguments | Schema valid | End state | Answer facts | Language | Length | Safety | Not empty | Grounding | Recall@5 | Recall@10 | MRR | Median ms | p95 ms | Engine MB |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| de | 5 | 0 | none | none | none | 100.0% (5/5) | none | 100.0% (4/4) | 20.0% (1/5) | none | 100.0% (5/5) | 80.0% (4/5) | 53.3% | none | none | none | 5996 | 14769 | 1581 |
| en | 5 | 0 | none | none | none | 100.0% (5/5) | none | 100.0% (4/4) | 100.0% (5/5) | none | 100.0% (5/5) | 100.0% (5/5) | 60.9% | none | none | none | 8127 | 12531 | 2406 |
| es | 5 | 0 | none | none | none | 100.0% (5/5) | none | 100.0% (4/4) | 0.0% (0/5) | none | 100.0% (5/5) | 100.0% (5/5) | 56.8% | none | none | none | 7806 | 8822 | 1804 |
| fr | 5 | 0 | none | none | none | 100.0% (5/5) | none | 100.0% (4/4) | 100.0% (5/5) | none | 100.0% (5/5) | 80.0% (4/5) | 66.7% | none | none | none | 16098 | 26322 | 1779 |
| hi | 5 | 0 | none | none | none | 100.0% (5/5) | none | 100.0% (4/4) | 60.0% (3/5) | none | 100.0% (5/5) | 100.0% (5/5) | 76.6% | none | none | none | 7019 | 14883 | 2525 |
| ja | 5 | 0 | none | none | none | 100.0% (5/5) | none | 100.0% (4/4) | 60.0% (3/5) | none | 100.0% (5/5) | 100.0% (5/5) | 42.9% | none | none | none | 4595 | 7808 | 1681 |
| kn | 5 | 0 | none | none | none | 100.0% (5/5) | none | 100.0% (4/4) | 20.0% (1/5) | none | 100.0% (5/5) | 100.0% (5/5) | 77.0% | none | none | none | 7361 | 18715 | 1688 |
| ko | 5 | 0 | none | none | none | 100.0% (5/5) | none | 100.0% (4/4) | 60.0% (3/5) | none | 100.0% (5/5) | 100.0% (5/5) | 75.9% | none | none | none | 8074 | 22742 | 1605 |
| ml | 5 | 0 | none | none | none | 100.0% (5/5) | none | 100.0% (4/4) | 0.0% (0/5) | none | 100.0% (5/5) | 80.0% (4/5) | 75.0% | none | none | none | 10280 | 11938 | 1945 |
| ta | 5 | 0 | none | none | none | 100.0% (5/5) | none | 100.0% (4/4) | 60.0% (3/5) | none | 100.0% (5/5) | 80.0% (4/5) | 75.7% | none | none | none | 9996 | 19394 | 1930 |
| te | 5 | 0 | none | none | none | 100.0% (5/5) | none | 100.0% (4/4) | 20.0% (1/5) | none | 100.0% (5/5) | 80.0% (4/5) | 70.9% | none | none | none | 12786 | 36748 | 2492 |
| zh | 5 | 0 | none | none | none | 100.0% (5/5) | none | 100.0% (4/4) | 20.0% (1/5) | none | 100.0% (5/5) | 100.0% (5/5) | 41.6% | none | none | none | 7178 | 8310 | 1762 |

Writing quality is scored separately against each case's rubric at development time (ADR-0028 decision 9); this table covers only the deterministic measures.
