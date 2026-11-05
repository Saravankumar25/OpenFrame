# AI evaluation report

- Cases: 393
- Gold folder: crates/straft-ai-eval/gold
- Model files: the pinned Straft AI of packaging/ai/ai-assets.lock.json
- Build: release
- Search model: Automatic
- Engine place: auto
- Sampling: deterministic (temperature 0, fixed seed)
- Wall time: 19.7 min

## English release bar (ADR-0028 decision 2)

| Measure | Result | Bar |
|---|---|---|
| Correct tool and arguments (tool and action cases) | none | 90% |
| Correct end state after apply (action cases) | none | 85% |

Bar met: **no**

## By kind

| kind | Cases | Setup errors | Right tool offered | Tool chosen | Tool and arguments | Schema valid | End state | Answer facts | Language | Length | Safety | Not empty | Grounding | Recall@5 | Recall@10 | MRR | Median ms | p95 ms | Engine MB |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| conversation | 393 | 0 | none | none | none | 100.0% (393/393) | none | 96.4% (324/336) | 100.0% (393/393) | 97.5% (39/40) | none | 100.0% (393/393) | 65.7% | none | none | none | 1682 | 10087 | 2465 |

## By language

| language | Cases | Setup errors | Right tool offered | Tool chosen | Tool and arguments | Schema valid | End state | Answer facts | Language | Length | Safety | Not empty | Grounding | Recall@5 | Recall@10 | MRR | Median ms | p95 ms | Engine MB |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| en | 393 | 0 | none | none | none | 100.0% (393/393) | none | 96.4% (324/336) | 100.0% (393/393) | 97.5% (39/40) | none | 100.0% (393/393) | 65.7% | none | none | none | 1682 | 10087 | 2465 |

## Answer path and fallback

Cases by kind and the answer path they took.

| Kind | write | navigate | change | fixed | Cases |
|---|---|---|---|---|---|
| conversation | 328 | 0 | 0 | 65 | 393 |
| all | 328 | 0 | 0 | 65 | 393 |

Cases by kind and the fallback they used. `last_resort` is the last-resort answer; `abstained` is a could-not-find answer.

| Kind | none | last_resort | abstained | Cases |
|---|---|---|---|---|
| conversation | 363 | 25 | 5 | 393 |
| all | 363 | 25 | 5 | 393 |

## Conversation bench

Deterministic checks only; the rubric and human labels come at development time. Invented names, wrong scenes and missing quotes are rates over all names, scene citations and quotes in the answers.

