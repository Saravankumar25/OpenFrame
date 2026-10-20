# WindowsHTML

Every screen and state of OpenFrame Studio for Windows, captured from the running app as standalone HTML pages,
in Light and Dark mode of the default theme. It is the source for the website's screen recreations (Chapter 2).

- **809 screens and states**, each in `light/` and `dark/` under the same `NNN-name.html`.
- Captured from commit `0dd87ae` at a 1600x1000 window (the pages are laid out for that size).
- [index.html](index.html) lists every screen with a Light and Dark switch. Controls that lead to another captured
  screen carry an invisible link to it.

## How the pages were made

Nothing here is drawn by hand. The capture tour `tests/e2e/specs/zz-capture.test.mjs` drives the real app through
the end-to-end WebDriver harness. It seeds the invented demo project Night Bus (`tests/e2e/capture/seed.mjs`: the
bundled example screenplay, Beat Board, characters, Idea Vault, a second draft with comments, breakdown, catalog,
locations, cast and crew, shots, a storyboard, a moodboard, a schedule with days, call sheets, a budget, notes,
tasks and files) and an empty project for empty states, then walks the sections in `tests/e2e/capture/sections/`.

For each state, `tests/e2e/lib/capture.mjs` takes a full-window screenshot and serializes the web view's own DOM
(the web view is the app): form values become attributes, scroll positions are kept with an invisible
`scroll-initial-target` marker, canvas and app-asset images become the same pixels cut from the screenshot, and
scripts are removed. `scripts/windows-html/build-windows-html.mjs` stores the app's style sheets once in
`assets/` and writes the pages. They use no script and load nothing from the network. Text uses the Windows
fonts Segoe UI and Courier New, as the app does; they are not embedded (their licences do not allow it), so a
computer without them substitutes similar fonts.

## How to make them again

Build the debug app once (`node tests/e2e/run.mjs --only=zz-capture` builds it), then run the sections in eight
parallel parts, merge them and check every page in four shards (about 50 minutes):

```bash
node scripts/windows-html/parallel.mjs
```

The whole tour in one data root, one section after another (about 2.5 hours), gives the same pages:

```bash
E2E_CAPTURE=1 node tests/e2e/run.mjs --only=zz-capture --no-build
```

The run replaces `light/`, `dark/`, `assets/`, `index.html` and this file. Raw screenshots stay in
`.build-cache/windows-html-capture/` (not committed; 2400x1500 pixels each at 150 percent display scaling).

## Check

Every page was opened in the same WebView2 at the same size and compared with its screenshot (the visual diff
helper of `90-visual-baseline`): a pixel differs when a colour channel differs by more than 24 of 255, and a
page fails above 0.5 % differing pixels. 1618 of 1618 pages passed; the
largest difference was 0.460 %.

| Section | Screens |
|---|---|
| seed | 12 |
| 10-shell | 38 |
| 95-app-home | 40 |
| 20-vault | 78 |
| 30-story | 75 |
| 40-screenplay | 95 |
| 50-breakdown | 35 |
| 85-ai-packages | 60 |
| 60-production-plan | 99 |
| 70-production-schedule | 157 |
| 80-pages | 120 |

