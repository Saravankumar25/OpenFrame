# WindowsHTML

Every screen and state of OpenFrame Studio for Windows, captured from the running app as standalone HTML pages,
in Light and Dark mode of the default theme. It is the source for the website's screen recreations (Chapter 2).

- **819 screens and states**, each in `light/` and `dark/` under the same `NNN-name.html`.
- Captured from commit `269f68ab` at a 1600x1000 window (the pages are laid out for that size).
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
E2E_CAPTURE=1 OPENFRAME_LIBRARY_SETUP=1 node tests/e2e/run.mjs --only=zz-capture --no-build
```

The run replaces `light/`, `dark/`, `assets/`, `index.html` and this file. Raw screenshots stay in
`.build-cache/windows-html-capture/` (not committed; 2400x1500 pixels each at 150 percent display scaling).

## Check

Every page was opened in the same WebView2 at the same size and compared with its screenshot (the visual diff
helper of `90-visual-baseline`): a pixel differs when a colour channel differs by more than 24 of 255, and a
page fails above 0.5 % differing pixels. 1638 of 1638 pages passed; the
largest difference was 0.461 %.

| Section | Screens |
|---|---|
| seed | 15 |
| 10-shell | 38 |
| 95-app-home | 41 |
| 20-vault | 78 |
| 30-story | 80 |
| 40-screenplay | 96 |
| 50-breakdown | 35 |
| 85-ai-packages | 60 |
| 60-production-plan | 99 |
| 70-production-schedule | 157 |
| 80-pages | 120 |

| No. | Screen | Light | Dark | Result |
|---|---|---|---|---|
| 001 | [app-library-setup-default](light/001-app-library-setup-default.html) | 0.000 % | 0.000 % | pass |
| 002 | [app-library-setup-new-location](light/002-app-library-setup-new-location.html) | 0.000 % | 0.000 % | pass |
| 003 | [app-library-setup-locate](light/003-app-library-setup-locate.html) | 0.000 % | 0.000 % | pass |
| 004 | [app-first-launch-welcome](light/004-app-first-launch-welcome.html) | 0.000 % | 0.000 % | pass |
| 005 | [app-first-launch-name-required](light/005-app-first-launch-name-required.html) | 0.000 % | 0.000 % | pass |
| 006 | [app-home-empty](light/006-app-home-empty.html) | 0.000 % | 0.000 % | pass |
| 007 | [app-new-project-dialog](light/007-app-new-project-dialog.html) | 0.000 % | 0.000 % | pass |
| 008 | [app-new-project-title-required](light/008-app-new-project-title-required.html) | 0.000 % | 0.000 % | pass |
| 009 | [app-new-project-filled](light/009-app-new-project-filled.html) | 0.000 % | 0.000 % | pass |
| 010 | [app-new-project-optional-details](light/010-app-new-project-optional-details.html) | 0.000 % | 0.000 % | pass |
| 011 | [home-new-project](light/011-home-new-project.html) | 0.000 % | 0.001 % | pass |
| 012 | [sp-import-source](light/012-sp-import-source.html) | 0.000 % | 0.000 % | pass |
| 013 | [sp-import-sample-chosen](light/013-sp-import-sample-chosen.html) | 0.135 % | 0.133 % | pass |
| 014 | [sp-import-preview](light/014-sp-import-preview.html) | 0.001 % | 0.001 % | pass |
| 015 | [sp-import-complete](light/015-sp-import-complete.html) | 0.003 % | 0.003 % | pass |
| 016 | [shell-project-home](light/016-shell-project-home.html) | 0.000 % | 0.000 % | pass |
| 017 | [shell-project-menu](light/017-shell-project-menu.html) | 0.000 % | 0.000 % | pass |
| 018 | [shell-create-menu](light/018-shell-create-menu.html) | 0.000 % | 0.000 % | pass |
| 019 | [shell-nav-collapsed](light/019-shell-nav-collapsed.html) | 0.000 % | 0.000 % | pass |
| 020 | [shell-nav-collapsed-files](light/020-shell-nav-collapsed-files.html) | 0.000 % | 0.000 % | pass |
| 021 | [shell-search-empty](light/021-shell-search-empty.html) | 0.000 % | 0.000 % | pass |
| 022 | [shell-search-results](light/022-shell-search-results.html) | 0.303 % | 0.362 % | pass |
| 023 | [shell-search-results-second-active](light/023-shell-search-results-second-active.html) | 0.303 % | 0.362 % | pass |
| 024 | [shell-search-results-location](light/024-shell-search-results-location.html) | 0.314 % | 0.372 % | pass |
| 025 | [shell-search-no-matches](light/025-shell-search-no-matches.html) | 0.309 % | 0.367 % | pass |
| 026 | [shell-search-global-scope-empty](light/026-shell-search-global-scope-empty.html) | 0.000 % | 0.000 % | pass |
| 027 | [shell-search-global-scope](light/027-shell-search-global-scope.html) | 0.303 % | 0.362 % | pass |
| 028 | [shell-status-menu](light/028-shell-status-menu.html) | 0.002 % | 0.002 % | pass |
| 029 | [shell-toast-undoable](light/029-shell-toast-undoable.html) | 0.000 % | 0.000 % | pass |
| 030 | [shell-project-menu-undo-enabled](light/030-shell-project-menu-undo-enabled.html) | 0.000 % | 0.000 % | pass |
| 031 | [shell-toast-undid](light/031-shell-toast-undid.html) | 0.000 % | 0.000 % | pass |
| 032 | [shell-project-menu-redo-enabled](light/032-shell-project-menu-redo-enabled.html) | 0.000 % | 0.000 % | pass |
| 033 | [shell-status-panel](light/033-shell-status-panel.html) | 0.000 % | 0.000 % | pass |
| 034 | [shell-open-recovery-no-checkpoint](light/034-shell-open-recovery-no-checkpoint.html) | 0.002 % | 0.002 % | pass |
| 035 | [shell-toast-saved](light/035-shell-toast-saved.html) | 0.000 % | 0.000 % | pass |
| 036 | [shell-open-recovery-with-checkpoint](light/036-shell-open-recovery-with-checkpoint.html) | 0.001 % | 0.000 % | pass |
| 037 | [shell-display-name-dialog](light/037-shell-display-name-dialog.html) | 0.000 % | 0.000 % | pass |
| 038 | [shell-display-name-empty-error](light/038-shell-display-name-empty-error.html) | 0.000 % | 0.000 % | pass |
| 039 | [shell-status-menu-before-archive](light/039-shell-status-menu-before-archive.html) | 0.002 % | 0.002 % | pass |
| 040 | [shell-toast-archived](light/040-shell-toast-archived.html) | 0.000 % | 0.000 % | pass |
| 041 | [shell-archived-banner](light/041-shell-archived-banner.html) | 0.000 % | 0.000 % | pass |
| 042 | [shell-status-menu-archived](light/042-shell-status-menu-archived.html) | 0.000 % | 0.000 % | pass |
| 043 | [shell-toast-restored](light/043-shell-toast-restored.html) | 0.000 % | 0.000 % | pass |
| 044 | [shell-project-settings-dialog](light/044-shell-project-settings-dialog.html) | 0.000 % | 0.000 % | pass |
| 045 | [shell-project-settings-edited](light/045-shell-project-settings-edited.html) | 0.280 % | 0.285 % | pass |
| 046 | [shell-project-settings-title-required](light/046-shell-project-settings-title-required.html) | 0.000 % | 0.000 % | pass |
| 047 | [shell-back-enabled](light/047-shell-back-enabled.html) | 0.000 % | 0.000 % | pass |
| 048 | [shell-back-and-forward-enabled](light/048-shell-back-and-forward-enabled.html) | 0.050 % | 0.000 % | pass |
| 049 | [shell-forward-enabled](light/049-shell-forward-enabled.html) | 0.050 % | 0.000 % | pass |
| 050 | [shell-empty-project-home](light/050-shell-empty-project-home.html) | 0.000 % | 0.000 % | pass |
| 051 | [shell-empty-project-menu-pinned](light/051-shell-empty-project-menu-pinned.html) | 0.000 % | 0.000 % | pass |
| 052 | [shell-empty-project-status-panel](light/052-shell-empty-project-status-panel.html) | 0.000 % | 0.000 % | pass |
| 053 | [shell-empty-project-settings-dialog](light/053-shell-empty-project-settings-dialog.html) | 0.000 % | 0.000 % | pass |
| 054 | [apphome-projects](light/054-apphome-projects.html) | 0.000 % | 0.000 % | pass |
| 055 | [apphome-card-menu](light/055-apphome-card-menu.html) | 0.001 % | 0.001 % | pass |
| 056 | [apphome-card-context-menu](light/056-apphome-card-context-menu.html) | 0.005 % | 0.005 % | pass |
| 057 | [apphome-rename-dialog](light/057-apphome-rename-dialog.html) | 0.000 % | 0.000 % | pass |
| 058 | [apphome-rename-empty-error](light/058-apphome-rename-empty-error.html) | 0.000 % | 0.000 % | pass |
| 059 | [apphome-rename-filled](light/059-apphome-rename-filled.html) | 0.195 % | 0.197 % | pass |
| 060 | [apphome-toast-renamed](light/060-apphome-toast-renamed.html) | 0.000 % | 0.000 % | pass |
| 061 | [apphome-delete-dialog](light/061-apphome-delete-dialog.html) | 0.000 % | 0.000 % | pass |
| 062 | [apphome-delete-typed](light/062-apphome-delete-typed.html) | 0.265 % | 0.267 % | pass |
| 063 | [apphome-pinned-section](light/063-apphome-pinned-section.html) | 0.000 % | 0.000 % | pass |
| 064 | [apphome-card-menu-pinned](light/064-apphome-card-menu-pinned.html) | 0.006 % | 0.006 % | pass |
| 065 | [apphome-all-pinned](light/065-apphome-all-pinned.html) | 0.000 % | 0.000 % | pass |
| 066 | [apphome-search-results](light/066-apphome-search-results.html) | 0.130 % | 0.152 % | pass |
| 067 | [apphome-search-no-match](light/067-apphome-search-no-match.html) | 0.156 % | 0.179 % | pass |
| 068 | [apphome-toast-archived](light/068-apphome-toast-archived.html) | 0.000 % | 0.000 % | pass |
| 069 | [apphome-search-archived-match](light/069-apphome-search-archived-match.html) | 0.184 % | 0.207 % | pass |
| 070 | [apphome-archived-view](light/070-apphome-archived-view.html) | 0.000 % | 0.000 % | pass |
| 071 | [apphome-archived-row-menu](light/071-apphome-archived-row-menu.html) | 0.000 % | 0.000 % | pass |
| 072 | [apphome-delete-dialog-archived](light/072-apphome-delete-dialog-archived.html) | 0.000 % | 0.000 % | pass |
| 073 | [apphome-archived-search-no-match](light/073-apphome-archived-search-no-match.html) | 0.153 % | 0.178 % | pass |
| 074 | [apphome-all-archived](light/074-apphome-all-archived.html) | 0.004 % | 0.004 % | pass |
| 075 | [apphome-archived-view-two](light/075-apphome-archived-view-two.html) | 0.000 % | 0.000 % | pass |
| 076 | [apphome-toast-restored](light/076-apphome-toast-restored.html) | 0.000 % | 0.000 % | pass |
| 077 | [apphome-archived-empty](light/077-apphome-archived-empty.html) | 0.000 % | 0.000 % | pass |
| 078 | [apphome-restore-package-dialog](light/078-apphome-restore-package-dialog.html) | 0.000 % | 0.000 % | pass |
| 079 | [apphome-quick-capture-dialog](light/079-apphome-quick-capture-dialog.html) | 0.000 % | 0.000 % | pass |
| 080 | [apphome-global-vault](light/080-apphome-global-vault.html) | 0.208 % | 0.000 % | pass |
| 081 | [apphome-global-vault-add-menu](light/081-apphome-global-vault-add-menu.html) | 0.000 % | 0.000 % | pass |
| 082 | [apphome-settings-appearance](light/082-apphome-settings-appearance.html) | 0.000 % | 0.000 % | pass |
| 083 | [apphome-settings-shortcuts](light/083-apphome-settings-shortcuts.html) | 0.008 % | 0.005 % | pass |
| 084 | [apphome-settings-shortcuts-filtered](light/084-apphome-settings-shortcuts-filtered.html) | 0.149 % | 0.176 % | pass |
| 085 | [apphome-settings-shortcuts-recording](light/085-apphome-settings-shortcuts-recording.html) | 0.008 % | 0.006 % | pass |
| 086 | [apphome-settings-storage](light/086-apphome-settings-storage.html) | 0.000 % | 0.000 % | pass |
| 087 | [apphome-settings-about](light/087-apphome-settings-about.html) | 0.001 % | 0.001 % | pass |
| 088 | [apphome-settings-about-support](light/088-apphome-settings-about-support.html) | 0.001 % | 0.001 % | pass |
| 089 | [apphome-display-name-dialog](light/089-apphome-display-name-dialog.html) | 0.000 % | 0.000 % | pass |
| 090 | [apphome-display-name-empty-error](light/090-apphome-display-name-empty-error.html) | 0.000 % | 0.000 % | pass |
| 091 | [apphome-new-project-dialog](light/091-apphome-new-project-dialog.html) | 0.000 % | 0.000 % | pass |
| 092 | [apphome-new-project-title-required](light/092-apphome-new-project-title-required.html) | 0.000 % | 0.000 % | pass |
| 093 | [apphome-new-project-filled](light/093-apphome-new-project-filled.html) | 0.000 % | 0.000 % | pass |
| 094 | [apphome-new-project-optional-details](light/094-apphome-new-project-optional-details.html) | 0.000 % | 0.000 % | pass |
| 095 | [vault-grid-all-items](light/095-vault-grid-all-items.html) | 0.002 % | 0.002 % | pass |
| 096 | [vault-view-card](light/096-vault-view-card.html) | 0.002 % | 0.002 % | pass |
| 097 | [vault-view-list](light/097-vault-view-list.html) | 0.000 % | 0.000 % | pass |
| 098 | [vault-view-folder](light/098-vault-view-folder.html) | 0.002 % | 0.002 % | pass |
| 099 | [vault-view-folder-open](light/099-vault-view-folder-open.html) | 0.000 % | 0.000 % | pass |
| 100 | [vault-view-pinned](light/100-vault-view-pinned.html) | 0.000 % | 0.000 % | pass |
| 101 | [vault-view-recently-added](light/101-vault-view-recently-added.html) | 0.002 % | 0.002 % | pass |
| 102 | [vault-view-recently-modified](light/102-vault-view-recently-modified.html) | 0.002 % | 0.002 % | pass |
| 103 | [vault-view-collection](light/103-vault-view-collection.html) | 0.000 % | 0.000 % | pass |
| 104 | [vault-view-tag](light/104-vault-view-tag.html) | 0.000 % | 0.000 % | pass |
| 105 | [vault-view-folder-from-panel](light/105-vault-view-folder-from-panel.html) | 0.000 % | 0.000 % | pass |
| 106 | [vault-add-menu](light/106-vault-add-menu.html) | 0.002 % | 0.002 % | pass |
| 107 | [vault-search-results](light/107-vault-search-results.html) | 0.001 % | 0.001 % | pass |
| 108 | [vault-search-no-results](light/108-vault-search-no-results.html) | 0.000 % | 0.001 % | pass |
| 109 | [vault-scope-global](light/109-vault-scope-global.html) | 0.000 % | 0.000 % | pass |
| 110 | [vault-global-pinned-empty](light/110-vault-global-pinned-empty.html) | 0.000 % | 0.000 % | pass |
| 111 | [vault-global-item-context-menu](light/111-vault-global-item-context-menu.html) | 0.000 % | 0.000 % | pass |
| 112 | [vault-copy-to-project-dialog](light/112-vault-copy-to-project-dialog.html) | 0.000 % | 0.000 % | pass |
| 113 | [vault-copy-to-project-done](light/113-vault-copy-to-project-done.html) | 0.000 % | 0.000 % | pass |
| 114 | [vault-drawer-copied-from-global](light/114-vault-drawer-copied-from-global.html) | 0.000 % | 0.000 % | pass |
| 115 | [vault-drawer-note](light/115-vault-drawer-note.html) | 0.000 % | 0.000 % | pass |
| 116 | [vault-drawer-quote](light/116-vault-drawer-quote.html) | 0.000 % | 0.000 % | pass |
| 117 | [vault-drawer-url](light/117-vault-drawer-url.html) | 0.000 % | 0.000 % | pass |
| 118 | [vault-drawer-url-invalid](light/118-vault-drawer-url-invalid.html) | 0.000 % | 0.000 % | pass |
| 119 | [vault-drawer-note-new](light/119-vault-drawer-note-new.html) | 0.000 % | 0.000 % | pass |
| 120 | [vault-item-deleted-toast](light/120-vault-item-deleted-toast.html) | 0.002 % | 0.002 % | pass |
| 121 | [vault-item-context-menu](light/121-vault-item-context-menu.html) | 0.000 % | 0.000 % | pass |
| 122 | [vault-item-context-menu-pinned](light/122-vault-item-context-menu-pinned.html) | 0.002 % | 0.002 % | pass |
| 123 | [vault-multiselect-bar](light/123-vault-multiselect-bar.html) | 0.010 % | 0.011 % | pass |
| 124 | [vault-item-context-menu-multi](light/124-vault-item-context-menu-multi.html) | 0.008 % | 0.009 % | pass |
| 125 | [vault-copy-to-global-dialog](light/125-vault-copy-to-global-dialog.html) | 0.003 % | 0.001 % | pass |
| 126 | [vault-move-to-folder-dialog](light/126-vault-move-to-folder-dialog.html) | 0.003 % | 0.001 % | pass |
| 127 | [vault-move-to-folder-new-folder](light/127-vault-move-to-folder-new-folder.html) | 0.003 % | 0.001 % | pass |
| 128 | [vault-add-to-collection-dialog](light/128-vault-add-to-collection-dialog.html) | 0.003 % | 0.001 % | pass |
| 129 | [vault-tag-dialog](light/129-vault-tag-dialog.html) | 0.003 % | 0.001 % | pass |
| 130 | [vault-tag-dialog-required](light/130-vault-tag-dialog-required.html) | 0.003 % | 0.001 % | pass |
| 131 | [vault-add-url-dialog](light/131-vault-add-url-dialog.html) | 0.000 % | 0.000 % | pass |
| 132 | [vault-add-url-invalid](light/132-vault-add-url-invalid.html) | 0.000 % | 0.000 % | pass |
| 133 | [vault-quick-capture-dialog](light/133-vault-quick-capture-dialog.html) | 0.000 % | 0.000 % | pass |
| 134 | [vault-quick-capture-filled](light/134-vault-quick-capture-filled.html) | 0.000 % | 0.000 % | pass |
| 135 | [vault-quick-capture-saved-toast](light/135-vault-quick-capture-saved-toast.html) | 0.000 % | 0.000 % | pass |
| 136 | [vault-sketch-dialog](light/136-vault-sketch-dialog.html) | 0.097 % | 0.129 % | pass |
| 137 | [vault-sketch-dialog-drawn](light/137-vault-sketch-dialog-drawn.html) | 0.270 % | 0.302 % | pass |
| 138 | [vault-record-note-dialog](light/138-vault-record-note-dialog.html) | 0.000 % | 0.000 % | pass |
| 139 | [vault-send-to-beat-board-dialog](light/139-vault-send-to-beat-board-dialog.html) | 0.004 % | 0.004 % | pass |
| 140 | [vault-send-to-beat-board-sequence](light/140-vault-send-to-beat-board-sequence.html) | 0.004 % | 0.004 % | pass |
| 141 | [vault-send-to-beat-board-character](light/141-vault-send-to-beat-board-character.html) | 0.004 % | 0.004 % | pass |
| 142 | [vault-new-collection-dialog](light/142-vault-new-collection-dialog.html) | 0.000 % | 0.000 % | pass |
| 143 | [vault-new-collection-name-required](light/143-vault-new-collection-name-required.html) | 0.000 % | 0.000 % | pass |
| 144 | [vault-collection-empty](light/144-vault-collection-empty.html) | 0.000 % | 0.000 % | pass |
| 145 | [vault-new-folder-dialog](light/145-vault-new-folder-dialog.html) | 0.000 % | 0.000 % | pass |
| 146 | [vault-new-folder-name-required](light/146-vault-new-folder-name-required.html) | 0.000 % | 0.000 % | pass |
| 147 | [vault-folder-empty](light/147-vault-folder-empty.html) | 0.000 % | 0.000 % | pass |
| 148 | [vault-folder-context-menu](light/148-vault-folder-context-menu.html) | 0.000 % | 0.000 % | pass |
| 149 | [vault-collection-context-menu](light/149-vault-collection-context-menu.html) | 0.000 % | 0.000 % | pass |
| 150 | [vault-rename-folder-dialog](light/150-vault-rename-folder-dialog.html) | 0.000 % | 0.000 % | pass |
| 151 | [vault-rename-collection-dialog](light/151-vault-rename-collection-dialog.html) | 0.000 % | 0.000 % | pass |
| 152 | [vault-folder-deleted-toast](light/152-vault-folder-deleted-toast.html) | 0.000 % | 0.000 % | pass |
| 153 | [vault-folder-card-context-menu](light/153-vault-folder-card-context-menu.html) | 0.002 % | 0.002 % | pass |
| 154 | [vault-pinned-empty](light/154-vault-pinned-empty.html) | 0.000 % | 0.000 % | pass |
| 155 | [vault-grid-all-types](light/155-vault-grid-all-types.html) | 0.004 % | 0.002 % | pass |
| 156 | [vault-view-list-all-types](light/156-vault-view-list-all-types.html) | 0.000 % | 0.000 % | pass |
| 157 | [vault-drawer-image](light/157-vault-drawer-image.html) | 0.003 % | 0.066 % | pass |
| 158 | [vault-drawer-sketch](light/158-vault-drawer-sketch.html) | 0.003 % | 0.066 % | pass |
| 159 | [vault-drawer-voice](light/159-vault-drawer-voice.html) | 0.066 % | 0.128 % | pass |
| 160 | [vault-drawer-audio](light/160-vault-drawer-audio.html) | 0.066 % | 0.129 % | pass |
| 161 | [vault-drawer-video](light/161-vault-drawer-video.html) | 0.003 % | 0.066 % | pass |
| 162 | [vault-drawer-pdf](light/162-vault-drawer-pdf.html) | 0.003 % | 0.066 % | pass |
| 163 | [vault-drawer-document](light/163-vault-drawer-document.html) | 0.003 % | 0.066 % | pass |
| 164 | [vault-drawer-file](light/164-vault-drawer-file.html) | 0.003 % | 0.066 % | pass |
| 165 | [vault-drawer-file-unavailable](light/165-vault-drawer-file-unavailable.html) | 0.003 % | 0.066 % | pass |
| 166 | [vault-drawer-not-found](light/166-vault-drawer-not-found.html) | 0.000 % | 0.000 % | pass |
| 167 | [vault-recently-deleted](light/167-vault-recently-deleted.html) | 0.000 % | 0.000 % | pass |
| 168 | [vault-purge-confirm](light/168-vault-purge-confirm.html) | 0.005 % | 0.004 % | pass |
| 169 | [vault-recently-deleted-empty](light/169-vault-recently-deleted-empty.html) | 0.000 % | 0.000 % | pass |
| 170 | [vault-empty-project](light/170-vault-empty-project.html) | 0.000 % | 0.000 % | pass |
| 171 | [vault-empty-add-menu](light/171-vault-empty-add-menu.html) | 0.000 % | 0.000 % | pass |
| 172 | [vault-send-to-beat-board-no-acts](light/172-vault-send-to-beat-board-no-acts.html) | 0.002 % | 0.002 % | pass |
| 173 | [story-page-filled](light/173-story-page-filled.html) | 0.000 % | 0.000 % | pass |
| 174 | [story-page-highlight-colours](light/174-story-page-highlight-colours.html) | 0.000 % | 0.000 % | pass |
| 175 | [story-page-underline-colours](light/175-story-page-underline-colours.html) | 0.000 % | 0.000 % | pass |
| 176 | [story-page-dark](light/176-story-page-dark.html) | 0.000 % | 0.000 % | pass |
| 177 | [story-page-export-dialog](light/177-story-page-export-dialog.html) | 0.000 % | 0.000 % | pass |
| 178 | [story-board-filled](light/178-story-board-filled.html) | 0.000 % | 0.000 % | pass |
| 179 | [story-board-horizontal](light/179-story-board-horizontal.html) | 0.000 % | 0.000 % | pass |
| 180 | [story-board-add-menu](light/180-story-board-add-menu.html) | 0.000 % | 0.000 % | pass |
| 181 | [story-board-more-actions-menu](light/181-story-board-more-actions-menu.html) | 0.000 % | 0.000 % | pass |
| 182 | [story-board-zoom-50](light/182-story-board-zoom-50.html) | 0.000 % | 0.000 % | pass |
| 183 | [story-board-zoom-150](light/183-story-board-zoom-150.html) | 0.000 % | 0.000 % | pass |
| 184 | [story-board-filter-active](light/184-story-board-filter-active.html) | 0.000 % | 0.000 % | pass |
| 185 | [story-board-parking-lot-closed](light/185-story-board-parking-lot-closed.html) | 0.000 % | 0.000 % | pass |
| 186 | [story-board-act-menu](light/186-story-board-act-menu.html) | 0.000 % | 0.000 % | pass |
| 187 | [story-board-act-collapsed](light/187-story-board-act-collapsed.html) | 0.000 % | 0.000 % | pass |
| 188 | [story-board-act-rename-inline](light/188-story-board-act-rename-inline.html) | 0.000 % | 0.000 % | pass |
| 189 | [story-board-new-act-input](light/189-story-board-new-act-input.html) | 0.000 % | 0.000 % | pass |
| 190 | [story-board-sequence-context-menu](light/190-story-board-sequence-context-menu.html) | 0.000 % | 0.000 % | pass |
| 191 | [story-board-card-context-menu](light/191-story-board-card-context-menu.html) | 0.000 % | 0.000 % | pass |
| 192 | [story-board-beat-context-menu](light/192-story-board-beat-context-menu.html) | 0.000 % | 0.000 % | pass |
| 193 | [story-board-parked-card-context-menu](light/193-story-board-parked-card-context-menu.html) | 0.000 % | 0.000 % | pass |
| 194 | [story-board-card-selected](light/194-story-board-card-selected.html) | 0.000 % | 0.000 % | pass |
| 195 | [story-board-multiselect-bar](light/195-story-board-multiselect-bar.html) | 0.000 % | 0.000 % | pass |
| 196 | [story-multiselect-move-dialog](light/196-story-multiselect-move-dialog.html) | 0.000 % | 0.000 % | pass |
| 197 | [story-export-dialog-selected](light/197-story-export-dialog-selected.html) | 0.000 % | 0.000 % | pass |
| 198 | [story-card-drawer](light/198-story-card-drawer.html) | 0.000 % | 0.000 % | pass |
| 199 | [story-card-drawer-parked](light/199-story-card-drawer-parked.html) | 0.000 % | 0.000 % | pass |
| 200 | [story-beat-drawer](light/200-story-beat-drawer.html) | 0.000 % | 0.000 % | pass |
| 201 | [story-sequence-drawer](light/201-story-sequence-drawer.html) | 0.000 % | 0.000 % | pass |
| 202 | [story-delete-act-dialog](light/202-story-delete-act-dialog.html) | 0.007 % | 0.006 % | pass |
| 203 | [story-delete-act-dialog-delete-all](light/203-story-delete-act-dialog-delete-all.html) | 0.007 % | 0.006 % | pass |
| 204 | [story-delete-sequence-dialog](light/204-story-delete-sequence-dialog.html) | 0.004 % | 0.003 % | pass |
| 205 | [story-delete-sequence-dialog-delete-all](light/205-story-delete-sequence-dialog-delete-all.html) | 0.004 % | 0.003 % | pass |
| 206 | [story-move-dialog](light/206-story-move-dialog.html) | 0.000 % | 0.000 % | pass |
| 207 | [story-act-note-dialog](light/207-story-act-note-dialog.html) | 0.000 % | 0.000 % | pass |
| 208 | [story-card-display-dialog](light/208-story-card-display-dialog.html) | 0.000 % | 0.000 % | pass |
| 209 | [story-card-display-dialog-changed](light/209-story-card-display-dialog-changed.html) | 0.000 % | 0.000 % | pass |
| 210 | [story-board-rich-cards](light/210-story-board-rich-cards.html) | 0.000 % | 0.000 % | pass |
| 211 | [story-build-dialog](light/211-story-build-dialog.html) | 0.000 % | 0.000 % | pass |
| 212 | [story-build-dialog-new-screenplay](light/212-story-build-dialog-new-screenplay.html) | 0.000 % | 0.000 % | pass |
| 213 | [story-build-dialog-title-clash](light/213-story-build-dialog-title-clash.html) | 0.000 % | 0.000 % | pass |
| 214 | [story-build-dialog-preselected](light/214-story-build-dialog-preselected.html) | 0.000 % | 0.000 % | pass |
| 215 | [story-build-dialog-missing-heading](light/215-story-build-dialog-missing-heading.html) | 0.002 % | 0.002 % | pass |
| 216 | [story-apply-order-dialog](light/216-story-apply-order-dialog.html) | 0.004 % | 0.004 % | pass |
| 217 | [story-apply-order-dialog-locked](light/217-story-apply-order-dialog-locked.html) | 0.005 % | 0.005 % | pass |
| 218 | [story-export-dialog](light/218-story-export-dialog.html) | 0.000 % | 0.000 % | pass |
| 219 | [story-outline-view](light/219-story-outline-view.html) | 0.000 % | 0.000 % | pass |
| 220 | [story-outline-act-menu](light/220-story-outline-act-menu.html) | 0.000 % | 0.000 % | pass |
| 221 | [story-outline-card-context-menu](light/221-story-outline-card-context-menu.html) | 0.000 % | 0.000 % | pass |
| 222 | [story-outline-new-act-input](light/222-story-outline-new-act-input.html) | 0.000 % | 0.000 % | pass |
| 223 | [story-outline-card-drawer](light/223-story-outline-card-drawer.html) | 0.000 % | 0.000 % | pass |
| 224 | [story-characters-list](light/224-story-characters-list.html) | 0.066 % | 0.063 % | pass |
| 225 | [story-character-detail-venkat](light/225-story-character-detail-venkat.html) | 0.071 % | 0.068 % | pass |
| 226 | [story-character-more-menu](light/226-story-character-more-menu.html) | 0.049 % | 0.046 % | pass |
| 227 | [story-new-character-dialog](light/227-story-new-character-dialog.html) | 0.024 % | 0.031 % | pass |
| 228 | [story-new-character-duplicate-warning](light/228-story-new-character-duplicate-warning.html) | 0.020 % | 0.026 % | pass |
| 229 | [story-edit-character-dialog](light/229-story-edit-character-dialog.html) | 0.024 % | 0.031 % | pass |
| 230 | [story-add-relationship-dialog](light/230-story-add-relationship-dialog.html) | 0.025 % | 0.033 % | pass |
| 231 | [story-add-relationship-dialog-filled](light/231-story-add-relationship-dialog-filled.html) | 0.025 % | 0.033 % | pass |
| 232 | [story-relationship-map](light/232-story-relationship-map.html) | 0.004 % | 0.004 % | pass |
| 233 | [story-relationship-map-whole-film](light/233-story-relationship-map-whole-film.html) | 0.005 % | 0.004 % | pass |
| 234 | [story-delete-character-dialog-lakshmi-in-use](light/234-story-delete-character-dialog-lakshmi-in-use.html) | 0.029 % | 0.036 % | pass |
| 235 | [story-delete-character-dialog-padma-in-use](light/235-story-delete-character-dialog-padma-in-use.html) | 0.009 % | 0.009 % | pass |
| 236 | [story-character-archived-toast](light/236-story-character-archived-toast.html) | 0.066 % | 0.063 % | pass |
| 237 | [story-characters-show-archived](light/237-story-characters-show-archived.html) | 0.006 % | 0.005 % | pass |
| 238 | [story-timeline-filled](light/238-story-timeline-filled.html) | 0.004 % | 0.003 % | pass |
| 239 | [story-timeline-scenes-selected](light/239-story-timeline-scenes-selected.html) | 0.004 % | 0.003 % | pass |
| 240 | [story-assign-story-day-dialog](light/240-story-assign-story-day-dialog.html) | 0.000 % | 0.000 % | pass |
| 241 | [story-assign-story-day-dialog-filled](light/241-story-assign-story-day-dialog-filled.html) | 0.000 % | 0.000 % | pass |
| 242 | [story-timeline-day-assigned](light/242-story-timeline-day-assigned.html) | 0.006 % | 0.006 % | pass |
| 243 | [story-timeline-time-note](light/243-story-timeline-time-note.html) | 0.006 % | 0.006 % | pass |
| 244 | [story-timeline-grouped-days](light/244-story-timeline-grouped-days.html) | 0.006 % | 0.006 % | pass |
| 245 | [story-page-empty](light/245-story-page-empty.html) | 0.000 % | 0.000 % | pass |
| 246 | [story-board-empty](light/246-story-board-empty.html) | 0.000 % | 0.000 % | pass |
| 247 | [story-board-empty-add-menu](light/247-story-board-empty-add-menu.html) | 0.000 % | 0.000 % | pass |
| 248 | [story-board-empty-new-act-input](light/248-story-board-empty-new-act-input.html) | 0.000 % | 0.000 % | pass |
| 249 | [story-build-dialog-no-cards](light/249-story-build-dialog-no-cards.html) | 0.001 % | 0.001 % | pass |
| 250 | [story-apply-order-dialog-no-screenplay](light/250-story-apply-order-dialog-no-screenplay.html) | 0.000 % | 0.000 % | pass |
| 251 | [story-characters-empty](light/251-story-characters-empty.html) | 0.000 % | 0.000 % | pass |
| 252 | [story-timeline-empty](light/252-story-timeline-empty.html) | 0.000 % | 0.000 % | pass |
| 253 | [sp-editor-standard](light/253-sp-editor-standard.html) | 0.006 % | 0.005 % | pass |
| 254 | [sp-editor-scrolled-mid-script](light/254-sp-editor-scrolled-mid-script.html) | 0.007 % | 0.006 % | pass |
| 255 | [sp-zoom-50](light/255-sp-zoom-50.html) | 0.008 % | 0.007 % | pass |
| 256 | [sp-zoom-200](light/256-sp-zoom-200.html) | 0.006 % | 0.005 % | pass |
| 257 | [sp-navigator-columns-menu](light/257-sp-navigator-columns-menu.html) | 0.006 % | 0.005 % | pass |
| 258 | [sp-navigator-columns-menu-checked](light/258-sp-navigator-columns-menu-checked.html) | 0.006 % | 0.005 % | pass |
| 259 | [sp-navigator-columns-shown](light/259-sp-navigator-columns-shown.html) | 0.034 % | 0.005 % | pass |
| 260 | [sp-navigator-hidden](light/260-sp-navigator-hidden.html) | 0.095 % | 0.005 % | pass |
| 261 | [sp-scene-context-menu](light/261-sp-scene-context-menu.html) | 0.006 % | 0.005 % | pass |
| 262 | [sp-scene-hub-drawer](light/262-sp-scene-hub-drawer.html) | 0.363 % | 0.363 % | pass |
| 263 | [sp-scene-number-dialog](light/263-sp-scene-number-dialog.html) | 0.002 % | 0.002 % | pass |
| 264 | [sp-scene-number-warning](light/264-sp-scene-number-warning.html) | 0.002 % | 0.002 % | pass |
| 265 | [sp-reorder-warning-dialog](light/265-sp-reorder-warning-dialog.html) | 0.007 % | 0.007 % | pass |
| 266 | [sp-element-type-menu](light/266-sp-element-type-menu.html) | 0.004 % | 0.003 % | pass |
| 267 | [sp-suggestion-list](light/267-sp-suggestion-list.html) | 0.007 % | 0.007 % | pass |
| 268 | [sp-suggestion-list-create-new](light/268-sp-suggestion-list-create-new.html) | 0.006 % | 0.006 % | pass |
| 269 | [sp-comment-float-button](light/269-sp-comment-float-button.html) | 0.101 % | 0.100 % | pass |
| 270 | [sp-comment-composer-text](light/270-sp-comment-composer-text.html) | 0.004 % | 0.003 % | pass |
| 271 | [sp-standard-comments-panel](light/271-sp-standard-comments-panel.html) | 0.006 % | 0.005 % | pass |
| 272 | [sp-find-empty](light/272-sp-find-empty.html) | 0.079 % | 0.003 % | pass |
| 273 | [sp-find-results](light/273-sp-find-results.html) | 0.006 % | 0.005 % | pass |
| 274 | [sp-find-replace-filled](light/274-sp-find-replace-filled.html) | 0.006 % | 0.005 % | pass |
| 275 | [sp-find-no-results](light/275-sp-find-no-results.html) | 0.004 % | 0.003 % | pass |
| 276 | [sp-find-this-scene](light/276-sp-find-this-scene.html) | 0.004 % | 0.003 % | pass |
| 277 | [sp-dark-page](light/277-sp-dark-page.html) | 0.036 % | 0.035 % | pass |
| 278 | [sp-view-menu](light/278-sp-view-menu.html) | 0.036 % | 0.035 % | pass |
| 279 | [sp-title-page-dialog](light/279-sp-title-page-dialog.html) | 0.001 % | 0.001 % | pass |
| 280 | [sp-title-page-empty-title](light/280-sp-title-page-empty-title.html) | 0.001 % | 0.001 % | pass |
| 281 | [sp-page-setup-dialog](light/281-sp-page-setup-dialog.html) | 0.001 % | 0.001 % | pass |
| 282 | [sp-page-setup-header-filled](light/282-sp-page-setup-header-filled.html) | 0.001 % | 0.001 % | pass |
| 283 | [sp-custom-elements-empty](light/283-sp-custom-elements-empty.html) | 0.001 % | 0.001 % | pass |
| 284 | [sp-custom-elements-add-form](light/284-sp-custom-elements-add-form.html) | 0.004 % | 0.004 % | pass |
| 285 | [sp-page-view-drawer](light/285-sp-page-view-drawer.html) | 0.004 % | 0.003 % | pass |
| 286 | [sp-spell-menu](light/286-sp-spell-menu.html) | 0.461 % | 0.457 % | pass |
| 287 | [sp-writing-room-beat-board](light/287-sp-writing-room-beat-board.html) | 0.009 % | 0.008 % | pass |
| 288 | [sp-writing-room-beat-board-all-cards](light/288-sp-writing-room-beat-board-all-cards.html) | 0.231 % | 0.007 % | pass |
| 289 | [sp-writing-room-no-panel](light/289-sp-writing-room-no-panel.html) | 0.006 % | 0.005 % | pass |
| 290 | [sp-panel-scene-notes](light/290-sp-panel-scene-notes.html) | 0.005 % | 0.004 % | pass |
| 291 | [sp-panel-private-note-menu](light/291-sp-panel-private-note-menu.html) | 0.005 % | 0.004 % | pass |
| 292 | [sp-panel-private-note-composer](light/292-sp-panel-private-note-composer.html) | 0.005 % | 0.004 % | pass |
| 293 | [sp-panel-scene-note-editing](light/293-sp-panel-scene-note-editing.html) | 0.005 % | 0.004 % | pass |
| 294 | [sp-panel-characters](light/294-sp-panel-characters.html) | 0.013 % | 0.009 % | pass |
| 295 | [sp-panel-characters-match-menu](light/295-sp-panel-characters-match-menu.html) | 0.011 % | 0.007 % | pass |
| 296 | [sp-panel-comments-open](light/296-sp-panel-comments-open.html) | 0.008 % | 0.006 % | pass |
| 297 | [sp-panel-comments-actions-menu](light/297-sp-panel-comments-actions-menu.html) | 0.008 % | 0.006 % | pass |
| 298 | [sp-panel-comments-resolved](light/298-sp-panel-comments-resolved.html) | 0.008 % | 0.006 % | pass |
| 299 | [sp-panel-comments-reply](light/299-sp-panel-comments-reply.html) | 0.008 % | 0.006 % | pass |
| 300 | [sp-panel-comments-composer-scene](light/300-sp-panel-comments-composer-scene.html) | 0.006 % | 0.005 % | pass |
| 301 | [sp-panel-comments-composer-draft](light/301-sp-panel-comments-composer-draft.html) | 0.006 % | 0.005 % | pass |
| 302 | [sp-panel-draft-info](light/302-sp-panel-draft-info.html) | 0.006 % | 0.005 % | pass |
| 303 | [sp-writing-room-two-panels](light/303-sp-writing-room-two-panels.html) | 0.005 % | 0.004 % | pass |
| 304 | [sp-focus-mode](light/304-sp-focus-mode.html) | 0.000 % | 0.000 % | pass |
| 305 | [sp-draft-switch-menu](light/305-sp-draft-switch-menu.html) | 0.006 % | 0.005 % | pass |
| 306 | [sp-editor-older-draft](light/306-sp-editor-older-draft.html) | 0.005 % | 0.005 % | pass |
| 307 | [sp-panel-comments-empty](light/307-sp-panel-comments-empty.html) | 0.007 % | 0.006 % | pass |
| 308 | [sp-panel-comments-resolved-empty](light/308-sp-panel-comments-resolved-empty.html) | 0.007 % | 0.006 % | pass |
| 309 | [sp-drafts-drawer](light/309-sp-drafts-drawer.html) | 0.007 % | 0.006 % | pass |
| 310 | [sp-draft-actions-menu](light/310-sp-draft-actions-menu.html) | 0.007 % | 0.006 % | pass |
| 311 | [sp-draft-actions-menu-other](light/311-sp-draft-actions-menu-other.html) | 0.007 % | 0.006 % | pass |
| 312 | [sp-rename-draft-dialog](light/312-sp-rename-draft-dialog.html) | 0.003 % | 0.003 % | pass |
| 313 | [sp-delete-draft-dialog-current](light/313-sp-delete-draft-dialog-current.html) | 0.003 % | 0.003 % | pass |
| 314 | [sp-delete-draft-dialog](light/314-sp-delete-draft-dialog.html) | 0.003 % | 0.003 % | pass |
| 315 | [sp-new-draft-dialog](light/315-sp-new-draft-dialog.html) | 0.002 % | 0.002 % | pass |
| 316 | [sp-new-draft-previous-source](light/316-sp-new-draft-previous-source.html) | 0.002 % | 0.002 % | pass |
| 317 | [sp-new-draft-duplicate-name](light/317-sp-new-draft-duplicate-name.html) | 0.169 % | 0.166 % | pass |
| 318 | [sp-compare-text-only](light/318-sp-compare-text-only.html) | 0.012 % | 0.010 % | pass |
| 319 | [sp-compare-scene-only](light/319-sp-compare-scene-only.html) | 0.006 % | 0.008 % | pass |
| 320 | [sp-compare-second-change](light/320-sp-compare-second-change.html) | 0.137 % | 0.010 % | pass |
| 321 | [sp-compare-same-draft](light/321-sp-compare-same-draft.html) | 0.000 % | 0.000 % | pass |
| 322 | [sp-lock-dialog](light/322-sp-lock-dialog.html) | 0.004 % | 0.004 % | pass |
| 323 | [sp-locked-draft](light/323-sp-locked-draft.html) | 0.005 % | 0.004 % | pass |
| 324 | [sp-locked-edit-dialog](light/324-sp-locked-edit-dialog.html) | 0.003 % | 0.003 % | pass |
| 325 | [sp-start-revision-dialog](light/325-sp-start-revision-dialog.html) | 0.003 % | 0.003 % | pass |
| 326 | [sp-start-revision-filled](light/326-sp-start-revision-filled.html) | 0.003 % | 0.003 % | pass |
| 327 | [sp-start-revision-custom-colour](light/327-sp-start-revision-custom-colour.html) | 0.003 % | 0.003 % | pass |
| 328 | [sp-revisions-drawer-locked](light/328-sp-revisions-drawer-locked.html) | 0.007 % | 0.007 % | pass |
| 329 | [sp-revision-draft-editor](light/329-sp-revision-draft-editor.html) | 0.002 % | 0.001 % | pass |
| 330 | [sp-navigator-revision-column](light/330-sp-navigator-revision-column.html) | 0.002 % | 0.002 % | pass |
| 331 | [sp-revisions-drawer-revision](light/331-sp-revisions-drawer-revision.html) | 0.007 % | 0.006 % | pass |
| 332 | [sp-page-view-revision-marks](light/332-sp-page-view-revision-marks.html) | 0.014 % | 0.014 % | pass |
| 333 | [sp-export-revision-marked](light/333-sp-export-revision-marked.html) | 0.008 % | 0.008 % | pass |
| 334 | [sp-export-dialog](light/334-sp-export-dialog.html) | 0.002 % | 0.002 % | pass |
| 335 | [sp-export-more-formats](light/335-sp-export-more-formats.html) | 0.002 % | 0.002 % | pass |
| 336 | [sp-export-selected-none](light/336-sp-export-selected-none.html) | 0.005 % | 0.005 % | pass |
| 337 | [sp-export-selected-scenes](light/337-sp-export-selected-scenes.html) | 0.015 % | 0.016 % | pass |
| 338 | [sp-print-preview](light/338-sp-print-preview.html) | 0.002 % | 0.002 % | pass |
| 339 | [sp-print-preview-zoomed](light/339-sp-print-preview-zoomed.html) | 0.002 % | 0.002 % | pass |
| 340 | [sp-print-preview-no-title-page](light/340-sp-print-preview-no-title-page.html) | 0.005 % | 0.005 % | pass |
| 341 | [sp-import-pasted-text](light/341-sp-import-pasted-text.html) | 0.002 % | 0.002 % | pass |
| 342 | [sp-import-failed](light/342-sp-import-failed.html) | 0.002 % | 0.002 % | pass |
| 343 | [sp-screenplay-picker-menu](light/343-sp-screenplay-picker-menu.html) | 0.006 % | 0.005 % | pass |
| 344 | [sp-new-screenplay-dialog](light/344-sp-new-screenplay-dialog.html) | 0.002 % | 0.002 % | pass |
| 345 | [sp-new-screenplay-duplicate-title](light/345-sp-new-screenplay-duplicate-title.html) | 0.002 % | 0.002 % | pass |
| 346 | [sp-rename-screenplay-dialog](light/346-sp-rename-screenplay-dialog.html) | 0.002 % | 0.002 % | pass |
| 347 | [sp-delete-screenplay-dialog](light/347-sp-delete-screenplay-dialog.html) | 0.004 % | 0.004 % | pass |
| 348 | [sp-empty-workspace](light/348-sp-empty-workspace.html) | 0.000 % | 0.000 % | pass |
| 349 | [bd-no-source](light/349-bd-no-source.html) | 0.000 % | 0.000 % | pass |
| 350 | [bd-select-source-empty-project](light/350-bd-select-source-empty-project.html) | 0.103 % | 0.000 % | pass |
| 351 | [bd-main](light/351-bd-main.html) | 0.013 % | 0.012 % | pass |
| 352 | [bd-change-source-dialog](light/352-bd-change-source-dialog.html) | 0.005 % | 0.005 % | pass |
| 353 | [bd-filter-menu](light/353-bd-filter-menu.html) | 0.009 % | 0.008 % | pass |
| 354 | [bd-filter-only-suggested](light/354-bd-filter-only-suggested.html) | 0.013 % | 0.012 % | pass |
| 355 | [bd-filter-by-category](light/355-bd-filter-by-category.html) | 0.010 % | 0.009 % | pass |
| 356 | [bd-filter-no-match](light/356-bd-filter-no-match.html) | 0.004 % | 0.003 % | pass |
| 357 | [bd-scene-list-hidden](light/357-bd-scene-list-hidden.html) | 0.096 % | 0.008 % | pass |
| 358 | [bd-panel-hidden](light/358-bd-panel-hidden.html) | 0.096 % | 0.012 % | pass |
| 359 | [bd-both-hidden](light/359-bd-both-hidden.html) | 0.097 % | 0.008 % | pass |
| 360 | [bd-scene-suggestions](light/360-bd-scene-suggestions.html) | 0.013 % | 0.012 % | pass |
| 361 | [bd-suggestion-review-dialog](light/361-bd-suggestion-review-dialog.html) | 0.014 % | 0.014 % | pass |
| 362 | [bd-edit-suggestion-dialog](light/362-bd-edit-suggestion-dialog.html) | 0.006 % | 0.006 % | pass |
| 363 | [bd-possible-existing-item-dialog](light/363-bd-possible-existing-item-dialog.html) | 0.005 % | 0.004 % | pass |
| 364 | [bd-scene-marked-complete](light/364-bd-scene-marked-complete.html) | 0.013 % | 0.012 % | pass |
| 365 | [bd-scene-quick-reuse](light/365-bd-scene-quick-reuse.html) | 0.013 % | 0.012 % | pass |
| 366 | [bd-scene-nothing-yet](light/366-bd-scene-nothing-yet.html) | 0.013 % | 0.012 % | pass |
| 367 | [bd-tag-menu](light/367-bd-tag-menu.html) | 0.114 % | 0.111 % | pass |
| 368 | [bd-tag-element-dialog](light/368-bd-tag-element-dialog.html) | 0.005 % | 0.005 % | pass |
| 369 | [bd-add-element-existing-match](light/369-bd-add-element-existing-match.html) | 0.005 % | 0.005 % | pass |
| 370 | [bd-element-tagged](light/370-bd-element-tagged.html) | 0.013 % | 0.012 % | pass |
| 371 | [bd-element-actions-menu](light/371-bd-element-actions-menu.html) | 0.013 % | 0.012 % | pass |
| 372 | [bd-element-notes-dialog](light/372-bd-element-notes-dialog.html) | 0.005 % | 0.005 % | pass |
| 373 | [bd-show-in-script](light/373-bd-show-in-script.html) | 0.077 % | 0.073 % | pass |
| 374 | [bd-add-element-dialog](light/374-bd-add-element-dialog.html) | 0.005 % | 0.005 % | pass |
| 375 | [bd-export-dialog](light/375-bd-export-dialog.html) | 0.005 % | 0.005 % | pass |
| 376 | [bd-export-selected-scenes](light/376-bd-export-selected-scenes.html) | 0.007 % | 0.008 % | pass |
| 377 | [bd-export-csv](light/377-bd-export-csv.html) | 0.005 % | 0.005 % | pass |
| 378 | [bd-scene-no-text](light/378-bd-scene-no-text.html) | 0.005 % | 0.004 % | pass |
| 379 | [bd-scene-changed-banner](light/379-bd-scene-changed-banner.html) | 0.005 % | 0.004 % | pass |
| 380 | [bd-scene-changes-dialog](light/380-bd-scene-changes-dialog.html) | 0.001 % | 0.002 % | pass |
| 381 | [bd-needs-review-view](light/381-bd-needs-review-view.html) | 0.318 % | 0.017 % | pass |
| 382 | [bd-removed-scenes-view](light/382-bd-removed-scenes-view.html) | 0.306 % | 0.008 % | pass |
| 383 | [bd-removed-scene-panel](light/383-bd-removed-scene-panel.html) | 0.002 % | 0.002 % | pass |
| 384 | [ai-panel-ready](light/384-ai-panel-ready.html) | 0.000 % | 0.000 % | pass |
| 385 | [ai-settings-menu](light/385-ai-settings-menu.html) | 0.000 % | 0.000 % | pass |
| 386 | [ai-scope-current-screenplay](light/386-ai-scope-current-screenplay.html) | 0.051 % | 0.000 % | pass |
| 387 | [ai-scope-beat-board](light/387-ai-scope-beat-board.html) | 0.000 % | 0.000 % | pass |
| 388 | [ai-offline-settings](light/388-ai-offline-settings.html) | 0.002 % | 0.002 % | pass |
| 389 | [ai-offline-technical-details](light/389-ai-offline-technical-details.html) | 0.008 % | 0.007 % | pass |
| 390 | [ai-offline-remove-confirm](light/390-ai-offline-remove-confirm.html) | 0.001 % | 0.001 % | pass |
| 391 | [ai-question-typed](light/391-ai-question-typed.html) | 0.000 % | 0.000 % | pass |
| 392 | [ai-working](light/392-ai-working.html) | 0.000 % | 0.000 % | pass |
| 393 | [ai-answer-characters](light/393-ai-answer-characters.html) | 0.000 % | 0.000 % | pass |
| 394 | [ai-answer-characters-sources](light/394-ai-answer-characters-sources.html) | 0.000 % | 0.000 % | pass |
| 395 | [ai-panel-screenplay](light/395-ai-panel-screenplay.html) | 0.037 % | 0.001 % | pass |
| 396 | [ai-answer-locations](light/396-ai-answer-locations.html) | 0.002 % | 0.001 % | pass |
| 397 | [ai-answer-locations-sources-open](light/397-ai-answer-locations-sources-open.html) | 0.002 % | 0.001 % | pass |
| 398 | [ai-answer-scene-count](light/398-ai-answer-scene-count.html) | 0.002 % | 0.001 % | pass |
| 399 | [ai-proposed-changes](light/399-ai-proposed-changes.html) | 0.002 % | 0.001 % | pass |
| 400 | [ai-proposal-rejected](light/400-ai-proposal-rejected.html) | 0.002 % | 0.001 % | pass |
| 401 | [ai-proposed-changes-second](light/401-ai-proposed-changes-second.html) | 0.002 % | 0.001 % | pass |
| 402 | [ai-proposal-applied](light/402-ai-proposal-applied.html) | 0.002 % | 0.001 % | pass |
| 403 | [ai-answer-delete-request](light/403-ai-answer-delete-request.html) | 0.002 % | 0.001 % | pass |
| 404 | [ai-panel-story](light/404-ai-panel-story.html) | 0.036 % | 0.000 % | pass |
| 405 | [ai-panel-breakdown](light/405-ai-panel-breakdown.html) | 0.049 % | 0.012 % | pass |
| 406 | [ai-panel-production](light/406-ai-panel-production.html) | 0.037 % | 0.001 % | pass |
| 407 | [ai-panel-vault](light/407-ai-panel-vault.html) | 0.036 % | 0.000 % | pass |
| 408 | [ai-offline-health-filled](light/408-ai-offline-health-filled.html) | 0.000 % | 0.000 % | pass |
| 409 | [ai-panel-empty-project](light/409-ai-panel-empty-project.html) | 0.000 % | 0.001 % | pass |
| 410 | [pkg-project-menu](light/410-pkg-project-menu.html) | 0.000 % | 0.000 % | pass |
| 411 | [pkg-backup-dialog](light/411-pkg-backup-dialog.html) | 0.001 % | 0.000 % | pass |
| 412 | [pkg-export-project-dialog](light/412-pkg-export-project-dialog.html) | 0.000 % | 0.000 % | pass |
| 413 | [pkg-export-script-dialog](light/413-pkg-export-script-dialog.html) | 0.000 % | 0.000 % | pass |
| 414 | [pkg-export-script-comments](light/414-pkg-export-script-comments.html) | 0.000 % | 0.000 % | pass |
| 415 | [pkg-export-script-no-scene](light/415-pkg-export-script-no-scene.html) | 0.000 % | 0.000 % | pass |
| 416 | [pkg-export-script-scene-chosen](light/416-pkg-export-script-scene-chosen.html) | 0.004 % | 0.004 % | pass |
| 417 | [pkg-export-beat-board-dialog](light/417-pkg-export-beat-board-dialog.html) | 0.001 % | 0.001 % | pass |
| 418 | [pkg-export-beat-board-one-act](light/418-pkg-export-beat-board-one-act.html) | 0.000 % | 0.000 % | pass |
| 419 | [pkg-import-exchange-choose](light/419-pkg-import-exchange-choose.html) | 0.000 % | 0.000 % | pass |
| 420 | [pkg-review-queue-empty](light/420-pkg-review-queue-empty.html) | 0.000 % | 0.000 % | pass |
| 421 | [pkg-history-imports](light/421-pkg-history-imports.html) | 0.000 % | 0.000 % | pass |
| 422 | [pkg-history-review-records](light/422-pkg-history-review-records.html) | 0.000 % | 0.000 % | pass |
| 423 | [pkg-history-my-reviews](light/423-pkg-history-my-reviews.html) | 0.000 % | 0.000 % | pass |
| 424 | [pkg-history-package-log](light/424-pkg-history-package-log.html) | 0.000 % | 0.000 % | pass |
| 425 | [pkg-open-project-package-dialog](light/425-pkg-open-project-package-dialog.html) | 0.000 % | 0.000 % | pass |
| 426 | [licence-trial-chip](light/426-licence-trial-chip.html) | 0.000 % | 0.000 % | pass |
| 427 | [licence-buy-unavailable](light/427-licence-buy-unavailable.html) | 0.155 % | 0.150 % | pass |
| 428 | [licence-trial-reminder](light/428-licence-trial-reminder.html) | 0.000 % | 0.000 % | pass |
| 429 | [licence-trial-ending-chip](light/429-licence-trial-ending-chip.html) | 0.000 % | 0.000 % | pass |
| 430 | [licence-data-note](light/430-licence-data-note.html) | 0.000 % | 0.000 % | pass |
| 431 | [licence-data-note-read](light/431-licence-data-note-read.html) | 0.000 % | 0.000 % | pass |
| 432 | [licence-data-note-confirmed](light/432-licence-data-note-confirmed.html) | 0.013 % | 0.014 % | pass |
| 433 | [licence-tutorial-step-1](light/433-licence-tutorial-step-1.html) | 0.004 % | 0.004 % | pass |
| 434 | [licence-tutorial-step-2](light/434-licence-tutorial-step-2.html) | 0.004 % | 0.004 % | pass |
| 435 | [licence-tutorial-step-3](light/435-licence-tutorial-step-3.html) | 0.095 % | 0.094 % | pass |
| 436 | [licence-tutorial-step-4](light/436-licence-tutorial-step-4.html) | 0.005 % | 0.004 % | pass |
| 437 | [licence-tutorial-step-4-ticked](light/437-licence-tutorial-step-4-ticked.html) | 0.018 % | 0.018 % | pass |
| 438 | [licence-welcome-family](light/438-licence-welcome-family.html) | 0.000 % | 0.000 % | pass |
| 439 | [licence-check-needed](light/439-licence-check-needed.html) | 0.002 % | 0.002 % | pass |
| 440 | [licence-check-purchase-button](light/440-licence-check-purchase-button.html) | 0.000 % | 0.000 % | pass |
| 441 | [licence-locked-trial-ended-microsoft-store](light/441-licence-locked-trial-ended-microsoft-store.html) | 0.001 % | 0.001 % | pass |
| 442 | [licence-locked-buy-unavailable](light/442-licence-locked-buy-unavailable.html) | 0.001 % | 0.001 % | pass |
| 443 | [licence-locked-revoked](light/443-licence-locked-revoked.html) | 0.230 % | 0.227 % | pass |
| 444 | [prod-overview](light/444-prod-overview.html) | 0.001 % | 0.001 % | pass |
| 445 | [prod-overview-source-dialog](light/445-prod-overview-source-dialog.html) | 0.001 % | 0.001 % | pass |
| 446 | [prod-overview-no-source](light/446-prod-overview-no-source.html) | 0.000 % | 0.000 % | pass |
| 447 | [prod-overview-start-setup-dialog](light/447-prod-overview-start-setup-dialog.html) | 0.000 % | 0.000 % | pass |
| 448 | [prod-catalog-empty](light/448-prod-catalog-empty.html) | 0.000 % | 0.000 % | pass |
| 449 | [prod-locations-empty](light/449-prod-locations-empty.html) | 0.000 % | 0.000 % | pass |
| 450 | [prod-cast-empty](light/450-prod-cast-empty.html) | 0.000 % | 0.000 % | pass |
| 451 | [prod-crew-empty](light/451-prod-crew-empty.html) | 0.002 % | 0.004 % | pass |
| 452 | [prod-moodboards-empty](light/452-prod-moodboards-empty.html) | 0.000 % | 0.000 % | pass |
| 453 | [prod-storyboards-empty](light/453-prod-storyboards-empty.html) | 0.000 % | 0.000 % | pass |
| 454 | [prod-shots-no-script](light/454-prod-shots-no-script.html) | 0.001 % | 0.001 % | pass |
| 455 | [prod-catalog](light/455-prod-catalog.html) | 0.000 % | 0.000 % | pass |
| 456 | [prod-catalog-category-props](light/456-prod-catalog-category-props.html) | 0.000 % | 0.000 % | pass |
| 457 | [prod-catalog-more-menu](light/457-prod-catalog-more-menu.html) | 0.000 % | 0.000 % | pass |
| 458 | [prod-catalog-no-match](light/458-prod-catalog-no-match.html) | 0.000 % | 0.000 % | pass |
| 459 | [prod-catalog-row-menu](light/459-prod-catalog-row-menu.html) | 0.000 % | 0.000 % | pass |
| 460 | [prod-catalog-drawer](light/460-prod-catalog-drawer.html) | 0.000 % | 0.000 % | pass |
| 461 | [bd-catalog-scenes-banner](light/461-bd-catalog-scenes-banner.html) | 0.049 % | 0.011 % | pass |
| 462 | [prod-catalog-archive-confirm](light/462-prod-catalog-archive-confirm.html) | 0.000 % | 0.000 % | pass |
| 463 | [prod-catalog-replace-dialog](light/463-prod-catalog-replace-dialog.html) | 0.014 % | 0.014 % | pass |
| 464 | [prod-catalog-delete-confirm](light/464-prod-catalog-delete-confirm.html) | 0.000 % | 0.000 % | pass |
| 465 | [prod-catalog-add-dialog](light/465-prod-catalog-add-dialog.html) | 0.000 % | 0.000 % | pass |
| 466 | [prod-catalog-add-duplicate-warning](light/466-prod-catalog-add-duplicate-warning.html) | 0.002 % | 0.003 % | pass |
| 467 | [prod-catalog-export-dialog](light/467-prod-catalog-export-dialog.html) | 0.000 % | 0.000 % | pass |
| 468 | [prod-locations](light/468-prod-locations.html) | 0.000 % | 0.000 % | pass |
| 469 | [prod-locations-filter-confirmed](light/469-prod-locations-filter-confirmed.html) | 0.000 % | 0.000 % | pass |
| 470 | [prod-locations-no-match](light/470-prod-locations-no-match.html) | 0.000 % | 0.000 % | pass |
| 471 | [prod-location-row-menu](light/471-prod-location-row-menu.html) | 0.000 % | 0.000 % | pass |
| 472 | [prod-location-drawer](light/472-prod-location-drawer.html) | 0.000 % | 0.000 % | pass |
| 473 | [prod-location-drawer-notes](light/473-prod-location-drawer-notes.html) | 0.000 % | 0.000 % | pass |
| 474 | [prod-location-delete-confirm](light/474-prod-location-delete-confirm.html) | 0.000 % | 0.000 % | pass |
| 475 | [prod-location-drawer-hospital](light/475-prod-location-drawer-hospital.html) | 0.000 % | 0.000 % | pass |
| 476 | [prod-location-rejected-banner](light/476-prod-location-rejected-banner.html) | 0.000 % | 0.000 % | pass |
| 477 | [prod-location-replace-dialog](light/477-prod-location-replace-dialog.html) | 0.000 % | 0.000 % | pass |
| 478 | [prod-location-add-dialog](light/478-prod-location-add-dialog.html) | 0.000 % | 0.000 % | pass |
| 479 | [prod-cast](light/479-prod-cast.html) | 0.000 % | 0.000 % | pass |
| 480 | [prod-cast-assign-dialog](light/480-prod-cast-assign-dialog.html) | 0.006 % | 0.005 % | pass |
| 481 | [prod-cast-assign-alternate-banner](light/481-prod-cast-assign-alternate-banner.html) | 0.009 % | 0.008 % | pass |
| 482 | [prod-cast-row-menu](light/482-prod-cast-row-menu.html) | 0.000 % | 0.000 % | pass |
| 483 | [prod-cast-assign-existing-dialog](light/483-prod-cast-assign-existing-dialog.html) | 0.005 % | 0.005 % | pass |
| 484 | [prod-cast-drawer](light/484-prod-cast-drawer.html) | 0.000 % | 0.000 % | pass |
| 485 | [prod-cast-remove-confirm](light/485-prod-cast-remove-confirm.html) | 0.000 % | 0.000 % | pass |
| 486 | [prod-crew](light/486-prod-crew.html) | 0.002 % | 0.004 % | pass |
| 487 | [prod-crew-drawer](light/487-prod-crew-drawer.html) | 0.002 % | 0.004 % | pass |
| 488 | [prod-crew-remove-confirm](light/488-prod-crew-remove-confirm.html) | 0.001 % | 0.001 % | pass |
| 489 | [prod-crew-add-dialog](light/489-prod-crew-add-dialog.html) | 0.000 % | 0.000 % | pass |
| 490 | [prod-moodboard](light/490-prod-moodboard.html) | 0.000 % | 0.000 % | pass |
| 491 | [prod-moodboard-note-selected](light/491-prod-moodboard-note-selected.html) | 0.000 % | 0.000 % | pass |
| 492 | [prod-moodboard-multi-selection](light/492-prod-moodboard-multi-selection.html) | 0.000 % | 0.000 % | pass |
| 493 | [prod-moodboard-item-menu](light/493-prod-moodboard-item-menu.html) | 0.000 % | 0.000 % | pass |
| 494 | [prod-moodboard-note-drawer](light/494-prod-moodboard-note-drawer.html) | 0.000 % | 0.000 % | pass |
| 495 | [prod-moodboard-note-error](light/495-prod-moodboard-note-error.html) | 0.000 % | 0.000 % | pass |
| 496 | [prod-moodboard-link-drawer](light/496-prod-moodboard-link-drawer.html) | 0.000 % | 0.000 % | pass |
| 497 | [prod-moodboard-image-drawer](light/497-prod-moodboard-image-drawer.html) | 0.000 % | 0.000 % | pass |
| 498 | [prod-moodboard-add-image-menu](light/498-prod-moodboard-add-image-menu.html) | 0.000 % | 0.000 % | pass |
| 499 | [prod-moodboard-vault-dialog](light/499-prod-moodboard-vault-dialog.html) | 0.000 % | 0.000 % | pass |
| 500 | [prod-moodboard-add-note-dialog](light/500-prod-moodboard-add-note-dialog.html) | 0.000 % | 0.000 % | pass |
| 501 | [prod-moodboard-add-note-error](light/501-prod-moodboard-add-note-error.html) | 0.138 % | 0.136 % | pass |
| 502 | [prod-moodboard-add-link-dialog](light/502-prod-moodboard-add-link-dialog.html) | 0.000 % | 0.000 % | pass |
| 503 | [prod-moodboard-add-link-filled](light/503-prod-moodboard-add-link-filled.html) | 0.000 % | 0.000 % | pass |
| 504 | [prod-moodboard-list-menu](light/504-prod-moodboard-list-menu.html) | 0.000 % | 0.000 % | pass |
| 505 | [prod-moodboard-rename-dialog](light/505-prod-moodboard-rename-dialog.html) | 0.000 % | 0.000 % | pass |
| 506 | [prod-moodboard-board-notes-dialog](light/506-prod-moodboard-board-notes-dialog.html) | 0.000 % | 0.000 % | pass |
| 507 | [prod-moodboard-delete-confirm](light/507-prod-moodboard-delete-confirm.html) | 0.005 % | 0.004 % | pass |
| 508 | [prod-moodboard-export-dialog](light/508-prod-moodboard-export-dialog.html) | 0.000 % | 0.000 % | pass |
| 509 | [prod-moodboard-new-dialog](light/509-prod-moodboard-new-dialog.html) | 0.000 % | 0.000 % | pass |
| 510 | [prod-moodboard-new-error](light/510-prod-moodboard-new-error.html) | 0.233 % | 0.230 % | pass |
| 511 | [prod-moodboard-new-filled](light/511-prod-moodboard-new-filled.html) | 0.000 % | 0.000 % | pass |
| 512 | [prod-moodboard-empty-canvas](light/512-prod-moodboard-empty-canvas.html) | 0.000 % | 0.000 % | pass |
| 513 | [prod-storyboard-panels](light/513-prod-storyboard-panels.html) | 0.009 % | 0.010 % | pass |
| 514 | [prod-storyboard-new-dialog](light/514-prod-storyboard-new-dialog.html) | 0.000 % | 0.000 % | pass |
| 515 | [prod-storyboard-new-error](light/515-prod-storyboard-new-error.html) | 0.220 % | 0.217 % | pass |
| 516 | [prod-storyboard-no-panels](light/516-prod-storyboard-no-panels.html) | 0.000 % | 0.000 % | pass |
| 517 | [prod-storyboard-panel-menu](light/517-prod-storyboard-panel-menu.html) | 0.009 % | 0.010 % | pass |
| 518 | [prod-storyboard-move-panel-dialog](light/518-prod-storyboard-move-panel-dialog.html) | 0.000 % | 0.000 % | pass |
| 519 | [prod-storyboard-panel-drawer](light/519-prod-storyboard-panel-drawer.html) | 0.009 % | 0.010 % | pass |
| 520 | [prod-storyboard-panel-duration-error](light/520-prod-storyboard-panel-duration-error.html) | 0.009 % | 0.010 % | pass |
| 521 | [prod-storyboard-add-panel-menu](light/521-prod-storyboard-add-panel-menu.html) | 0.009 % | 0.010 % | pass |
| 522 | [prod-storyboard-sketch-dialog](light/522-prod-storyboard-sketch-dialog.html) | 0.044 % | 0.087 % | pass |
| 523 | [prod-storyboard-sketch-drawn](light/523-prod-storyboard-sketch-drawn.html) | 0.089 % | 0.111 % | pass |
| 524 | [prod-storyboard-actions-menu](light/524-prod-storyboard-actions-menu.html) | 0.010 % | 0.011 % | pass |
| 525 | [prod-storyboard-rename-dialog](light/525-prod-storyboard-rename-dialog.html) | 0.001 % | 0.000 % | pass |
| 526 | [prod-storyboard-link-scene-dialog](light/526-prod-storyboard-link-scene-dialog.html) | 0.000 % | 0.000 % | pass |
| 527 | [prod-storyboard-new-for-scene-dialog](light/527-prod-storyboard-new-for-scene-dialog.html) | 0.000 % | 0.000 % | pass |
| 528 | [prod-storyboard-delete-confirm](light/528-prod-storyboard-delete-confirm.html) | 0.006 % | 0.005 % | pass |
| 529 | [prod-storyboard-export-dialog](light/529-prod-storyboard-export-dialog.html) | 0.000 % | 0.000 % | pass |
| 530 | [prod-storyboard-scene-no-board](light/530-prod-storyboard-scene-no-board.html) | 0.138 % | 0.000 % | pass |
| 531 | [prod-shots-list](light/531-prod-shots-list.html) | 0.000 % | 0.001 % | pass |
| 532 | [prod-shots-grid](light/532-prod-shots-grid.html) | 0.003 % | 0.003 % | pass |
| 533 | [prod-shots-all-scenes](light/533-prod-shots-all-scenes.html) | 0.001 % | 0.002 % | pass |
| 534 | [prod-shots-scene-empty](light/534-prod-shots-scene-empty.html) | 0.000 % | 0.001 % | pass |
| 535 | [prod-shots-composer-error](light/535-prod-shots-composer-error.html) | 0.165 % | 0.001 % | pass |
| 536 | [prod-shot-menu](light/536-prod-shot-menu.html) | 0.000 % | 0.001 % | pass |
| 537 | [prod-shot-drawer](light/537-prod-shot-drawer.html) | 0.007 % | 0.007 % | pass |
| 538 | [prod-shot-move-dialog](light/538-prod-shot-move-dialog.html) | 0.000 % | 0.000 % | pass |
| 539 | [prod-shot-attach-dialog](light/539-prod-shot-attach-dialog.html) | 0.000 % | 0.000 % | pass |
| 540 | [prod-shots-scene-actions-menu](light/540-prod-shots-scene-actions-menu.html) | 0.000 % | 0.001 % | pass |
| 541 | [prod-shots-copy-planning-dialog](light/541-prod-shots-copy-planning-dialog.html) | 0.000 % | 0.000 % | pass |
| 542 | [prod-shots-export-dialog](light/542-prod-shots-export-dialog.html) | 0.000 % | 0.000 % | pass |
| 543 | [prod-schedule-board](light/543-prod-schedule-board.html) | 0.016 % | 0.014 % | pass |
| 544 | [prod-schedule-list](light/544-prod-schedule-list.html) | 0.015 % | 0.013 % | pass |
| 545 | [prod-schedule-calendar](light/545-prod-schedule-calendar.html) | 0.020 % | 0.018 % | pass |
| 546 | [prod-schedule-calendar-next-month](light/546-prod-schedule-calendar-next-month.html) | 0.015 % | 0.013 % | pass |
| 547 | [prod-schedule-filter-board](light/547-prod-schedule-filter-board.html) | 0.018 % | 0.016 % | pass |
| 548 | [prod-schedule-filter-list](light/548-prod-schedule-filter-list.html) | 0.017 % | 0.015 % | pass |
| 549 | [prod-schedule-filter-no-match](light/549-prod-schedule-filter-no-match.html) | 0.015 % | 0.013 % | pass |
| 550 | [prod-schedule-create-day-dialog](light/550-prod-schedule-create-day-dialog.html) | 0.005 % | 0.006 % | pass |
| 551 | [prod-schedule-create-day-filled](light/551-prod-schedule-create-day-filled.html) | 0.005 % | 0.006 % | pass |
| 552 | [prod-schedule-day-created](light/552-prod-schedule-day-created.html) | 0.017 % | 0.016 % | pass |
| 553 | [prod-schedule-day-detail-empty](light/553-prod-schedule-day-detail-empty.html) | 0.001 % | 0.001 % | pass |
| 554 | [prod-schedule-off-day-dialog](light/554-prod-schedule-off-day-dialog.html) | 0.006 % | 0.007 % | pass |
| 555 | [prod-schedule-off-day-filled](light/555-prod-schedule-off-day-filled.html) | 0.006 % | 0.007 % | pass |
| 556 | [prod-schedule-off-day-created](light/556-prod-schedule-off-day-created.html) | 0.017 % | 0.015 % | pass |
| 557 | [prod-schedule-day-menu](light/557-prod-schedule-day-menu.html) | 0.021 % | 0.018 % | pass |
| 558 | [prod-schedule-edit-date-dialog](light/558-prod-schedule-edit-date-dialog.html) | 0.005 % | 0.007 % | pass |
| 559 | [prod-schedule-day-menu-empty-day](light/559-prod-schedule-day-menu-empty-day.html) | 0.019 % | 0.017 % | pass |
| 560 | [prod-schedule-edit-date-dialog-no-call-sheet](light/560-prod-schedule-edit-date-dialog-no-call-sheet.html) | 0.005 % | 0.006 % | pass |
| 561 | [prod-schedule-strip-menu-scheduled](light/561-prod-schedule-strip-menu-scheduled.html) | 0.018 % | 0.016 % | pass |
| 562 | [prod-schedule-strip-menu-unscheduled](light/562-prod-schedule-strip-menu-unscheduled.html) | 0.017 % | 0.015 % | pass |
| 563 | [prod-schedule-strip-moved](light/563-prod-schedule-strip-moved.html) | 0.017 % | 0.015 % | pass |
| 564 | [prod-schedule-marker-dialog](light/564-prod-schedule-marker-dialog.html) | 0.008 % | 0.009 % | pass |
| 565 | [prod-schedule-marker-error-duration](light/565-prod-schedule-marker-error-duration.html) | 0.094 % | 0.094 % | pass |
| 566 | [prod-schedule-marker-error-note](light/566-prod-schedule-marker-error-note.html) | 0.094 % | 0.094 % | pass |
| 567 | [prod-schedule-marker-dialog-filled](light/567-prod-schedule-marker-dialog-filled.html) | 0.008 % | 0.009 % | pass |
| 568 | [prod-schedule-marker-added](light/568-prod-schedule-marker-added.html) | 0.017 % | 0.016 % | pass |
| 569 | [prod-schedule-marker-edit-dialog](light/569-prod-schedule-marker-edit-dialog.html) | 0.008 % | 0.009 % | pass |
| 570 | [prod-schedule-strip-drawer](light/570-prod-schedule-strip-drawer.html) | 0.021 % | 0.019 % | pass |
| 571 | [prod-schedule-strip-drawer-error-estimate](light/571-prod-schedule-strip-drawer-error-estimate.html) | 0.021 % | 0.019 % | pass |
| 572 | [prod-schedule-strip-drawer-error-start](light/572-prod-schedule-strip-drawer-error-start.html) | 0.021 % | 0.019 % | pass |
| 573 | [prod-schedule-strip-drawer-error-pages](light/573-prod-schedule-strip-drawer-error-pages.html) | 0.021 % | 0.019 % | pass |
| 574 | [prod-schedule-strip-drawer-unscheduled](light/574-prod-schedule-strip-drawer-unscheduled.html) | 0.017 % | 0.016 % | pass |
| 575 | [prod-schedule-strip-drawer-scheduled-day-1](light/575-prod-schedule-strip-drawer-scheduled-day-1.html) | 0.021 % | 0.019 % | pass |
| 576 | [prod-schedule-warnings-drawer](light/576-prod-schedule-warnings-drawer.html) | 0.018 % | 0.017 % | pass |
| 577 | [prod-schedule-warnings-kept](light/577-prod-schedule-warnings-kept.html) | 0.083 % | 0.017 % | pass |
| 578 | [prod-schedule-warnings-dismissed](light/578-prod-schedule-warnings-dismissed.html) | 0.018 % | 0.017 % | pass |
| 579 | [prod-schedule-warning-open-day](light/579-prod-schedule-warning-open-day.html) | 0.004 % | 0.003 % | pass |
| 580 | [prod-schedule-suggestions-drawer](light/580-prod-schedule-suggestions-drawer.html) | 0.259 % | 0.017 % | pass |
| 581 | [prod-schedule-suggestions-apply-dialog](light/581-prod-schedule-suggestions-apply-dialog.html) | 0.008 % | 0.010 % | pass |
| 582 | [prod-schedule-settings-dialog](light/582-prod-schedule-settings-dialog.html) | 0.008 % | 0.009 % | pass |
| 583 | [prod-schedule-settings-error](light/583-prod-schedule-settings-error.html) | 0.100 % | 0.100 % | pass |
| 584 | [prod-schedule-settings-finalized-selected](light/584-prod-schedule-settings-finalized-selected.html) | 0.008 % | 0.009 % | pass |
| 585 | [prod-schedule-finalized-banner](light/585-prod-schedule-finalized-banner.html) | 0.022 % | 0.021 % | pass |
| 586 | [prod-schedule-settings-reopen](light/586-prod-schedule-settings-reopen.html) | 0.009 % | 0.010 % | pass |
| 587 | [prod-schedule-settings-strict](light/587-prod-schedule-settings-strict.html) | 0.022 % | 0.023 % | pass |
| 588 | [prod-schedule-strict-chip](light/588-prod-schedule-strict-chip.html) | 0.018 % | 0.017 % | pass |
| 589 | [prod-schedule-keep-anyway-dialog](light/589-prod-schedule-keep-anyway-dialog.html) | 0.008 % | 0.010 % | pass |
| 590 | [prod-schedule-day-detail](light/590-prod-schedule-day-detail.html) | 0.004 % | 0.003 % | pass |
| 591 | [prod-schedule-day-detail-add-break-menu](light/591-prod-schedule-day-detail-add-break-menu.html) | 0.004 % | 0.003 % | pass |
| 592 | [prod-schedule-day-detail-actions-menu](light/592-prod-schedule-day-detail-actions-menu.html) | 0.006 % | 0.005 % | pass |
| 593 | [prod-schedule-day-export-dialog](light/593-prod-schedule-day-export-dialog.html) | 0.001 % | 0.001 % | pass |
| 594 | [prod-schedule-day-detail-timeline](light/594-prod-schedule-day-detail-timeline.html) | 0.005 % | 0.005 % | pass |
| 595 | [prod-schedule-day-detail-over-target](light/595-prod-schedule-day-detail-over-target.html) | 0.006 % | 0.006 % | pass |
| 596 | [prod-schedule-day-detail-time-error](light/596-prod-schedule-day-detail-time-error.html) | 0.005 % | 0.005 % | pass |
| 597 | [prod-schedule-day-detail-off](light/597-prod-schedule-day-detail-off.html) | 0.001 % | 0.001 % | pass |
| 598 | [prod-schedule-export-dialog](light/598-prod-schedule-export-dialog.html) | 0.008 % | 0.009 % | pass |
| 599 | [prod-dood-table](light/599-prod-dood-table.html) | 0.006 % | 0.006 % | pass |
| 600 | [prod-dood-cell-menu](light/600-prod-dood-cell-menu.html) | 0.006 % | 0.006 % | pass |
| 601 | [prod-dood-corrected](light/601-prod-dood-corrected.html) | 0.007 % | 0.007 % | pass |
| 602 | [prod-dood-note-dialog](light/602-prod-dood-note-dialog.html) | 0.004 % | 0.004 % | pass |
| 603 | [prod-dood-simple-codes](light/603-prod-dood-simple-codes.html) | 0.003 % | 0.005 % | pass |
| 604 | [prod-dood-one-cast](light/604-prod-dood-one-cast.html) | 0.005 % | 0.005 % | pass |
| 605 | [prod-dood-day-range](light/605-prod-dood-day-range.html) | 0.006 % | 0.006 % | pass |
| 606 | [prod-dood-export-dialog](light/606-prod-dood-export-dialog.html) | 0.004 % | 0.004 % | pass |
| 607 | [prod-daily-day-1](light/607-prod-daily-day-1.html) | 0.009 % | 0.008 % | pass |
| 608 | [prod-daily-no-call-sheet](light/608-prod-daily-no-call-sheet.html) | 0.007 % | 0.006 % | pass |
| 609 | [prod-daily-day-4](light/609-prod-daily-day-4.html) | 0.002 % | 0.002 % | pass |
| 610 | [prod-daily-off-day](light/610-prod-daily-off-day.html) | 0.000 % | 0.000 % | pass |
| 611 | [prod-daily-export-dialog](light/611-prod-daily-export-dialog.html) | 0.004 % | 0.003 % | pass |
| 612 | [prod-sides-setup](light/612-prod-sides-setup.html) | 0.012 % | 0.011 % | pass |
| 613 | [prod-sides-day-4](light/613-prod-sides-day-4.html) | 0.017 % | 0.016 % | pass |
| 614 | [prod-sides-selected-cast-none](light/614-prod-sides-selected-cast-none.html) | 0.011 % | 0.011 % | pass |
| 615 | [prod-sides-selected-cast](light/615-prod-sides-selected-cast.html) | 0.017 % | 0.016 % | pass |
| 616 | [prod-sides-cover-page](light/616-prod-sides-cover-page.html) | 0.028 % | 0.027 % | pass |
| 617 | [prod-sides-scene-unchecked](light/617-prod-sides-scene-unchecked.html) | 0.019 % | 0.018 % | pass |
| 618 | [prod-sides-no-scenes](light/618-prod-sides-no-scenes.html) | 0.000 % | 0.000 % | pass |
| 619 | [prod-sides-saved](light/619-prod-sides-saved.html) | 0.158 % | 0.154 % | pass |
| 620 | [prod-sides-live-with-saved](light/620-prod-sides-live-with-saved.html) | 0.012 % | 0.011 % | pass |
| 621 | [prod-sides-export-dialog](light/621-prod-sides-export-dialog.html) | 0.007 % | 0.007 % | pass |
| 622 | [prod-reports-scene](light/622-prod-reports-scene.html) | 0.001 % | 0.001 % | pass |
| 623 | [prod-reports-location](light/623-prod-reports-location.html) | 0.001 % | 0.001 % | pass |
| 624 | [prod-reports-cast-scene](light/624-prod-reports-cast-scene.html) | 0.002 % | 0.003 % | pass |
| 625 | [prod-reports-prop](light/625-prod-reports-prop.html) | 0.002 % | 0.003 % | pass |
| 626 | [prod-reports-schedule](light/626-prod-reports-schedule.html) | 0.002 % | 0.003 % | pass |
| 627 | [prod-reports-breakdown-completeness](light/627-prod-reports-breakdown-completeness.html) | 0.001 % | 0.001 % | pass |
| 628 | [prod-reports-filtered](light/628-prod-reports-filtered.html) | 0.002 % | 0.003 % | pass |
| 629 | [prod-reports-filter-no-match](light/629-prod-reports-filter-no-match.html) | 0.002 % | 0.003 % | pass |
| 630 | [prod-reports-saved-snapshot](light/630-prod-reports-saved-snapshot.html) | 0.003 % | 0.003 % | pass |
| 631 | [prod-reports-live-with-snapshots](light/631-prod-reports-live-with-snapshots.html) | 0.001 % | 0.001 % | pass |
| 632 | [prod-reports-export-dialog](light/632-prod-reports-export-dialog.html) | 0.000 % | 0.000 % | pass |
| 633 | [prod-budget-filled](light/633-prod-budget-filled.html) | 0.005 % | 0.005 % | pass |
| 634 | [prod-budget-add-line-dialog](light/634-prod-budget-add-line-dialog.html) | 0.001 % | 0.001 % | pass |
| 635 | [prod-budget-add-line-error-description](light/635-prod-budget-add-line-error-description.html) | 0.133 % | 0.126 % | pass |
| 636 | [prod-budget-add-line-error-planned](light/636-prod-budget-add-line-error-planned.html) | 0.131 % | 0.126 % | pass |
| 637 | [prod-budget-add-line-filled](light/637-prod-budget-add-line-filled.html) | 0.001 % | 0.001 % | pass |
| 638 | [prod-budget-line-added](light/638-prod-budget-line-added.html) | 0.005 % | 0.005 % | pass |
| 639 | [prod-budget-edit-line-dialog](light/639-prod-budget-edit-line-dialog.html) | 0.001 % | 0.001 % | pass |
| 640 | [prod-budget-edit-plan-dialog](light/640-prod-budget-edit-plan-dialog.html) | 0.001 % | 0.001 % | pass |
| 641 | [prod-budget-edit-plan-error](light/641-prod-budget-edit-plan-error.html) | 0.093 % | 0.091 % | pass |
| 642 | [prod-budget-snapshot-name-dialog](light/642-prod-budget-snapshot-name-dialog.html) | 0.001 % | 0.001 % | pass |
| 643 | [prod-budget-snapshots-saved](light/643-prod-budget-snapshots-saved.html) | 0.005 % | 0.003 % | pass |
| 644 | [prod-budget-snapshot-view](light/644-prod-budget-snapshot-view.html) | 0.002 % | 0.002 % | pass |
| 645 | [prod-budget-export-dialog](light/645-prod-budget-export-dialog.html) | 0.001 % | 0.001 % | pass |
| 646 | [cs-list](light/646-cs-list.html) | 0.000 % | 0.000 % | pass |
| 647 | [cs-row-context-menu](light/647-cs-row-context-menu.html) | 0.000 % | 0.000 % | pass |
| 648 | [cs-row-actions-menu](light/648-cs-row-actions-menu.html) | 0.000 % | 0.000 % | pass |
| 649 | [cs-search](light/649-cs-search.html) | 0.000 % | 0.000 % | pass |
| 650 | [cs-search-no-match](light/650-cs-search-no-match.html) | 0.000 % | 0.000 % | pass |
| 651 | [cs-filter-draft](light/651-cs-filter-draft.html) | 0.000 % | 0.000 % | pass |
| 652 | [cs-filter-final-none](light/652-cs-filter-final-none.html) | 0.000 % | 0.000 % | pass |
| 653 | [cs-create-dialog](light/653-cs-create-dialog.html) | 0.000 % | 0.001 % | pass |
| 654 | [cs-create-dialog-day-chosen](light/654-cs-create-dialog-day-chosen.html) | 0.000 % | 0.001 % | pass |
| 655 | [cs-editor-draft](light/655-cs-editor-draft.html) | 0.005 % | 0.005 % | pass |
| 656 | [cs-editor-draft-lower](light/656-cs-editor-draft-lower.html) | 0.004 % | 0.004 % | pass |
| 657 | [cs-add-section-menu](light/657-cs-add-section-menu.html) | 0.012 % | 0.010 % | pass |
| 658 | [cs-section-weather](light/658-cs-section-weather.html) | 0.004 % | 0.004 % | pass |
| 659 | [cs-section-extras](light/659-cs-section-extras.html) | 0.362 % | 0.360 % | pass |
| 660 | [cs-section-departments](light/660-cs-section-departments.html) | 0.338 % | 0.004 % | pass |
| 661 | [cs-section-advance](light/661-cs-section-advance.html) | 0.145 % | 0.004 % | pass |
| 662 | [cs-section-crew](light/662-cs-section-crew.html) | 0.145 % | 0.004 % | pass |
| 663 | [cs-section-communication](light/663-cs-section-communication.html) | 0.004 % | 0.004 % | pass |
| 664 | [cs-section-special-notes](light/664-cs-section-special-notes.html) | 0.004 % | 0.004 % | pass |
| 665 | [cs-section-attachments](light/665-cs-section-attachments.html) | 0.004 % | 0.004 % | pass |
| 666 | [cs-section-reference-images](light/666-cs-section-reference-images.html) | 0.004 % | 0.004 % | pass |
| 667 | [cs-section-custom-field](light/667-cs-section-custom-field.html) | 0.004 % | 0.004 % | pass |
| 668 | [cs-finalize-dialog-missing-crew-call](light/668-cs-finalize-dialog-missing-crew-call.html) | 0.005 % | 0.003 % | pass |
| 669 | [cs-finalize-crew-call-required](light/669-cs-finalize-crew-call-required.html) | 0.017 % | 0.017 % | pass |
| 670 | [cs-needs-refresh-banner](light/670-cs-needs-refresh-banner.html) | 0.014 % | 0.014 % | pass |
| 671 | [cs-finalize-stale-dialog](light/671-cs-finalize-stale-dialog.html) | 0.012 % | 0.010 % | pass |
| 672 | [cs-refresh-dialog](light/672-cs-refresh-dialog.html) | 0.003 % | 0.002 % | pass |
| 673 | [cs-refreshed](light/673-cs-refreshed.html) | 0.005 % | 0.005 % | pass |
| 674 | [cs-refresh-dialog-no-changes](light/674-cs-refresh-dialog-no-changes.html) | 0.003 % | 0.002 % | pass |
| 675 | [cs-editor-export-dialog](light/675-cs-editor-export-dialog.html) | 0.003 % | 0.002 % | pass |
| 676 | [cs-finalize-dialog](light/676-cs-finalize-dialog.html) | 0.007 % | 0.006 % | pass |
| 677 | [cs-final](light/677-cs-final.html) | 0.001 % | 0.001 % | pass |
| 678 | [cs-final-schedule-changed](light/678-cs-final-schedule-changed.html) | 0.011 % | 0.010 % | pass |
| 679 | [cs-new-revision-dialog](light/679-cs-new-revision-dialog.html) | 0.001 % | 0.001 % | pass |
| 680 | [cs-revision-v2](light/680-cs-revision-v2.html) | 0.005 % | 0.005 % | pass |
| 681 | [cs-replaced-by-newer-revision](light/681-cs-replaced-by-newer-revision.html) | 0.001 % | 0.001 % | pass |
| 682 | [cs-list-with-revisions](light/682-cs-list-with-revisions.html) | 0.246 % | 0.241 % | pass |
| 683 | [cs-list-show-replaced](light/683-cs-list-show-replaced.html) | 0.013 % | 0.014 % | pass |
| 684 | [cs-filter-needs-refresh](light/684-cs-filter-needs-refresh.html) | 0.000 % | 0.000 % | pass |
| 685 | [cs-filter-final](light/685-cs-filter-final.html) | 0.000 % | 0.000 % | pass |
| 686 | [cs-list-export-dialog](light/686-cs-list-export-dialog.html) | 0.000 % | 0.000 % | pass |
| 687 | [cs-create-dialog-every-day-done](light/687-cs-create-dialog-every-day-done.html) | 0.000 % | 0.000 % | pass |
| 688 | [prod-schedule-day-deleted](light/688-prod-schedule-day-deleted.html) | 0.015 % | 0.014 % | pass |
| 689 | [cs-list-day-deleted](light/689-cs-list-day-deleted.html) | 0.000 % | 0.000 % | pass |
| 690 | [cs-day-deleted-banner](light/690-cs-day-deleted-banner.html) | 0.005 % | 0.005 % | pass |
| 691 | [cs-deleted-toast](light/691-cs-deleted-toast.html) | 0.000 % | 0.000 % | pass |
| 692 | [prod-schedule-empty-no-source](light/692-prod-schedule-empty-no-source.html) | 0.000 % | 0.000 % | pass |
| 693 | [prod-dood-empty](light/693-prod-dood-empty.html) | 0.000 % | 0.000 % | pass |
| 694 | [prod-daily-empty](light/694-prod-daily-empty.html) | 0.000 % | 0.000 % | pass |
| 695 | [prod-sides-empty](light/695-prod-sides-empty.html) | 0.000 % | 0.000 % | pass |
| 696 | [prod-reports-empty](light/696-prod-reports-empty.html) | 0.001 % | 0.001 % | pass |
| 697 | [prod-budget-start](light/697-prod-budget-start.html) | 0.000 % | 0.000 % | pass |
| 698 | [cs-list-empty](light/698-cs-list-empty.html) | 0.000 % | 0.000 % | pass |
| 699 | [cs-create-dialog-no-schedule](light/699-cs-create-dialog-no-schedule.html) | 0.000 % | 0.000 % | pass |
| 700 | [files-list](light/700-files-list.html) | 0.000 % | 0.000 % | pass |
| 701 | [files-grid](light/701-files-grid.html) | 0.000 % | 0.000 % | pass |
| 702 | [files-folder-contracts](light/702-files-folder-contracts.html) | 0.000 % | 0.000 % | pass |
| 703 | [files-folder-top-level](light/703-files-folder-top-level.html) | 0.000 % | 0.000 % | pass |
| 704 | [files-search-results](light/704-files-search-results.html) | 0.003 % | 0.003 % | pass |
| 705 | [files-search-no-match](light/705-files-search-no-match.html) | 0.003 % | 0.003 % | pass |
| 706 | [files-add-menu](light/706-files-add-menu.html) | 0.000 % | 0.000 % | pass |
| 707 | [files-row-menu](light/707-files-row-menu.html) | 0.000 % | 0.000 % | pass |
| 708 | [files-row-context-menu](light/708-files-row-context-menu.html) | 0.001 % | 0.001 % | pass |
| 709 | [files-details-drawer](light/709-files-details-drawer.html) | 0.004 % | 0.004 % | pass |
| 710 | [files-details-notes](light/710-files-details-notes.html) | 0.004 % | 0.004 % | pass |
| 711 | [files-details-renamed](light/711-files-details-renamed.html) | 0.004 % | 0.004 % | pass |
| 712 | [files-file-deleted-toast](light/712-files-file-deleted-toast.html) | 0.000 % | 0.000 % | pass |
| 713 | [files-linked-file](light/713-files-linked-file.html) | 0.000 % | 0.000 % | pass |
| 714 | [files-unavailable-linked-file](light/714-files-unavailable-linked-file.html) | 0.000 % | 0.000 % | pass |
| 715 | [files-unavailable-details](light/715-files-unavailable-details.html) | 0.004 % | 0.004 % | pass |
| 716 | [files-linked-file-deleted-toast](light/716-files-linked-file-deleted-toast.html) | 0.000 % | 0.000 % | pass |
| 717 | [files-new-folder-dialog](light/717-files-new-folder-dialog.html) | 0.000 % | 0.000 % | pass |
| 718 | [files-new-folder-filled](light/718-files-new-folder-filled.html) | 0.195 % | 0.197 % | pass |
| 719 | [files-folder-created](light/719-files-folder-created.html) | 0.000 % | 0.000 % | pass |
| 720 | [files-rename-folder-dialog](light/720-files-rename-folder-dialog.html) | 0.000 % | 0.000 % | pass |
| 721 | [files-delete-folder-confirm](light/721-files-delete-folder-confirm.html) | 0.000 % | 0.000 % | pass |
| 722 | [files-folder-deleted-toast](light/722-files-folder-deleted-toast.html) | 0.000 % | 0.000 % | pass |
| 723 | [files-empty](light/723-files-empty.html) | 0.000 % | 0.000 % | pass |
| 724 | [notes-empty](light/724-notes-empty.html) | 0.000 % | 0.000 % | pass |
| 725 | [activity-new-project](light/725-activity-new-project.html) | 0.001 % | 0.001 % | pass |
| 726 | [trash-empty](light/726-trash-empty.html) | 0.000 % | 0.000 % | pass |
| 727 | [notes-filled](light/727-notes-filled.html) | 0.000 % | 0.000 % | pass |
| 728 | [notes-search-filtered](light/728-notes-search-filtered.html) | 0.003 % | 0.003 % | pass |
| 729 | [notes-search-no-match](light/729-notes-search-no-match.html) | 0.003 % | 0.003 % | pass |
| 730 | [notes-note-editor](light/730-notes-note-editor.html) | 0.006 % | 0.006 % | pass |
| 731 | [notes-note-editor-pinned](light/731-notes-note-editor-pinned.html) | 0.006 % | 0.006 % | pass |
| 732 | [notes-new-note-editor](light/732-notes-new-note-editor.html) | 0.006 % | 0.006 % | pass |
| 733 | [notes-new-note-filled](light/733-notes-new-note-filled.html) | 0.006 % | 0.006 % | pass |
| 734 | [notes-note-deleted-toast](light/734-notes-note-deleted-toast.html) | 0.000 % | 0.000 % | pass |
| 735 | [notes-task-editor-new](light/735-notes-task-editor-new.html) | 0.163 % | 0.194 % | pass |
| 736 | [notes-task-editor-title-error](light/736-notes-task-editor-title-error.html) | 0.132 % | 0.130 % | pass |
| 737 | [notes-task-editor-filled](light/737-notes-task-editor-filled.html) | 0.000 % | 0.000 % | pass |
| 738 | [notes-task-editor-related-results](light/738-notes-task-editor-related-results.html) | 0.003 % | 0.003 % | pass |
| 739 | [notes-task-editor-related-chosen](light/739-notes-task-editor-related-chosen.html) | 0.000 % | 0.000 % | pass |
| 740 | [notes-task-added-toast](light/740-notes-task-added-toast.html) | 0.003 % | 0.002 % | pass |
| 741 | [notes-task-editor-existing](light/741-notes-task-editor-existing.html) | 0.001 % | 0.001 % | pass |
| 742 | [notes-tasks-done](light/742-notes-tasks-done.html) | 0.001 % | 0.001 % | pass |
| 743 | [notes-tasks-all](light/743-notes-tasks-all.html) | 0.003 % | 0.002 % | pass |
| 744 | [notes-task-completed-toast](light/744-notes-task-completed-toast.html) | 0.000 % | 0.000 % | pass |
| 745 | [notes-task-deleted-toast](light/745-notes-task-deleted-toast.html) | 0.000 % | 0.000 % | pass |
| 746 | [activity-all](light/746-activity-all.html) | 0.011 % | 0.010 % | pass |
| 747 | [activity-filter-drafts](light/747-activity-filter-drafts.html) | 0.001 % | 0.001 % | pass |
| 748 | [activity-filter-story](light/748-activity-filter-story.html) | 0.009 % | 0.007 % | pass |
| 749 | [activity-filter-production](light/749-activity-filter-production.html) | 0.014 % | 0.013 % | pass |
| 750 | [activity-filter-exchange](light/750-activity-filter-exchange.html) | 0.000 % | 0.000 % | pass |
| 751 | [trash-filled](light/751-trash-filled.html) | 0.007 % | 0.005 % | pass |
| 752 | [trash-search-filtered](light/752-trash-search-filtered.html) | 0.006 % | 0.007 % | pass |
| 753 | [trash-search-no-match](light/753-trash-search-no-match.html) | 0.003 % | 0.003 % | pass |
| 754 | [trash-purge-confirm](light/754-trash-purge-confirm.html) | 0.003 % | 0.005 % | pass |
| 755 | [trash-global-empty](light/755-trash-global-empty.html) | 0.000 % | 0.000 % | pass |
| 756 | [trash-global-filled](light/756-trash-global-filled.html) | 0.000 % | 0.000 % | pass |
| 757 | [trash-global-purge-confirm](light/757-trash-global-purge-confirm.html) | 0.002 % | 0.001 % | pass |
| 758 | [trash-purged-toast](light/758-trash-purged-toast.html) | 0.000 % | 0.000 % | pass |
| 759 | [templates-page](light/759-templates-page.html) | 0.003 % | 0.003 % | pass |
| 760 | [templates-actions-menu-builtin](light/760-templates-actions-menu-builtin.html) | 0.002 % | 0.002 % | pass |
| 761 | [templates-preview-dialog](light/761-templates-preview-dialog.html) | 0.001 % | 0.000 % | pass |
| 762 | [templates-create-dialog](light/762-templates-create-dialog.html) | 0.001 % | 0.000 % | pass |
| 763 | [templates-create-name-error](light/763-templates-create-name-error.html) | 0.107 % | 0.106 % | pass |
| 764 | [templates-create-filled](light/764-templates-create-filled.html) | 0.001 % | 0.000 % | pass |
| 765 | [templates-created-toast](light/765-templates-created-toast.html) | 0.003 % | 0.003 % | pass |
| 766 | [templates-actions-menu-own](light/766-templates-actions-menu-own.html) | 0.006 % | 0.005 % | pass |
| 767 | [templates-edit-dialog](light/767-templates-edit-dialog.html) | 0.001 % | 0.000 % | pass |
| 768 | [templates-apply-dialog](light/768-templates-apply-dialog.html) | 0.001 % | 0.000 % | pass |
| 769 | [templates-delete-confirm](light/769-templates-delete-confirm.html) | 0.001 % | 0.000 % | pass |
| 770 | [templates-global-deleted-toast](light/770-templates-global-deleted-toast.html) | 0.003 % | 0.003 % | pass |
| 771 | [templates-project-deleted-toast](light/771-templates-project-deleted-toast.html) | 0.003 % | 0.003 % | pass |
| 772 | [settings-general](light/772-settings-general.html) | 0.002 % | 0.002 % | pass |
| 773 | [settings-general-dirty](light/773-settings-general-dirty.html) | 0.002 % | 0.002 % | pass |
| 774 | [settings-general-title-error](light/774-settings-general-title-error.html) | 0.002 % | 0.002 % | pass |
| 775 | [settings-general-lower](light/775-settings-general-lower.html) | 0.002 % | 0.002 % | pass |
| 776 | [settings-export-diagnostics-dialog](light/776-settings-export-diagnostics-dialog.html) | 0.000 % | 0.001 % | pass |
| 777 | [settings-export-diagnostics-contents](light/777-settings-export-diagnostics-contents.html) | 0.000 % | 0.001 % | pass |
| 778 | [settings-writing](light/778-settings-writing.html) | 0.000 % | 0.000 % | pass |
| 779 | [settings-writing-word-added](light/779-settings-writing-word-added.html) | 0.000 % | 0.000 % | pass |
| 780 | [settings-writing-dictionary-filled](light/780-settings-writing-dictionary-filled.html) | 0.000 % | 0.000 % | pass |
| 781 | [settings-writing-dictionary-filtered](light/781-settings-writing-dictionary-filtered.html) | 0.000 % | 0.000 % | pass |
| 782 | [settings-writing-word-removed-toast](light/782-settings-writing-word-removed-toast.html) | 0.000 % | 0.000 % | pass |
| 783 | [settings-writing-spelling-toggled](light/783-settings-writing-spelling-toggled.html) | 0.003 % | 0.003 % | pass |
| 784 | [settings-writing-word-error](light/784-settings-writing-word-error.html) | 0.116 % | 0.000 % | pass |
| 785 | [settings-shortcuts](light/785-settings-shortcuts.html) | 0.014 % | 0.003 % | pass |
| 786 | [settings-shortcuts-filtered](light/786-settings-shortcuts-filtered.html) | 0.004 % | 0.003 % | pass |
| 787 | [settings-shortcuts-no-match](light/787-settings-shortcuts-no-match.html) | 0.000 % | 0.000 % | pass |
| 788 | [settings-shortcut-recording](light/788-settings-shortcut-recording.html) | 0.023 % | 0.010 % | pass |
| 789 | [settings-shortcut-invalid-plain-key](light/789-settings-shortcut-invalid-plain-key.html) | 0.075 % | 0.071 % | pass |
| 790 | [settings-shortcut-invalid-reserved](light/790-settings-shortcut-invalid-reserved.html) | 0.059 % | 0.055 % | pass |
| 791 | [settings-shortcut-duplicate-error](light/791-settings-shortcut-duplicate-error.html) | 0.086 % | 0.004 % | pass |
| 792 | [settings-shortcut-changed](light/792-settings-shortcut-changed.html) | 0.084 % | 0.003 % | pass |
| 793 | [settings-shortcut-conflict-dialog](light/793-settings-shortcut-conflict-dialog.html) | 0.000 % | 0.000 % | pass |
| 794 | [settings-shortcut-moved](light/794-settings-shortcut-moved.html) | 0.009 % | 0.006 % | pass |
| 795 | [settings-shortcut-restore-conflict-dialog](light/795-settings-shortcut-restore-conflict-dialog.html) | 0.001 % | 0.000 % | pass |
| 796 | [settings-shortcut-restore-all-confirm](light/796-settings-shortcut-restore-all-confirm.html) | 0.001 % | 0.000 % | pass |
| 797 | [settings-shortcuts-restored-toast](light/797-settings-shortcuts-restored-toast.html) | 0.014 % | 0.003 % | pass |
| 798 | [settings-shortcuts-conflict-banner](light/798-settings-shortcuts-conflict-banner.html) | 0.009 % | 0.007 % | pass |
| 799 | [settings-offline-ai-ready](light/799-settings-offline-ai-ready.html) | 0.001 % | 0.001 % | pass |
| 800 | [settings-offline-ai-remove-confirm](light/800-settings-offline-ai-remove-confirm.html) | 0.001 % | 0.001 % | pass |
| 801 | [settings-offline-ai-technical-details](light/801-settings-offline-ai-technical-details.html) | 0.029 % | 0.024 % | pass |
| 802 | [settings-offline-ai-stopped-toast](light/802-settings-offline-ai-stopped-toast.html) | 0.241 % | 0.024 % | pass |
| 803 | [settings-offline-ai-graphics-off](light/803-settings-offline-ai-graphics-off.html) | 0.029 % | 0.024 % | pass |
| 804 | [settings-advanced](light/804-settings-advanced.html) | 0.000 % | 0.000 % | pass |
| 805 | [settings-advanced-changed-toast](light/805-settings-advanced-changed-toast.html) | 0.000 % | 0.000 % | pass |
| 806 | [settings-advanced-restored-toast](light/806-settings-advanced-restored-toast.html) | 0.000 % | 0.000 % | pass |
| 807 | [settings-appearance](light/807-settings-appearance.html) | 0.000 % | 0.000 % | pass |
| 808 | [settings-appearance-my-themes-empty](light/808-settings-appearance-my-themes-empty.html) | 0.000 % | 0.000 % | pass |
| 809 | [settings-theme-paste-dialog](light/809-settings-theme-paste-dialog.html) | 0.000 % | 0.000 % | pass |
| 810 | [settings-theme-too-large](light/810-settings-theme-too-large.html) | 0.000 % | 0.000 % | pass |
| 811 | [settings-theme-paste-filled](light/811-settings-theme-paste-filled.html) | 0.000 % | 0.000 % | pass |
| 812 | [settings-theme-check-corrected](light/812-settings-theme-check-corrected.html) | 0.001 % | 0.000 % | pass |
| 813 | [settings-theme-check-readability](light/813-settings-theme-check-readability.html) | 0.001 % | 0.000 % | pass |
| 814 | [settings-theme-check-rejected](light/814-settings-theme-check-rejected.html) | 0.000 % | 0.000 % | pass |
| 815 | [settings-theme-check-ok](light/815-settings-theme-check-ok.html) | 0.004 % | 0.003 % | pass |
| 816 | [settings-my-themes-filled](light/816-settings-my-themes-filled.html) | 0.030 % | 0.030 % | pass |
| 817 | [settings-my-themes-rename-dialog](light/817-settings-my-themes-rename-dialog.html) | 0.000 % | 0.000 % | pass |
| 818 | [settings-my-themes-delete-confirm](light/818-settings-my-themes-delete-confirm.html) | 0.000 % | 0.000 % | pass |
| 819 | [settings-my-themes-deleted-toast](light/819-settings-my-themes-deleted-toast.html) | 0.030 % | 0.030 % | pass |

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
