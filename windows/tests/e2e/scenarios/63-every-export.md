# Every export format

Spec: `tests/e2e/specs/63-every-export.test.mjs`

Tags: `@e2e-data-root: own`

Before: an empty data folder of its own. The app starts for the first time. Exports are written to an `exports` folder inside the data folder, through the core, the same way the Export dialogs call it (no system Save dialog).

Production exports (schedule, Day Out of Days, call sheet, sides, reports, shot list, storyboard sheet, moodboard) are in `61-schedule.md` and `62-call-sheets-exports.md`; packages in `70-packages.md`.

## The screenplay exports as PDF, Final Draft, Fountain, DOCX and plain text, and each text format imports back

1. Do the first launch: name 'E2E Tester', project title 'Out And Back'.
2. Import the Night Bus screenplay (see the README).
3. Through the core: export the current draft as PDF, Final Draft (.fdx), Fountain, DOCX and plain text, with the title page and scene numbers.
4. Check: each file exists and holds at least 100 bytes; the PDF starts with "%PDF-"; the Fountain file contains "You came back.".
5. Through the core: for each of the Final Draft, Fountain, DOCX and plain text files, preview the import and apply it as a new draft named 'Back from <format>'.
6. Check: each new draft has 3 scenes, one scene heading contains "BUS STOP", and one element reads "You came back.".

## The Story Board exports as PDF

1. Click Story. Click Add Act, type 'Act One' and press Enter.
2. Click Add, choose Scene Card, type 'Meera waits at the bus stop' and press Enter.
3. Through the core: export the Story Board as PDF.
4. Check: the file holds at least 500 bytes and starts with "%PDF-".

## The diagnostics bundle is written where the person chose, and holds no project text

1. Through the core: export the diagnostics bundle to `diagnostics.zip` in the exports folder.
2. Check: the file is a zip file (it starts with "PK").
3. Check: the bytes do not contain "Meera waits at the bus stop".

## Platform notes

- None beyond the shared steps.
