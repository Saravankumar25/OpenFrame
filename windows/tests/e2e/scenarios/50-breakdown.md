# Breakdown workspace basics

Spec: `tests/e2e/specs/50-breakdown.test.mjs`

Tags: `@e2e-data-root: own`

Before: an empty data folder of its own. The app starts for the first time.

"The scene panel" means the breakdown panel of the open scene ("Scene 1 breakdown"), without its "Quick reuse" section, which names items from other scenes on purpose. "Import the Night Shift screenplay" is the shared step in the README of this folder.

## Import a screenplay and choose it as the Production Source

1. Do the first launch: type 'E2E Tester' as the name, click Continue, click Create Project, type 'Copper Tunnel' as the title, click Create.
2. Import the Night Shift screenplay.
3. Click Breakdown in the Workspaces navigation.
4. Check: the screen shows "Choose a screenplay source to begin the breakdown."
5. Click Select Source.
6. Click the draft in the "Screenplay drafts" choice in the dialog.
7. Click Set as Production Source in the dialog.
8. Check: the screen shows "Production Source:".
9. Check: the panel "Scene 1 breakdown" shows.
10. Check: the screen is quiet.

## Suggestions are added only when accepted

1. Click Suggest Elements.
2. Check: the screen shows "Nothing is added to production until you accept."
3. In the scene panel, click the first button whose name starts with "Accept ". Note the element name after "Accept ".
4. Check: the screen shows "Accepted “<name>”".
5. Check: the scene panel shows that name.
6. Check: the screen is quiet.

## Add an element by hand, undo and redo, then edit its notes

1. Click Add Element.
2. Choose Props in the category list.
3. Type 'Flickering monitor' in the name field.
4. Check: the dialog no longer shows "Looking for existing catalog items".
5. Click Add to Scene 1 in the dialog.
6. Check: the screen shows "Added “Flickering monitor” to Scene 1".
7. Check: the scene panel shows "Flickering monitor".
8. Move focus off any text field and press Ctrl+Z.
9. Check: the scene panel no longer shows "Flickering monitor".
10. Press Ctrl+Y.
11. Check: the scene panel shows "Flickering monitor".
12. Click Actions for Flickering monitor, then choose Edit notes.
13. Type 'Must flicker on cue' in the notes field.
14. Click Save in the dialog.
15. Check: the scene panel shows "Must flicker on cue".
16. Check: the screen is quiet.

## Remove it from the scene, then restore it from Recently Deleted

1. Click Actions for Flickering monitor, then choose Remove from scene.
2. Check: the screen shows "Removed “Flickering monitor” from Scene 1. The catalog item is kept."
3. Check: the scene panel no longer shows "Flickering monitor".
4. Open the project menu and choose Recently Deleted.
5. Click Restore Flickering monitor (Scene 1).
6. Click Breakdown in the Workspaces navigation.
7. Check: the scene panel shows "Flickering monitor" and "Must flicker on cue".
8. Check: the screen is quiet.

## Mark the scene complete; scene 2 has its own breakdown

1. Click Mark Complete.
2. Check: the panel "Scene 1 breakdown" shows a Complete button.
3. Click the second scene in the list "Scenes in the Production Source".
4. Check: the panel "Scene 2 breakdown" shows.
5. Check: the scene panel does not show "Flickering monitor".
6. Check: the screen is quiet.

## After a relaunch, the source, the element, its notes and the completion are kept

1. Open the project menu and choose Close project.
2. Quit the app and start it again on the same data.
3. Click the project row "Open Copper Tunnel". Check: no recovery question shows.
4. Click Breakdown in the Workspaces navigation.
5. Check: the screen shows "Production Source:".
6. Click the first scene in the list "Scenes in the Production Source".
7. Check: the panel "Scene 1 breakdown" shows "Flickering monitor" and "Must flicker on cue".
8. Check: the panel "Scene 1 breakdown" shows a Complete button.
9. Check: the screen is quiet.
10. Open the project menu and choose Close project.

## Notes

- On the Mac, Ctrl+Z and Ctrl+Y use Cmd instead.
