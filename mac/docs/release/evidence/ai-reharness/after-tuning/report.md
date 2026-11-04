# AI evaluation report

- Cases: 393
- Gold folder: crates/openframe-ai-eval/gold
- Model files: the pinned Offline AI of packaging/ai/ai-assets.lock.json
- Build: release
- Search model: Automatic
- Engine place: auto
- Sampling: deterministic (temperature 0, fixed seed)
- Wall time: 29.8 min

## English release bar (ADR-0028 decision 2)

| Measure | Result | Bar |
|---|---|---|
| Correct tool and arguments (tool and action cases) | none | 90% |
| Correct end state after apply (action cases) | none | 85% |

Bar met: **no**

## By kind

| kind | Cases | Setup errors | Right tool offered | Tool chosen | Tool and arguments | Schema valid | End state | Answer facts | Language | Length | Safety | Not empty | Grounding | Recall@5 | Recall@10 | MRR | Median ms | p95 ms | Engine MB |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| conversation | 393 | 0 | none | none | none | 100.0% (393/393) | none | 96.4% (324/336) | 100.0% (393/393) | 100.0% (40/40) | none | 100.0% (393/393) | 64.5% | none | none | none | 2274 | 15828 | 2445 |

## By language

| language | Cases | Setup errors | Right tool offered | Tool chosen | Tool and arguments | Schema valid | End state | Answer facts | Language | Length | Safety | Not empty | Grounding | Recall@5 | Recall@10 | MRR | Median ms | p95 ms | Engine MB |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| en | 393 | 0 | none | none | none | 100.0% (393/393) | none | 96.4% (324/336) | 100.0% (393/393) | 100.0% (40/40) | none | 100.0% (393/393) | 64.5% | none | none | none | 2274 | 15828 | 2445 |

## Answer path and fallback

Cases by kind and the answer path they took.

| Kind | write | navigate | change | fixed | Cases |
|---|---|---|---|---|---|
| conversation | 328 | 0 | 0 | 65 | 393 |
| all | 328 | 0 | 0 | 65 | 393 |

Cases by kind and the fallback they used. `last_resort` is the last-resort answer; `abstained` is a could-not-find answer.

| Kind | none | last_resort | abstained | Cases |
|---|---|---|---|---|
| conversation | 367 | 22 | 4 | 393 |
| all | 367 | 22 | 4 | 393 |

## Conversation bench

Deterministic checks only; the rubric and human labels come at development time. Invented names, wrong scenes and missing quotes are rates over all names, scene citations and quotes in the answers.