| Question type | Cases | Setup errors | Answer facts | Invented names | Wrong scene citations | Missing quotes | Within length | Correct abstention | Repetition | Median ms |
|---|---|---|---|---|---|---|---|---|---|---|
| all | 393 | 0 | 96.4% (324/336) | 37.5% (401/1070) | 0.0% (0/44) | 74.1% (20/27) | 97.5% (39/40) | 73.3% (11/15) | 0.8% (3/393) | 1682 |
| 001 | 3 | 0 | 100.0% (2/2) | 66.7% (4/6) | none | none | none | 100.0% (1/1) | 0.0% (0/3) | 1062 |
| 002 | 3 | 0 | 100.0% (3/3) | 75.0% (6/8) | none | none | none | none | 0.0% (0/3) | 2136 |
| 003 | 3 | 0 | 100.0% (2/2) | 87.5% (7/8) | none | none | none | none | 0.0% (0/3) | 760 |
| 004 | 3 | 0 | 100.0% (3/3) | 100.0% (5/5) | none | none | none | none | 0.0% (0/3) | 1889 |
| 005 | 3 | 0 | 100.0% (3/3) | 100.0% (4/4) | 0.0% (0/1) | none | none | none | 0.0% (0/3) | 719 |
| 006 | 3 | 0 | 100.0% (3/3) | 37.9% (11/29) | none | none | none | none | 0.0% (0/3) | 2583 |
| 007 | 3 | 0 | 100.0% (2/2) | 83.3% (10/12) | none | none | none | none | 0.0% (0/3) | 1908 |
| 008 | 3 | 0 | 66.7% (2/3) | 56.2% (9/16) | none | 0.0% (0/4) | none | none | 0.0% (0/3) | 3866 |
| 009 | 3 | 0 | 100.0% (3/3) | 66.7% (4/6) | none | none | none | none | 0.0% (0/3) | 1009 |
| 01 | 2 | 0 | 100.0% (2/2) | 25.0% (1/4) | 0.0% (0/4) | none | none | none | 0.0% (0/2) | 6177 |
| 010 | 3 | 0 | 100.0% (2/2) | 33.3% (2/6) | none | none | none | none | 0.0% (0/3) | 1350 |
| 011 | 2 | 0 | 100.0% (2/2) | 20.0% (1/5) | none | none | none | none | 0.0% (0/2) | 7225 |
| 012 | 2 | 0 | 100.0% (2/2) | 60.0% (3/5) | none | none | none | none | 0.0% (0/2) | 1164 |
| 013 | 2 | 0 | 50.0% (1/2) | 33.3% (1/3) | none | none | none | none | 0.0% (0/2) | 801 |
| 014 | 2 | 0 | 100.0% (2/2) | 66.7% (2/3) | 0.0% (0/1) | none | none | none | 0.0% (0/2) | 1844 |
| 015 | 2 | 0 | 100.0% (2/2) | 100.0% (2/2) | none | none | none | none | 0.0% (0/2) | 754 |
| 016 | 2 | 0 | 100.0% (2/2) | 100.0% (3/3) | none | none | none | none | 0.0% (0/2) | 4626 |
| 017 | 2 | 0 | 100.0% (2/2) | 100.0% (3/3) | 0.0% (0/1) | none | none | none | 0.0% (0/2) | 1146 |
| 018 | 2 | 0 | 100.0% (2/2) | 100.0% (2/2) | none | none | none | none | 0.0% (0/2) | 1014 |
| 019 | 2 | 0 | 100.0% (2/2) | 100.0% (2/2) | none | none | none | none | 0.0% (0/2) | 885 |
| 02 | 2 | 0 | 100.0% (2/2) | 33.3% (2/6) | 0.0% (0/2) | none | none | none | 0.0% (0/2) | 2789 |
| 020 | 2 | 0 | 100.0% (1/1) | 50.0% (2/4) | none | none | none | none | 0.0% (0/2) | 1539 |
| 021 | 1 | 0 | 100.0% (1/1) | none | none | none | none | none | 0.0% (0/1) | 1312 |
| 022 | 1 | 0 | 100.0% (1/1) | 100.0% (3/3) | none | none | none | none | 0.0% (0/1) | 8 |
| 023 | 1 | 0 | 100.0% (1/1) | 100.0% (2/2) | none | none | none | none | 0.0% (0/1) | 7 |
| 024 | 1 | 0 | 100.0% (1/1) | 100.0% (2/2) | none | none | none | none | 0.0% (0/1) | 6 |
| 025 | 1 | 0 | 100.0% (1/1) | 100.0% (2/2) | none | none | none | none | 0.0% (0/1) | 5 |
| 026 | 1 | 0 | 100.0% (1/1) | 100.0% (2/2) | none | none | none | none | 0.0% (0/1) | 8 |
| 027 | 1 | 0 | 100.0% (1/1) | 100.0% (2/2) | none | none | none | none | 0.0% (0/1) | 11 |
| 028 | 1 | 0 | 100.0% (1/1) | 100.0% (2/2) | none | none | none | none | 0.0% (0/1) | 10 |
| 029 | 1 | 0 | 100.0% (1/1) | 100.0% (1/1) | none | none | none | none | 0.0% (0/1) | 7 |
| 03 | 2 | 0 | 50.0% (1/2) | 0.0% (0/11) | 0.0% (0/1) | 0.0% (0/1) | none | none | 50.0% (1/2) | 15388 |
| 030 | 1 | 0 | 100.0% (1/1) | 100.0% (3/3) | none | none | none | none | 0.0% (0/1) | 6 |
| 031 | 1 | 0 | 100.0% (1/1) | 50.0% (1/2) | none | none | none | none | 0.0% (0/1) | 3 |
| 032 | 1 | 0 | 100.0% (1/1) | 100.0% (2/2) | none | none | none | none | 0.0% (0/1) | 451 |
| 033 | 1 | 0 | 100.0% (1/1) | 100.0% (2/2) | none | none | none | none | 0.0% (0/1) | 414 |
| 034 | 1 | 0 | 100.0% (1/1) | 100.0% (3/3) | none | none | none | none | 0.0% (0/1) | 7 |
| 035 | 1 | 0 | 100.0% (1/1) | 0.0% (0/1) | none | none | none | none | 0.0% (0/1) | 7 |
| 036 | 1 | 0 | 100.0% (1/1) | 100.0% (2/2) | none | none | none | none | 0.0% (0/1) | 444 |
| 037 | 1 | 0 | 100.0% (1/1) | 100.0% (1/1) | none | none | none | none | 0.0% (0/1) | 436 |
| 038 | 1 | 0 | 100.0% (1/1) | 100.0% (2/2) | none | none | none | none | 0.0% (0/1) | 444 |
| 039 | 1 | 0 | 100.0% (1/1) | 0.0% (0/1) | none | none | none | none | 0.0% (0/1) | 7 |
| 04 | 2 | 0 | 100.0% (2/2) | 11.1% (1/9) | 0.0% (0/2) | none | none | none | 0.0% (0/2) | 12111 |
| 040 | 1 | 0 | 100.0% (1/1) | 100.0% (2/2) | none | none | none | none | 0.0% (0/1) | 433 |
| 041 | 1 | 0 | 100.0% (1/1) | 100.0% (2/2) | none | none | none | none | 0.0% (0/1) | 6 |
| 042 | 1 | 0 | 100.0% (1/1) | 100.0% (3/3) | none | none | none | none | 0.0% (0/1) | 5 |
| 043 | 1 | 0 | 100.0% (1/1) | 100.0% (2/2) | none | none | none | none | 0.0% (0/1) | 5 |
| 044 | 1 | 0 | 100.0% (1/1) | 100.0% (1/1) | none | none | none | none | 0.0% (0/1) | 5 |
| 045 | 1 | 0 | 100.0% (1/1) | 100.0% (2/2) | none | none | none | none | 0.0% (0/1) | 458 |
| 046 | 1 | 0 | 100.0% (1/1) | 100.0% (2/2) | none | none | none | none | 0.0% (0/1) | 6 |
| 047 | 1 | 0 | 100.0% (1/1) | 0.0% (0/1) | none | none | none | none | 0.0% (0/1) | 6 |
| 048 | 1 | 0 | 100.0% (1/1) | 50.0% (1/2) | none | none | none | none | 0.0% (0/1) | 436 |
| 049 | 1 | 0 | 100.0% (1/1) | 100.0% (1/1) | none | none | none | none | 0.0% (0/1) | 6 |
| 05 | 2 | 0 | 100.0% (1/1) | 44.4% (4/9) | none | none | none | none | 0.0% (0/2) | 7088 |
| 050 | 1 | 0 | 100.0% (1/1) | 100.0% (2/2) | none | none | none | none | 0.0% (0/1) | 452 |
| 051 | 1 | 0 | 100.0% (1/1) | 100.0% (3/3) | none | none | none | none | 0.0% (0/1) | 6 |
| 052 | 1 | 0 | 100.0% (1/1) | 100.0% (2/2) | none | none | none | none | 0.0% (0/1) | 453 |
| 053 | 1 | 0 | 100.0% (1/1) | 100.0% (1/1) | none | none | none | none | 0.0% (0/1) | 5 |
| 054 | 1 | 0 | 100.0% (1/1) | 100.0% (2/2) | none | none | none | none | 0.0% (0/1) | 6 |
| 055 | 1 | 0 | 100.0% (1/1) | 100.0% (3/3) | none | none | none | none | 0.0% (0/1) | 6 |
| 056 | 1 | 0 | 100.0% (1/1) | 0.0% (0/1) | none | none | none | none | 0.0% (0/1) | 7 |
| 057 | 1 | 0 | 100.0% (1/1) | 100.0% (2/2) | none | none | none | none | 0.0% (0/1) | 442 |
| 058 | 1 | 0 | 100.0% (1/1) | 100.0% (2/2) | none | none | none | none | 0.0% (0/1) | 418 |
| 059 | 1 | 0 | 100.0% (1/1) | 100.0% (2/2) | none | none | none | none | 0.0% (0/1) | 4 |
| 06 | 2 | 0 | 100.0% (2/2) | 54.5% (6/11) | 0.0% (0/3) | none | none | none | 0.0% (0/2) | 3043 |
| 060 | 1 | 0 | 100.0% (1/1) | 100.0% (2/2) | none | none | none | none | 0.0% (0/1) | 429 |
| 07 | 2 | 0 | 100.0% (2/2) | 22.2% (4/18) | none | none | none | none | 0.0% (0/2) | 11572 |
| 08 | 2 | 0 | 0.0% (0/2) | 0.0% (0/5) | 0.0% (0/1) | none | none | none | 0.0% (0/2) | 2453 |
| 09 | 2 | 0 | 100.0% (2/2) | 100.0% (3/3) | none | none | none | none | 0.0% (0/2) | 3491 |
| 1 | 2 | 0 | 50.0% (1/2) | 40.0% (2/5) | 0.0% (0/1) | none | none | none | 0.0% (0/2) | 3954 |
| 10 | 2 | 0 | 100.0% (2/2) | 33.3% (3/9) | none | none | none | none | 0.0% (0/2) | 3494 |
| 11 | 2 | 0 | 100.0% (2/2) | 0.0% (0/2) | none | none | none | none | 0.0% (0/2) | 8223 |
| 12 | 2 | 0 | 100.0% (2/2) | 0.0% (0/5) | 0.0% (0/2) | 0.0% (0/1) | none | none | 0.0% (0/2) | 12082 |
| 13 | 2 | 0 | 100.0% (2/2) | 63.6% (7/11) | 0.0% (0/2) | none | none | none | 0.0% (0/2) | 6551 |
| 14 | 2 | 0 | 100.0% (2/2) | 100.0% (5/5) | none | none | none | none | 0.0% (0/2) | 3167 |
| 15 | 2 | 0 | 100.0% (2/2) | 66.7% (4/6) | none | none | none | none | 0.0% (0/2) | 8137 |
| 2 | 2 | 0 | 100.0% (2/2) | 18.2% (2/11) | 0.0% (0/2) | none | none | none | 0.0% (0/2) | 15337 |
| 3 | 2 | 0 | none | 75.0% (3/4) | none | none | none | none | 0.0% (0/2) | 2347 |
| 4 | 2 | 0 | 100.0% (2/2) | 66.7% (8/12) | 0.0% (0/1) | none | none | none | 0.0% (0/2) | 4139 |
| 5 | 2 | 0 | 100.0% (2/2) | 50.0% (2/4) | none | none | none | none | 0.0% (0/2) | 1937 |
| beat | 10 | 0 | 100.0% (8/8) | 11.1% (3/27) | none | none | none | none | 0.0% (0/10) | 3305 |
| brief | 10 | 0 | 70.0% (7/10) | 0.0% (0/42) | none | none | 100.0% (10/10) | none | 0.0% (0/10) | 3739 |
| brk | 10 | 0 | 100.0% (9/9) | 5.9% (1/17) | 0.0% (0/3) | none | none | none | 0.0% (0/10) | 1889 |
| budget | 10 | 0 | 100.0% (8/8) | 68.4% (13/19) | none | 50.0% (1/2) | none | none | 0.0% (0/10) | 1592 |
| cast | 10 | 0 | 100.0% (4/4) | 50.0% (6/12) | none | none | none | 100.0% (1/1) | 0.0% (0/10) | 1082 |
| cat | 3 | 0 | 100.0% (3/3) | none | none | none | none | none | 0.0% (0/3) | 1214 |
| char | 10 | 0 | 100.0% (10/10) | 3.8% (3/78) | none | none | 100.0% (10/10) | none | 10.0% (1/10) | 5424 |
| chars | 10 | 0 | 100.0% (10/10) | 4.9% (2/41) | none | none | none | none | 0.0% (0/10) | 1407 |
| cs | 10 | 0 | 100.0% (8/8) | 58.3% (7/12) | none | none | none | 100.0% (1/1) | 0.0% (0/10) | 2068 |
| daily | 10 | 0 | 100.0% (7/7) | 50.0% (9/18) | none | 100.0% (3/3) | none | none | 0.0% (0/10) | 1629 |
| dood | 10 | 0 | 100.0% (9/9) | 42.9% (12/28) | none | 100.0% (1/1) | none | none | 0.0% (0/10) | 2135 |
| files | 10 | 0 | 100.0% (9/9) | 75.0% (3/4) | none | none | none | none | 0.0% (0/10) | 1455 |
| guide | 10 | 0 | 100.0% (10/10) | 79.6% (43/54) | none | 100.0% (1/1) | none | none | 0.0% (0/10) | 2704 |
| loc | 10 | 0 | 100.0% (8/8) | 17.6% (3/17) | none | none | none | none | 0.0% (0/10) | 1012 |
| mood | 10 | 0 | 100.0% (9/9) | 23.1% (3/13) | none | none | none | none | 0.0% (0/10) | 2234 |
| notes | 10 | 0 | 100.0% (9/9) | 50.0% (9/18) | none | 100.0% (1/1) | none | none | 0.0% (0/10) | 1357 |
| outline | 2 | 0 | none | 9.1% (1/11) | none | none | none | 0.0% (0/2) | 0.0% (0/2) | 17816 |
| ovr | 10 | 0 | 100.0% (10/10) | 100.0% (1/1) | none | none | none | none | 0.0% (0/10) | 356 |
| rel | 10 | 0 | none | 1.1% (1/89) | none | none | 100.0% (10/10) | none | 10.0% (1/10) | 6879 |
| reports | 2 | 0 | none | 0.0% (0/3) | none | none | none | 50.0% (1/2) | 0.0% (0/2) | 1788 |
| scenes | 10 | 0 | 77.8% (7/9) | 0.0% (0/27) | 0.0% (0/9) | none | none | none | 0.0% (0/10) | 2415 |
| sched | 10 | 0 | 87.5% (7/8) | 33.3% (2/6) | 0.0% (0/3) | none | none | none | 0.0% (0/10) | 1052 |
| settings | 8 | 0 | 100.0% (8/8) | 88.2% (30/34) | none | 100.0% (3/3) | none | none | 0.0% (0/8) | 7520 |
| shots | 10 | 0 | 100.0% (10/10) | 33.3% (3/9) | 0.0% (0/2) | none | none | none | 0.0% (0/10) | 1092 |
| sides | 2 | 0 | none | 0.0% (0/2) | none | none | none | 100.0% (2/2) | 0.0% (0/2) | 421 |
| spage | 2 | 0 | none | 0.0% (0/2) | none | none | none | 100.0% (2/2) | 0.0% (0/2) | 410 |
| story | 20 | 0 | 100.0% (18/18) | 17.1% (14/82) | none | 100.0% (2/2) | 90.0% (9/10) | none | 0.0% (0/20) | 7499 |
| tasks | 10 | 0 | 100.0% (10/10) | 50.0% (11/22) | 0.0% (0/1) | none | none | none | 0.0% (0/10) | 2192 |
| timeline | 2 | 0 | none | 100.0% (2/2) | 0.0% (0/2) | none | none | 50.0% (1/2) | 0.0% (0/2) | 1482 |
| vault | 10 | 0 | 100.0% (10/10) | 42.9% (9/21) | none | 100.0% (8/8) | none | none | 0.0% (0/10) | 1414 |
| vglobal | 2 | 0 | none | 50.0% (1/2) | none | none | none | 100.0% (2/2) | 0.0% (0/2) | 5 |

