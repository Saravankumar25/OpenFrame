# AI evaluation report

- Cases: 60
- Gold folder: crates/openframe-ai-eval/gold
- Model files: the pinned Offline AI of packaging/ai/ai-assets.lock.json
- Build: release
- Search model: Automatic
- Routes: on (regex direct routes active)
- Engine place: auto
- Sampling: deterministic (temperature 0, fixed seed)
- Wall time: 41.5 min

## English release bar (ADR-0028 decision 2)

| Measure | Result | Bar |
|---|---|---|
| Correct tool and arguments (tool and action cases) | none | 90% |
| Correct end state after apply (action cases) | none | 85% |

Bar met: **no**

## By kind

| kind | Cases | Setup errors | Right tool offered | Tool chosen | Tool and arguments | Schema valid | End state | Answer facts | Language | Length | Safety | Not empty | Grounding | Recall@5 | Recall@10 | MRR | Median ms | p95 ms | Engine MB |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| conversation | 60 | 0 | none | none | none | 100.0% (60/60) | none | 42.9% (24/56) | 100.0% (60/60) | 55.0% (33/60) | none | 95.0% (57/60) | 79.5% | none | none | none | 38984 | 59634 | 4111 |

## By language

| language | Cases | Setup errors | Right tool offered | Tool chosen | Tool and arguments | Schema valid | End state | Answer facts | Language | Length | Safety | Not empty | Grounding | Recall@5 | Recall@10 | MRR | Median ms | p95 ms | Engine MB |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| en | 60 | 0 | none | none | none | 100.0% (60/60) | none | 42.9% (24/56) | 100.0% (60/60) | 55.0% (33/60) | none | 95.0% (57/60) | 79.5% | none | none | none | 38984 | 59634 | 4111 |

## Conversation bench

Deterministic checks only; the rubric and human labels come at development time. Invented names, wrong scenes and missing quotes are rates over all names, scene citations and quotes in the answers.

| Question type | Cases | Setup errors | Answer facts | Invented names | Wrong scene citations | Missing quotes | Within length | Correct abstention | Repetition | Median ms |
|---|---|---|---|---|---|---|---|---|---|---|
| all | 60 | 0 | 42.9% (24/56) | 13.1% (32/245) | 0.0% (0/117) | 80.0% (8/10) | 55.0% (33/60) | 0.0% (0/4) | 8.3% (5/60) | 38984 |
| abstain | 4 | 0 | none | 37.5% (3/8) | none | none | 100.0% (4/4) | 0.0% (0/4) | 0.0% (0/4) | 33851 |
| character | 10 | 0 | 90.0% (9/10) | 12.2% (5/41) | 0.0% (0/17) | 0.0% (0/1) | 90.0% (9/10) | none | 0.0% (0/10) | 38616 |
| compare | 5 | 0 | 20.0% (1/5) | 7.5% (3/40) | 0.0% (0/41) | none | 20.0% (1/5) | none | 20.0% (1/5) | 44005 |
| critique | 4 | 0 | 50.0% (2/4) | 15.0% (3/20) | 0.0% (0/10) | 100.0% (1/1) | 25.0% (1/4) | none | 25.0% (1/4) | 51496 |
| explain | 10 | 0 | 10.0% (1/10) | 17.1% (6/35) | 0.0% (0/14) | 100.0% (1/1) | 40.0% (4/10) | none | 0.0% (0/10) | 4738 |
| followup | 4 | 0 | 25.0% (1/4) | 0.0% (0/18) | 0.0% (0/9) | 100.0% (1/1) | 50.0% (2/4) | none | 0.0% (0/4) | 30091 |
| ideas | 5 | 0 | 80.0% (4/5) | 13.3% (6/45) | 0.0% (0/24) | 100.0% (5/5) | 60.0% (3/5) | none | 40.0% (2/5) | 47164 |
| relationship | 8 | 0 | 25.0% (2/8) | 0.0% (0/16) | none | 0.0% (0/1) | 50.0% (4/8) | none | 0.0% (0/8) | 45618 |
| theme | 5 | 0 | 40.0% (2/5) | 33.3% (4/12) | 0.0% (0/2) | none | 40.0% (2/5) | none | 20.0% (1/5) | 34706 |
| whole | 5 | 0 | 40.0% (2/5) | 20.0% (2/10) | none | none | 60.0% (3/5) | none | 0.0% (0/5) | 49397 |

Writing quality is scored separately against each case's rubric at development time (ADR-0028 decision 9); this table covers only the deterministic measures.
