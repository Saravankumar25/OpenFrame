# Dossier by leaves (round R3), required-fact recall

Run on 2026-11-03: `ai-eval --dossier-only --only sec-` over the 706 section-scoped cases, no model writing. Each case is built under
the sections it picks. `dossier-report.md` is the final run (debug build of ai-eval; recall does not depend on the build).
`dossier-results.jsonl` has one line per case without the dossier text. `first-run-before-recall-fixes-release.md` is the first run,
in a release build, before the recall fixes (87.3 percent); its build time line is the latency measure.

Result: required facts in the dossier 93.6 percent (1,077 of 1,151 entries), 90.3 percent of cases fully covered. Target 90 percent.

Dossier build time on the Low Tide fixture, release build, first run: mean 15.8 ms, 95th percentile 38.7 ms, maximum 562 ms (one case,
the first after the engine started). The final debug run: mean 21.2 ms, 95th percentile 48.7 ms, maximum 1,068 ms.

Below 90 percent, by leaf: files/files 37.5 percent (the text of a project file is not indexed, only its name and note; whether the
assistant may read small text files is an owner decision), production/day-out-of-days 83.7, production/overview 85.7,
production/storyboards 87.5, notes-tasks/notes 88.9, help/settings 88.5 (final run). Cases whose answer type was read as chat or change
(about 8) get no facts by design; that is the answer-type stage, not the fetch.
