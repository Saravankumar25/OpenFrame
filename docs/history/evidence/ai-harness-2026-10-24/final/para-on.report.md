# AI evaluation report

- Cases: 90
- Gold folder: crates/openframe-ai-eval/gold
- Model files: the pinned Offline AI of packaging/ai/ai-assets.lock.json
- Build: release
- Search model: Automatic
- Routes: on (regex direct routes active)
- Engine place: auto
- Sampling: deterministic (temperature 0, fixed seed)
- Wall time: 18.1 min

## English release bar (ADR-0028 decision 2)

| Measure | Result | Bar |
|---|---|---|
| Correct tool and arguments (tool and action cases) | none | 90% |
| Correct end state after apply (action cases) | none | 85% |

Bar met: **no**

## By kind

| kind | Cases | Setup errors | Right tool offered | Tool chosen | Tool and arguments | Schema valid | End state | Answer facts | Language | Length | Safety | Not empty | Grounding | Recall@5 | Recall@10 | MRR | Median ms | p95 ms | Engine MB |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| paraphrase | 90 | 0 | 95.6% (86/90) | 46.7% (42/90) | 44.4% (40/90) | 100.0% (90/90) | none | 51.1% (23/45) | 93.3% (84/90) | none | none | 96.7% (87/90) | 77.7% | none | none | none | 7077 | 16333 | 4242 |

## By language

| language | Cases | Setup errors | Right tool offered | Tool chosen | Tool and arguments | Schema valid | End state | Answer facts | Language | Length | Safety | Not empty | Grounding | Recall@5 | Recall@10 | MRR | Median ms | p95 ms | Engine MB |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| de | 2 | 0 | 100.0% (2/2) | 0.0% (0/2) | 0.0% (0/2) | 100.0% (2/2) | none | 0.0% (0/1) | 50.0% (1/2) | none | none | 100.0% (2/2) | 100.0% | none | none | none | 6780 | 6780 | 1030 |
| en | 80 | 0 | 95.0% (76/80) | 46.2% (37/80) | 43.8% (35/80) | 100.0% (80/80) | none | 47.5% (19/40) | 100.0% (80/80) | none | none | 96.2% (77/80) | 78.5% | none | none | none | 7661 | 15193 | 4242 |
| es | 2 | 0 | 100.0% (2/2) | 50.0% (1/2) | 50.0% (1/2) | 100.0% (2/2) | none | 100.0% (1/1) | 0.0% (0/2) | none | none | 100.0% (2/2) | 0.0% | none | none | none | 9203 | 9203 | 2143 |
| fr | 2 | 0 | 100.0% (2/2) | 50.0% (1/2) | 50.0% (1/2) | 100.0% (2/2) | none | 100.0% (1/1) | 0.0% (0/2) | none | none | 100.0% (2/2) | 61.3% | none | none | none | 43345 | 43345 | 2168 |
| hi | 2 | 0 | 100.0% (2/2) | 100.0% (2/2) | 100.0% (2/2) | 100.0% (2/2) | none | 100.0% (1/1) | 100.0% (2/2) | none | none | 100.0% (2/2) | 33.3% | none | none | none | 9680 | 9680 | 2094 |
| ja | 2 | 0 | 100.0% (2/2) | 50.0% (1/2) | 50.0% (1/2) | 100.0% (2/2) | none | 100.0% (1/1) | 50.0% (1/2) | none | none | 100.0% (2/2) | 100.0% | none | none | none | 6427 | 6427 | 1035 |

## Lanes and intents

Cases by kind and the lane they took.

| Kind | direct | chat | recipe | agent | action | Cases |
|---|---|---|---|---|---|---|
| paraphrase | 29 | 7 | 1 | 52 | 1 | 90 |
| all | 29 | 7 | 1 | 52 | 1 | 90 |

Cases that had a closest intent, by its score. Matched: the intent's tool is a tool the case expects. Unjudged: the case expects no tool.

| Score | Matched | Missed | Unjudged | Matched of judged |
|---|---|---|---|---|
| 0.82 to 0.84 | 2 | 7 | 0 | 22.2% (2/9) |
| 0.84 to 0.86 | 3 | 3 | 0 | 50.0% (3/6) |
| 0.86 to 0.88 | 7 | 8 | 0 | 46.7% (7/15) |
| 0.88 to 0.90 | 5 | 3 | 0 | 62.5% (5/8) |
| 0.90 to 0.92 | 14 | 1 | 0 | 93.3% (14/15) |
| 0.92 to 0.94 | 7 | 1 | 0 | 87.5% (7/8) |
| 0.94 to 0.96 | 7 | 0 | 0 | 100.0% (7/7) |
| 0.96 to 0.98 | 2 | 0 | 0 | 100.0% (2/2) |
| 0.98 to 1.00 | 1 | 0 | 0 | 100.0% (1/1) |

## Held-out paraphrases

Scored like tool cases and not part of the English bar. This is the honest estimate of accuracy on real wording.

| Language | Cases | Setup errors | Right tool offered | Tool chosen | Tool and arguments | Schema valid | Answer facts | Median ms |
|---|---|---|---|---|---|---|---|---|
| all | 90 | 0 | 95.6% (86/90) | 46.7% (42/90) | 44.4% (40/90) | 100.0% (90/90) | 51.1% (23/45) | 7077 |
| de | 2 | 0 | 100.0% (2/2) | 0.0% (0/2) | 0.0% (0/2) | 100.0% (2/2) | 0.0% (0/1) | 6780 |
| en | 80 | 0 | 95.0% (76/80) | 46.2% (37/80) | 43.8% (35/80) | 100.0% (80/80) | 47.5% (19/40) | 7661 |
| es | 2 | 0 | 100.0% (2/2) | 50.0% (1/2) | 50.0% (1/2) | 100.0% (2/2) | 100.0% (1/1) | 9203 |
| fr | 2 | 0 | 100.0% (2/2) | 50.0% (1/2) | 50.0% (1/2) | 100.0% (2/2) | 100.0% (1/1) | 43345 |
| hi | 2 | 0 | 100.0% (2/2) | 100.0% (2/2) | 100.0% (2/2) | 100.0% (2/2) | 100.0% (1/1) | 9680 |
| ja | 2 | 0 | 100.0% (2/2) | 50.0% (1/2) | 50.0% (1/2) | 100.0% (2/2) | 100.0% (1/1) | 6427 |

Writing quality is scored separately against each case's rubric at development time (ADR-0028 decision 9); this table covers only the deterministic measures.
