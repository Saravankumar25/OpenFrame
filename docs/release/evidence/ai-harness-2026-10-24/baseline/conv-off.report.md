# AI evaluation report

- Cases: 60
- Gold folder: crates/openframe-ai-eval/gold
- Model files: the pinned Offline AI of packaging/ai/ai-assets.lock.json
- Build: release
- Search model: Automatic
- Routes: off (model path only)
- Engine place: auto
- Sampling: deterministic (temperature 0, fixed seed)
- Wall time: 21.4 min

## English release bar (ADR-0028 decision 2)

| Measure | Result | Bar |
|---|---|---|
| Correct tool and arguments (tool and action cases) | none | 90% |
| Correct end state after apply (action cases) | none | 85% |

Bar met: **no**

## By kind

| kind | Cases | Setup errors | Right tool offered | Tool chosen | Tool and arguments | Schema valid | End state | Answer facts | Language | Length | Safety | Not empty | Grounding | Recall@5 | Recall@10 | MRR | Median ms | p95 ms | Engine MB |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| conversation | 60 | 0 | none | none | none | 100.0% (60/60) | none | 42.9% (24/56) | 100.0% (60/60) | 66.7% (40/60) | none | 90.0% (54/60) | 78.2% | none | none | none | 13683 | 26119 | 3490 |

## By language

| language | Cases | Setup errors | Right tool offered | Tool chosen | Tool and arguments | Schema valid | End state | Answer facts | Language | Length | Safety | Not empty | Grounding | Recall@5 | Recall@10 | MRR | Median ms | p95 ms | Engine MB |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| en | 60 | 0 | none | none | none | 100.0% (60/60) | none | 42.9% (24/56) | 100.0% (60/60) | 66.7% (40/60) | none | 90.0% (54/60) | 78.2% | none | none | none | 13683 | 26119 | 3490 |

## Conversation bench

Deterministic checks only; the rubric and human labels come at development time. Invented names, wrong scenes and missing quotes are rates over all names, scene citations and quotes in the answers.

| Question type | Cases | Setup errors | Answer facts | Invented names | Wrong scene citations | Missing quotes | Within length | Correct abstention | Repetition | Median ms |
|---|---|---|---|---|---|---|---|---|---|---|
| all | 60 | 0 | 42.9% (24/56) | 13.4% (33/247) | 0.0% (0/128) | 77.8% (7/9) | 66.7% (40/60) | 0.0% (0/4) | 13.3% (8/60) | 13683 |
| abstain | 4 | 0 | none | 25.0% (2/8) | none | none | 100.0% (4/4) | 0.0% (0/4) | 0.0% (0/4) | 12662 |
| character | 10 | 0 | 90.0% (9/10) | 0.0% (0/38) | 0.0% (0/19) | 0.0% (0/1) | 90.0% (9/10) | none | 0.0% (0/10) | 13522 |
| compare | 5 | 0 | 40.0% (2/5) | 7.7% (3/39) | 0.0% (0/41) | none | 20.0% (1/5) | none | 20.0% (1/5) | 16113 |
| critique | 4 | 0 | 50.0% (2/4) | 14.3% (3/21) | 0.0% (0/11) | none | 50.0% (2/4) | none | 25.0% (1/4) | 17670 |
| explain | 10 | 0 | 0.0% (0/10) | 15.9% (7/44) | 0.0% (0/24) | 50.0% (1/2) | 80.0% (8/10) | none | 20.0% (2/10) | 15471 |
| followup | 4 | 0 | 25.0% (1/4) | 0.0% (0/6) | 0.0% (0/1) | none | 100.0% (4/4) | none | 0.0% (0/4) | 11852 |
| ideas | 5 | 0 | 80.0% (4/5) | 19.5% (8/41) | 0.0% (0/21) | 100.0% (5/5) | 80.0% (4/5) | none | 40.0% (2/5) | 18230 |
| relationship | 8 | 0 | 25.0% (2/8) | 0.0% (0/20) | 0.0% (0/8) | 100.0% (1/1) | 62.5% (5/8) | none | 12.5% (1/8) | 13227 |
| theme | 5 | 0 | 40.0% (2/5) | 33.3% (5/15) | 0.0% (0/2) | none | 40.0% (2/5) | none | 20.0% (1/5) | 14430 |
| whole | 5 | 0 | 40.0% (2/5) | 33.3% (5/15) | 0.0% (0/1) | none | 20.0% (1/5) | none | 0.0% (0/5) | 12800 |

Writing quality is scored separately against each case's rubric at development time (ADR-0028 decision 9); this table covers only the deterministic measures.
