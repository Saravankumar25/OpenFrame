# Offline AI assistant with the bundled model

Spec: `tests/e2e/specs/72-ai-assistant.test.mjs`

Tags: `@e2e-data-root: own`

Before: an empty data folder of its own. The app carries the real bundled model and runs with fixed AI sampling (temperature 0, fixed seed), so the checks are about the kind of result and its effects, never the wording. The app starts for the first time.

"Import the Night Bus screenplay" and "Ask Q" are shared steps in the README of this folder. A result has a kind (Answer, Navigate, Clarify, Proposal, Suggestion). "Exact" means it is marked "Exact · from project data". "Its sources" are the source chips under it. "The engines" are the Offline AI engine processes the app started. "The chat engine" is the engine that is not the embedding engine.

## AI is ready on first launch: no download, no setup, no model names

1. Do the first launch: type 'E2E Tester' as the name, click Continue, click Create Project, type 'Night Bus' as the title, click Create.
2. Import the Night Bus screenplay.
3. Click Assistant.
4. Check: the AI Assistant panel shows its Ask or command box.
5. Check: the screen shows "AI Ready".
6. Check: no visible text says "download", "install offline AI", "gemma", "gguf", "llama", "q8_0" or "embedding".
7. Check: the chat engine is not running yet (it starts on the first question).

## Settings, Offline AI shows the installed parts and Remove, and offers no download while installed

1. Click Offline AI settings, then choose Offline AI settings.
2. Check: the "Offline AI" section shows "OpenFrame AI runs on this computer after a one-time download".
3. Open the "Technical details" section.
4. Check: the table "Offline AI components" has exactly three rows.
5. Check: one row names "Gemma-Terms-of-Use".
6. Check: every row shows a 64-character SHA-256 hash.
7. Check: the technical details say "read-only".
8. Check: the button "Remove OpenFrame AI…" is shown, and there is no button "Download OpenFrame AI", "Download Offline
   AI" or "Update Offline AI".

## An exact answer comes from project data, with sources

1. Click Screenplay in the Workspaces navigation. Open the AI Assistant if it is closed.
2. Choose Current Screenplay in the scope list.
3. Ask 'How many scenes are in the script?'
4. Check: the result is an Answer, marked exact.
5. Check: the answer contains the number 3.
6. Check: the answer has at least one source.
7. Check: the chat engine is now running.

## A meaning question is answered from project context it looked up

1. Choose Whole Project in the scope list.
2. Ask 'Which scene happens at the bus stop?'
3. Check: the result is an Answer or a Navigate.
4. Check: one of its sources names "BUS STOP".
5. Click Home in the Workspaces navigation.
6. Click the newest source chip that names "BUS STOP".
7. Check: the Screenplay workspace opens.

## A navigation command opens the place it names

1. Click Home in the Workspaces navigation. Open the AI Assistant if it is closed.
2. Ask 'Open Scene 2.'
3. Check: the result is a Navigate.
4. Check: the Screenplay workspace opens.

## No network: the app and the engines use this computer only while AI works

1. Check: at least one engine is running.
2. Check: every engine runs from the bundled AI folder next to the app.
3. Check: every engine was started to listen on 127.0.0.1 only and in offline mode.
4. Check: no engine has an API key on its command line.
5. Choose Current Screenplay in the scope list.
6. Ask 'How many scenes are in the script?'
7. Check: the result is an Answer.
8. Check: no process of the app or its engines holds a connection to an address outside this computer, and none listens on an outside address.

## Stop Offline AI frees the engines; the next question starts them again

1. Open the AI Assistant if it is closed.
2. Click Offline AI settings, then choose Stop Offline AI.
3. Check: the screen shows "Offline AI stopped".
4. Check: within 30 seconds no engine is running.
5. Ask 'How many scenes are in the script?'
6. Check: the result is an Answer.
7. Check: at least one engine is running again.

## The conversation is still there after a relaunch

1. Count the results in the panel. Check: there are at least 4.
2. Open the project menu and choose Close project.
3. Quit the app and start it again on the same data.
4. Click the project row "Open Night Bus". Check: no recovery question shows.
5. Open the AI Assistant.
6. Check: the panel shows at least as many results as before.
7. Check: the panel shows "AI Ready".

## The docked panel remembers whether it was open when the window loads again

1. Check: the AI Assistant is open.
2. Reload the window and wait for the Workspaces navigation.
3. Check: the AI Assistant opens again.
4. Click Close AI Assistant.
5. Reload the window and wait for the Workspaces navigation.
6. Check: the AI Assistant stays closed.

## Notes

- Every Ask waits up to 4 minutes: the first answer also loads the model, and one answer takes 20 to 40 seconds on a laptop processor.
- The open or closed state of the panel is kept in the window's local storage, so it is checked with a reload, not a relaunch.

## Platform notes

- The engines are `llama-server.exe` processes, found with the Windows process list. Mac: the engine binary name and how to list its processes and sockets are to decide in phase M-track.
