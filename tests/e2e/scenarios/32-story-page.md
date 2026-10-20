# Story page

Spec: `tests/e2e/specs/32-story-page.test.mjs`

Tags: `@e2e-data-root: own`

Before: an empty data folder of its own. The app starts for the first time.

"The page" means the writing box named "Story text" on the Story tab. "The story views" means the tabs of the Story workspace, read left to right.

## The Story tab is first, Board stays the default, and the page starts empty

1. Do the first launch: type 'E2E Tester' as the name, click Continue, click Create Project, type 'Night Pages' as the title, click Create.
2. Click Story in the Workspaces navigation.
3. Check: the screen shows "Your Beat Board is empty." (the Board tab is still the one that opens).
4. Check: the story views are, in order, "Story", "Board", "Outline", "Characters", "Timeline".
5. Click the Story tab.
6. Check: the screen shows "Write your story freely. This page is not linked to anything else."
7. Check: the page is empty and its placeholder reads "Start writing your story here."
8. Check: the screen shows none of "Parking Lot", "Build Screenplay", "Search / filter cards", "No scene numbers".
9. Check: the screen is quiet.

## Type paragraphs: the page saves them and counts the words

1. Click the page.
2. Type 'The last bus leaves at midnight.', press Enter twice, type 'Meera runs, one shoe in her hand, and the driver pretends not to see her.', press Enter twice, type 'By the time the doors close she has already forgiven him.'
3. Check: within 10 seconds the save state under the page reads "Saved".
4. Check: the page holds the three paragraphs exactly as typed.
5. Check: the screen shows the word count of the three paragraphs, for example "32 words".
6. Check: the screen is quiet.

## Switching tabs keeps the text, and typed text is stored when leaving

1. Click the page, press Ctrl+End, press Enter twice and type 'Last line, typed just before leaving.'
2. Click the Board tab straight away.
3. Check: the screen shows "Your Beat Board is empty."
4. Click the Story tab.
5. Check: within 10 seconds the page holds the three paragraphs and the last line, separated by blank lines.

## Relaunch: the text is still there

1. Check: the save state under the page reads "Saved".
2. Open the project menu and choose Close project.
3. Quit the app and start it again on the same data.
4. Click the project row "Open Night Pages". Check: no recovery question shows.
5. Click Story in the Workspaces navigation, then the Story tab.
6. Check: within 15 seconds the page holds the three paragraphs and the last line, exactly as before.
7. Check: the screen is quiet.

## Export: the dialog offers Plain text, PDF and Word, and the files hold the text

1. Click Export.
2. Check: the screen shows "Export Story" with the formats "Plain text (.txt)", "PDF" and "Word (.docx)".
3. Click Cancel in the dialog.
4. Through the core: export the Story page as plain text to a file in the data folder.
5. Check: the text file holds the page with Windows line endings (after an optional byte order mark) and nothing else.
6. Through the core: export the Story page as PDF to a file in the data folder.
7. Check: the file starts with "%PDF-".
8. Through the core: export the Story page as Word to a file in the data folder.
9. Check: the file is a zip package of more than 500 bytes.
10. Open the project menu and choose Close project.

## Notes

- On the Mac, Ctrl+End uses Cmd+Down.

## Platform notes

- System Save dialogs skipped: the exports ask the core with a path in the data folder, as in `63-every-export`.
