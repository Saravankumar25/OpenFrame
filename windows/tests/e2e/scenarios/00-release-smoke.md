# Release smoke

Spec: `tests/e2e/specs/00-release-smoke.test.mjs`

Tags: `@e2e-data-root: own`

Before: an empty data folder of its own. The app starts for the first time. This is the short check that runs first on every build, and the first thing a release run proves on the hardened executable. If it fails, the rest of a release run is not worth reading.

"Import the Night Bus screenplay" and "Ask Q" are shared steps in the README of this folder.

## Start, create a project, type in the screenplay until it says Saved, close the project

1. Do the first launch: type 'E2E Tester' as the name, click Continue, click Create Project, type 'Smoke Night' as the title, click Create.
2. Import the Night Bus screenplay.
3. Click the last element of the screenplay. Press Ctrl+End, then End, then Enter.
4. Press Ctrl+2 so the new element is an Action.
5. Type 'The driver waits one more minute.'
6. Check: the editor toolbar reads "Saved".
7. Open the project menu and choose Close project.
8. Check: Application Home shows and the project shell is gone.

## Start again, reopen the project, and the typed line is there

1. Quit the app and start it again on the same data.
2. Check: Application Home shows.
3. Click the project row "Open Smoke Night". Check: no recovery question shows.
4. Click Screenplay in the Workspaces navigation.
5. Check: a line of the screenplay ends with "The driver waits one more minute."

## The Offline AI answers from the project, and nothing leaves this computer

1. Ask 'Which scenes take place at night?'
2. Check: the answer has text.
3. Check: the result is not of kind Failed or Unavailable.
4. Check: at least one Offline AI engine ran for the answer.
5. Check: no process of the app, its engines or its web view holds a connection to an address outside this computer.
6. Check: every socket these processes listen on is on 127.0.0.1 or ::1 only.

## Notes

- On the Mac, Ctrl+End and Ctrl+2 use Cmd instead.
- The process and socket list is saved to `tests/e2e/artifacts/release-smoke-network.json`.

## Platform notes

- The processes are the app, `llama-server.exe` engines and WebView2 (`msedgewebview2.exe`), found with the Windows process and network commands. Mac: the engine and WebKit process names and an equivalent of those commands; to decide in phase M-track.
