# AI work survives a crash

Spec: `tests/e2e/specs/75-ai-crash-safe-apply.test.mjs`

Tags: `@e2e-data-root: own`

Before: an empty data folder of its own. The app carries the real bundled model with fixed AI sampling. The app starts for the first time.

"Ask Q", "the card" and "Go to Tasks" are as in `74-ai-change-sets.md` and the README of this folder. "Kill the app" means: end every process of the app at once, engines included, as a crash or power loss would. "The engines" are the Offline AI engine processes.

"Relaunch after the kill" means:

1. Wait about 1.5 seconds and start the app again on the same data.
2. Check: the screen shows "Your projects".
3. Click the project row "Open Night Bus".
4. Check: the Workspaces navigation shows.
5. If an enabled Open Recovery button shows within 5 seconds, click it and check it goes away.

## Killed while applying: after a relaunch the Change Set is all there or not there at all

1. Do the first launch: type 'E2E Tester' as the name, click Continue, click Create Project, type 'Night Bus' as the title, click Create.
2. Click Assistant.
3. Ask 'Create a task to scout the railway station.'
4. Check: the result is a Proposal.
5. Check: the card says how many changes it makes, at least one. Call that number N.
6. Click Apply Changes on the card.
7. Kill the app at once.
8. Relaunch after the kill.
9. Go to Tasks.
10. Check: there are either no open tasks or exactly N (all or nothing).
11. Open the AI Assistant.
12. Check: the conversation is restored with at least one result.
13. Check: the card is still shown.
14. Check: if there are N tasks, the card's state says "Applied"; if there are none, it does not.

## Killed while the engine answers: no engine is left running, and AI works after a relaunch

1. Open the AI Assistant.
2. Type 'Which scene happens at the railway station?' in Ask or command and press Enter. Do not wait for the answer.
3. Check: within 2 minutes at least one engine is running.
4. Kill the app.
5. Check: within 15 seconds no engine from the app's bundled AI folder is running, whoever started it.
6. Relaunch after the kill.
7. Ask 'Create a task to check the bus timetable.'
8. Check: the result is a Proposal, an Answer or a Clarify.

## Notes

- Every Ask waits up to 4 minutes.

## Platform notes

- Killing the app and finding the engines use the Windows process list (`llama-server.exe`). Mac: to decide in phase M-track.
