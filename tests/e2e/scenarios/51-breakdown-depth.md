# Breakdown depth

Spec: `tests/e2e/specs/51-breakdown-depth.test.mjs`

Tags: `@e2e-data-root: own`

Before: an empty data folder of its own. The app starts for the first time.

"Import the Night Shift screenplay" is the shared step in the README of this folder. "The scene list" is the list "Scenes in the Production Source". "The breakdown panel" is the panel of the open scene. "Choose filter X" means: click the button whose name starts with "Filter scenes:", then choose X. "Production tab X" means: open Production if needed, then click X in the Production navigation.

"Add element C, N" means: click Add Element, choose C in the category list, type N in the name field, check the dialog no longer shows "Looking for existing catalog items", click the dialog button starting with "Add to Scene", then check the screen shows "Added “N”".

## Set the source; every category shows a colour, an icon, text and a tooltip

1. Do the first launch: type 'E2E Tester' as the name, click Continue, click Create Project, type 'Teak Signal' as the title, click Create.
2. Import the Night Shift screenplay.
3. Click Breakdown in the Workspaces navigation.
4. Click Select Source, click the draft in the "Screenplay drafts" choice, then click Set as Production Source.
5. Check: the panel "Scene 1 breakdown" shows.
6. Add element Props, 'Brass key'.
7. Check: the Props section of the panel shows a category mark.
8. Check: the mark's tooltip starts with "Props: ".
9. Check: the mark shows the text "Props".
10. Check: the mark has an icon.
11. Check: the mark's dot has a colour (it is not transparent).
12. Check: the screen is quiet.

## Accept Selected is one undo step; Reject Selected removes the rest; Show in script

1. Click Suggest Elements.
2. Check: the screen shows "Nothing is added to production until you accept."
3. Click Review…
4. Check: the dialog has a button starting with "Accept Selected", and the count in its brackets is at least 2. Call that count N.
5. Click Accept Selected.
6. Check: the screen shows "Accepted ".
7. Press Esc.
8. Check: the breakdown panel no longer shows "Suggested elements:".
9. Move focus off any text field and press Ctrl+Z.
10. Check: the breakdown panel shows "Suggested elements: N" (one undo brought every suggestion back).
11. Click Review…
12. Untick the first checkbox in the dialog.
13. Click Reject Selected (N minus 1) in the dialog.
14. Check: the screen shows "Rejected".
15. Press Esc.
16. Check: the breakdown panel shows "Suggested elements: 1".
17. In the panel "Scene 1 breakdown", click the button whose name starts with "Accept ". Note the element name.
18. Check: the screen shows "Accepted “<name>”".
19. Click Actions for that element, then choose Show in script.
20. Check: the words in the script that suggested it are highlighted.
21. Check: the screen is quiet.

## Filters: only scenes missing a breakdown, then quick reuse fills scene 2

1. Choose filter Only missing a breakdown.
2. Check: the scene list shows one scene.
3. Click that scene.
4. Check: the panel "Scene 2 breakdown" shows.
5. Click Reuse Brass key (Props) in Scene 2.
6. Check: the screen shows "Reused “Brass key” in Scene 2".
7. Check: the scene list is empty.
8. Choose filter All scenes.
9. Check: the scene list shows two scenes.
10. Check: the screen is quiet.

## A catalog item opens all its scenes

1. Production tab Catalog.
2. Click the table row for "Brass key".
3. Click Open Scenes in the drawer.
4. Check: the screen shows "scenes that use Brass key".
5. Check: the scene list shows two scenes.
6. Click Show all scenes.
7. Check: the screen is quiet.

## Location call sheet fields, availability dates, and crew availability

1. Production tab Locations.
2. Click Add Location.
3. Type 'Signal Cabin' in the location name field and 'Town Hospital, Station Road' in the nearest hospital field.
4. Click Add Location in the dialog.
5. Check: the screen shows "Added location “Signal Cabin”".
6. Check: the location's hospital field reads "Town Hospital, Station Road".
7. Set the availability start date to 20 June 2027, the end date to 22 June 2027, and the note to 'Track repairs'.
8. Click Add Dates.
9. Check: the screen shows "Updated availability of Signal Cabin" and "Track repairs".
10. Production tab Cast & Crew.
11. Click Crew, then click Add Person.
12. Type 'Imran Sheikh' in the name field and 'Cinematographer' in the role field.
13. Click Add to Crew in the dialog.
14. Check: the screen shows "Added Imran Sheikh to the crew".
15. Type 'Weekends only' in the availability field and press Tab.
16. Click Close in the drawer.
17. Click the table row for "Imran Sheikh".
18. Check: the availability field reads "Weekends only".
19. Check: the screen is quiet.

## Advanced settings change a category colour; everything survives a relaunch

1. Open the project menu and choose Project settings.
2. Click the Advanced tab in the Settings tabs.
3. Click Red in the "Props colour" group.
4. Check: the screen shows "Changed the look of Props".
5. Check: the screen is quiet.
6. Open the project menu and choose Close project.
7. Quit the app and start it again on the same data.
8. Click the project row "Open Teak Signal". Check: no recovery question shows.
9. Open the project menu and choose Project settings, then click the Advanced tab.
10. Check: Red in the "Props colour" group is pressed.
11. Production tab Locations.
12. Click the card "Signal Cabin".
13. Check: the hospital field reads "Town Hospital, Station Road".
14. Check: the screen shows "Track repairs".
15. Click Breakdown in the Workspaces navigation.
16. Check: the panel "Scene 1 breakdown" shows "Brass key".
17. Check: the screen is quiet.
18. Open the project menu and choose Close project.

## Notes

- On the Mac, Ctrl+Z uses Cmd instead.
- The availability dates are set without the system date picker.
