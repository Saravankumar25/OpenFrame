# Packages and backup

Spec: `tests/e2e/specs/70-packages.test.mjs`

Tags: `@e2e-data-root: own`

Before: an empty data folder of its own, with an empty `package-out` folder inside it. The app starts for the first time.

File names used below, all in `package-out`:

- the backup: `Harbour Lights backup.ofbackup`
- the project package: `Harbour Lights.ofproject`
- the review package: `Harbour Lights review.ofscriptreview`
- the response package: `Harbour Lights reply.ofresponse`
- the damaged backup: `damaged.ofbackup`

"Choose F in the system dialog T" means: the system file dialog titled T opens, type the full path of F in its File name box and press its Save or Open button. "Import the Night Shift screenplay" is the shared step in the README of this folder.

## Create a backup through Create Backup and the Save dialog

1. Do the first launch: type 'E2E Tester' as the name, click Continue, click Create Project, type 'Harbour Lights' as the title, click Create.
2. Import the Night Shift screenplay.
3. Click the first element of the screenplay.
4. Click Notes in the Screenplay toolbar, then click Add private note.
5. Type 'Harbour private note for the restore check' in the Private note field, then click Save on the private note.
6. Check: the screen shows "Harbour private note for the restore check".
7. Open the project menu and choose Create Backup…
8. Check: the screen shows "A recoverable copy of supported project state."
9. Click Browse… in the dialog. Choose the backup file in the system dialog "Save backup to".
10. Check: the Backup location field shows the full path of the backup file.
11. Click Create Backup in the dialog.
12. Check: within 60 seconds the screen shows "Backup created."
13. Check: the backup file exists on disk.
14. Click Close in the dialog.
15. Check: the screen is quiet.

## Export a Full Project Package and a screenplay review package

1. Open the project menu and choose Export Full Project…
2. Check: the screen shows "Private project transfer."
3. Click Export Project Package… in the dialog. Choose the project package file in the system dialog "Export Project Package".
4. Check: within 60 seconds the screen shows "Project package exported."
5. Click Close in the dialog.
6. Check: the project package file exists on disk.
7. Open the project menu and choose Export Review Package (script)…
8. Check: the screen shows "Script Review Package", and then "Private notes" once the preview has loaded.
9. Click Export Package… in the dialog. Choose the review package file in the system dialog "Export Screenplay review package".
10. Check: within 30 seconds the screen shows "Package exported."
11. Click Close in the dialog.
12. Check: the review package file exists on disk.
13. Open the project menu and choose Import & Export History.
14. Click Package log in the dialog.
15. Check: the screen shows "Created backup" and "Exported project package".
16. Press Esc.
17. Check: the screen is quiet.

## Review round trip: comment as the reviewer, send a response, preview and import it

1. Open the project menu and choose Open Review Package… Choose the review package file in the system dialog "Open Review Package".
2. Check: within 20 seconds the screen shows "Review package. Comments are separate from the original project."
3. Click the Comment field and type 'Tighten the harbour line.'
4. Press Ctrl+Enter.
5. Check: the screen shows "Yours".
6. Click Export Response Package. Choose the response package file in the system dialog "Export Response Package".
7. Check: within 30 seconds the screen shows "Response package exported."
8. Check: the response package file exists on disk.
9. Click Close review.
10. Open the project menu and choose Import Exchange Package…
11. Click Choose Package… in the dialog. Choose the response package file in the system dialog "Import Exchange Package".
12. Click Show Comments in the dialog.
13. Check: the screen shows "Tighten the harbour line." (the preview lists it before anything is imported).
14. Click Accept All Safe in the dialog.
15. Check: within 30 seconds the screen shows "Imported:".
16. Click Close in the dialog.
17. Check: the screen is quiet.

## Restore the backup from Application Home: the collision choice opens it as a copy

1. Open the project menu and choose Close project.
2. Click Restore Backup or Package…
3. Click Choose Package… in the dialog. Choose the backup file in the system dialog "Open Project Package".
4. Check: the screen shows "This project already exists on this computer".
5. Check: the screen shows "The private note in this package will become yours."
6. Check: the screen shows "Open as Copy".
7. Click Open as Copy in the dialog.
8. Check: within 60 seconds the screen shows "Opened as a copy."
9. Click Close in the dialog.
10. Check: the window title is "Harbour Lights (copy) - OpenFrame Studio".
11. Check: the screen is quiet.

## A damaged backup is refused and nothing is created

1. Write the damaged backup file: a few bytes that start like a zip archive and then stop ("PK", two control bytes, then " this is not a whole package").
2. Open the project menu and choose Close project.
3. Note the project cards on Application Home.
4. Click Restore Backup or Package…
5. Click Choose Package… in the dialog. Choose the damaged backup file in the system dialog "Open Project Package".
6. Check: the screen shows a message with the word "package" or "archive", and still offers "Choose Package…".
7. Click Cancel in the dialog.
8. Check: the project cards are the same as before.

## After a relaunch, the original and the restored copy are both listed

1. Quit the app and start it again on the same data.
2. Check: Application Home lists a card starting with "Open Harbour Lights (copy)".
3. Check: Application Home lists a card starting with "Open Harbour Lights" that is not the copy.

## Notes

- On the Mac, Ctrl+Enter uses Cmd instead.

## Platform notes

- The system Save and Open dialogs are the Windows file dialogs, filled through Windows UI Automation by typing the full path into File name. Mac: the NSSavePanel and NSOpenPanel; how XCUITest fills them is to decide in phase M-track.
