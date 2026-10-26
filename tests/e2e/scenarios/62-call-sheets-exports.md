# Call sheets, budget, templates, samples and exports

Spec: `tests/e2e/specs/62-call-sheets-exports.test.mjs`

Tags: `@e2e-data-root: own`

Before: an empty data folder of its own, and an empty temporary folder for exports. The app starts for the first time.

Many steps here ask the core directly to set up a plan or write an export with a path, the same way the screens do. They are marked "Through the core". "Production tab X" means: open Production if needed, then click X in the Production navigation. "The call sheet" is the panel named "Call sheet document".

"Check the PDF F" means: check the file F exists, starts with "%PDF-", and contains the brand line "Created using OpenFrame". "Check the PDF F has no brand line" means the same, except that the file must not contain the brand line.

## Open a sample script that ships with the app

1. Do the first launch: type 'E2E Tester' as the name, click Continue, click Create Project, type 'Lost Property Unit' as the title, click Create.
2. Click Create, then choose Import Screenplay.
3. Click Example Short in the "Sample script" choice.
4. Click Preview in the dialog.
5. Check: the screen shows "Import Screenplay · Preview".
6. Check: the dialog shows "Scenes" with the figure 10.
7. Click Import in the dialog. Check: the screen shows "Import complete".
8. Click Open Screenplay in the dialog.
9. Check: the Screenplay workspace opens.
10. Check: the screen is quiet.

## Plan two shooting days from the Production Source

1. Through the core: set the first draft of the first screenplay as the Production Source.
2. Through the core: create a shooting schedule with days on 14 June 2027 (Day 1) and 15 June 2027.
3. Through the core: move the first two unscheduled scenes into Day 1.
4. Through the core: for those two scenes, suggest breakdown elements and accept them all.
5. Through the core: check the Day Out of Days has at least one cast row.
6. Through the core: add a 45 minute meal break to Day 1 after the first scene, and set its crew call to 07:00, shooting start to 08:00 and target wrap to 19:00.
7. Production tab Schedule.
8. Check: the workspace shows "Shoot Day 1".
9. Check: the screen is quiet.

## Create a call sheet: six default sections, Needs input, Add Section

1. Click Call Sheets in the Workspaces navigation.
2. Click Create Call Sheet, then click Create Call Sheet in the dialog.
3. Check: the call sheet shows.
4. Check: the call sheet has the sections "DAY AND TIMES", "PRODUCTION", "LOCATIONS", "SCENE SCHEDULE", "TALENT / CAST" and "PRACTICAL / SAFETY INFORMATION".
5. Check: the call sheet has no "WEATHER" section yet.
6. Check: at least one empty value in the call sheet shows "Needs input".
7. Check: the Crew call field reads "07:00" (it comes from the day).
8. Click Add Section, then choose Weather.
9. Check: the call sheet shows "WEATHER".
10. Select all text in the Forecast field and type 'Light rain after 18:00'. Move focus off the field.
11. Through the core: check the project lists one call sheet.
12. Check: the screen is quiet.

## A schedule change marks the sheet Needs refresh, and the preview lists what changed

1. Through the core: move the next unscheduled scene into Day 1.
2. Check: the workspace shows "Needs refresh".
3. Click Review Update.
4. Check: the screen shows "Update call sheet: review changes", and the dialog mentions a scene.
5. Click Apply Update in the dialog.
6. Check: the workspace no longer shows "Needs refresh."
7. Check: the Forecast field still reads "Light rain after 18:00".
8. Check: the screen is quiet.

## Finalize freezes a snapshot; a new revision follows the schedule

1. Click Finalize…, then click Finalize in the dialog.
2. Check: the workspace shows "Finalized " followed by a date.
3. Check: the Crew call field is disabled.
4. Through the core: export the call sheet as PDF. Check the PDF `call-sheet.pdf`.
5. Through the core: change Day 1's crew call to 07:30.
6. Check: the workspace shows "Start New Revision".
7. Check: the Crew call field still reads "07:00" (the final sheet did not change).
8. Click Start New Revision, then click Create New Revision in the dialog.
9. Check: the workspace shows "v2".
10. Check: the Crew call field reads "07:30".
11. Check: the screen is quiet.

## The budget keeps Planned and Actual with the difference

1. Production tab Budget.
2. Click Start budget, then click Add Line.
3. Type 'Lead actor' in the description field, '50000' in the planned amount field and '55000' in the actual amount field.
4. Click Add Line in the dialog.
5. Check: the workspace shows "over plan".
6. Move focus off any text field and press Ctrl+Z.
7. Check: the workspace shows "No lines yet".
8. Check: the screen is quiet.

## A call sheet from the Indie Call Sheet template is a copy of its own

1. Through the core: list the built-in templates. Check: they include "Feature Film Screenplay", "Short Film Screenplay", "Episodic / Series", "Simple Beat Board", "Indie Call Sheet", "Shot List", "Moodboard" and "Lightweight Budget".
2. Through the core: apply the Indie Call Sheet template to the shooting day that has no call sheet and is not an off day.
3. Through the core: check the new call sheet has a Weather section and a Crew section.
4. Check: the screen is quiet.

## A report and the visual planning exports carry the brand line, sides do not

1. Through the core: export the sides for Day 1 with a cover. Check the PDF `sides.pdf` has no brand line (sides are pages of the script).
2. Through the core: export the scene report. Check the PDF `scene-report.pdf`.
3. Through the core: export the shooting schedule. Check the PDF `schedule.pdf`.
4. Through the core: export the Day Out of Days. Check the PDF `dood.pdf`.
5. Through the core: export the breakdown report. Check the PDF `breakdown.pdf`.
6. Through the core: apply the Shot List template to the first scene, then export the shot list. Check the PDF `shots.pdf`.
7. Through the core: create a moodboard named "Look", add the note "Wet asphalt at night", and export it. Check the PDF `moodboard.pdf`.
8. Through the core: create a storyboard named "Opening" for the first scene, add a placeholder panel "Rain on the platform roof", and export the storyboard sheet. Check the PDF `storyboard.pdf`.
9. Through the core: export the first draft of the screenplay as PDF to `script.pdf`.
10. Check: `script.pdf` does not contain "Created using OpenFrame" (screenplay files stay clean).
11. Check: the screen is quiet.

## Notes and tasks for the shoot

1. Through the core: create the note "Rain cover" with the text "Keep the tarps on the truck."
2. Through the core: create the task "Confirm the railway permit".
3. Open the project menu and choose Notes & Tasks.
4. Check: the workspace shows "Rain cover" and "Confirm the railway permit".
5. Check: the screen is quiet.

## Quit, relaunch and find everything again

1. Open the project menu and choose Close project.
2. Quit the app and start it again on the same data.
3. Click the project row "Open Lost Property Unit". Check: no recovery question shows.
4. Through the core: check the call sheets include one at revision 2 and one with status Final.
5. Click Call Sheets in the Workspaces navigation.
6. Check: the workspace shows "Shoot Day 1".
7. Check: the screen is quiet.

## Notes

- On the Mac, Ctrl+Z uses Cmd instead.
- The planning, template and export steps go through the core with a file path, so the system Save dialog is skipped. A Mac run may drive the same screens by hand where XCUITest can.
