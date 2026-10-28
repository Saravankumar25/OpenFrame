# AI evaluation report

- Cases: 90
- Gold folder: crates/openframe-ai-eval/gold
- Model files: the pinned Offline AI of packaging/ai/ai-assets.lock.json
- Build: release
- Search model: Automatic
- Routes: on (regex direct routes active)
- Engine place: auto
- Sampling: deterministic (temperature 0, fixed seed)
- Wall time: 25.2 min

## English release bar (ADR-0028 decision 2)

| Measure | Result | Bar |
|---|---|---|
| Correct tool and arguments (tool and action cases) | none | 90% |
| Correct end state after apply (action cases) | none | 85% |

Bar met: **no**

## By kind

| kind | Cases | Setup errors | Right tool offered | Tool chosen | Tool and arguments | Schema valid | End state | Answer facts | Language | Length | Safety | Not empty | Grounding | Recall@5 | Recall@10 | MRR | Median ms | p95 ms | Engine MB |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| paraphrase | 90 | 0 | 95.6% (86/90) | 20.0% (18/90) | 20.0% (18/90) | 100.0% (90/90) | none | 37.8% (17/45) | 96.7% (87/90) | none | none | 94.4% (85/90) | 72.9% | none | none | none | 11192 | 24175 | 3688 |

## By language

| language | Cases | Setup errors | Right tool offered | Tool chosen | Tool and arguments | Schema valid | End state | Answer facts | Language | Length | Safety | Not empty | Grounding | Recall@5 | Recall@10 | MRR | Median ms | p95 ms | Engine MB |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| de | 2 | 0 | 100.0% (2/2) | 0.0% (0/2) | 0.0% (0/2) | 100.0% (2/2) | none | 0.0% (0/1) | 50.0% (1/2) | none | none | 100.0% (2/2) | 51.2% | none | none | none | 14704 | 14704 | 2516 |
| en | 80 | 0 | 95.0% (76/80) | 21.2% (17/80) | 21.2% (17/80) | 100.0% (80/80) | none | 37.5% (15/40) | 100.0% (80/80) | none | none | 95.0% (76/80) | 71.6% | none | none | none | 11192 | 24175 | 3688 |
| es | 2 | 0 | 100.0% (2/2) | 0.0% (0/2) | 0.0% (0/2) | 100.0% (2/2) | none | 0.0% (0/1) | 50.0% (1/2) | none | none | 100.0% (2/2) | 78.6% | none | none | none | 11740 | 11740 | 2684 |
| fr | 2 | 0 | 100.0% (2/2) | 0.0% (0/2) | 0.0% (0/2) | 100.0% (2/2) | none | 100.0% (1/1) | 50.0% (1/2) | none | none | 50.0% (1/2) | 84.7% | none | none | none | 13880 | 13880 | 2681 |
| hi | 2 | 0 | 100.0% (2/2) | 50.0% (1/2) | 50.0% (1/2) | 100.0% (2/2) | none | 100.0% (1/1) | 100.0% (2/2) | none | none | 100.0% (2/2) | 72.7% | none | none | none | 12744 | 12744 | 2541 |
| ja | 2 | 0 | 100.0% (2/2) | 0.0% (0/2) | 0.0% (0/2) | 100.0% (2/2) | none | 0.0% (0/1) | 100.0% (2/2) | none | none | 100.0% (2/2) | none | none | none | none | 8130 | 8130 | 2549 |

## Held-out paraphrases

Scored like tool cases and not part of the English bar. This is the honest estimate of accuracy on real wording.

| Language | Cases | Setup errors | Right tool offered | Tool chosen | Tool and arguments | Schema valid | Answer facts | Median ms |
|---|---|---|---|---|---|---|---|---|
| all | 90 | 0 | 95.6% (86/90) | 20.0% (18/90) | 20.0% (18/90) | 100.0% (90/90) | 37.8% (17/45) | 11192 |
| de | 2 | 0 | 100.0% (2/2) | 0.0% (0/2) | 0.0% (0/2) | 100.0% (2/2) | 0.0% (0/1) | 14704 |
| en | 80 | 0 | 95.0% (76/80) | 21.2% (17/80) | 21.2% (17/80) | 100.0% (80/80) | 37.5% (15/40) | 11192 |
| es | 2 | 0 | 100.0% (2/2) | 0.0% (0/2) | 0.0% (0/2) | 100.0% (2/2) | 0.0% (0/1) | 11740 |
| fr | 2 | 0 | 100.0% (2/2) | 0.0% (0/2) | 0.0% (0/2) | 100.0% (2/2) | 100.0% (1/1) | 13880 |
| hi | 2 | 0 | 100.0% (2/2) | 50.0% (1/2) | 50.0% (1/2) | 100.0% (2/2) | 100.0% (1/1) | 12744 |
| ja | 2 | 0 | 100.0% (2/2) | 0.0% (0/2) | 0.0% (0/2) | 100.0% (2/2) | 0.0% (0/1) | 8130 |

Writing quality is scored separately against each case's rubric at development time (ADR-0028 decision 9); this table covers only the deterministic measures.
