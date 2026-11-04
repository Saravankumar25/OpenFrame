# Out-of-section pointer: final measure

`ai-eval --dossier-only --only sec-`, 597 cases, 2026-11-04. Full table: `dossier-report.md`.

| Measure | Bar | First measure | After tuning |
|---|---|---|---|
| Precision | 90 percent | 56.8 percent | 92.6 percent (75 of 81) |
| Recall | 90 percent | 42.2 percent | 68.8 percent (75 of 109) |
| False pointers | at most 5 percent | 3.4 percent | 1.0 percent (6 of 597) |

Precision and false pointers meet their bars. Recall does not, and the bar was not lowered. When the pointer stays silent the
person gets the plain "I could not find X. I looked in A and B." with the sections they picked, which is honest but less
helpful. Three of the six false pointers come from how the name resolver homes a name, so the next fix belongs there.
