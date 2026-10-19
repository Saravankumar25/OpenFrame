# MacHTML: every screen of OpenFrame Studio for the Mac

179 screens in 20 areas, each captured twice from the real app: the default theme in Light mode (`light/`) and in Dark mode (`dark/`). Open `index.html` and use the Light and Dark switch; every page has the same switch. Page `NNN-name.html` is the same screen in both folders.

## How these were made

- The capture tour (`apps/macos/Sources/OpenFrameE2EDriver/Scenarios/CaptureTour.swift`, scenarios `zz-capture/*`) drives the real app through the accessibility tree, seeds a project called Night Bus (the bundled example screenplay, story acts and cards, characters, Idea Vault items, a breakdown, a schedule with days, a call sheet, budget lines, notes and tasks) and visits every screen it can reach. It is run twice. Each run first sets Settings, Appearance, Mode to Light or Dark (default theme), so the app draws itself in that mode.
- For every screen it writes a 2x picture of the whole window (title bar, toolbar, sidebar and content, as the window server draws it, with any sheet or popover at its real position) and a list of every accessibility element with its role, identifier, label, value, enabled state and frame in points.
- Menus are drawn from the real menu items, because an open menu is not a window the app can draw.
- `scripts/mac-html/build-mac-html.mjs` turns that into one page per screen. The page shows the window at its own size (one CSS pixel is one point; the 2x picture is shown at point size, so it is sharp), with the window corner radius and shadow, and every element laid over it as a real HTML element (buttons, menus as select boxes, fields with their value, text you can select) within a point of the real frame. The "Show HTML only" switch hides the picture so the HTML layer draws the screen alone. Where a control leads to another captured screen it is a link. Nothing is loaded from the network.
- Commit of the app these came from: `3b4feb1`.

## Regenerate

    apps/macos/E2E/capture.sh "<path to OpenFrame Studio.app or the debug executable>"

That runs the tour in Light and in Dark, builds this folder, and renders every page in WebKit to compare it with its picture (`scripts/mac-html/check-pages.swift`). The generator alone:

    node scripts/mac-html/build-mac-html.mjs --both <light run folder> <dark run folder> MacHTML <commit>

## Not captured

- Screens that need the system file panel (open, save, choose a folder): Open Review Package, Restore Backup or Package and the Browse buttons. The panel runs in another process and blocks the driver.
- Pop-up menus are drawn from their items, not photographed.
- The licence and trial screens (the internal build has none) and the AI Assistant in its ready and download states (a debug build has no model files; the panel shows its unavailable state). The team gate shows only on a build that has one.
- Voice recording (needs the microphone permission dialog), drag and drop states, and screens that need real files (image attachments, PDFs).

## Steps the last run skipped

- search palette home: not found within 6 s: id=palette-field label=- role=-
- folder: no folder
- button script-tag-menu: control is disabled: AXMenuButton id=script-tag-menu label=Tag Selection as title=Tag Selection as value=
- menu budget-currency-menu: not found within 6 s: id=budget-start-currency label=- role=-
- explore files-view: not a menu button: AXRadioGroup id=files-view label=View, View title= value=
- explore tasks-filter: not a menu button: AXRadioGroup id=tasks-filter label=Show tasks, Show tasks title= value=
- explore activity-filter: not a menu button: AXRadioGroup id=activity-filter label=Show activity, Show activity title= value=
- ai panel: not found within 15 s: id=ai-input label=- role=-

## Verification: pages against the pictures

Every page was rendered in WebKit at the window's own size (one CSS pixel is one point) and compared with its capture picture. The number is the mean difference over all colour channels, in percent; a page over 1.5% fails.

### light: passed

