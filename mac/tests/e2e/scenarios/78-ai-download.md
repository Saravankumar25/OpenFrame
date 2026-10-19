# Download OpenFrame AI after install

Spec: `tests/e2e/specs/78-ai-download.test.mjs`

Tags: `@e2e-data-root: own`, `@e2e-ai-models: none`

Before: an empty data folder of its own, with nothing downloaded. The host serves the model set normally.

"The card" is the section "Download OpenFrame AI" (in the AI panel or in Settings, Offline AI). "Ready" means: click Assistant if it is closed, and check that within 4 minutes the Ask or command box shows. "The language model file" and "the search model file" are the two files the host's signed list names.

## Before the click: the not-installed state with the size, and no request to the host

1. Do the first launch: name 'E2E Tester', project title 'Harbour Lights'.
2. Click Assistant.
3. Check: within 15 seconds the card shows "A one-time download of <size> GB. It needs <size> GB of free space" and "uses the internet once".
4. Check: the panel has no question box.
5. Check: the host was asked nothing.

## An interrupted download resumes where it stopped and AI becomes ready

1. Make the host cut its next answer for the language model file after 300 MB.
2. Click Download OpenFrame AI.
3. Check: the card shows "The OpenFrame AI download stopped".
4. Click Try again. Check: ready.
5. Check: the first request for the language model file asked for the whole file and a later one asked for the rest only (a range).
6. Check: the host was asked only for the three pinned files, at their Hugging Face paths, and the set version in
   `models/active.json` is the one computed from the lock.

## With the host down, AI answers on this computer

1. Make the host refuse every request.
2. Ask 'Summarise this project in one sentence.'.
3. Check: the answer is not empty, and the host was asked nothing more.

## A restart with the host down finds OpenFrame AI and asks the host nothing

1. Open the project menu and choose Close project. Quit the app and start it again.
2. Check: Application Home shows. Click the project row "Open Harbour Lights".
3. Check: ready within 30 seconds, and the host was asked nothing more.

## Remove takes the AI away; a damaged file is set aside; Pause and Resume continue the download

1. Make the host serve normally. Click Offline AI settings, then choose Offline AI settings.
2. Click Remove OpenFrame AI…, then Remove.
3. Check: the screen shows "OpenFrame AI was removed." and "Not downloaded".
4. Make the host send the language model file with one byte changed. Click Download OpenFrame AI.
5. Check: within 4 minutes the card shows "damaged and has been set aside".
6. Make the host send slowly (40 MB a second). Click Try again.
7. Check: the card shows "Downloading:" with some bytes. Click Pause. Check: the card shows "Paused at".
8. Make the host serve normally. Click Resume download.
9. Check: within 4 minutes the screen shows "AI Ready", and a request for the language model file asked for the rest only.
10. Check: ready.

## Platform notes

- Free space comes from Windows (`GetDiskFreeSpaceExW`). Mac: the same figure from the file system, and the models live in the app's container. To confirm in phase M-track.
- Install with no network and uninstall keeping the projects are installer checks (Windows phase W9).
