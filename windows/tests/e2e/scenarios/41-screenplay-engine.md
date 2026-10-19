# Screenplay engine

Spec: `tests/e2e/specs/41-screenplay-engine.test.mjs`

Tags: `@e2e-data-root: own`

Before: an empty data folder of its own, with an empty `exports` folder inside it. The app starts for the first time.

"The elements" means the screenplay editor elements, read as "type: text". "Check: Saved" means the editor toolbar status reads "Saved". "View menu X" means: click View and more, then choose X.

Some steps ask the core directly, the same way the screen does. They stand in for the system Save and Open dialogs and for two settings, as the Notes explain.

## Write with the element flow and insert a page break

1. Do the first launch: type 'E2E Tester' as the name, click Continue, click Create Project, type 'Signal Box' as the title, click Create.
2. Click Screenplay in the Workspaces navigation, then click New Screenplay.
3. Check: the editor opens.
4. Click the first element.
5. Type 'INT. SIGNAL BOX - NIGHT' and press Enter.
6. Type 'The telephone rings twice.' and press Enter.
7. Type 'ASHA'. Check: the suggestion list offers "Create New: ASHA". Press Esc. Check: the list closes.
8. Press Enter. Type 'Who is there?'
9. Press Ctrl+Enter.
10. Type 'Nobody answers.'
11. Check: the elements include a page break, directly after "dialogue: Who is there?" and directly before "dialogue: Nobody answers."
12. Check: Saved.
13. Check: the screen shows "2 pages".
14. Check: the screen is quiet.

## Title page fields with a live preview

1. View menu Title page.
2. Select all text in the date field and type '4 October 2026'.
3. Select all text in the phone field and type '+91 98765 43210'.
4. Check: the title page preview shows "4 October 2026".
5. Click Save in the dialog.
6. Check: the screen shows "Saved the title page".

## Page setup: A4 with a header

1. View menu Page setup.
2. Choose A4 in the paper list.
3. Select all text in the header field and type '{title} {draft}'.
4. Click Save in the dialog.
5. Check: the screen shows "Changed the page setup".
6. Ask the core for the pagination of the current draft. Check: the paper is A4 with 59 lines per page.

## A custom element under Advanced, used in the script

1. View menu Custom elements.
2. Click Add custom element.
3. Select all text in the name field and type 'Telephone'.
4. Choose dialogue in the look list.
5. Choose Ctrl+Alt+1 in the shortcut list.
6. Click Add in the dialog.
7. Check: the screen shows "Added the custom element “Telephone”".
8. Click Done in the dialog.
9. Click the action element "The telephone rings twice."
10. Click the Element type button and choose Telephone.
11. Check: the elements include "custom: The telephone rings twice."
12. Press Tab.
13. Check: the element is still "custom: The telephone rings twice." (Tab leaves a custom element as it is).
14. Check: Saved.
15. View menu Page view.
16. Check: the Page view panel opens and shows "The telephone rings twice." and "Page 2".
17. Click Close in the Page view panel.
18. Check: the screen is quiet.

## Lock, start a dated revision, edit it and see the changed pages

1. Click Lock in the Screenplay toolbar.
2. Click Lock as Shooting Draft in the dialog.
3. Check: the screen shows "as the Shooting Draft".
4. Click Revisions in the Screenplay toolbar.
5. Click Start Revision in the Revisions panel.
6. Set the revision date field to 4 October 2026.
7. Click Start Revision in the dialog.
8. Check: the screen shows "stays locked".
9. Click the dialogue element "Nobody answers.", press End and type ' Again.'
10. Check: Saved.
11. Ask the core for the revision pages of the current draft. Check: only page 2 changed, and the revision date is 2026-10-04.
12. Click Revisions in the Screenplay toolbar.
13. Check: the screen shows "Changed pages:".
14. Click Close in the Revisions panel.
15. Check: the screen is quiet.

## Export PDF and FDX, then import the FDX back as a draft

1. Ask the core to export the current draft as PDF, with a title page and scene numbers, to `signal-box.pdf` in the exports folder.
2. Check: the export reports 2 pages.
3. Check: the PDF file exists and is larger than 500 bytes.
4. Check: the PDF does not contain the word "OpenFrame" (no branding on a screenplay).
5. Ask the core to export the current draft as FDX, with a title page and scene numbers, to `signal-box.fdx` in the exports folder.
6. Check: the FDX file holds a paragraph of type "TELEPHONE", a paragraph that starts a new page, and scene number 1.
7. Ask the core to preview an import of that FDX file, then apply it as a new draft named "From Final Draft" in the same screenplay.
8. Check: the new draft holds the custom element "The telephone rings twice.", a page break, and its first scene is numbered "1".

## Telugu and Hindi print in the PDF; the page break follows a remapped shortcut

1. Ask the core to add an action line to the first scene of the current draft with this text: 'మీరా రైలు స్టేషన్‌లో వేచి ఉంది. मीरा ने कहा, “क्षमा करें।”'
2. Check: the editor shows the line containing "మీరా రైలు".
3. Ask the core to export the current draft as PDF, without a title page, to `signal-box-te.pdf` in the exports folder.
4. Check: the PDF embeds the fonts Noto Sans Telugu and Noto Sans Devanagari.
5. Change the shortcut for Insert page break to Ctrl+Shift+Enter (the spec sets it in the shortcut store that Settings uses).
6. Count the page breaks in the elements.
7. Click the Telugu action line and press End.
8. Press Ctrl+Enter.
9. Check: the number of page breaks did not change (the old key no longer inserts one).
10. Press Ctrl+Shift+Enter.
11. Check: there is one more page break.
12. Put the Insert page break shortcut back to its default.
13. Check: Saved.
14. Check: the screen is quiet.

## After a relaunch, page setup, the custom element and the revision text are kept; keep writing

1. Open the project menu and choose Close project.
2. Quit the app and start it again on the same data.
3. Click the project row "Open Signal Box". Check: no recovery question shows.
4. Click Screenplay in the Workspaces navigation.
5. Check: the elements include "dialogue: Nobody answers. Again."
6. Ask the core for the page setup of the screenplay. Check: the paper is A4 and the header is "{title} {draft}".
7. Click the dialogue element containing "Again", press End, then press Enter.
8. Type 'Static on the line.'
9. Press Ctrl+Alt+1.
10. Check: the elements include "custom: Static on the line."
11. Check: Saved.
12. Check: the screen is quiet.
13. Open the project menu and choose Close project.

## Notes

- On the Mac, Ctrl+Enter, Ctrl+Shift+Enter and Ctrl+Alt+1 use Cmd in place of Ctrl, and Alt is Option. The custom element shortcut stored as Ctrl+Alt+1 on Windows is to decide in phase M-track.
- The exports and the re-import call the same operations the Export and Import dialogs call, with a path inside the data folder. The system Save and Open dialogs are the only steps skipped.
- The Indian-script line is added through the core, not typed, and the shortcut is changed through the shortcut store, not through the Settings screen. A Mac run may type the line and use Settings instead.
- The revision date is set without the system date picker.

## Platform notes

- The system Save and Open dialogs are skipped on Windows. Mac: to decide in phase M-track.
