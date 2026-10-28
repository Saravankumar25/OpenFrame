# AI evaluation report

- Cases: 90
- Gold folder: crates/openframe-ai-eval/gold
- Model files: the pinned Offline AI of packaging/ai/ai-assets.lock.json
- Build: release
- Search model: Automatic
- Routes: off (model path only)
- Engine place: auto
- Sampling: deterministic (temperature 0, fixed seed)
- Wall time: 30.2 min

## English release bar (ADR-0028 decision 2)

| Measure | Result | Bar |
|---|---|---|
| Correct tool and arguments (tool and action cases) | none | 90% |
| Correct end state after apply (action cases) | none | 85% |

Bar met: **no**

## By kind

| kind | Cases | Setup errors | Right tool offered | Tool chosen | Tool and arguments | Schema valid | End state | Answer facts | Language | Length | Safety | Not empty | Grounding | Recall@5 | Recall@10 | MRR | Median ms | p95 ms | Engine MB |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| paraphrase | 90 | 0 | 95.6% (86/90) | 13.3% (12/90) | 13.3% (12/90) | 100.0% (90/90) | none | 26.7% (12/45) | 96.7% (87/90) | none | none | 93.3% (84/90) | 73.0% | none | none | none | 12112 | 24886 | 3098 |

## By language

| language | Cases | Setup errors | Right tool offered | Tool chosen | Tool and arguments | Schema valid | End state | Answer facts | Language | Length | Safety | Not empty | Grounding | Recall@5 | Recall@10 | MRR | Median ms | p95 ms | Engine MB |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| de | 2 | 0 | 100.0% (2/2) | 0.0% (0/2) | 0.0% (0/2) | 100.0% (2/2) | none | 0.0% (0/1) | 50.0% (1/2) | none | none | 100.0% (2/2) | 51.2% | none | none | none | 14606 | 14606 | 1609 |
| en | 80 | 0 | 95.0% (76/80) | 15.0% (12/80) | 15.0% (12/80) | 100.0% (80/80) | none | 27.5% (11/40) | 100.0% (80/80) | none | none | 93.8% (75/80) | 71.6% | none | none | none | 12112 | 24886 | 3098 |
| es | 2 | 0 | 100.0% (2/2) | 0.0% (0/2) | 0.0% (0/2) | 100.0% (2/2) | none | 0.0% (0/1) | 50.0% (1/2) | none | none | 100.0% (2/2) | 78.6% | none | none | none | 11432 | 11432 | 2821 |
| fr | 2 | 0 | 100.0% (2/2) | 0.0% (0/2) | 0.0% (0/2) | 100.0% (2/2) | none | 100.0% (1/1) | 50.0% (1/2) | none | none | 50.0% (1/2) | 84.7% | none | none | none | 14655 | 14655 | 1543 |
| hi | 2 | 0 | 100.0% (2/2) | 0.0% (0/2) | 0.0% (0/2) | 100.0% (2/2) | none | 0.0% (0/1) | 100.0% (2/2) | none | none | 100.0% (2/2) | 81.8% | none | none | none | 12836 | 12836 | 2875 |
| ja | 2 | 0 | 100.0% (2/2) | 0.0% (0/2) | 0.0% (0/2) | 100.0% (2/2) | none | 0.0% (0/1) | 100.0% (2/2) | none | none | 100.0% (2/2) | none | none | none | none | 9591 | 9591 | 1577 |

## Held-out paraphrases

Scored like tool cases and not part of the English bar. This is the honest estimate of accuracy on real wording.

| Language | Cases | Setup errors | Right tool offered | Tool chosen | Tool and arguments | Schema valid | Answer facts | Median ms |
|---|---|---|---|---|---|---|---|---|
| all | 90 | 0 | 95.6% (86/90) | 13.3% (12/90) | 13.3% (12/90) | 100.0% (90/90) | 26.7% (12/45) | 12112 |
| de | 2 | 0 | 100.0% (2/2) | 0.0% (0/2) | 0.0% (0/2) | 100.0% (2/2) | 0.0% (0/1) | 14606 |
| en | 80 | 0 | 95.0% (76/80) | 15.0% (12/80) | 15.0% (12/80) | 100.0% (80/80) | 27.5% (11/40) | 12112 |
| es | 2 | 0 | 100.0% (2/2) | 0.0% (0/2) | 0.0% (0/2) | 100.0% (2/2) | 0.0% (0/1) | 11432 |
| fr | 2 | 0 | 100.0% (2/2) | 0.0% (0/2) | 0.0% (0/2) | 100.0% (2/2) | 100.0% (1/1) | 14655 |
| hi | 2 | 0 | 100.0% (2/2) | 0.0% (0/2) | 0.0% (0/2) | 100.0% (2/2) | 0.0% (0/1) | 12836 |
| ja | 2 | 0 | 100.0% (2/2) | 0.0% (0/2) | 0.0% (0/2) | 100.0% (2/2) | 0.0% (0/1) | 9591 |

Writing quality is scored separately against each case's rubric at development time (ADR-0028 decision 9); this table covers only the deterministic measures.
