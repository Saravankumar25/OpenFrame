# Recently Deleted 30-day purge

Spec: `tests/e2e/specs/25-recently-deleted-purge.test.mjs`

Tags: `@e2e-data-root: own`

Before: an empty data folder of its own. The app starts for the first time.

"Make a note X" means: click New Note, type X in the note title field, type 'X: written by the end-to-end suite.' in the note body field, click Close in the drawer, then check the Project Notes list has a note starting with X.

"Delete the note X" means: click the note X in the Project Notes list, click Delete in the drawer, check the screen shows "Moved note “X” to Recently Deleted", then check the note is gone from the list.

## Deleted items show 30 days left on both Recently Deleted screens

1. Do the first launch: type 'E2E Tester' as the name, click Continue, click Create Project, type 'Low Tide' as the title, click Create.
2. Open the project menu and choose Notes & Tasks.
3. Check: the screen shows "No notes yet."
4. Make a note 'Harbour master'.
5. Make a note 'Old ferry route'.
6. Make a note 'New ferry route'.
7. Delete the note "Old ferry route".
8. Delete the note "New ferry route".
9. Click Idea Vault in the Workspaces navigation.
10. Click Quick Capture, type 'A ferry that only sails when nobody is waiting.' in the Idea field, click Save in the dialog.
11. Check: the screen shows "Saved to the Project Idea Vault" and the Vault has one card.
12. Click the card and press Delete.
13. Check: the Vault has no cards.
14. Click Recently Deleted.
15. Check: the screen shows "Deleted items stay here for 30 days, then they are removed for good."
16. Check: the table has a "Removed in" column and exactly one row.
17. Check: that row shows "30 days left".
18. Check: the screen is quiet.
19. Click Back to Vault.
20. Open the project menu and choose Recently Deleted.
21. Check: the screen shows "Deleted items stay here for 30 days, then they are removed for good." and the table has a "Removed in" column.
22. Check: the row for "Old ferry route" shows "30 days left".
23. Check: the row for "New ferry route" shows "30 days left".
24. Check: no row shows "Harbour master" (a live note is never in Recently Deleted).
25. Check: the screen is quiet.

## After 30 days, the next open removes what is due, and nothing else

1. Open the project menu and choose Close project.
2. Quit the app and wait about 1.5 seconds.
3. With the app closed, change the project file so the deletes look older: "Old ferry route" deleted 31 days ago, "New ferry route" deleted 29 and a half days ago, the Vault idea deleted 40 days ago.
4. Start the app.
5. Check: the screen shows "Your projects".
6. Click the project row "Open Low Tide". Check: no recovery question shows.
7. Open the project menu and choose Recently Deleted.
8. Check: a row for "New ferry route" shows "1 day left".
9. Check: there is no row for "Old ferry route".
10. Check: the table has exactly one row.
11. Open the project menu and choose Notes & Tasks.
12. Check: the note "Harbour master" is listed.
13. Check: the note "Old ferry route" is not listed.
14. Click Idea Vault in the Workspaces navigation, then click Recently Deleted.
15. Check: the screen shows "Nothing deleted".
16. Click Back to Vault.
17. Check: the screen is quiet.

## The Owner can still delete an item for good before its 30 days are up

1. Open the project menu and choose Recently Deleted.
2. Click Delete New ferry route permanently.
3. Click Delete Permanently in the dialog.
4. Check: there is no row for "New ferry route".
5. Check: the screen is quiet.

## After a relaunch, nothing removed comes back and the live note stays

1. Open the project menu and choose Close project.
2. Quit the app and start it again on the same data.
3. Click the project row "Open Low Tide". Check: no recovery question shows.
4. Open the project menu and choose Notes & Tasks.
5. Check: the note "Harbour master" is listed.
6. Check: neither "Old ferry route" nor "New ferry route" is listed.
7. Check: the screen is quiet.
8. Open the project menu and choose Close project.

## Notes

- Step 3 of the second scenario changes the delete times of the Recently Deleted record and of the deleted item itself, straight in the SQLite project file, while no app holds it. It is a test setup step, not something a person does. No network is used at any point.
- The project file is the one `project.sqlite` under the data folder's `projects` folder, never a backup copy.
