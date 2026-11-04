# AI evaluation report

- Cases: 393
- Gold folder: crates/openframe-ai-eval/gold
- Model files: the pinned OpenFrame AI of packaging/ai/ai-assets.lock.json
- Build: release
- Search model: Automatic
- Engine place: auto
- Sampling: deterministic (temperature 0, fixed seed)
- Wall time: 20.1 min

## English release bar (ADR-0028 decision 2)

| Measure | Result | Bar |
|---|---|---|
| Correct tool and arguments (tool and action cases) | none | 90% |
| Correct end state after apply (action cases) | none | 85% |

Bar met: **no**

## By kind

| kind | Cases | Setup errors | Right tool offered | Tool chosen | Tool and arguments | Schema valid | End state | Answer facts | Language | Length | Safety | Not empty | Grounding | Recall@5 | Recall@10 | MRR | Median ms | p95 ms | Engine MB |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| conversation | 393 | 0 | none | none | none | 100.0% (393/393) | none | 95.8% (322/336) | 100.0% (393/393) | 97.5% (39/40) | none | 100.0% (393/393) | 66.9% | none | none | none | 1773 | 10097 | 2375 |

## By language

| language | Cases | Setup errors | Right tool offered | Tool chosen | Tool and arguments | Schema valid | End state | Answer facts | Language | Length | Safety | Not empty | Grounding | Recall@5 | Recall@10 | MRR | Median ms | p95 ms | Engine MB |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| en | 393 | 0 | none | none | none | 100.0% (393/393) | none | 95.8% (322/336) | 100.0% (393/393) | 97.5% (39/40) | none | 100.0% (393/393) | 66.9% | none | none | none | 1773 | 10097 | 2375 |

## Answer path and fallback

Cases by kind and the answer path they took.

| Kind | write | navigate | change | fixed | Cases |
|---|---|---|---|---|---|
| conversation | 328 | 0 | 0 | 65 | 393 |
| all | 328 | 0 | 0 | 65 | 393 |

Cases by kind and the fallback they used. `last_resort` is the last-resort answer; `abstained` is a could-not-find answer.

| Kind | none | last_resort | abstained | Cases |
|---|---|---|---|---|
| conversation | 367 | 23 | 3 | 393 |
| all | 367 | 23 | 3 | 393 |

## Conversation bench

Deterministic checks only; the rubric and human labels come at development time. Invented names, wrong scenes and missing quotes are rates over all names, scene citations and quotes in the answers.