| Question type | Cases | Setup errors | Answer facts | Invented names | Wrong scene citations | Missing quotes | Within length | Correct abstention | Repetition | Median ms |
|---|---|---|---|---|---|---|---|---|---|---|
| all | 393 | 0 | 96.4% (324/336) | 37.4% (419/1120) | 0.0% (0/39) | 78.3% (18/23) | 100.0% (40/40) | 66.7% (10/15) | 2.0% (8/393) | 2274 |
| 001 | 3 | 0 | 100.0% (2/2) | 71.4% (5/7) | none | none | none | 0.0% (0/1) | 0.0% (0/3) | 1997 |
| 002 | 3 | 0 | 100.0% (3/3) | 75.0% (6/8) | none | none | none | none | 0.0% (0/3) | 4072 |
| 003 | 3 | 0 | 100.0% (2/2) | 87.5% (7/8) | none | none | none | none | 0.0% (0/3) | 1476 |
| 004 | 3 | 0 | 100.0% (3/3) | 100.0% (5/5) | none | none | none | none | 0.0% (0/3) | 2507 |
| 005 | 3 | 0 | 100.0% (3/3) | 100.0% (4/4) | 0.0% (0/1) | none | none | none | 0.0% (0/3) | 1130 |
| 006 | 3 | 0 | 100.0% (3/3) | 20.8% (5/24) | none | none | none | none | 0.0% (0/3) | 8608 |
| 007 | 3 | 0 | 100.0% (2/2) | 66.7% (10/15) | none | none | none | none | 0.0% (0/3) | 3254 |
| 008 | 3 | 0 | 100.0% (3/3) | 44.4% (4/9) | none | none | none | none | 0.0% (0/3) | 2549 |
| 009 | 3 | 0 | 100.0% (3/3) | 55.6% (5/9) | none | none | none | none | 0.0% (0/3) | 1475 |
| 01 | 2 | 0 | 50.0% (1/2) | 16.7% (1/6) | none | none | none | none | 0.0% (0/2) | 15539 |
| 010 | 3 | 0 | 100.0% (2/2) | 33.3% (2/6) | none | none | none | none | 0.0% (0/3) | 2451 |
| 011 | 2 | 0 | 100.0% (2/2) | 25.0% (1/4) | none | none | none | none | 0.0% (0/2) | 6676 |
| 012 | 2 | 0 | 100.0% (2/2) | 60.0% (3/5) | none | none | none | none | 0.0% (0/2) | 1708 |
| 013 | 2 | 0 | 50.0% (1/2) | 33.3% (1/3) | none | none | none | none | 0.0% (0/2) | 1206 |
| 014 | 2 | 0 | 100.0% (2/2) | 40.0% (2/5) | 0.0% (0/1) | none | none | none | 0.0% (0/2) | 2939 |
| 015 | 2 | 0 | 100.0% (2/2) | 100.0% (2/2) | none | none | none | none | 0.0% (0/2) | 1089 |
| 016 | 2 | 0 | 100.0% (2/2) | 93.3% (14/15) | none | none | none | none | 0.0% (0/2) | 7659 |
| 017 | 2 | 0 | 100.0% (2/2) | 100.0% (3/3) | 0.0% (0/1) | 100.0% (1/1) | none | none | 0.0% (0/2) | 2395 |
| 018 | 2 | 0 | 100.0% (2/2) | 100.0% (2/2) | none | none | none | none | 0.0% (0/2) | 1013 |
| 019 | 2 | 0 | 100.0% (2/2) | 100.0% (2/2) | none | none | none | none | 0.0% (0/2) | 1556 |
| 02 | 2 | 0 | 100.0% (2/2) | 33.3% (2/6) | 0.0% (0/2) | none | none | none | 0.0% (0/2) | 4723 |
| 020 | 2 | 0 | 100.0% (1/1) | 50.0% (2/4) | none | none | none | none | 0.0% (0/2) | 2515 |
| 021 | 1 | 0 | 100.0% (1/1) | none | none | none | none | none | 0.0% (0/1) | 2211 |
| 022 | 1 | 0 | 100.0% (1/1) | 100.0% (3/3) | none | none | none | none | 0.0% (0/1) | 7 |
| 023 | 1 | 0 | 100.0% (1/1) | 100.0% (2/2) | none | none | none | none | 0.0% (0/1) | 7 |
| 024 | 1 | 0 | 100.0% (1/1) | 100.0% (2/2) | none | none | none | none | 0.0% (0/1) | 8 |
| 025 | 1 | 0 | 100.0% (1/1) | 100.0% (2/2) | none | none | none | none | 0.0% (0/1) | 5 |
| 026 | 1 | 0 | 100.0% (1/1) | 100.0% (2/2) | none | none | none | none | 0.0% (0/1) | 7 |
| 027 | 1 | 0 | 100.0% (1/1) | 100.0% (2/2) | none | none | none | none | 0.0% (0/1) | 7 |
| 028 | 1 | 0 | 100.0% (1/1) | 100.0% (2/2) | none | none | none | none | 0.0% (0/1) | 10 |
| 029 | 1 | 0 | 100.0% (1/1) | 100.0% (1/1) | none | none | none | none | 0.0% (0/1) | 7 |
| 03 | 2 | 0 | 50.0% (1/2) | 0.0% (0/13) | 0.0% (0/1) | 0.0% (0/1) | none | none | 50.0% (1/2) | 25484 |
| 030 | 1 | 0 | 100.0% (1/1) | 100.0% (3/3) | none | none | none | none | 0.0% (0/1) | 6 |
| 031 | 1 | 0 | 100.0% (1/1) | 50.0% (1/2) | none | none | none | none | 0.0% (0/1) | 4 |
| 032 | 1 | 0 | 100.0% (1/1) | 100.0% (2/2) | none | none | none | none | 0.0% (0/1) | 589 |
| 033 | 1 | 0 | 100.0% (1/1) | 100.0% (2/2) | none | none | none | none | 0.0% (0/1) | 869 |
| 034 | 1 | 0 | 100.0% (1/1) | 100.0% (3/3) | none | none | none | none | 0.0% (0/1) | 10 |
| 035 | 1 | 0 | 100.0% (1/1) | 0.0% (0/1) | none | none | none | none | 0.0% (0/1) | 9 |
| 036 | 1 | 0 | 100.0% (1/1) | 100.0% (2/2) | none | none | none | none | 0.0% (0/1) | 856 |
| 037 | 1 | 0 | 100.0% (1/1) | 100.0% (1/1) | none | none | none | none | 0.0% (0/1) | 468 |
| 038 | 1 | 0 | 100.0% (1/1) | 100.0% (2/2) | none | none | none | none | 0.0% (0/1) | 517 |
| 039 | 1 | 0 | 100.0% (1/1) | 0.0% (0/1) | none | none | none | none | 0.0% (0/1) | 9 |
| 04 | 2 | 0 | 100.0% (2/2) | 14.3% (2/14) | 0.0% (0/2) | 0.0% (0/1) | none | none | 0.0% (0/2) | 20114 |
| 040 | 1 | 0 | 100.0% (1/1) | 100.0% (2/2) | none | none | none | none | 0.0% (0/1) | 891 |
| 041 | 1 | 0 | 100.0% (1/1) | 100.0% (2/2) | none | none | none | none | 0.0% (0/1) | 7 |
| 042 | 1 | 0 | 100.0% (1/1) | 100.0% (3/3) | none | none | none | none | 0.0% (0/1) | 7 |
| 043 | 1 | 0 | 100.0% (1/1) | 100.0% (2/2) | none | none | none | none | 0.0% (0/1) | 10 |
| 044 | 1 | 0 | 100.0% (1/1) | 100.0% (1/1) | none | none | none | none | 0.0% (0/1) | 10 |
| 045 | 1 | 0 | 100.0% (1/1) | 100.0% (2/2) | none | none | none | none | 0.0% (0/1) | 881 |
| 046 | 1 | 0 | 100.0% (1/1) | 100.0% (2/2) | none | none | none | none | 0.0% (0/1) | 7 |
| 047 | 1 | 0 | 100.0% (1/1) | 0.0% (0/1) | none | none | none | none | 0.0% (0/1) | 6 |
| 048 | 1 | 0 | 100.0% (1/1) | 50.0% (1/2) | none | none | none | none | 0.0% (0/1) | 454 |
| 049 | 1 | 0 | 100.0% (1/1) | 100.0% (1/1) | none | none | none | none | 0.0% (0/1) | 5 |
| 05 | 2 | 0 | 100.0% (1/1) | 45.5% (5/11) | none | none | none | none | 0.0% (0/2) | 7246 |
| 050 | 1 | 0 | 100.0% (1/1) | 100.0% (2/2) | none | none | none | none | 0.0% (0/1) | 533 |
| 051 | 1 | 0 | 100.0% (1/1) | 100.0% (3/3) | none | none | none | none | 0.0% (0/1) | 8 |
| 052 | 1 | 0 | 100.0% (1/1) | 100.0% (2/2) | none | none | none | none | 0.0% (0/1) | 882 |
| 053 | 1 | 0 | 100.0% (1/1) | 100.0% (1/1) | none | none | none | none | 0.0% (0/1) | 6 |
| 054 | 1 | 0 | 100.0% (1/1) | 100.0% (2/2) | none | none | none | none | 0.0% (0/1) | 9 |
| 055 | 1 | 0 | 100.0% (1/1) | 100.0% (3/3) | none | none | none | none | 0.0% (0/1) | 10 |
| 056 | 1 | 0 | 100.0% (1/1) | 0.0% (0/1) | none | none | none | none | 0.0% (0/1) | 12 |
| 057 | 1 | 0 | 100.0% (1/1) | 100.0% (2/2) | none | none | none | none | 0.0% (0/1) | 890 |
| 058 | 1 | 0 | 100.0% (1/1) | 100.0% (2/2) | none | none | none | none | 0.0% (0/1) | 432 |
| 059 | 1 | 0 | 100.0% (1/1) | 100.0% (2/2) | none | none | none | none | 0.0% (0/1) | 4 |
| 06 | 2 | 0 | 100.0% (2/2) | 54.5% (6/11) | 0.0% (0/3) | none | none | none | 50.0% (1/2) | 5115 |
| 060 | 1 | 0 | 100.0% (1/1) | 100.0% (2/2) | none | none | none | none | 0.0% (0/1) | 550 |
| 07 | 2 | 0 | 100.0% (2/2) | 18.8% (3/16) | none | none | none | none | 0.0% (0/2) | 34670 |
| 08 | 2 | 0 | 0.0% (0/2) | 0.0% (0/5) | 0.0% (0/1) | none | none | none | 0.0% (0/2) | 3981 |
| 09 | 2 | 0 | 100.0% (2/2) | 50.0% (2/4) | none | none | none | none | 0.0% (0/2) | 5172 |
| 1 | 2 | 0 | 50.0% (1/2) | 40.0% (2/5) | 0.0% (0/1) | none | none | none | 0.0% (0/2) | 6237 |
| 10 | 2 | 0 | 100.0% (2/2) | 37.5% (3/8) | none | none | none | none | 0.0% (0/2) | 6244 |
| 11 | 2 | 0 | 100.0% (2/2) | 16.7% (1/6) | none | none | none | none | 0.0% (0/2) | 12582 |
| 12 | 2 | 0 | 100.0% (2/2) | 0.0% (0/6) | 0.0% (0/2) | 0.0% (0/1) | none | none | 0.0% (0/2) | 18622 |
| 13 | 2 | 0 | 100.0% (2/2) | 62.5% (5/8) | 0.0% (0/2) | none | none | none | 0.0% (0/2) | 9100 |
| 14 | 2 | 0 | 100.0% (2/2) | 100.0% (5/5) | none | none | none | none | 0.0% (0/2) | 5585 |
| 15 | 2 | 0 | 100.0% (2/2) | 42.9% (3/7) | none | none | none | none | 0.0% (0/2) | 7905 |
| 2 | 2 | 0 | 50.0% (1/2) | 20.0% (4/20) | 0.0% (0/2) | none | none | none | 0.0% (0/2) | 26982 |
| 3 | 2 | 0 | none | 75.0% (3/4) | none | none | none | none | 0.0% (0/2) | 3527 |
| 4 | 2 | 0 | 100.0% (2/2) | 53.8% (7/13) | 0.0% (0/1) | none | none | none | 50.0% (1/2) | 6268 |
| 5 | 2 | 0 | 100.0% (2/2) | 50.0% (2/4) | none | none | none | none | 0.0% (0/2) | 2992 |
| beat | 10 | 0 | 100.0% (8/8) | 9.4% (3/32) | none | none | none | none | 0.0% (0/10) | 4058 |
| brief | 10 | 0 | 70.0% (7/10) | 0.0% (0/39) | 0.0% (0/2) | none | 100.0% (10/10) | none | 10.0% (1/10) | 5156 |
| brk | 10 | 0 | 100.0% (9/9) | 0.0% (0/12) | 0.0% (0/3) | none | none | none | 0.0% (0/10) | 2112 |
| budget | 10 | 0 | 100.0% (8/8) | 85.7% (12/14) | none | none | none | none | 0.0% (0/10) | 2083 |
| cast | 10 | 0 | 100.0% (4/4) | 53.8% (7/13) | none | none | none | 100.0% (1/1) | 0.0% (0/10) | 1478 |
| cat | 3 | 0 | 100.0% (3/3) | none | none | none | none | none | 0.0% (0/3) | 1518 |
| char | 10 | 0 | 100.0% (10/10) | 5.4% (4/74) | none | none | 100.0% (10/10) | none | 10.0% (1/10) | 8520 |
| chars | 10 | 0 | 100.0% (10/10) | 24.1% (14/58) | none | none | none | none | 0.0% (0/10) | 1710 |
| cs | 10 | 0 | 100.0% (8/8) | 58.3% (7/12) | none | none | none | 100.0% (1/1) | 0.0% (0/10) | 3787 |
| daily | 10 | 0 | 100.0% (7/7) | 40.0% (10/25) | none | 100.0% (4/4) | none | none | 10.0% (1/10) | 2537 |
| dood | 10 | 0 | 100.0% (9/9) | 30.8% (8/26) | none | 100.0% (1/1) | none | none | 0.0% (0/10) | 2592 |
| files | 10 | 0 | 100.0% (9/9) | 80.0% (4/5) | none | none | none | none | 0.0% (0/10) | 1774 |
| guide | 10 | 0 | 90.0% (9/10) | 77.6% (38/49) | none | none | none | none | 0.0% (0/10) | 3132 |
| loc | 10 | 0 | 100.0% (8/8) | 17.6% (3/17) | none | none | none | none | 0.0% (0/10) | 1461 |
| mood | 10 | 0 | 100.0% (9/9) | 28.6% (4/14) | none | 100.0% (2/2) | none | none | 0.0% (0/10) | 3382 |
| notes | 10 | 0 | 100.0% (9/9) | 40.0% (6/15) | none | 100.0% (1/1) | none | none | 0.0% (0/10) | 1650 |
| outline | 2 | 0 | none | 25.0% (1/4) | none | none | none | 0.0% (0/2) | 0.0% (0/2) | 37038 |
| ovr | 10 | 0 | 100.0% (10/10) | 100.0% (1/1) | none | none | none | none | 0.0% (0/10) | 433 |
| rel | 10 | 0 | none | 1.1% (1/88) | none | none | 100.0% (10/10) | none | 0.0% (0/10) | 9871 |
| reports | 2 | 0 | none | 33.3% (1/3) | none | none | none | 50.0% (1/2) | 0.0% (0/2) | 2873 |
| scenes | 10 | 0 | 88.9% (8/9) | 0.0% (0/29) | 0.0% (0/6) | none | none | none | 0.0% (0/10) | 2922 |
| sched | 10 | 0 | 100.0% (8/8) | 33.3% (2/6) | 0.0% (0/3) | none | none | none | 0.0% (0/10) | 1215 |
| settings | 8 | 0 | 100.0% (8/8) | 87.0% (40/46) | none | 100.0% (1/1) | none | none | 0.0% (0/8) | 13128 |
| shots | 10 | 0 | 100.0% (10/10) | 44.4% (4/9) | 0.0% (0/2) | none | none | none | 0.0% (0/10) | 1433 |
| sides | 2 | 0 | none | 0.0% (0/2) | none | none | none | 100.0% (2/2) | 0.0% (0/2) | 510 |
| spage | 2 | 0 | none | 0.0% (0/2) | none | none | none | 100.0% (2/2) | 0.0% (0/2) | 849 |
| story | 20 | 0 | 100.0% (18/18) | 17.8% (16/90) | none | 75.0% (3/4) | 100.0% (10/10) | none | 10.0% (2/20) | 13032 |
| tasks | 10 | 0 | 100.0% (10/10) | 50.0% (11/22) | 0.0% (0/1) | none | none | none | 0.0% (0/10) | 2709 |
| timeline | 2 | 0 | none | 100.0% (2/2) | 0.0% (0/2) | none | none | 50.0% (1/2) | 0.0% (0/2) | 1969 |
| vault | 10 | 0 | 100.0% (10/10) | 40.9% (9/22) | none | 83.3% (5/6) | none | none | 0.0% (0/10) | 1999 |
| vglobal | 2 | 0 | none | 50.0% (1/2) | none | none | none | 100.0% (2/2) | 0.0% (0/2) | 7 |

