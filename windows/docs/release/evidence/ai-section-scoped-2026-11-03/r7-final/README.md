# Final model run of the section-scoped Straft AI

Run on 2026-11-04: `ai-eval --only sec-s-`, the fixed sample of 302 cases, release build of the eval tool, Gemma 3 1B on the
processor. Other builds and end-to-end specs ran on the same computer for most of the run, so the times are high. Set against
the old build on the same questions (`../r0-sections/`), which ignored the picked sections.

| Measure | Old build | This build | Bar |
|---|---|---|---|
| Answer facts | 47.1% (66/140) | 71.7% (109/152) | |
| Required facts in the answer | 43.3% (142/328) | 49.7% (169/340) | |
| Required facts in the dossier, all 706 cases | | 94.9% (1092/1151) | 90%, met |
| Invented names | 32.1% (235/733) | 39.7% (372/937) | no worse than R0, **not met** |
| Wrong scene citations | 0.0% | 0.0% (0/80) | |
| Pointer precision, all 706 cases | | 92.6% | 90%, met |
| Pointer recall, all 706 cases | | 68.8% | 90%, **not met** |
| False pointers, all 706 cases | | 1.0% | at most 5%, met |
| One sentence, median and 95th percentile | 5.1 s, 10.4 s | 7.9 s, 20.0 s | 15 s, 30 s, met |
| Short, median and 95th percentile | 7.3 s, 19.4 s | 10.0 s, 27.7 s | 15 s, 30 s, met |
| Long 250, median and 95th percentile | | 37.0 s, 69.9 s | 30 s, 60 s, **not met** |
| Long 350, median and 95th percentile | | 51.7 s, 124.6 s | 30 s, 60 s, **not met** |

The old build's times were measured on a quieter computer, so the rise in the short classes is not all from this work. Three
bars are missed and none was lowered (ADR-0040 decision 8): invented names, pointer recall and long answer speed. The rise in
invented names comes with longer answers. Names counted went from 733 to 937. A person should spot-check about one case in ten
before these numbers count (gold/README.md).
