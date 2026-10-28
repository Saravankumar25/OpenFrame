# Story workspace basics

Spec: `tests/e2e/specs/30-story.test.mjs`

Tags: `@e2e-data-root: own`

Before: an empty data folder of its own. The app starts for the first time.

"The cards" means the scene cards on the board, read in order by their names ("Scene Card: <text>"). "The acts" means the act sections, read by their names ("Act <title>").

## An act, a sequence and two scene cards

1. Do the first launch: type 'E2E Tester' as the name, click Continue, click Create Project, type 'Paper Boats' as the title, click Create.
2. Click Story in the Workspaces navigation, then click the Board tab.
3. Check: the screen shows "Your Beat Board is empty."
4. Click Add Act. Type 'Act One' in the Act title field and press Enter.
5. Check: the acts include "Act One".
6. Click Add, then choose Sequence. Type 'Departure' in the Sequence name field and press Enter.
7. Check: the screen shows "Departure".
8. Click Add, then choose Scene Card. Type 'Boats fold themselves at dawn' in the Short description field and press Enter.
9. Check: the cards include "Boats fold themselves at dawn".
10. Click Add, then choose Scene Card. Type 'The river carries the boats away' in the Short description field and press Enter.
11. Check: the cards are, in order, "Boats fold themselves at dawn", "The river carries the boats away".
12. Check: the screen is quiet.

## Edit a card in its drawer, then undo and redo

1. Double-click the card "Boats fold themselves at dawn".
2. Check: the drawer opens with its short description field.
3. Select all text in the short description field and type 'Boats fold themselves at first light'.
4. Check: the status in the Scene Card drawer reads "Saved".
5. Click Close in the Scene Card drawer.
6. Check: the cards include "Boats fold themselves at first light".
7. Move focus off any text field and press Ctrl+Z.
8. Check: the cards include "Boats fold themselves at dawn".
9. Press Ctrl+Y.
10. Check: the cards include "Boats fold themselves at first light".
11. Check: the screen is quiet.

## Rename the act, then undo and redo

1. Click Act options for Act One, then choose Rename.
2. Select all text in the Act title field, type 'Act One [em dash] Launch' and press Enter.
3. Check: the acts include "Act One [em dash] Launch".
4. Press Ctrl+Z.
5. Check: the acts include "Act One".
6. Press Ctrl+Y.
7. Check: the acts include "Act One [em dash] Launch".
8. Check: the screen is quiet.

## Delete a card: undo, redo, then restore from Recently Deleted

1. Click the card "The river carries the boats away" and press Delete.
2. Check: the card is gone.
3. Press Ctrl+Z.
4. Check: the card is back.
5. Press Ctrl+Y.
6. Check: the card is gone again.
7. Open the project menu and choose Recently Deleted.
8. Click Restore The river carries the boats away.
9. Check: the screen shows "Restored “The river carries the boats away”".
10. Click Story in the Workspaces navigation, then click the Board tab.
11. Check: the cards are, in order, "Boats fold themselves at first light", "The river carries the boats away" (the restored card returns to its place).
12. Check: the screen is quiet.

## Characters: create, edit, delete and restore

1. Click the Characters tab.
2. Click Add Character.
3. Type 'Mira' in the character name field and 'Boat maker' in the role field.
4. Click Create Character in the dialog.
5. Check: the screen shows "Added character Mira".
6. Click Edit in the workspace.
7. Select all text in the role field and type 'Paper boat maker'.
8. Click Save in the dialog.
9. Check: the workspace shows "Paper boat maker".
10. Click More actions for Mira, then choose Delete.
11. Click Delete Character in the dialog.
12. Check: the screen shows "Deleted character Mira".
13. Open the project menu and choose Recently Deleted.
14. Click Restore Mira.
15. Click Story in the Workspaces navigation, then click the Board tab. Then click the Characters tab.
16. Check: the workspace shows "Paper boat maker".
17. Check: the screen is quiet.

## The Outline shows the same structure

1. Click the Outline tab.
2. Check: the rows of the "Story outline" tree include one with "Boats fold themselves at first light" and one with "The river carries the boats away".
3. Check: the screen is quiet.
4. Click the Board tab.

## After a relaunch, the act, the cards and the character are kept

1. Open the project menu and choose Close project.
2. Quit the app and start it again on the same data.
3. Click the project row "Open Paper Boats". Check: no recovery question shows.
4. Click Story in the Workspaces navigation, then click the Board tab.
5. Check: the cards are, in order, "Boats fold themselves at first light", "The river carries the boats away".
6. Check: the acts include "Act One [em dash] Launch".
7. Click the Characters tab.
8. Check: the workspace shows "Paper boat maker".
9. Check: the screen is quiet.
10. Click the Board tab.
11. Open the project menu and choose Close project.

## Notes

- On the Mac, Ctrl+Z and Ctrl+Y use Cmd instead.
