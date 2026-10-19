# Project pages

Spec: `tests/e2e/specs/85-project-pages.test.mjs`

Tags: `@e2e-data-root: own`

Before: an empty data folder of its own. The app starts for the first time.

"The templates" are the entries of the "This project" section on the Templates tab, named "Preview <name>, ...". "Templates tab" means: open the project menu, choose Project settings, click the Templates tab in the Settings tabs, and check the screen shows "Create Template".

"Make a template X" means: click Create Template, type X in the template name field, click Create in the dialog, then check the templates include X.

"Delete the template X" means: click Actions for X, choose Delete, check the screen shows "Moved template “X” to Recently Deleted", then check the templates no longer include X.

## Project settings: change the title and logline, save, undo, redo

1. Do the first launch: type 'E2E Tester' as the name, click Continue, click Create Project, type 'Tide Line' as the title, click Create.
2. Open the project menu and choose Project settings.
3. Select all text in the title field and type 'Tide Line Revised'.
4. Type 'A ferry pilot races the tide to reach her brother.' in the logline field.
5. Click Save.
6. Check: the screen shows "Updated project settings".
7. Check: the window title is "Tide Line Revised - OpenFrame Studio".
8. Move focus off any text field and press Ctrl+Z.
9. Check: the window title is "Tide Line - OpenFrame Studio" and the logline field is empty.
10. Press Ctrl+Y.
11. Check: the window title is "Tide Line Revised - OpenFrame Studio" and the logline field reads "A ferry pilot races the tide to reach her brother."
12. Check: the screen is quiet.

## Templates: make two, delete both, restore one, delete the other for good

1. Templates tab.
2. Make a template 'Night exterior call'.
3. Make a template 'Old call template'.
4. Delete the template "Night exterior call".
5. Delete the template "Old call template".
6. Check: the screen is quiet.
7. Open the project menu and choose Recently Deleted.
8. Click Restore Night exterior call.
9. Check: the screen shows "Restored “Night exterior call”".
10. Click Delete Old call template permanently.
11. Check: the screen shows "Delete “Old call template” permanently?"
12. Click Delete Permanently in the dialog.
13. Check: the screen shows "“Old call template” was deleted permanently."
14. Check: the workspace no longer shows "Old call template".
15. Check: the screen is quiet.
16. Templates tab.
17. Check: the templates include "Night exterior call".

## Activity lists what happened and filters by kind

1. Open the project menu and choose Activity.
2. Check: the screen shows "Activity History".
3. Check: the Activity list has at least one entry, and one entry names "E2E Tester".
4. Click Story in the "Show activity" choice.
5. Check: the screen shows "Nothing of this kind has happened yet."
6. Click All in the "Show activity" choice.
7. Check: the Activity list shows.
8. Check: the screen is quiet.

## After a relaunch, the settings, the restored template and the permanent delete are kept

1. Open the project menu and choose Close project.
2. Quit the app and start it again on the same data.
3. Click the project row "Open Tide Line Revised". Check: no recovery question shows.
4. Open the project menu and choose Project settings.
5. Check: the logline field reads "A ferry pilot races the tide to reach her brother."
6. Templates tab.
7. Check: the templates include "Night exterior call" and do not include "Old call template".
8. Open the project menu and choose Recently Deleted.
9. Check: the screen shows "Nothing has been deleted."
10. Check: the screen is quiet.
11. Open the project menu and choose Close project.

## Notes

- On the Mac, Ctrl+Z and Ctrl+Y use Cmd instead.
