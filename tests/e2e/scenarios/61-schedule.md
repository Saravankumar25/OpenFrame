# Shooting schedule and Day Out of Days

Spec: `tests/e2e/specs/61-schedule.test.mjs`

Tags: `@e2e-data-root: own`

Before: an empty data folder of its own, and an empty temporary folder for exports. The app starts for the first time.

"Import the Night Shift screenplay" is the shared step in the README of this folder. "Production tab X" means: open Production if needed, then click X in the Production navigation. "The Day 1 column" is the stripboard column whose day name starts with "Shoot Day 1". "The pool" is the Unscheduled area of the board. "The Day Out of Days table" is the table named "Day Out of Days".

"Create a shooting day on D" means: click Create Shooting Day, set the date field to D, click Create Day in the dialog.

## Create the schedule from the Production Source

1. Do the first launch: type 'E2E Tester' as the name, click Continue, click Create Project, type 'Signal Yard' as the title, click Create.
2. Import the Night Shift screenplay.
3. Click Breakdown in the Workspaces navigation.
4. Click Select Source, click the draft in the "Screenplay drafts" choice, then click Set as Production Source.
5. Check: the screen shows "Production Source:".
6. For scene 1 and then scene 2: click the scene in the list "Scenes in the Production Source", check its breakdown panel shows, click Suggest Elements, then click every button in the panel whose name starts with "Accept " until none is left.
7. Production tab Schedule.
8. Click Create Shooting Schedule, then click Create Schedule in the dialog.
9. Check: the workspace shows "2 unscheduled".
10. Check: the screen is quiet.

## Days and an off day; drag a scene into a day, undo and redo; move a scene with the menu

1. Create a shooting day on 14 June 2027.
2. Create a shooting day on 15 June 2027.
3. Click Off Day, then click Add Off Day in the dialog.
4. Check: the workspace shows "Shoot Day 2 · Tue 15 Jun 2027".
5. Drag the strip "Scene 1" from the pool into the Day 1 column with the mouse.
6. Check: the Day 1 column shows "1 scene".
7. Move focus off any text field and press Ctrl+Z.
8. Check: the pool shows "unscheduled" and the Day 1 column shows "No scenes scheduled".
9. Press Ctrl+Y.
10. Check: the Day 1 column shows "1 scene".
11. Click Scene 2 actions, then choose Shoot Day 1.
12. Check: the Day 1 column shows "2 scenes".
13. Check: the Off Day column shows "Off day".
14. Check: the screen is quiet.

## A meal break, day times and the computed timeline

1. Click Shoot Day 1 actions, then choose Meal Break.
2. Type '45m' in the duration field. Click Add in the dialog.
3. Check: the Day 1 column shows "Meal Break".
4. Click "Shoot Day 1 · Mon 14 Jun 2027".
5. Check: the workspace shows "What exactly are we doing on this day?"
6. Select all text in Crew call for Shoot Day 1 and type '07:00'.
7. Select all text in Shooting start for Shoot Day 1 and type '08:00'. Move focus off the field.
8. Check: the "Timeline of Shoot Day 1" list shows "07:00", "Crew call", "08:00", "Shooting starts", "Meal Break" and "Estimated wrap".
9. Select all text in Day target length and type '30'. Move focus off the field.
10. Check: the workspace shows "Over target".
11. Check: the screen is quiet.
12. Click Back to schedule.

## Conflicts are advice: Keep anyway; grouping suggestions move nothing

1. Check: the Day 1 column shows "2 scenes".
2. Check: the warnings button does not read "0 warnings".
3. Click the warnings button.
4. Check: the "Schedule assistance" panel opens.
5. Click Keep anyway in the panel.
6. Check: the panel shows "Kept anyway".
7. Click Close in the panel.
8. Check: the Day 1 column still shows "2 scenes". Note its full text.
9. Click Suggest Grouping.
10. Check: the "Grouping suggestions" panel opens.
11. Click Close in the panel.
12. Check: the Day 1 column text is unchanged.
13. Check: the screen is quiet.

## Filters only change what is shown

1. Click List.
2. Choose Unscheduled in Filter by scheduled state.
3. Check: the workspace shows "No matching scenes." and "Showing 0 of 2 scenes".
4. Click Clear filters.
5. Check: the workspace no longer shows "No matching scenes."
6. Click Board.
7. Check: the Day 1 column shows "2 scenes".
8. Check: the screen is quiet.

## The call sheet stale review follows the day times

1. Click Shoot Day 1 actions, then choose Create Call Sheet.
2. Production tab Schedule.
3. Check: the Day 1 column shows "Call sheet: Draft".
4. Click "Shoot Day 1 · Mon 14 Jun 2027".
5. Select all text in Target wrap for Shoot Day 1 and type '18:00'. Move focus off the field.
6. Click Back to schedule.
7. Check: the Day 1 column shows "Call sheet: Needs refresh".
8. Press Ctrl+Z.
9. Check: the Day 1 column shows "Call sheet: Draft".
10. Press Ctrl+Y.
11. Check: the screen is quiet.

## Day Out of Days: derived, corrected by hand, undone

1. Production tab Day Out of Days.
2. Check: the Day Out of Days table shows.
3. Check: the first cell for Shoot Day 1 shows a working code (SWF, SW or W).
4. Click a cell for Shoot Day 2, then choose T.
5. Check: the workspace shows "1 corrected by hand".
6. Press Ctrl+Z.
7. Check: the table no longer shows "T*".
8. Press Ctrl+Y.
9. Check: the workspace shows "1 corrected by hand".
10. Click the cell for Shoot Day 2 again, then choose Add a note.
11. Select all text in the note field and type 'Flies in the night before'. Click Save in the dialog.
12. Check: a cell is named with "note: Flies in the night before".
13. Check: the screen is quiet.

## Exports: the dialogs open, and the files are the same byte for byte

1. Click Export.
2. Check: the screen shows "Export Day Out of Days". Press Esc.
3. Production tab Schedule.
4. Click Export.
5. Check: the screen shows "Export Shooting Schedule". Press Esc.
6. For the shooting schedule and for the Day Out of Days, in PDF and in XLSX:
   1. Ask the core to export it to a file A in the export folder.
   2. Wait about 1 second.
   3. Ask the core to export it again to a file B.
   4. Check: both files exist, and A and B are the same byte for byte.
7. Check: the screen is quiet.

## After a relaunch, days, scenes, breaks, times and corrections are kept

1. Open the project menu and choose Close project.
2. Quit the app and start it again on the same data.
3. Click the project row "Open Signal Yard". Check: no recovery question shows.
4. Production tab Schedule.
5. Check: the Day 1 column shows "2 scenes", "Meal Break" and "wrap".
6. Production tab Day Out of Days.
7. Check: the workspace shows "1 corrected by hand".
8. Check: a cell is named with "note: Flies in the night before".
9. Check: the screen is quiet.
10. Open the project menu and choose Close project.

## Notes

- On the Mac, Ctrl+Z and Ctrl+Y use Cmd instead.
- The drag is a real mouse drag that moves past 6 pixels before it counts, in small steps, with a short pause before the release.
- The dates are set without the system date picker.
- The exports are asked of the core with a path, so the system Save dialog is skipped. The Export dialogs are only opened and closed.
