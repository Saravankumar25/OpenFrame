# MacHTML: every screen of OpenFrame Studio for the Mac

692 screens in 26 areas, each captured from the real app in Light mode (`light/`) and in Dark mode
(`dark/`). Open `index.html`. Page `NNN-name.html` is the same screen in both folders.

## What MacHTML is, and how it differs from WindowsHTML

WindowsHTML saved the Windows app's own DOM and CSS, because that app is a web view. The Mac app is native SwiftUI: there
is no DOM to save. MacHTML is built from two things captured at the same moment for every screen:

- the 2x picture of the window as the window server draws it (title bar, toolbar, sidebar, content, and any sheet or menu
  at its real position), and
- the full accessibility tree of the window: role, identifier, label, title, value, enabled, selected and focused state,
  and the frame of every element in points.

## How the pages are made

- The capture tour (`apps/macos/Sources/OpenFrameE2EDriver/Scenarios/CaptureTour*.swift`, scenarios `zz-capture/*`) drives
  the real app, seeds a project and visits every screen it can reach, once with Settings, Appearance, Mode set to Light and
  once with Dark. Only screens present in both runs become pages. A screen that looks the same as an earlier one is dropped.
- `scripts/mac-html/build-mac-html.mjs` writes one page per screen. The window picture is the background, cut into the
  main containers (toolbar, sidebar, content, inspector, status bar, each sheet and menu), found from the toolbar, the
  split group and its columns, the main scroll area and the sheet windows in the tree. Each part shows its own piece of
  the one picture, so together they give back the picture pixel for pixel. The window has a 10 point corner radius and a
  shadow drawn outside the picture.
- Every accessibility element is a real HTML element at its exact frame (one CSS pixel is one point): text as selectable
  text, buttons as buttons, fields with their value, check boxes and switches, radio buttons, pop-up buttons as select
  boxes with their choices, lists and outlines as lists, menus as menus, each with its label as `aria-label` and its
  identifier as `data-id`. In the normal view this layer is transparent over the picture.
- The HTML only switch at the top of each page hides the picture and shows the HTML layer alone on the window colour.
- A control that led to another captured screen is a link to that page. Each page also links to the same page in the
  other mode and to the index.
- No page holds a script, and nothing is loaded from the network. The Light and Dark switch on the index and the HTML only
  switch are plain CSS.
- Commit of the app these came from: `8d40f4e`.

## Window size

The window is 1280 x 634 points (shown at one CSS pixel per point; the pictures are 2x). Some screens were captured at another size: 1280 x 627 (46 screens), 1280 x 630 (20 screens), 760 x 548 (14 screens), 1280 x 633 (13 screens), 1280 x 628 (5 screens), 1280 x 631 (4 screens), 1280 x 632 (3 screens), 340 x 326 (1 screen), 1000 x 633 (1 screen).

## Regenerate

    apps/macos/E2E/capture.sh "apps/macos/build/OpenFrame Studio.app"

That runs the tour in Light and in Dark (with `OPENFRAME_E2E_PARALLEL=n` it runs n runners side by side), builds this
folder and checks every page (`scripts/mac-html/check-pages.swift`). To rebuild the pages from an earlier capture
without running the app:

    OPENFRAME_CAPTURE_SKIP_RUN=1 OPENFRAME_CAPTURE_DIR=<capture folder with light/ and dark/> apps/macos/E2E/capture.sh "apps/macos/build/OpenFrame Studio.app"

## Not captured

