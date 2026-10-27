# Mac AI after the Windows sync: measured runs (2026-10-27)

Code: branch `mac/windows-sync` at `c54ecedd` (the Windows conversational assistant of ADR-0037 in the Mac core). Binary:
`cargo build --release -p openframe-ai-eval --bin ai-eval`. Machine: Apple M1, 16 GB, macOS 26.5; llama.cpp b11259 with
Metal; Gemma 3 1B q8_0; deterministic sampling. Each run once. Swift builds and E2E runs shared the machine during the
runs, so times are upper bounds, not quiet-machine times; the accuracy measures are deterministic.

Folders: `ai-after-port-lt-metal` (Low Tide sets on Metal), `ai-after-port-turn-only` (answer type read),
`ai-after-port-en-metal` (all English sets), `ai-after-port-lt-cpu` (Low Tide with Graphics card Off). A first
after-port run measured nothing (relative engine folders, every case "AI unavailable") and was deleted; the eval fix is
`c54ecedd`.

## Against Windows (Low Tide, 274 cases; Windows numbers from `../../ai-conversational-after-s8-2026-10-26/`)

| Measure | Mac, Metal | Mac, processor | Windows |
|---|---|---|---|
| Answer facts | 68.4% | 69.1% | 64.0% |
| Required facts | 71.4% | | 68.3% |
| Length class | 92.0% | | 93.2% |
| No re-ask | 97.2% | | 97.2% |
| Repetition | 0.8% | 0.4% | 0.4% |
| Correct abstention | 70.0% | 75.0% | 15% before the fix round |
| Answer group | 91.6% | | 90.4% |
| Answer type, 40 unseen messages | 87.5% exact, 92.5% group | | 87.5%, 92.5% |
| Tool and arguments (23 tool and action cases) | 73.9% | 73.9% | 69.6% |
| End state (7 action cases) | 85.7% | 85.7% | 85.7% |
| Median time | 2.5 s | 5.1 s | 4.4 s |
| p95 time | 20.4 s | 62.7 s | 22.9 s |
| First question after open | 19.8 s | | |
| Prompt cache reuse | 67.3% | 65.2% | 64.9% |

## Against the Mac baseline (English sets)

| Measure | Before (Mac routes) | After (conversational) |
|---|---|---|
| Action cases: tool and arguments | 96.2% (76/79) | 94.2% (81/86) |
| Action cases: end state | 89.9% | 89.5% |
| Read tool cases: tool chosen | 100% (81/81) | 27.8% (27/97) |
| Retrieval recall at 5 | 99.4% | 99.4% |
| Safety | 100% | 100% |
| Median / p95 | 0.8 s / 26.4 s | 1.9 s / 13.3 s |

## Bars (spec section 7.5 proposed bars and ADR-0028), Low Tide on Metal

| Bar | Mac, Metal | Met |
|---|---|---|
| Median 15 s or lower, p95 30 s or lower | 2.5 s, 20.4 s | yes (processor only: p95 62.7 s, no) |
| Poison and safety sets 100% | 100% (English safety set) | yes |
| End state 85% | 85.7% (Low Tide), 89.5% (English) | yes |
| Tool and arguments 90% | 73.9% (Low Tide), 59.0% (English, read cases now answered from facts) | no, as on Windows (69.6%) |
| Required facts (95% lookup and count, 80% explain and relation) | 71.4% over all | no |
| Invented names 2% or lower | 37.6% of names | no |
| Wrong scene citations 0% | 1.7% (1 of 58) | no |
| Repetition 0% | 0.8% (2 of 251) | no |
| Correct abstention 90% | 70.0% | no |
| No re-ask 98% | 97.2% | no |
| No navigation on explain 100% | 97.0% | no |
| Answer type 90% | 84.1% exact, 91.6% group | no on exact, yes on group |
| Length class 95% | 92.0% | no |

The Windows run after S8 misses the same bars by similar margins; the Mac matches or beats Windows on answer facts,
required facts, answer group, abstention, latency and cache reuse, and is slightly behind on length class and
repetition. The orders forbid tuning AI behaviour on the Mac alone (one core, ADR-0034); a fix belongs in the shared
core on both platforms. The tool bar is an owner decision (keep, change or split it for read questions).

Limits: one machine, one run each, synthetic fixture, the gold labels of the Windows sets.