| Question type | Cases | Setup errors | Answer facts | Invented names | Wrong scene citations | Missing quotes | Within length | Correct abstention | Repetition | Median ms |
|---|---|---|---|---|---|---|---|---|---|---|
| all | 393 | 0 | 95.8% (322/336) | 35.3% (390/1104) | 0.0% (0/41) | 84.6% (22/26) | 97.5% (39/40) | 73.3% (11/15) | 1.8% (7/393) | 1773 |
| 001 | 3 | 0 | 100.0% (2/2) | 71.4% (5/7) | none | none | none | 100.0% (1/1) | 0.0% (0/3) | 1037 |
| 002 | 3 | 0 | 100.0% (3/3) | 75.0% (6/8) | none | none | none | none | 0.0% (0/3) | 2123 |
| 003 | 3 | 0 | 100.0% (2/2) | 87.5% (7/8) | none | none | none | none | 0.0% (0/3) | 701 |
| 004 | 3 | 0 | 100.0% (3/3) | 100.0% (5/5) | none | none | none | none | 0.0% (0/3) | 1548 |
| 005 | 3 | 0 | 100.0% (3/3) | 100.0% (4/4) | 0.0% (0/1) | none | none | none | 0.0% (0/3) | 680 |
| 006 | 3 | 0 | 100.0% (3/3) | 26.9% (7/26) | none | none | none | none | 0.0% (0/3) | 5576 |
| 007 | 3 | 0 | 100.0% (2/2) | 66.7% (10/15) | none | none | none | none | 0.0% (0/3) | 1879 |
| 008 | 3 | 0 | 100.0% (3/3) | 41.7% (5/12) | none | none | none | none | 0.0% (0/3) | 1629 |
| 009 | 3 | 0 | 100.0% (3/3) | 66.7% (4/6) | none | none | none | none | 0.0% (0/3) | 959 |
| 01 | 2 | 0 | 50.0% (1/2) | 16.7% (1/6) | none | none | none | none | 0.0% (0/2) | 7367 |
| 010 | 3 | 0 | 100.0% (2/2) | 40.0% (2/5) | 0.0% (0/1) | none | none | none | 0.0% (0/3) | 1329 |
| 011 | 2 | 0 | 100.0% (2/2) | 20.0% (1/5) | none | none | none | none | 0.0% (0/2) | 2731 |
| 012 | 2 | 0 | 100.0% (2/2) | 60.0% (3/5) | none | none | none | none | 0.0% (0/2) | 1114 |
| 013 | 2 | 0 | 50.0% (1/2) | 33.3% (1/3) | none | none | none | none | 0.0% (0/2) | 770 |
| 014 | 2 | 0 | 100.0% (2/2) | 40.0% (2/5) | 0.0% (0/1) | none | none | none | 0.0% (0/2) | 1690 |
| 015 | 2 | 0 | 100.0% (2/2) | 100.0% (2/2) | none | none | none | none | 0.0% (0/2) | 727 |
| 016 | 2 | 0 | 100.0% (2/2) | 100.0% (12/12) | none | none | none | none | 0.0% (0/2) | 4252 |
| 017 | 2 | 0 | 100.0% (2/2) | 100.0% (3/3) | 0.0% (0/1) | none | none | none | 0.0% (0/2) | 1100 |
| 018 | 2 | 0 | 100.0% (2/2) | 100.0% (2/2) | none | none | none | none | 0.0% (0/2) | 852 |
| 019 | 2 | 0 | 100.0% (2/2) | 100.0% (2/2) | none | none | none | none | 0.0% (0/2) | 836 |
| 02 | 2 | 0 | 100.0% (2/2) | 33.3% (2/6) | 0.0% (0/2) | none | none | none | 0.0% (0/2) | 2676 |
| 020 | 2 | 0 | 100.0% (1/1) | 50.0% (2/4) | none | none | none | none | 0.0% (0/2) | 1511 |
| 021 | 1 | 0 | 100.0% (1/1) | none | none | none | none | none | 0.0% (0/1) | 1255 |
| 022 | 1 | 0 | 100.0% (1/1) | 100.0% (3/3) | none | none | none | none | 0.0% (0/1) | 7 |
| 023 | 1 | 0 | 100.0% (1/1) | 100.0% (2/2) | none | none | none | none | 0.0% (0/1) | 7 |
| 024 | 1 | 0 | 100.0% (1/1) | 100.0% (2/2) | none | none | none | none | 0.0% (0/1) | 6 |
| 025 | 1 | 0 | 100.0% (1/1) | 100.0% (2/2) | none | none | none | none | 0.0% (0/1) | 5 |
| 026 | 1 | 0 | 100.0% (1/1) | 100.0% (2/2) | none | none | none | none | 0.0% (0/1) | 7 |
| 027 | 1 | 0 | 100.0% (1/1) | 100.0% (2/2) | none | none | none | none | 0.0% (0/1) | 8 |
| 028 | 1 | 0 | 100.0% (1/1) | 100.0% (2/2) | none | none | none | none | 0.0% (0/1) | 10 |
| 029 | 1 | 0 | 100.0% (1/1) | 100.0% (1/1) | none | none | none | none | 0.0% (0/1) | 6 |
| 03 | 2 | 0 | 50.0% (1/2) | 0.0% (0/10) | 0.0% (0/1) | 0.0% (0/1) | none | none | 50.0% (1/2) | 14116 |
| 030 | 1 | 0 | 100.0% (1/1) | 100.0% (3/3) | none | none | none | none | 0.0% (0/1) | 6 |
| 031 | 1 | 0 | 100.0% (1/1) | 50.0% (1/2) | none | none | none | none | 0.0% (0/1) | 3 |
| 032 | 1 | 0 | 100.0% (1/1) | 100.0% (2/2) | none | none | none | none | 0.0% (0/1) | 435 |
| 033 | 1 | 0 | 100.0% (1/1) | 100.0% (2/2) | none | none | none | none | 0.0% (0/1) | 407 |
| 034 | 1 | 0 | 100.0% (1/1) | 100.0% (3/3) | none | none | none | none | 0.0% (0/1) | 6 |
| 035 | 1 | 0 | 100.0% (1/1) | 0.0% (0/1) | none | none | none | none | 0.0% (0/1) | 7 |
| 036 | 1 | 0 | 100.0% (1/1) | 100.0% (2/2) | none | none | none | none | 0.0% (0/1) | 438 |
| 037 | 1 | 0 | 100.0% (1/1) | 100.0% (1/1) | none | none | none | none | 0.0% (0/1) | 422 |
| 038 | 1 | 0 | 100.0% (1/1) | 100.0% (2/2) | none | none | none | none | 0.0% (0/1) | 423 |
| 039 | 1 | 0 | 100.0% (1/1) | 0.0% (0/1) | none | none | none | none | 0.0% (0/1) | 7 |
| 04 | 2 | 0 | 100.0% (2/2) | 0.0% (0/11) | 0.0% (0/2) | none | none | none | 0.0% (0/2) | 17948 |
| 040 | 1 | 0 | 100.0% (1/1) | 100.0% (2/2) | none | none | none | none | 0.0% (0/1) | 443 |
| 041 | 1 | 0 | 100.0% (1/1) | 100.0% (2/2) | none | none | none | none | 0.0% (0/1) | 5 |
| 042 | 1 | 0 | 100.0% (1/1) | 100.0% (3/3) | none | none | none | none | 0.0% (0/1) | 5 |
| 043 | 1 | 0 | 100.0% (1/1) | 100.0% (2/2) | none | none | none | none | 0.0% (0/1) | 6 |
| 044 | 1 | 0 | 100.0% (1/1) | 100.0% (1/1) | none | none | none | none | 0.0% (0/1) | 6 |
| 045 | 1 | 0 | 100.0% (1/1) | 100.0% (2/2) | none | none | none | none | 0.0% (0/1) | 421 |
| 046 | 1 | 0 | 100.0% (1/1) | 100.0% (2/2) | none | none | none | none | 0.0% (0/1) | 6 |
| 047 | 1 | 0 | 100.0% (1/1) | 0.0% (0/1) | none | none | none | none | 0.0% (0/1) | 6 |
| 048 | 1 | 0 | 100.0% (1/1) | 50.0% (1/2) | none | none | none | none | 0.0% (0/1) | 449 |
| 049 | 1 | 0 | 100.0% (1/1) | 100.0% (1/1) | none | none | none | none | 0.0% (0/1) | 5 |
| 05 | 2 | 0 | 100.0% (1/1) | 45.5% (5/11) | none | none | none | none | 0.0% (0/2) | 6162 |
| 050 | 1 | 0 | 100.0% (1/1) | 100.0% (2/2) | none | none | none | none | 0.0% (0/1) | 425 |
| 051 | 1 | 0 | 100.0% (1/1) | 100.0% (3/3) | none | none | none | none | 0.0% (0/1) | 5 |
| 052 | 1 | 0 | 100.0% (1/1) | 100.0% (2/2) | none | none | none | none | 0.0% (0/1) | 422 |
| 053 | 1 | 0 | 100.0% (1/1) | 100.0% (1/1) | none | none | none | none | 0.0% (0/1) | 4 |
| 054 | 1 | 0 | 100.0% (1/1) | 100.0% (2/2) | none | none | none | none | 0.0% (0/1) | 6 |
| 055 | 1 | 0 | 100.0% (1/1) | 100.0% (3/3) | none | none | none | none | 0.0% (0/1) | 6 |
| 056 | 1 | 0 | 100.0% (1/1) | 0.0% (0/1) | none | none | none | none | 0.0% (0/1) | 7 |
| 057 | 1 | 0 | 100.0% (1/1) | 100.0% (2/2) | none | none | none | none | 0.0% (0/1) | 423 |
| 058 | 1 | 0 | 100.0% (1/1) | 100.0% (2/2) | none | none | none | none | 0.0% (0/1) | 399 |
| 059 | 1 | 0 | 100.0% (1/1) | 100.0% (2/2) | none | none | none | none | 0.0% (0/1) | 4 |
| 06 | 2 | 0 | 100.0% (2/2) | 54.5% (6/11) | 0.0% (0/3) | none | none | none | 0.0% (0/2) | 2898 |
| 060 | 1 | 0 | 100.0% (1/1) | 100.0% (2/2) | none | none | none | none | 0.0% (0/1) | 437 |
| 07 | 2 | 0 | 100.0% (2/2) | 7.1% (1/14) | none | none | none | none | 0.0% (0/2) | 10617 |
| 08 | 2 | 0 | 0.0% (0/2) | 0.0% (0/5) | 0.0% (0/1) | none | none | none | 0.0% (0/2) | 2382 |
| 09 | 2 | 0 | 100.0% (2/2) | 100.0% (2/2) | none | none | none | none | 0.0% (0/2) | 3286 |
| 1 | 2 | 0 | 50.0% (1/2) | 40.0% (2/5) | 0.0% (0/1) | none | none | none | 0.0% (0/2) | 3714 |
| 10 | 2 | 0 | 100.0% (2/2) | 33.3% (3/9) | none | none | none | none | 0.0% (0/2) | 3273 |
| 11 | 2 | 0 | 100.0% (2/2) | 25.0% (2/8) | none | none | none | none | 0.0% (0/2) | 8477 |
| 12 | 2 | 0 | 100.0% (2/2) | 0.0% (0/6) | 0.0% (0/2) | 0.0% (0/1) | none | none | 0.0% (0/2) | 4261 |
| 13 | 2 | 0 | 100.0% (2/2) | 60.0% (6/10) | 0.0% (0/2) | none | none | none | 0.0% (0/2) | 5386 |
| 14 | 2 | 0 | 100.0% (2/2) | 100.0% (5/5) | none | none | none | none | 0.0% (0/2) | 3170 |
| 15 | 2 | 0 | 100.0% (2/2) | 44.4% (4/9) | none | none | none | none | 0.0% (0/2) | 10907 |
| 2 | 2 | 0 | 50.0% (1/2) | 16.7% (2/12) | 0.0% (0/2) | 100.0% (1/1) | none | none | 0.0% (0/2) | 15309 |
| 3 | 2 | 0 | none | 75.0% (3/4) | none | none | none | none | 0.0% (0/2) | 2262 |
| 4 | 2 | 0 | 50.0% (1/2) | 54.5% (6/11) | 0.0% (0/1) | none | none | none | 0.0% (0/2) | 3573 |
| 5 | 2 | 0 | 100.0% (2/2) | 50.0% (2/4) | none | none | none | none | 0.0% (0/2) | 1862 |
| beat | 10 | 0 | 100.0% (8/8) | 6.7% (2/30) | none | none | none | none | 0.0% (0/10) | 3274 |
| brief | 10 | 0 | 90.0% (9/10) | 0.0% (0/50) | 0.0% (0/1) | 0.0% (0/2) | 100.0% (10/10) | none | 10.0% (1/10) | 3045 |
| brk | 10 | 0 | 100.0% (9/9) | 0.0% (0/13) | 0.0% (0/4) | none | none | none | 10.0% (1/10) | 2177 |
| budget | 10 | 0 | 100.0% (8/8) | 83.3% (10/12) | none | none | none | none | 0.0% (0/10) | 1461 |
| cast | 10 | 0 | 100.0% (4/4) | 46.2% (6/13) | none | none | none | 100.0% (1/1) | 0.0% (0/10) | 1772 |
| cat | 3 | 0 | 100.0% (3/3) | none | none | none | none | none | 0.0% (0/3) | 1773 |
| char | 10 | 0 | 100.0% (10/10) | 4.8% (4/83) | none | none | 100.0% (10/10) | none | 10.0% (1/10) | 4797 |
| chars | 10 | 0 | 100.0% (10/10) | 8.7% (4/46) | none | none | none | none | 0.0% (0/10) | 1683 |
| cs | 10 | 0 | 100.0% (8/8) | 58.3% (7/12) | none | none | none | 100.0% (1/1) | 0.0% (0/10) | 2689 |
| daily | 10 | 0 | 100.0% (7/7) | 37.5% (9/24) | none | 100.0% (3/3) | none | none | 0.0% (0/10) | 2334 |
| dood | 10 | 0 | 100.0% (9/9) | 33.3% (8/24) | none | 100.0% (1/1) | none | none | 0.0% (0/10) | 2097 |
| files | 10 | 0 | 88.9% (8/9) | 75.0% (3/4) | none | none | none | none | 0.0% (0/10) | 1479 |
| guide | 10 | 0 | 90.0% (9/10) | 74.5% (38/51) | none | none | none | none | 0.0% (0/10) | 2722 |
| loc | 10 | 0 | 100.0% (8/8) | 22.2% (4/18) | none | none | none | none | 0.0% (0/10) | 1576 |
| mood | 10 | 0 | 100.0% (9/9) | 23.1% (3/13) | none | 100.0% (2/2) | none | none | 0.0% (0/10) | 3080 |
| notes | 10 | 0 | 100.0% (9/9) | 42.9% (6/14) | none | 100.0% (1/1) | none | none | 0.0% (0/10) | 1354 |
| outline | 2 | 0 | none | 7.7% (1/13) | none | none | none | 0.0% (0/2) | 0.0% (0/2) | 23175 |
| ovr | 10 | 0 | 100.0% (10/10) | 100.0% (1/1) | none | none | none | none | 0.0% (0/10) | 453 |
| rel | 10 | 0 | none | 1.1% (1/92) | none | none | 100.0% (10/10) | none | 0.0% (0/10) | 4864 |
| reports | 2 | 0 | none | 33.3% (1/3) | none | none | none | 50.0% (1/2) | 0.0% (0/2) | 1825 |
| scenes | 10 | 0 | 88.9% (8/9) | 0.0% (0/25) | 0.0% (0/9) | none | none | none | 0.0% (0/10) | 2767 |
| sched | 10 | 0 | 87.5% (7/8) | 33.3% (2/6) | 0.0% (0/1) | none | none | none | 0.0% (0/10) | 1411 |
| settings | 8 | 0 | 87.5% (7/8) | 84.6% (33/39) | none | none | none | none | 0.0% (0/8) | 8022 |
| shots | 10 | 0 | 100.0% (10/10) | 40.0% (4/10) | 0.0% (0/2) | none | none | none | 0.0% (0/10) | 1371 |
| sides | 2 | 0 | none | 0.0% (0/2) | none | none | none | 100.0% (2/2) | 0.0% (0/2) | 413 |
| spage | 2 | 0 | none | 0.0% (0/2) | none | none | none | 100.0% (2/2) | 0.0% (0/2) | 394 |
| story | 20 | 0 | 100.0% (18/18) | 14.5% (12/83) | none | 100.0% (1/1) | 90.0% (9/10) | none | 15.0% (3/20) | 6690 |
| tasks | 10 | 0 | 100.0% (10/10) | 52.4% (11/21) | 0.0% (0/1) | 100.0% (5/5) | none | none | 0.0% (0/10) | 2076 |
| timeline | 2 | 0 | none | 100.0% (2/2) | 0.0% (0/2) | none | none | 50.0% (1/2) | 0.0% (0/2) | 1514 |
| vault | 10 | 0 | 100.0% (10/10) | 47.8% (11/23) | none | 100.0% (8/8) | none | none | 0.0% (0/10) | 1947 |
| vglobal | 2 | 0 | none | 50.0% (1/2) | none | none | none | 100.0% (2/2) | 0.0% (0/2) | 4 |

