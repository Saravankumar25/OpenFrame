# Beat Board full flow

Spec: `tests/e2e/specs/31-story-board-flow.test.mjs`

Tags: `@e2e-data-root: own`

Before: an empty data folder of its own. The app starts for the first time.

The cards used below:

- A: 'Rain finds the letter box'
- B: 'Meera reads the first letter'
- C: 'The postman never came back'
- Beat: 'A second letter arrives'

"Add a card X" means: click Add, choose Scene Card, type X in the Short description field, press Enter, then check the card shows. "The cards" means the scene cards on the board, in order. "The cards in Act One" means the cards inside the section "Act Act One".

## Create an act, a sequence and three cards

1. Do the first launch: type 'E2E Tester' as the name, click Continue, click Create Project, type 'Monsoon Letters' as the title, click Create.
2. Click Story in the Workspaces navigation, then click the Board tab.
3. Check: the screen shows "Your Beat Board is empty."
4. Click Add Act. Type 'Act One' in the Act title field and press Enter.
5. Click Add, then choose Sequence. Type 'The Letters' in the Sequence name field and press Enter.
6. Check: the screen shows "The Letters".
7. Add a card A, then B, then C.
8. Check: the cards are A, B, C.
9. Check: the screen is quiet.

## Drag a card with the mouse, then undo and redo

1. Press the mouse on card C, move it up past 6 pixels, then drag it onto the top quarter of card A and release.
2. Check: the cards are C, A, B.
3. Move focus off any text field and press Ctrl+Z.
4. Check: the cards are A, B, C.
5. Press Ctrl+Y.
6. Check: the cards are C, A, B.
7. Check: the screen is quiet.

## Parking Lot: park a card and restore it to its place

1. If the Parking Lot button is not pressed, click it.
2. Right-click card A and choose Send to Parking Lot.
3. Check: the Parking Lot holds card A.
4. Check: the cards in Act One are C, B.
5. Right-click card A and choose Restore to Story.
6. Check: the cards in Act One are C, A, B.
7. Check: the screen is quiet.

## Convert a beat to a scene card

1. Click card B.
2. Click Add, then choose Beat.
3. Type 'A second letter arrives' in the Beat text field and press Enter.
4. Check: the beat "Beat: A second letter arrives" shows.
5. Right-click the beat and choose Convert to Scene.
6. Check: the cards include "A second letter arrives".
7. Check: the beat now reads "Beat: A second letter arrives (converted)".
8. Press Ctrl+Z.
9. Check: the cards no longer include "A second letter arrives".
10. Press Ctrl+Y.
11. Check: the cards include "A second letter arrives" again.
12. Check: the screen is quiet.

## A long scene description and the Card Display presets

1. Double-click card A.
2. Type 'The street is flooded to the knees. Meera wades to the box with a torch in her teeth.' in the long description field.
3. Check: the status in the Scene Card drawer reads "Saved".
4. Click Close in the Scene Card drawer.
5. Check: no card shows a long description preview (the Minimal preset is the default).
6. Click More Beat Board actions, then choose Card Display Settings.
7. Choose the preset Story in the dialog.
8. Check: a card shows a long description preview starting with "The street is flooded".
9. Tick Shot/Storyboard Status in the dialog.
10. Check: the preset Custom becomes selected.
11. Click Done in the dialog.
12. Check: the cards in Act One are C, A, B, then "A second letter arrives" (the layout changed no story data).
13. Check: the screen is quiet.

## Build Screenplay leaves out a card without a heading and never invents one

1. Click Build Screenplay.
2. Check: the screen shows "Build Screenplay from Beat Board", "no valid scene heading" and "OpenFrame never makes up a heading."
3. Type 'EXT. POST OFFICE - NIGHT' in Heading for scene 1.
4. Type 'EXT. FLOODED STREET - NIGHT' in Heading for scene 2.
5. Type 'INT. MEERA'S ROOM - NIGHT' in Heading for scene 3.
6. Check: Build Screenplay in the dialog is disabled.
7. Click Leave out scene 4.
8. Check: Build Screenplay becomes enabled.
9. Check: the screen shows "3 of 4 cards will be built".
10. Click Build Screenplay in the dialog.
11. Check: the Screenplay workspace opens and the screen shows "Draft 1 from Beat Board".
12. Check: the scene headings of the draft are exactly "EXT. POST OFFICE - NIGHT", "EXT. FLOODED STREET - NIGHT", "INT. MEERA'S ROOM - NIGHT", in that order.
13. Check: the screen is quiet.
14. Click Story in the Workspaces navigation, then click the Board tab.

## Delete two cards together and restore them in reverse order

1. Click card A, then hold Ctrl and click card B.
2. Check: two cards are selected.
3. Press Delete.
4. Check: the screen shows "Delete 2 items?" and "Screenplay scenes are never deleted from the Beat Board."
5. Click Delete in the dialog.
6. Check: cards A and B are gone.
7. Open the project menu and choose Recently Deleted.
8. Click Restore Meera reads the first letter. Check: the screen shows "Restored “Meera reads the first letter”".
9. Click Restore Rain finds the letter box. Check: the screen shows "Restored “Rain finds the letter box”".
10. Click Story in the Workspaces navigation, then click the Board tab.
11. Check: the cards in Act One are C, A, B, then "A second letter arrives".
12. Check: the screen is quiet.

## Horizontal layout: acts become rows, a card moves to another act, then back to Vertical

1. In the Beat Board layout toggle, click Horizontal.
2. Check: the board shows the Horizontal layout.
3. Click Add Act. Type 'Act Two' in the Act title field and press Enter.
4. Check: Act One, Act Two and the Parking Lot share the same left edge and sit one below the other, top to bottom, and an act row is wider than two columns.
5. Check: in Act One, card A sits to the right of card C on the same line (cards flow left to right).
6. Press the mouse on the card "A second letter arrives", move it up past 6 pixels, drag it into the body of Act Two and release.
7. Check: the cards in Act Two are "A second letter arrives", and the cards in Act One are C, A, B.
8. Save a screenshot of the Horizontal board as `beatboard-horizontal.png` in the test artifacts.
9. Press Ctrl+Z.
10. Check: the cards in Act One are C, A, B, then "A second letter arrives".
11. In the Beat Board layout toggle, click Vertical.
12. Check: the board shows the Vertical layout, and Act Two sits to the right of Act One, level with it.
13. Check: the screen is quiet.

## After a relaunch, the order, the long description and the Card Display setting are kept

1. Open the project menu and choose Close project.
2. Quit the app and start it again on the same data.
3. Click the project row "Open Monsoon Letters". Check: no recovery question shows.
4. Click Story in the Workspaces navigation, then click the Board tab.
5. Check: the cards in Act One are C, A, B, then "A second letter arrives".
6. Check: a card shows a long description preview starting with "The street is flooded".
7. Double-click card A.
8. Check: the long description field reads exactly 'The street is flooded to the knees. Meera wades to the box with a torch in her teeth.'
9. Click Close in the Scene Card drawer.
10. Check: the screen is quiet.
11. Open the project menu and choose Close project.

## Notes

- On the Mac, Ctrl+Z, Ctrl+Y and Ctrl+click use Cmd instead.
- The drag is a real mouse drag with a pause before the release, not a keyboard move.
