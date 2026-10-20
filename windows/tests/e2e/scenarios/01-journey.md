# Desktop journey

Spec: `tests/e2e/specs/01-journey.test.mjs`

Tags: none (shared journey data). This scenario seeds the demo project "Night Bus" that `02-restart`, `90-visual-baseline` and `95-reachability` open later, in that order.

Before: the shared journey data folder is empty. The app starts for the first time. All scenarios below run in order in one running app.

## First launch, name, create a project, land on Project Home

1. Start the app.
2. Check: the screen shows "Welcome to OpenFrame Studio".
3. Type 'E2E Tester' in the name field of the welcome dialog.
4. Click Continue in the dialog.
5. Click Create Project.
6. Type 'Night Bus' in the project title field.
7. Click Create in the dialog.
8. Check: the screen shows "Start with your ideas, build your story, or open a screenplay."
9. Check: the window title is "Night Bus - OpenFrame Studio".
10. Check: the screen is quiet.

## Idea Vault: add a note with Quick Capture

1. Click Idea Vault in the Workspaces navigation.
2. Click Quick Capture.
3. Type 'Arjun meets Meera again at a rainy bus stop.' in the Idea field.
4. Click Save in the dialog.
5. Check: the screen shows "Arjun meets Meera again at a rainy bus stop."
6. Check: the screen is quiet.

## Story: an act, a sequence, two cards, keyboard reorder, undo and redo

1. Click Story in the Workspaces navigation.
2. Click Add Act.
3. Type 'Act 1' in the Act title field and press Enter.
4. Check: the screen shows "ACT 1".
5. Click Add, then choose Sequence from the menu.
6. Type 'The return' in the Sequence name field and press Enter.
7. Check: the screen shows "The return".
8. Click Add, then choose Scene Card from the menu.
9. Type 'Bus stop reunion in the rain' in the Short description field and press Enter.
10. Click the "Beat Board" heading so no text field has focus.
11. Press Ctrl+N.
12. Check: a new Short description field shows.
13. Type 'Arjun confesses at the police station' and press Enter.
14. Check: the scene cards read, in order, "Bus stop reunion in the rain", then "Arjun confesses at the police station", each with its text shown.
15. Click the card "Arjun confesses at the police station".
16. Press Alt+Up arrow.
17. Check: "Arjun confesses at the police station" is now the first card.
18. Press Ctrl+Z.
19. Check: "Bus stop reunion in the rain" is the first card again.
20. Press Ctrl+Y.
21. Check: "Arjun confesses at the police station" is the first card again.
22. Check: the screen is quiet.

## Build Screenplay opens the new screenplay at the first scene

1. Click Build Screenplay.
2. Type 'INT. POLICE STATION [em dash] NIGHT' in Heading for scene 1.
3. Type 'EXT. BUS STOP [em dash] NIGHT' in Heading for scene 2.
4. Click Build Screenplay in the dialog.
5. Check: the Screenplay workspace opens with the editor filled.
6. Check: the first three elements are a scene heading "INT. POLICE STATION [em dash] NIGHT", a note "Arjun confesses at the police station", and an empty action.
7. Check: the draft has exactly two scene headings.
8. Check: the screen is quiet.

## Type in the screenplay editor, and it saves itself

1. Click the first action element in the editor.
2. Type 'Arjun enters carrying a pistol.' and press Enter.
3. Type 'ARJUN'.
4. Check: the suggestion list offers "Create New: ARJUN".
5. Press Esc. Check: the suggestion list closes and the name stays as typed.
6. Press Enter.
7. Type 'I was there that night.'
8. Check: elements three to five are action "Arjun enters carrying a pistol.", character "ARJUN", dialogue "I was there that night."
9. Check: the editor toolbar shows "Saved".
10. Wait about 3 seconds with the cursor still, so the app remembers the place in the screenplay.
11. Check: the screen is quiet.

## Import pasted Fountain from the Create menu and open the new draft

1. Click Create, then choose Import Screenplay.
2. Paste this text into the paste box:

   ```
   Title: Black Rain

   INT. RAILWAY PLATFORM - DAWN

   Meera waits with a red umbrella.

   MEERA
   The last train is gone.

   EXT. LEVEL CROSSING - DAY

   Arjun runs across the tracks.
   ```

3. Click Preview in the dialog.
4. Check: the dialog shows "Import Screenplay · Preview", "INT. RAILWAY PLATFORM - DAWN" and "EXT. LEVEL CROSSING - DAY".
5. Click Import in the dialog.
6. Check: the screen shows "Import complete".
7. Click Open Screenplay in the dialog.
8. Check: the first element of the open draft is the scene heading "INT. RAILWAY PLATFORM - DAWN".
9. Check: no element of the draft contains "Title:" (the title page did not become script text).
10. Wait about 3 seconds with the cursor still.
11. Check: the screen is quiet.

## Breakdown: choose the Production Source and accept suggestions