| Page | Mean difference |
|---|---|
| home-empty | 0.000% |
| menu-app | 0.000% |
| menu-file-home | 0.000% |
| menu-view-home | 0.000% |
| new-project-sheet | 0.000% |
| new-project-error | 0.000% |
| new-project-filled | 0.000% |
| new-project-type-menu | 0.000% |
| settings-general | 0.000% |
| settings-appearance | 0.000% |
| settings-offline-ai | 0.000% |
| settings-diagnostics | 0.000% |
| about | 0.000% |
| project-home-new | 0.000% |
| project-home | 0.000% |
| project-menu | 0.000% |
| menu-file-project | 0.000% |
| menu-edit-project | 0.000% |
| menu-go | 0.000% |
| project-rename-sheet | 0.000% |
| project-home-ai-panel | 0.000% |
| search-palette-project | 0.000% |
| search-palette-results | 0.000% |
| home-filled | 0.000% |
| home-card-menu | 0.000% |
| home-global-vault | 0.000% |
| home-two-projects | 0.000% |
| screenplay-editor-typing | 0.000% |
| screenplay-tools-menu | 0.000% |
| story-board-empty | 0.000% |
| story-add-menu | 0.000% |
| story-add-act-inline | 0.000% |
| story-add-sequence-sheet | 0.000% |
| story-add-card-sheet | 0.000% |
| story-board-filled | 0.000% |
| story-card-drawer | 0.000% |
| story-more-menu | 0.000% |
| story-card-display-sheet | 0.000% |
| story-apply-order-sheet | 0.000% |
| story-build-sheet | 0.000% |
| story-parking-lot | 0.000% |
| story-outline | 0.000% |
| story-timeline | 0.000% |
| story-characters-empty | 0.000% |
| story-add-character-sheet | 0.000% |
| story-characters | 0.000% |
| story-character-edit | 0.000% |
| story-character-menu | 0.000% |
| vault-empty | 0.000% |
| vault-add-menu | 0.000% |
| vault-add-note | 0.000% |
| vault-add-url | 0.000% |
| vault-add-audio-voice-note | 0.000% |
| vault-add-quote | 0.000% |
| vault-add-quick-capture | 0.000% |
| vault-quick-capture-sheet | 0.000% |
| vault-filled | 0.000% |
| vault-view-rectangle-grid-1x2 | 0.000% |
| vault-view-list | 0.000% |
| vault-view-move | 0.000% |
| vault-item-drawer | 0.000% |
| vault-drawer-send-to-story | 0.000% |
| vault-scope-global | 0.000% |
| vault-scope-this-project | 0.000% |
| vault-new-folder-sheet | 0.000% |
| vault-new-collection-sheet | 0.000% |
| vault-with-folder-and-collection | 0.000% |
| vault-recently-deleted | 0.000% |
| breakdown-no-source | 0.000% |
| breakdown-source-sheet | 0.000% |
| breakdown-scenes | 0.000% |
| breakdown-suggestions | 0.000% |
| breakdown-accepted-one | 0.000% |
| breakdown-all-accepted | 0.000% |
| breakdown-add-element-sheet | 0.000% |
| breakdown-add-category-menu | 0.000% |
| breakdown-add-element-filled | 0.000% |
| breakdown-scene-2 | 0.000% |
| breakdown-mark-scene-complete | 0.000% |
| breakdown-quick-reuse | 0.000% |
| breakdown-change-source | 0.000% |
| breakdown-tab-catalog | 0.000% |
| breakdown-tab-locations | 0.000% |
| breakdown-tab-cast-crew | 0.000% |
| breakdown-tab-categories | 0.000% |
| schedule-empty | 0.000% |
| schedule-create-sheet | 0.000% |
| schedule-board-unscheduled | 0.000% |
| schedule-add-day-sheet | 0.000% |
| schedule-add-off-day-sheet | 0.000% |
| schedule-strip-menu | 0.000% |
| schedule-board | 0.000% |
| schedule-day-menu | 0.000% |
| schedule-meal-break-sheet | 0.000% |
| schedule-day-detail | 0.000% |
| schedule-day-detail-times | 0.000% |
| schedule-day-export-day | 0.000% |
| schedule-list | 0.000% |
| schedule-calendar | 0.000% |
| schedule-suggest-grouping | 0.000% |
| schedule-warnings | 0.000% |
| schedule-schedule-settings | 0.000% |
| schedule-export-schedule | 0.000% |
| schedule-unscheduled-pool | 0.000% |
| schedule-dood | 0.000% |
| schedule-dood-export-day-out-of-days | 0.000% |
| callsheets-empty | 0.000% |
| callsheet-create-sheet | 0.000% |
| callsheet-create-day-menu | 0.000% |
| callsheet-create-template-menu | 0.000% |
| callsheet-editor | 0.000% |
| callsheet-section-menu | 0.000% |
| callsheet-section-weather | 0.000% |
| callsheet-section-extras | 0.000% |
| callsheet-section-crew | 0.000% |
| callsheet-section-walkie-channels-and-contacts | 0.000% |
| callsheet-add-crew-row | 0.000% |
| callsheet-finalize | 0.000% |
| callsheet-export | 0.000% |
| callsheet-refresh-from-schedule | 0.000% |
| callsheets-list | 0.000% |
| callsheets-tab-daily-view | 0.000% |
| callsheets-tab-sides | 0.000% |
| callsheets-tab-reports | 0.000% |
| budget-start | 0.000% |
| budget-empty-table | 0.000% |
| budget-add-line-sheet | 0.000% |
| budget-add-line-filled | 0.000% |
| budget-table | 0.000% |
| budget-edit-plan | 0.000% |
| budget-save-a-snapshot | 0.000% |
| budget-export-budget | 0.000% |
| visual-moodboards | 0.000% |
| moodboard-new-sheet | 0.000% |
| moodboard-canvas | 0.000% |
| moodboard-add-image-menu | 0.000% |
| moodboard-add-image-from-idea-vault | 0.000% |
| moodboard-add-note | 0.000% |
| moodboard-add-a-link | 0.000% |
| moodboard-export-moodboard | 0.000% |
| visual-tab-moodboards | 0.000% |
| visual-tab-storyboards | 0.000% |
| visual-tab-shot-lists | 0.000% |
| storyboard-new-storyboard | 0.000% |
| shots-export-shot-list | 0.000% |
| files-empty | 0.000% |
| files-add-menu | 0.000% |
| files-new-folder-sheet | 0.000% |
| files-with-folder | 0.000% |
| notes-empty | 0.000% |
| notes-new-note | 0.000% |
| notes-note-written | 0.000% |
| notes-pin-the-note | 0.000% |
| tasks-new-task | 0.000% |
| tasks-due-date | 0.000% |
| notes-and-tasks | 0.000% |
| activity | 0.000% |
| recently-deleted-empty | 0.000% |
| menu-file-with-project | 0.000% |
| package-back-up-project | 0.000% |
| package-export-full-project | 0.000% |
| package-import-exchange-package | 0.000% |
| package-review-queue | 0.000% |
| package-import-export-history | 0.000% |
| package-history-review-records | 0.000% |
| package-history-my-reviews | 0.000% |
| package-history-package-log | 0.000% |
| package-print | 0.000% |
| package-share-export-review-package-script | 0.000% |
| package-share-story-board-package | 0.000% |
| home-after-crash | 0.000% |
| recovery-alert | 0.000% |
| home-project-unavailable | 0.000% |
| project-unavailable | 0.000% |
| ai-panel | 0.000% |
| ai-settings-menu | 0.000% |
| ai-panel-story | 0.000% |
| ai-panel-breakdown | 0.000% |
| ai-panel-visual-planning | 0.000% |

