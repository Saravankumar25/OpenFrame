# Screen defects found on 2026-11-05, for the app work

Found while every screen of `windows/WindowsHTML` and `mac/MacHTML` was looked at in light and dark. This file is internal and never ships.

## Repaired on the website, still wrong in the app or its capture

Windows, in `WIN_REPAIRS` and `winRepair` of `scripts/shots.mjs`:

- `vault-multiselect-bar`, `vault-item-context-menu-multi`: the selection tick covers the first letter of the card title.
- `story-board-multiselect-bar`: the selection bar cuts the line under the Beat Board title in half.
- `notes-task-editor-related-results`: a long result title is squeezed to one word per line.
- `prod-moodboard-new-filled`: the name error stays on screen after a name is typed.
- `prod-shot-menu`, `prod-shot-drawer`, `prod-shot-move-dialog`, `prod-shot-attach-dialog`, `prod-shots-scene-actions-menu`, `prod-shots-copy-planning-dialog`, `prod-shots-export-dialog`: the shot composer error stays on screen from the step before.
- `pkg-export-script-scene-chosen`, `pkg-export-beat-board-one-act`, the search palette: "1 scenes", "1 comments", "1 acts", "1 sequences", "1 beats", "1 commands".
- The sample themes are signed "Capture tour".

Mac, in `scripts/mac-repairs.mjs`:

- `package-backup-ready`, `package-backup-portable`, `package-backup-created`, `package-export-full-project-done`, `package-share-script-done`, `package-review-response-exported`: the test folder name `capture-packages` shows in paths.
- `schedule-strip-drawer`: raw `**Location:**` and `**Cast:**` in the scene drawer.
- `pages-files-list` light: an empty toast shell.
- Light screens 080 to 107: the capture left the Dark page option on, so the script page is dark on a light window.
- `vault-collection-empty`, `vault-collection-with-item`, `vault-add-to-collection-sheet` dark: a focus ring over a sidebar label.

## Not repaired: shown as captured

Mac, real app layout seen in the pictures:

- `settings-theme-paste-sheet`, `settings-theme-paste-error`: the Import theme sheet runs past the bottom of the window, so its buttons are not visible.
- `screenplay-compare-drafts`, `screenplay-compare-text-only`, `-next-change`, `-scene-only`, `-inline`, `-before-menu`, `-same-draft`: the buttons read "Pi", "N", "Do...".
- `story-add-relationship-sheet`, `story-relationship-suggestions`, `story-add-relationship-filled`: the relationship field is two characters wide.
- `breakdown-filter-none`, `breakdown-filter-complete`: the Scenes heading breaks into "Scene" and "s".
- `breakdown-filter-missing`: the scene list slides under the sidebar.
- `breakdown-element-context-menu`, `schedule-day-detail-menu`: the menu is clipped at the window edge.
- `schedule-day-edit-date`: a second date control beside the field, showing another day.
- `callsheet-editor` and its sections: "Needs inpu" in the Crew Call box.
- `budget-snapshot-view`: the Difference column is cut off until scrolled.
- `package-import-preview`, `package-import-preview-expanded`: cut-off button labels.
- `pages-deleted-items`, `pages-deleted-restored`, `pages-deleted-purge-confirm`: list rows stretched to about 360 px, with one row out of view.
- `vault-new-collection-sheet` dark: a focus ring around a sidebar row.
- `home-global-vault`: the search placeholder is clipped.

Mac, wrong state for the name, recapture:

- `vault-recently-deleted-empty`: a band in the middle of the window, side lists missing.
- `vault-pinned-empty`: shows three notes.
- `vault-folder-empty`, `vault-folder-with-item`: the folder is selected but the whole vault shows.
- `screenplay-revisions-changed`: says "No revisions yet" while the toolbar shows Revision A.
- `screenplay-tools-writing-room-mode`: nothing shows a writing room mode.
- `story-board-zoom-out`: named for 70 percent, shows 100 percent.
- Dark vault and Breakdown screens: the window is inactive (grey traffic lights) in some captures.

Mac, never captured, simulated by `scripts/sim-mac.mjs` until a capture with the AI model exists: AI answers, proposals and Straft AI settings details, plus changed scene notices, removed scenes, import failed, a moved linked file, call sheet notices and the trial reminder. The answers and some sample values are illustrative.

Mac, no screen at all: sketches in the Idea Vault (the Mac app has none) and changing a scene number. Their capability entries have an empty Mac list.

Windows, as captured:

- `prod-moodboard-image-drawer`: shows the link drawer.
- `story-delete-character-dialog-lakshmi-in-use`, `-padma-in-use`: the dialog counts 44 and 12 scenes where the list shows 22 and 6.
- `ai-answer-scene-count`: asked how many scenes, the answer says "That is in Notes & Tasks > Tasks". The tutorial says so honestly. Recapture with a real count, and check the pointer in the assistant.
- `story-beat-drawer`: the None chip touches the first swatch.
- `prod-dood-corrected`, `prod-dood-note-dialog`: a bare code letter "T" instead of a word.
