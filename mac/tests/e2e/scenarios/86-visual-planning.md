# Visual planning: canvas, sketch dialog and drag reorder

Spec: `tests/e2e/specs/86-visual-planning.test.mjs`

Tags: `@e2e-data-root: own`

Before: an empty data folder of its own. The app starts for the first time. Every drag here is a real mouse drag: press, move a few pixels, move to the target, release.

## A note on the moodboard canvas is dragged to a new place

1. Do the first launch: name 'E2E Tester', project title 'Pictures First'.
2. Import the Night Bus screenplay (see the README).
3. Click Production, then the Moodboards tab. Click New Moodboard, type 'Overall Look' in the board name field, click Create Moodboard.
4. Click Add note, type 'Cold blue light, warm windows' in the Note field, click Add Note.
5. Check: a tile named "Note: Cold blue light, warm windows" shows on the canvas. Note its position.
6. Drag the tile 160 pixels right and 90 pixels down.
7. Check: the tile moved more than 100 pixels right and more than 50 pixels down.

## The sketch dialog draws, takes a stroke back, and saves a storyboard panel

1. Click the Storyboards tab. Click Draw / Sketch.
2. Check: the sketch area shows.
3. Draw two crossing strokes in the sketch area.
4. Click Undo in the dialog (one stroke is taken back).
5. Click Save Panel.
6. Check: the storyboard has one panel and it shows a picture.

## Two empty panels are added and the last one is dragged to the front

1. Click Add Panel in the panel grid and choose Empty Panel. Do it twice.
2. Check: the panels are, in order: picture, empty, empty.
3. Drag the third panel onto the first panel's place.
4. Check: the first panel is now an empty one.

## After a restart the note is where it was dropped and the panels keep their order

1. Click the Moodboards tab and note the note tile's position.
2. Open the project menu and choose Close project. Quit the app and start it again on the same data. Click the project row "Open Pictures First".
3. Click Production, then Moodboards. Check: the note tile is at the noted position.
4. Click Storyboards. Check: three panels show, in order: empty, picture, empty.

## Platform notes

- None beyond the shared steps.
