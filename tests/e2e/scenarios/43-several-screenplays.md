# Several screenplays per project

Spec: `tests/e2e/specs/43-several-screenplays.test.mjs`

Tags: `@e2e-data-root: own`

Before: an empty data folder of its own. The app starts for the first time.

"Picker X, then Y" means: click the screenplay picker button named "Screenplay: X. Switch screenplay", then choose the menu item that starts with Y. "Check: Saved" means the editor toolbar status reads "Saved". "The screenplay list" is the list of screenplay titles the core reports for the project.

## Add a second screenplay, write in both and switch between them

1. Do the first launch: type 'E2E Tester' as the name, click Continue, click Create Project, type 'Two Rivers' as the title, click Create.
2. Click Screenplay in the Workspaces navigation, then click New Screenplay.
3. Check: the editor opens. Click the first element and type 'The feature opens on the river.'
4. Check: Saved.
5. Picker "Two Rivers", then New Screenplay.
6. Select all text in the title field and type 'Two Rivers'.
7. Check: the screen shows "Another screenplay here already has this title".
8. Select all text in the title field, type 'Two Rivers (short)' and press Enter.
9. Check: the picker reads "Screenplay: Two Rivers (short). Switch screenplay".
10. Click the first element and type 'The short opens on the bridge.'
11. Check: Saved.
12. Check: no line ends with "The feature opens on the river." (the short starts empty).
13. Picker "Two Rivers (short)", then "Two Rivers ·".
14. Check: a line ends with "The feature opens on the river."
15. Click that line, press End and type ' At dawn.'
16. Check: Saved.
17. Picker "Two Rivers", then Two Rivers (short).
18. Check: a line ends with "The short opens on the bridge."
19. Picker "Two Rivers (short)", then Rename Screenplay.
20. Select all text in the title field and type 'Two Rivers: The Short'.
21. Click Rename in the dialog.
22. Check: the picker reads "Screenplay: Two Rivers: The Short. Switch screenplay".
23. Check: the screenplay list is "Two Rivers", "Two Rivers: The Short".
24. Check: the screen is quiet.

## After a relaunch, both screenplays keep their own text

1. Open the project menu and choose Close project.
2. Quit the app and start it again on the same data.
3. Click the project row "Open Two Rivers". Check: no recovery question shows.
4. Click Screenplay in the Workspaces navigation.
5. Check: a line ends with "The short opens on the bridge." (the screenplay open last comes back).
6. Picker "Two Rivers: The Short", then "Two Rivers ·".
7. Check: a line ends with "The feature opens on the river. At dawn."
8. Check: the project has two screenplays, each with one draft.
9. Check: the screen is quiet.
10. Open the project menu and choose Close project.

## Delete a screenplay to Recently Deleted and restore it with its text

1. Click the project row "Open Two Rivers". Check: no recovery question shows.
2. Click Screenplay in the Workspaces navigation.
3. Check: a line ends with "The feature opens on the river. At dawn."
4. Picker "Two Rivers", then Two Rivers: The Short.
5. Check: a line ends with "The short opens on the bridge."
6. Picker "Two Rivers: The Short", then Delete Screenplay.
7. Check: the screen shows "Delete “Two Rivers: The Short”?" and "They stay there for 30 days".
8. Click Delete Screenplay in the dialog.
9. Check: the picker reads "Screenplay: Two Rivers. Switch screenplay".
10. Check: a line ends with "The feature opens on the river. At dawn."
11. Check: the screenplay list is only "Two Rivers".
12. Click the project button "Project: Two Rivers. Switch or manage projects", then choose Recently Deleted.
13. Check: the screen shows "Two Rivers: The Short".
14. Click Restore Two Rivers: The Short.
15. Click Screenplay in the Workspaces navigation.
16. Picker "Two Rivers", then Two Rivers: The Short.
17. Check: a line ends with "The short opens on the bridge."
18. Check: the screenplay list is "Two Rivers", "Two Rivers: The Short".
19. Check: the screen is quiet.
20. Open the project menu and choose Close project.

## Notes

- The menu item "Two Rivers ·" is the feature screenplay; its label carries more text after the dot.
