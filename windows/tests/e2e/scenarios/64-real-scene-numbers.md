# Real scene numbers, nearest hospital and Home counts

Spec: `tests/e2e/specs/64-real-scene-numbers.test.mjs`

Tags: `@e2e-data-root: own`

Before: an empty data folder of its own. The app starts for the first time.

"The Night Ward screenplay" is this Fountain text, pasted through Create, Import Screenplay (preview, import, Open Screenplay): title 'Ward Nine'; scene one `INT. HOSPITAL - NIGHT #120A#` with one action line and the cue KAVYA with 'Bed four is awake.'; scene two `EXT. CAR PARK - NIGHT #121#` with one action line and the cue RAVI with 'Any news?'. The scene numbers are written in the script, so the real numbers are 120A and 121, while the positions are 1 and 2.

"Production tab X" means: open Production if needed, then click X in the Production navigation. "The card drawer" is the drawer "Scene Card".

"Link scene 120A" means: in the card drawer choose the entry containing "Scene 120A" in the choice "Screenplay scene", then click Link, then check the drawer shows "Scene 120A".

## A Scene Card links to an imported scene, unlinks, links again and keeps the link

1. Do the first launch: type 'E2E Tester' as the name, click Continue, click Create Project, type 'Corridor Nine' as the title, click Create.
2. Import the Night Ward screenplay.
3. Click Story in the Workspaces navigation, then click the Board tab. Click Add Act, type 'Act One' and press Enter.
4. Click Add, choose Scene Card, type 'Kavya sees the bed is empty' and press Enter.
5. Double-click the card "Scene Card: Kavya sees the bed is empty".
6. Check: the drawer shows "Not linked to a screenplay scene".
7. Link scene 120A.
8. Click Unlink in the drawer.
9. Check: the drawer shows "Not linked to a screenplay scene".
10. Link scene 120A.
11. Check: the screen is quiet.
12. Click Close in the drawer. Open the project menu and choose Close project.
13. Quit the app and start it again on the same data. Click the project row "Open Corridor Nine".
14. Open the Beat Board and double-click the card again.
15. Check: the drawer shows "Scene 120A".
16. Click Close in the drawer.
17. Check: the screen is quiet.

## Breakdown and Schedule show Scene 120A, not its position

1. Click Breakdown in the Workspaces navigation, click Select Source, click the draft in the "Screenplay drafts" choice, then click Set as Production Source.
2. Check: the panel "Scene 120A breakdown" shows.
3. Check: the list "Scenes in the Production Source" shows "120A" and "121".
4. Click Add Element, choose "Location / Set", type 'Ward Nine Corridor' in the name field, check the dialog no longer shows "Looking for existing catalog items", click the dialog button starting with "Add to Scene", then check the screen shows "Added “Ward Nine Corridor”".
5. Check: the screen is quiet.
6. Production tab Schedule. Click Create Shooting Schedule, then click Create Schedule in the dialog.
7. Check: the workspace shows "2 unscheduled".
8. Click Create Shooting Day, set the date field to 14 June 2027, click Create Day in the dialog.
9. Click the button "Scene 120A actions" and choose "Shoot Day 1".
10. Check: the workspace shows "1 scene".
11. Check: the workspace shows "120A" and "121".
12. Check: no button named "Scene 1 actions" or "Scene 2 actions" exists.
13. Check: the screen is quiet.

## The call sheet shows Scene 120A and the nearest hospital typed on the location

1. Production tab Locations. Click the card "Ward Nine Corridor".
2. Select all text in the field "Nearest hospital", type 'Town Infirmary, Station Road, 14 minutes by car' and press Tab. Click Close in the drawer.
3. Check: the screen is quiet.
4. Click Call Sheets in the Workspaces navigation, click Create Call Sheet, then click Create Call Sheet in the dialog.
5. Check: the call sheet document shows "120A".
6. Check: the field "Ward Nine Corridor · Nearest hospital" in the safety block reads 'Town Infirmary, Station Road, 14 minutes by car'.
7. Check: the screen is quiet.
8. Select all text in that field and type 'Override: Lakeside Clinic, 6 minutes'. Move focus off the field.
9. Check: the saved call sheet holds the typed text as the hospital of the location.
10. Through the core: move the unscheduled scene onto Shoot Day 1.
11. Check: within 15 seconds the workspace shows "Needs refresh".
12. Click Review Update, check the dialog shows "Update call sheet: review changes" and lists scenes, then click Apply Update in the dialog.
13. Check: the workspace no longer shows "Needs refresh.".
14. Check: the call sheet document shows "121".
15. Check: the field "Ward Nine Corridor · Nearest hospital" still reads 'Override: Lakeside Clinic, 6 minutes'.
16. Check: the screen is quiet.

## Project Home counts the current draft's scenes and the shooting days without an off day

1. Production tab Schedule. Click Off Day, then click Add Off Day in the dialog.
2. Click Screenplay in the Workspaces navigation. Click the button whose name starts with "Draft: ", choose New Draft, type 'Second pass' as the name, click Create Draft, and check the screen shows "Created draft “Second pass”".
3. Click Home in the Workspaces navigation.
4. Check: within 10 seconds the "Production status" card shows 2 for "Screenplay drafts".
5. Check: the card shows 2 for "Screenplay scenes" (the current draft's scenes, not 4).
6. Check: the card shows 1 for "Shooting days" (the off day is not counted).
7. Check: the screen is quiet.

## Notes

- Not covered: an omitted scene on Home. A scene is omitted only when a numbered scene is deleted in a way no screen reaches in one step; the core tests cover it.
- Step 10 of the call sheet scenario moves the scene through the core, as `62-call-sheets-exports` does.

## Platform notes

- None beyond the shared steps.