- The system Open and Save panels (they run in another process and block the driver).
- Real drag and drop.
- Voice recording (needs the microphone and its permission dialog).
- The print panel.
- Forced crashes and disk failures.
- Screens captured in one mode only: plan-storyboard-panel-duration-error, plan-storyboard-panel-filled, plan-storyboard-panel-open, plan-storyboard-panel-shot, plan-storyboard-panels-described.
- Steps the tour skipped:
  - Light: pop-up screenplay-page-setup-paper: no pop-up button Paper
  - Light: page setup paper: no pop-up button Paper
  - Light: custom element form: not found within 8 s: id=custom-element-add label=- role=-
  - Light: element picker with custom: no menu item starting with Telephone in ["", "Scene Heading", "Action", "Character", "Parenthetical", "Dialogue", "Transition", "Shot", "General note (not printed)", "", "Insert Page Br
  - Light: open and finalize: not true within 12 s: the call sheet is final
  - Light: location notes: no text input took focus: AXTextField id=loc-note-parking label=Parking: title= value=
  - Light: shot open: not found within 6 s: id=shot-size label=- role=-
  - Light: menu pages-templates-type-menu: not a menu button: AXStaticText id= label= title= value=Type
  - Light: template create: element has no frame: AXTextField id= label= title= value=
  - Light: own template: no own template
  - Light: template apply: no built-in template
  - Light: templates from project home: not found within 8 s: id=- label=Templates role=-
  - Light: menu screenplay-private-note-actions: the menu did not open: AXMenuButton id=private-note-actions label=Private note actions title=Actions value=
  - Light: private note editing: not found within 15 s: the menu button private-note-actions
  - Light: edit line: not found within 6 s: id=of-bl-desc label=- role=-
  - Light: snapshot: not found within 8 s: id=budget-snapshot-close label=- role=-
  - Light: rename sequence: not found within 4 s: id=- label=Sequence name role=-
  - Light: collapse sequence: not found within 8 s: id=- label=Collapse Missing the bus role=AXButton
  - Light: moodboard tile selected: not found within 5 s: id=moodboard-front label=- role=-
  - Light: moodboard link inspector: not found within 8 s: id=moodboard-item-url label=- role=-
  - Light: delete in use: no item Delete in the More menu
  - Light: delete unused: no item Delete in the More menu
  - Light: archive: not found within 8 s: id=- label=Show archived characters role=-
  - Light: timeline: not found within 8 s: id=- label=Timeline role=AXRadioButton
  - Light: report snapshot: not found within 8 s: id=- label=Back to live reports role=-
  - Light: delete things: not found within 8 s: id=task-title label=- role=-
  - Dark: pop-up screenplay-page-setup-paper: no pop-up button Paper
  - Dark: page setup paper: no pop-up button Paper
  - Dark: custom element form: not found within 8 s: id=custom-element-add label=- role=-
  - Dark: element picker with custom: no menu item starting with Telephone in ["", "Scene Heading", "Action", "Character", "Parenthetical", "Dialogue", "Transition", "Shot", "General note (not printed)", "", "Insert Page Bre
  - Dark: open and finalize: not true within 12 s: the call sheet is final
  - Dark: location notes: no text input took focus: AXTextField id=loc-note-parking label=Parking: title= value=
  - Dark: shot open: not found within 6 s: id=shot-size label=- role=-
  - Dark: menu pages-templates-type-menu: not a menu button: AXStaticText id= label= title= value=Type
  - Dark: template create: element has no frame: AXTextField id= label= title= value=
  - Dark: own template: no own template
  - Dark: template apply: no built-in template
  - Dark: templates from project home: not found within 8 s: id=- label=Templates role=-
  - Dark: menu screenplay-private-note-actions: the menu did not open: AXMenuButton id=private-note-actions label=Private note actions title=Actions value=
  - Dark: private note editing: not found within 15 s: the menu button private-note-actions
  - Dark: edit line: not found within 6 s: id=of-bl-desc label=- role=-
  - Dark: snapshot: not found within 8 s: id=budget-snapshot-close label=- role=-
  - Dark: rename sequence: not found within 4 s: id=- label=Sequence name role=-
  - Dark: collapse sequence: not found within 8 s: id=- label=Collapse Missing the bus role=AXButton
  - Dark: moodboard tile selected: not found within 5 s: id=moodboard-front label=- role=-
  - Dark: moodboard link inspector: not found within 8 s: id=moodboard-item-url label=- role=-
  - Dark: delete in use: no item Delete in the More menu
  - Dark: delete unused: no item Delete in the More menu
  - Dark: archive: not found within 8 s: id=- label=Show archived characters role=-
  - Dark: timeline: not found within 8 s: id=- label=Timeline role=AXRadioButton
  - Dark: report snapshot: not found within 8 s: id=- label=Back to live reports role=-
  - Dark: storyboard panel open: not found within 8 s: id=panel-inspector label=- role=-
  - Dark: storyboard panel shot: not found within 8 s: id=panel-create-shot label=- role=-
  - Dark: delete things: not found within 8 s: id=task-title label=- role=-

## Count

692 screens, 1384 pages (Light and Dark).

## Verification: pages against the pictures

Every page was rendered in WebKit in its normal view at the window's own size and at the picture's scale (2x), and
the window part was compared with its capture picture. A pixel differs when any colour channel differs by more than
24 of 255; a page fails when more than 0.5 percent of its pixels differ (the window's transparent rounded corners
are not compared). The check also looks for every non-empty label and value of the accessibility tree in the page
(as text, a control value or an aria-label); any miss fails the page.

### light: every page passed

| Page | Differing pixels | HTML misses | Result |
|---|---|---|---|
| home-welcome | 0.000% | 0 | pass |
| home-welcome-filled | 0.000% | 0 | pass |
| home-empty | 0.000% | 0 | pass |
| menu-app | 0.000% | 0 | pass |
| menu-file-home | 0.000% | 0 | pass |
| menu-edit-home | 0.000% | 0 | pass |
| menu-view-home | 0.000% | 0 | pass |
| menu-go-home | 0.000% | 0 | pass |
| menu-window-home | 0.000% | 0 | pass |
| menu-help-home | 0.000% | 0 | pass |
| new-project-sheet | 0.000% | 0 | pass |
| new-project-filled | 0.000% | 0 | pass |
| new-project-type-menu | 0.000% | 0 | pass |
| new-project-short-film | 0.000% | 0 | pass |
| search-palette-home | 0.000% | 0 | pass |
| search-palette-home-commands | 0.000% | 0 | pass |
| search-palette-empty | 0.000% | 0 | pass |
| home-shortcuts-sheet | 0.000% | 0 | pass |
| about | 0.000% | 0 | pass |
| settings-general | 0.000% | 0 | pass |
| settings-general-name-edited | 0.000% | 0 | pass |
| settings-appearance | 0.000% | 0 | pass |
| settings-appearance-scrolled | 0.000% | 0 | pass |
| settings-appearance-theme-chosen | 0.000% | 0 | pass |
| settings-theme-paste-sheet | 0.000% | 0 | pass |
| settings-theme-paste-error | 0.000% | 0 | pass |
| settings-keyboard-shortcuts | 0.000% | 0 | pass |
| settings-keyboard-shortcuts-scrolled | 0.000% | 0 | pass |
| settings-shortcuts-filtered | 0.000% | 0 | pass |
| settings-shortcuts-none | 0.000% | 0 | pass |
| settings-shortcut-recording | 0.000% | 0 | pass |
| settings-offline-ai | 0.000% | 0 | pass |
| settings-diagnostics | 0.000% | 0 | pass |
| home-import-toast | 0.479% | 0 | pass (differences between 25,61 and 1271,570 points) |
| project-home | 0.000% | 0 | pass |
| project-menu | 0.000% | 0 | pass |
| project-new-menu | 0.000% | 0 | pass |
| project-home-status-menu | 0.000% | 0 | pass |
| menu-app-project | 0.000% | 0 | pass |
| menu-file-project | 0.000% | 0 | pass |
| menu-edit-project | 0.000% | 0 | pass |
| menu-view-project | 0.000% | 0 | pass |
| menu-go | 0.000% | 0 | pass |
| menu-window-project | 0.000% | 0 | pass |
| menu-help-project | 0.000% | 0 | pass |
| project-rename-sheet | 0.000% | 0 | pass |
| project-rename-empty | 0.000% | 0 | pass |
| project-home-sidebar-hidden | 0.000% | 0 | pass |
| project-home-after-back | 0.000% | 0 | pass |
| project-home-ai-panel | 0.000% | 0 | pass |
| search-palette-project | 0.000% | 0 | pass |
| search-palette-results | 0.000% | 0 | pass |
| project-shortcuts-sheet | 0.000% | 0 | pass |
| home-filled | 0.000% | 0 | pass |
| home-card-menu | 0.000% | 0 | pass |
| home-global-vault | 0.000% | 0 | pass |
| home-rename-sheet | 0.000% | 0 | pass |
| home-delete-sheet | 0.000% | 0 | pass |
| home-delete-typed | 0.000% | 0 | pass |
| project-home-empty | 0.000% | 0 | pass |
| home-two-projects | 0.000% | 0 | pass |
| home-search-results | 0.015% | 0 | pass (differences between 714,121 and 727,134 points) |
| home-search-none | 0.015% | 0 | pass (differences between 714,121 and 727,134 points) |
| home-pinned | 0.000% | 0 | pass |
| home-one-archived | 0.000% | 0 | pass |
| home-archived-page | 0.000% | 0 | pass |
| home-archived-empty | 0.000% | 0 | pass |
| screenplay-editor-typing | 0.000% | 0 | pass |
| screenplay-tools-menu | 0.000% | 0 | pass |
| screenplay-tools-add-comment | 0.000% | 0 | pass |
| screenplay-tools-scene-hub | 0.000% | 0 | pass |
| screenplay-tools-drafts | 0.000% | 0 | pass |
| screenplay-tools-lock-as-shooting-draft | 0.000% | 0 | pass |
| screenplay-tools-page-setup | 0.000% | 0 | pass |
| screenplay-tools-title-page | 0.000% | 0 | pass |
| screenplay-tools-custom-elements | 0.000% | 0 | pass |
| screenplay-tools-page-view | 0.000% | 0 | pass |
| screenplay-tools-import-screenplay | 0.000% | 0 | pass |
| screenplay-tools-export-screenplay | 0.000% | 0 | pass |
| screenplay-draft-menu | 0.000% | 0 | pass |
| screenplay-draft-new-draft | 0.000% | 0 | pass |
| screenplay-draft-manage-drafts | 0.000% | 0 | pass |
| screenplay-element-picker | 0.000% | 0 | pass |
| screenplay-element-picker-dialogue | 0.000% | 0 | pass |
| screenplay-comments | 0.000% | 0 | pass |
| screenplay-compare-drafts | 0.000% | 0 | pass |
| screenplay-find-and-replace | 0.000% | 0 | pass |
| screenplay-picker-menu | 0.000% | 0 | pass |
| screenplay-new-screenplay-sheet | 0.000% | 0 | pass |
| screenplay-new-screenplay-duplicate | 0.000% | 0 | pass |
| screenplay-rename-screenplay-sheet | 0.000% | 0 | pass |
| screenplay-delete-screenplay-sheet | 0.000% | 0 | pass |
| screenplay-scenes-hidden | 0.000% | 0 | pass |
| screenplay-navigator-columns-menu | 0.000% | 0 | pass |
| screenplay-navigator-columns-shown | 0.000% | 0 | pass |
| screenplay-scene-row-context | 0.000% | 0 | pass |
| screenplay-editor-context | 0.000% | 0 | pass |
| screenplay-editor-selection | 0.000% | 0 | pass |
| screenplay-editor-context-selection | 0.000% | 0 | pass |
| screenplay-editor-context-spelling | 0.000% | 0 | pass |
| screenplay-zoom-50 | 0.000% | 0 | pass |
| screenplay-zoom-150 | 0.000% | 0 | pass |
| screenplay-zoom-200 | 0.000% | 0 | pass |
| screenplay-layout-focus | 0.000% | 0 | pass |
| screenplay-empty-editor | 0.000% | 0 | pass |
| screenplay-empty-element-picker | 0.000% | 0 | pass |
| screenplay-empty-draft-menu | 0.000% | 0 | pass |
| screenplay-empty-comments | 0.000% | 0 | pass |
| screenplay-empty-scene-hub | 0.000% | 0 | pass |
| screenplay-empty-draft-info | 0.000% | 0 | pass |
| screenplay-empty-drafts | 0.000% | 0 | pass |
| screenplay-empty-page-view | 0.000% | 0 | pass |
| screenplay-empty-export | 0.000% | 0 | pass |
| screenplay-empty-suggestions | 0.000% | 0 | pass |
| screenplay-empty-first-scene | 0.000% | 0 | pass |
| screenplay-find-bar | 0.015% | 0 | pass (differences between 1095,123 and 1108,136 points) |
| screenplay-find-results | 0.014% | 0 | pass (differences between 1095,123 and 1108,134 points) |
| screenplay-replace-filled | 0.000% | 0 | pass |
| screenplay-replace-whole-script | 0.000% | 0 | pass |
| screenplay-replace-done | 0.000% | 0 | pass |
| screenplay-replace-no-match | 0.000% | 0 | pass |
| screenplay-scene-hub-details-edited | 0.000% | 0 | pass |
| screenplay-scene-hub-from-list | 0.000% | 0 | pass |
| screenplay-comment-composer-quote | 0.000% | 0 | pass |
| screenplay-comment-posted | 0.000% | 0 | pass |
| screenplay-comment-reply-composer | 0.000% | 0 | pass |
| screenplay-comment-with-reply | 0.000% | 0 | pass |
| screenplay-comment-composer-draft | 0.000% | 0 | pass |
| screenplay-comments-two-open | 0.000% | 0 | pass |
| screenplay-comments-after-resolve | 0.000% | 0 | pass |
| screenplay-comments-resolved-tab | 0.000% | 0 | pass |
| screenplay-comment-actions | 0.000% | 0 | pass |
| screenplay-comment-in-discussion | 0.000% | 0 | pass |
| screenplay-comment-composer-scene | 0.000% | 0 | pass |
| screenplay-new-draft-named | 0.000% | 0 | pass |
| screenplay-second-draft-created | 0.000% | 0 | pass |
| screenplay-second-draft-edited | 0.000% | 0 | pass |
| screenplay-draft-menu-two | 0.000% | 0 | pass |
| screenplay-drafts-two | 0.000% | 0 | pass |
| screenplay-draft-actions-current | 0.000% | 0 | pass |
| screenplay-draft-actions-other | 0.000% | 0 | pass |
| screenplay-draft-rename | 0.000% | 0 | pass |
| screenplay-draft-delete | 0.000% | 0 | pass |
| screenplay-draft-new-from | 0.000% | 0 | pass |
| screenplay-compare-text-only | 0.000% | 0 | pass |
| screenplay-compare-next-change | 0.000% | 0 | pass |
| screenplay-compare-scene-only | 0.000% | 0 | pass |
| screenplay-compare-inline | 0.000% | 0 | pass |
| screenplay-compare-before-menu | 0.000% | 0 | pass |
| screenplay-compare-same-draft | 0.000% | 0 | pass |
| screenplay-locked | 0.000% | 0 | pass |
| screenplay-tools-menu-locked | 0.000% | 0 | pass |
| screenplay-revisions-locked | 0.000% | 0 | pass |
| screenplay-start-revision | 0.000% | 0 | pass |
| screenplay-revision-started | 0.000% | 0 | pass |
| screenplay-revision-edited | 0.000% | 0 | pass |
| screenplay-revisions-changed | 0.000% | 0 | pass |
| screenplay-page-view-revision | 0.000% | 0 | pass |
| screenplay-page-view-marks-toggled | 0.000% | 0 | pass |
| screenplay-drafts-locked-and-revision | 0.000% | 0 | pass |
| screenplay-revision-label-colour | 0.000% | 0 | pass |
| screenplay-inspector-draft-revision | 0.000% | 0 | pass |
| screenplay-title-page-filled | 0.000% | 0 | pass |
| screenplay-title-page-saved | 0.000% | 0 | pass |
| screenplay-page-view-a4 | 0.000% | 0 | pass |
| screenplay-export-fdx | 0.000% | 0 | pass |
| screenplay-export-fountain | 0.000% | 0 | pass |
| screenplay-export-docx | 0.000% | 0 | pass |
| screenplay-export-txt | 0.000% | 0 | pass |
| screenplay-export-selected-scenes | 0.000% | 0 | pass |
| screenplay-export-options | 0.000% | 0 | pass |
| screenplay-print-preview | 0.000% | 0 | pass |
| screenplay-print-preview-page-2 | 0.000% | 0 | pass |
| screenplay-print-preview-zoomed | 0.000% | 0 | pass |
| screenplay-print-preview-no-title | 0.000% | 0 | pass |
| screenplay-print-from-menu | 0.000% | 0 | pass |
| story-board-empty | 0.000% | 0 | pass |
| story-add-menu | 0.000% | 0 | pass |
| story-timeline-empty | 0.000% | 0 | pass |
| story-page-empty | 0.000% | 0 | pass |
| story-page-written | 0.000% | 0 | pass |
| story-page-export-sheet | 0.000% | 0 | pass |
| story-page-export-docx | 0.000% | 0 | pass |
| story-add-act-inline | 0.000% | 0 | pass |
| story-board-one-act | 0.000% | 0 | pass |
| story-add-sequence-inline | 0.000% | 0 | pass |
| story-sequence-empty | 0.000% | 0 | pass |
| story-add-card-inline | 0.000% | 0 | pass |
| story-add-beat-inline | 0.000% | 0 | pass |
| story-board-filled | 0.000% | 0 | pass |
| story-card-selected | 0.000% | 0 | pass |
| story-multi-select | 0.000% | 0 | pass |
| story-move-sheet | 0.000% | 0 | pass |
| story-card-context-menu | 0.000% | 0 | pass |
| story-beat-context-menu | 0.000% | 0 | pass |
| story-sequence-context-menu | 0.000% | 0 | pass |
| story-parked-card-context-menu | 0.000% | 0 | pass |
| story-act-context-menu | 0.000% | 0 | pass |
| story-card-inspector | 0.000% | 0 | pass |
| story-card-inspector-filled | 0.000% | 0 | pass |
| story-board-card-details | 0.000% | 0 | pass |
| story-beat-inspector | 0.000% | 0 | pass |
| story-sequence-inspector | 0.000% | 0 | pass |
| story-parked-card-inspector | 0.000% | 0 | pass |
| story-act-rename-inline | 0.000% | 0 | pass |
| story-act-note-sheet | 0.000% | 0 | pass |
| story-delete-act-sheet | 0.000% | 0 | pass |
| story-move-destination-menu | 0.000% | 0 | pass |
| story-build-sheet-one-card | 0.000% | 0 | pass |
| story-deleted-notice | 0.000% | 0 | pass |
| story-act-collapsed | 0.000% | 0 | pass |
| story-board-filtered | 0.000% | 0 | pass |
| story-board-filter-no-match | 0.000% | 0 | pass |
| story-board-parking-hidden | 0.000% | 0 | pass |
| story-more-menu | 0.000% | 0 | pass |
| story-card-display-sheet | 0.000% | 0 | pass |
| story-card-display-production | 0.000% | 0 | pass |
| story-board-production-cards | 0.000% | 0 | pass |
| story-apply-order-sheet | 0.000% | 0 | pass |
| story-build-sheet | 0.000% | 0 | pass |
| story-outline | 0.000% | 0 | pass |
| story-outline-act-menu | 0.000% | 0 | pass |
| story-outline-row-context-menu | 0.000% | 0 | pass |
| story-outline-row-selected | 0.000% | 0 | pass |
| story-outline-inspector | 0.000% | 0 | pass |
| story-outline-act-collapsed | 0.000% | 0 | pass |
| story-outline-filtered | 0.000% | 0 | pass |
| story-beat-converted | 0.000% | 0 | pass |
| story-board-unassigned | 0.000% | 0 | pass |
| story-characters-empty | 0.000% | 0 | pass |
| story-add-character-sheet | 0.000% | 0 | pass |
| story-add-character-filled | 0.000% | 0 | pass |
| story-character-one | 0.000% | 0 | pass |
| story-add-character-match | 0.000% | 0 | pass |
| story-characters | 0.000% | 0 | pass |
| story-character-edit-sheet | 0.000% | 0 | pass |
| story-character-more-menu | 0.000% | 0 | pass |
| story-relationship-map-empty | 0.000% | 0 | pass |
| story-add-relationship-sheet | 0.000% | 0 | pass |
| story-relationship-suggestions | 0.000% | 0 | pass |
| story-add-relationship-filled | 0.000% | 0 | pass |
| story-character-relationships | 0.000% | 0 | pass |
| story-relationship-map | 0.000% | 0 | pass |
| story-character-linked-card | 0.000% | 0 | pass |
| story-characters-archived-hidden | 0.000% | 0 | pass |
| vault-empty | 0.000% | 0 | pass |
| vault-recently-deleted-empty | 0.000% | 0 | pass |
| vault-add-menu | 0.000% | 0 | pass |
| vault-add-note | 0.000% | 0 | pass |
| vault-add-url | 0.000% | 0 | pass |
| vault-add-audio-voice-note | 0.000% | 0 | pass |
| vault-add-quote | 0.000% | 0 | pass |
| vault-add-quick-capture | 0.000% | 0 | pass |
| vault-quick-capture-typed | 0.000% | 0 | pass |
| vault-saved-notice | 0.000% | 0 | pass |
| vault-add-url-error | 0.000% | 0 | pass |
| vault-add-url-filled | 0.000% | 0 | pass |
| vault-drawer-quote | 0.000% | 0 | pass |
| vault-filled | 0.000% | 0 | pass |
| vault-view-card | 0.000% | 0 | pass |
| vault-view-list | 0.000% | 0 | pass |
| vault-view-folder | 0.000% | 0 | pass |
| vault-drawer-note | 0.000% | 0 | pass |
| vault-drawer-flag-menu | 0.000% | 0 | pass |
| vault-drawer-flag-and-tag | 0.000% | 0 | pass |
| vault-move-sheet | 0.000% | 0 | pass |
| vault-copy-to-global-sheet | 0.000% | 0 | pass |
| vault-send-to-story-sheet | 0.000% | 0 | pass |
| vault-drawer-url | 0.000% | 0 | pass |
| vault-item-selected | 0.000% | 0 | pass |
| vault-tag-sheet | 0.000% | 0 | pass |
| vault-card-context-menu | 0.000% | 0 | pass |
| vault-card-flag-submenu | 0.000% | 0 | pass |
| vault-pinned-empty | 0.000% | 0 | pass |
| vault-pinned-notice | 0.000% | 0 | pass |
| vault-pinned | 0.000% | 0 | pass |
| vault-tag-sheet-filled | 0.000% | 0 | pass |
| vault-tag-view | 0.000% | 0 | pass |
| vault-new-folder-sheet | 0.000% | 0 | pass |
| vault-new-folder-error | 0.000% | 0 | pass |
| vault-folder-empty | 0.000% | 0 | pass |
| vault-move-sheet-chosen | 0.000% | 0 | pass |
| vault-folder-with-item | 0.000% | 0 | pass |
| vault-folder-context-menu | 0.000% | 0 | pass |
| vault-rename-folder-sheet | 0.000% | 0 | pass |
| vault-new-subfolder-sheet | 0.000% | 0 | pass |
| vault-folder-mode-top | 0.000% | 0 | pass |
| vault-folder-mode-inside | 0.000% | 0 | pass |
| vault-recently-added | 0.000% | 0 | pass |
| vault-search-results | 0.015% | 0 | pass (differences between 1260,19 and 1272,32 points) |
| vault-search-none | 0.015% | 0 | pass (differences between 1260,19 and 1272,32 points) |
| vault-new-collection-sheet | 0.000% | 0 | pass |
| vault-collection-empty | 0.000% | 0 | pass |
| vault-collection-with-item | 0.000% | 0 | pass |
| vault-add-to-collection-sheet | 0.000% | 0 | pass |
| vault-collection-context-menu | 0.000% | 0 | pass |
| vault-rename-collection-sheet | 0.000% | 0 | pass |
| vault-deleted-notice | 0.000% | 0 | pass |
| vault-recently-deleted | 0.000% | 0 | pass |
| vault-purge-confirm | 0.000% | 0 | pass |
| vault-restored | 0.000% | 0 | pass |
| vault-global-empty | 0.000% | 0 | pass |
| vault-global-with-copy | 0.000% | 0 | pass |
| vault-global-drawer | 0.000% | 0 | pass |
| vault-global-card-context-menu | 0.000% | 0 | pass |
| breakdown-no-source | 0.000% | 0 | pass |
| breakdown-source-sheet | 0.000% | 0 | pass |
| breakdown-source-sheet-reason | 0.000% | 0 | pass |
| breakdown-scenes | 0.000% | 0 | pass |
| breakdown-scene-3 | 0.000% | 0 | pass |
| breakdown-scenes-hidden | 0.000% | 0 | pass |
| breakdown-script-only | 0.000% | 0 | pass |
| breakdown-panel-hidden | 0.000% | 0 | pass |
| breakdown-filter-menu | 0.000% | 0 | pass |
| breakdown-filter-none | 0.000% | 0 | pass |
| breakdown-filter-missing | 0.000% | 0 | pass |
| breakdown-change-source | 0.000% | 0 | pass |
| breakdown-export-sheet | 0.000% | 0 | pass |
| breakdown-export-csv | 0.000% | 0 | pass |
| breakdown-export-xlsx | 0.000% | 0 | pass |
| breakdown-export-options | 0.000% | 0 | pass |
| breakdown-script-selection | 0.000% | 0 | pass |
| breakdown-script-context-menu | 0.000% | 0 | pass |
| breakdown-tag-menu | 0.000% | 0 | pass |
| breakdown-tag-sheet | 0.000% | 0 | pass |
| breakdown-tagged | 0.000% | 0 | pass |
| breakdown-suggestions | 0.000% | 0 | pass |
| breakdown-review-sheet | 0.000% | 0 | pass |
| breakdown-review-deselected | 0.000% | 0 | pass |
| breakdown-edit-suggestion | 0.000% | 0 | pass |
| breakdown-accepted-one | 0.000% | 0 | pass |
| breakdown-rejected-one | 0.000% | 0 | pass |
| breakdown-all-accepted | 0.000% | 0 | pass |
| breakdown-category-collapsed | 0.000% | 0 | pass |
| breakdown-element-selected | 0.000% | 0 | pass |
| breakdown-element-actions-menu | 0.000% | 0 | pass |
| breakdown-notes-sheet | 0.000% | 0 | pass |
| breakdown-notes-filled | 0.000% | 0 | pass |
| breakdown-element-notes | 0.000% | 0 | pass |
| breakdown-scene-complete | 0.000% | 0 | pass |
| breakdown-add-element-sheet | 0.000% | 0 | pass |
| breakdown-add-category-menu | 0.000% | 0 | pass |
| breakdown-add-element-filled | 0.000% | 0 | pass |
| breakdown-element-added | 0.000% | 0 | pass |
| breakdown-add-element-existing | 0.000% | 0 | pass |
| breakdown-element-removed | 0.000% | 0 | pass |
| breakdown-element-restored | 0.000% | 0 | pass |
| breakdown-scene-2 | 0.000% | 0 | pass |
| breakdown-reused | 0.000% | 0 | pass |
| breakdown-catalog-filter | 0.000% | 0 | pass |
| breakdown-filter-cast | 0.000% | 0 | pass |
| breakdown-empty-no-source | 0.000% | 0 | pass |
| breakdown-empty-source-sheet | 0.000% | 0 | pass |
| schedule-dood-empty | 0.000% | 0 | pass |
| schedule-daily-empty | 0.000% | 0 | pass |
| schedule-sides-empty | 0.000% | 0 | pass |
| schedule-empty | 0.000% | 0 | pass |
| schedule-create-sheet | 0.000% | 0 | pass |
| schedule-board-unscheduled | 0.000% | 0 | pass |
| schedule-add-day-sheet | 0.000% | 0 | pass |
| schedule-add-day-error | 0.000% | 0 | pass |
| schedule-board-three-days | 0.000% | 0 | pass |
| schedule-add-off-day-sheet | 0.000% | 0 | pass |
| schedule-board-off-day | 0.000% | 0 | pass |
| schedule-strip-menu | 0.000% | 0 | pass |
| schedule-board | 0.000% | 0 | pass |
| schedule-strip-drawer | 0.000% | 0 | pass |
| schedule-day-menu | 0.000% | 0 | pass |
| schedule-meal-break-sheet | 0.000% | 0 | pass |
| schedule-board-meal-break | 0.000% | 0 | pass |
| schedule-break-menu | 0.000% | 0 | pass |
| schedule-edit-break-sheet | 0.000% | 0 | pass |
| schedule-filter-menu | 0.000% | 0 | pass |
| schedule-board-filtered | 0.000% | 0 | pass |
| schedule-day-detail | 0.000% | 0 | pass |
| schedule-day-detail-times | 0.000% | 0 | pass |
| schedule-day-detail-menu | 0.000% | 0 | pass |
| schedule-day-add-menu | 0.000% | 0 | pass |
| schedule-day-add-meal-break | 0.000% | 0 | pass |
| schedule-day-add-travel | 0.000% | 0 | pass |
| schedule-day-add-company-move | 0.000% | 0 | pass |
| schedule-day-add-custom-note | 0.000% | 0 | pass |
| schedule-day-edit-date | 0.000% | 0 | pass |
| schedule-day-export | 0.000% | 0 | pass |
| schedule-day-empty | 0.000% | 0 | pass |
| schedule-list | 0.000% | 0 | pass |
| schedule-calendar | 0.000% | 0 | pass |
| schedule-suggest-grouping | 0.000% | 0 | pass |
| schedule-warnings | 0.000% | 0 | pass |
| schedule-schedule-settings | 0.000% | 0 | pass |
| schedule-export-schedule | 0.000% | 0 | pass |
| schedule-settings-error | 0.000% | 0 | pass |
| schedule-dood | 0.000% | 0 | pass |
| schedule-dood-code-style-menu | 0.000% | 0 | pass |
| schedule-dood-export | 0.000% | 0 | pass |
| schedule-daily | 0.000% | 0 | pass |
| schedule-daily-day-menu | 0.000% | 0 | pass |
| schedule-daily-export | 0.000% | 0 | pass |
| schedule-daily-day-2 | 0.000% | 0 | pass |
| schedule-daily-day-3 | 0.000% | 0 | pass |
| schedule-sides | 0.000% | 0 | pass |
| schedule-sides-day-menu | 0.000% | 0 | pass |
| schedule-sides-selected-cast | 0.000% | 0 | pass |
| schedule-sides-cover | 0.000% | 0 | pass |
| schedule-sides-export | 0.000% | 0 | pass |
| schedule-sides-saved | 0.000% | 0 | pass |
| schedule-sides-snapshot | 0.000% | 0 | pass |
| schedule-reports-scene | 0.000% | 0 | pass |
| schedule-reports-location | 0.000% | 0 | pass |
| schedule-reports-cast-scene | 0.000% | 0 | pass |
| schedule-reports-prop | 0.000% | 0 | pass |
| schedule-reports-schedule | 0.000% | 0 | pass |
| schedule-reports-breakdown-completeness | 0.000% | 0 | pass |
| schedule-reports-filtered | 0.000% | 0 | pass |
| schedule-reports-export | 0.000% | 0 | pass |
| callsheets-empty | 0.000% | 0 | pass |
| callsheet-create-sheet | 0.000% | 0 | pass |
| callsheet-create-day-menu | 0.000% | 0 | pass |
| callsheet-create-template-menu | 0.000% | 0 | pass |
| callsheet-editor | 0.000% | 0 | pass |
| callsheet-section-menu | 0.000% | 0 | pass |
| callsheet-section-weather | 0.000% | 0 | pass |
| callsheet-section-extras | 0.000% | 0 | pass |
| callsheet-section-crew | 0.000% | 0 | pass |
| callsheet-section-walkie-channels-and-contacts | 0.000% | 0 | pass |
| callsheet-section-attachments | 0.000% | 0 | pass |
| callsheet-add-crew-row | 0.000% | 0 | pass |
| callsheet-export | 0.000% | 0 | pass |
| callsheet-refresh-from-schedule | 0.000% | 0 | pass |
| callsheet-delete-confirm | 0.000% | 0 | pass |
| callsheets-list-one | 0.000% | 0 | pass |
| callsheet-create-sheet-day-2 | 0.000% | 0 | pass |
| callsheet-editor-day-2 | 0.000% | 0 | pass |
| callsheets-list | 0.000% | 0 | pass |
| callsheet-row-menu | 0.000% | 0 | pass |
| callsheet-list-export | 0.000% | 0 | pass |
| callsheets-search | 0.000% | 0 | pass |
| callsheets-no-match | 0.000% | 0 | pass |
| callsheets-filter-final | 0.000% | 0 | pass |
| callsheet-finalize-sheet | 0.000% | 0 | pass |
| budget-start | 0.000% | 0 | pass |
| budget-currency-menu | 0.000% | 0 | pass |
| budget-empty-table | 0.000% | 0 | pass |
| budget-add-line-sheet | 0.000% | 0 | pass |
| budget-category-menu | 0.000% | 0 | pass |
| budget-add-line-error | 0.000% | 0 | pass |
| budget-one-line | 0.000% | 0 | pass |
| budget-add-line-filled | 0.000% | 0 | pass |
| budget-table | 0.000% | 0 | pass |
| budget-edit-plan | 0.000% | 0 | pass |
| budget-plan-currency-menu | 0.000% | 0 | pass |
| budget-with-plan | 0.000% | 0 | pass |
| budget-snapshot-name-sheet | 0.000% | 0 | pass |
| budget-snapshots | 0.000% | 0 | pass |
| budget-export | 0.000% | 0 | pass |
| plan-overview-empty | 0.000% | 0 | pass |
| plan-catalog-empty | 0.000% | 0 | pass |
| plan-catalog-add-empty | 0.000% | 0 | pass |
| plan-catalog-add-category-menu | 0.000% | 0 | pass |
| plan-locations-empty | 0.000% | 0 | pass |
| plan-cast-empty | 0.000% | 0 | pass |
| plan-crew-empty | 0.000% | 0 | pass |
| plan-moodboards-empty | 0.000% | 0 | pass |
| plan-moodboard-new-empty | 0.000% | 0 | pass |
| plan-moodboard-new-no-name | 0.000% | 0 | pass |
| plan-storyboards-empty | 0.000% | 0 | pass |
| plan-storyboard-new-no-name | 0.000% | 0 | pass |
| plan-shots-no-script | 0.000% | 0 | pass |
| plan-overview-filled | 0.000% | 0 | pass |
| plan-catalog-items | 0.000% | 0 | pass |
| plan-catalog-filter-props | 0.000% | 0 | pass |
| plan-catalog-filter-wardrobe | 0.000% | 0 | pass |
| plan-catalog-filter-cast | 0.000% | 0 | pass |
| plan-catalog-filter-locations | 0.000% | 0 | pass |
| plan-catalog-more-menu | 0.000% | 0 | pass |
| plan-catalog-search-none | 0.000% | 0 | pass |
| plan-catalog-row-menu | 0.000% | 0 | pass |
| plan-catalog-item-open | 0.000% | 0 | pass |
| plan-catalog-item-status-menu | 0.000% | 0 | pass |
| plan-catalog-item-notes | 0.000% | 0 | pass |
| plan-catalog-archive-sheet | 0.000% | 0 | pass |
| plan-catalog-delete-sheet | 0.000% | 0 | pass |
| plan-catalog-add-duplicate | 0.000% | 0 | pass |
| plan-catalog-add-filled | 0.000% | 0 | pass |
| plan-catalog-added | 0.000% | 0 | pass |
| plan-catalog-show-archived | 0.000% | 0 | pass |
| plan-location-add-empty | 0.000% | 0 | pass |
| plan-location-add-filled | 0.000% | 0 | pass |
| plan-location-open | 0.000% | 0 | pass |
| plan-location-confirmed | 0.000% | 0 | pass |
| plan-locations-four | 0.000% | 0 | pass |
| plan-locations-filter-idea | 0.000% | 0 | pass |
| plan-locations-filter-shortlisted | 0.000% | 0 | pass |
| plan-locations-filter-confirmed | 0.000% | 0 | pass |
| plan-locations-filter-rejected | 0.000% | 0 | pass |
| plan-location-card-menu | 0.000% | 0 | pass |
| plan-location-rejected-open | 0.000% | 0 | pass |
| plan-location-delete-sheet | 0.000% | 0 | pass |
| plan-locations-search-none | 0.000% | 0 | pass |
| plan-cast-needs-casting | 0.000% | 0 | pass |
| plan-cast-assign-sheet | 0.000% | 0 | pass |
| plan-cast-assign-filled | 0.000% | 0 | pass |
| plan-cast-linked-one | 0.000% | 0 | pass |
| plan-cast-add-empty | 0.000% | 0 | pass |
| plan-cast-add-character | 0.000% | 0 | pass |
| plan-cast-add-new-character | 0.000% | 0 | pass |
| plan-cast-linked | 0.000% | 0 | pass |
| plan-cast-row-menu | 0.000% | 0 | pass |
| plan-cast-open | 0.000% | 0 | pass |
| plan-crew-add-empty | 0.000% | 0 | pass |
| plan-crew-add-filled | 0.000% | 0 | pass |
| plan-crew-open | 0.000% | 0 | pass |
| plan-crew-list | 0.000% | 0 | pass |
| plan-crew-row-menu | 0.000% | 0 | pass |
| plan-crew-delete-sheet | 0.000% | 0 | pass |
| plan-overview-cast | 0.000% | 0 | pass |
| plan-moodboard-new-sheet | 0.000% | 0 | pass |
| plan-moodboard-scene-menu | 0.000% | 0 | pass |
| plan-moodboard-new-suggestion | 0.000% | 0 | pass |
| plan-moodboard-canvas-empty | 0.000% | 0 | pass |
| plan-moodboard-add-image-menu | 0.000% | 0 | pass |
| plan-moodboard-vault-sheet | 0.000% | 0 | pass |
| plan-moodboard-note-sheet | 0.000% | 0 | pass |
| plan-moodboard-link-sheet | 0.000% | 0 | pass |
| plan-moodboard-note-private | 0.000% | 0 | pass |
| plan-moodboard-canvas-items | 0.000% | 0 | pass |
| plan-moodboard-inspector | 0.000% | 0 | pass |
| plan-moodboard-two-boards | 0.000% | 0 | pass |
| plan-moodboard-row-menu | 0.000% | 0 | pass |
| plan-moodboard-rename-sheet | 0.000% | 0 | pass |
| plan-moodboard-notes-sheet | 0.000% | 0 | pass |
| plan-moodboard-delete-sheet | 0.000% | 0 | pass |
| plan-moodboard-export-sheet | 0.000% | 0 | pass |
| plan-storyboard-scene-empty | 0.000% | 0 | pass |
| plan-storyboard-scene-menu | 0.000% | 0 | pass |
| plan-storyboard-add-menu | 0.000% | 0 | pass |
| plan-storyboard-sketch-sheet | 0.000% | 0 | pass |
| plan-storyboard-panels | 0.000% | 0 | pass |
| plan-storyboard-panel-menu | 0.000% | 0 | pass |
| plan-storyboard-actions-menu | 0.000% | 0 | pass |
| plan-storyboard-rename-sheet | 0.000% | 0 | pass |
| plan-storyboard-new-sheet | 0.000% | 0 | pass |
| plan-storyboard-new-scene-menu | 0.000% | 0 | pass |
| plan-storyboard-export-sheet | 0.000% | 0 | pass |
| plan-shots-all-empty | 0.000% | 0 | pass |
| plan-shots-scene-menu | 0.000% | 0 | pass |
| plan-shots-scene-empty | 0.000% | 0 | pass |
| plan-shots-composer-error | 0.000% | 0 | pass |
| plan-shots-list | 0.000% | 0 | pass |
| plan-shots-grid | 0.000% | 0 | pass |
| plan-shots-more-menu | 0.000% | 0 | pass |
| plan-shots-copy-sheet | 0.000% | 0 | pass |
| plan-shot-row-menu | 0.000% | 0 | pass |
| plan-shot-move-sheet | 0.000% | 0 | pass |
| plan-shot-attach-sheet | 0.000% | 0 | pass |
| plan-shot-open | 0.000% | 0 | pass |
| plan-shots-export-sheet | 0.000% | 0 | pass |
| files-empty | 0.000% | 0 | pass |
| pages-files-add-menu | 0.000% | 0 | pass |
| files-new-folder-sheet | 0.000% | 0 | pass |
| pages-files-new-folder-named | 0.000% | 0 | pass |
| files-with-folder | 0.000% | 0 | pass |
| pages-files-one-file | 0.000% | 0 | pass |
| pages-files-list | 0.000% | 0 | pass |
| pages-files-grid | 0.000% | 0 | pass |
| pages-files-grid-selected | 0.000% | 0 | pass |
| pages-files-context-menu | 0.000% | 0 | pass |
| pages-files-selected | 0.000% | 0 | pass |
| pages-files-details-notes | 0.000% | 0 | pass |
| pages-files-moved | 0.000% | 0 | pass |
| pages-files-linked-details | 0.000% | 0 | pass |
| pages-files-folder-open | 0.000% | 0 | pass |
| pages-files-folder-menu | 0.000% | 0 | pass |
| pages-files-rename-folder | 0.000% | 0 | pass |
| pages-files-delete-folder | 0.000% | 0 | pass |
| pages-files-new-subfolder | 0.000% | 0 | pass |
| pages-files-folder-empty | 0.000% | 0 | pass |
| pages-files-search | 0.000% | 0 | pass |
| pages-files-search-none | 0.000% | 0 | pass |
| pages-files-unavailable | 0.000% | 0 | pass |
| pages-files-unavailable-menu | 0.000% | 0 | pass |
| pages-files-unavailable-details | 0.000% | 0 | pass |
| pages-files-deleted | 0.000% | 0 | pass |
| notes-empty | 0.000% | 0 | pass |
| notes-new-note | 0.000% | 0 | pass |
| notes-note-written | 0.000% | 0 | pass |
| pages-notes-pinned | 0.000% | 0 | pass |
| pages-notes-two | 0.000% | 0 | pass |
| tasks-new-task | 0.000% | 0 | pass |
| pages-tasks-titled | 0.000% | 0 | pass |
| pages-tasks-due | 0.000% | 0 | pass |
| pages-tasks-owner-menu | 0.000% | 0 | pass |
| pages-tasks-related-search | 0.000% | 0 | pass |
| notes-and-tasks | 0.000% | 0 | pass |
| pages-tasks-one-done | 0.000% | 0 | pass |
| pages-tasks-filter-done | 0.000% | 0 | pass |
| pages-tasks-filter-all | 0.000% | 0 | pass |
| pages-tasks-owner-filter | 0.000% | 0 | pass |
| pages-tasks-context-menu | 0.000% | 0 | pass |
| pages-tasks-done-editor | 0.000% | 0 | pass |
| pages-tasks-editor | 0.000% | 0 | pass |
| pages-notes-context-menu | 0.000% | 0 | pass |
| pages-notes-open | 0.000% | 0 | pass |
| pages-notes-search | 0.000% | 0 | pass |
| pages-notes-search-none | 0.000% | 0 | pass |
| activity | 0.000% | 0 | pass |
| pages-activity-drafts | 0.000% | 0 | pass |
| pages-activity-story | 0.000% | 0 | pass |
| pages-activity-production | 0.000% | 0 | pass |
| pages-activity-exchange | 0.000% | 0 | pass |
| recently-deleted-empty | 0.000% | 0 | pass |
| pages-deleted-global-empty | 0.000% | 0 | pass |
| pages-deleted-items | 0.000% | 0 | pass |
| pages-deleted-search | 0.000% | 0 | pass |
| pages-deleted-search-none | 0.000% | 0 | pass |
| pages-deleted-purge-confirm | 0.000% | 0 | pass |
| pages-deleted-restored | 0.000% | 0 | pass |
| pages-notes-restored | 0.000% | 0 | pass |
| pages-settings-general | 0.000% | 0 | pass |
| pages-settings-general-edited | 0.000% | 0 | pass |
| pages-settings-title-error | 0.000% | 0 | pass |
| pages-settings-saved | 0.000% | 0 | pass |
| pages-settings-status-menu | 0.000% | 0 | pass |
| pages-settings-type-menu | 0.000% | 0 | pass |
| pages-settings-writing | 0.000% | 0 | pass |
| pages-settings-shortcuts | 0.000% | 0 | pass |
| pages-settings-offline-ai | 0.000% | 0 | pass |
| pages-settings-advanced | 0.000% | 0 | pass |
| pages-settings-appearance | 0.000% | 0 | pass |
| pages-settings-writing-word | 0.000% | 0 | pass |
| pages-settings-language-menu | 0.000% | 0 | pass |
| pages-templates | 0.000% | 0 | pass |
| pages-templates-preview | 0.000% | 0 | pass |
| pages-templates-actions-menu | 0.000% | 0 | pass |
| pages-templates-new | 0.000% | 0 | pass |
| ai-closed-home | 0.000% | 0 | pass |
| ai-panel | 0.000% | 0 | pass |
| ai-settings-menu | 0.000% | 0 | pass |
| ai-panel-idea-vault | 0.000% | 0 | pass |
| ai-panel-story | 0.000% | 0 | pass |
| ai-panel-screenplay | 0.000% | 0 | pass |
| ai-panel-breakdown | 0.000% | 0 | pass |
| ai-panel-production | 0.000% | 0 | pass |
| ai-panel-schedule | 0.000% | 0 | pass |
| ai-panel-call-sheets | 0.000% | 0 | pass |
| ai-panel-files | 0.000% | 0 | pass |
| ai-closed-screenplay | 0.000% | 0 | pass |
| ai-closed-breakdown | 0.000% | 0 | pass |
| ai-panel-sidebar-hidden | 0.000% | 0 | pass |
| ai-panel-narrow-window | 0.000% | 0 | pass |
| ai-search-palette | 0.000% | 0 | pass |
| menu-file-with-project | 0.000% | 0 | pass |
| package-share-for-review-menu | 0.000% | 0 | pass |
| package-import-export-history | 0.000% | 0 | pass |
| package-review-queue | 0.000% | 0 | pass |
| package-import-exchange-package | 0.000% | 0 | pass |
| package-print | 0.000% | 0 | pass |
| package-back-up-project | 0.000% | 0 | pass |
| package-backup-ready | 0.000% | 0 | pass |
| package-backup-portable | 0.000% | 0 | pass |
| package-backup-created | 0.000% | 0 | pass |
| package-export-full-project | 0.000% | 0 | pass |
| package-export-full-project-portable | 0.000% | 0 | pass |
| package-export-full-project-done | 0.000% | 0 | pass |
| package-share-script | 0.000% | 0 | pass |
| package-share-beat-board | 0.000% | 0 | pass |
| package-share-beat-board-one-act | 0.000% | 0 | pass |
| package-share-script-choose-scenes | 0.000% | 0 | pass |
| package-share-script-one-scene | 0.000% | 0 | pass |
| package-share-script-comments | 0.000% | 0 | pass |
| package-share-script-done | 0.000% | 0 | pass |
| package-review-viewer | 0.000% | 0 | pass |
| package-review-comment-typed | 0.000% | 0 | pass |
| package-review-comment-added | 0.000% | 0 | pass |
| package-review-response-exported | 0.000% | 0 | pass |
| package-import-preview | 0.000% | 0 | pass |
| package-import-preview-expanded | 0.000% | 0 | pass |
| package-import-done | 0.000% | 0 | pass |
| package-review-queue-after-import | 0.000% | 0 | pass |
| package-history-imports | 0.000% | 0 | pass |
| package-history-review-records | 0.000% | 0 | pass |
| package-history-my-reviews | 0.000% | 0 | pass |
| package-history-package-log | 0.000% | 0 | pass |
| package-restore | 0.000% | 0 | pass |
| package-restore-damaged | 0.000% | 0 | pass |
| package-restore-collision | 0.000% | 0 | pass |
| package-restore-opened-copy | 0.000% | 0 | pass |
| home-after-crash | 0.000% | 0 | pass |
| recovery-alert | 0.000% | 0 | pass |
| pages-recovery-kept | 0.000% | 0 | pass |
| home-project-unavailable | 0.000% | 0 | pass |
| project-unavailable | 0.000% | 0 | pass |

692 of 692 pages pass (a page fails above 0.50% differing pixels, channel difference over 24, or with any HTML miss); 0 HTML misses in all.

### dark: every page passed

| Page | Differing pixels | HTML misses | Result |
|---|---|---|---|
| home-welcome | 0.000% | 0 | pass |
| home-welcome-filled | 0.000% | 0 | pass |
| home-empty | 0.000% | 0 | pass |
| menu-app | 0.000% | 0 | pass |
| menu-file-home | 0.000% | 0 | pass |
| menu-edit-home | 0.000% | 0 | pass |
| menu-view-home | 0.000% | 0 | pass |
| menu-go-home | 0.000% | 0 | pass |
| menu-window-home | 0.000% | 0 | pass |
| menu-help-home | 0.000% | 0 | pass |
| new-project-sheet | 0.000% | 0 | pass |
| new-project-filled | 0.000% | 0 | pass |
| new-project-type-menu | 0.000% | 0 | pass |
| new-project-short-film | 0.000% | 0 | pass |
| search-palette-home | 0.000% | 0 | pass |
| search-palette-home-commands | 0.000% | 0 | pass |
| search-palette-empty | 0.000% | 0 | pass |
| home-shortcuts-sheet | 0.000% | 0 | pass |
| about | 0.000% | 0 | pass |
| settings-general | 0.000% | 0 | pass |
| settings-general-name-edited | 0.000% | 0 | pass |
| settings-appearance | 0.000% | 0 | pass |
| settings-appearance-scrolled | 0.000% | 0 | pass |
| settings-appearance-theme-chosen | 0.000% | 0 | pass |
| settings-theme-paste-sheet | 0.000% | 0 | pass |
| settings-theme-paste-error | 0.000% | 0 | pass |
| settings-keyboard-shortcuts | 0.000% | 0 | pass |
| settings-keyboard-shortcuts-scrolled | 0.000% | 0 | pass |
| settings-shortcuts-filtered | 0.000% | 0 | pass |
| settings-shortcuts-none | 0.000% | 0 | pass |
| settings-shortcut-recording | 0.000% | 0 | pass |
| settings-offline-ai | 0.000% | 0 | pass |
| settings-diagnostics | 0.000% | 0 | pass |
| home-import-toast | 0.000% | 0 | pass |
| project-home | 0.000% | 0 | pass |
| project-menu | 0.000% | 0 | pass |
| project-new-menu | 0.000% | 0 | pass |
| project-home-status-menu | 0.000% | 0 | pass |
| menu-app-project | 0.000% | 0 | pass |
| menu-file-project | 0.000% | 0 | pass |
| menu-edit-project | 0.000% | 0 | pass |
| menu-view-project | 0.000% | 0 | pass |
| menu-go | 0.000% | 0 | pass |
| menu-window-project | 0.000% | 0 | pass |
| menu-help-project | 0.000% | 0 | pass |
| project-rename-sheet | 0.000% | 0 | pass |
| project-rename-empty | 0.000% | 0 | pass |
| project-home-sidebar-hidden | 0.000% | 0 | pass |
| project-home-after-back | 0.000% | 0 | pass |
| project-home-ai-panel | 0.000% | 0 | pass |
| search-palette-project | 0.000% | 0 | pass |
| search-palette-results | 0.000% | 0 | pass |
| project-shortcuts-sheet | 0.000% | 0 | pass |
| home-filled | 0.000% | 0 | pass |
| home-card-menu | 0.000% | 0 | pass |
| home-global-vault | 0.000% | 0 | pass |
| home-rename-sheet | 0.000% | 0 | pass |
| home-delete-sheet | 0.000% | 0 | pass |
| home-delete-typed | 0.000% | 0 | pass |
| project-home-empty | 0.000% | 0 | pass |
| home-two-projects | 0.000% | 0 | pass |
| home-search-results | 0.010% | 0 | pass (differences between 714,121 and 727,134 points) |
| home-search-none | 0.010% | 0 | pass (differences between 714,121 and 727,134 points) |
| home-pinned | 0.000% | 0 | pass |
| home-one-archived | 0.000% | 0 | pass |
| home-archived-page | 0.000% | 0 | pass |
| home-archived-empty | 0.000% | 0 | pass |
| screenplay-editor-typing | 0.000% | 0 | pass |
| screenplay-tools-menu | 0.000% | 0 | pass |
| screenplay-tools-add-comment | 0.000% | 0 | pass |
| screenplay-tools-scene-hub | 0.000% | 0 | pass |
| screenplay-tools-drafts | 0.000% | 0 | pass |
| screenplay-tools-lock-as-shooting-draft | 0.000% | 0 | pass |
| screenplay-tools-page-setup | 0.000% | 0 | pass |
| screenplay-tools-title-page | 0.000% | 0 | pass |
| screenplay-tools-custom-elements | 0.000% | 0 | pass |
| screenplay-tools-page-view | 0.000% | 0 | pass |
| screenplay-tools-import-screenplay | 0.000% | 0 | pass |
| screenplay-tools-export-screenplay | 0.000% | 0 | pass |
| screenplay-draft-menu | 0.000% | 0 | pass |
| screenplay-draft-new-draft | 0.000% | 0 | pass |
| screenplay-draft-manage-drafts | 0.000% | 0 | pass |
| screenplay-element-picker | 0.000% | 0 | pass |
| screenplay-element-picker-dialogue | 0.000% | 0 | pass |
| screenplay-comments | 0.000% | 0 | pass |
| screenplay-compare-drafts | 0.000% | 0 | pass |
| screenplay-find-and-replace | 0.000% | 0 | pass |
| screenplay-picker-menu | 0.000% | 0 | pass |
| screenplay-new-screenplay-sheet | 0.000% | 0 | pass |
| screenplay-new-screenplay-duplicate | 0.000% | 0 | pass |
| screenplay-rename-screenplay-sheet | 0.000% | 0 | pass |
| screenplay-delete-screenplay-sheet | 0.000% | 0 | pass |
| screenplay-scenes-hidden | 0.000% | 0 | pass |
| screenplay-navigator-columns-menu | 0.000% | 0 | pass |
| screenplay-navigator-columns-shown | 0.000% | 0 | pass |
| screenplay-scene-row-context | 0.000% | 0 | pass |
| screenplay-editor-context | 0.000% | 0 | pass |
| screenplay-editor-selection | 0.000% | 0 | pass |
| screenplay-editor-context-selection | 0.000% | 0 | pass |
| screenplay-editor-context-spelling | 0.000% | 0 | pass |
| screenplay-zoom-50 | 0.000% | 0 | pass |
| screenplay-zoom-150 | 0.000% | 0 | pass |
| screenplay-zoom-200 | 0.000% | 0 | pass |
| screenplay-layout-focus | 0.000% | 0 | pass |
| screenplay-empty-editor | 0.000% | 0 | pass |
| screenplay-empty-element-picker | 0.000% | 0 | pass |
| screenplay-empty-draft-menu | 0.000% | 0 | pass |
| screenplay-empty-comments | 0.000% | 0 | pass |
| screenplay-empty-scene-hub | 0.000% | 0 | pass |
| screenplay-empty-draft-info | 0.000% | 0 | pass |
| screenplay-empty-drafts | 0.000% | 0 | pass |
| screenplay-empty-page-view | 0.000% | 0 | pass |
| screenplay-empty-export | 0.000% | 0 | pass |
| screenplay-empty-suggestions | 0.000% | 0 | pass |
| screenplay-empty-first-scene | 0.000% | 0 | pass |
| screenplay-find-bar | 0.011% | 0 | pass (differences between 1095,123 and 1108,134 points) |
| screenplay-find-results | 0.011% | 0 | pass (differences between 1095,123 and 1108,134 points) |
| screenplay-replace-filled | 0.000% | 0 | pass |
| screenplay-replace-whole-script | 0.000% | 0 | pass |
| screenplay-replace-done | 0.000% | 0 | pass |
| screenplay-replace-no-match | 0.000% | 0 | pass |
| screenplay-scene-hub-details-edited | 0.000% | 0 | pass |
| screenplay-scene-hub-from-list | 0.000% | 0 | pass |
| screenplay-comment-composer-quote | 0.000% | 0 | pass |
| screenplay-comment-posted | 0.000% | 0 | pass |
| screenplay-comment-reply-composer | 0.000% | 0 | pass |
| screenplay-comment-with-reply | 0.000% | 0 | pass |
| screenplay-comment-composer-draft | 0.000% | 0 | pass |
| screenplay-comments-two-open | 0.000% | 0 | pass |
| screenplay-comments-after-resolve | 0.000% | 0 | pass |
| screenplay-comments-resolved-tab | 0.000% | 0 | pass |
| screenplay-comment-actions | 0.000% | 0 | pass |
| screenplay-comment-in-discussion | 0.000% | 0 | pass |
| screenplay-comment-composer-scene | 0.000% | 0 | pass |
| screenplay-new-draft-named | 0.000% | 0 | pass |
| screenplay-second-draft-created | 0.000% | 0 | pass |
| screenplay-second-draft-edited | 0.000% | 0 | pass |
| screenplay-draft-menu-two | 0.000% | 0 | pass |
| screenplay-drafts-two | 0.000% | 0 | pass |
| screenplay-draft-actions-current | 0.000% | 0 | pass |
| screenplay-draft-actions-other | 0.000% | 0 | pass |
| screenplay-draft-rename | 0.000% | 0 | pass |
| screenplay-draft-delete | 0.000% | 0 | pass |
| screenplay-draft-new-from | 0.000% | 0 | pass |
| screenplay-compare-text-only | 0.000% | 0 | pass |
| screenplay-compare-next-change | 0.000% | 0 | pass |
| screenplay-compare-scene-only | 0.000% | 0 | pass |
| screenplay-compare-inline | 0.000% | 0 | pass |
| screenplay-compare-before-menu | 0.000% | 0 | pass |
| screenplay-compare-same-draft | 0.000% | 0 | pass |
| screenplay-locked | 0.000% | 0 | pass |
| screenplay-tools-menu-locked | 0.000% | 0 | pass |
| screenplay-revisions-locked | 0.000% | 0 | pass |
| screenplay-start-revision | 0.000% | 0 | pass |
| screenplay-revision-started | 0.000% | 0 | pass |
| screenplay-revision-edited | 0.000% | 0 | pass |
| screenplay-revisions-changed | 0.000% | 0 | pass |
| screenplay-page-view-revision | 0.000% | 0 | pass |
| screenplay-page-view-marks-toggled | 0.000% | 0 | pass |
| screenplay-drafts-locked-and-revision | 0.000% | 0 | pass |
| screenplay-revision-label-colour | 0.000% | 0 | pass |
| screenplay-inspector-draft-revision | 0.000% | 0 | pass |
| screenplay-title-page-filled | 0.000% | 0 | pass |
| screenplay-title-page-saved | 0.000% | 0 | pass |
| screenplay-page-view-a4 | 0.000% | 0 | pass |
| screenplay-export-fdx | 0.000% | 0 | pass |
| screenplay-export-fountain | 0.000% | 0 | pass |
| screenplay-export-docx | 0.000% | 0 | pass |
| screenplay-export-txt | 0.000% | 0 | pass |
| screenplay-export-selected-scenes | 0.000% | 0 | pass |
| screenplay-export-options | 0.000% | 0 | pass |
| screenplay-print-preview | 0.000% | 0 | pass |
| screenplay-print-preview-page-2 | 0.000% | 0 | pass |
| screenplay-print-preview-zoomed | 0.000% | 0 | pass |
| screenplay-print-preview-no-title | 0.000% | 0 | pass |
| screenplay-print-from-menu | 0.000% | 0 | pass |
| story-board-empty | 0.000% | 0 | pass |
| story-add-menu | 0.000% | 0 | pass |
| story-timeline-empty | 0.000% | 0 | pass |
| story-page-empty | 0.000% | 0 | pass |
| story-page-written | 0.000% | 0 | pass |
| story-page-export-sheet | 0.000% | 0 | pass |
| story-page-export-docx | 0.000% | 0 | pass |
| story-add-act-inline | 0.000% | 0 | pass |
| story-board-one-act | 0.000% | 0 | pass |
| story-add-sequence-inline | 0.000% | 0 | pass |
| story-sequence-empty | 0.000% | 0 | pass |
| story-add-card-inline | 0.000% | 0 | pass |
| story-add-beat-inline | 0.000% | 0 | pass |
| story-board-filled | 0.000% | 0 | pass |
| story-card-selected | 0.000% | 0 | pass |
| story-multi-select | 0.000% | 0 | pass |
| story-move-sheet | 0.000% | 0 | pass |
| story-card-context-menu | 0.000% | 0 | pass |
| story-beat-context-menu | 0.000% | 0 | pass |
| story-sequence-context-menu | 0.000% | 0 | pass |
| story-parked-card-context-menu | 0.000% | 0 | pass |
| story-act-context-menu | 0.000% | 0 | pass |
| story-card-inspector | 0.000% | 0 | pass |
| story-card-inspector-filled | 0.000% | 0 | pass |
| story-board-card-details | 0.000% | 0 | pass |
| story-beat-inspector | 0.000% | 0 | pass |
| story-sequence-inspector | 0.000% | 0 | pass |
| story-parked-card-inspector | 0.000% | 0 | pass |
| story-act-rename-inline | 0.000% | 0 | pass |
| story-act-note-sheet | 0.000% | 0 | pass |
| story-delete-act-sheet | 0.000% | 0 | pass |
| story-move-destination-menu | 0.000% | 0 | pass |
| story-build-sheet-one-card | 0.000% | 0 | pass |
| story-deleted-notice | 0.000% | 0 | pass |
| story-act-collapsed | 0.000% | 0 | pass |
| story-board-filtered | 0.000% | 0 | pass |
| story-board-filter-no-match | 0.000% | 0 | pass |
| story-board-parking-hidden | 0.000% | 0 | pass |
| story-more-menu | 0.000% | 0 | pass |
| story-card-display-sheet | 0.000% | 0 | pass |
| story-card-display-production | 0.000% | 0 | pass |
| story-board-production-cards | 0.000% | 0 | pass |
| story-apply-order-sheet | 0.000% | 0 | pass |
| story-build-sheet | 0.000% | 0 | pass |
| story-outline | 0.000% | 0 | pass |
| story-outline-act-menu | 0.000% | 0 | pass |
| story-outline-row-context-menu | 0.000% | 0 | pass |
| story-outline-row-selected | 0.000% | 0 | pass |
| story-outline-inspector | 0.000% | 0 | pass |
| story-outline-act-collapsed | 0.000% | 0 | pass |
| story-outline-filtered | 0.000% | 0 | pass |
| story-beat-converted | 0.000% | 0 | pass |
| story-board-unassigned | 0.000% | 0 | pass |
| story-characters-empty | 0.000% | 0 | pass |
| story-add-character-sheet | 0.000% | 0 | pass |
| story-add-character-filled | 0.000% | 0 | pass |
| story-character-one | 0.000% | 0 | pass |
| story-add-character-match | 0.000% | 0 | pass |
| story-characters | 0.000% | 0 | pass |
| story-character-edit-sheet | 0.000% | 0 | pass |
| story-character-more-menu | 0.000% | 0 | pass |
| story-relationship-map-empty | 0.000% | 0 | pass |
| story-add-relationship-sheet | 0.000% | 0 | pass |
| story-relationship-suggestions | 0.000% | 0 | pass |
| story-add-relationship-filled | 0.000% | 0 | pass |
| story-character-relationships | 0.000% | 0 | pass |
| story-relationship-map | 0.000% | 0 | pass |
| story-character-linked-card | 0.000% | 0 | pass |
| story-characters-archived-hidden | 0.000% | 0 | pass |
| vault-empty | 0.000% | 0 | pass |
| vault-recently-deleted-empty | 0.000% | 0 | pass |
| vault-add-menu | 0.000% | 0 | pass |
| vault-add-note | 0.000% | 0 | pass |
| vault-add-url | 0.000% | 0 | pass |
| vault-add-audio-voice-note | 0.000% | 0 | pass |
| vault-add-quote | 0.000% | 0 | pass |
| vault-add-quick-capture | 0.000% | 0 | pass |
| vault-quick-capture-typed | 0.000% | 0 | pass |
| vault-saved-notice | 0.000% | 0 | pass |
| vault-add-url-error | 0.000% | 0 | pass |
| vault-add-url-filled | 0.000% | 0 | pass |
| vault-drawer-quote | 0.000% | 0 | pass |
| vault-filled | 0.000% | 0 | pass |
| vault-view-card | 0.000% | 0 | pass |
| vault-view-list | 0.000% | 0 | pass |
| vault-view-folder | 0.000% | 0 | pass |
| vault-drawer-note | 0.000% | 0 | pass |
| vault-drawer-flag-menu | 0.000% | 0 | pass |
| vault-drawer-flag-and-tag | 0.000% | 0 | pass |
| vault-move-sheet | 0.000% | 0 | pass |
| vault-copy-to-global-sheet | 0.000% | 0 | pass |
| vault-send-to-story-sheet | 0.000% | 0 | pass |
| vault-drawer-url | 0.000% | 0 | pass |
| vault-item-selected | 0.000% | 0 | pass |
| vault-tag-sheet | 0.000% | 0 | pass |
| vault-card-context-menu | 0.000% | 0 | pass |
| vault-card-flag-submenu | 0.000% | 0 | pass |
| vault-pinned-empty | 0.000% | 0 | pass |
| vault-pinned-notice | 0.000% | 0 | pass |
| vault-pinned | 0.000% | 0 | pass |
| vault-tag-sheet-filled | 0.000% | 0 | pass |
| vault-tag-view | 0.000% | 0 | pass |
| vault-new-folder-sheet | 0.000% | 0 | pass |
| vault-new-folder-error | 0.000% | 0 | pass |
| vault-folder-empty | 0.000% | 0 | pass |
| vault-move-sheet-chosen | 0.000% | 0 | pass |
| vault-folder-with-item | 0.000% | 0 | pass |
| vault-folder-context-menu | 0.000% | 0 | pass |
| vault-rename-folder-sheet | 0.000% | 0 | pass |
| vault-new-subfolder-sheet | 0.000% | 0 | pass |
| vault-folder-mode-top | 0.000% | 0 | pass |
| vault-folder-mode-inside | 0.000% | 0 | pass |
| vault-recently-added | 0.000% | 0 | pass |
| vault-search-results | 0.015% | 0 | pass (differences between 1260,19 and 1272,32 points) |
| vault-search-none | 0.015% | 0 | pass (differences between 1260,19 and 1272,32 points) |
| vault-new-collection-sheet | 0.000% | 0 | pass |
| vault-collection-empty | 0.000% | 0 | pass |
| vault-collection-with-item | 0.000% | 0 | pass |
| vault-add-to-collection-sheet | 0.000% | 0 | pass |
| vault-collection-context-menu | 0.000% | 0 | pass |
| vault-rename-collection-sheet | 0.000% | 0 | pass |
| vault-deleted-notice | 0.000% | 0 | pass |
| vault-recently-deleted | 0.000% | 0 | pass |
| vault-purge-confirm | 0.000% | 0 | pass |
| vault-restored | 0.000% | 0 | pass |
| vault-global-empty | 0.000% | 0 | pass |
| vault-global-with-copy | 0.000% | 0 | pass |
| vault-global-drawer | 0.000% | 0 | pass |
| vault-global-card-context-menu | 0.000% | 0 | pass |
| breakdown-no-source | 0.000% | 0 | pass |
| breakdown-source-sheet | 0.000% | 0 | pass |
| breakdown-source-sheet-reason | 0.000% | 0 | pass |
| breakdown-scenes | 0.000% | 0 | pass |
| breakdown-scene-3 | 0.000% | 0 | pass |
| breakdown-scenes-hidden | 0.000% | 0 | pass |
| breakdown-script-only | 0.000% | 0 | pass |
| breakdown-panel-hidden | 0.000% | 0 | pass |
| breakdown-filter-menu | 0.000% | 0 | pass |
| breakdown-filter-none | 0.000% | 0 | pass |
| breakdown-filter-missing | 0.000% | 0 | pass |
| breakdown-change-source | 0.000% | 0 | pass |
| breakdown-export-sheet | 0.000% | 0 | pass |
| breakdown-export-csv | 0.000% | 0 | pass |
| breakdown-export-xlsx | 0.000% | 0 | pass |
| breakdown-export-options | 0.000% | 0 | pass |
| breakdown-script-selection | 0.000% | 0 | pass |
| breakdown-script-context-menu | 0.000% | 0 | pass |
| breakdown-tag-menu | 0.000% | 0 | pass |
| breakdown-tag-sheet | 0.000% | 0 | pass |
| breakdown-tagged | 0.000% | 0 | pass |
| breakdown-suggestions | 0.000% | 0 | pass |
| breakdown-review-sheet | 0.000% | 0 | pass |
| breakdown-review-deselected | 0.000% | 0 | pass |
| breakdown-edit-suggestion | 0.000% | 0 | pass |
| breakdown-accepted-one | 0.000% | 0 | pass |
| breakdown-rejected-one | 0.000% | 0 | pass |
| breakdown-all-accepted | 0.000% | 0 | pass |
| breakdown-category-collapsed | 0.000% | 0 | pass |
| breakdown-element-selected | 0.000% | 0 | pass |
| breakdown-element-actions-menu | 0.000% | 0 | pass |
| breakdown-notes-sheet | 0.000% | 0 | pass |
| breakdown-notes-filled | 0.000% | 0 | pass |
| breakdown-element-notes | 0.000% | 0 | pass |
| breakdown-scene-complete | 0.000% | 0 | pass |
| breakdown-add-element-sheet | 0.000% | 0 | pass |
| breakdown-add-category-menu | 0.000% | 0 | pass |
| breakdown-add-element-filled | 0.000% | 0 | pass |
| breakdown-element-added | 0.000% | 0 | pass |
| breakdown-add-element-existing | 0.000% | 0 | pass |
| breakdown-element-removed | 0.000% | 0 | pass |
| breakdown-element-restored | 0.000% | 0 | pass |
| breakdown-scene-2 | 0.000% | 0 | pass |
| breakdown-reused | 0.000% | 0 | pass |
| breakdown-catalog-filter | 0.000% | 0 | pass |
| breakdown-filter-cast | 0.000% | 0 | pass |
| breakdown-empty-no-source | 0.000% | 0 | pass |
| breakdown-empty-source-sheet | 0.000% | 0 | pass |
| schedule-dood-empty | 0.000% | 0 | pass |
| schedule-daily-empty | 0.000% | 0 | pass |
| schedule-sides-empty | 0.000% | 0 | pass |
| schedule-empty | 0.000% | 0 | pass |
| schedule-create-sheet | 0.000% | 0 | pass |
| schedule-board-unscheduled | 0.000% | 0 | pass |
| schedule-add-day-sheet | 0.000% | 0 | pass |
| schedule-add-day-error | 0.000% | 0 | pass |
| schedule-board-three-days | 0.000% | 0 | pass |
| schedule-add-off-day-sheet | 0.000% | 0 | pass |
| schedule-board-off-day | 0.000% | 0 | pass |
| schedule-strip-menu | 0.000% | 0 | pass |
| schedule-board | 0.000% | 0 | pass |
| schedule-strip-drawer | 0.000% | 0 | pass |
| schedule-day-menu | 0.000% | 0 | pass |
| schedule-meal-break-sheet | 0.000% | 0 | pass |
| schedule-board-meal-break | 0.000% | 0 | pass |
| schedule-break-menu | 0.000% | 0 | pass |
| schedule-edit-break-sheet | 0.000% | 0 | pass |
| schedule-filter-menu | 0.000% | 0 | pass |
| schedule-board-filtered | 0.000% | 0 | pass |
| schedule-day-detail | 0.000% | 0 | pass |
| schedule-day-detail-times | 0.000% | 0 | pass |
| schedule-day-detail-menu | 0.000% | 0 | pass |
| schedule-day-add-menu | 0.000% | 0 | pass |
| schedule-day-add-meal-break | 0.000% | 0 | pass |
| schedule-day-add-travel | 0.000% | 0 | pass |
| schedule-day-add-company-move | 0.000% | 0 | pass |
| schedule-day-add-custom-note | 0.000% | 0 | pass |
| schedule-day-edit-date | 0.000% | 0 | pass |
| schedule-day-export | 0.000% | 0 | pass |
| schedule-day-empty | 0.000% | 0 | pass |
| schedule-list | 0.000% | 0 | pass |
| schedule-calendar | 0.000% | 0 | pass |
| schedule-suggest-grouping | 0.000% | 0 | pass |
| schedule-warnings | 0.000% | 0 | pass |
| schedule-schedule-settings | 0.000% | 0 | pass |
| schedule-export-schedule | 0.000% | 0 | pass |
| schedule-settings-error | 0.000% | 0 | pass |
| schedule-dood | 0.000% | 0 | pass |
| schedule-dood-code-style-menu | 0.000% | 0 | pass |
| schedule-dood-export | 0.000% | 0 | pass |
| schedule-daily | 0.000% | 0 | pass |
| schedule-daily-day-menu | 0.000% | 0 | pass |
| schedule-daily-export | 0.000% | 0 | pass |
| schedule-daily-day-2 | 0.000% | 0 | pass |
| schedule-daily-day-3 | 0.000% | 0 | pass |
| schedule-sides | 0.000% | 0 | pass |
| schedule-sides-day-menu | 0.000% | 0 | pass |
| schedule-sides-selected-cast | 0.000% | 0 | pass |
| schedule-sides-cover | 0.000% | 0 | pass |
| schedule-sides-export | 0.000% | 0 | pass |
| schedule-sides-saved | 0.000% | 0 | pass |
| schedule-sides-snapshot | 0.000% | 0 | pass |
| schedule-reports-scene | 0.000% | 0 | pass |
| schedule-reports-location | 0.000% | 0 | pass |
| schedule-reports-cast-scene | 0.000% | 0 | pass |
| schedule-reports-prop | 0.000% | 0 | pass |
| schedule-reports-schedule | 0.000% | 0 | pass |
| schedule-reports-breakdown-completeness | 0.000% | 0 | pass |
| schedule-reports-filtered | 0.000% | 0 | pass |
| schedule-reports-export | 0.000% | 0 | pass |
| callsheets-empty | 0.000% | 0 | pass |
| callsheet-create-sheet | 0.000% | 0 | pass |
| callsheet-create-day-menu | 0.000% | 0 | pass |
| callsheet-create-template-menu | 0.000% | 0 | pass |
| callsheet-editor | 0.000% | 0 | pass |
| callsheet-section-menu | 0.000% | 0 | pass |
| callsheet-section-weather | 0.000% | 0 | pass |
| callsheet-section-extras | 0.000% | 0 | pass |
| callsheet-section-crew | 0.000% | 0 | pass |
| callsheet-section-walkie-channels-and-contacts | 0.000% | 0 | pass |
| callsheet-section-attachments | 0.000% | 0 | pass |
| callsheet-add-crew-row | 0.000% | 0 | pass |
| callsheet-export | 0.000% | 0 | pass |
| callsheet-refresh-from-schedule | 0.000% | 0 | pass |
| callsheet-delete-confirm | 0.000% | 0 | pass |
| callsheets-list-one | 0.000% | 0 | pass |
| callsheet-create-sheet-day-2 | 0.000% | 0 | pass |
| callsheet-editor-day-2 | 0.000% | 0 | pass |
| callsheets-list | 0.000% | 0 | pass |
| callsheet-row-menu | 0.000% | 0 | pass |
| callsheet-list-export | 0.000% | 0 | pass |
| callsheets-search | 0.000% | 0 | pass |
| callsheets-no-match | 0.000% | 0 | pass |
| callsheets-filter-final | 0.000% | 0 | pass |
| callsheet-finalize-sheet | 0.000% | 0 | pass |
| budget-start | 0.000% | 0 | pass |
| budget-currency-menu | 0.000% | 0 | pass |
| budget-empty-table | 0.000% | 0 | pass |
| budget-add-line-sheet | 0.000% | 0 | pass |
| budget-category-menu | 0.000% | 0 | pass |
| budget-add-line-error | 0.000% | 0 | pass |
| budget-one-line | 0.000% | 0 | pass |
| budget-add-line-filled | 0.000% | 0 | pass |
| budget-table | 0.000% | 0 | pass |
| budget-edit-plan | 0.000% | 0 | pass |
| budget-plan-currency-menu | 0.000% | 0 | pass |
| budget-with-plan | 0.000% | 0 | pass |
| budget-snapshot-name-sheet | 0.000% | 0 | pass |
| budget-snapshots | 0.000% | 0 | pass |
| budget-export | 0.000% | 0 | pass |
| plan-overview-empty | 0.000% | 0 | pass |
| plan-catalog-empty | 0.000% | 0 | pass |
| plan-catalog-add-empty | 0.000% | 0 | pass |
| plan-catalog-add-category-menu | 0.000% | 0 | pass |
| plan-locations-empty | 0.000% | 0 | pass |
| plan-cast-empty | 0.000% | 0 | pass |
| plan-crew-empty | 0.000% | 0 | pass |
| plan-moodboards-empty | 0.000% | 0 | pass |
| plan-moodboard-new-empty | 0.000% | 0 | pass |
| plan-moodboard-new-no-name | 0.000% | 0 | pass |
| plan-storyboards-empty | 0.000% | 0 | pass |
| plan-storyboard-new-no-name | 0.000% | 0 | pass |
| plan-shots-no-script | 0.000% | 0 | pass |
| plan-overview-filled | 0.000% | 0 | pass |
| plan-catalog-items | 0.000% | 0 | pass |
| plan-catalog-filter-props | 0.000% | 0 | pass |
| plan-catalog-filter-wardrobe | 0.000% | 0 | pass |
| plan-catalog-filter-cast | 0.000% | 0 | pass |
| plan-catalog-filter-locations | 0.000% | 0 | pass |
| plan-catalog-more-menu | 0.320% | 0 | pass (differences between 29,64 and 1242,533 points) |
| plan-catalog-search-none | 0.000% | 0 | pass |
| plan-catalog-row-menu | 0.000% | 0 | pass |
| plan-catalog-item-open | 0.000% | 0 | pass |
| plan-catalog-item-status-menu | 0.000% | 0 | pass |
| plan-catalog-item-notes | 0.000% | 0 | pass |
| plan-catalog-archive-sheet | 0.000% | 0 | pass |
| plan-catalog-delete-sheet | 0.000% | 0 | pass |
| plan-catalog-add-duplicate | 0.000% | 0 | pass |
| plan-catalog-add-filled | 0.000% | 0 | pass |
| plan-catalog-added | 0.000% | 0 | pass |
| plan-catalog-show-archived | 0.000% | 0 | pass |
| plan-location-add-empty | 0.000% | 0 | pass |
| plan-location-add-filled | 0.000% | 0 | pass |
| plan-location-open | 0.000% | 0 | pass |
| plan-location-confirmed | 0.000% | 0 | pass |
| plan-locations-four | 0.000% | 0 | pass |
| plan-locations-filter-idea | 0.000% | 0 | pass |
| plan-locations-filter-shortlisted | 0.000% | 0 | pass |
| plan-locations-filter-confirmed | 0.000% | 0 | pass |
| plan-locations-filter-rejected | 0.000% | 0 | pass |
| plan-location-card-menu | 0.000% | 0 | pass |
| plan-location-rejected-open | 0.000% | 0 | pass |
| plan-location-delete-sheet | 0.000% | 0 | pass |
| plan-locations-search-none | 0.000% | 0 | pass |
| plan-cast-needs-casting | 0.000% | 0 | pass |
| plan-cast-assign-sheet | 0.000% | 0 | pass |
| plan-cast-assign-filled | 0.000% | 0 | pass |
| plan-cast-linked-one | 0.000% | 0 | pass |
| plan-cast-add-empty | 0.000% | 0 | pass |
| plan-cast-add-character | 0.000% | 0 | pass |
| plan-cast-add-new-character | 0.000% | 0 | pass |
| plan-cast-linked | 0.000% | 0 | pass |
| plan-cast-row-menu | 0.000% | 0 | pass |
| plan-cast-open | 0.000% | 0 | pass |
| plan-crew-add-empty | 0.000% | 0 | pass |
| plan-crew-add-filled | 0.000% | 0 | pass |
| plan-crew-open | 0.000% | 0 | pass |
| plan-crew-list | 0.000% | 0 | pass |
| plan-crew-row-menu | 0.000% | 0 | pass |
| plan-crew-delete-sheet | 0.000% | 0 | pass |
| plan-overview-cast | 0.000% | 0 | pass |
| plan-moodboard-new-sheet | 0.000% | 0 | pass |
| plan-moodboard-scene-menu | 0.000% | 0 | pass |
| plan-moodboard-new-suggestion | 0.000% | 0 | pass |
| plan-moodboard-canvas-empty | 0.000% | 0 | pass |
| plan-moodboard-add-image-menu | 0.000% | 0 | pass |
| plan-moodboard-vault-sheet | 0.000% | 0 | pass |
| plan-moodboard-note-sheet | 0.000% | 0 | pass |
| plan-moodboard-link-sheet | 0.000% | 0 | pass |
| plan-moodboard-note-private | 0.000% | 0 | pass |
| plan-moodboard-canvas-items | 0.000% | 0 | pass |
| plan-moodboard-inspector | 0.000% | 0 | pass |
| plan-moodboard-two-boards | 0.000% | 0 | pass |
| plan-moodboard-row-menu | 0.000% | 0 | pass |
| plan-moodboard-rename-sheet | 0.000% | 0 | pass |
| plan-moodboard-notes-sheet | 0.000% | 0 | pass |
| plan-moodboard-delete-sheet | 0.000% | 0 | pass |
| plan-moodboard-export-sheet | 0.000% | 0 | pass |
| plan-storyboard-scene-empty | 0.000% | 0 | pass |
| plan-storyboard-scene-menu | 0.000% | 0 | pass |
| plan-storyboard-add-menu | 0.000% | 0 | pass |
| plan-storyboard-sketch-sheet | 0.000% | 0 | pass |
| plan-storyboard-panels | 0.000% | 0 | pass |
| plan-storyboard-panel-menu | 0.000% | 0 | pass |
| plan-storyboard-actions-menu | 0.000% | 0 | pass |
| plan-storyboard-rename-sheet | 0.000% | 0 | pass |
| plan-storyboard-new-sheet | 0.000% | 0 | pass |
| plan-storyboard-new-scene-menu | 0.000% | 0 | pass |
| plan-storyboard-export-sheet | 0.000% | 0 | pass |
| plan-shots-all-empty | 0.000% | 0 | pass |
| plan-shots-scene-menu | 0.000% | 0 | pass |
| plan-shots-scene-empty | 0.000% | 0 | pass |
| plan-shots-composer-error | 0.000% | 0 | pass |
| plan-shots-list | 0.000% | 0 | pass |
| plan-shots-grid | 0.000% | 0 | pass |
| plan-shots-more-menu | 0.000% | 0 | pass |
| plan-shots-copy-sheet | 0.000% | 0 | pass |
| plan-shot-row-menu | 0.000% | 0 | pass |
| plan-shot-move-sheet | 0.000% | 0 | pass |
| plan-shot-attach-sheet | 0.000% | 0 | pass |
| plan-shot-open | 0.000% | 0 | pass |
| plan-shots-export-sheet | 0.000% | 0 | pass |
| files-empty | 0.000% | 0 | pass |
| pages-files-add-menu | 0.000% | 0 | pass |
| files-new-folder-sheet | 0.000% | 0 | pass |
| pages-files-new-folder-named | 0.000% | 0 | pass |
| files-with-folder | 0.000% | 0 | pass |
| pages-files-one-file | 0.000% | 0 | pass |
| pages-files-list | 0.000% | 0 | pass |
| pages-files-grid | 0.000% | 0 | pass |
| pages-files-grid-selected | 0.000% | 0 | pass |
| pages-files-context-menu | 0.000% | 0 | pass |
| pages-files-selected | 0.000% | 0 | pass |
| pages-files-details-notes | 0.000% | 0 | pass |
| pages-files-moved | 0.000% | 0 | pass |
| pages-files-linked-details | 0.000% | 0 | pass |
| pages-files-folder-open | 0.000% | 0 | pass |
| pages-files-folder-menu | 0.000% | 0 | pass |
| pages-files-rename-folder | 0.000% | 0 | pass |
| pages-files-delete-folder | 0.000% | 0 | pass |
| pages-files-new-subfolder | 0.000% | 0 | pass |
| pages-files-folder-empty | 0.000% | 0 | pass |
| pages-files-search | 0.000% | 0 | pass |
| pages-files-search-none | 0.000% | 0 | pass |
| pages-files-unavailable | 0.000% | 0 | pass |
| pages-files-unavailable-menu | 0.000% | 0 | pass |
| pages-files-unavailable-details | 0.000% | 0 | pass |
| pages-files-deleted | 0.000% | 0 | pass |
| notes-empty | 0.000% | 0 | pass |
| notes-new-note | 0.000% | 0 | pass |
| notes-note-written | 0.000% | 0 | pass |
| pages-notes-pinned | 0.000% | 0 | pass |
| pages-notes-two | 0.000% | 0 | pass |
| tasks-new-task | 0.000% | 0 | pass |
| pages-tasks-titled | 0.000% | 0 | pass |
| pages-tasks-due | 0.000% | 0 | pass |
| pages-tasks-owner-menu | 0.000% | 0 | pass |
| pages-tasks-related-search | 0.000% | 0 | pass |
| notes-and-tasks | 0.000% | 0 | pass |
| pages-tasks-one-done | 0.000% | 0 | pass |
| pages-tasks-filter-done | 0.000% | 0 | pass |
| pages-tasks-filter-all | 0.000% | 0 | pass |
| pages-tasks-owner-filter | 0.000% | 0 | pass |
| pages-tasks-context-menu | 0.000% | 0 | pass |
| pages-tasks-done-editor | 0.000% | 0 | pass |
| pages-tasks-editor | 0.000% | 0 | pass |
| pages-notes-context-menu | 0.000% | 0 | pass |
| pages-notes-open | 0.000% | 0 | pass |
| pages-notes-search | 0.000% | 0 | pass |
| pages-notes-search-none | 0.000% | 0 | pass |
| activity | 0.000% | 0 | pass |
| pages-activity-drafts | 0.000% | 0 | pass |
| pages-activity-story | 0.000% | 0 | pass |
| pages-activity-production | 0.000% | 0 | pass |
| pages-activity-exchange | 0.000% | 0 | pass |
| recently-deleted-empty | 0.000% | 0 | pass |
| pages-deleted-global-empty | 0.000% | 0 | pass |
| pages-deleted-items | 0.000% | 0 | pass |
| pages-deleted-search | 0.000% | 0 | pass |
| pages-deleted-search-none | 0.000% | 0 | pass |
| pages-deleted-purge-confirm | 0.000% | 0 | pass |
| pages-deleted-restored | 0.000% | 0 | pass |
| pages-notes-restored | 0.000% | 0 | pass |
| pages-settings-general | 0.000% | 0 | pass |
| pages-settings-general-edited | 0.000% | 0 | pass |
| pages-settings-title-error | 0.000% | 0 | pass |
| pages-settings-saved | 0.000% | 0 | pass |
| pages-settings-status-menu | 0.000% | 0 | pass |
| pages-settings-type-menu | 0.000% | 0 | pass |
| pages-settings-writing | 0.000% | 0 | pass |
| pages-settings-shortcuts | 0.000% | 0 | pass |
| pages-settings-offline-ai | 0.000% | 0 | pass |
| pages-settings-advanced | 0.000% | 0 | pass |
| pages-settings-appearance | 0.000% | 0 | pass |
| pages-settings-writing-word | 0.000% | 0 | pass |
| pages-settings-language-menu | 0.000% | 0 | pass |
| pages-templates | 0.000% | 0 | pass |
| pages-templates-preview | 0.000% | 0 | pass |
| pages-templates-actions-menu | 0.000% | 0 | pass |
| pages-templates-new | 0.000% | 0 | pass |
| ai-closed-home | 0.000% | 0 | pass |
| ai-panel | 0.000% | 0 | pass |
| ai-settings-menu | 0.000% | 0 | pass |
| ai-panel-idea-vault | 0.000% | 0 | pass |
| ai-panel-story | 0.000% | 0 | pass |
| ai-panel-screenplay | 0.000% | 0 | pass |
| ai-panel-breakdown | 0.000% | 0 | pass |
| ai-panel-production | 0.000% | 0 | pass |
| ai-panel-schedule | 0.000% | 0 | pass |
| ai-panel-call-sheets | 0.000% | 0 | pass |
| ai-panel-files | 0.000% | 0 | pass |
| ai-closed-screenplay | 0.000% | 0 | pass |
| ai-closed-breakdown | 0.000% | 0 | pass |
| ai-panel-sidebar-hidden | 0.000% | 0 | pass |
| ai-panel-narrow-window | 0.000% | 0 | pass |
| ai-search-palette | 0.000% | 0 | pass |
| menu-file-with-project | 0.000% | 0 | pass |
| package-share-for-review-menu | 0.000% | 0 | pass |
| package-import-export-history | 0.000% | 0 | pass |
| package-review-queue | 0.000% | 0 | pass |
| package-import-exchange-package | 0.000% | 0 | pass |
| package-print | 0.000% | 0 | pass |
| package-back-up-project | 0.000% | 0 | pass |
| package-backup-ready | 0.000% | 0 | pass |
| package-backup-portable | 0.000% | 0 | pass |
| package-backup-created | 0.000% | 0 | pass |
| package-export-full-project | 0.000% | 0 | pass |
| package-export-full-project-portable | 0.000% | 0 | pass |
| package-export-full-project-done | 0.000% | 0 | pass |
| package-share-script | 0.000% | 0 | pass |
| package-share-beat-board | 0.000% | 0 | pass |
| package-share-beat-board-one-act | 0.000% | 0 | pass |
| package-share-script-choose-scenes | 0.000% | 0 | pass |
| package-share-script-one-scene | 0.000% | 0 | pass |
| package-share-script-comments | 0.000% | 0 | pass |
| package-share-script-done | 0.000% | 0 | pass |
| package-review-viewer | 0.000% | 0 | pass |
| package-review-comment-typed | 0.000% | 0 | pass |
| package-review-comment-added | 0.000% | 0 | pass |
| package-review-response-exported | 0.000% | 0 | pass |
| package-import-preview | 0.000% | 0 | pass |
| package-import-preview-expanded | 0.000% | 0 | pass |
| package-import-done | 0.000% | 0 | pass |
| package-review-queue-after-import | 0.000% | 0 | pass |
| package-history-imports | 0.000% | 0 | pass |
| package-history-review-records | 0.000% | 0 | pass |
| package-history-my-reviews | 0.000% | 0 | pass |
| package-history-package-log | 0.000% | 0 | pass |
| package-restore | 0.087% | 0 | pass (differences between 30,136 and 1191,488 points) |
| package-restore-damaged | 0.000% | 0 | pass |
| package-restore-collision | 0.000% | 0 | pass |
| package-restore-opened-copy | 0.000% | 0 | pass |
| home-after-crash | 0.000% | 0 | pass |
| recovery-alert | 0.000% | 0 | pass |
| pages-recovery-kept | 0.000% | 0 | pass |
| home-project-unavailable | 0.000% | 0 | pass |
| project-unavailable | 0.000% | 0 | pass |

692 of 692 pages pass (a page fails above 0.50% differing pixels, channel difference over 24, or with any HTML miss); 0 HTML misses in all.

## Known limits of this capture (2026-10-20)

- The tour asks for a 1280 x 620 point window; the app's own minimum height makes most windows 1280 x 634, and about
  110 screens kept a few points more or less (listed under Window size). A narrow-window state is narrow on purpose.
- Eight runners shared one 1280 x 720 point display. A window that was not frontmost when its picture was taken looks
  inactive (dimmed sidebar and toolbar), as macOS draws it. Run fewer runners (OPENFRAME_E2E_PARALLEL=1) for frontmost
  windows everywhere.
- A sheet is composited over the window with a drawn shadow; where the sheet picture has transparent corners the shadow
  shows as a light rim, most visible in Dark.
- The dev app has no Offline AI engine, so no AI answer, download card or See sources state is captured.
- Images, audio and video reach the app only through the system Open panel or drag and drop, so no image item,
  moodboard picture, storyboard image or character portrait is captured.