Expectation flags (harness spec 7.4). Cases that set none of them show none. Flags the build cannot measure yet are listed as not counted, never as passed or failed.

| Question type | Required facts | Abstain names | No navigation | No data dump | No internal words | Length class | No re-ask | Answer type | Answer group | Not counted |
|---|---|---|---|---|---|---|---|---|---|---|
| all | 89.6% (173/193) | 66.7% (10/15) | 100.0% (42/42) | none | 100.0% (393/393) | 93.4% (367/393) | 100.0% (20/20) | 78.6% (309/393) | 91.3% (359/393) | 0 |
| 001 | none | 100.0% (1/1) | none | none | 100.0% (3/3) | 100.0% (3/3) | 100.0% (1/1) | 100.0% (3/3) | 100.0% (3/3) | 0 |
| 002 | none | none | none | none | 100.0% (3/3) | 100.0% (3/3) | 100.0% (1/1) | 100.0% (3/3) | 100.0% (3/3) | 0 |
| 003 | 100.0% (1/1) | none | none | none | 100.0% (3/3) | 100.0% (3/3) | 100.0% (1/1) | 66.7% (2/3) | 100.0% (3/3) | 0 |
| 004 | none | none | none | none | 100.0% (3/3) | 100.0% (3/3) | 100.0% (1/1) | 66.7% (2/3) | 100.0% (3/3) | 0 |
| 005 | none | none | none | none | 100.0% (3/3) | 100.0% (3/3) | 100.0% (1/1) | 66.7% (2/3) | 100.0% (3/3) | 0 |
| 006 | none | none | none | none | 100.0% (3/3) | 100.0% (3/3) | 100.0% (1/1) | 33.3% (1/3) | 66.7% (2/3) | 0 |
| 007 | 100.0% (1/1) | none | none | none | 100.0% (3/3) | 100.0% (3/3) | 100.0% (1/1) | 66.7% (2/3) | 66.7% (2/3) | 0 |
| 008 | 0.0% (0/1) | none | none | none | 100.0% (3/3) | 100.0% (3/3) | 100.0% (1/1) | 66.7% (2/3) | 100.0% (3/3) | 0 |
| 009 | none | none | none | none | 100.0% (3/3) | 100.0% (3/3) | 100.0% (1/1) | 100.0% (3/3) | 100.0% (3/3) | 0 |
| 01 | 66.7% (2/3) | none | none | none | 100.0% (2/2) | 50.0% (1/2) | none | 100.0% (2/2) | 100.0% (2/2) | 0 |
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
| 07 | 100.0% (2/2) | none | none | none | 100.0% (2/2) | 50.0% (1/2) | none | 50.0% (1/2) | 50.0% (1/2) | 0 |
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
| beat | 71.4% (5/7) | none | none | none | 100.0% (10/10) | 90.0% (9/10) | none | 70.0% (7/10) | 90.0% (9/10) | 0 |
| brief | 100.0% (25/25) | none | 100.0% (10/10) | none | 100.0% (10/10) | 100.0% (10/10) | none | 100.0% (10/10) | 100.0% (10/10) | 0 |
| brk | 100.0% (2/2) | none | none | none | 100.0% (10/10) | 90.0% (9/10) | none | 80.0% (8/10) | 100.0% (10/10) | 0 |
| budget | 100.0% (6/6) | none | none | none | 100.0% (10/10) | 90.0% (9/10) | none | 50.0% (5/10) | 100.0% (10/10) | 0 |
| cast | 100.0% (5/5) | 100.0% (1/1) | none | none | 100.0% (10/10) | 100.0% (10/10) | none | 90.0% (9/10) | 100.0% (10/10) | 0 |
| cat | none | none | none | none | 100.0% (3/3) | 100.0% (3/3) | none | 100.0% (3/3) | 100.0% (3/3) | 0 |
| char | 100.0% (14/14) | none | 100.0% (10/10) | none | 100.0% (10/10) | 100.0% (10/10) | none | 100.0% (10/10) | 100.0% (10/10) | 0 |
| chars | none | none | none | none | 100.0% (10/10) | 70.0% (7/10) | none | 50.0% (5/10) | 50.0% (5/10) | 0 |
| cs | 0.0% (0/4) | 100.0% (1/1) | none | none | 100.0% (10/10) | 100.0% (10/10) | none | 90.0% (9/10) | 100.0% (10/10) | 0 |
| daily | 80.0% (4/5) | none | none | none | 100.0% (10/10) | 100.0% (10/10) | none | 90.0% (9/10) | 100.0% (10/10) | 0 |
| dood | 50.0% (1/2) | none | none | none | 100.0% (10/10) | 90.0% (9/10) | none | 90.0% (9/10) | 100.0% (10/10) | 0 |
| files | 100.0% (2/2) | none | none | none | 100.0% (10/10) | 100.0% (10/10) | none | 90.0% (9/10) | 100.0% (10/10) | 0 |
| guide | none | none | none | none | 100.0% (10/10) | 90.0% (9/10) | none | 80.0% (8/10) | 80.0% (8/10) | 0 |
| loc | 100.0% (4/4) | none | none | none | 100.0% (10/10) | 100.0% (10/10) | none | 100.0% (10/10) | 100.0% (10/10) | 0 |
| mood | 100.0% (1/1) | none | none | none | 100.0% (10/10) | 90.0% (9/10) | none | 90.0% (9/10) | 90.0% (9/10) | 0 |
| notes | 0.0% (0/1) | none | none | none | 100.0% (10/10) | 90.0% (9/10) | none | 80.0% (8/10) | 80.0% (8/10) | 0 |
| outline | none | 0.0% (0/2) | none | none | 100.0% (2/2) | 50.0% (1/2) | none | 0.0% (0/2) | 50.0% (1/2) | 0 |
| ovr | none | none | none | none | 100.0% (10/10) | 100.0% (10/10) | none | 100.0% (10/10) | 100.0% (10/10) | 0 |
| rel | 100.0% (30/30) | none | 100.0% (10/10) | none | 100.0% (10/10) | 100.0% (10/10) | none | 100.0% (10/10) | 100.0% (10/10) | 0 |
| reports | none | 50.0% (1/2) | none | none | 100.0% (2/2) | 100.0% (2/2) | none | 100.0% (2/2) | 100.0% (2/2) | 0 |
| scenes | 100.0% (7/7) | none | 100.0% (2/2) | none | 100.0% (10/10) | 70.0% (7/10) | none | 60.0% (6/10) | 80.0% (8/10) | 0 |
| sched | 100.0% (3/3) | none | none | none | 100.0% (10/10) | 100.0% (10/10) | none | 80.0% (8/10) | 100.0% (10/10) | 0 |
| settings | none | none | none | none | 100.0% (8/8) | 75.0% (6/8) | none | 100.0% (8/8) | 100.0% (8/8) | 0 |
| shots | none | none | none | none | 100.0% (10/10) | 100.0% (10/10) | none | 100.0% (10/10) | 100.0% (10/10) | 0 |
| sides | none | 100.0% (2/2) | none | none | 100.0% (2/2) | 100.0% (2/2) | none | 50.0% (1/2) | 100.0% (2/2) | 0 |
| spage | none | 100.0% (2/2) | none | none | 100.0% (2/2) | 100.0% (2/2) | none | 50.0% (1/2) | 50.0% (1/2) | 0 |
| story | 74.2% (23/31) | none | 100.0% (10/10) | none | 100.0% (20/20) | 75.0% (15/20) | none | 80.0% (16/20) | 90.0% (18/20) | 0 |
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
| Median, all questions (393 asked) | 1.7 s | 15.0 s or lower: met |
| 95th percentile, all questions | 10.5 s | 30.0 s or lower: met |
| Median, questions that used the engine (347) | 2.0 s | |
| 95th percentile, same | 10.9 s | |
| First question after the project opened | 4.6 s | |
| First question that used the engine | 4.6 s | |
| Prompt tokens served from stored work | 52.1% (343953 of 660631) | |
| Completion tokens, engine calls | 37682, 715 | |

| Length class | Questions | Median | 95th percentile |
|---|---|---|---|
| narrative | 45 | 4.8 s | 11.6 s |
| one_sentence | 228 | 1.2 s | 3.0 s |
| paragraph | 35 | 3.0 s | 15.3 s |
| short | 73 | 1.9 s | 8.1 s |
| story | 12 | 11.8 s | 33.4 s |

| Stage | Mean per question |
|---|---|
| dossier | 6 ms |
| engine_generation | 2447 ms |
| engine_prompt | 884 ms |
| turn | 132 ms |

Stage times come from the AI trace and from what the request recorded; a stage nobody records is not listed. Engine prompt and generation are the engine's own figures and include the earlier turns a conversation case asks first, so they add up to more than the time of the last question.
