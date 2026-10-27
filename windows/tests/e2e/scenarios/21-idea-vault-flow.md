# Idea Vault full flow

Spec: `tests/e2e/specs/21-idea-vault-flow.test.mjs`

Tags: `@e2e-data-root: own`, `@e2e-fake-media`

Before: an empty data folder of its own. The app starts for the first time, with a fake microphone and camera that need no device and ask no permission.

"Quick capture X" means: click Quick Capture, type X in the Idea field, click Save in the dialog, then check a Vault card starting with the first words of X shows.

"Card menu X, then Y" means: right-click the Vault card whose name starts with X, then choose the menu item Y.

"Choose the Vault X" means: click X in the "Which Idea Vault" choice.

## Record a voice note with the microphone

1. Do the first launch: type 'E2E Tester' as the name, click Continue, click Create Project, type 'Letter Town' as the title, click Create.
2. Click Idea Vault in the Workspaces navigation.
3. Click Record Note.
4. Check: within 15 seconds the screen shows "Recording…"
5. Type 'Night bus idea' in the recording name field.
6. Wait about 1.5 seconds.
7. Click Stop & Save in the dialog.
8. Check: a Vault card starting with "Night bus idea" shows.
9. Check: the screen is quiet.

## Research flags: set one, see it as words, undo, change it

1. Quick capture 'Police procedure: order of evidence handling at a small station.'
2. Card menu "Police procedure", then Needs Expert Check.
3. Check: the screen shows "as Needs Expert Check".
4. Check: the card shows "Needs Expert Check".
5. Card menu "Police procedure", then Checked.
6. Check: the card shows "Checked" and no longer shows "Needs Expert".
7. Move focus off any text field and press Ctrl+Z.
8. Check: the card shows "Needs Expert Check" again.
9. Press Ctrl+Y.
10. Check: the card no longer shows "Needs Expert".
11. Double-click the card "Police procedure".
12. Check: the research flag field in the drawer is set to Checked.
13. Check: the screen shows "OpenFrame never marks research as true just because you imported it."
14. Click Close in the drawer. Check: the drawer closes.
15. Check: the screen is quiet.

## Global Idea Vault: capture there, copy into the project

1. Choose the Vault Global.
2. Quick capture 'A city where every letter is delivered one year late.'
3. Card menu "A city where every letter", then Copy to Project.
4. Click Copy to Project in the dialog.
5. Check: the screen shows "to the Project Idea Vault".
6. Choose the Vault This Project.
7. Check: a card starting with "A city where every letter" shows.
8. Check: the screen is quiet.

## Send to the Beat Board as a scene card; the Vault original stays

1. Quick capture 'Meera finds her own handwriting on an old envelope.'
2. Card menu "Meera finds her own", then Send to Story.
3. Check: the screen shows "Send to Beat Board".
4. Click Create Copy in Beat Board in the dialog.
5. Check: the Vault still has a card starting with "Meera finds her own".
6. Click Story in the Workspaces navigation, then click the Board tab.
7. If the Parking Lot button is not pressed, click it.
8. Check: a scene card named "Scene Card: Meera finds her own handwriting on an old envelope." shows.
9. Check: the screen is quiet.
10. Click Idea Vault in the Workspaces navigation.

## After a relaunch, the voice note plays, and the flags and copies are kept

1. Open the project menu and choose Close project.
2. Quit the app and start it again on the same data.
3. Click the project row "Open Letter Town". Check: no recovery question shows.
4. Click Idea Vault in the Workspaces navigation.
5. Check: a card starting with "Night bus idea" shows.
6. Check: the card "Police procedure" shows "Checked".
7. Check: a card starting with "A city where every letter" shows.
8. Double-click the card "Night bus idea".
9. Check: the drawer holds an audio player, and the recording loads with a length greater than zero.
10. Click Close in the drawer.
11. Choose the Vault Global.
12. Check: a card starting with "A city where every letter" shows.
13. Check: the screen is quiet.
14. Open the project menu and choose Close project.

## Notes

- On the Mac, Ctrl+Z and Ctrl+Y use Cmd instead.
- Dropping files into the Vault from the file manager is an operating system drag that the test driver cannot do. Component and Rust tests cover it, so it is not a scenario here.

## Platform notes

- The fake microphone is a WebView2 start argument (`--use-fake-device-for-media-stream`, `--use-fake-ui-for-media-stream`). Mac: to decide in phase M-track.
- The drag-in note names Windows Explorer. Mac: Finder.
