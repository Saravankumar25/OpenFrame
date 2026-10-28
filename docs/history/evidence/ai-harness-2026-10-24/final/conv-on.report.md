# AI evaluation report

- Cases: 60
- Gold folder: crates/openframe-ai-eval/gold
- Model files: the pinned Offline AI of packaging/ai/ai-assets.lock.json
- Build: release
- Search model: Automatic
- Routes: on (regex direct routes active)
- Engine place: auto
- Sampling: deterministic (temperature 0, fixed seed)
- Wall time: 35.8 min

## English release bar (ADR-0028 decision 2)

| Measure | Result | Bar |
|---|---|---|
| Correct tool and arguments (tool and action cases) | none | 90% |
| Correct end state after apply (action cases) | none | 85% |

Bar met: **no**

## By kind

| kind | Cases | Setup errors | Right tool offered | Tool chosen | Tool and arguments | Schema valid | End state | Answer facts | Language | Length | Safety | Not empty | Grounding | Recall@5 | Recall@10 | MRR | Median ms | p95 ms | Engine MB |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| conversation | 60 | 0 | none | none | none | 100.0% (60/60) | none | 46.4% (26/56) | 100.0% (60/60) | 71.7% (43/60) | none | 100.0% (60/60) | 85.0% | none | none | none | 32256 | 64080 | 2653 |

## By language

| language | Cases | Setup errors | Right tool offered | Tool chosen | Tool and arguments | Schema valid | End state | Answer facts | Language | Length | Safety | Not empty | Grounding | Recall@5 | Recall@10 | MRR | Median ms | p95 ms | Engine MB |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| en | 60 | 0 | none | none | none | 100.0% (60/60) | none | 46.4% (26/56) | 100.0% (60/60) | 71.7% (43/60) | none | 100.0% (60/60) | 85.0% | none | none | none | 32256 | 64080 | 2653 |

## Lanes and intents

Cases by kind and the lane they took.

| Kind | direct | chat | recipe | agent | action | Cases |
|---|---|---|---|---|---|---|
| conversation | 7 | 43 | 3 | 7 | 0 | 60 |
| all | 7 | 43 | 3 | 7 | 0 | 60 |

Cases that had a closest intent, by its score. Matched: the intent's tool is a tool the case expects. Unjudged: the case expects no tool.

| Score | Matched | Missed | Unjudged | Matched of judged |
|---|---|---|---|---|
| 0.78 to 0.80 | 0 | 0 | 2 | none |
| 0.84 to 0.86 | 0 | 0 | 3 | none |
| 0.86 to 0.88 | 0 | 0 | 2 | none |
| 0.88 to 0.90 | 0 | 0 | 6 | none |
| 0.90 to 0.92 | 0 | 0 | 2 | none |

## Conversation bench

Deterministic checks only; the rubric and human labels come at development time. Invented names, wrong scenes and missing quotes are rates over all names, scene citations and quotes in the answers.

| Question type | Cases | Setup errors | Answer facts | Invented names | Wrong scene citations | Missing quotes | Within length | Correct abstention | Repetition | Median ms |
|---|---|---|---|---|---|---|---|---|---|---|
| all | 60 | 0 | 46.4% (26/56) | 7.7% (11/142) | 0.0% (0/86) | 0.0% (0/1) | 71.7% (43/60) | 50.0% (2/4) | 0.0% (0/60) | 32256 |
| abstain | 4 | 0 | none | 0.0% (0/6) | 0.0% (0/1) | none | 100.0% (4/4) | 50.0% (2/4) | 0.0% (0/4) | 32121 |
| character | 10 | 0 | 90.0% (9/10) | 17.2% (5/29) | 0.0% (0/2) | none | 100.0% (10/10) | none | 0.0% (0/10) | 24305 |
| compare | 5 | 0 | 20.0% (1/5) | 5.6% (1/18) | 0.0% (0/8) | none | 60.0% (3/5) | none | 0.0% (0/5) | 52121 |
| critique | 4 | 0 | 50.0% (2/4) | 0.0% (0/20) | 0.0% (0/3) | none | 100.0% (4/4) | none | 0.0% (0/4) | 46045 |
| explain | 10 | 0 | 50.0% (5/10) | 0.0% (0/18) | 0.0% (0/9) | none | 70.0% (7/10) | none | 0.0% (0/10) | 36363 |
| followup | 4 | 0 | 25.0% (1/4) | 0.0% (0/9) | 0.0% (0/3) | none | 75.0% (3/4) | none | 0.0% (0/4) | 33866 |
| ideas | 5 | 0 | 40.0% (2/5) | 20.0% (1/5) | 0.0% (0/1) | none | 40.0% (2/5) | none | 0.0% (0/5) | 36479 |
| relationship | 8 | 0 | 37.5% (3/8) | 0.0% (0/17) | 0.0% (0/8) | none | 62.5% (5/8) | none | 0.0% (0/8) | 28652 |
| theme | 5 | 0 | 60.0% (3/5) | 0.0% (0/6) | 0.0% (0/1) | 0.0% (0/1) | 40.0% (2/5) | none | 0.0% (0/5) | 13809 |
| whole | 5 | 0 | 0.0% (0/5) | 28.6% (4/14) | 0.0% (0/50) | none | 60.0% (3/5) | none | 0.0% (0/5) | 53318 |

Writing quality is scored separately against each case's rubric at development time (ADR-0028 decision 9); this table covers only the deterministic measures.
