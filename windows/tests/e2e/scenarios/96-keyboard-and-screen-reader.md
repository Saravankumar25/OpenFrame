# Keyboard and screen reader pass

Spec: `tests/e2e/specs/96-keyboard-and-screen-reader.test.mjs`

Tags: `@e2e-data-root: own`

Before: an empty data folder of its own. The app starts for the first time. Findings are written to `keyboard-and-screen-reader.json` in the artifacts folder.

## A project with a screenplay to walk through

1. Do the first launch: name 'E2E Tester', project title 'Quiet Keys'.
2. Import the Night Bus screenplay (see the README).

## Each of the eight main workspaces: every Tab stop is named and visibly focused, and the window has no unnamed control

This scenario runs once each for Home, Idea Vault, Story, Screenplay, Breakdown, Production, Call Sheets and Files.

1. Click the workspace in the Workspaces navigation and wait about 1 second.
2. Move focus off everything and scroll to the top.
3. Press Tab, up to 80 times. After each press:
   - Check: focus is on an element, not on the page itself.
   - Check: the focused element is visible.
   - Check: the focused element has an accessible name (a label, a title, a placeholder or visible text).
   - Check: the focused element shows a focus mark: an outline or a ring, or a text caret in an editor.
   - Check: focus has not stayed on the same element for four presses in a row (a trap).
4. Read the window through the operating system's accessibility tree (the tree a screen reader reads).
5. Check: no visible button, field, box, check box, radio button, menu item, tab, link, slider, list item or tree item in the window lacks a name.

## Notes

- Mac: Tab moves through every control only when "Keyboard navigation" is on in System Settings; the run turns it on first.

## Platform notes

- The accessibility tree is read with Windows UI Automation, which Narrator uses. Mac: the macOS accessibility tree that VoiceOver uses, through XCUITest. To decide in phase M-track.