Expectation flags (harness spec 7.4). Cases that set none of them show none. Flags the build cannot measure yet are listed as not counted, never as passed or failed.

| Question type | Required facts | Abstain names | No navigation | No data dump | No internal words | Length class | No re-ask | Answer type | Answer group | Not counted |
|---|---|---|---|---|---|---|---|---|---|---|
| all | 92.7% (179/193) | 60.0% (9/15) | 100.0% (42/42) | none | 100.0% (393/393) | 94.4% (371/393) | 100.0% (20/20) | 78.6% (309/393) | 91.3% (359/393) | 0 |
| 001 | none | 0.0% (0/1) | none | none | 100.0% (3/3) | 100.0% (3/3) | 100.0% (1/1) | 100.0% (3/3) | 100.0% (3/3) | 0 |
| 002 | none | none | none | none | 100.0% (3/3) | 100.0% (3/3) | 100.0% (1/1) | 100.0% (3/3) | 100.0% (3/3) | 0 |
| 003 | 100.0% (1/1) | none | none | none | 100.0% (3/3) | 100.0% (3/3) | 100.0% (1/1) | 66.7% (2/3) | 100.0% (3/3) | 0 |
| 004 | none | none | none | none | 100.0% (3/3) | 100.0% (3/3) | 100.0% (1/1) | 66.7% (2/3) | 100.0% (3/3) | 0 |
| 005 | none | none | none | none | 100.0% (3/3) | 100.0% (3/3) | 100.0% (1/1) | 66.7% (2/3) | 100.0% (3/3) | 0 |
| 006 | none | none | none | none | 100.0% (3/3) | 100.0% (3/3) | 100.0% (1/1) | 33.3% (1/3) | 66.7% (2/3) | 0 |
| 007 | 100.0% (1/1) | none | none | none | 100.0% (3/3) | 100.0% (3/3) | 100.0% (1/1) | 66.7% (2/3) | 66.7% (2/3) | 0 |
| 008 | 0.0% (0/1) | none | none | none | 100.0% (3/3) | 100.0% (3/3) | 100.0% (1/1) | 66.7% (2/3) | 100.0% (3/3) | 0 |
| 009 | none | none | none | none | 100.0% (3/3) | 100.0% (3/3) | 100.0% (1/1) | 100.0% (3/3) | 100.0% (3/3) | 0 |
| 01 | 66.7% (2/3) | none | none | none | 100.0% (2/2) | 100.0% (2/2) | none | 100.0% (2/2) | 100.0% (2/2) | 0 |
| 010 | 100.0% (2/2) | none | none | none | 100.0% (3/3) | 100.0% (3/3) | 100.0% (1/1) | 100.0% (3/3) | 100.0% (3/3) | 0 |
| 011 | none | none | none | none | 100.0% (2/2) | 50.0% (1/2) | 100.0% (1/1) | 50.0% (1/2) | 50.0% (1/2) | 0 |
| 012 | none | none | none | none | 100.0% (2/2) | 100.0% (2/2) | 100.0% (1/1) | 100.0% (2/2) | 100.0% (2/2) | 0 |
| 013 | none | none | none | none | 100.0% (2/2) | 100.0% (2/2) | 100.0% (1/1) | 50.0% (1/2) | 100.0% (2/2) | 0 |
| 014 | none | none | none | none | 100.0% (2/2) | 100.0% (2/2) | 100.0% (1/1) | 50.0% (1/2) | 50.0% (1/2) | 0 |
| 015 | none | none | none | none | 100.0% (2/2) | 100.0% (2/2) | 100.0% (1/1) | 50.0% (1/2) | 100.0% (2/2) | 0 |
| 016 | 100.0% (1/1) | none | none | none | 100.0% (2/2) | 100.0% (2/2) | 100.0% (1/1) | 50.0% (1/2) | 100.0% (2/2) | 0 |
| 017 | none | none | none | none | 100.0% (2/2) | 100.0% (2/2) | 100.0% (1/1) | 50.0% (1/2) | 100.0% (2/2) | 0 |
| 018 | none | none | none | none | 100.0% (2/2) | 100.0% (2/2) | 100.0% (1/1) | 50.0% (1/2) | 100.0% (2/2) | 0 |
| 019 | none | none | none | none | 100.0% (2/2) | 100.0% (2/2) | 100.0% (1/1) | 100.0% (2/2) | 100.0% (2/2) | 0 |
| 02 | 100.0% (2/2) | none | none | none | 100.0% (2/2) | 100.0% (2/2) | none | 100.0% (2/2) | 100.0% (2/2) | 0 |
| 020 | 100.0% (1/1) | none | none | none | 100.0% (2/2) | 100.0% (2/2) | 100.0% (1/1) | 50.0% (1/2) | 100.0% (2/2) | 0 |
| 021 | none | none | none | none | 100.0% (1/1) | 100.0% (1/1) | none | 100.0% (1/1) | 100.0% (1/1) | 0 |
| 022 | none | none | none | none | 100.0% (1/1) | 100.0% (1/1) | none | 100.0% (1/1) | 100.0% (1/1) | 0 |
| 023 | none | none | none | none | 100.0% (1/1) | 100.0% (1/1) | none | 100.0% (1/1) | 100.0% (1/1) | 0 |
| 024 | none | none | none | none | 100.0% (1/1) | 100.0% (1/1) | none | 100.0% (1/1) | 100.0% (1/1) | 0 |
| 025 | none | none | none | none | 100.0% (1/1) | 100.0% (1/1) | none | 0.0% (0/1) | 100.0% (1/1) | 0 |
| 026 | none | none | none | none | 100.0% (1/1) | 100.0% (1/1) | none | 0.0% (0/1) | 0.0% (0/1) | 0 |
| 027 | none | none | none | none | 100.0% (1/1) | 100.0% (1/1) | none | 100.0% (1/1) | 100.0% (1/1) | 0 |
| 028 | none | none | none | none | 100.0% (1/1) | 100.0% (1/1) | none | 0.0% (0/1) | 100.0% (1/1) | 0 |
| 029 | none | none | none | none | 100.0% (1/1) | 100.0% (1/1) | none | 0.0% (0/1) | 0.0% (0/1) | 0 |
| 03 | 100.0% (2/2) | none | none | none | 100.0% (2/2) | 100.0% (2/2) | none | 100.0% (2/2) | 100.0% (2/2) | 0 |
| 030 | none | none | none | none | 100.0% (1/1) | 100.0% (1/1) | none | 100.0% (1/1) | 100.0% (1/1) | 0 |
| 031 | none | none | none | none | 100.0% (1/1) | 100.0% (1/1) | none | 0.0% (0/1) | 0.0% (0/1) | 0 |
| 032 | none | none | none | none | 100.0% (1/1) | 100.0% (1/1) | none | 100.0% (1/1) | 100.0% (1/1) | 0 |
| 033 | none | none | none | none | 100.0% (1/1) | 100.0% (1/1) | none | 0.0% (0/1) | 0.0% (0/1) | 0 |
| 034 | none | none | none | none | 100.0% (1/1) | 100.0% (1/1) | none | 100.0% (1/1) | 100.0% (1/1) | 0 |
| 035 | none | none | none | none | 100.0% (1/1) | 100.0% (1/1) | none | 0.0% (0/1) | 100.0% (1/1) | 0 |
| 036 | none | none | none | none | 100.0% (1/1) | 100.0% (1/1) | none | 100.0% (1/1) | 100.0% (1/1) | 0 |
| 037 | none | none | none | none | 100.0% (1/1) | 100.0% (1/1) | none | 100.0% (1/1) | 100.0% (1/1) | 0 |
| 038 | none | none | none | none | 100.0% (1/1) | 100.0% (1/1) | none | 100.0% (1/1) | 100.0% (1/1) | 0 |
| 039 | none | none | none | none | 100.0% (1/1) | 100.0% (1/1) | none | 0.0% (0/1) | 100.0% (1/1) | 0 |
| 04 | 100.0% (4/4) | none | none | none | 100.0% (2/2) | 100.0% (2/2) | none | 50.0% (1/2) | 100.0% (2/2) | 0 |
| 040 | none | none | none | none | 100.0% (1/1) | 100.0% (1/1) | none | 100.0% (1/1) | 100.0% (1/1) | 0 |
| 041 | none | none | none | none | 100.0% (1/1) | 100.0% (1/1) | none | 100.0% (1/1) | 100.0% (1/1) | 0 |
| 042 | none | none | none | none | 100.0% (1/1) | 100.0% (1/1) | none | 100.0% (1/1) | 100.0% (1/1) | 0 |
| 043 | none | none | none | none | 100.0% (1/1) | 100.0% (1/1) | none | 100.0% (1/1) | 100.0% (1/1) | 0 |
| 044 | none | none | none | none | 100.0% (1/1) | 100.0% (1/1) | none | 0.0% (0/1) | 100.0% (1/1) | 0 |
| 045 | none | none | none | none | 100.0% (1/1) | 100.0% (1/1) | none | 100.0% (1/1) | 100.0% (1/1) | 0 |
| 046 | none | none | none | none | 100.0% (1/1) | 100.0% (1/1) | none | 0.0% (0/1) | 0.0% (0/1) | 0 |
| 047 | none | none | none | none | 100.0% (1/1) | 100.0% (1/1) | none | 0.0% (0/1) | 100.0% (1/1) | 0 |
| 048 | none | none | none | none | 100.0% (1/1) | 100.0% (1/1) | none | 100.0% (1/1) | 100.0% (1/1) | 0 |
| 049 | none | none | none | none | 100.0% (1/1) | 100.0% (1/1) | none | 0.0% (0/1) | 0.0% (0/1) | 0 |
| 05 | 100.0% (3/3) | none | none | none | 100.0% (2/2) | 100.0% (2/2) | none | 100.0% (2/2) | 100.0% (2/2) | 0 |
| 050 | none | none | none | none | 100.0% (1/1) | 100.0% (1/1) | none | 100.0% (1/1) | 100.0% (1/1) | 0 |
| 051 | none | none | none | none | 100.0% (1/1) | 100.0% (1/1) | none | 100.0% (1/1) | 100.0% (1/1) | 0 |
| 052 | none | none | none | none | 100.0% (1/1) | 100.0% (1/1) | none | 100.0% (1/1) | 100.0% (1/1) | 0 |
| 053 | none | none | none | none | 100.0% (1/1) | 100.0% (1/1) | none | 0.0% (0/1) | 0.0% (0/1) | 0 |
| 054 | none | none | none | none | 100.0% (1/1) | 100.0% (1/1) | none | 0.0% (0/1) | 100.0% (1/1) | 0 |
| 055 | none | none | none | none | 100.0% (1/1) | 100.0% (1/1) | none | 100.0% (1/1) | 100.0% (1/1) | 0 |
| 056 | none | none | none | none | 100.0% (1/1) | 100.0% (1/1) | none | 0.0% (0/1) | 100.0% (1/1) | 0 |
| 057 | none | none | none | none | 100.0% (1/1) | 100.0% (1/1) | none | 100.0% (1/1) | 100.0% (1/1) | 0 |
| 058 | none | none | none | none | 100.0% (1/1) | 100.0% (1/1) | none | 0.0% (0/1) | 0.0% (0/1) | 0 |
| 059 | none | none | none | none | 100.0% (1/1) | 100.0% (1/1) | none | 100.0% (1/1) | 100.0% (1/1) | 0 |
| 06 | 100.0% (4/4) | none | none | none | 100.0% (2/2) | 100.0% (2/2) | none | 100.0% (2/2) | 100.0% (2/2) | 0 |
| 060 | none | none | none | none | 100.0% (1/1) | 100.0% (1/1) | none | 100.0% (1/1) | 100.0% (1/1) | 0 |
| 07 | 50.0% (1/2) | none | none | none | 100.0% (2/2) | 50.0% (1/2) | none | 50.0% (1/2) | 50.0% (1/2) | 0 |
| 08 | 100.0% (2/2) | none | none | none | 100.0% (2/2) | 100.0% (2/2) | none | 100.0% (2/2) | 100.0% (2/2) | 0 |
| 09 | none | none | none | none | 100.0% (2/2) | 100.0% (2/2) | none | 100.0% (2/2) | 100.0% (2/2) | 0 |
| 1 | 50.0% (1/2) | none | none | none | 100.0% (2/2) | 100.0% (2/2) | none | 100.0% (2/2) | 100.0% (2/2) | 0 |
| 10 | none | none | none | none | 100.0% (2/2) | 100.0% (2/2) | none | 100.0% (2/2) | 100.0% (2/2) | 0 |
| 11 | none | none | none | none | 100.0% (2/2) | 100.0% (2/2) | none | 0.0% (0/2) | 100.0% (2/2) | 0 |
| 12 | 100.0% (1/1) | none | none | none | 100.0% (2/2) | 100.0% (2/2) | none | 100.0% (2/2) | 100.0% (2/2) | 0 |
| 13 | 100.0% (2/2) | none | none | none | 100.0% (2/2) | 100.0% (2/2) | none | 50.0% (1/2) | 100.0% (2/2) | 0 |
| 14 | none | none | none | none | 100.0% (2/2) | 100.0% (2/2) | none | 100.0% (2/2) | 100.0% (2/2) | 0 |
| 15 | none | none | none | none | 100.0% (2/2) | 100.0% (2/2) | none | 0.0% (0/2) | 100.0% (2/2) | 0 |
| 2 | 100.0% (3/3) | none | none | none | 100.0% (2/2) | 100.0% (2/2) | none | 100.0% (2/2) | 100.0% (2/2) | 0 |
| 3 | 100.0% (4/4) | none | none | none | 100.0% (2/2) | 100.0% (2/2) | none | 100.0% (2/2) | 100.0% (2/2) | 0 |
| 4 | 100.0% (1/1) | none | none | none | 100.0% (2/2) | 100.0% (2/2) | none | 100.0% (2/2) | 100.0% (2/2) | 0 |
| 5 | 100.0% (2/2) | none | none | none | 100.0% (2/2) | 100.0% (2/2) | none | 0.0% (0/2) | 0.0% (0/2) | 0 |
| beat | 100.0% (7/7) | none | none | none | 100.0% (10/10) | 90.0% (9/10) | none | 70.0% (7/10) | 90.0% (9/10) | 0 |
| brief | 100.0% (25/25) | none | 100.0% (10/10) | none | 100.0% (10/10) | 100.0% (10/10) | none | 100.0% (10/10) | 100.0% (10/10) | 0 |
| brk | 100.0% (2/2) | none | none | none | 100.0% (10/10) | 90.0% (9/10) | none | 80.0% (8/10) | 100.0% (10/10) | 0 |
| budget | 100.0% (6/6) | none | none | none | 100.0% (10/10) | 90.0% (9/10) | none | 50.0% (5/10) | 100.0% (10/10) | 0 |
| cast | 100.0% (5/5) | 100.0% (1/1) | none | none | 100.0% (10/10) | 90.0% (9/10) | none | 90.0% (9/10) | 100.0% (10/10) | 0 |
| cat | none | none | none | none | 100.0% (3/3) | 100.0% (3/3) | none | 100.0% (3/3) | 100.0% (3/3) | 0 |
| char | 100.0% (14/14) | none | 100.0% (10/10) | none | 100.0% (10/10) | 100.0% (10/10) | none | 100.0% (10/10) | 100.0% (10/10) | 0 |
| chars | none | none | none | none | 100.0% (10/10) | 70.0% (7/10) | none | 50.0% (5/10) | 50.0% (5/10) | 0 |
| cs | 0.0% (0/4) | 100.0% (1/1) | none | none | 100.0% (10/10) | 100.0% (10/10) | none | 90.0% (9/10) | 100.0% (10/10) | 0 |
| daily | 80.0% (4/5) | none | none | none | 100.0% (10/10) | 100.0% (10/10) | none | 90.0% (9/10) | 100.0% (10/10) | 0 |
| dood | 50.0% (1/2) | none | none | none | 100.0% (10/10) | 80.0% (8/10) | none | 90.0% (9/10) | 100.0% (10/10) | 0 |
| files | 100.0% (2/2) | none | none | none | 100.0% (10/10) | 100.0% (10/10) | none | 90.0% (9/10) | 100.0% (10/10) | 0 |
| guide | none | none | none | none | 100.0% (10/10) | 90.0% (9/10) | none | 80.0% (8/10) | 80.0% (8/10) | 0 |
| loc | 100.0% (4/4) | none | none | none | 100.0% (10/10) | 100.0% (10/10) | none | 100.0% (10/10) | 100.0% (10/10) | 0 |
| mood | 100.0% (1/1) | none | none | none | 100.0% (10/10) | 100.0% (10/10) | none | 90.0% (9/10) | 90.0% (9/10) | 0 |
| notes | 0.0% (0/1) | none | none | none | 100.0% (10/10) | 90.0% (9/10) | none | 80.0% (8/10) | 80.0% (8/10) | 0 |
| outline | none | 0.0% (0/2) | none | none | 100.0% (2/2) | 100.0% (2/2) | none | 0.0% (0/2) | 50.0% (1/2) | 0 |
| ovr | none | none | none | none | 100.0% (10/10) | 100.0% (10/10) | none | 100.0% (10/10) | 100.0% (10/10) | 0 |
| rel | 100.0% (30/30) | none | 100.0% (10/10) | none | 100.0% (10/10) | 100.0% (10/10) | none | 100.0% (10/10) | 100.0% (10/10) | 0 |
| reports | none | 50.0% (1/2) | none | none | 100.0% (2/2) | 100.0% (2/2) | none | 100.0% (2/2) | 100.0% (2/2) | 0 |
| scenes | 100.0% (7/7) | none | 100.0% (2/2) | none | 100.0% (10/10) | 80.0% (8/10) | none | 60.0% (6/10) | 80.0% (8/10) | 0 |
| sched | 100.0% (3/3) | none | none | none | 100.0% (10/10) | 100.0% (10/10) | none | 80.0% (8/10) | 100.0% (10/10) | 0 |
| settings | none | none | none | none | 100.0% (8/8) | 100.0% (8/8) | none | 100.0% (8/8) | 100.0% (8/8) | 0 |
| shots | none | none | none | none | 100.0% (10/10) | 90.0% (9/10) | none | 100.0% (10/10) | 100.0% (10/10) | 0 |
| sides | none | 100.0% (2/2) | none | none | 100.0% (2/2) | 100.0% (2/2) | none | 50.0% (1/2) | 100.0% (2/2) | 0 |
| spage | none | 100.0% (2/2) | none | none | 100.0% (2/2) | 100.0% (2/2) | none | 50.0% (1/2) | 50.0% (1/2) | 0 |
| story | 90.3% (28/31) | none | 100.0% (10/10) | none | 100.0% (20/20) | 80.0% (16/20) | none | 80.0% (16/20) | 90.0% (18/20) | 0 |
| tasks | none | none | none | none | 100.0% (10/10) | 90.0% (9/10) | none | 100.0% (10/10) | 100.0% (10/10) | 0 |
| timeline | none | 0.0% (0/2) | none | none | 100.0% (2/2) | 100.0% (2/2) | none | 0.0% (0/2) | 100.0% (2/2) | 0 |
| vault | none | none | none | none | 100.0% (10/10) | 90.0% (9/10) | none | 90.0% (9/10) | 90.0% (9/10) | 0 |
| vglobal | none | 100.0% (2/2) | none | none | 100.0% (2/2) | 100.0% (2/2) | none | 0.0% (0/2) | 50.0% (1/2) | 0 |

