# Writing comfort

Spec: `tests/e2e/specs/42-writing-comfort.test.mjs`

Tags: `@e2e-data-root: own`

Before: an empty data folder of its own. The app starts for the first time.

"The elements" means the screenplay editor elements, read as "type: text". "The last element" is the bottom one. "An empty dialogue line" is a dialogue element with no text (it may show its "Dialogue" placeholder). "Check: Saved" means the editor toolbar status reads "Saved". "Check: the list offers X" means the suggestion list under the cursor shows the option X. "The Beat Board characters" is the list the Characters page shows (the spec asks the core for it).

## Smart suggestions build a scene without retyping and never change text silently

1. Do the first launch: type 'E2E Tester' as the name, click Continue, click Create Project, type 'Bus Depot' as the title, click Create.
2. Click Screenplay in the Workspaces navigation, then click New Screenplay.
3. Check: the editor opens. Click the first element.
4. Type 'IN'. Check: the list offers "INT.". Press Enter.
5. Type 'BUS DEPOT - '.
6. Type 'NI'. Check: the list offers "NIGHT". Press Enter.
7. Check: the first element is "scene_heading: INT. BUS DEPOT - NIGHT".
8. Press Enter. Type 'Meera waits for teh last bus.' (with the misspelling) and press Enter.
9. Type 'MEERA'. Check: the list offers "Create New: MEERA". Press Enter.
10. Check: the screen shows "Added MEERA to Characters".
11. Check: the last two elements are "character: MEERA" and an empty dialogue line.
12. Type 'Is it late?' and press Enter.
13. Press Enter again (an empty action line starts a new scene).
14. Type 'EXT. ROAD - LATER'.
15. Press Esc. Check: the suggestion list closes and the text stays as typed.
16. Press Enter. Type 'The bus is late.' and press Enter.
17. Type 'ME'. Check: the list offers "MEERA". Press Enter.
18. Check: there are two "character: MEERA" elements, and the last two elements are "character: MEERA" and an empty dialogue line.
19. Type 'Late.'
20. Check: Saved.
21. Check: the first five elements are "scene_heading: INT. BUS DEPOT - NIGHT", "action: Meera waits for teh last bus.", "character: MEERA", "dialogue: Is it late?", "scene_heading: EXT. ROAD - LATER".
22. Check: the screen is quiet.

## The element flow: Tab moves through the dialogue block, and a manual change sticks

1. Press Enter (the dialogue line is followed by an action line).
2. Press Tab (the action line becomes a character line).
3. Type 'RAVI'.
4. Press Ctrl+2.
5. Check: the last element is "action: RAVI".
6. Type ' runs.'
7. Check: the last element is "action: RAVI runs."
8. Check: Saved.
9. Check: the screen is quiet.

## One Enter after a new character name creates it and moves on to Dialogue; Undo takes each step back

1. Press Enter (the action line is followed by a character line).
2. Type 'NISHA'. Check: the list offers "Create New: NISHA". Press Enter.
3. Check: the screen shows "Added NISHA to Characters".
4. Check: the last two elements are "character: NISHA" and an empty dialogue line.
5. Check: the Beat Board characters include "NISHA".
6. Check: Saved.
7. Press Ctrl+Z.
8. Check: the last element is "character: NISHA" (only the Dialogue step was taken back).
9. Check: Saved.
10. Check: the Beat Board characters still include "NISHA".
11. Press Ctrl+Z.
12. Check: the Beat Board characters no longer include "NISHA".
13. Check: the last element is still "character: NISHA" (the typed name stays until its own undo).
14. Check: the screen is quiet.

## One Enter on a known name from the list fills it and moves on to Dialogue; Undo takes the Dialogue step back first

1. Click the last element and press End.
2. If the suggestion list is open (NISHA is no longer a character, so it offers Create New), press Esc.
3. Press Enter (to a dialogue line), Enter again (the empty dialogue line becomes an action line), then Tab (the action line becomes a character line).
4. Count the "character: MEERA" elements.
5. Type 'ME'. Check: the list offers "MEERA". Press Enter.
6. Check: the last two elements are "character: MEERA" and an empty dialogue line.
7. Check: Saved.
8. Press Ctrl+Z.
9. Check: the last element is "character: MEERA".
10. Check: Saved.
11. Press Ctrl+Z.
12. Check: the count of "character: MEERA" elements is back to the count from step 4.
13. Check: the screen is quiet.

## Spelling: unknown words are underlined, and Add to dictionary removes the underline