| No. | Screen | Light | Dark | Result |
|---|---|---|---|---|
| 001 | [app-first-launch-welcome](light/001-app-first-launch-welcome.html) | 0.000 % | 0.000 % | pass |
| 002 | [app-first-launch-name-required](light/002-app-first-launch-name-required.html) | 0.000 % | 0.000 % | pass |
| 003 | [app-home-empty](light/003-app-home-empty.html) | 0.000 % | 0.000 % | pass |
| 004 | [app-new-project-dialog](light/004-app-new-project-dialog.html) | 0.000 % | 0.000 % | pass |
| 005 | [app-new-project-title-required](light/005-app-new-project-title-required.html) | 0.000 % | 0.000 % | pass |
| 006 | [app-new-project-filled](light/006-app-new-project-filled.html) | 0.000 % | 0.000 % | pass |
| 007 | [app-new-project-optional-details](light/007-app-new-project-optional-details.html) | 0.000 % | 0.000 % | pass |
| 008 | [home-new-project](light/008-home-new-project.html) | 0.000 % | 0.001 % | pass |
| 009 | [sp-import-source](light/009-sp-import-source.html) | 0.000 % | 0.000 % | pass |
| 010 | [sp-import-sample-chosen](light/010-sp-import-sample-chosen.html) | 0.135 % | 0.133 % | pass |
| 011 | [sp-import-preview](light/011-sp-import-preview.html) | 0.001 % | 0.001 % | pass |
| 012 | [sp-import-complete](light/012-sp-import-complete.html) | 0.003 % | 0.003 % | pass |
| 013 | [shell-project-home](light/013-shell-project-home.html) | 0.000 % | 0.000 % | pass |
| 014 | [shell-project-menu](light/014-shell-project-menu.html) | 0.000 % | 0.000 % | pass |
| 015 | [shell-create-menu](light/015-shell-create-menu.html) | 0.000 % | 0.000 % | pass |
| 016 | [shell-nav-collapsed](light/016-shell-nav-collapsed.html) | 0.000 % | 0.000 % | pass |
| 017 | [shell-nav-collapsed-files](light/017-shell-nav-collapsed-files.html) | 0.000 % | 0.000 % | pass |
| 018 | [shell-search-empty](light/018-shell-search-empty.html) | 0.000 % | 0.000 % | pass |
| 019 | [shell-search-results](light/019-shell-search-results.html) | 0.303 % | 0.362 % | pass |
| 020 | [shell-search-results-second-active](light/020-shell-search-results-second-active.html) | 0.303 % | 0.362 % | pass |
| 021 | [shell-search-results-location](light/021-shell-search-results-location.html) | 0.314 % | 0.372 % | pass |
| 022 | [shell-search-no-matches](light/022-shell-search-no-matches.html) | 0.309 % | 0.367 % | pass |
| 023 | [shell-search-global-scope-empty](light/023-shell-search-global-scope-empty.html) | 0.000 % | 0.000 % | pass |
| 024 | [shell-search-global-scope](light/024-shell-search-global-scope.html) | 0.303 % | 0.362 % | pass |
| 025 | [shell-status-menu](light/025-shell-status-menu.html) | 0.002 % | 0.002 % | pass |
| 026 | [shell-toast-undoable](light/026-shell-toast-undoable.html) | 0.000 % | 0.000 % | pass |
| 027 | [shell-project-menu-undo-enabled](light/027-shell-project-menu-undo-enabled.html) | 0.000 % | 0.000 % | pass |
| 028 | [shell-toast-undid](light/028-shell-toast-undid.html) | 0.000 % | 0.000 % | pass |
| 029 | [shell-project-menu-redo-enabled](light/029-shell-project-menu-redo-enabled.html) | 0.000 % | 0.000 % | pass |
| 030 | [shell-status-panel](light/030-shell-status-panel.html) | 0.000 % | 0.000 % | pass |
| 031 | [shell-open-recovery-no-checkpoint](light/031-shell-open-recovery-no-checkpoint.html) | 0.002 % | 0.002 % | pass |
| 032 | [shell-toast-saved](light/032-shell-toast-saved.html) | 0.000 % | 0.000 % | pass |
| 033 | [shell-open-recovery-with-checkpoint](light/033-shell-open-recovery-with-checkpoint.html) | 0.001 % | 0.000 % | pass |
| 034 | [shell-display-name-dialog](light/034-shell-display-name-dialog.html) | 0.000 % | 0.000 % | pass |
| 035 | [shell-display-name-empty-error](light/035-shell-display-name-empty-error.html) | 0.000 % | 0.000 % | pass |
| 036 | [shell-status-menu-before-archive](light/036-shell-status-menu-before-archive.html) | 0.002 % | 0.002 % | pass |
| 037 | [shell-toast-archived](light/037-shell-toast-archived.html) | 0.000 % | 0.000 % | pass |
| 038 | [shell-archived-banner](light/038-shell-archived-banner.html) | 0.000 % | 0.000 % | pass |
| 039 | [shell-status-menu-archived](light/039-shell-status-menu-archived.html) | 0.000 % | 0.000 % | pass |
| 040 | [shell-toast-restored](light/040-shell-toast-restored.html) | 0.000 % | 0.000 % | pass |
| 041 | [shell-project-settings-dialog](light/041-shell-project-settings-dialog.html) | 0.000 % | 0.000 % | pass |
| 042 | [shell-project-settings-edited](light/042-shell-project-settings-edited.html) | 0.280 % | 0.285 % | pass |
| 043 | [shell-project-settings-title-required](light/043-shell-project-settings-title-required.html) | 0.000 % | 0.000 % | pass |
| 044 | [shell-back-enabled](light/044-shell-back-enabled.html) | 0.000 % | 0.000 % | pass |
| 045 | [shell-back-and-forward-enabled](light/045-shell-back-and-forward-enabled.html) | 0.050 % | 0.000 % | pass |
| 046 | [shell-forward-enabled](light/046-shell-forward-enabled.html) | 0.050 % | 0.000 % | pass |
| 047 | [shell-empty-project-home](light/047-shell-empty-project-home.html) | 0.000 % | 0.000 % | pass |
| 048 | [shell-empty-project-menu-pinned](light/048-shell-empty-project-menu-pinned.html) | 0.000 % | 0.000 % | pass |
| 049 | [shell-empty-project-status-panel](light/049-shell-empty-project-status-panel.html) | 0.000 % | 0.000 % | pass |
| 050 | [shell-empty-project-settings-dialog](light/050-shell-empty-project-settings-dialog.html) | 0.000 % | 0.000 % | pass |
| 051 | [apphome-projects](light/051-apphome-projects.html) | 0.000 % | 0.000 % | pass |
| 052 | [apphome-card-menu](light/052-apphome-card-menu.html) | 0.001 % | 0.001 % | pass |
| 053 | [apphome-card-context-menu](light/053-apphome-card-context-menu.html) | 0.005 % | 0.005 % | pass |
| 054 | [apphome-rename-dialog](light/054-apphome-rename-dialog.html) | 0.000 % | 0.000 % | pass |
| 055 | [apphome-rename-empty-error](light/055-apphome-rename-empty-error.html) | 0.000 % | 0.000 % | pass |
| 056 | [apphome-rename-filled](light/056-apphome-rename-filled.html) | 0.195 % | 0.197 % | pass |
| 057 | [apphome-toast-renamed](light/057-apphome-toast-renamed.html) | 0.000 % | 0.000 % | pass |
| 058 | [apphome-delete-dialog](light/058-apphome-delete-dialog.html) | 0.000 % | 0.000 % | pass |
| 059 | [apphome-delete-typed](light/059-apphome-delete-typed.html) | 0.265 % | 0.267 % | pass |
| 060 | [apphome-pinned-section](light/060-apphome-pinned-section.html) | 0.000 % | 0.000 % | pass |
| 061 | [apphome-card-menu-pinned](light/061-apphome-card-menu-pinned.html) | 0.006 % | 0.005 % | pass |
| 062 | [apphome-all-pinned](light/062-apphome-all-pinned.html) | 0.000 % | 0.000 % | pass |
| 063 | [apphome-search-results](light/063-apphome-search-results.html) | 0.130 % | 0.152 % | pass |
| 064 | [apphome-search-no-match](light/064-apphome-search-no-match.html) | 0.156 % | 0.179 % | pass |
| 065 | [apphome-toast-archived](light/065-apphome-toast-archived.html) | 0.000 % | 0.000 % | pass |
| 066 | [apphome-search-archived-match](light/066-apphome-search-archived-match.html) | 0.184 % | 0.207 % | pass |
| 067 | [apphome-archived-view](light/067-apphome-archived-view.html) | 0.000 % | 0.000 % | pass |
| 068 | [apphome-archived-row-menu](light/068-apphome-archived-row-menu.html) | 0.000 % | 0.000 % | pass |
| 069 | [apphome-delete-dialog-archived](light/069-apphome-delete-dialog-archived.html) | 0.000 % | 0.000 % | pass |
| 070 | [apphome-archived-search-no-match](light/070-apphome-archived-search-no-match.html) | 0.153 % | 0.178 % | pass |
| 071 | [apphome-all-archived](light/071-apphome-all-archived.html) | 0.000 % | 0.004 % | pass |
| 072 | [apphome-archived-view-two](light/072-apphome-archived-view-two.html) | 0.000 % | 0.000 % | pass |
| 073 | [apphome-toast-restored](light/073-apphome-toast-restored.html) | 0.068 % | 0.000 % | pass |
| 074 | [apphome-archived-empty](light/074-apphome-archived-empty.html) | 0.000 % | 0.000 % | pass |
| 075 | [apphome-restore-package-dialog](light/075-apphome-restore-package-dialog.html) | 0.000 % | 0.000 % | pass |
| 076 | [apphome-quick-capture-dialog](light/076-apphome-quick-capture-dialog.html) | 0.000 % | 0.000 % | pass |
| 077 | [apphome-global-vault](light/077-apphome-global-vault.html) | 0.208 % | 0.000 % | pass |
| 078 | [apphome-global-vault-add-menu](light/078-apphome-global-vault-add-menu.html) | 0.000 % | 0.000 % | pass |
| 079 | [apphome-settings-appearance](light/079-apphome-settings-appearance.html) | 0.000 % | 0.000 % | pass |
| 080 | [apphome-settings-shortcuts](light/080-apphome-settings-shortcuts.html) | 0.008 % | 0.005 % | pass |
| 081 | [apphome-settings-shortcuts-filtered](light/081-apphome-settings-shortcuts-filtered.html) | 0.149 % | 0.176 % | pass |
| 082 | [apphome-settings-shortcuts-recording](light/082-apphome-settings-shortcuts-recording.html) | 0.008 % | 0.006 % | pass |
| 083 | [apphome-settings-about](light/083-apphome-settings-about.html) | 0.001 % | 0.001 % | pass |
| 084 | [apphome-settings-about-support](light/084-apphome-settings-about-support.html) | 0.001 % | 0.001 % | pass |
| 085 | [apphome-display-name-dialog](light/085-apphome-display-name-dialog.html) | 0.000 % | 0.000 % | pass |
| 086 | [apphome-display-name-empty-error](light/086-apphome-display-name-empty-error.html) | 0.000 % | 0.000 % | pass |
| 087 | [apphome-new-project-dialog](light/087-apphome-new-project-dialog.html) | 0.000 % | 0.000 % | pass |
| 088 | [apphome-new-project-title-required](light/088-apphome-new-project-title-required.html) | 0.000 % | 0.000 % | pass |
| 089 | [apphome-new-project-filled](light/089-apphome-new-project-filled.html) | 0.000 % | 0.000 % | pass |
| 090 | [apphome-new-project-optional-details](light/090-apphome-new-project-optional-details.html) | 0.000 % | 0.000 % | pass |
| 091 | [vault-grid-all-items](light/091-vault-grid-all-items.html) | 0.002 % | 0.002 % | pass |
| 092 | [vault-view-card](light/092-vault-view-card.html) | 0.002 % | 0.002 % | pass |
| 093 | [vault-view-list](light/093-vault-view-list.html) | 0.000 % | 0.000 % | pass |
| 094 | [vault-view-folder](light/094-vault-view-folder.html) | 0.002 % | 0.002 % | pass |
| 095 | [vault-view-folder-open](light/095-vault-view-folder-open.html) | 0.000 % | 0.000 % | pass |
| 096 | [vault-view-pinned](light/096-vault-view-pinned.html) | 0.000 % | 0.000 % | pass |
| 097 | [vault-view-recently-added](light/097-vault-view-recently-added.html) | 0.002 % | 0.002 % | pass |
| 098 | [vault-view-recently-modified](light/098-vault-view-recently-modified.html) | 0.002 % | 0.002 % | pass |
| 099 | [vault-view-collection](light/099-vault-view-collection.html) | 0.000 % | 0.000 % | pass |
| 100 | [vault-view-tag](light/100-vault-view-tag.html) | 0.000 % | 0.000 % | pass |
| 101 | [vault-view-folder-from-panel](light/101-vault-view-folder-from-panel.html) | 0.000 % | 0.000 % | pass |
| 102 | [vault-add-menu](light/102-vault-add-menu.html) | 0.002 % | 0.002 % | pass |
| 103 | [vault-search-results](light/103-vault-search-results.html) | 0.001 % | 0.001 % | pass |
| 104 | [vault-search-no-results](light/104-vault-search-no-results.html) | 0.000 % | 0.001 % | pass |
| 105 | [vault-scope-global](light/105-vault-scope-global.html) | 0.000 % | 0.000 % | pass |
| 106 | [vault-global-pinned-empty](light/106-vault-global-pinned-empty.html) | 0.000 % | 0.000 % | pass |
| 107 | [vault-global-item-context-menu](light/107-vault-global-item-context-menu.html) | 0.000 % | 0.000 % | pass |
| 108 | [vault-copy-to-project-dialog](light/108-vault-copy-to-project-dialog.html) | 0.000 % | 0.000 % | pass |
| 109 | [vault-copy-to-project-done](light/109-vault-copy-to-project-done.html) | 0.000 % | 0.000 % | pass |
| 110 | [vault-drawer-copied-from-global](light/110-vault-drawer-copied-from-global.html) | 0.000 % | 0.000 % | pass |
| 111 | [vault-drawer-note](light/111-vault-drawer-note.html) | 0.000 % | 0.000 % | pass |
| 112 | [vault-drawer-quote](light/112-vault-drawer-quote.html) | 0.000 % | 0.000 % | pass |
| 113 | [vault-drawer-url](light/113-vault-drawer-url.html) | 0.000 % | 0.000 % | pass |
| 114 | [vault-drawer-url-invalid](light/114-vault-drawer-url-invalid.html) | 0.000 % | 0.000 % | pass |
| 115 | [vault-drawer-note-new](light/115-vault-drawer-note-new.html) | 0.000 % | 0.000 % | pass |
| 116 | [vault-item-deleted-toast](light/116-vault-item-deleted-toast.html) | 0.002 % | 0.002 % | pass |
| 117 | [vault-item-context-menu](light/117-vault-item-context-menu.html) | 0.000 % | 0.000 % | pass |
| 118 | [vault-item-context-menu-pinned](light/118-vault-item-context-menu-pinned.html) | 0.002 % | 0.002 % | pass |
| 119 | [vault-multiselect-bar](light/119-vault-multiselect-bar.html) | 0.010 % | 0.011 % | pass |
| 120 | [vault-item-context-menu-multi](light/120-vault-item-context-menu-multi.html) | 0.008 % | 0.009 % | pass |
| 121 | [vault-copy-to-global-dialog](light/121-vault-copy-to-global-dialog.html) | 0.003 % | 0.001 % | pass |
| 122 | [vault-move-to-folder-dialog](light/122-vault-move-to-folder-dialog.html) | 0.003 % | 0.001 % | pass |
| 123 | [vault-move-to-folder-new-folder](light/123-vault-move-to-folder-new-folder.html) | 0.003 % | 0.001 % | pass |
| 124 | [vault-add-to-collection-dialog](light/124-vault-add-to-collection-dialog.html) | 0.003 % | 0.001 % | pass |
| 125 | [vault-tag-dialog](light/125-vault-tag-dialog.html) | 0.003 % | 0.001 % | pass |
| 126 | [vault-tag-dialog-required](light/126-vault-tag-dialog-required.html) | 0.003 % | 0.001 % | pass |
| 127 | [vault-add-url-dialog](light/127-vault-add-url-dialog.html) | 0.000 % | 0.000 % | pass |
| 128 | [vault-add-url-invalid](light/128-vault-add-url-invalid.html) | 0.000 % | 0.000 % | pass |
| 129 | [vault-quick-capture-dialog](light/129-vault-quick-capture-dialog.html) | 0.000 % | 0.000 % | pass |
| 130 | [vault-quick-capture-filled](light/130-vault-quick-capture-filled.html) | 0.000 % | 0.000 % | pass |
| 131 | [vault-quick-capture-saved-toast](light/131-vault-quick-capture-saved-toast.html) | 0.000 % | 0.000 % | pass |
| 132 | [vault-sketch-dialog](light/132-vault-sketch-dialog.html) | 0.097 % | 0.129 % | pass |
| 133 | [vault-sketch-dialog-drawn](light/133-vault-sketch-dialog-drawn.html) | 0.270 % | 0.302 % | pass |
| 134 | [vault-record-note-dialog](light/134-vault-record-note-dialog.html) | 0.000 % | 0.000 % | pass |
| 135 | [vault-send-to-beat-board-dialog](light/135-vault-send-to-beat-board-dialog.html) | 0.004 % | 0.004 % | pass |
| 136 | [vault-send-to-beat-board-sequence](light/136-vault-send-to-beat-board-sequence.html) | 0.004 % | 0.004 % | pass |
| 137 | [vault-send-to-beat-board-character](light/137-vault-send-to-beat-board-character.html) | 0.004 % | 0.004 % | pass |
| 138 | [vault-new-collection-dialog](light/138-vault-new-collection-dialog.html) | 0.000 % | 0.000 % | pass |
| 139 | [vault-new-collection-name-required](light/139-vault-new-collection-name-required.html) | 0.000 % | 0.000 % | pass |
| 140 | [vault-collection-empty](light/140-vault-collection-empty.html) | 0.000 % | 0.000 % | pass |
| 141 | [vault-new-folder-dialog](light/141-vault-new-folder-dialog.html) | 0.000 % | 0.000 % | pass |
| 142 | [vault-new-folder-name-required](light/142-vault-new-folder-name-required.html) | 0.000 % | 0.000 % | pass |
| 143 | [vault-folder-empty](light/143-vault-folder-empty.html) | 0.000 % | 0.000 % | pass |
| 144 | [vault-folder-context-menu](light/144-vault-folder-context-menu.html) | 0.000 % | 0.000 % | pass |
| 145 | [vault-collection-context-menu](light/145-vault-collection-context-menu.html) | 0.000 % | 0.000 % | pass |
| 146 | [vault-rename-folder-dialog](light/146-vault-rename-folder-dialog.html) | 0.000 % | 0.000 % | pass |
| 147 | [vault-rename-collection-dialog](light/147-vault-rename-collection-dialog.html) | 0.000 % | 0.000 % | pass |
| 148 | [vault-folder-deleted-toast](light/148-vault-folder-deleted-toast.html) | 0.000 % | 0.000 % | pass |
| 149 | [vault-folder-card-context-menu](light/149-vault-folder-card-context-menu.html) | 0.002 % | 0.002 % | pass |
| 150 | [vault-pinned-empty](light/150-vault-pinned-empty.html) | 0.000 % | 0.000 % | pass |
| 151 | [vault-grid-all-types](light/151-vault-grid-all-types.html) | 0.004 % | 0.002 % | pass |
| 152 | [vault-view-list-all-types](light/152-vault-view-list-all-types.html) | 0.000 % | 0.000 % | pass |
| 153 | [vault-drawer-image](light/153-vault-drawer-image.html) | 0.003 % | 0.066 % | pass |
| 154 | [vault-drawer-sketch](light/154-vault-drawer-sketch.html) | 0.003 % | 0.066 % | pass |
| 155 | [vault-drawer-voice](light/155-vault-drawer-voice.html) | 0.066 % | 0.128 % | pass |
| 156 | [vault-drawer-audio](light/156-vault-drawer-audio.html) | 0.066 % | 0.129 % | pass |
| 157 | [vault-drawer-video](light/157-vault-drawer-video.html) | 0.003 % | 0.066 % | pass |
| 158 | [vault-drawer-pdf](light/158-vault-drawer-pdf.html) | 0.003 % | 0.066 % | pass |
| 159 | [vault-drawer-document](light/159-vault-drawer-document.html) | 0.003 % | 0.066 % | pass |
| 160 | [vault-drawer-file](light/160-vault-drawer-file.html) | 0.003 % | 0.066 % | pass |
| 161 | [vault-drawer-file-unavailable](light/161-vault-drawer-file-unavailable.html) | 0.003 % | 0.066 % | pass |
| 162 | [vault-drawer-not-found](light/162-vault-drawer-not-found.html) | 0.000 % | 0.000 % | pass |
| 163 | [vault-recently-deleted](light/163-vault-recently-deleted.html) | 0.000 % | 0.000 % | pass |
| 164 | [vault-purge-confirm](light/164-vault-purge-confirm.html) | 0.005 % | 0.004 % | pass |
| 165 | [vault-recently-deleted-empty](light/165-vault-recently-deleted-empty.html) | 0.000 % | 0.000 % | pass |
| 166 | [vault-empty-project](light/166-vault-empty-project.html) | 0.000 % | 0.000 % | pass |
| 167 | [vault-empty-add-menu](light/167-vault-empty-add-menu.html) | 0.000 % | 0.000 % | pass |
| 168 | [vault-send-to-beat-board-no-acts](light/168-vault-send-to-beat-board-no-acts.html) | 0.002 % | 0.002 % | pass |
| 169 | [story-page-filled](light/169-story-page-filled.html) | 0.000 % | 0.000 % | pass |
| 170 | [story-page-export-dialog](light/170-story-page-export-dialog.html) | 0.000 % | 0.000 % | pass |
| 171 | [story-board-filled](light/171-story-board-filled.html) | 0.000 % | 0.000 % | pass |
| 172 | [story-board-add-menu](light/172-story-board-add-menu.html) | 0.000 % | 0.000 % | pass |
| 173 | [story-board-more-actions-menu](light/173-story-board-more-actions-menu.html) | 0.000 % | 0.000 % | pass |
| 174 | [story-board-zoom-50](light/174-story-board-zoom-50.html) | 0.000 % | 0.000 % | pass |
| 175 | [story-board-zoom-150](light/175-story-board-zoom-150.html) | 0.000 % | 0.000 % | pass |
| 176 | [story-board-filter-active](light/176-story-board-filter-active.html) | 0.000 % | 0.000 % | pass |
| 177 | [story-board-parking-lot-closed](light/177-story-board-parking-lot-closed.html) | 0.000 % | 0.000 % | pass |
| 178 | [story-board-act-menu](light/178-story-board-act-menu.html) | 0.000 % | 0.000 % | pass |
| 179 | [story-board-act-collapsed](light/179-story-board-act-collapsed.html) | 0.000 % | 0.000 % | pass |
| 180 | [story-board-act-rename-inline](light/180-story-board-act-rename-inline.html) | 0.000 % | 0.000 % | pass |
| 181 | [story-board-new-act-input](light/181-story-board-new-act-input.html) | 0.000 % | 0.000 % | pass |
| 182 | [story-board-sequence-context-menu](light/182-story-board-sequence-context-menu.html) | 0.000 % | 0.000 % | pass |
| 183 | [story-board-card-context-menu](light/183-story-board-card-context-menu.html) | 0.000 % | 0.000 % | pass |
| 184 | [story-board-beat-context-menu](light/184-story-board-beat-context-menu.html) | 0.000 % | 0.000 % | pass |
| 185 | [story-board-parked-card-context-menu](light/185-story-board-parked-card-context-menu.html) | 0.000 % | 0.000 % | pass |
| 186 | [story-board-card-selected](light/186-story-board-card-selected.html) | 0.000 % | 0.000 % | pass |
| 187 | [story-board-multiselect-bar](light/187-story-board-multiselect-bar.html) | 0.000 % | 0.000 % | pass |
| 188 | [story-multiselect-move-dialog](light/188-story-multiselect-move-dialog.html) | 0.000 % | 0.000 % | pass |
| 189 | [story-export-dialog-selected](light/189-story-export-dialog-selected.html) | 0.000 % | 0.000 % | pass |
| 190 | [story-card-drawer](light/190-story-card-drawer.html) | 0.000 % | 0.000 % | pass |
| 191 | [story-card-drawer-parked](light/191-story-card-drawer-parked.html) | 0.000 % | 0.000 % | pass |
| 192 | [story-beat-drawer](light/192-story-beat-drawer.html) | 0.000 % | 0.000 % | pass |
| 193 | [story-sequence-drawer](light/193-story-sequence-drawer.html) | 0.000 % | 0.000 % | pass |
| 194 | [story-delete-act-dialog](light/194-story-delete-act-dialog.html) | 0.007 % | 0.006 % | pass |
| 195 | [story-delete-act-dialog-delete-all](light/195-story-delete-act-dialog-delete-all.html) | 0.007 % | 0.006 % | pass |
| 196 | [story-delete-sequence-dialog](light/196-story-delete-sequence-dialog.html) | 0.004 % | 0.003 % | pass |
| 197 | [story-delete-sequence-dialog-delete-all](light/197-story-delete-sequence-dialog-delete-all.html) | 0.004 % | 0.003 % | pass |
| 198 | [story-move-dialog](light/198-story-move-dialog.html) | 0.000 % | 0.000 % | pass |
| 199 | [story-act-note-dialog](light/199-story-act-note-dialog.html) | 0.000 % | 0.000 % | pass |
| 200 | [story-card-display-dialog](light/200-story-card-display-dialog.html) | 0.000 % | 0.000 % | pass |
| 201 | [story-card-display-dialog-changed](light/201-story-card-display-dialog-changed.html) | 0.000 % | 0.000 % | pass |
| 202 | [story-board-rich-cards](light/202-story-board-rich-cards.html) | 0.000 % | 0.000 % | pass |
| 203 | [story-build-dialog](light/203-story-build-dialog.html) | 0.000 % | 0.000 % | pass |
| 204 | [story-build-dialog-new-screenplay](light/204-story-build-dialog-new-screenplay.html) | 0.000 % | 0.000 % | pass |
| 205 | [story-build-dialog-title-clash](light/205-story-build-dialog-title-clash.html) | 0.000 % | 0.000 % | pass |
| 206 | [story-build-dialog-preselected](light/206-story-build-dialog-preselected.html) | 0.000 % | 0.000 % | pass |
| 207 | [story-build-dialog-missing-heading](light/207-story-build-dialog-missing-heading.html) | 0.002 % | 0.002 % | pass |
| 208 | [story-apply-order-dialog](light/208-story-apply-order-dialog.html) | 0.004 % | 0.004 % | pass |
| 209 | [story-apply-order-dialog-locked](light/209-story-apply-order-dialog-locked.html) | 0.005 % | 0.005 % | pass |
| 210 | [story-export-dialog](light/210-story-export-dialog.html) | 0.000 % | 0.000 % | pass |
| 211 | [story-outline-view](light/211-story-outline-view.html) | 0.000 % | 0.000 % | pass |
| 212 | [story-outline-act-menu](light/212-story-outline-act-menu.html) | 0.000 % | 0.000 % | pass |
| 213 | [story-outline-card-context-menu](light/213-story-outline-card-context-menu.html) | 0.000 % | 0.000 % | pass |
| 214 | [story-outline-new-act-input](light/214-story-outline-new-act-input.html) | 0.000 % | 0.000 % | pass |
| 215 | [story-outline-card-drawer](light/215-story-outline-card-drawer.html) | 0.000 % | 0.000 % | pass |
| 216 | [story-characters-list](light/216-story-characters-list.html) | 0.066 % | 0.063 % | pass |
| 217 | [story-character-detail-venkat](light/217-story-character-detail-venkat.html) | 0.071 % | 0.068 % | pass |
| 218 | [story-character-more-menu](light/218-story-character-more-menu.html) | 0.049 % | 0.046 % | pass |
| 219 | [story-new-character-dialog](light/219-story-new-character-dialog.html) | 0.024 % | 0.031 % | pass |
| 220 | [story-new-character-duplicate-warning](light/220-story-new-character-duplicate-warning.html) | 0.020 % | 0.026 % | pass |
| 221 | [story-edit-character-dialog](light/221-story-edit-character-dialog.html) | 0.024 % | 0.031 % | pass |
| 222 | [story-add-relationship-dialog](light/222-story-add-relationship-dialog.html) | 0.025 % | 0.033 % | pass |
| 223 | [story-add-relationship-dialog-filled](light/223-story-add-relationship-dialog-filled.html) | 0.025 % | 0.033 % | pass |
| 224 | [story-relationship-map](light/224-story-relationship-map.html) | 0.005 % | 0.004 % | pass |
| 225 | [story-delete-character-dialog-lakshmi-in-use](light/225-story-delete-character-dialog-lakshmi-in-use.html) | 0.029 % | 0.036 % | pass |
| 226 | [story-delete-character-dialog-padma-in-use](light/226-story-delete-character-dialog-padma-in-use.html) | 0.009 % | 0.009 % | pass |
| 227 | [story-character-archived-toast](light/227-story-character-archived-toast.html) | 0.066 % | 0.063 % | pass |
| 228 | [story-characters-show-archived](light/228-story-characters-show-archived.html) | 0.006 % | 0.005 % | pass |
| 229 | [story-timeline-filled](light/229-story-timeline-filled.html) | 0.004 % | 0.003 % | pass |
| 230 | [story-timeline-scenes-selected](light/230-story-timeline-scenes-selected.html) | 0.004 % | 0.003 % | pass |
| 231 | [story-assign-story-day-dialog](light/231-story-assign-story-day-dialog.html) | 0.000 % | 0.000 % | pass |
| 232 | [story-assign-story-day-dialog-filled](light/232-story-assign-story-day-dialog-filled.html) | 0.000 % | 0.000 % | pass |
| 233 | [story-timeline-day-assigned](light/233-story-timeline-day-assigned.html) | 0.006 % | 0.006 % | pass |
| 234 | [story-timeline-time-note](light/234-story-timeline-time-note.html) | 0.006 % | 0.006 % | pass |
| 235 | [story-timeline-grouped-days](light/235-story-timeline-grouped-days.html) | 0.006 % | 0.006 % | pass |
| 236 | [story-page-empty](light/236-story-page-empty.html) | 0.000 % | 0.000 % | pass |
| 237 | [story-board-empty](light/237-story-board-empty.html) | 0.000 % | 0.000 % | pass |
| 238 | [story-board-empty-add-menu](light/238-story-board-empty-add-menu.html) | 0.000 % | 0.000 % | pass |
| 239 | [story-board-empty-new-act-input](light/239-story-board-empty-new-act-input.html) | 0.000 % | 0.000 % | pass |
| 240 | [story-build-dialog-no-cards](light/240-story-build-dialog-no-cards.html) | 0.001 % | 0.001 % | pass |
| 241 | [story-apply-order-dialog-no-screenplay](light/241-story-apply-order-dialog-no-screenplay.html) | 0.000 % | 0.000 % | pass |
| 242 | [story-characters-empty](light/242-story-characters-empty.html) | 0.000 % | 0.000 % | pass |
| 243 | [story-timeline-empty](light/243-story-timeline-empty.html) | 0.000 % | 0.000 % | pass |
| 244 | [sp-editor-standard](light/244-sp-editor-standard.html) | 0.005 % | 0.005 % | pass |
| 245 | [sp-editor-scrolled-mid-script](light/245-sp-editor-scrolled-mid-script.html) | 0.007 % | 0.006 % | pass |
| 246 | [sp-zoom-50](light/246-sp-zoom-50.html) | 0.008 % | 0.007 % | pass |
| 247 | [sp-zoom-200](light/247-sp-zoom-200.html) | 0.006 % | 0.005 % | pass |
| 248 | [sp-navigator-columns-menu](light/248-sp-navigator-columns-menu.html) | 0.006 % | 0.005 % | pass |
| 249 | [sp-navigator-columns-menu-checked](light/249-sp-navigator-columns-menu-checked.html) | 0.006 % | 0.005 % | pass |
| 250 | [sp-navigator-columns-shown](light/250-sp-navigator-columns-shown.html) | 0.034 % | 0.005 % | pass |
| 251 | [sp-navigator-hidden](light/251-sp-navigator-hidden.html) | 0.095 % | 0.005 % | pass |
| 252 | [sp-scene-context-menu](light/252-sp-scene-context-menu.html) | 0.006 % | 0.005 % | pass |
| 253 | [sp-scene-hub-drawer](light/253-sp-scene-hub-drawer.html) | 0.363 % | 0.363 % | pass |
| 254 | [sp-scene-number-dialog](light/254-sp-scene-number-dialog.html) | 0.002 % | 0.002 % | pass |
| 255 | [sp-scene-number-warning](light/255-sp-scene-number-warning.html) | 0.002 % | 0.002 % | pass |
| 256 | [sp-reorder-warning-dialog](light/256-sp-reorder-warning-dialog.html) | 0.007 % | 0.007 % | pass |
| 257 | [sp-element-type-menu](light/257-sp-element-type-menu.html) | 0.004 % | 0.003 % | pass |
| 258 | [sp-suggestion-list](light/258-sp-suggestion-list.html) | 0.007 % | 0.007 % | pass |
| 259 | [sp-suggestion-list-create-new](light/259-sp-suggestion-list-create-new.html) | 0.006 % | 0.006 % | pass |
| 260 | [sp-comment-float-button](light/260-sp-comment-float-button.html) | 0.101 % | 0.100 % | pass |
| 261 | [sp-comment-composer-text](light/261-sp-comment-composer-text.html) | 0.004 % | 0.003 % | pass |
| 262 | [sp-standard-comments-panel](light/262-sp-standard-comments-panel.html) | 0.006 % | 0.005 % | pass |
| 263 | [sp-find-empty](light/263-sp-find-empty.html) | 0.079 % | 0.003 % | pass |
| 264 | [sp-find-results](light/264-sp-find-results.html) | 0.006 % | 0.005 % | pass |
| 265 | [sp-find-replace-filled](light/265-sp-find-replace-filled.html) | 0.006 % | 0.005 % | pass |
| 266 | [sp-find-no-results](light/266-sp-find-no-results.html) | 0.004 % | 0.003 % | pass |
| 267 | [sp-find-this-scene](light/267-sp-find-this-scene.html) | 0.004 % | 0.003 % | pass |
| 268 | [sp-view-menu](light/268-sp-view-menu.html) | 0.036 % | 0.035 % | pass |
| 269 | [sp-title-page-dialog](light/269-sp-title-page-dialog.html) | 0.001 % | 0.001 % | pass |
| 270 | [sp-title-page-empty-title](light/270-sp-title-page-empty-title.html) | 0.001 % | 0.001 % | pass |
| 271 | [sp-page-setup-dialog](light/271-sp-page-setup-dialog.html) | 0.001 % | 0.001 % | pass |
| 272 | [sp-page-setup-header-filled](light/272-sp-page-setup-header-filled.html) | 0.001 % | 0.001 % | pass |
| 273 | [sp-custom-elements-empty](light/273-sp-custom-elements-empty.html) | 0.001 % | 0.001 % | pass |
| 274 | [sp-custom-elements-add-form](light/274-sp-custom-elements-add-form.html) | 0.004 % | 0.004 % | pass |
| 275 | [sp-page-view-drawer](light/275-sp-page-view-drawer.html) | 0.004 % | 0.003 % | pass |
| 276 | [sp-spell-menu](light/276-sp-spell-menu.html) | 0.460 % | 0.456 % | pass |
| 277 | [sp-writing-room-beat-board](light/277-sp-writing-room-beat-board.html) | 0.009 % | 0.008 % | pass |
| 278 | [sp-writing-room-beat-board-all-cards](light/278-sp-writing-room-beat-board-all-cards.html) | 0.231 % | 0.007 % | pass |
| 279 | [sp-writing-room-no-panel](light/279-sp-writing-room-no-panel.html) | 0.006 % | 0.005 % | pass |
| 280 | [sp-panel-scene-notes](light/280-sp-panel-scene-notes.html) | 0.005 % | 0.004 % | pass |
| 281 | [sp-panel-private-note-menu](light/281-sp-panel-private-note-menu.html) | 0.005 % | 0.004 % | pass |
| 282 | [sp-panel-private-note-composer](light/282-sp-panel-private-note-composer.html) | 0.005 % | 0.004 % | pass |
| 283 | [sp-panel-scene-note-editing](light/283-sp-panel-scene-note-editing.html) | 0.005 % | 0.004 % | pass |
| 284 | [sp-panel-characters](light/284-sp-panel-characters.html) | 0.013 % | 0.009 % | pass |
| 285 | [sp-panel-characters-match-menu](light/285-sp-panel-characters-match-menu.html) | 0.011 % | 0.007 % | pass |
| 286 | [sp-panel-comments-open](light/286-sp-panel-comments-open.html) | 0.008 % | 0.006 % | pass |
| 287 | [sp-panel-comments-actions-menu](light/287-sp-panel-comments-actions-menu.html) | 0.008 % | 0.006 % | pass |
| 288 | [sp-panel-comments-resolved](light/288-sp-panel-comments-resolved.html) | 0.008 % | 0.006 % | pass |
| 289 | [sp-panel-comments-reply](light/289-sp-panel-comments-reply.html) | 0.008 % | 0.006 % | pass |
| 290 | [sp-panel-comments-composer-scene](light/290-sp-panel-comments-composer-scene.html) | 0.006 % | 0.005 % | pass |
| 291 | [sp-panel-comments-composer-draft](light/291-sp-panel-comments-composer-draft.html) | 0.006 % | 0.005 % | pass |
| 292 | [sp-panel-draft-info](light/292-sp-panel-draft-info.html) | 0.006 % | 0.005 % | pass |
| 293 | [sp-writing-room-two-panels](light/293-sp-writing-room-two-panels.html) | 0.005 % | 0.004 % | pass |
| 294 | [sp-focus-mode](light/294-sp-focus-mode.html) | 0.000 % | 0.000 % | pass |
| 295 | [sp-draft-switch-menu](light/295-sp-draft-switch-menu.html) | 0.006 % | 0.005 % | pass |
| 296 | [sp-editor-older-draft](light/296-sp-editor-older-draft.html) | 0.005 % | 0.005 % | pass |
| 297 | [sp-panel-comments-empty](light/297-sp-panel-comments-empty.html) | 0.007 % | 0.006 % | pass |
| 298 | [sp-panel-comments-resolved-empty](light/298-sp-panel-comments-resolved-empty.html) | 0.007 % | 0.006 % | pass |
| 299 | [sp-drafts-drawer](light/299-sp-drafts-drawer.html) | 0.007 % | 0.006 % | pass |
| 300 | [sp-draft-actions-menu](light/300-sp-draft-actions-menu.html) | 0.007 % | 0.006 % | pass |
| 301 | [sp-draft-actions-menu-other](light/301-sp-draft-actions-menu-other.html) | 0.007 % | 0.006 % | pass |
| 302 | [sp-rename-draft-dialog](light/302-sp-rename-draft-dialog.html) | 0.003 % | 0.003 % | pass |
| 303 | [sp-delete-draft-dialog-current](light/303-sp-delete-draft-dialog-current.html) | 0.003 % | 0.003 % | pass |
| 304 | [sp-delete-draft-dialog](light/304-sp-delete-draft-dialog.html) | 0.003 % | 0.003 % | pass |
| 305 | [sp-new-draft-dialog](light/305-sp-new-draft-dialog.html) | 0.002 % | 0.002 % | pass |
| 306 | [sp-new-draft-previous-source](light/306-sp-new-draft-previous-source.html) | 0.002 % | 0.002 % | pass |
| 307 | [sp-new-draft-duplicate-name](light/307-sp-new-draft-duplicate-name.html) | 0.169 % | 0.166 % | pass |
| 308 | [sp-compare-text-only](light/308-sp-compare-text-only.html) | 0.012 % | 0.010 % | pass |
| 309 | [sp-compare-scene-only](light/309-sp-compare-scene-only.html) | 0.006 % | 0.008 % | pass |
| 310 | [sp-compare-second-change](light/310-sp-compare-second-change.html) | 0.137 % | 0.010 % | pass |
| 311 | [sp-compare-same-draft](light/311-sp-compare-same-draft.html) | 0.000 % | 0.000 % | pass |
| 312 | [sp-lock-dialog](light/312-sp-lock-dialog.html) | 0.004 % | 0.004 % | pass |
| 313 | [sp-locked-draft](light/313-sp-locked-draft.html) | 0.005 % | 0.004 % | pass |
| 314 | [sp-locked-edit-dialog](light/314-sp-locked-edit-dialog.html) | 0.003 % | 0.003 % | pass |
| 315 | [sp-start-revision-dialog](light/315-sp-start-revision-dialog.html) | 0.003 % | 0.003 % | pass |
| 316 | [sp-start-revision-filled](light/316-sp-start-revision-filled.html) | 0.003 % | 0.003 % | pass |
| 317 | [sp-start-revision-custom-colour](light/317-sp-start-revision-custom-colour.html) | 0.003 % | 0.003 % | pass |
| 318 | [sp-revisions-drawer-locked](light/318-sp-revisions-drawer-locked.html) | 0.007 % | 0.007 % | pass |
| 319 | [sp-revision-draft-editor](light/319-sp-revision-draft-editor.html) | 0.002 % | 0.001 % | pass |
| 320 | [sp-navigator-revision-column](light/320-sp-navigator-revision-column.html) | 0.002 % | 0.002 % | pass |
| 321 | [sp-revisions-drawer-revision](light/321-sp-revisions-drawer-revision.html) | 0.007 % | 0.006 % | pass |
| 322 | [sp-page-view-revision-marks](light/322-sp-page-view-revision-marks.html) | 0.014 % | 0.014 % | pass |
| 323 | [sp-export-revision-marked](light/323-sp-export-revision-marked.html) | 0.008 % | 0.008 % | pass |
| 324 | [sp-export-dialog](light/324-sp-export-dialog.html) | 0.002 % | 0.002 % | pass |
| 325 | [sp-export-more-formats](light/325-sp-export-more-formats.html) | 0.002 % | 0.002 % | pass |
| 326 | [sp-export-selected-none](light/326-sp-export-selected-none.html) | 0.005 % | 0.005 % | pass |
| 327 | [sp-export-selected-scenes](light/327-sp-export-selected-scenes.html) | 0.015 % | 0.016 % | pass |
| 328 | [sp-print-preview](light/328-sp-print-preview.html) | 0.002 % | 0.002 % | pass |
| 329 | [sp-print-preview-zoomed](light/329-sp-print-preview-zoomed.html) | 0.002 % | 0.002 % | pass |
| 330 | [sp-print-preview-no-title-page](light/330-sp-print-preview-no-title-page.html) | 0.005 % | 0.005 % | pass |
| 331 | [sp-import-pasted-text](light/331-sp-import-pasted-text.html) | 0.002 % | 0.002 % | pass |
| 332 | [sp-import-failed](light/332-sp-import-failed.html) | 0.002 % | 0.002 % | pass |
| 333 | [sp-screenplay-picker-menu](light/333-sp-screenplay-picker-menu.html) | 0.006 % | 0.005 % | pass |
| 334 | [sp-new-screenplay-dialog](light/334-sp-new-screenplay-dialog.html) | 0.002 % | 0.002 % | pass |
| 335 | [sp-new-screenplay-duplicate-title](light/335-sp-new-screenplay-duplicate-title.html) | 0.002 % | 0.002 % | pass |
| 336 | [sp-rename-screenplay-dialog](light/336-sp-rename-screenplay-dialog.html) | 0.002 % | 0.002 % | pass |
| 337 | [sp-delete-screenplay-dialog](light/337-sp-delete-screenplay-dialog.html) | 0.004 % | 0.004 % | pass |
| 338 | [sp-empty-workspace](light/338-sp-empty-workspace.html) | 0.000 % | 0.000 % | pass |
| 339 | [bd-no-source](light/339-bd-no-source.html) | 0.000 % | 0.000 % | pass |
| 340 | [bd-select-source-empty-project](light/340-bd-select-source-empty-project.html) | 0.103 % | 0.000 % | pass |
| 341 | [bd-main](light/341-bd-main.html) | 0.013 % | 0.012 % | pass |
| 342 | [bd-change-source-dialog](light/342-bd-change-source-dialog.html) | 0.005 % | 0.005 % | pass |
| 343 | [bd-filter-menu](light/343-bd-filter-menu.html) | 0.009 % | 0.008 % | pass |
| 344 | [bd-filter-only-suggested](light/344-bd-filter-only-suggested.html) | 0.013 % | 0.012 % | pass |
| 345 | [bd-filter-by-category](light/345-bd-filter-by-category.html) | 0.010 % | 0.009 % | pass |
| 346 | [bd-filter-no-match](light/346-bd-filter-no-match.html) | 0.004 % | 0.003 % | pass |
| 347 | [bd-scene-list-hidden](light/347-bd-scene-list-hidden.html) | 0.096 % | 0.008 % | pass |
| 348 | [bd-panel-hidden](light/348-bd-panel-hidden.html) | 0.096 % | 0.012 % | pass |
| 349 | [bd-both-hidden](light/349-bd-both-hidden.html) | 0.097 % | 0.008 % | pass |
| 350 | [bd-scene-suggestions](light/350-bd-scene-suggestions.html) | 0.013 % | 0.012 % | pass |
| 351 | [bd-suggestion-review-dialog](light/351-bd-suggestion-review-dialog.html) | 0.014 % | 0.014 % | pass |
| 352 | [bd-edit-suggestion-dialog](light/352-bd-edit-suggestion-dialog.html) | 0.006 % | 0.007 % | pass |
| 353 | [bd-possible-existing-item-dialog](light/353-bd-possible-existing-item-dialog.html) | 0.006 % | 0.004 % | pass |
| 354 | [bd-scene-marked-complete](light/354-bd-scene-marked-complete.html) | 0.013 % | 0.012 % | pass |
| 355 | [bd-scene-quick-reuse](light/355-bd-scene-quick-reuse.html) | 0.013 % | 0.012 % | pass |
| 356 | [bd-scene-nothing-yet](light/356-bd-scene-nothing-yet.html) | 0.013 % | 0.012 % | pass |
| 357 | [bd-tag-menu](light/357-bd-tag-menu.html) | 0.114 % | 0.111 % | pass |
| 358 | [bd-tag-element-dialog](light/358-bd-tag-element-dialog.html) | 0.005 % | 0.005 % | pass |
| 359 | [bd-add-element-existing-match](light/359-bd-add-element-existing-match.html) | 0.005 % | 0.005 % | pass |
| 360 | [bd-element-tagged](light/360-bd-element-tagged.html) | 0.013 % | 0.012 % | pass |
| 361 | [bd-element-actions-menu](light/361-bd-element-actions-menu.html) | 0.013 % | 0.012 % | pass |
| 362 | [bd-element-notes-dialog](light/362-bd-element-notes-dialog.html) | 0.005 % | 0.005 % | pass |
| 363 | [bd-show-in-script](light/363-bd-show-in-script.html) | 0.074 % | 0.075 % | pass |
| 364 | [bd-add-element-dialog](light/364-bd-add-element-dialog.html) | 0.005 % | 0.005 % | pass |
| 365 | [bd-export-dialog](light/365-bd-export-dialog.html) | 0.005 % | 0.005 % | pass |
| 366 | [bd-export-selected-scenes](light/366-bd-export-selected-scenes.html) | 0.007 % | 0.008 % | pass |
| 367 | [bd-export-csv](light/367-bd-export-csv.html) | 0.012 % | 0.013 % | pass |
| 368 | [bd-scene-no-text](light/368-bd-scene-no-text.html) | 0.005 % | 0.004 % | pass |
| 369 | [bd-scene-changed-banner](light/369-bd-scene-changed-banner.html) | 0.005 % | 0.004 % | pass |
| 370 | [bd-scene-changes-dialog](light/370-bd-scene-changes-dialog.html) | 0.001 % | 0.002 % | pass |
| 371 | [bd-needs-review-view](light/371-bd-needs-review-view.html) | 0.319 % | 0.017 % | pass |
| 372 | [bd-removed-scenes-view](light/372-bd-removed-scenes-view.html) | 0.306 % | 0.008 % | pass |
| 373 | [bd-removed-scene-panel](light/373-bd-removed-scene-panel.html) | 0.002 % | 0.002 % | pass |
| 374 | [ai-panel-ready](light/374-ai-panel-ready.html) | 0.000 % | 0.000 % | pass |
| 375 | [ai-settings-menu](light/375-ai-settings-menu.html) | 0.000 % | 0.000 % | pass |
| 376 | [ai-scope-current-screenplay](light/376-ai-scope-current-screenplay.html) | 0.051 % | 0.000 % | pass |
| 377 | [ai-scope-beat-board](light/377-ai-scope-beat-board.html) | 0.051 % | 0.000 % | pass |
| 378 | [ai-offline-settings](light/378-ai-offline-settings.html) | 0.002 % | 0.002 % | pass |
| 379 | [ai-offline-technical-details](light/379-ai-offline-technical-details.html) | 0.028 % | 0.024 % | pass |
| 380 | [ai-offline-remove-confirm](light/380-ai-offline-remove-confirm.html) | 0.001 % | 0.001 % | pass |
| 381 | [ai-question-typed](light/381-ai-question-typed.html) | 0.000 % | 0.000 % | pass |
| 382 | [ai-working](light/382-ai-working.html) | 0.000 % | 0.000 % | pass |
| 383 | [ai-answer-characters](light/383-ai-answer-characters.html) | 0.000 % | 0.000 % | pass |
| 384 | [ai-answer-characters-sources](light/384-ai-answer-characters-sources.html) | 0.000 % | 0.000 % | pass |
| 385 | [ai-panel-screenplay](light/385-ai-panel-screenplay.html) | 0.037 % | 0.001 % | pass |
| 386 | [ai-answer-locations](light/386-ai-answer-locations.html) | 0.002 % | 0.001 % | pass |
| 387 | [ai-answer-locations-sources-open](light/387-ai-answer-locations-sources-open.html) | 0.004 % | 0.004 % | pass |
| 388 | [ai-answer-scene-count](light/388-ai-answer-scene-count.html) | 0.002 % | 0.001 % | pass |
| 389 | [ai-proposed-changes](light/389-ai-proposed-changes.html) | 0.002 % | 0.001 % | pass |
| 390 | [ai-proposal-rejected](light/390-ai-proposal-rejected.html) | 0.002 % | 0.001 % | pass |
| 391 | [ai-proposed-changes-second](light/391-ai-proposed-changes-second.html) | 0.002 % | 0.001 % | pass |
| 392 | [ai-proposal-applied](light/392-ai-proposal-applied.html) | 0.002 % | 0.001 % | pass |
| 393 | [ai-answer-delete-request](light/393-ai-answer-delete-request.html) | 0.002 % | 0.001 % | pass |
| 394 | [ai-panel-story](light/394-ai-panel-story.html) | 0.036 % | 0.000 % | pass |
| 395 | [ai-panel-breakdown](light/395-ai-panel-breakdown.html) | 0.049 % | 0.012 % | pass |
| 396 | [ai-panel-production](light/396-ai-panel-production.html) | 0.037 % | 0.001 % | pass |
| 397 | [ai-panel-vault](light/397-ai-panel-vault.html) | 0.036 % | 0.000 % | pass |
| 398 | [ai-offline-health-filled](light/398-ai-offline-health-filled.html) | 0.000 % | 0.000 % | pass |
| 399 | [ai-panel-empty-project](light/399-ai-panel-empty-project.html) | 0.000 % | 0.001 % | pass |
| 400 | [pkg-project-menu](light/400-pkg-project-menu.html) | 0.000 % | 0.000 % | pass |
| 401 | [pkg-backup-dialog](light/401-pkg-backup-dialog.html) | 0.001 % | 0.000 % | pass |
| 402 | [pkg-export-project-dialog](light/402-pkg-export-project-dialog.html) | 0.000 % | 0.000 % | pass |
| 403 | [pkg-export-script-dialog](light/403-pkg-export-script-dialog.html) | 0.000 % | 0.000 % | pass |
| 404 | [pkg-export-script-comments](light/404-pkg-export-script-comments.html) | 0.000 % | 0.000 % | pass |
| 405 | [pkg-export-script-no-scene](light/405-pkg-export-script-no-scene.html) | 0.000 % | 0.000 % | pass |
| 406 | [pkg-export-script-scene-chosen](light/406-pkg-export-script-scene-chosen.html) | 0.004 % | 0.004 % | pass |
| 407 | [pkg-export-beat-board-dialog](light/407-pkg-export-beat-board-dialog.html) | 0.001 % | 0.001 % | pass |
| 408 | [pkg-export-beat-board-one-act](light/408-pkg-export-beat-board-one-act.html) | 0.000 % | 0.000 % | pass |
| 409 | [pkg-import-exchange-choose](light/409-pkg-import-exchange-choose.html) | 0.000 % | 0.000 % | pass |
| 410 | [pkg-review-queue-empty](light/410-pkg-review-queue-empty.html) | 0.000 % | 0.000 % | pass |
| 411 | [pkg-history-imports](light/411-pkg-history-imports.html) | 0.000 % | 0.000 % | pass |
| 412 | [pkg-history-review-records](light/412-pkg-history-review-records.html) | 0.000 % | 0.000 % | pass |
| 413 | [pkg-history-my-reviews](light/413-pkg-history-my-reviews.html) | 0.000 % | 0.000 % | pass |
| 414 | [pkg-history-package-log](light/414-pkg-history-package-log.html) | 0.000 % | 0.000 % | pass |
| 415 | [pkg-open-project-package-dialog](light/415-pkg-open-project-package-dialog.html) | 0.000 % | 0.000 % | pass |
| 416 | [licence-trial-chip](light/416-licence-trial-chip.html) | 0.000 % | 0.000 % | pass |
| 417 | [licence-buy-unavailable](light/417-licence-buy-unavailable.html) | 0.155 % | 0.150 % | pass |
| 418 | [licence-trial-reminder](light/418-licence-trial-reminder.html) | 0.000 % | 0.000 % | pass |
| 419 | [licence-trial-ending-chip](light/419-licence-trial-ending-chip.html) | 0.000 % | 0.000 % | pass |
| 420 | [licence-data-note](light/420-licence-data-note.html) | 0.000 % | 0.000 % | pass |
| 421 | [licence-data-note-read](light/421-licence-data-note-read.html) | 0.001 % | 0.001 % | pass |
| 422 | [licence-data-note-confirmed](light/422-licence-data-note-confirmed.html) | 0.014 % | 0.014 % | pass |
| 423 | [licence-tutorial-step-1](light/423-licence-tutorial-step-1.html) | 0.004 % | 0.004 % | pass |
| 424 | [licence-tutorial-step-2](light/424-licence-tutorial-step-2.html) | 0.004 % | 0.004 % | pass |
| 425 | [licence-tutorial-step-3](light/425-licence-tutorial-step-3.html) | 0.095 % | 0.094 % | pass |
| 426 | [licence-tutorial-step-4](light/426-licence-tutorial-step-4.html) | 0.005 % | 0.004 % | pass |
| 427 | [licence-tutorial-step-4-ticked](light/427-licence-tutorial-step-4-ticked.html) | 0.018 % | 0.018 % | pass |
| 428 | [licence-welcome-family](light/428-licence-welcome-family.html) | 0.000 % | 0.000 % | pass |
| 429 | [licence-check-needed](light/429-licence-check-needed.html) | 0.002 % | 0.002 % | pass |
| 430 | [licence-check-purchase-button](light/430-licence-check-purchase-button.html) | 0.000 % | 0.000 % | pass |
| 431 | [licence-locked-trial-ended-microsoft-store](light/431-licence-locked-trial-ended-microsoft-store.html) | 0.001 % | 0.001 % | pass |
| 432 | [licence-locked-buy-unavailable](light/432-licence-locked-buy-unavailable.html) | 0.001 % | 0.001 % | pass |
| 433 | [licence-locked-revoked](light/433-licence-locked-revoked.html) | 0.230 % | 0.227 % | pass |
| 434 | [prod-overview](light/434-prod-overview.html) | 0.001 % | 0.001 % | pass |
| 435 | [prod-overview-source-dialog](light/435-prod-overview-source-dialog.html) | 0.001 % | 0.001 % | pass |
| 436 | [prod-overview-no-source](light/436-prod-overview-no-source.html) | 0.000 % | 0.000 % | pass |
| 437 | [prod-overview-start-setup-dialog](light/437-prod-overview-start-setup-dialog.html) | 0.000 % | 0.000 % | pass |
| 438 | [prod-catalog-empty](light/438-prod-catalog-empty.html) | 0.000 % | 0.000 % | pass |
| 439 | [prod-locations-empty](light/439-prod-locations-empty.html) | 0.000 % | 0.000 % | pass |
| 440 | [prod-cast-empty](light/440-prod-cast-empty.html) | 0.000 % | 0.000 % | pass |
| 441 | [prod-crew-empty](light/441-prod-crew-empty.html) | 0.002 % | 0.004 % | pass |
| 442 | [prod-moodboards-empty](light/442-prod-moodboards-empty.html) | 0.000 % | 0.000 % | pass |
| 443 | [prod-storyboards-empty](light/443-prod-storyboards-empty.html) | 0.000 % | 0.000 % | pass |
| 444 | [prod-shots-no-script](light/444-prod-shots-no-script.html) | 0.001 % | 0.001 % | pass |
| 445 | [prod-catalog](light/445-prod-catalog.html) | 0.000 % | 0.000 % | pass |
| 446 | [prod-catalog-category-props](light/446-prod-catalog-category-props.html) | 0.000 % | 0.000 % | pass |
| 447 | [prod-catalog-more-menu](light/447-prod-catalog-more-menu.html) | 0.000 % | 0.000 % | pass |
| 448 | [prod-catalog-no-match](light/448-prod-catalog-no-match.html) | 0.000 % | 0.000 % | pass |
| 449 | [prod-catalog-row-menu](light/449-prod-catalog-row-menu.html) | 0.000 % | 0.000 % | pass |
| 450 | [prod-catalog-drawer](light/450-prod-catalog-drawer.html) | 0.000 % | 0.000 % | pass |
| 451 | [bd-catalog-scenes-banner](light/451-bd-catalog-scenes-banner.html) | 0.049 % | 0.011 % | pass |
| 452 | [prod-catalog-archive-confirm](light/452-prod-catalog-archive-confirm.html) | 0.000 % | 0.000 % | pass |
| 453 | [prod-catalog-replace-dialog](light/453-prod-catalog-replace-dialog.html) | 0.014 % | 0.014 % | pass |
| 454 | [prod-catalog-delete-confirm](light/454-prod-catalog-delete-confirm.html) | 0.000 % | 0.000 % | pass |
| 455 | [prod-catalog-add-dialog](light/455-prod-catalog-add-dialog.html) | 0.000 % | 0.000 % | pass |
| 456 | [prod-catalog-add-duplicate-warning](light/456-prod-catalog-add-duplicate-warning.html) | 0.002 % | 0.003 % | pass |
| 457 | [prod-catalog-export-dialog](light/457-prod-catalog-export-dialog.html) | 0.000 % | 0.000 % | pass |
| 458 | [prod-locations](light/458-prod-locations.html) | 0.000 % | 0.000 % | pass |
| 459 | [prod-locations-filter-confirmed](light/459-prod-locations-filter-confirmed.html) | 0.000 % | 0.000 % | pass |
| 460 | [prod-locations-no-match](light/460-prod-locations-no-match.html) | 0.000 % | 0.000 % | pass |
| 461 | [prod-location-row-menu](light/461-prod-location-row-menu.html) | 0.000 % | 0.000 % | pass |
| 462 | [prod-location-drawer](light/462-prod-location-drawer.html) | 0.000 % | 0.000 % | pass |
| 463 | [prod-location-drawer-notes](light/463-prod-location-drawer-notes.html) | 0.000 % | 0.000 % | pass |
| 464 | [prod-location-delete-confirm](light/464-prod-location-delete-confirm.html) | 0.000 % | 0.000 % | pass |
| 465 | [prod-location-drawer-hospital](light/465-prod-location-drawer-hospital.html) | 0.000 % | 0.000 % | pass |
| 466 | [prod-location-rejected-banner](light/466-prod-location-rejected-banner.html) | 0.000 % | 0.000 % | pass |
| 467 | [prod-location-replace-dialog](light/467-prod-location-replace-dialog.html) | 0.000 % | 0.000 % | pass |
| 468 | [prod-location-add-dialog](light/468-prod-location-add-dialog.html) | 0.000 % | 0.000 % | pass |
| 469 | [prod-cast](light/469-prod-cast.html) | 0.000 % | 0.000 % | pass |
| 470 | [prod-cast-assign-dialog](light/470-prod-cast-assign-dialog.html) | 0.006 % | 0.005 % | pass |
| 471 | [prod-cast-assign-alternate-banner](light/471-prod-cast-assign-alternate-banner.html) | 0.009 % | 0.008 % | pass |
| 472 | [prod-cast-row-menu](light/472-prod-cast-row-menu.html) | 0.000 % | 0.000 % | pass |
| 473 | [prod-cast-assign-existing-dialog](light/473-prod-cast-assign-existing-dialog.html) | 0.005 % | 0.005 % | pass |
| 474 | [prod-cast-drawer](light/474-prod-cast-drawer.html) | 0.000 % | 0.000 % | pass |
| 475 | [prod-cast-remove-confirm](light/475-prod-cast-remove-confirm.html) | 0.000 % | 0.000 % | pass |
| 476 | [prod-crew](light/476-prod-crew.html) | 0.002 % | 0.004 % | pass |
| 477 | [prod-crew-drawer](light/477-prod-crew-drawer.html) | 0.002 % | 0.004 % | pass |
| 478 | [prod-crew-remove-confirm](light/478-prod-crew-remove-confirm.html) | 0.001 % | 0.001 % | pass |
| 479 | [prod-crew-add-dialog](light/479-prod-crew-add-dialog.html) | 0.000 % | 0.000 % | pass |
| 480 | [prod-moodboard](light/480-prod-moodboard.html) | 0.000 % | 0.000 % | pass |
| 481 | [prod-moodboard-note-selected](light/481-prod-moodboard-note-selected.html) | 0.000 % | 0.000 % | pass |
| 482 | [prod-moodboard-multi-selection](light/482-prod-moodboard-multi-selection.html) | 0.000 % | 0.000 % | pass |
| 483 | [prod-moodboard-item-menu](light/483-prod-moodboard-item-menu.html) | 0.000 % | 0.000 % | pass |
| 484 | [prod-moodboard-note-drawer](light/484-prod-moodboard-note-drawer.html) | 0.000 % | 0.000 % | pass |
| 485 | [prod-moodboard-note-error](light/485-prod-moodboard-note-error.html) | 0.000 % | 0.000 % | pass |
| 486 | [prod-moodboard-link-drawer](light/486-prod-moodboard-link-drawer.html) | 0.000 % | 0.000 % | pass |
| 487 | [prod-moodboard-image-drawer](light/487-prod-moodboard-image-drawer.html) | 0.000 % | 0.000 % | pass |
| 488 | [prod-moodboard-add-image-menu](light/488-prod-moodboard-add-image-menu.html) | 0.000 % | 0.000 % | pass |
| 489 | [prod-moodboard-vault-dialog](light/489-prod-moodboard-vault-dialog.html) | 0.000 % | 0.000 % | pass |
| 490 | [prod-moodboard-add-note-dialog](light/490-prod-moodboard-add-note-dialog.html) | 0.000 % | 0.000 % | pass |
| 491 | [prod-moodboard-add-note-error](light/491-prod-moodboard-add-note-error.html) | 0.138 % | 0.136 % | pass |
| 492 | [prod-moodboard-add-link-dialog](light/492-prod-moodboard-add-link-dialog.html) | 0.000 % | 0.000 % | pass |
| 493 | [prod-moodboard-add-link-filled](light/493-prod-moodboard-add-link-filled.html) | 0.000 % | 0.000 % | pass |
| 494 | [prod-moodboard-list-menu](light/494-prod-moodboard-list-menu.html) | 0.000 % | 0.000 % | pass |
| 495 | [prod-moodboard-rename-dialog](light/495-prod-moodboard-rename-dialog.html) | 0.000 % | 0.000 % | pass |
| 496 | [prod-moodboard-board-notes-dialog](light/496-prod-moodboard-board-notes-dialog.html) | 0.000 % | 0.000 % | pass |
| 497 | [prod-moodboard-delete-confirm](light/497-prod-moodboard-delete-confirm.html) | 0.005 % | 0.004 % | pass |
| 498 | [prod-moodboard-export-dialog](light/498-prod-moodboard-export-dialog.html) | 0.000 % | 0.000 % | pass |
| 499 | [prod-moodboard-new-dialog](light/499-prod-moodboard-new-dialog.html) | 0.000 % | 0.000 % | pass |
| 500 | [prod-moodboard-new-error](light/500-prod-moodboard-new-error.html) | 0.233 % | 0.230 % | pass |
| 501 | [prod-moodboard-new-filled](light/501-prod-moodboard-new-filled.html) | 0.000 % | 0.000 % | pass |
| 502 | [prod-moodboard-empty-canvas](light/502-prod-moodboard-empty-canvas.html) | 0.000 % | 0.000 % | pass |
| 503 | [prod-storyboard-panels](light/503-prod-storyboard-panels.html) | 0.009 % | 0.010 % | pass |
| 504 | [prod-storyboard-new-dialog](light/504-prod-storyboard-new-dialog.html) | 0.000 % | 0.000 % | pass |
| 505 | [prod-storyboard-new-error](light/505-prod-storyboard-new-error.html) | 0.220 % | 0.217 % | pass |
| 506 | [prod-storyboard-no-panels](light/506-prod-storyboard-no-panels.html) | 0.000 % | 0.000 % | pass |
| 507 | [prod-storyboard-panel-menu](light/507-prod-storyboard-panel-menu.html) | 0.009 % | 0.010 % | pass |
| 508 | [prod-storyboard-move-panel-dialog](light/508-prod-storyboard-move-panel-dialog.html) | 0.000 % | 0.000 % | pass |
| 509 | [prod-storyboard-panel-drawer](light/509-prod-storyboard-panel-drawer.html) | 0.009 % | 0.010 % | pass |
| 510 | [prod-storyboard-panel-duration-error](light/510-prod-storyboard-panel-duration-error.html) | 0.009 % | 0.010 % | pass |
| 511 | [prod-storyboard-add-panel-menu](light/511-prod-storyboard-add-panel-menu.html) | 0.009 % | 0.010 % | pass |
| 512 | [prod-storyboard-sketch-dialog](light/512-prod-storyboard-sketch-dialog.html) | 0.044 % | 0.087 % | pass |
| 513 | [prod-storyboard-sketch-drawn](light/513-prod-storyboard-sketch-drawn.html) | 0.089 % | 0.111 % | pass |
| 514 | [prod-storyboard-actions-menu](light/514-prod-storyboard-actions-menu.html) | 0.010 % | 0.011 % | pass |
| 515 | [prod-storyboard-rename-dialog](light/515-prod-storyboard-rename-dialog.html) | 0.001 % | 0.000 % | pass |
| 516 | [prod-storyboard-link-scene-dialog](light/516-prod-storyboard-link-scene-dialog.html) | 0.000 % | 0.000 % | pass |
| 517 | [prod-storyboard-new-for-scene-dialog](light/517-prod-storyboard-new-for-scene-dialog.html) | 0.000 % | 0.000 % | pass |
| 518 | [prod-storyboard-delete-confirm](light/518-prod-storyboard-delete-confirm.html) | 0.006 % | 0.005 % | pass |
| 519 | [prod-storyboard-export-dialog](light/519-prod-storyboard-export-dialog.html) | 0.000 % | 0.000 % | pass |
| 520 | [prod-storyboard-scene-no-board](light/520-prod-storyboard-scene-no-board.html) | 0.138 % | 0.000 % | pass |
| 521 | [prod-shots-list](light/521-prod-shots-list.html) | 0.000 % | 0.001 % | pass |
| 522 | [prod-shots-grid](light/522-prod-shots-grid.html) | 0.003 % | 0.003 % | pass |
| 523 | [prod-shots-all-scenes](light/523-prod-shots-all-scenes.html) | 0.001 % | 0.002 % | pass |
| 524 | [prod-shots-scene-empty](light/524-prod-shots-scene-empty.html) | 0.000 % | 0.001 % | pass |
| 525 | [prod-shots-composer-error](light/525-prod-shots-composer-error.html) | 0.165 % | 0.001 % | pass |
| 526 | [prod-shot-menu](light/526-prod-shot-menu.html) | 0.000 % | 0.001 % | pass |
| 527 | [prod-shot-drawer](light/527-prod-shot-drawer.html) | 0.007 % | 0.007 % | pass |
| 528 | [prod-shot-move-dialog](light/528-prod-shot-move-dialog.html) | 0.000 % | 0.000 % | pass |
| 529 | [prod-shot-attach-dialog](light/529-prod-shot-attach-dialog.html) | 0.000 % | 0.000 % | pass |
| 530 | [prod-shots-scene-actions-menu](light/530-prod-shots-scene-actions-menu.html) | 0.000 % | 0.001 % | pass |
| 531 | [prod-shots-copy-planning-dialog](light/531-prod-shots-copy-planning-dialog.html) | 0.000 % | 0.000 % | pass |
| 532 | [prod-shots-export-dialog](light/532-prod-shots-export-dialog.html) | 0.000 % | 0.000 % | pass |
| 533 | [prod-schedule-board](light/533-prod-schedule-board.html) | 0.014 % | 0.014 % | pass |
| 534 | [prod-schedule-list](light/534-prod-schedule-list.html) | 0.015 % | 0.013 % | pass |
| 535 | [prod-schedule-calendar](light/535-prod-schedule-calendar.html) | 0.020 % | 0.018 % | pass |
| 536 | [prod-schedule-calendar-next-month](light/536-prod-schedule-calendar-next-month.html) | 0.015 % | 0.013 % | pass |
| 537 | [prod-schedule-filter-board](light/537-prod-schedule-filter-board.html) | 0.018 % | 0.016 % | pass |
| 538 | [prod-schedule-filter-list](light/538-prod-schedule-filter-list.html) | 0.017 % | 0.015 % | pass |
| 539 | [prod-schedule-filter-no-match](light/539-prod-schedule-filter-no-match.html) | 0.015 % | 0.013 % | pass |
| 540 | [prod-schedule-create-day-dialog](light/540-prod-schedule-create-day-dialog.html) | 0.005 % | 0.006 % | pass |
| 541 | [prod-schedule-create-day-filled](light/541-prod-schedule-create-day-filled.html) | 0.005 % | 0.006 % | pass |
| 542 | [prod-schedule-day-created](light/542-prod-schedule-day-created.html) | 0.017 % | 0.016 % | pass |
| 543 | [prod-schedule-day-detail-empty](light/543-prod-schedule-day-detail-empty.html) | 0.001 % | 0.001 % | pass |
| 544 | [prod-schedule-off-day-dialog](light/544-prod-schedule-off-day-dialog.html) | 0.006 % | 0.007 % | pass |
| 545 | [prod-schedule-off-day-filled](light/545-prod-schedule-off-day-filled.html) | 0.006 % | 0.007 % | pass |
| 546 | [prod-schedule-off-day-created](light/546-prod-schedule-off-day-created.html) | 0.017 % | 0.015 % | pass |
| 547 | [prod-schedule-day-menu](light/547-prod-schedule-day-menu.html) | 0.021 % | 0.018 % | pass |
| 548 | [prod-schedule-edit-date-dialog](light/548-prod-schedule-edit-date-dialog.html) | 0.005 % | 0.007 % | pass |
| 549 | [prod-schedule-day-menu-empty-day](light/549-prod-schedule-day-menu-empty-day.html) | 0.019 % | 0.017 % | pass |
| 550 | [prod-schedule-edit-date-dialog-no-call-sheet](light/550-prod-schedule-edit-date-dialog-no-call-sheet.html) | 0.005 % | 0.006 % | pass |
| 551 | [prod-schedule-strip-menu-scheduled](light/551-prod-schedule-strip-menu-scheduled.html) | 0.018 % | 0.016 % | pass |
| 552 | [prod-schedule-strip-menu-unscheduled](light/552-prod-schedule-strip-menu-unscheduled.html) | 0.017 % | 0.015 % | pass |
| 553 | [prod-schedule-strip-moved](light/553-prod-schedule-strip-moved.html) | 0.017 % | 0.015 % | pass |
| 554 | [prod-schedule-marker-dialog](light/554-prod-schedule-marker-dialog.html) | 0.008 % | 0.009 % | pass |
| 555 | [prod-schedule-marker-error-duration](light/555-prod-schedule-marker-error-duration.html) | 0.094 % | 0.094 % | pass |
| 556 | [prod-schedule-marker-error-note](light/556-prod-schedule-marker-error-note.html) | 0.094 % | 0.094 % | pass |
| 557 | [prod-schedule-marker-dialog-filled](light/557-prod-schedule-marker-dialog-filled.html) | 0.008 % | 0.009 % | pass |
| 558 | [prod-schedule-marker-added](light/558-prod-schedule-marker-added.html) | 0.017 % | 0.016 % | pass |
| 559 | [prod-schedule-marker-edit-dialog](light/559-prod-schedule-marker-edit-dialog.html) | 0.008 % | 0.009 % | pass |
| 560 | [prod-schedule-strip-drawer](light/560-prod-schedule-strip-drawer.html) | 0.021 % | 0.019 % | pass |
| 561 | [prod-schedule-strip-drawer-error-estimate](light/561-prod-schedule-strip-drawer-error-estimate.html) | 0.021 % | 0.019 % | pass |
| 562 | [prod-schedule-strip-drawer-error-start](light/562-prod-schedule-strip-drawer-error-start.html) | 0.021 % | 0.019 % | pass |
| 563 | [prod-schedule-strip-drawer-error-pages](light/563-prod-schedule-strip-drawer-error-pages.html) | 0.021 % | 0.019 % | pass |
| 564 | [prod-schedule-strip-drawer-unscheduled](light/564-prod-schedule-strip-drawer-unscheduled.html) | 0.017 % | 0.016 % | pass |
| 565 | [prod-schedule-strip-drawer-scheduled-day-1](light/565-prod-schedule-strip-drawer-scheduled-day-1.html) | 0.021 % | 0.019 % | pass |
| 566 | [prod-schedule-warnings-drawer](light/566-prod-schedule-warnings-drawer.html) | 0.018 % | 0.017 % | pass |
| 567 | [prod-schedule-warnings-kept](light/567-prod-schedule-warnings-kept.html) | 0.083 % | 0.017 % | pass |
| 568 | [prod-schedule-warnings-dismissed](light/568-prod-schedule-warnings-dismissed.html) | 0.018 % | 0.017 % | pass |
| 569 | [prod-schedule-warning-open-day](light/569-prod-schedule-warning-open-day.html) | 0.004 % | 0.003 % | pass |
| 570 | [prod-schedule-suggestions-drawer](light/570-prod-schedule-suggestions-drawer.html) | 0.259 % | 0.017 % | pass |
| 571 | [prod-schedule-suggestions-apply-dialog](light/571-prod-schedule-suggestions-apply-dialog.html) | 0.008 % | 0.010 % | pass |
| 572 | [prod-schedule-settings-dialog](light/572-prod-schedule-settings-dialog.html) | 0.008 % | 0.009 % | pass |
| 573 | [prod-schedule-settings-error](light/573-prod-schedule-settings-error.html) | 0.100 % | 0.100 % | pass |
| 574 | [prod-schedule-settings-finalized-selected](light/574-prod-schedule-settings-finalized-selected.html) | 0.008 % | 0.009 % | pass |
| 575 | [prod-schedule-finalized-banner](light/575-prod-schedule-finalized-banner.html) | 0.022 % | 0.021 % | pass |
| 576 | [prod-schedule-settings-reopen](light/576-prod-schedule-settings-reopen.html) | 0.009 % | 0.010 % | pass |
| 577 | [prod-schedule-settings-strict](light/577-prod-schedule-settings-strict.html) | 0.022 % | 0.023 % | pass |
| 578 | [prod-schedule-strict-chip](light/578-prod-schedule-strict-chip.html) | 0.018 % | 0.017 % | pass |
| 579 | [prod-schedule-keep-anyway-dialog](light/579-prod-schedule-keep-anyway-dialog.html) | 0.008 % | 0.010 % | pass |
| 580 | [prod-schedule-day-detail](light/580-prod-schedule-day-detail.html) | 0.004 % | 0.003 % | pass |
| 581 | [prod-schedule-day-detail-add-break-menu](light/581-prod-schedule-day-detail-add-break-menu.html) | 0.004 % | 0.003 % | pass |
| 582 | [prod-schedule-day-detail-actions-menu](light/582-prod-schedule-day-detail-actions-menu.html) | 0.006 % | 0.005 % | pass |
| 583 | [prod-schedule-day-export-dialog](light/583-prod-schedule-day-export-dialog.html) | 0.001 % | 0.001 % | pass |
| 584 | [prod-schedule-day-detail-timeline](light/584-prod-schedule-day-detail-timeline.html) | 0.005 % | 0.005 % | pass |
| 585 | [prod-schedule-day-detail-over-target](light/585-prod-schedule-day-detail-over-target.html) | 0.006 % | 0.006 % | pass |
| 586 | [prod-schedule-day-detail-time-error](light/586-prod-schedule-day-detail-time-error.html) | 0.005 % | 0.005 % | pass |
| 587 | [prod-schedule-day-detail-off](light/587-prod-schedule-day-detail-off.html) | 0.001 % | 0.001 % | pass |
| 588 | [prod-schedule-export-dialog](light/588-prod-schedule-export-dialog.html) | 0.008 % | 0.009 % | pass |
| 589 | [prod-dood-table](light/589-prod-dood-table.html) | 0.006 % | 0.006 % | pass |
| 590 | [prod-dood-cell-menu](light/590-prod-dood-cell-menu.html) | 0.006 % | 0.006 % | pass |
| 591 | [prod-dood-corrected](light/591-prod-dood-corrected.html) | 0.007 % | 0.007 % | pass |
| 592 | [prod-dood-note-dialog](light/592-prod-dood-note-dialog.html) | 0.004 % | 0.004 % | pass |
| 593 | [prod-dood-simple-codes](light/593-prod-dood-simple-codes.html) | 0.003 % | 0.005 % | pass |
| 594 | [prod-dood-one-cast](light/594-prod-dood-one-cast.html) | 0.005 % | 0.005 % | pass |
| 595 | [prod-dood-day-range](light/595-prod-dood-day-range.html) | 0.006 % | 0.006 % | pass |
| 596 | [prod-dood-export-dialog](light/596-prod-dood-export-dialog.html) | 0.004 % | 0.004 % | pass |
| 597 | [prod-daily-day-1](light/597-prod-daily-day-1.html) | 0.009 % | 0.008 % | pass |
| 598 | [prod-daily-no-call-sheet](light/598-prod-daily-no-call-sheet.html) | 0.007 % | 0.006 % | pass |
| 599 | [prod-daily-day-4](light/599-prod-daily-day-4.html) | 0.002 % | 0.002 % | pass |
| 600 | [prod-daily-off-day](light/600-prod-daily-off-day.html) | 0.000 % | 0.000 % | pass |
| 601 | [prod-daily-export-dialog](light/601-prod-daily-export-dialog.html) | 0.004 % | 0.003 % | pass |
| 602 | [prod-sides-setup](light/602-prod-sides-setup.html) | 0.002 % | 0.002 % | pass |
| 603 | [prod-sides-day-4](light/603-prod-sides-day-4.html) | 0.006 % | 0.006 % | pass |
| 604 | [prod-sides-selected-cast-none](light/604-prod-sides-selected-cast-none.html) | 0.001 % | 0.001 % | pass |
| 605 | [prod-sides-selected-cast](light/605-prod-sides-selected-cast.html) | 0.019 % | 0.019 % | pass |
| 606 | [prod-sides-cover-page](light/606-prod-sides-cover-page.html) | 0.016 % | 0.016 % | pass |
| 607 | [prod-sides-scene-unchecked](light/607-prod-sides-scene-unchecked.html) | 0.007 % | 0.007 % | pass |
| 608 | [prod-sides-no-scenes](light/608-prod-sides-no-scenes.html) | 0.001 % | 0.001 % | pass |
| 609 | [prod-sides-saved](light/609-prod-sides-saved.html) | 0.148 % | 0.145 % | pass |
| 610 | [prod-sides-live-with-saved](light/610-prod-sides-live-with-saved.html) | 0.002 % | 0.002 % | pass |
| 611 | [prod-sides-export-dialog](light/611-prod-sides-export-dialog.html) | 0.002 % | 0.002 % | pass |
| 612 | [prod-reports-scene](light/612-prod-reports-scene.html) | 0.001 % | 0.001 % | pass |
| 613 | [prod-reports-location](light/613-prod-reports-location.html) | 0.001 % | 0.001 % | pass |
| 614 | [prod-reports-cast-scene](light/614-prod-reports-cast-scene.html) | 0.002 % | 0.003 % | pass |
| 615 | [prod-reports-prop](light/615-prod-reports-prop.html) | 0.002 % | 0.003 % | pass |
| 616 | [prod-reports-schedule](light/616-prod-reports-schedule.html) | 0.002 % | 0.003 % | pass |
| 617 | [prod-reports-breakdown-completeness](light/617-prod-reports-breakdown-completeness.html) | 0.001 % | 0.001 % | pass |
| 618 | [prod-reports-filtered](light/618-prod-reports-filtered.html) | 0.002 % | 0.003 % | pass |
| 619 | [prod-reports-filter-no-match](light/619-prod-reports-filter-no-match.html) | 0.002 % | 0.003 % | pass |
| 620 | [prod-reports-saved-snapshot](light/620-prod-reports-saved-snapshot.html) | 0.003 % | 0.003 % | pass |
| 621 | [prod-reports-live-with-snapshots](light/621-prod-reports-live-with-snapshots.html) | 0.001 % | 0.001 % | pass |
| 622 | [prod-reports-export-dialog](light/622-prod-reports-export-dialog.html) | 0.000 % | 0.000 % | pass |
| 623 | [prod-budget-filled](light/623-prod-budget-filled.html) | 0.005 % | 0.005 % | pass |
| 624 | [prod-budget-add-line-dialog](light/624-prod-budget-add-line-dialog.html) | 0.001 % | 0.001 % | pass |
| 625 | [prod-budget-add-line-error-description](light/625-prod-budget-add-line-error-description.html) | 0.133 % | 0.126 % | pass |
| 626 | [prod-budget-add-line-error-planned](light/626-prod-budget-add-line-error-planned.html) | 0.131 % | 0.126 % | pass |
| 627 | [prod-budget-add-line-filled](light/627-prod-budget-add-line-filled.html) | 0.001 % | 0.001 % | pass |
| 628 | [prod-budget-line-added](light/628-prod-budget-line-added.html) | 0.005 % | 0.005 % | pass |
| 629 | [prod-budget-edit-line-dialog](light/629-prod-budget-edit-line-dialog.html) | 0.001 % | 0.001 % | pass |
| 630 | [prod-budget-edit-plan-dialog](light/630-prod-budget-edit-plan-dialog.html) | 0.001 % | 0.001 % | pass |
| 631 | [prod-budget-edit-plan-error](light/631-prod-budget-edit-plan-error.html) | 0.093 % | 0.091 % | pass |
| 632 | [prod-budget-snapshot-name-dialog](light/632-prod-budget-snapshot-name-dialog.html) | 0.001 % | 0.001 % | pass |
| 633 | [prod-budget-snapshots-saved](light/633-prod-budget-snapshots-saved.html) | 0.005 % | 0.003 % | pass |
| 634 | [prod-budget-snapshot-view](light/634-prod-budget-snapshot-view.html) | 0.002 % | 0.002 % | pass |
| 635 | [prod-budget-export-dialog](light/635-prod-budget-export-dialog.html) | 0.001 % | 0.001 % | pass |
| 636 | [cs-list](light/636-cs-list.html) | 0.000 % | 0.000 % | pass |
| 637 | [cs-row-context-menu](light/637-cs-row-context-menu.html) | 0.000 % | 0.000 % | pass |
| 638 | [cs-row-actions-menu](light/638-cs-row-actions-menu.html) | 0.000 % | 0.000 % | pass |
| 639 | [cs-search](light/639-cs-search.html) | 0.000 % | 0.000 % | pass |
| 640 | [cs-search-no-match](light/640-cs-search-no-match.html) | 0.000 % | 0.000 % | pass |
| 641 | [cs-filter-draft](light/641-cs-filter-draft.html) | 0.000 % | 0.000 % | pass |
| 642 | [cs-filter-final-none](light/642-cs-filter-final-none.html) | 0.000 % | 0.000 % | pass |
| 643 | [cs-create-dialog](light/643-cs-create-dialog.html) | 0.000 % | 0.001 % | pass |
| 644 | [cs-create-dialog-day-chosen](light/644-cs-create-dialog-day-chosen.html) | 0.000 % | 0.001 % | pass |
| 645 | [cs-editor-draft](light/645-cs-editor-draft.html) | 0.007 % | 0.007 % | pass |
| 646 | [cs-editor-draft-lower](light/646-cs-editor-draft-lower.html) | 0.007 % | 0.006 % | pass |
| 647 | [cs-add-section-menu](light/647-cs-add-section-menu.html) | 0.014 % | 0.012 % | pass |
| 648 | [cs-section-weather](light/648-cs-section-weather.html) | 0.007 % | 0.006 % | pass |
| 649 | [cs-section-extras](light/649-cs-section-extras.html) | 0.362 % | 0.360 % | pass |
| 650 | [cs-section-departments](light/650-cs-section-departments.html) | 0.338 % | 0.004 % | pass |
| 651 | [cs-section-advance](light/651-cs-section-advance.html) | 0.145 % | 0.004 % | pass |
| 652 | [cs-section-crew](light/652-cs-section-crew.html) | 0.145 % | 0.004 % | pass |
| 653 | [cs-section-communication](light/653-cs-section-communication.html) | 0.004 % | 0.004 % | pass |
| 654 | [cs-section-special-notes](light/654-cs-section-special-notes.html) | 0.004 % | 0.004 % | pass |
| 655 | [cs-section-attachments](light/655-cs-section-attachments.html) | 0.004 % | 0.004 % | pass |
| 656 | [cs-section-reference-images](light/656-cs-section-reference-images.html) | 0.004 % | 0.004 % | pass |
| 657 | [cs-section-custom-field](light/657-cs-section-custom-field.html) | 0.004 % | 0.004 % | pass |
| 658 | [cs-finalize-dialog-missing-crew-call](light/658-cs-finalize-dialog-missing-crew-call.html) | 0.006 % | 0.005 % | pass |
| 659 | [cs-finalize-crew-call-required](light/659-cs-finalize-crew-call-required.html) | 0.013 % | 0.013 % | pass |
| 660 | [cs-needs-refresh-banner](light/660-cs-needs-refresh-banner.html) | 0.016 % | 0.016 % | pass |
| 661 | [cs-finalize-stale-dialog](light/661-cs-finalize-stale-dialog.html) | 0.014 % | 0.012 % | pass |
| 662 | [cs-refresh-dialog](light/662-cs-refresh-dialog.html) | 0.003 % | 0.002 % | pass |
| 663 | [cs-refreshed](light/663-cs-refreshed.html) | 0.007 % | 0.007 % | pass |
| 664 | [cs-refresh-dialog-no-changes](light/664-cs-refresh-dialog-no-changes.html) | 0.003 % | 0.002 % | pass |
| 665 | [cs-editor-export-dialog](light/665-cs-editor-export-dialog.html) | 0.005 % | 0.004 % | pass |
| 666 | [cs-finalize-dialog](light/666-cs-finalize-dialog.html) | 0.009 % | 0.007 % | pass |
| 667 | [cs-final](light/667-cs-final.html) | 0.003 % | 0.003 % | pass |
| 668 | [cs-final-schedule-changed](light/668-cs-final-schedule-changed.html) | 0.013 % | 0.012 % | pass |
| 669 | [cs-new-revision-dialog](light/669-cs-new-revision-dialog.html) | 0.001 % | 0.001 % | pass |
| 670 | [cs-revision-v2](light/670-cs-revision-v2.html) | 0.007 % | 0.007 % | pass |
| 671 | [cs-replaced-by-newer-revision](light/671-cs-replaced-by-newer-revision.html) | 0.003 % | 0.003 % | pass |
| 672 | [cs-list-with-revisions](light/672-cs-list-with-revisions.html) | 0.246 % | 0.241 % | pass |
| 673 | [cs-list-show-replaced](light/673-cs-list-show-replaced.html) | 0.013 % | 0.014 % | pass |
| 674 | [cs-filter-needs-refresh](light/674-cs-filter-needs-refresh.html) | 0.000 % | 0.000 % | pass |
| 675 | [cs-filter-final](light/675-cs-filter-final.html) | 0.000 % | 0.000 % | pass |
| 676 | [cs-list-export-dialog](light/676-cs-list-export-dialog.html) | 0.000 % | 0.000 % | pass |
| 677 | [cs-create-dialog-every-day-done](light/677-cs-create-dialog-every-day-done.html) | 0.000 % | 0.000 % | pass |
| 678 | [prod-schedule-day-deleted](light/678-prod-schedule-day-deleted.html) | 0.015 % | 0.014 % | pass |
| 679 | [cs-list-day-deleted](light/679-cs-list-day-deleted.html) | 0.000 % | 0.000 % | pass |
| 680 | [cs-day-deleted-banner](light/680-cs-day-deleted-banner.html) | 0.007 % | 0.007 % | pass |
| 681 | [cs-deleted-toast](light/681-cs-deleted-toast.html) | 0.000 % | 0.000 % | pass |
| 682 | [prod-schedule-empty-no-source](light/682-prod-schedule-empty-no-source.html) | 0.000 % | 0.000 % | pass |
| 683 | [prod-dood-empty](light/683-prod-dood-empty.html) | 0.000 % | 0.000 % | pass |
| 684 | [prod-daily-empty](light/684-prod-daily-empty.html) | 0.000 % | 0.000 % | pass |
| 685 | [prod-sides-empty](light/685-prod-sides-empty.html) | 0.000 % | 0.000 % | pass |
| 686 | [prod-reports-empty](light/686-prod-reports-empty.html) | 0.001 % | 0.001 % | pass |
| 687 | [prod-budget-start](light/687-prod-budget-start.html) | 0.000 % | 0.000 % | pass |
| 688 | [cs-list-empty](light/688-cs-list-empty.html) | 0.000 % | 0.000 % | pass |
| 689 | [cs-create-dialog-no-schedule](light/689-cs-create-dialog-no-schedule.html) | 0.000 % | 0.000 % | pass |
| 690 | [files-list](light/690-files-list.html) | 0.000 % | 0.000 % | pass |
| 691 | [files-grid](light/691-files-grid.html) | 0.000 % | 0.000 % | pass |
| 692 | [files-folder-contracts](light/692-files-folder-contracts.html) | 0.000 % | 0.000 % | pass |
| 693 | [files-folder-top-level](light/693-files-folder-top-level.html) | 0.000 % | 0.000 % | pass |
| 694 | [files-search-results](light/694-files-search-results.html) | 0.003 % | 0.003 % | pass |
| 695 | [files-search-no-match](light/695-files-search-no-match.html) | 0.003 % | 0.003 % | pass |
| 696 | [files-add-menu](light/696-files-add-menu.html) | 0.000 % | 0.000 % | pass |
| 697 | [files-row-menu](light/697-files-row-menu.html) | 0.000 % | 0.000 % | pass |
| 698 | [files-row-context-menu](light/698-files-row-context-menu.html) | 0.001 % | 0.001 % | pass |
| 699 | [files-details-drawer](light/699-files-details-drawer.html) | 0.005 % | 0.005 % | pass |
| 700 | [files-details-notes](light/700-files-details-notes.html) | 0.005 % | 0.005 % | pass |
| 701 | [files-details-renamed](light/701-files-details-renamed.html) | 0.005 % | 0.005 % | pass |
| 702 | [files-file-deleted-toast](light/702-files-file-deleted-toast.html) | 0.000 % | 0.000 % | pass |
| 703 | [files-linked-file](light/703-files-linked-file.html) | 0.000 % | 0.000 % | pass |
| 704 | [files-unavailable-linked-file](light/704-files-unavailable-linked-file.html) | 0.000 % | 0.000 % | pass |
| 705 | [files-unavailable-details](light/705-files-unavailable-details.html) | 0.005 % | 0.005 % | pass |
| 706 | [files-linked-file-deleted-toast](light/706-files-linked-file-deleted-toast.html) | 0.000 % | 0.000 % | pass |
| 707 | [files-new-folder-dialog](light/707-files-new-folder-dialog.html) | 0.000 % | 0.000 % | pass |
| 708 | [files-new-folder-filled](light/708-files-new-folder-filled.html) | 0.195 % | 0.197 % | pass |
| 709 | [files-folder-created](light/709-files-folder-created.html) | 0.000 % | 0.000 % | pass |
| 710 | [files-rename-folder-dialog](light/710-files-rename-folder-dialog.html) | 0.000 % | 0.000 % | pass |
| 711 | [files-delete-folder-confirm](light/711-files-delete-folder-confirm.html) | 0.000 % | 0.000 % | pass |
| 712 | [files-folder-deleted-toast](light/712-files-folder-deleted-toast.html) | 0.000 % | 0.000 % | pass |
| 713 | [files-empty](light/713-files-empty.html) | 0.000 % | 0.000 % | pass |
| 714 | [notes-empty](light/714-notes-empty.html) | 0.000 % | 0.000 % | pass |
| 715 | [activity-new-project](light/715-activity-new-project.html) | 0.001 % | 0.001 % | pass |
| 716 | [trash-empty](light/716-trash-empty.html) | 0.000 % | 0.000 % | pass |
| 717 | [notes-filled](light/717-notes-filled.html) | 0.000 % | 0.000 % | pass |
| 718 | [notes-search-filtered](light/718-notes-search-filtered.html) | 0.003 % | 0.003 % | pass |
| 719 | [notes-search-no-match](light/719-notes-search-no-match.html) | 0.003 % | 0.003 % | pass |
| 720 | [notes-note-editor](light/720-notes-note-editor.html) | 0.007 % | 0.006 % | pass |
| 721 | [notes-note-editor-pinned](light/721-notes-note-editor-pinned.html) | 0.007 % | 0.006 % | pass |
| 722 | [notes-new-note-editor](light/722-notes-new-note-editor.html) | 0.007 % | 0.007 % | pass |
| 723 | [notes-new-note-filled](light/723-notes-new-note-filled.html) | 0.007 % | 0.007 % | pass |
| 724 | [notes-note-deleted-toast](light/724-notes-note-deleted-toast.html) | 0.000 % | 0.000 % | pass |
| 725 | [notes-task-editor-new](light/725-notes-task-editor-new.html) | 0.163 % | 0.194 % | pass |
| 726 | [notes-task-editor-title-error](light/726-notes-task-editor-title-error.html) | 0.132 % | 0.130 % | pass |
| 727 | [notes-task-editor-filled](light/727-notes-task-editor-filled.html) | 0.000 % | 0.000 % | pass |
| 728 | [notes-task-editor-related-results](light/728-notes-task-editor-related-results.html) | 0.003 % | 0.003 % | pass |
| 729 | [notes-task-editor-related-chosen](light/729-notes-task-editor-related-chosen.html) | 0.000 % | 0.000 % | pass |
| 730 | [notes-task-added-toast](light/730-notes-task-added-toast.html) | 0.003 % | 0.002 % | pass |
| 731 | [notes-task-editor-existing](light/731-notes-task-editor-existing.html) | 0.001 % | 0.001 % | pass |
| 732 | [notes-tasks-done](light/732-notes-tasks-done.html) | 0.001 % | 0.001 % | pass |
| 733 | [notes-tasks-all](light/733-notes-tasks-all.html) | 0.003 % | 0.002 % | pass |
| 734 | [notes-task-completed-toast](light/734-notes-task-completed-toast.html) | 0.000 % | 0.000 % | pass |
| 735 | [notes-task-deleted-toast](light/735-notes-task-deleted-toast.html) | 0.000 % | 0.000 % | pass |
| 736 | [activity-all](light/736-activity-all.html) | 0.011 % | 0.010 % | pass |
| 737 | [activity-filter-drafts](light/737-activity-filter-drafts.html) | 0.001 % | 0.001 % | pass |
| 738 | [activity-filter-story](light/738-activity-filter-story.html) | 0.009 % | 0.007 % | pass |
| 739 | [activity-filter-production](light/739-activity-filter-production.html) | 0.014 % | 0.013 % | pass |
| 740 | [activity-filter-exchange](light/740-activity-filter-exchange.html) | 0.000 % | 0.000 % | pass |
| 741 | [trash-filled](light/741-trash-filled.html) | 0.007 % | 0.005 % | pass |
| 742 | [trash-search-filtered](light/742-trash-search-filtered.html) | 0.006 % | 0.007 % | pass |
| 743 | [trash-search-no-match](light/743-trash-search-no-match.html) | 0.003 % | 0.003 % | pass |
| 744 | [trash-purge-confirm](light/744-trash-purge-confirm.html) | 0.003 % | 0.005 % | pass |
| 745 | [trash-global-empty](light/745-trash-global-empty.html) | 0.000 % | 0.000 % | pass |
| 746 | [trash-global-filled](light/746-trash-global-filled.html) | 0.000 % | 0.000 % | pass |
| 747 | [trash-global-purge-confirm](light/747-trash-global-purge-confirm.html) | 0.002 % | 0.001 % | pass |
| 748 | [trash-purged-toast](light/748-trash-purged-toast.html) | 0.000 % | 0.000 % | pass |
| 749 | [templates-page](light/749-templates-page.html) | 0.003 % | 0.003 % | pass |
| 750 | [templates-actions-menu-builtin](light/750-templates-actions-menu-builtin.html) | 0.002 % | 0.002 % | pass |
| 751 | [templates-preview-dialog](light/751-templates-preview-dialog.html) | 0.001 % | 0.000 % | pass |
| 752 | [templates-create-dialog](light/752-templates-create-dialog.html) | 0.001 % | 0.000 % | pass |
| 753 | [templates-create-name-error](light/753-templates-create-name-error.html) | 0.107 % | 0.106 % | pass |
| 754 | [templates-create-filled](light/754-templates-create-filled.html) | 0.001 % | 0.000 % | pass |
| 755 | [templates-created-toast](light/755-templates-created-toast.html) | 0.003 % | 0.003 % | pass |
| 756 | [templates-actions-menu-own](light/756-templates-actions-menu-own.html) | 0.006 % | 0.005 % | pass |
| 757 | [templates-edit-dialog](light/757-templates-edit-dialog.html) | 0.001 % | 0.000 % | pass |
| 758 | [templates-apply-dialog](light/758-templates-apply-dialog.html) | 0.001 % | 0.000 % | pass |
| 759 | [templates-delete-confirm](light/759-templates-delete-confirm.html) | 0.001 % | 0.000 % | pass |
| 760 | [templates-global-deleted-toast](light/760-templates-global-deleted-toast.html) | 0.003 % | 0.003 % | pass |
| 761 | [templates-project-deleted-toast](light/761-templates-project-deleted-toast.html) | 0.003 % | 0.003 % | pass |
| 762 | [settings-general](light/762-settings-general.html) | 0.002 % | 0.002 % | pass |
| 763 | [settings-general-dirty](light/763-settings-general-dirty.html) | 0.002 % | 0.002 % | pass |
| 764 | [settings-general-title-error](light/764-settings-general-title-error.html) | 0.002 % | 0.002 % | pass |
| 765 | [settings-general-lower](light/765-settings-general-lower.html) | 0.002 % | 0.002 % | pass |
| 766 | [settings-export-diagnostics-dialog](light/766-settings-export-diagnostics-dialog.html) | 0.000 % | 0.001 % | pass |
| 767 | [settings-export-diagnostics-contents](light/767-settings-export-diagnostics-contents.html) | 0.000 % | 0.001 % | pass |
| 768 | [settings-writing](light/768-settings-writing.html) | 0.000 % | 0.000 % | pass |
| 769 | [settings-writing-word-added](light/769-settings-writing-word-added.html) | 0.000 % | 0.000 % | pass |
| 770 | [settings-writing-dictionary-filled](light/770-settings-writing-dictionary-filled.html) | 0.000 % | 0.000 % | pass |
| 771 | [settings-writing-dictionary-filtered](light/771-settings-writing-dictionary-filtered.html) | 0.000 % | 0.000 % | pass |
| 772 | [settings-writing-word-removed-toast](light/772-settings-writing-word-removed-toast.html) | 0.000 % | 0.000 % | pass |
| 773 | [settings-writing-spelling-toggled](light/773-settings-writing-spelling-toggled.html) | 0.003 % | 0.003 % | pass |
| 774 | [settings-writing-word-error](light/774-settings-writing-word-error.html) | 0.116 % | 0.000 % | pass |
| 775 | [settings-shortcuts](light/775-settings-shortcuts.html) | 0.014 % | 0.003 % | pass |
| 776 | [settings-shortcuts-filtered](light/776-settings-shortcuts-filtered.html) | 0.004 % | 0.003 % | pass |
| 777 | [settings-shortcuts-no-match](light/777-settings-shortcuts-no-match.html) | 0.000 % | 0.000 % | pass |
| 778 | [settings-shortcut-recording](light/778-settings-shortcut-recording.html) | 0.023 % | 0.010 % | pass |
| 779 | [settings-shortcut-invalid-plain-key](light/779-settings-shortcut-invalid-plain-key.html) | 0.075 % | 0.071 % | pass |
| 780 | [settings-shortcut-invalid-reserved](light/780-settings-shortcut-invalid-reserved.html) | 0.059 % | 0.055 % | pass |
| 781 | [settings-shortcut-duplicate-error](light/781-settings-shortcut-duplicate-error.html) | 0.086 % | 0.004 % | pass |
| 782 | [settings-shortcut-changed](light/782-settings-shortcut-changed.html) | 0.084 % | 0.003 % | pass |
| 783 | [settings-shortcut-conflict-dialog](light/783-settings-shortcut-conflict-dialog.html) | 0.000 % | 0.000 % | pass |
| 784 | [settings-shortcut-moved](light/784-settings-shortcut-moved.html) | 0.009 % | 0.006 % | pass |
| 785 | [settings-shortcut-restore-conflict-dialog](light/785-settings-shortcut-restore-conflict-dialog.html) | 0.001 % | 0.000 % | pass |
| 786 | [settings-shortcut-restore-all-confirm](light/786-settings-shortcut-restore-all-confirm.html) | 0.001 % | 0.000 % | pass |
| 787 | [settings-shortcuts-restored-toast](light/787-settings-shortcuts-restored-toast.html) | 0.014 % | 0.003 % | pass |
| 788 | [settings-shortcuts-conflict-banner](light/788-settings-shortcuts-conflict-banner.html) | 0.009 % | 0.007 % | pass |
| 789 | [settings-offline-ai-ready](light/789-settings-offline-ai-ready.html) | 0.002 % | 0.002 % | pass |
| 790 | [settings-offline-ai-remove-confirm](light/790-settings-offline-ai-remove-confirm.html) | 0.001 % | 0.001 % | pass |
| 791 | [settings-offline-ai-technical-details](light/791-settings-offline-ai-technical-details.html) | 0.030 % | 0.025 % | pass |
| 792 | [settings-offline-ai-stopped-toast](light/792-settings-offline-ai-stopped-toast.html) | 0.242 % | 0.025 % | pass |
| 793 | [settings-offline-ai-graphics-off](light/793-settings-offline-ai-graphics-off.html) | 0.030 % | 0.025 % | pass |
| 794 | [settings-advanced](light/794-settings-advanced.html) | 0.000 % | 0.000 % | pass |
| 795 | [settings-advanced-changed-toast](light/795-settings-advanced-changed-toast.html) | 0.000 % | 0.000 % | pass |
| 796 | [settings-advanced-restored-toast](light/796-settings-advanced-restored-toast.html) | 0.000 % | 0.000 % | pass |
| 797 | [settings-appearance](light/797-settings-appearance.html) | 0.000 % | 0.000 % | pass |
| 798 | [settings-appearance-my-themes-empty](light/798-settings-appearance-my-themes-empty.html) | 0.000 % | 0.000 % | pass |
| 799 | [settings-theme-paste-dialog](light/799-settings-theme-paste-dialog.html) | 0.000 % | 0.000 % | pass |
| 800 | [settings-theme-too-large](light/800-settings-theme-too-large.html) | 0.000 % | 0.000 % | pass |
| 801 | [settings-theme-paste-filled](light/801-settings-theme-paste-filled.html) | 0.000 % | 0.000 % | pass |
| 802 | [settings-theme-check-corrected](light/802-settings-theme-check-corrected.html) | 0.001 % | 0.000 % | pass |
| 803 | [settings-theme-check-readability](light/803-settings-theme-check-readability.html) | 0.001 % | 0.000 % | pass |
| 804 | [settings-theme-check-rejected](light/804-settings-theme-check-rejected.html) | 0.000 % | 0.000 % | pass |
| 805 | [settings-theme-check-ok](light/805-settings-theme-check-ok.html) | 0.004 % | 0.003 % | pass |
| 806 | [settings-my-themes-filled](light/806-settings-my-themes-filled.html) | 0.030 % | 0.030 % | pass |
| 807 | [settings-my-themes-rename-dialog](light/807-settings-my-themes-rename-dialog.html) | 0.000 % | 0.000 % | pass |
| 808 | [settings-my-themes-delete-confirm](light/808-settings-my-themes-delete-confirm.html) | 0.000 % | 0.000 % | pass |
| 809 | [settings-my-themes-deleted-toast](light/809-settings-my-themes-deleted-toast.html) | 0.030 % | 0.030 % | pass |