## Out-of-section pointer (ADR-0041)

| Pointer precision | Pointer recall | False-pointer rate |
|---|---|---|
| 100.0% (59/59) | 98.3% (59/60) | 0.0% (0/333) |

Writing quality is scored separately against each case's rubric at development time (ADR-0028 decision 9); this table covers only the deterministic measures.

## Latency (spec section 7.5)

| Measure | Value | Bar |
|---|---|---|
| Median, all questions (393 asked) | 2.3 s | 15.0 s or lower: met |
| 95th percentile, all questions | 16.0 s | 30.0 s or lower: met |
| Median, questions that used the engine (347) | 2.6 s | |
| 95th percentile, same | 16.6 s | |
| First question after the project opened | 7.0 s | |
| First question that used the engine | 7.0 s | |
| Prompt tokens served from stored work | 52.6% (349219 of 663329) | |
| Completion tokens, engine calls | 38868, 720 | |

| Length class | Questions | Median | 95th percentile |
|---|---|---|---|
| narrative | 45 | 7.3 s | 23.8 s |
| one_sentence | 228 | 1.6 s | 5.0 s |
| paragraph | 35 | 4.2 s | 25.5 s |
| short | 73 | 2.5 s | 12.4 s |
| story | 12 | 19.0 s | 45.3 s |

| Stage | Mean per question |
|---|---|
| dossier | 6 ms |
| engine_generation | 3788 ms |
| engine_prompt | 1292 ms |
| turn | 184 ms |

Stage times come from the AI trace and from what the request recorded; a stage nobody records is not listed. Engine prompt and generation are the engine's own figures and include the earlier turns a conversation case asks first, so they add up to more than the time of the last question.
