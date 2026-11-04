# R5 dossier recall (2026-11-04)

Run: `ai-eval --dossier-only --only sec-` on a debug build, 706 cases, Low Tide fixtures.
The report and results are from the build before the last change (the Settings guide section is
now always added as a third section for help/settings). They are not yet re-run with it.

| Section | R3 | R5 |
|---|---|---|
| production/day-out-of-days | 83.7% (36/43) | 100.0% (43/43) |
| production/overview | 85.7% (6/7) | 85.7% (6/7), gold case sec-s-production-overview-003 expects 13, the Overview screen read says 21 |
| production/storyboards | 87.5% (14/16) | 100.0% (16/16) |
| help/settings | 88.5% (23/26) | 92.3% (24/26), two regressions fixed by the later change |
| notes-tasks/notes | 88.9% (16/18) | 100.0% (18/18) |
| all sec- cases | 93.6% (1077/1151) | 94.8% (1091/1151) |