## Not captured

States the app cannot reach from inside the web view, or that need hardware or a failure on purpose:

- **Windows Open, Save and folder dialogs**: They belong to Windows, not the web view (Import, Export, Add File, Browse, Locate, backups and packages).
- **Drag and drop states**: Dragging files from Explorer and the drag previews on the Beat Board, schedule and Idea Vault need a real mouse drag across windows.
- **Microphone recording**: Recording a voice note needs a microphone; the Record Note dialog is captured only where the tour can reach it.
- **Printing**: The Windows print dialog opened by Print in Print Preview.
- **Explorer and other apps**: Reveal in File Manager and Open externally open other programs.
- **Failure screens that need a broken disk or a crash**: Save errors, the leave guard, the recovery offer after a forced quit, error boundaries and list load errors.

States the tour recorded as unreachable:

- **shell-save-status-saving** (10-shell): The amber Saving state lasts a fraction of a second after a change and cannot be held for a screenshot.
- **shell-recovery-offer** (10-shell): The offer to open a recovery state only appears after the app was killed in the middle of a session.
- **shell-save-error-banner** (10-shell): Needs the disk write to fail (a read-only or removed drive).
- **shell-save-error-dialog** (10-shell): Needs the disk write to fail (a read-only or removed drive).
- **shell-leave-guard** (10-shell): Needs unsaved changes after a failed save, which needs the disk write to fail.
- **shell-workspace-error-boundary** (10-shell): Only appears after a view crashes; the app has no way to force that.
- **shell-open-project-location** (10-shell): Opens File Explorer outside the app window.
- **apphome-open-project-folder-picker** (95-app-home): Open Project opens the native Windows folder picker, which is outside the app window.
- **apphome-card-unavailable** (95-app-home): Needs a project whose folder was moved or deleted outside the app.
- **apphome-unavailable-dialog** (95-app-home): Needs a project whose folder was moved or deleted outside the app.
- **apphome-load-error** (95-app-home): The project list loads from the local database, which cannot be made to fail on demand.
- **apphome-new-project-cloud-warning** (95-app-home): Needs the projects folder to sit inside OneDrive, Dropbox or Google Drive.
- **apphome-new-project-failed** (95-app-home): Needs a parent folder that refuses writes.
- **apphome-new-project-location-picker** (95-app-home): Change... opens the native Windows folder picker.
- **vault-record-note-saved** (20-vault): needs a real recording; the test window has no microphone input
- **vault-add-files-native-picker** (20-vault): Add > Image, File, Audio file and Video open the Windows file picker, which a screenshot tour cannot drive
- **vault-add-result-dialog** (20-vault): shown only after the native picker adds a file that cannot be read
- **vault-drop-overlay** (20-vault): appears while files are dragged in from Explorer, an OS drag that WebDriver cannot start
- **story-card-drawer-linked** (30-story): the banner Used to create screenplay Scene n appears only after a card is built into a screenplay, which would add a screenplay the other sections do not expect
- **story-delete-linked-card-dialog** (30-story): needs a card that was built into a screenplay scene, which would add a screenplay the other sections do not expect
- **sp-compare-no-differences** (40-screenplay): Needs two identical drafts; the demo drafts differ, and making a copy would leave an extra deleted draft in Recently Deleted.
- **sp-print-os-dialog** (40-screenplay): Print opens the Windows print dialog, which is outside the app window and cannot be captured.
- **sp-export-save-dialog** (40-screenplay): Export… opens the native Save dialog (Export Screenplay), which is outside the web page.
- **sp-export-complete** (40-screenplay): The Export complete screen comes only after the native Save dialog is answered.
- **sp-import-file-picker** (40-screenplay): Browse… in Import Screenplay opens the native file dialog (Choose a screenplay to import).
- **ai-scope-options-open** (85-ai-packages): The Scope list is a native select; its drop-down is drawn by Windows outside the page and does not appear in a page capture. Each scope is captured as the chosen value instead
- **ai-offline-download-states** (85-ai-packages): The download, paused, failed, not enough space and damaged states of OpenFrame AI need the model host steered through a failed or interrupted download; this tour runs with the models installed
- **ai-offline-not-downloaded** (85-ai-packages): Needs a run without the AI models installed; this tour runs with them
- **ai-export-report** (85-ai-packages): Export AI report opens the native save dialog; the saved toast needs a file chosen there
- **ai-runtime-failed** (85-ai-packages): The runtime failed banner needs the AI engine to crash mid-answer (covered by the crash-safe apply spec, not by a capture)
- **pkg-open-review-package** (85-ai-packages): Open Review Package opens the Windows file picker straight away, with no dialog of its own first; the viewer needs a .ofscriptreview file chosen in that native picker
- **pkg-backup-save-location** (85-ai-packages): Browse in Create Backup opens the native save dialog (Save backup to); the location, progress, done and cancelled states need a file chosen there
- **pkg-export-project-save-location** (85-ai-packages): Export Project Package opens the native save dialog; the progress and Project package exported states need a file chosen there
- **pkg-export-package-save-location** (85-ai-packages): Export Package opens the native save dialog; the Package exported state needs a file chosen there
- **pkg-import-exchange-review** (85-ai-packages): Choose Package opens the native open dialog; the preview, stale-package choice and result need a package from a second project
- **pkg-open-project-choose-file** (85-ai-packages): Choose Package opens the native open dialog; the package details, collision choice and Back Up and Replace confirmation need a package chosen there
- **licence-mac-first-launch** (85-ai-packages): The Mac first-launch screen only exists on the Mac App Store channel, which is a Mac shell; it is captured on the Mac track
- **prod-catalog-add-image** (60-production-plan): Add Image opens a native image picker
- **prod-location-add-photos** (60-production-plan): Add Photos opens a native image picker
- **prod-cast-add-photo** (60-production-plan): Add Photo opens a native image picker
- **prod-moodboard-add-image-from-file** (60-production-plan): From file opens a native image picker
- **prod-storyboard-import-image** (60-production-plan): Import Image opens a native image picker
- **prod-shot-reference-image** (60-production-plan): Add reference image opens a native image picker
- **prod-schedule-script-changes** (70-production-schedule): the banner and its dialog appear only after the Production Source script is edited; editing it would change the shared Night Bus script for every other section
- **prod-schedule-with-source-no-schedule** (70-production-schedule): needs a project that has a Production Source but no shooting schedule; both demo projects have one or none
- **prod-budget-review-banner** (70-production-schedule): the budget showed no review reminder after the schedule changes in this section
- **files-add-copy-dialog** (80-pages): Add File opens the Windows file picker (native dialog), which the WebDriver session cannot see or drive.
- **files-link-dialog** (80-pages): Link to file opens the Windows file picker (native dialog).
- **files-export-copy-dialog** (80-pages): Export Copy opens the Windows Save dialog (native dialog).
- **files-drop-overlay** (80-pages): The drop overlay needs a real drag of files from Explorer into the window (OS drag and drop).
- **files-reveal-in-file-manager** (80-pages): Reveal in File Manager and Open start another Windows application outside the window.
- **activity-empty-all** (80-pages): The unfiltered "No major activity yet." never shows: creating any project already logs an entry. The empty look is captured by activity-filter-exchange.
- **activity-show-older** (80-pages): Show older activity appears only after 200 or more entries, more than the demo project holds.
- **settings-export-diagnostics-complete** (80-pages): Export saves through the Windows Save dialog (native dialog) before the Export complete screen appears.
- **settings-offline-ai-not-downloaded** (80-pages): Needs the AI models removed from this computer (spec header e2e-ai-models: none); the tour runs with the models present.
- **settings-offline-ai-download-states** (80-pages): Downloading, paused, failed, not enough space, damaged and cannot-download need the test model host steered to fail or stall, which the tour does not run.
- **settings-offline-ai-health-filled** (80-pages): The AI health table fills only after questions were asked in the AI Assistant, which happens after this section in the tour.
- **settings-offline-ai-export-report** (80-pages): Export AI report opens the Windows Save dialog (native dialog).
- **settings-theme-import-file** (80-pages): Import Theme File opens the Windows file picker (native dialog).
- **settings-my-themes-export** (80-pages): Export of a theme opens the Windows Save dialog (native dialog).
- **settings-theme-fallback-banner** (80-pages): Shown only when the saved theme has disappeared from the app database, which cannot be arranged from the window.
