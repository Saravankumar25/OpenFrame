# Application Home and Project Home

Spec: `tests/e2e/specs/10-home.test.mjs`

Tags: `@e2e-data-root: own`

Before: an empty data folder of its own. The app starts for the first time.

"Refresh Home" below means: click Global Idea Vault, then click Your projects, then check the screen shows "Pick up where you left off".

Project cards on Application Home are named "Open <title>, <type>, <status>", for example "Open Harbour Lights, Feature Film, Writing".

## First launch creates a project that opens on an empty Project Home

1. Do the first launch: type 'E2E Tester' as the name, click Continue, click Create Project, type 'Harbour Lights' as the title, click Create.
2. Check: the window title is "Harbour Lights - OpenFrame Studio".
3. Check: the screen shows "Harbour Lights".
4. Check: the list "Ways to begin" shows.
5. Check: the screen is quiet.

## Change the status on Project Home, then undo and redo

1. Check: the status button reads "Status: Idea. Change status".
2. Click the status button, then choose Writing.
3. Check: the status button reads "Status: Writing. Change status".
4. Check: the screen shows "Changed project status to Writing".
5. Move focus off any text field and press Ctrl+Z.
6. Check: the status button reads "Status: Idea. Change status".
7. Press Ctrl+Y.
8. Check: the status button reads "Status: Writing. Change status".
9. Check: the screen is quiet.

## Application Home shows a renamed project without leaving the page

1. Open the project menu and choose Close project.
2. Check: Application Home shows and the project shell is gone.
3. Check: the only project card is "Open Harbour Lights, Feature Film, Writing".
4. Click More actions for Harbour Lights, then choose Rename.
5. Select all text in the title field, then type 'Harbour Lights Redux'.
6. Click Rename in the dialog.
7. Check: the screen shows "Renamed to “Harbour Lights Redux”."
8. Check: the cards include "Open Harbour Lights Redux, Feature Film, Writing".

## Application Home: duplicate, search, archive and restore

1. Refresh Home.
2. Check: the cards include "Open Harbour Lights Redux, Feature Film, Writing".
3. Click More actions for Harbour Lights Redux, then choose Duplicate Project.
4. Check: the screen shows "It is a separate project with its own identity."
5. Refresh Home.
6. Check: there are two project cards. The second one is a Feature Film whose title is not "Harbour Lights Redux". Call that title the copy title.
7. Type 'Redux' in Search projects.
8. Check: every card shown contains "Redux".
9. Clear Search projects.
10. Check: two cards show again.
11. Click More actions for the copy, then choose Archive.
12. Check: the screen shows "“<copy title>” was archived."
13. Refresh Home.
14. Check: only one card shows.
15. Click Open Archived (1).
16. Check: the screen shows "Archived projects" and the copy title.
17. Click Restore.
18. Check: the screen shows "“<copy title>” is back in your projects."
19. Click Back to projects.
20. Refresh Home.
21. Check: two cards show.
22. Check: the screen is quiet.

## After a relaunch, both projects, the new title and the status are kept

1. Quit the app and start it again on the same data.
2. Check: Application Home shows two cards, one of them "Open Harbour Lights Redux, Feature Film, Writing".
3. Click the card "Open Harbour Lights Redux".
4. Check: no recovery question shows.
5. Check: the window title is "Harbour Lights Redux - OpenFrame Studio".
6. Check: the status button reads "Status: Writing. Change status".
7. Check: the screen is quiet.
8. Open the project menu and choose Close project.

## Delete Project: the typed name confirms, the folder leaves, Home forgets it after a relaunch

1. Note how many items the test bin folder of the data folder holds.
2. Click More actions for the copy project (the card that is not Harbour Lights Redux), then choose Delete Project.
3. Check: the confirm field shows, and the Delete Project button in the dialog is disabled.
4. Type the copy title in the confirm field.
5. Check: Delete Project becomes enabled.
6. Click Delete Project.
7. Check: the screen shows "“<copy title>” was moved to the Windows Recycle Bin."
8. Refresh Home.
9. Check: only one card shows.
10. Check: the test bin folder holds one more item than before.
11. Quit the app and start it again on the same data.
12. Check: the only card is "Open Harbour Lights Redux, Feature Film, Writing".
13. Check: the screen is quiet.

## Notes

- On the Mac, Ctrl+Z and Ctrl+Y use Cmd instead.

## Platform notes

- Delete Project moves the project folder to the Windows Recycle Bin, and the message names it. A debug build on a test data folder moves it into a `recycle-bin` folder inside the data folder instead, so the test never touches the computer's own bin. Mac: the Trash; the message text and the test folder are to decide in phase M-track.
