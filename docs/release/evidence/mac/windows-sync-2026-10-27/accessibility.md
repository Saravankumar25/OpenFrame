# Mac accessibility pass (phase M9), 2026-10-27

Scenarios: `apps/macos/Sources/OpenFrameE2EDriver/Scenarios/AccessibilityScenarios.swift`
(`96-keyboard-and-screen-reader/a11y-pass-names-light-and-dark` and `.../a11y-pass-keyboard-reach`).
Last run: `release-out/e2e/20261027-184401/`, both pass. Each run writes `accessibility-names.txt`,
`accessibility-names.json` and `accessibility-keyboard.txt` to its scenario folder.

## What was checked

The app opens a project with the Night Bus screenplay imported. In Light and then Dark, the pass reads
the accessibility tree (the tree VoiceOver reads) of 30 places: Home, Idea Vault, Story (Story page, Beat
Board, Outline, Characters), Screenplay, Breakdown, the 13 Production tabs, Call Sheets, Files, the AI
Assistant panel, and the six Settings tabs. For each visible control (buttons, menu buttons, text fields,
check boxes, pop-ups, sliders, tabs, links, lists) it records a missing name, two visible controls with
one identifier, and images with no description that are not hidden. Between 3 (AI panel) and 34
(Production: Catalog) controls are read per place.

## Counts before and after (per mode; Light and Dark gave the same counts)

| Place | Before | After |
|---|---|---|
| Idea Vault | 1 shared identifier (`vault-empty` on Add Note and Record Note) | 0 |
| Story: Beat Board and Outline | 1 shared identifier each (`story-empty` on Add Scene and Add Act) | 0 |
| Story: Characters | 1 shared identifier (`story-characters` on Add Character and Show archived) | 0 |
| Production: Budget | 1 shared identifier (`budget-empty` on Currency and Start Budget) | 0 |
| Screenplay | 1 unnamed list (the scene navigator) | 0 |
| Settings: Keyboard Shortcuts | 1 unnamed list (`shortcuts-list`) | 0 |
| All other places | 0 | 0 |

No unnamed button, field or image was found in any place. The shared identifiers came from an
identifier on an empty-state container that SwiftUI copied over its buttons' own identifiers; the
container is now its own accessibility group, so each button keeps its identifier. The lists are now
named "Scenes" (the Windows name) and "Keyboard shortcuts".

## Keyboard only

"Keyboard navigation" is off in System Settings on this machine. With it off, macOS Tab stops only at
text fields, text views and lists, and the app cannot turn the setting on for itself (tested: the
preference, a launch argument and the app's own domain all leave it off). The run does not change the
person's system settings. Results with it off, in all eight workspaces: every text field and list is
reached, every stop is named, focus never sticks, and Shift+Tab walks the stops back in order. The
screenplay text view takes Tab as text; Control+Tab leaves it, as macOS expects. Home, Breakdown,
Production and Call Sheets have no field or list, so Tab stays on the sidebar; their buttons are reached
only with Keyboard navigation on (or through the menus and shortcuts).

## What remains (needs a person)

- A walk with Keyboard navigation on in System Settings, to see Tab reach the buttons of Home,
  Breakdown, Production and Call Sheets, and to see the focus ring in Light and Dark.
- VoiceOver itself: reading order, whether changes are announced (saved, imported, errors), how rows
  of the scene navigator and lists are read, and the AI Assistant panel with a model installed (this
  build showed 3 controls in it).
- Command+J once needed a second press to open the AI Assistant after the Settings window closed (seen
  once in four runs, not reproduced on its own). A person should try to repeat it.
- The toolbar items speak their visible title ("Assistant") rather than the label set in code ("Open
  AI Assistant"); the visible title is a fair name, so this was left.
- Dynamic Type and Increase Contrast were not part of this pass.
