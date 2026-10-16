# Bundled Offline AI: missing, damaged, whole again

Spec: `tests/e2e/specs/76-ai-bundle-integrity.test.mjs`

Tags: `@e2e-data-root: own`, `@e2e-ai-bundle: own`, `@e2e-debug-only: reads its AI from its own folder (OPENFRAME_AI_BUNDLE_DIR), which a release build refuses`

Before: an empty data folder of its own. The app reads its AI from its own copy of the build's AI folder. In that copy, files are linked to the build and only a file a scenario changes is a real copy.

The bundle files used below:

- the chat model: `models/gemma.../model.gguf`
- the search model: `models/bge.../model.gguf`
- the engine library: `runtimes/<name>/ggml-base.dll`

"The repair message" is the panel text "Reinstall OpenFrame Studio to repair it". "Change one byte of F" means: make F a real copy, then flip one byte in its middle, so the size stays the same. "Restart with layout L" means: open the project menu and choose Close project, quit the app, lay out the AI copy as L, start the app, check Application Home shows, then click the project row "Open Night Bus" and check no recovery question shows. "Ask if open Q" means: if the Ask or command box shows within 5 seconds, type Q and press Enter.

"Ask Q" and "Go to Tasks" are as in the README of this folder and `74-ai-change-sets.md`. "The engines" are the Offline AI engine processes of the app.

## A missing model file turns AI off with a plain message; nothing offers a download

1. Lay out the AI copy with the chat model left out.
2. Start the app.
3. Do the first launch: type 'E2E Tester' as the name, click Continue, click Create Project, type 'Night Bus' as the title, click Create.
4. Click Assistant.
5. Check: within 15 seconds the panel shows the repair message.
6. Check: the panel shows "missing from this installation" and "Everything else in OpenFrame works normally."
7. Check: the panel has no question box.
8. Check: no visible text says "download".
9. Click Offline AI settings, then choose Offline AI settings.
10. Check: the screen shows "Needs a reinstall".
11. Check: no visible text says "download offline AI" or "remove offline AI".
12. Go to Tasks.
13. Click New Task. Type 'Confirm pier permit' in the task title field. Click Add Task.
14. Check: the screen shows "Added task “Confirm pier permit”".
15. Check: the only open task is "Confirm pier permit".
16. Check: no engine was started.

## An altered model of the right size is caught before any engine starts

1. Restart with layout: a whole copy, then change one byte of the search model.
2. Click Assistant.
3. Ask if open 'How many scenes are in the script?'
4. Check: within 4 minutes the panel shows "changed or damaged" and the repair message.
5. Check: no visible text says "download".
6. Check: no engine was started.

## An altered engine library is caught before the engine is launched

1. Restart with layout: a whole copy, then change one byte of the engine library.
2. Click Assistant.
3. Ask if open 'How many scenes are in the script?'
4. Check: within 4 minutes the panel shows "changed or damaged" and the repair message.
5. Check: no engine was started.

## With the files whole again, AI answers with the real model

1. Restart with layout: a whole copy, nothing changed.
2. Click Assistant.
3. Ask 'Scouting the railway station needs a task.' (worded so that the model must answer: a plain "Create a task to ..." command is handled without the model).
4. Check: the result is a Proposal, an Answer or a Clarify.
5. Check: at least one engine is running.

## Notes

- This file is skipped on the release build, which refuses a separate AI folder.

## Platform notes

- The engine library is a Windows `.dll` and the engines are `llama-server.exe` processes. Mac: the `.dylib` and engine names inside the app bundle; to decide in phase M-track.
- Files are hard-linked from the build so a second copy of the model is not needed. Mac: to decide in phase M-track.
