# Offline AI files: missing, damaged, whole again

Spec: `tests/e2e/specs/76-ai-files-integrity.test.mjs`

Tags: `@e2e-data-root: own`

Before: an empty data folder of its own, with the model set installed as a finished download leaves it. The language model file of that set is removed before the app starts.

"The download-again message" is the panel text "An OpenFrame AI file is missing or damaged". "The reinstall message" is "Reinstall OpenFrame Studio to repair it". "Change one byte of F" means: make F a real copy, then flip one byte in its middle, so the size stays the same. "Restart with C" means: open the project menu and choose Close project, quit the app, do C, start the app, check Application Home shows, then click the project row "Open Night Bus". "Ask if open Q" means: if the Ask or command box shows within 5 seconds, type Q and press Enter. "The engines" are the Offline AI engine processes of the app.

## A missing model file turns AI off and offers the download again; the rest of OpenFrame works

1. Start the app. Do the first launch: name 'E2E Tester', project title 'Night Bus'.
2. Click Assistant.
3. Check: within 15 seconds the panel shows the download-again message and "Download OpenFrame AI again".
4. Check: the panel does not show the reinstall message, and has no question box.
5. Click Offline AI settings, then choose Offline AI settings. Check: the screen shows "Needs a new download".
6. Go to Tasks. Click New Task, type 'Confirm pier permit' as the title, click Add Task.
7. Check: the screen shows "Added task “Confirm pier permit”" and the open tasks are exactly 'Confirm pier permit'.
8. Check: no engine runs, and the host was asked nothing.

## Download again restores the files from the host and AI answers with the real model

1. Click Assistant, then Download again.
2. Check: within 4 minutes the Ask or command box shows.
3. Check: the host was asked for the signed list.
4. Ask 'Scouting the railway station needs a task.'
5. Check: the result is a proposal, an answer or a question back, and at least one engine runs.

## An altered model of the right size is caught before any engine starts

1. Restart with: change one byte of the search model file.
2. Click Assistant. Ask if open 'How many scenes are in the script?'.
3. Check: within 4 minutes the panel shows the download-again message, and nothing on the screen shows the reinstall message.
4. Check: no engine runs.

## An altered engine library is caught before launch and asks for a reinstall

1. Make a copy of the build (the executable and its `ai` folder) in the data folder, with one byte changed in `runtimes/llama-cpp-b11259-win-x64-cpu/ggml-base.dll`.
2. Restart with: install the model set again; then start the copy instead of the build.
3. Click Assistant. Ask if open 'How many scenes are in the script?'.
4. Check: within 4 minutes the panel shows the reinstall message, and does not offer "Download OpenFrame AI".
5. Check: no engine runs.

## Platform notes

- Mac: the engine is a `.dylib` inside the app bundle; a changed copy of the bundle is launched the same way. To confirm in phase M-track.
