# Screenplay workspace basics

Spec: `tests/e2e/specs/40-screenplay.test.mjs`

Tags: `@e2e-data-root: own`

Before: an empty data folder of its own. The app starts for the first time.

"The elements" means the screenplay editor elements, read as "type: text", for example "dialogue: It has to be perfect." "Check: Saved" means the editor toolbar status reads "Saved". The draft switcher button is named "Draft: <name>. Switch draft".

## A new screenplay: write a scene with the Enter flow, and it saves itself

1. Do the first launch: type 'E2E Tester' as the name, click Continue, click Create Project, type 'Glass Harbour' as the title, click Create.
2. Click Screenplay in the Workspaces navigation.
3. Click New Screenplay.
4. Check: the editor opens with at least one element.
5. Note the name of the draft from the draft switcher. Check: it has a name. Call it the first draft.
6. Click the first element.
7. Type 'INT. GLASS WORKSHOP [em dash] NIGHT' and press Enter.
8. Type 'Asha shapes a lens over the blue flame.' and press Enter.
9. Type 'ASHA'.
10. Check: the suggestion list offers "Create New: ASHA". Press Esc. Check: the list closes.
11. Press Enter.
12. Type 'It has to be perfect.'
13. Check: the first four elements are "scene_heading: INT. GLASS WORKSHOP [em dash] NIGHT", "action: Asha shapes a lens over the blue flame.", "character: ASHA", "dialogue: It has to be perfect."
14. Check: Saved.
15. Check: the screen is quiet.

## Undo and redo typing from inside the editor

1. Wait 4.5 seconds (typing more than 4 seconds apart is a separate undo step).
2. Type ' Now.'
3. Check: the elements include "dialogue: It has to be perfect. Now."
4. Check: Saved.
5. Press Ctrl+Z, with the cursor still in the editor.
6. Check: the elements include "dialogue: It has to be perfect."
7. Press Ctrl+Y.
8. Check: the elements include "dialogue: It has to be perfect. Now."
9. Check: Saved.
10. Check: the screen is quiet.

## A second draft is edited on its own, renamed, deleted and restored

1. Click the draft switcher of the first draft, then choose New Draft.
2. Select all text in the draft name field and type 'Rewrite A'.
3. Click Create Draft in the dialog.
4. Check: the screen shows "Created draft “Rewrite A”".
5. Check: the draft switcher reads "Draft: Rewrite A. Switch draft".
6. Check: the elements include "dialogue: It has to be perfect. Now." (the new draft is a copy).
7. Click the last dialogue element, press End and type ' Again.'
8. Check: the elements include "dialogue: It has to be perfect. Now. Again."
9. Check: Saved.
10. Click the draft switcher "Draft: Rewrite A. Switch draft" and choose the first draft.
11. Check: the draft switcher names the first draft.
12. Check: the elements include "dialogue: It has to be perfect. Now." (the first draft did not change).
13. Click Drafts in the Screenplay toolbar. Check: the Drafts drawer opens.
14. Click Actions for Rewrite A, then choose Rename.
15. Select all text in the draft name field and type 'Rewrite B'. Click Save in the dialog.
16. Check: the Drafts drawer lists "Rewrite B".
17. If "Rewrite B" is marked Current, click Actions for the first draft, choose Mark current, and check the first draft is now marked Current. (Only a draft that is not current can be deleted.)
18. Click Actions for Rewrite B, then choose Delete.
19. Click Delete Draft in the dialog.
20. Check: the screen shows "Moved “Rewrite B” to Recently Deleted".
21. Check: the Drafts drawer no longer lists "Rewrite B".
22. Click Close in the Drafts drawer.
23. Open the project menu and choose Recently Deleted.
24. Click Restore Rewrite B.
25. Check: the screen shows "Restored “Rewrite B”".
26. Click Screenplay in the Workspaces navigation.
27. Click the draft switcher of the first draft and choose Rewrite B.
28. Check: the elements include "dialogue: It has to be perfect. Now. Again."
29. Check: the screen is quiet.

## After a relaunch, both drafts and their text are kept

1. Open the project menu and choose Close project.
2. Quit the app and start it again on the same data.
3. Click the project row "Open Glass Harbour". Check: no recovery question shows.
4. Click Screenplay in the Workspaces navigation.
5. Check: the editor shows at least four elements.
6. Check: if the open draft is Rewrite B, the elements include "dialogue: It has to be perfect. Now. Again."; otherwise they include "dialogue: It has to be perfect. Now."
7. Click the draft switcher and choose the other draft.
8. Check: the other draft shows its own dialogue line in the same way.
9. Check: the first three elements are "scene_heading: INT. GLASS WORKSHOP [em dash] NIGHT", "action: Asha shapes a lens over the blue flame.", "character: ASHA".
10. Check: the screen is quiet.
11. Open the project menu and choose Close project.

## Notes

- On the Mac, Ctrl+Z and Ctrl+Y use Cmd instead.
