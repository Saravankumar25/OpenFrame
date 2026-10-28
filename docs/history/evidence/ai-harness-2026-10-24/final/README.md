# AI harness final run, 2026-10-25

Measured after the AI harness rebuild (ADR-0035), with the evaluation binary built from commit 93f0a190
(the S8 build, which adds the answer sanitizer and the intent gates that offer the two closest tools). Same
setup as the [baseline](../baseline/README.md): local model Gemma 3 1B Q8_0, search model Multilingual E5
small, graphics placement automatic, deterministic sampling, development laptop (Intel Core Ultra 5 125H),
release build of the evaluation tool, direct routes on. The CPU was shared with other builds, so times are
upper bounds.

| Run | Set | Key result | Baseline |
|---|---|---|---|
| para-on | paraphrase (90) | correct tool 46.7% (42/90), tool and arguments 44.4%, answer facts 51.1% (23/45) | 20.0%, 20.0%, 37.8% |
| conv-on | conversation.en (60) | answer facts 46.4% (26/56), invented names 7.7% (11/142), wrong scene citations 0% (0/86), within length 71.7% (43/60), correct abstention 2 of 4, repetition 0% (0/60), median 32 s | 42.9%, 13.1%, none counted, 55.0%, 0 of 4, 8.3%, median 39 s |
| tool-on | tools.en, 81 cases | correct tool and arguments 97.5% (79/81) | 100.0% (81/81) |
| safety | safety.en (41) | safety checks 41 of 41 | not measured in the baseline |

Notes on the numbers:

- The tool set is the 81 cases of `tools.en.jsonl` whose id starts with `tool-en`, the same cases the baseline
  report counted (the baseline README table says 160, which is the line count of the whole file).
- Tool and arguments fell from 100.0% to 97.5% (two cases). The regex routes were written on that wording, so
  the figure measures the routes and is not evidence for real requests; the held-out paraphrase row is.
- Within length was 71.7% here. The S7 build measured 86.7%. The drop was traced to the S8 `evidence_echo`
  check, which counted a restated script line as a copy and cut correct explanations on the last pass. It was
  fixed in commit 73a41425 after this run and has **not been measured again**.
- The S7 build measured poisoning 5 of 5 and safety 41 of 41; this run repeats safety (41 of 41).

What did not run: the poisoning, writing and routes-off runs. Their processes stopped at once with "the
Offline AI is not available in this run" because an end-to-end start reset the engine bundle folder while the
batch was running. No result is claimed for them here.

Limits of these figures:

- The held-out paraphrase set was used once to calibrate the intent thresholds in S8, so it is no longer
  strictly held out. An honest final figure needs a fresh set written without looking at these cases.
- Paraphrase accuracy (46.7%) is far below the 90 percent bar of ADR-0028. The bar is not met.
- The gold sets are drafted by Claude and wait for the owner's approval before any number counts as official.

Each `*.report.md` is the harness report; each `*.summary.json` holds the counts.
