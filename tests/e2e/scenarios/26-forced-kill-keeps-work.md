# Work survives a forced kill

Spec: `tests/e2e/specs/26-forced-kill-keeps-work.test.mjs`

Tags: `@e2e-data-root: own`

Before: an empty data folder of its own. The app starts for the first time.

## Add an idea, an act and a card, a character, a note and a screenplay line; each one saves

1. Do the first launch: name 'E2E Tester', project title 'Hard Stop'.
2. Click Idea Vault in the Workspaces navigation. Click Quick Capture. Type 'A ferry that only sails when nobody is waiting.' in the Idea field. Click Save in the dialog.
3. Check: the screen shows "Saved to the Project Idea Vault".
4. Click Story in the Workspaces navigation, then click the Board tab. Click Add Act, type 'Act One' in the act title field and press Enter.
5. Click Add, choose Scene Card, type 'The ferry leaves without its captain' in the short description field and press Enter.
6. Check: a Scene Card named "The ferry leaves without its captain" is on the board.
7. Click the Characters tab. Click Add Character, type 'Ilse' in the name field, click Create Character.
8. Check: the screen shows "Added character Ilse". Click the Board tab.
9. Open the project menu and choose Notes & Tasks. Click New Note, type 'Ferry timetable' in the title field, click Close in the drawer.
10. Check: the Project Notes list has a note starting "Ferry timetable".
11. Click Screenplay. Click New Screenplay. Click the first line of the editor and type 'The engine coughs twice and dies.'
12. Check: within 15 seconds the editor's save state says "Saved".

## The app is killed; after a new start every piece of work is there

1. End the app abruptly: kill its process tree, with no chance to close (as a crash or a power cut would).
2. Start the app again on the same data folder.
3. Check: within 30 seconds Application Home shows "Your projects".
4. Click the project row "Open Hard Stop". Check: the Workspaces navigation shows.
5. If the recovery question shows within 5 seconds, click Open Recovery and check the question closes.
6. Click Idea Vault. Check: the page shows "A ferry that only sails".
7. Click Story in the Workspaces navigation, then click the Board tab. Check: the Scene Card "The ferry leaves without its captain" is on the board.
8. Click the Characters tab. Check: the page shows "Ilse". Click the Board tab.
9. Open the project menu and choose Notes & Tasks. Check: a note starts with "Ferry timetable".
10. Click Screenplay. Check: an editor line ends with "The engine coughs twice and dies."

## After the recovery the project takes new work and saves it

1. Click the last line of the editor, press Ctrl+End, then End, then Enter, then Ctrl+2 (Action).
2. Type 'The captain swims after it.'
3. Check: within 15 seconds the editor says "Saved", and a line ends with "The captain swims after it."

## Notes

- Mac: Cmd+End and Cmd+2 where the steps say Ctrl.

## Platform notes

- The kill ends the process tree with the Windows process tools. Mac: `kill -9` on the app and its children.