Expectation flags (harness spec 7.4). Cases that set none of them show none. Flags the build cannot measure yet are listed as not counted, never as passed or failed.

| Question type | Required facts | Abstain names | No navigation | No data dump | No internal words | Length class | No re-ask | Answer type | Answer group | Not counted |
|---|---|---|---|---|---|---|---|---|---|---|
| all | 89.6% (173/193) | 66.7% (10/15) | 100.0% (42/42) | none | 100.0% (393/393) | 93.9% (369/393) | 100.0% (20/20) | 78.6% (309/393) | 91.3% (359/393) | 0 |
| 001 | none | 100.0% (1/1) | none | none | 100.0% (3/3) | 100.0% (3/3) | 100.0% (1/1) | 100.0% (3/3) | 100.0% (3/3) | 0 |
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
| 11 | none | none | none | none | 100.0% (2/2) | 50.0% (1/2) | none | 0.0% (0/2) | 100.0% (2/2) | 0 |
| 12 | 100.0% (1/1) | none | none | none | 100.0% (2/2) | 100.0% (2/2) | none | 100.0% (2/2) | 100.0% (2/2) | 0 |
| 13 | 100.0% (2/2) | none | none | none | 100.0% (2/2) | 100.0% (2/2) | none | 50.0% (1/2) | 100.0% (2/2) | 0 |
| 14 | none | none | none | none | 100.0% (2/2) | 100.0% (2/2) | none | 100.0% (2/2) | 100.0% (2/2) | 0 |
| 15 | none | none | none | none | 100.0% (2/2) | 100.0% (2/2) | none | 0.0% (0/2) | 100.0% (2/2) | 0 |
| 2 | 100.0% (3/3) | none | none | none | 100.0% (2/2) | 100.0% (2/2) | none | 100.0% (2/2) | 100.0% (2/2) | 0 |
| 3 | 100.0% (4/4) | none | none | none | 100.0% (2/2) | 100.0% (2/2) | none | 100.0% (2/2) | 100.0% (2/2) | 0 |
| 4 | 100.0% (1/1) | none | none | none | 100.0% (2/2) | 100.0% (2/2) | none | 100.0% (2/2) | 100.0% (2/2) | 0 |
| 5 | 100.0% (2/2) | none | none | none | 100.0% (2/2) | 100.0% (2/2) | none | 0.0% (0/2) | 0.0% (0/2) | 0 |
| beat | 71.4% (5/7) | none | none | none | 100.0% (10/10) | 100.0% (10/10) | none | 70.0% (7/10) | 90.0% (9/10) | 0 |
| brief | 100.0% (25/25) | none | 100.0% (10/10) | none | 100.0% (10/10) | 100.0% (10/10) | none | 100.0% (10/10) | 100.0% (10/10) | 0 |
| brk | 100.0% (2/2) | none | none | none | 100.0% (10/10) | 90.0% (9/10) | none | 80.0% (8/10) | 100.0% (10/10) | 0 |
| budget | 100.0% (6/6) | none | none | none | 100.0% (10/10) | 100.0% (10/10) | none | 50.0% (5/10) | 100.0% (10/10) | 0 |
| cast | 100.0% (5/5) | 100.0% (1/1) | none | none | 100.0% (10/10) | 90.0% (9/10) | none | 90.0% (9/10) | 100.0% (10/10) | 0 |
| cat | none | none | none | none | 100.0% (3/3) | 100.0% (3/3) | none | 100.0% (3/3) | 100.0% (3/3) | 0 |
| char | 100.0% (14/14) | none | 100.0% (10/10) | none | 100.0% (10/10) | 100.0% (10/10) | none | 100.0% (10/10) | 100.0% (10/10) | 0 |
| chars | none | none | none | none | 100.0% (10/10) | 70.0% (7/10) | none | 50.0% (5/10) | 50.0% (5/10) | 0 |
| cs | 0.0% (0/4) | 100.0% (1/1) | none | none | 100.0% (10/10) | 100.0% (10/10) | none | 90.0% (9/10) | 100.0% (10/10) | 0 |
| daily | 80.0% (4/5) | none | none | none | 100.0% (10/10) | 100.0% (10/10) | none | 90.0% (9/10) | 100.0% (10/10) | 0 |
| dood | 50.0% (1/2) | none | none | none | 100.0% (10/10) | 90.0% (9/10) | none | 90.0% (9/10) | 100.0% (10/10) | 0 |
| files | 100.0% (2/2) | none | none | none | 100.0% (10/10) | 100.0% (10/10) | none | 90.0% (9/10) | 100.0% (10/10) | 0 |
| guide | none | none | none | none | 100.0% (10/10) | 90.0% (9/10) | none | 80.0% (8/10) | 80.0% (8/10) | 0 |
| loc | 100.0% (4/4) | none | none | none | 100.0% (10/10) | 100.0% (10/10) | none | 100.0% (10/10) | 100.0% (10/10) | 0 |
| mood | 100.0% (1/1) | none | none | none | 100.0% (10/10) | 100.0% (10/10) | none | 90.0% (9/10) | 90.0% (9/10) | 0 |
| notes | 0.0% (0/1) | none | none | none | 100.0% (10/10) | 90.0% (9/10) | none | 80.0% (8/10) | 80.0% (8/10) | 0 |
| outline | none | 0.0% (0/2) | none | none | 100.0% (2/2) | 50.0% (1/2) | none | 0.0% (0/2) | 50.0% (1/2) | 0 |
| ovr | none | none | none | none | 100.0% (10/10) | 100.0% (10/10) | none | 100.0% (10/10) | 100.0% (10/10) | 0 |
| rel | 100.0% (30/30) | none | 100.0% (10/10) | none | 100.0% (10/10) | 100.0% (10/10) | none | 100.0% (10/10) | 100.0% (10/10) | 0 |
| reports | none | 50.0% (1/2) | none | none | 100.0% (2/2) | 100.0% (2/2) | none | 100.0% (2/2) | 100.0% (2/2) | 0 |
| scenes | 100.0% (7/7) | none | 100.0% (2/2) | none | 100.0% (10/10) | 80.0% (8/10) | none | 60.0% (6/10) | 80.0% (8/10) | 0 |
| sched | 100.0% (3/3) | none | none | none | 100.0% (10/10) | 100.0% (10/10) | none | 80.0% (8/10) | 100.0% (10/10) | 0 |
| settings | none | none | none | none | 100.0% (8/8) | 62.5% (5/8) | none | 100.0% (8/8) | 100.0% (8/8) | 0 |
| shots | none | none | none | none | 100.0% (10/10) | 100.0% (10/10) | none | 100.0% (10/10) | 100.0% (10/10) | 0 |
| sides | none | 100.0% (2/2) | none | none | 100.0% (2/2) | 100.0% (2/2) | none | 50.0% (1/2) | 100.0% (2/2) | 0 |
| spage | none | 100.0% (2/2) | none | none | 100.0% (2/2) | 100.0% (2/2) | none | 50.0% (1/2) | 50.0% (1/2) | 0 |
| story | 77.4% (24/31) | none | 100.0% (10/10) | none | 100.0% (20/20) | 75.0% (15/20) | none | 80.0% (16/20) | 90.0% (18/20) | 0 |
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
| Median, all questions (393 asked) | 1.8 s | 15.0 s or lower: met |
| 95th percentile, all questions | 10.6 s | 30.0 s or lower: met |
| Median, questions that used the engine (347) | 2.2 s | |
| 95th percentile, same | 10.7 s | |
| First question after the project opened | 4.3 s | |
| First question that used the engine | 4.3 s | |
| Prompt tokens served from stored work | 51.5% (336895 of 653978) | |
| Completion tokens, engine calls | 37855, 715 | |

| Length class | Questions | Median | 95th percentile |
|---|---|---|---|
| narrative | 45 | 4.3 s | 13.9 s |
| one_sentence | 228 | 1.4 s | 3.5 s |
| paragraph | 35 | 3.1 s | 15.3 s |
| short | 73 | 2.4 s | 8.5 s |
| story | 12 | 11.4 s | 27.2 s |

| Stage | Mean per question |
|---|---|
| dossier | 6 ms |
| engine_generation | 2474 ms |
| engine_prompt | 931 ms |
| turn | 140 ms |

Stage times come from the AI trace and from what the request recorded; a stage nobody records is not listed. Engine prompt and generation are the engine's own figures and include the earlier turns a conversation case asks first, so they add up to more than the time of the last question.
