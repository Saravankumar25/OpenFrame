# Scene Hub, comments, drafts and compare

Spec: `tests/e2e/specs/44-scene-hub-comments-compare.test.mjs`

Tags: `@e2e-data-root: own`

Before: an empty data folder of its own. The app starts for the first time.

## The Scene Hub opens from a scene's context menu and lists what exists around the scene

1. Do the first launch: name 'E2E Tester', project title 'Second Look'.
2. Import the Night Bus screenplay (see the README).
3. Right-click the scene "BUS STOP" in the Scenes navigator and choose Scene Hub.
4. Check: a drawer opens with the text "What exists around this scene?" and its title names "BUS STOP".
5. Click Close in that drawer. Check: the drawer closes.

## A comment on a scene is counted on the Comments button and survives a restart

1. Right-click the scene "BUS STOP" and choose Add Comment.
2. Type 'Is the bus stop lit at night?' in the Comment field and click Comment.
3. Check: the page shows "Is the bus stop lit at night?".
4. Check: the Screenplay toolbar button reads "Comments (1)".
5. Open the project menu and choose Close project. Check: Application Home shows.
6. Quit the app and start it again on the same data. Click the project row "Open Second Look".
7. Click Screenplay. Check: within 15 seconds the toolbar button reads "Comments (1)".
8. Click "Comments (1)". Check: the page shows "Is the bus stop lit at night?".

## A second draft takes a change of its own, and Compare shows the two drafts side by side

1. Click the draft switcher (the button named "Draft: <name>. Switch draft") and choose New Draft.
2. Type 'Rewrite B' in the draft name field and click Create Draft.
3. Check: the screen shows "Created draft “Rewrite B”".
4. Click the last dialogue line, press End and type ' Every night.'
5. Check: a dialogue line reads "You came back. Every night.", and within 15 seconds the editor says "Saved".
6. Click Compare in the Screenplay toolbar.
7. Check: the side-by-side comparison shows, and it contains "Every night.".
8. At window sizes 1280 by 720, 1600 by 900 and 1920 by 1080, and again with the AI Assistant open, check: the bar with Back to script, the two draft pickers, the view switch and Previous and Next change sits above the Changes list and the Summary line; nothing in it overlaps them, each draft picker is no wider than 300 pixels, and the page does not scroll sideways.

## Notes

- Mac: a secondary click (Control-click) opens the scene's context menu.

## Platform notes

- None beyond the shared steps.