1. Check: within 10 seconds the word "teh" is underlined.
2. If no word is underlined at all, there is no English dictionary on this computer: check that nothing is underlined, and stop this scenario here.
3. Check: "MEERA" is not underlined (character names never are).
4. Right-click the underlined word "teh" and choose Add to dictionary.
5. Check: the screen shows "Added “teh” to your dictionary".
6. Check: "teh" is no longer underlined.
7. Check: the screen is quiet.

## Find and replace in this scene only, undone in one step

1. Click the second action element ("The bus is late.").
2. Check: the editor marks "The bus is late." as the current line.
3. Press Ctrl+F.
4. Click This scene.
5. Type 'bus' in the Find field and 'van' in the Replace with field.
6. Click Replace All.
7. Check: the screen shows "Replaced 1 match in Scene 2".
8. Check: the elements include "action: The van is late." and still include "action: Meera waits for teh last bus."
9. Click Close find (Esc).
10. Check: Saved.
11. Press Ctrl+Z.
12. Check: the elements include "action: The bus is late." again.
13. Check: the screen is quiet.

## Find scrolls the page to every match; Replace, Replace All, Match case and Whole word work

1. Click the second Action line, press End, then press Enter and type sixty short lines, each after an Enter. Lines 3 and 58 read "Walk 3, a torch burns." and "Walk 58, a torch burns."; the others read "Walk N goes on.".
2. Check: within 15 seconds the editor says "Saved", and the last line reads "Walk 60 goes on.".
3. Press Ctrl+F and type "torch" in the Find box.
4. Check: the count reads "1 of 2", and the highlighted match (line 3, far above the caret) is inside the visible part of the page.
5. Press Enter in the Find box.
6. Check: the count reads "2 of 2", and the highlighted match (line 58) is inside the visible part of the page.
7. Press Shift+Enter.
8. Check: the count reads "1 of 2" and the first match is on screen again.
9. Tick Match case and type "Torch": the count reads "No matches found". Untick Match case: the count reads "1 of 2".
10. Tick Whole word and type "torc": the count reads "No matches found". Untick Whole word and type "torch": the count reads "1 of 2".
11. Type "lamp" in Replace with and click Replace.
12. Check: line 3 reads "Walk 3, a lamp burns.", line 58 still reads "torch", the count reads "1 of 1", and the match on line 58 is on screen.
13. Click Replace All.
14. Check: the screen shows "Replaced 1 match", line 58 reads "lamp", and the count reads "No matches found".
15. Click Close find, check that the editor says "Saved" and that the screen is quiet.

## Navigator columns can be shown and are remembered

1. Click Navigator columns, then choose Page.
2. Check: the Scenes navigator shows "Page 1".
3. Click Navigator columns, then choose Notes.
4. Check: the Scenes navigator has a "Notes" column heading.
5. Check: the screen is quiet.

## A custom shortcut works in the editor and survives a relaunch; Restore all defaults

1. Open the project menu and choose Project settings.
2. Click Keyboard shortcuts.
3. Click Add a shortcut for Find in script, then press F3.
4. Check: the screen shows "Find in script: F3".
5. Check: the screen is quiet.
6. Open the project menu and choose Close project.
7. Quit the app and start it again on the same data.
8. Click the project row "Open Bus Depot". Check: no recovery question shows.
9. Click Screenplay in the Workspaces navigation.
10. Check: the editor shows at least five elements.
11. Check: the Scenes navigator still shows "Page 1" (the columns were remembered).
12. Click the first action element and press F3.
13. Check: the "Find in script" search bar opens.
14. Click Close find (Esc).
15. Open the project menu and choose Project settings, then click Keyboard shortcuts.
16. Click Restore all defaults, then click Restore all defaults in the dialog.
17. Check: the screen shows "All shortcuts are back to their defaults".
18. Check: the screen is quiet.
19. Open the project menu and choose Close project.

## Home reaches the same shortcut table from Help, and its tooltips name the keys

1. On Application Home, check: the New Project button has the tooltip "New Project (Ctrl+N)".
2. Click Help, then click Change shortcuts.
3. Check: a dialog opens with a table that has a "Command" column.
4. Click Add a shortcut for New Project in the dialog.
5. Press Esc.
6. Check: the dialog is still open (Esc belongs to the key recorder while it waits).
7. Press Esc again.
8. Check: the dialog closes.
9. Check: the screen is quiet.

## Notes

- On the Mac, Ctrl+2, Ctrl+Z and Ctrl+F use Cmd instead, and the tooltip would name Cmd+N. The exact Mac tooltip text is to decide in phase M-track.
- F3 is a Windows habit for Find. The Mac scenario may pick another free key.

## Platform notes

- The spelling check depends on an English dictionary on the computer (Windows spelling). Mac: the system spelling service; to decide in phase M-track.
