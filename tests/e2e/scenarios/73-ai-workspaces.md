# AI entry points in every workspace

Spec: `tests/e2e/specs/73-ai-workspaces.test.mjs`

Tags: `@e2e-data-root: own`

Before: an empty data folder of its own. The app carries the real bundled model with fixed AI sampling. The app starts for the first time.

"Import the Night Bus screenplay" and "Ask Q" are shared steps in the README of this folder. "The scope line" is the line in the AI Assistant that starts with "Using: ". "Reopen the assistant here" means: close the AI Assistant if it is open (Close AI Assistant), click Assistant, then wait until the scope line shows. "Answered" means the result kind is Answer, Navigate, Clarify, Proposal or Suggestion.

## The assistant takes the scope of each workspace

1. Do the first launch: type 'E2E Tester' as the name, click Continue, click Create Project, type 'Night Bus' as the title, click Create.
2. Import the Night Bus screenplay.
3. Click Home. Reopen the assistant here. Check: the scope line reads "Using: Whole Project".
4. Click Idea Vault. Reopen the assistant here. Check: the scope line reads "Using: Whole Project".
5. Click Story. Reopen the assistant here. Check: the scope line reads "Using: Beat Board".
6. Click Breakdown. Reopen the assistant here. Check: the scope line reads "Using: Production Workspace".
7. Click Production. Reopen the assistant here. Check: the scope line reads "Using: Production Workspace".
8. Click Call Sheets. Reopen the assistant here. Check: the scope line reads "Using: Production Workspace".
9. Click Screenplay. Reopen the assistant here. Check: the scope line reads "Using: Current Scene" or "Using: Current Screenplay".
10. Open the project menu and choose Notes & Tasks. Reopen the assistant here. Check: the scope line reads "Using: Whole Project".

## The keyboard shortcut opens and closes the assistant

1. Close the AI Assistant if it is open.
2. Press Ctrl+J.
3. Check: the AI Assistant opens.
4. Press Ctrl+J.
5. Check: the AI Assistant closes.

## A question asked in the Beat Board uses the Beat Board

1. Click Story in the Workspaces navigation. Click Assistant.
2. Ask 'What is on the Beat Board?'
3. Check: the question was answered.
4. Check: the scope line reads "Using: Beat Board".

## A question asked in the Screenplay is about that screenplay

1. Click Screenplay in the Workspaces navigation. Open the AI Assistant if it is closed.
2. Choose Current Screenplay in the scope list.
3. Ask 'Who speaks in this screenplay?'
4. Check: the question was answered.
5. Check: the answer names ARJUN or MEERA (in capitals or not).

## A question asked in Production uses the Production Workspace

1. Click Production in the Workspaces navigation. Open the AI Assistant if it is closed.
2. Ask 'Which locations are used in the script?'
3. Check: the question was answered.
4. Check: the answer or its sources name "BUS STOP", "RAILWAY STATION" or "POLICE STATION".

## Notes

- On the Mac, Ctrl+J uses Cmd instead.
- Every Ask waits up to 4 minutes.