1. Click Breakdown in the Workspaces navigation.
2. Click Select Source.
3. In the dialog, click the draft whose label starts with "Draft 1 from Beat Board".
4. Click Set as Production Source.
5. Check: the screen shows "Production Source: Draft 1 from Beat Board".
6. Click Suggest Elements.
7. Click Accept Pistol.
8. Click Accept Arjun.
9. Check: the workspace lists "Props", "1", "Pistol".
10. Check: the screen is quiet.

## Schedule: all scenes start unscheduled, then one moves into a shooting day

1. Click Production in the Workspaces navigation, then the Schedule tab.
2. Click Create Shooting Schedule.
3. Click Create Schedule in the dialog.
4. Check: the screen shows "2 scenes (0 scheduled, 2 unscheduled)".
5. Click Create Shooting Day.
6. Set the date field in the dialog to 14 June 2027.
7. Click Create Day in the dialog.
8. Check: the screen shows "Shoot Day 1".
9. Click Scene 1 actions, then choose Shoot Day 1 from the menu.
10. Check: the screen shows "2 scenes (1 scheduled, 1 unscheduled)".
11. Check: the scene details drawer did not open (no "SCHEDULED SCENE" text).
12. Check: the unscheduled scene pool does not scroll sideways.
13. Check: the screen is quiet.

## Call sheet from the shooting day

1. Click Shoot Day 1 actions, then choose Create Call Sheet.
2. Check: the screen shows "Call Sheet: Day 1".
3. Check: the workspace shows "INT. POLICE STATION [em dash] NIGHT" and "ARJUN".
4. Check: the screen is quiet.

## A schedule strip opens the production draft at its scene

1. Click Production in the Workspaces navigation, then the Schedule tab.
2. Click the strip for scene 1 in the shooting day column.
3. Click Open in Screenplay.
4. Check: the Screenplay workspace shows "Draft 1 from Beat Board" (not the imported draft that was open last).
5. Check: the cursor sits on the scene heading "INT. POLICE STATION [em dash] NIGHT".
6. Wait about 3 seconds with the cursor still.
7. Check: the screen is quiet.

## Search with Ctrl+K finds the scene and goes to it

1. Press Ctrl+K.
2. Type 'pistol'.
3. Check: a result containing "INT. POLICE STATION" shows.
4. Click that result.
5. Check: the Screenplay workspace opens with the cursor on "INT. POLICE STATION [em dash] NIGHT".
6. Wait about 3 seconds with the cursor still.
7. Check: the screen is quiet.

## Recently Deleted: delete a scene card and restore it

1. Click Story in the Workspaces navigation.
2. Click the card "Bus stop reunion in the rain".
3. Press Delete.
4. If a dialog asks to confirm (the card already built a screenplay scene), click its Delete or Move button.
5. Check: the card "Bus stop reunion in the rain" is gone.
6. Click the project button "Project: Night Bus. Switch or manage projects", then choose Recently Deleted.
7. Click Restore Bus stop reunion in the rain.
8. Click Story in the Workspaces navigation.
9. Check: the card "Bus stop reunion in the rain" is back.
10. Check: the screen is quiet.

## The AI panel is ready with the bundled Offline AI, with no download and no setup

1. Click Assistant.
2. Check: the AI Assistant panel shows its Ask or command box.
3. Check: the screen shows "AI Ready".
4. Check: no visible text names the model, format or runtime ("gemma", "gguf", "q4_k", "llama", "embedding") and nothing offers "download offline AI".
5. Check: the screen is quiet.

## Every workspace and production tab works beside the docked AI panel

1. Keep the AI Assistant open. Make the window 1200 by 720 pixels (the smallest supported size).
2. Check: the page is at most 1200 pixels wide.
3. Visit each workspace in turn: Home, Idea Vault, Story, Screenplay, Breakdown, Production, Call Sheets, Files. In each one:
   1. Check: the screen is quiet.
   2. Check: the AI Assistant docks cleanly.
4. Click Production. Visit every tab in the Production navigation in turn. In each tab, do the same two checks.
5. Click Story, then the Outline tab. Do the same two checks. Repeat for the Characters tab and the Timeline tab.
6. Open the project menu and choose Notes & Tasks. Do the same two checks. Repeat for Activity, Recently Deleted and Project settings.
7. Note the width of the page and of the panel.
8. Press Ctrl+J.
9. Check: the AI Assistant closes.
10. Check: the page is now as wide as the page and the panel were together.
11. Check: the screen is quiet.

## Close the project cleanly, and Application Home lists it

1. Click the project button "Project: Night Bus. Switch or manage projects", then choose Close project.
2. Check: the screen shows "Your projects".
3. Check: the screen shows "Night Bus".
4. Check: the screen is quiet.

## Notes

- On the Mac, every Ctrl shortcut here (Ctrl+N, Ctrl+Z, Ctrl+Y, Ctrl+K, Ctrl+J) uses Cmd instead, and Alt is Option.
- The date is set without the system date picker, because the test driver cannot drive it.

## Platform notes

- The window size check uses the Windows minimum window size from the Tauri configuration. Mac: to decide in phase M-track.
