# AI entry points in every workspace

Spec: `tests/e2e/specs/73-ai-workspaces.test.mjs`

Tags: `@e2e-data-root: own`

Before: an empty data folder of its own. The app carries the real bundled model with fixed AI sampling. The app starts for the first time.

"Import the Night Bus screenplay" and "Ask Q" are shared steps in the README of this folder. "Pick sections S" is a shared step in the README of this folder. "The sections line" is the line under an answer that starts with "Using: ", built from the sections the answer was read from. "The hint" is the line under Send. "Reopen Straft AI here" means: close the Straft AI if it is open (Close Straft AI), click Straft AI, then wait until the hint shows. "Answered" means the result kind is Answer, Navigate, Clarify, Proposal or Suggestion.

## Straft AI opens with nothing picked in every workspace

1. Do the first launch: type 'E2E Tester' as the name, click Continue, click Create Project, type 'Night Bus' as the title, click Create.
2. Import the Night Bus screenplay.
3. For each of Home, Idea Vault, Story, Screenplay, Breakdown, Production and Call Sheets: click it, then Reopen Straft AI here. Check: no section chip is shown, and the hint reads "Pick a section".
4. Open the project menu and choose Notes & Tasks. Reopen Straft AI here. Check: the hint reads "Pick a section".

## The keyboard shortcut opens and closes Straft AI

1. Close the Straft AI if it is open.
2. Press Ctrl+J.
3. Check: the Straft AI opens.
4. Press Ctrl+J.
5. Check: the Straft AI closes.

## A question asked under Beat Board is answered from the Beat Board

1. Click Story in the Workspaces navigation, then click the Board tab. Click Straft AI.
2. Pick sections "Beat Board".
3. Ask 'What is on the Beat Board?'
4. Check: the question was answered.
5. Check: the sections line reads "Using: Beat Board".

## A question asked under Scenes is about that screenplay

1. Click Screenplay in the Workspaces navigation. Open the Straft AI if it is closed.
2. Pick sections "Scenes".
3. Ask 'Who speaks in this screenplay?'
4. Check: the question was answered.
5. Check: the answer names ARJUN or MEERA (in capitals or not).

## A question asked under Scenes about the places in the script is answered from the script

1. Click Production in the Workspaces navigation. Open the Straft AI if it is closed.
2. Pick sections "Scenes".
3. Ask 'Which locations are used in the script?'
4. Check: the question was answered.
5. Check: the answer or its sources name "BUS STOP", "RAILWAY STATION" or "POLICE STATION".

## Notes

- On the Mac, Ctrl+J uses Cmd instead.
- Every Ask waits up to 4 minutes.
