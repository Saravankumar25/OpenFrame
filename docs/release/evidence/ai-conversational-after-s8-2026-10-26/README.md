# Low Tide run of the conversational assistant after slices S3 to S8, 2026-10-26

Build: commit d9c187cb (S7 and S8), release, processor engine, deterministic sampling, `--only lt-` (251 conversation and
23 tool cases). Wall time 71.0 min. Compared with the baseline of the old five-lane build
(`../ai-conversational-baseline-2026-10-26/`):

| Measure | Baseline | This run |
|---|---|---|
| Answer facts (`answer_has`) | 17.6% | 64.0% |
| Required facts | 25.9% | 68.3% |
| Length class | 65.7% | 93.2% |
| No re-ask after the screenplay was named | 83.1% | 97.2% |
| No navigation on explain turns | 85.1% | 97.0% |
| No data dump to an upset user | 85.7% | 95.2% |
| Repetition | 6.0% | 0.4% |
| Answer type, exact and group | not measured | 82.1%, 90.4% |
| Median and p95 time | 16.7 s, 30.8 s | 4.4 s, 22.9 s |
| Prompt tokens served from the engine's cache | not measured | 64.9% |
| Correct abstention | 15.0% | 15.0% (scored low because the code-written sentence said "The project does not hold", which the scorer's phrase list does not know; fixed in the next round) |
| Tool and arguments, tool and action cases | 91.3% | 69.6% (four of the misses are questions now answered from facts without a tool call, the old measure; two are real faults: screen navigation and "open" read as a verb) |

The baseline's money family ran on a fixture whose budget was 100 times too small (see its README), so money numbers are
not comparable. The old invented-name rate checks names against the screenplay text only and is not comparable either.
