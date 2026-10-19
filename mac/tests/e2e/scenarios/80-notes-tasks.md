# Notes and tasks

Spec: `tests/e2e/specs/80-notes-tasks.test.mjs`

Tags: `@e2e-data-root: own`

Before: an empty data folder of its own. The app starts for the first time.

"The notes" are the entries of the Project Notes list. A task's box is named "Mark “<title>” done" while open and "Reopen “<title>”" when done.

## A new note saves its title and text by itself; undo and redo the edit

1. Do the first launch: type 'E2E Tester' as the name, click Continue, click Create Project, type 'Tide Tables' as the title, click Create.
2. Open the project menu and choose Notes & Tasks.
3. Check: the screen shows "No notes yet."
4. Click New Note.
5. Type 'Location scout' in the note title field and 'Check the old pier at low tide.' in the note body field.
6. Click Close in the drawer. Check: the drawer closes.
7. Check: a note starting with "Location scout" is listed.
8. Move focus off any text field and press Ctrl+Z.
9. Check: no note starting with "Location scout" is listed.
10. Press Ctrl+Y.
11. Check: a note starting with "Location scout" is listed.
12. Check: the screen is quiet.

## Delete the note and restore it from Recently Deleted

1. Click the note "Location scout".
2. Click Delete in the drawer.
3. Check: the screen shows "Moved note “Location scout” to Recently Deleted".
4. Check: the notes list is empty.
5. Open the project menu and choose Recently Deleted.
6. Click Restore Location scout.
7. Check: the screen shows "Restored “Location scout”".
8. Open the project menu and choose Notes & Tasks.
9. Check: a note starting with "Location scout" is listed.
10. Check: the screen is quiet.

## Add a task, complete it, undo and redo the completion

1. Click New Task. Type 'Confirm pier permit' in the task title field. Click Add Task.
2. Check: the screen shows "Added task “Confirm pier permit”".
3. Tick the box "Mark “Confirm pier permit” done".
4. Check: the screen shows "Completed task “Confirm pier permit”".
5. Press Ctrl+Z.
6. Check: the box "Mark “Confirm pier permit” done" shows again (the task is open).
7. Press Ctrl+Y.
8. Click All in the "Show tasks" choice.
9. Check: the box "Reopen “Confirm pier permit”" shows (the task is done).
10. Check: the screen is quiet.

## After a relaunch, the note and the completed task are kept

1. Open the project menu and choose Close project.
2. Quit the app and start it again on the same data.
3. Click the project row "Open Tide Tables". Check: no recovery question shows.
4. Open the project menu and choose Notes & Tasks.
5. Check: a note starting with "Location scout" is listed.
6. Click All in the "Show tasks" choice.
7. Check: the box "Reopen “Confirm pier permit”" shows.
8. Click the note "Location scout".
9. Check: the note body reads exactly "Check the old pier at low tide."
10. Click Close in the drawer.
11. Check: the screen is quiet.
12. Open the project menu and choose Close project.

## Notes

- On the Mac, Ctrl+Z and Ctrl+Y use Cmd instead.