179 pages, mean difference 0.000%, 0 over 1.50%.

### dark: passed

| Page | Mean difference |
|---|---|
| home-empty | 0.000% |
| menu-app | 0.000% |
| menu-file-home | 0.000% |
| menu-view-home | 0.000% |
| new-project-sheet | 0.000% |
| new-project-error | 0.000% |
| new-project-filled | 0.000% |
| new-project-type-menu | 0.000% |
| settings-general | 0.000% |
| settings-appearance | 0.000% |
| settings-offline-ai | 0.000% |
| settings-diagnostics | 0.000% |
| about | 0.000% |
| project-home-new | 0.000% |
| project-home | 0.000% |
| project-menu | 0.000% |
| menu-file-project | 0.000% |
| menu-edit-project | 0.000% |
| menu-go | 0.000% |
| project-rename-sheet | 0.000% |
| project-home-ai-panel | 0.000% |
| search-palette-project | 0.000% |
| search-palette-results | 0.000% |
| home-filled | 0.000% |
| home-card-menu | 0.000% |
| home-global-vault | 0.000% |
| home-two-projects | 0.002% |
| screenplay-editor-typing | 0.000% |
| screenplay-tools-menu | 0.000% |
| story-board-empty | 0.000% |
| story-add-menu | 0.000% |
| story-add-act-inline | 0.000% |
| story-add-sequence-sheet | 0.000% |
| story-add-card-sheet | 0.000% |
| story-board-filled | 0.000% |
| story-card-drawer | 0.000% |
| story-more-menu | 0.000% |
| story-card-display-sheet | 0.000% |
| story-apply-order-sheet | 0.000% |
| story-build-sheet | 0.000% |
| story-parking-lot | 0.000% |
| story-outline | 0.000% |
| story-timeline | 0.000% |
| story-characters-empty | 0.000% |
| story-add-character-sheet | 0.000% |
| story-characters | 0.000% |
| story-character-edit | 0.000% |
| story-character-menu | 0.000% |
| vault-empty | 0.000% |
| vault-add-menu | 0.000% |
| vault-add-note | 0.000% |
| vault-add-url | 0.000% |
| vault-add-audio-voice-note | 0.000% |
| vault-add-quote | 0.000% |
| vault-add-quick-capture | 0.000% |
| vault-quick-capture-sheet | 0.000% |
| vault-filled | 0.000% |
| vault-view-rectangle-grid-1x2 | 0.000% |
| vault-view-list | 0.000% |
| vault-view-move | 0.000% |
| vault-item-drawer | 0.000% |
| vault-drawer-send-to-story | 0.000% |
| vault-scope-global | 0.000% |
| vault-scope-this-project | 0.000% |
| vault-new-folder-sheet | 0.000% |
| vault-new-collection-sheet | 0.000% |
| vault-with-folder-and-collection | 0.000% |
| vault-recently-deleted | 0.000% |
| breakdown-no-source | 0.000% |
| breakdown-source-sheet | 0.000% |
| breakdown-scenes | 0.000% |
| breakdown-suggestions | 0.000% |
| breakdown-accepted-one | 0.000% |
| breakdown-all-accepted | 0.000% |
| breakdown-add-element-sheet | 0.000% |
| breakdown-add-category-menu | 0.000% |
| breakdown-add-element-filled | 0.000% |
| breakdown-scene-2 | 0.000% |
| breakdown-mark-scene-complete | 0.000% |
| breakdown-quick-reuse | 0.000% |
| breakdown-change-source | 0.000% |
| breakdown-tab-catalog | 0.000% |
| breakdown-tab-locations | 0.000% |
| breakdown-tab-cast-crew | 0.000% |
| breakdown-tab-categories | 0.000% |
| schedule-empty | 0.000% |
| schedule-create-sheet | 0.000% |
| schedule-board-unscheduled | 0.000% |
| schedule-add-day-sheet | 0.000% |
| schedule-add-off-day-sheet | 0.000% |
| schedule-strip-menu | 0.000% |
| schedule-board | 0.000% |
| schedule-day-menu | 0.000% |
| schedule-meal-break-sheet | 0.000% |
| schedule-day-detail | 0.000% |
| schedule-day-detail-times | 0.000% |
| schedule-day-export-day | 0.000% |
| schedule-list | 0.000% |
| schedule-calendar | 0.000% |
| schedule-suggest-grouping | 0.000% |
| schedule-warnings | 0.000% |
| schedule-schedule-settings | 0.000% |
| schedule-export-schedule | 0.000% |
| schedule-unscheduled-pool | 0.000% |
| schedule-dood | 0.000% |
| schedule-dood-export-day-out-of-days | 0.000% |
| callsheets-empty | 0.000% |
| callsheet-create-sheet | 0.000% |
| callsheet-create-day-menu | 0.000% |
| callsheet-create-template-menu | 0.000% |
| callsheet-editor | 0.000% |
| callsheet-section-menu | 0.000% |
| callsheet-section-weather | 0.000% |
| callsheet-section-extras | 0.000% |
| callsheet-section-crew | 0.000% |
| callsheet-section-walkie-channels-and-contacts | 0.000% |
| callsheet-add-crew-row | 0.000% |
| callsheet-finalize | 0.000% |
| callsheet-export | 0.000% |
| callsheet-refresh-from-schedule | 0.000% |
| callsheets-list | 0.000% |
| callsheets-tab-daily-view | 0.000% |
| callsheets-tab-sides | 0.000% |
| callsheets-tab-reports | 0.000% |
| budget-start | 0.000% |
| budget-empty-table | 0.000% |
| budget-add-line-sheet | 0.000% |
| budget-add-line-filled | 0.000% |
| budget-table | 0.000% |
| budget-edit-plan | 0.000% |
| budget-save-a-snapshot | 0.000% |
| budget-export-budget | 0.000% |
| visual-moodboards | 0.000% |
| moodboard-new-sheet | 0.000% |
| moodboard-canvas | 0.000% |
| moodboard-add-image-menu | 0.000% |
| moodboard-add-image-from-idea-vault | 0.000% |
| moodboard-add-note | 0.000% |
| moodboard-add-a-link | 0.000% |
| moodboard-export-moodboard | 0.000% |
| visual-tab-moodboards | 0.000% |
| visual-tab-storyboards | 0.000% |
| visual-tab-shot-lists | 0.000% |
| storyboard-new-storyboard | 0.000% |
| shots-export-shot-list | 0.000% |
| files-empty | 0.000% |
| files-add-menu | 0.000% |
| files-new-folder-sheet | 0.000% |
| files-with-folder | 0.000% |
| notes-empty | 0.000% |
| notes-new-note | 0.000% |
| notes-note-written | 0.000% |
| notes-pin-the-note | 0.000% |
| tasks-new-task | 0.000% |
| tasks-due-date | 0.000% |
| notes-and-tasks | 0.000% |
| activity | 0.000% |
| recently-deleted-empty | 0.000% |
| menu-file-with-project | 0.000% |
| package-back-up-project | 0.000% |
| package-export-full-project | 0.000% |
| package-import-exchange-package | 0.000% |
| package-review-queue | 0.000% |
| package-import-export-history | 0.000% |
| package-history-review-records | 0.000% |
| package-history-my-reviews | 0.000% |
| package-history-package-log | 0.000% |
| package-print | 0.000% |
| package-share-export-review-package-script | 0.000% |
| package-share-story-board-package | 0.000% |
| home-after-crash | 0.000% |
| recovery-alert | 0.000% |
| home-project-unavailable | 0.000% |
| project-unavailable | 0.000% |
| ai-panel | 0.000% |
| ai-settings-menu | 0.000% |
| ai-panel-story | 0.000% |
| ai-panel-breakdown | 0.000% |
| ai-panel-visual-planning | 0.000% |

179 pages, mean difference 0.000%, 0 over 1.50%.
