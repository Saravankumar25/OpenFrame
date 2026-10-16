# Performance with the AI running

Spec: `tests/e2e/specs/97-performance.test.mjs`

Tags: `@e2e-data-root: own`

Before: an empty data folder of its own. The app starts for the first time. The budgets are those of `docs/engineering/21-performance-budgets.md`; the numbers that count come from the release build. The numbers are written to `performance.json` in the artifacts folder.

## A project with the 36-scene Example Feature

1. Do the first launch: name 'E2E Tester', project title 'Measured Feature'.
2. Click Create, choose Import Screenplay, choose the sample script "Example Feature", click Preview, then Import.
3. Check: within 60 seconds the screen shows "Import complete". Click Open Screenplay.
4. Open the project menu and choose Close project.

## P1 start and P2 project open stay inside their budgets

1. Quit the app. Start it again and time it until Application Home shows. Check: under 3 seconds.
2. Click the project row "Open Measured Feature" and time it until the Workspaces navigation shows. Check: under 1.5 seconds.
3. Check: the private memory of the app and its web view, summed, is under 400 MB.

## P3 typing latency in scene 20 of the screenplay is recorded

1. Click Screenplay. Start recording how long each key press takes until the next paint.
2. Click the twentieth action line, press End, and type ' The rain gets heavier as the night goes on and nobody leaves the bus stop.' at about 100 words a minute.
3. Record the number of keys, how many took over 16 ms, the median, the 95th percentile and the worst.

## The AI answers, and memory and processor use with the AI loaded are recorded

1. Ask the AI 'Which scenes take place at night?' and time it until the answer shows.
2. Check: the answer is not empty.
3. Record the memory of the AI engines, and of the app, web view and engines together.
4. Wait 5 seconds, then record the processor use of all of them over 30 seconds, as a percentage of one core.

## Platform notes

- Memory and processor use come from Windows performance counters. Mac: the same figures from the process tools. To decide in phase M-track.
