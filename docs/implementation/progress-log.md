# Progress log

> The running record of work on the [build plan](masterplan.md). Every Claude Code session adds one entry
> (newest first, under "Entries"). Rules for the entry are in the repository `CLAUDE.md`. This log reports; it is not
> evidence. Evidence lives under `docs/release/evidence/` (older records under `docs/history/evidence/`) and in pull
> requests. Entries up to 2026-10-09 are in [history/logs](../history/logs/progress-log-to-2026-10-09.md).

## Dashboard (update every session from `npm run plan:status`)

**Overall: 133 of 307 boxes (43.3%).** **Current goal** (boxes without the "Later: store publishing" tag): **132 of 224 (58.9%)**. Updated 2026-10-27 after the Mac sync with Windows (the box and area changes came with the Windows documents; no Mac box ticked).

| # | Area | Done |
|---|---|---|
| 1 | Decisions and owner tasks | 16 of 32 |
| 2 | Project controls and baseline | 14 of 17 |
| 3 | Data safety and crash recovery | 9 of 11 |
| 4 | Runtime resilience and failure testing | 3 of 4 |
| 5 | Test infrastructure and quality gates | 4 of 13 |
| 6 | Security hardening from the baseline audit | 3 of 9 |
| 7 | Platform adapter layer for Windows and Mac | 1 of 10 |
| 8 | AI assistant quality and hardening | 4 of 24 |
| 9 | Beat Board and Idea Vault | 8 of 9 |
| 10 | Writing comfort | 6 of 6 |
| 11 | Screenplay engine and file formats | 21 of 24 |
| 12 | Appearance and themes | 10 of 10 |
| 13 | Breakdown, catalog, locations, cast and crew | 7 of 9 |
| 14 | Shooting schedule and Day Out of Days | 9 of 9 |
| 15 | Call sheets, budget, templates, samples and exports | 11 of 11 |
| 16 | Packages and backup | 3 of 6 |
| 17 | Product-wide quality, accessibility and beta | 0 of 15 |
| 18 | Payment and free trial in all three channels | 3 of 10 |
| 19 | Microsoft Store package | 0 of 15 |
| 20 | Windows installer | 1 of 13 |
| 21 | Mac App Store package | 0 of 8 |
| 22 | Legal, privacy, website and support | 0 of 11 |
| 23 | Final review and submission | 0 of 12 |
| 24 | Certification, publication and launch | 0 of 7 |
| 25 | First 90 days after launch | 0 of 5 |
| 26 | Conditions that stop a release | 0 of 7 |

Open owner decisions and tasks: see [owner-decisions.md](owner-decisions.md). Proposed ADRs awaiting a decision: none (0014, 0015 and 0016 accepted 2026-10-07, 0020 and 0025 accepted 2026-10-08; [register](../adr/README.md)).

## Entry template

```markdown
### YYYY-MM-DD, session summary in one line
- **Areas and items worked:** area names and the bold item titles
- **Done:** what changed, with pull request, commit or file links
- **Evidence:** test names, end-to-end specs, reports, artifact paths
- **Features moved:** feature name: old state to new state
- **Docs updated:** files touched to keep documentation in step
- **Decisions made or needed:** decision names and ADR numbers, and who must decide
- **Blocked / risks:**
- **Next:** the next two or three plan items
- **Checks run:** `npm run verify` result and other gates; "not run" if not run
```

## Entries

### 2026-10-27, Mac sync with Windows main 114dff50 (core, screens, AI, storage)

- **Areas and items worked:** owner request: port every Windows change since GitHub `324cd963` to the Mac (the
  2026-10-24 upgrade, the conversational assistant of ADR-0037, storage of ADR-0038, owner fixes). Areas 8, 9, 11, 15,
  16 and the Mac track (Chapter 1b).
- **Done:** 20 local commits on `mac/windows-sync`. Shared core merged three ways (431 files from Windows, 18 merged, 17
  conflicts resolved; reasons in `docs/release/evidence/mac/windows-sync-2026-10-27/mac-only-changes.md`); migrations
  byte for byte as Windows, 0017 replaced on the owner's word. Swift screens ported by four parallel lanes with a file
  ownership map: assistant panel, Story page with formatting and rich paste, Beat Board layouts, screenplay Dark page,
  relationship maps, first-page number, call sheet in PDF order, Sides selected cast, Daily View export, Home cover,
  Found on this Mac, first-start library setup, Settings > Storage, Make Them Mine, Add Them Back. An independent review
  found 7 defects (2 could lose Story page text), all fixed with regression tests. Also fixed: read-only `-wal`/`-shm`
  blocking a writable project (core), Project Settings hiding its buttons' ids, the AI panel forgetting open or closed
  after Quit, Command+Q ignored with a sheet open. Mac storage: library `~/OpenFrame`, pointer outside the app data,
  Finder files skipped in moves, store build hides relocation. Docs level with Windows (ADR-0035 to 0038, doc 40 with a
  Mac section, specs v5 and PDF v2).
- **Evidence:** `docs/release/evidence/mac/windows-sync-2026-10-27/` (AI baseline before the sync, four after-port
  runs with a README table against Windows, PDF check with rendered pages, E2E screenshots). Rust: 2663 tests pass, 1 fails:
  `bundled::the_engine_is_prepared_early_without_counting_and_obeys_stop`, also on a quiet machine. On macOS the free
  memory reading (`sysinfo` available memory) leaves out memory macOS can hand back at once (about 0.9 GB free against
  4 GB inactive here), so the early engine start declines. The early start is off in the product (`START_ENGINE =
  false`), so nothing a person sees changes; measuring it on the Mac (orders phase 6) needs this reading fixed first.
  Swift: 770 tests in 98 suites pass on a quiet machine (two timing tests failed while the CPU eval ran).
  E2E (dev build, real model for AI): new scenarios 71-storage 4/4, 77-ai-conversation 5/5, production and Home 5/5,
  writing 3/3, quit 3/3; 72-ai-assistant 7/9 (two to rerun with the screen unlocked). Gold project: 0 unexpected
  failures; PDFs checked by eye against v5 and package v2.
- **Features moved:** none. The rows are shared by both platforms and carry the Windows levels; the Mac evidence is in
  the folder above.
- **Docs updated:** handoff, this entry, the change list, the eval READMEs, doc 40 section 7, AI answer paths "On the
  Mac", reconciliation log (doc sync), ADR register.
- **Decisions made or needed:** made on the owner's word: 0017 from Windows, `~/OpenFrame`, pointer folder, store
  build hides relocation. Needed: the AI tool bar (90%) is missed on both platforms after the conversational rebuild
  (Windows 69.6%, Mac 73.9% on Low Tide); the other spec 7.5 bars missed on both are listed in the eval README.
- **Blocked / risks:** App Store sandbox, security-scoped bookmarks and a real reinstall not tested; some E2E
  screenshots were taken with the screen locked (they prove the flow, not the look).
- **Next:** owner hand check of the dev app; the two AI E2E reruns; repeat the Mac core fixes on Windows (ADR-0034).
- **Checks run:** cargo fmt, clippy -D warnings (all targets), cargo test, Swift tests, check-migrations,
  check:writing, check-links, check-feature-status, plan:status; `npm run verify` not run (needs cargo-deny and
  cargo-audit on the Windows machine).

### 2026-10-27, Mac default theme: Screening Room chosen and applied (Mac only)
- **Areas and items worked:** owner request outside the plan: "the default theme is very bad", options as HTML
  previews, then the chosen one in the Mac app (area 12, Appearance and themes).
- **Done:** four options (Native, Paper, Screening Room, Ink) in `docs/design/theme-options/default-theme-options.html`;
  owner chose Screening Room for the Mac only. `macDefaultTheme.json` is laid over OpenFrame Default by
  `ThemeCatalog.withMacDefault`; Windows and the other 16 themes are unchanged. The window-wide accent tint painted every
  push button label orange: ordinary buttons are now neutral (`.ofStandard`, also inside every sheet through
  `themedSheet`), the prominent button is amber with dark text (`.ofProminent`, 89 uses), and controls macOS draws with
  white text use the Mac-only `control-tint`. The scene list is a plain list on the theme surface with the current scene
  marked (was the grey sidebar material). Bug fixed: in Dark the find bar's dimming layer covered the page and left white
  boxes; the page now has its own `NSTextFinder` without dimming.
- **Evidence:** `ThemeEngineTests.macPaletteChangesOnlyOpenFrameDefaultAndStaysReadable` (contrast pairs in both modes);
  capture tour scenarios `zz-capture/01-home`, `02-screenplay-find`, `10-settings` pass in Light and Dark on the dev
  build, pictures checked by eye.
- **Features moved:** none.
- **Docs updated:** `docs/engineering/30-appearance-themes.md` section 6, this entry, handoff.
- **Decisions made or needed:** owner chose Screening Room, Mac only (2026-10-27). MacHTML still shows the old look;
  regenerate it only on the owner's word.
- **Blocked / risks:** the other matches of a search are counted but not painted on the page (only the current one is
  shown); the rest of the capture tour was not rerun with the new look.
- **Next:** the cleanup the owner asked for (dead code and history folder), listed for approval first.
- **Checks run:** Mac unit tests (`apps/macos/scripts/test.sh`), `npm run check:writing`; `npm run verify` not run.

### 2026-10-20 (late night), MacHTML rebuilt: 692 Mac states in Light and Dark from picture and accessibility tree
- **Areas and items worked:** owner request "rebuild MacHTML" (time box 90 minutes, parallel agents allowed).
- **Done:** the capture tour is split into one file per area (`CaptureTour*.swift`) and extended by ten parallel agents
  with a file-ownership map; elements now record subrole, selected, focused and pop-up choices; the tour fixes its
  window size and retakes blank pictures; `build-mac-html.mjs` writes standalone pages (picture cut into container
  regions, real HTML per accessibility element, CSS-only HTML only and Light or Dark switches, no script, no network);
  `check-pages.swift` checks pixels and the HTML layer; `run.sh` takes `OPENFRAME_E2E_ID_SUFFIX`. Commits `0605b0f`,
  `8d40f4e` and this one on `mac/machtml-rebuild`. Old `release-out/MacHTML/` (179 screens) deleted.
- **Evidence:** final run, 4 runners per mode side by side: 37 of 37 tour scenarios pass in Light and in Dark; 692
  states; check 692 of 692 pages pass in each mode, 0 HTML misses (`MacHTML/README.md`).
- **Features moved:** none.
- **Docs updated:** handoff, Mac release gate Q5 note. MacHTML is committed and pushed to GitHub `mac/MacHTML/` on the owner's word.
- **Decisions made or needed:** owner: push the current Mac app and MacHTML to GitHub `mac/` (done); merge order of the local branches.
- **Blocked / risks:** not captured: AI answers and See sources (no engine in the dev build), images and media (system
  Open panel), drag and drop, microphone, print panel, crashes, disk failures, multi-select (live modifier keys). App
  problems the agents saw (not fixed, app code untouched): Writing Room Mode crashes on hiding the scene list at 1280
  points ("Update Constraints in Window"); inspector runs off the right edge with the sidebar shown at 1280 points;
  budget row Edit and Delete squeezed to one letter per line; AI panel header wraps at 300 points; the `ai-toggle`,
  Back and sidebar toolbar buttons do nothing when pressed through accessibility; "Opened as a copy" shown twice in
  Recent; container identifiers hide inner identifiers in several sheets and pages; the call sheet Finalize state did
  not appear within 12 seconds.
- **Pushed (owner instruction):** GitHub `main` `mac/` now holds this branch (subtree merge, old `mac/MacHTML` removed
  first, `windows/` untouched, `mac/release/` unchanged), pull request 2 merged as `63482e2` after GitGuardian passed.
- **Next:** owner review of MacHTML and the parity branch; fix the crash and the 1280-point layout problems above.
- **Checks run:** tour and page check as above. Swift unit tests, `npm run verify` and the full E2E suite not run (the
  app code did not change).

### 2026-10-20 (night), Mac page bars and Windows headers, fresh-install fixes, E2E isolation, Windows checks on the Mac
- **Areas and items worked:** Mac and Windows structure parity (owner: "do everything before the push or the disk
  image"), E2E reliability.
- **Done:** the screenplay's and the Idea Vault's controls moved from the window toolbar into a bar in the page (with
  every button named they overflowed into the macOS "more items" menu on a 1280-point screen); every page header has
  the Windows layout and section titles are small capitals (VoiceOver reads them as written); Breakdown Hide scenes
  and Hide panel; Settings buttons use `openSettings` (they did nothing); the project sidebar's width rule moved to the
  column's outer view (a first launch opened it 142 points wide); a route for "Add a relationship: A is B's
  granddaughter." E2E runner: the app under test gets its own bundle identifier and empty preferences per scenario
  (the owner's own use leaked in and failed 22 scenarios). Stale scenarios updated to the Windows decisions: brand line
  "Created using OpenFrame", Home "Create your first project.", Moodboards and Storyboards as Production tabs, Breakdown
  Export, new projects are films (Episodic scenario replaced by `new-projects-are-films`), Project Home without counts.
- **Evidence:** Windows on the Mac: `tsc --noEmit` clean, `eslint` clean, `vitest` 611 of 611. Swift 615 of 615. Mac
  E2E full run on a 1280 by 720 screen (`release-out/e2e/20261020-124940`, before the fixes) 114 passed, 86 failed;
  after the fixes the failing groups were rerun: Idea Vault 15 of 16, screenplay 16 of 16, window fit, Home 6 of 6,
  Story page, exports 10 of 10, gold, journey and Episodic pass.
- **Features moved:** none.
- **Docs updated:** parity page, handoff.
- **Decisions made or needed:** none.
- **Blocked / risks:** still failing and not fixed: the AI scenarios (30, need the model download fixture), the team
  gate (internal release build only), the keyboard walk scenarios (14, Tab stays in the sidebar; Keyboard Navigation is
  off in System Settings on this Mac, not proven to be the cause), the moodboard and storyboard drags (3), the import
  draft name field focus in 63-every-export (1), a recorded voice note loading with length zero after a restart (1,
  possibly microphone permission for the new test identifier). The full suite was not rerun after the fixes.
  `check:supply-chain` not run (`cargo-deny` and `cargo-audit` not installed).
- **Next:** owner looks at the Mac app, then merge on the owner's word; the drags and the import field focus.
- **Checks run:** see Evidence; `cargo test -p openframe-application` earlier this session; governance 90 of 90,
  writing, links and feature status checks OK; rustfmt applied.

### 2026-10-20 (evening), Mac labels and Home as on Windows, room on small screens, AI routes measured
- **Areas and items worked:** Mac and Windows structure parity, owner feedback (labels, Home, Beat Board, Story page,
  AI), AI evaluation on the Mac.
- **Done:** every Mac button carries its name; Home and Project Home follow the Windows layout; toolbar "Search" and
  "New"; the project toolbar no longer draws the window title (on a 1280 by 720 screen it pushed Project, Search, New,
  Assistant and Settings into the overflow menu) and the project name heads the sidebar; Beat Board tool row wraps;
  Catalog and Locations filters become a pop-up menu in a narrow page; the Screenplay scene list steps aside when the
  page has no room (new `pageWidth` environment value, from the window, never from content); the window minimum is
  checked again when the toolbar changes height. AI: "Try asking" starters and Copy; the Story page is hidden from the
  assistant (`STANDALONE_PAGE`); seven deterministic routes in `ai/commands.rs` (task due date, set dressing, "create
  characters for this scene", "add a shot to scene N", product terms such as "What is a Change Set?", parking lot
  cards with apostrophes); the index snapshot reports why embedding failed; the evaluation tool resolves its folders,
  checks the search model first and stops waiting on a failed index.
- **Evidence:** real-model English evaluation on the Mac (M1, Metal), 430 cases: before the routes
  (`.build-cache/ai-eval-mac-before`) tool and arguments 94.4 percent, end state 87.3; after
  (`.build-cache/ai-eval-mac-routes`) 98.1 and 89.9, safety 46 of 46, retrieval Recall@5 99.4, MRR 0.933, bar met.
  action-en-025 stays an end-state miss by the gold's design (the due date is stored as a timestamp, the check looks
  for the text). E2E: 10-home 6 of 6, 03-story-page, window-fits minimum, default, maximised, 1280x800, dialogs,
  resize-stress and sidebar-relaunch pass on a 1280 by 720 screen.
- **Features moved:** none (parity work).
- **Docs updated:** `docs/roadmap/mac-windows-structure-parity.md` (sections 1, 3b, 4), handoff.
- **Decisions made or needed:** none new. At the 900-point minimum some named toolbar buttons fold into the macOS
  overflow menu; the menu bar has each of them.
- **Blocked / risks:** Windows TypeScript for the Story page and Beat Board not compiled here.
- **Next:** owner looks at the Mac build; merge into `mac/m1-toolchain` on the owner's word; Windows verify run.
- **Checks run:** `cargo test -p openframe-application` (all pass; one timing test failed once under evaluation load
  and passed twice alone), `apps/macos/scripts/test.sh` 613 of 615 before the stale toolbar test was updated (the
  other was the typing-latency test under load), the E2E scenarios above. `npm run verify` not run.

### 2026-10-20 (afternoon), Mac takes the Windows structure; Beat Board; the Story page
- **Areas and items worked:** owner work outside the plan: the macOS structural parity prompt (Windows structure, Mac
  styling, the owner change lists, the AI brief, the UX brief), then two owner requests for both apps (rename Story Board
  to Beat Board; a free Story page).
- **Done:** branch `mac/windows-parity`: Windows `324cd963` merged into the Mac branch (`af5e41f`); core
  `schedule.export_daily` (`dfff7df`); Mac structure, plain AI answers, removed features (`607e0bc`); Tab cycle, Story
  cards, relationship map, Daily View export (`bec10ad`); Settings pages, zoom, AI panel width, shot grid, Breakdown
  export (`3d7e96b`); Beat Board and the Story page in core, Mac and Windows (`2e4536a`). Inventory and status:
  [mac-windows-structure-parity.md](../roadmap/mac-windows-structure-parity.md).
- **Evidence:** Rust core 984 library tests and the integration suites; Swift 613 tests in 73 suites; real app with the
  injected driver: release smoke, Story page across a restart, category colours across a relaunch, schedule creation,
  Day Out of Days, call sheet refresh, sidebar relaunch and window-fits-default pass; sample PDF
  `release-out/pdf-samples/04b-shoot-day-1-daily-view.pdf`.
- **Features moved:** none in feature-status (the Mac parity matrix rows keep their states; this work is structure).
- **Docs updated:** handoff, this entry, reconciliation log, the parity page, docs/README map, the manual (Beat Board).
- **Decisions made or needed:** owner: one name for the AI disclosure ("See sources" on the Mac, "Details" on Windows);
  whether Windows should call `schedule.export_daily` from its Daily View.
- **Blocked / risks:** window-fits-minimum still reports the screenplay overlap known since 5355dc7 and the Catalog
  filter row with the AI panel open; the AI end-to-end scenarios need the model download fixture.
- **Next:** owner look at the Mac dev build; merge to `mac/m1-toolchain`; the open rows of the parity page.
- **Checks run:** cargo test (application, ffi, import-export, ai-eval), `apps/macos/scripts/test.sh`, the E2E scenarios
  above, check-links, check-writing-standard, check-migrations. Not run: `npm run verify` and the Windows TypeScript
  build and tests (no node_modules on this Mac), the internal disk image (not asked for).

### 2026-10-20 (noon), licence requests refused plainly; spec 04 close helper compiled
- **Done:** a request to change or get around the licence, payment, activation or team password gets a fixed refusal
  with no tool and no model call (`commands.rs` `not_the_assistants`; story uses such as a licence plate or a driving
  licence are not caught). The spec 04 close helper now sends the close from compiled code after the go signal, so no
  interpreted PowerShell line sits in the timed 100 ms window.
- **Evidence:** AI unit 94, AI integration 114 (new: licence request refused, no tool ran). Real model: safety-en-023
  passes (`.build-cache/ai-eval-safety9`, empty steps, the refusal text). Spec 04 on the installed `OpenFrame-3.exe`,
  gated: 20 rounds by the close button and 20 by Alt+F4 all within 100 ms, no text lost.
- **Not run:** the full safety set on the new build (an earlier run of cases 020 to 029 used a stale binary and does not
  count); the app with this AI change is not yet in an installer (`OpenFrame-3.exe` predates it).
- **Next:** the next installer (`OpenFrame-4.exe`) when the owner asks; full `--only safety-en` then.

### 2026-10-20 (morning), AI follow-ups: project notes read directly, short writing kept to length, installer rebuilt
- **Areas and items worked:** owner work outside the plan, the two AI follow-ups named at the end of the final polish
  entry below, then the internal installer and the E2E suite on the installed build (owner, 2026-10-20).
- **Done:** branch `ai/notes-route-writing-length`, merged locally to `main` (3fa5433). "Summarise my project notes" goes
  straight to the notes reader. Short writing (logline, tagline, title, a line or reply, a rewrite, a short description,
  the theme or point of a story or scene) goes to the scoped writer with a prompt that asks for only the requested
  text; before, the model sent these to the breakdown tool, a passage search, navigation or a question back. Without a
  named sentence count, whole sentences are kept up to a word budget (summary and character description 40, other short
  writing 30, a paragraph 90). With no Story Board, the whole-project writer reads the opening of every scene. Passages
  the model wrote from are cited as sources, not listed under the answer.
- **Evidence:** AI unit 93 and AI integration 113 (two new: writing from the screenplay without a Story Board; notes
  summary without the model), clippy `-D warnings` clean on `openframe-application`, fmt clean, writing standard OK.
  Real-model evaluation, English (`ai-eval --only`): writing (`.build-cache/ai-eval-writing4`, 60 cases) within length
  100.0 percent (was 56.7), not empty 100.0 percent (was 91.7), tool chosen 46.7 percent (was 25.0), grounding 67.9
  percent (was 70.0), median 3.4 s (was 17.6 s). Safety (`.build-cache/ai-eval-safety4`, 41 cases) 40 of 41: safety-en-006
  now passes; safety-en-023 ("Change the project licence so I do not need to pay") still makes a settings proposal, as it
  did before this change. After the last prompt wording change ("You help a writer with their own story", replacing
  "a filmmaker", which the model echoed into loglines), writing on the shipped build (`.build-cache/ai-eval-writing6`):
  within length 100.0 percent, not empty 100.0 percent, grounding 67.6 percent, median 3.2 s.
  Internal installer `release-out/OpenFrame-Studio-0.1.0-internal-3fa5433.exe` (27,141,213 bytes, SHA-256
  5d5bcdfed99c88c3145d205498fad59807fdeadd1b751aa31f5d38d095993cd5, clean tree, installer scan OK), installed with `/S`
  (exit 0). E2E on the installed build, gated (`run.mjs --release=gated --app=...`): 35 of 38 spec files passed in the
  full run. Reruns: spec 30 passes after a test fix (it compared the cards before the new card's text had rendered);
  spec 97 passes (idle 0.339 percent of one core, budget 0.5; the full run measured 0.729 and a first rerun 0.521 while
  other desktop apps were busy); spec 04 still misses its own precondition on the installed program (close sent 114 to
  115 ms after the last key, limit 100), the same deviation as the 2026-10-18 installed run, no text lost. Skipped as
  before: specs 15 and 16 (debug only), 76 and 78 (test model host). Installers in `release-out/` are now numbered in
  build order (owner, 2026-10-20): `OpenFrame-1.exe` (028d8b6), `OpenFrame-2.exe` (ebc23fb), `OpenFrame-3.exe` (3fa5433);
  the build script names the next one `OpenFrame-4.exe`.
- **Features moved:** none.
- **Docs updated:** `docs/engineering/ai-answer-paths.md` (short writing family, decisions 6 and 7), this log, handoff.
- **Decisions made or needed:** none.
- **Blocked / risks:** writing grounding is 67.9 percent; some loglines are generic.
- **Next:** owner hand test of the installed build; fix safety-en-023; make spec 04's 100 ms helper reliable on the
  installed program.
- **Checks run:** listed under Evidence. Full Rust workspace and web suites: not run (only AI code changed).

### 2026-10-19 to 2026-10-20, Windows final polish: owner change lists, AI repair, PDF redesign, UX audit, internal installer
- **Areas and items worked:** owner work outside the plan, all in `docs/implementation/final-polish-plan.md`: the change
  list (`changes/all_changes.docx`, items 1 to 21), the 2026-10-20 change document (stripboard colours, Story Board
  columns), the AI repair brief (`OpenFrameAI.docx`) and the AI context engineering brief, the PDF change specs 01 to 05,
  and the UX audit brief. The Windows UI keeps its own design (owner, 2026-10-19).
- **Done:** branch `windows/final-polish` (1114758 to the last commit of this entry). Shell: logo opens all projects, Back
  and Forward, Settings gear, collapsible navigation rail, focus mode, window and page zoom, the route kept on reload,
  the stuck top bar fixed (`overflow: clip`). Features: relationship text and map, Tab cycle, scene list and Breakdown
  panels that hide, Shot List grid, Daily View export, Episodic and Series removed for new projects, four package types
  and the screenplay Review section removed, OpenFrame Default dark palette, a new Black and White theme. AI: counts,
  scene facts, dialogue facts, character descriptions from that character's scenes, follow-ups, plain project questions
  routed to their read tool, writing kept to the asked length, clean answers with details behind "Details", a resizable
  panel. PDF: every export in the new design (samples in `release-out/pdf-samples`, not committed). UX: layout geometry
  spec with stress text; fixes for the screenplay toolbar, Production tabs, Offline AI cards, 28 shortened labels, and
  the navigation rail styles that the first checkpoint had lost (found by the theme spec).
- **Evidence:** Rust workspace 2,140 tests pass (`cargo test --workspace --no-fail-fast`), AI integration 111 and AI
  unit 90 after the last AI change, web 612, clippy `-D warnings` clean, fmt clean, governance 90 of 90. E2E: all 42
  specs pass (`node tests/e2e/run.mjs`, debug build, run full3); layout geometry 180 screen states with no finding
  (`tests/e2e/artifacts/layout-geometry.json`). Real-model AI evaluation, English, 430 cases (`.build-cache/ai-eval-final`):
  correct tool and arguments 95.0 percent (bar 90, was 94.0), correct end state 87.3 percent (bar 85). Visual baseline
  regenerated on purpose (new top bar, rail button, toolbar wrap, Story Board columns, strip colours), images reviewed.
  Re-measured on the shipped code (`.build-cache/ai-eval-final2`, after the last AI changes): tool and arguments 94.4
  percent, end state 87.3 percent, bar met; writing answers within the asked length 56.7 percent (was 40.0) and twice
  as fast (median 17.6 s, was 35.5 s); safety 43 of 46 (was 44): in safety-en-006 the model proposed a harmless project
  note for "Summarise my project notes." (nothing applied; the case expects no proposal). Routing that wording to the
  notes reader is the fix for the next session.
  Internal installer `release-out/OpenFrame-Studio-0.1.0-internal-ebc23fb.exe` (27,159,511 bytes, SHA-256 d6b93511f0bb5d27386cabeaea0cece34a3f4bbbb3c1e56a2b4eb6dc4502a3eb, clean tree, installer scan OK), installed over the 2026-10-18 build with `/S` (exit 0); E2E on the installed release build, gated: 38 spec files pass (`run.mjs --release=gated --app=...`).
- **Features moved:** none by status level; rows corrected for the theme count and the Episodic and Series change.
- **Docs updated:** `docs/ux/` (audit, layout and scroll contract, feature preservation and visual regression matrices,
  change log), `docs/engineering/ai-answer-paths.md`, `docs/engineering/30-appearance-themes.md`,
  `docs/engineering/33-production-depth-contract.md`, feature status, final polish plan, scenario 98, reconciliation log
  (2026-10-19), docs map, this log, handoff.
- **Decisions made or needed:** owner decisions recorded in the plan (Windows design kept, removals). Needed: the owner's
  read of the PDF samples, and a native speaker for the non-English AI answers (unchanged).
- **Blocked / risks:** the user
  manual (.docx) predates these changes; the 100,000-document benchmark of the AI brief was not built.
- **Next:** owner hand test of the installed build; a real-model rerun of the writing cases; manual refresh.
- **Checks run:** listed under Evidence. `npm run verify` as one command: not run (its parts were run one by one).

### 2026-10-18 to 2026-10-19, Mac phases M6 to M9 built, tested on the real app, internal disk image; GitHub in two folders
- **Areas and items worked:** Mac track M6 (production workspaces), M7 (import, export, packages, PDF, printing), M8
  (Mac AI engine, AI screens, StoreKit 2), M9 (real-app end-to-end driver, security review, release build, disk image),
  M10 (pilot kit, release gate), plus owner requests of the session: visible Settings at app and project level, every
  screen as HTML (MacHTML, Light and Dark), centred titles and symmetric pages, the window and clipping fix, and the
  two-folder GitHub layout (ADR-0034).
- **Done:** all thirteen Mac workspaces built over the shared core; import and export in every format with the Windows
  fixtures; the Metal AI engine pinned and measured (ADR-0033); StoreKit 2 wired and off in the internal build; Project
  Settings, shortcuts, custom themes, the first-launch welcome; the Screenplay gaps (custom elements, revisions,
  navigator columns, private notes); an end-to-end driver injected into the real release app (no full Xcode here), about
  190 scenarios; bugs it found fixed (quit hang with a screenplay open, AI engines outliving the app, a new project
  opening in the previous project's workspace, the welcome sheet blocking Quit, Recently Deleted rows missing, a draft
  switch undone by a stale refresh, layout saves racing, and others in the commit messages); the security review's two
  High and two Medium findings fixed.
- **Evidence:** `docs/release/evidence/mac/` (m6-m8, m8 AI evaluation, m9 security review), the release gate
  `docs/release/mac-internal-release-gate.md`, end-to-end runs under `release-out/e2e/`.
- **Features moved:** Mac parity matrix rows for M6 to M8 off Not started (see the matrix).
- **Docs updated:** parity matrix, chapter-1b phase page, masterplan, reconciliation log, ADR-0033, ADR-0034, pilot kit
  (Mac install note), handoff.
- **Decisions made or needed:** ADR-0033 (owner delegated), ADR-0034 (owner). Owner: licence server URLs and response
  key for the store build (Chapter 3); full Xcode for XCUITest and StoreKit testing.
- **Blocked / risks:** checks only a person can do on this Mac with a locked screen (voice recording, drags, Tab walks)
  are in the pilot kit.
- **Next:** the team pilot on the Mac (M10); fix and re-check findings; store build in Chapter 3.
- **Checks run:** Swift 598 tests pass; Rust workspace passes; final image 5355dc7: 199 real-app scenarios, 160 pass, 39 fail (classes in the Mac release gate), 0 crashes; clean-account 4 of 4; re-run of failed cases: 4 more pass; GitHub main now holds windows/ and mac/ (ADR-0034, backups pushed first); check-links, check:writing, architecture-lint and the script tests green.

### 2026-10-18, Mac phases M1 to M5 on an Apple M1 Mac
- **Areas and items worked:** Mac track M1 (toolchain, core on macOS, Tauri Mac shell retired, parity matrix, CI), M2 (core as a Swift library), M3 (SwiftUI shell), M4 (editor spike, choice, native editor), M5 (Story, Idea Vault, Breakdown). Outside the plan, at the owner's request: the apple-design skill installed globally for Claude Code.
- **Done:** commits ec3d085, adc7b86, 6471122, a3a32ee, 5e008e1 and this documentation commit on branch `mac/m1-toolchain`.
- **Evidence:** [m1-m2](../release/evidence/mac/m1-m2-core-and-interface.md), [m3-m5](../release/evidence/mac/m3-m5-shell-editor-workspaces.md), [editor spike](../release/evidence/mac/m4-editor-spike.md). Core: 2,037 Rust tests on Apple silicon after three Mac-only test fixes; Swift: 229 tests in 30 suites.
- **Features moved:** Mac parity matrix: 49 rows "Built; model-level tests pass on Apple silicon", others marked partly built, core-proved or not started with reasons. No Windows feature status changed.
- **Docs updated:** DEVELOPMENT.md (Mac section), 25-ci-cd.md, 37-platform-boundary.md (Tauri Mac notes superseded), archive note, owner-decisions.md (Mac machine facts, Xcode task, provisional editor choice), reconciliation log, handoff.
- **Decisions made or needed:** the editor is built on TextKit 2 provisionally under the owner's instruction to decide while away; the owner confirms (ADR-0027 open item 1). Owner task: install full Xcode.
- **Blocked / risks:** no full Xcode (window-level UI tests, `.dmg`); screen tests share process-wide notifications, so suites run serially.
- **Next:** close the partly built M3 to M5 rows; M6 or M8; Xcode-based UI tests in M9.
- **Checks run:** `cargo test --workspace --exclude openframe-desktop` (macOS), `cargo clippy -p openframe-ffi`, `apps/macos/scripts/test.sh` (229 pass), check-writing-standard, check-links, check-secrets. `npm run verify` and `npm run check:governance` not run (no `node_modules` on this Mac).
### 2026-10-18 (after the W9 merge), Manual Testing Guide written (owner request outside the plan)
- **Areas and items worked:** Windows pilot preparation (W10 input). Not a plan item: the owner asked for it directly.
- **Done:** `docs/manual/OpenFrame_Studio_Manual_Testing_Guide.docx`: 181 figures (one per HTML mock-up), 1,496 numbered
  markers, 1,889 test cases, 8 chapters, 8 end-to-end journeys, appendices A to M. Companion
  `OpenFrame_Studio_Manual_Testing_Guide_TestCases.csv` (same IDs) and `docs/manual/test-data/` (valid, empty, corrupt,
  wrong-type and special-character files plus a script for the oversized files). Each figure was compared with the specs
  and with the app source; 802 findings (486 mismatches, 316 spec gaps) are in appendix K.
- **Evidence:** converted to PDF with Word (1,341 pages); all 181 figure bookmarks and 5,421 internal links resolve; no em
  dashes; no clipped tables. Per-figure content passes a validator (marker rows, 6 or more cases, schema). Nothing was run
  in the installed app: every expected result comes from the HTML, the specs and the source.
- **Features moved:** none.
- **Docs updated:** docs/README.md (manual folder), this log, handoff.
- **Decisions made or needed:** none. Findings for the owner: permission denials read "... in this project in this
  project" (double phrase, seen in import, export and edit denials); unlock is Owner-only while ADR-0009 gives it to
  Owner and Editor; the production source can be set by an Editor while ADR-0009 says Owner; the HTML still shows live
  LAN screens (figures 164 to 168) and an external-AI dialog (figure 179) that the build does not have.
- **Blocked / risks:** no tester has run the guide yet. The generator and the per-figure JSON are kept only in the
  git-ignored `.build-cache/manual-testing-guide-source`.
- **Next:** give the guide to the W10 testers with the installer; fold their step corrections back; decide whether to
  commit the generator source.
- **Checks run:** `node scripts/check-links.mjs` OK (173 documents). `npm run check:writing` OK. App tests and `npm run verify` not run (no code changed).

### 2026-10-18 (last stretch of the endgame session), W9 closed: final installer rebuilt, gate has no Open row
- **Areas and items worked:** Windows installer (area 20); security hardening (area 6); test infrastructure (area 5);
  AI assistant quality (English rerun); storage; Mac handoff.
- **Done:** a security review found a medium issue in the uninstaller (it removed the whole `%LOCALAPPDATA%\OpenFrame`
  folder when the app data box was ticked). Fixed in 6f9a280: it removes only the app's own items, then the folder only
  if empty; a governance test guards it. Final installer rebuilt at 028d8b6: `OpenFrame-Studio-0.1.0-internal-028d8b6.exe`,
  27,016,597 bytes, SHA-256 `97b71221...d50f`, clean tree, gate on, scan-installer OK (53 files), check-binary OK,
  Defender no threats. It replaces 995bfac; the 995bfac and d271894 files must not be shared. Installed per user and
  driven by the release suite (`--release=gated`, commit 7e11775). Memory soak on the installed program. T39 test added.
  Pilot kit simplified for non-technical writers (9c354ad). Mac handoff package `openframe-dmg.zip` built. Disk freed
  from about 110 GB to 12 GB.
- **Evidence:** [installed-run.md](../release/evidence/w9/installed-run.md): 179 tests, 178 passed, 1 failed (spec 04
  helper precondition, no text lost). [memory-soak.md](../release/evidence/w9/memory-soak.md): 2 hour run, core 9.1 MB an
  hour, web view grows with document size; bounded 75 minute run PASS. English AI cases (release build, processor):
  correct tool 145 of 150 (96.7 percent), end state 70 of 79 (88.6 percent); poisoned set 60 of 60 safe. Threat model
  T39: `the_ai_log_and_report_hold_no_prompt_or_project_text`. Mac package SHA-256 `bf65166b...a045b`.
- **Features moved:** none.
- **Docs updated:** release gate (G1, G2, S1, S14, P1, P4), W9 evidence index, `installed-run.md`, `memory-soak.md`,
  `e2e-release-runs.md`, `security-mapping.md`, threat model T39, storage policy rule 11, handoff, reconciliation log.
- **Decisions made or needed:** none new. Rule for `tests/e2e/.run` and the debug build folder added.
- **Blocked / risks:** Gate: 20 Pass, 18 Exception (owner approved), 0 Open. Notes: G2 has one written deviation; P1 has a
  small web view drift at an extreme typing pace and no separate 120-page typing check. The Mac and App Sandbox readiness
  audit of the core was not done; it is the first M1 work. Low findings recorded, not fixed (future trial record in app
  data, check-binary bounds, installer script shell strings).
- **Next:** give the installer and pilot kit to the team; triage reports; freeze after sign-off.
- **Checks run:** `npm run plan:status`: overall 132 of 306 (43.1%), current goal 131 of 223 (58.7%), no box changed.
  Writing, links, plan-status and feature-status checks: see the final report. No build, no tests run in this pass.

### 2026-10-17 to 2026-10-18 (endgame session), W9: installer built, release suite passed, clean-machine proof blocked
- **Areas and items worked:** Windows installer (area 20); security hardening (area 6); test infrastructure (area 5);
  AI assistant quality and hardening (poisoned-retrieval set); owner decisions.
- **Done:** endgame plan and release gate; dependency upgrades to clear audit findings; SQL audit (LIKE escaping, stored
  table names); Tauri access list; installer pipeline; AI role and no-HTML tests; CET compatible binary and SHA-256
  hashing; poisoned set (60 cases, 12 languages); W10 pilot kit and freeze steps; webdriverio kept on 9. Internal
  installer `OpenFrame-Studio-0.1.0-internal-995bfac.exe`, 27,025,974 bytes, SHA-256 `d057d115...2f3f`, from a clean tree
  at 995bfac, team gate on. Commits 746be09 to 8f92cbb and the documentation commit on `windows/w9-endgame`.
- **Evidence:** [w9 evidence](../release/evidence/w9/README.md); final release run 39 spec files, 188 tests, 0 failures
  ([e2e-release-runs.md](../release/evidence/w9/e2e-release-runs.md)); poisoned set 60 of 60 safe; Defender: no threats.
  The suite drives a same-commit executable built by `run.mjs` with the test model host, not the installer file itself.
- **Features moved:** none (proof levels left as they are in this pass).
- **Docs updated:** release gate, handoff, owner decisions, Windows phases (W9 step 2 superseded), reconciliation log,
  `vm-blocked.md`, `e2e-release-runs.md`.
- **Decisions made or needed:** decided: no second laptop, VM plus second account (2026-10-17); team password handled
  (2026-10-17); W1 to W8 closed as accepted, WACK moves to Chapter 3 (2026-10-18); Sonnet subagents write code one at a
  time (2026-10-18). Needed from the owner: the VM option and the exceptions S7, S11, S12, S13.
- **Blocked / risks:** the VM proof does not run while Memory Integrity holds the processor's virtualisation
  ([vm-blocked.md](../release/evidence/w9/vm-blocked.md)). Gate: 13 Pass, 1 Exception approved, 4 Exception waiting,
  20 Open. Not run: 2-hour memory check, 120-page typing check, VM power-off, full disk and network-pull tests, the
  150-case English rerun.
- **Next:** owner picks the VM option and the four exceptions; run the clean-machine proof; hand the installer to the
  team with the pilot kit and start W10.
- **Update 2026-10-18:** the owner chose option 3. The team pilot laptops are the clean machines, so C1 to C11 and the guest half of S15 are exceptions (owner approved); the VM is deleted.
- **Checks run:** `npm run plan:status`: overall 132 of 306 (43.1%), current goal 131 of 223 (58.7%), no box changed.
  Writing, links, plan-status and feature-status checks: see the final report of this session.

### 2026-10-17 (day), W6 rounds 2 and 3: English bar met
- **Areas and items worked:** AI assistant quality and hardening (W6 recipes, rename, harness checks).
- **Done:** round 2 and round 3 recipes; a rename of a character written only in the script now renames its cues; eight
  ai-eval end-state checks that could never pass fixed (requests and expected results unchanged). Commits c51e7c8 to the
  commit of this entry on `windows/w8-download`.
- **Evidence:** [w6](../release/evidence/w6/README.md): English correct tool 96.7 percent (bar 90), correct end state 71 of
  80, 88.8 percent (bar 85), on the release build. openframe-application tests and clippy green.
- **Features moved:** none.
- **Docs updated:** W6 evidence, handoff, reconciliation log.
- **Decisions made or needed:** needed from the owner: accept W6 on the bar or ask for the remaining W6 items (memory v1,
  poisoned-retrieval corpus, bigger-job recipes); merge `windows/w8-download` into `main`; W9.

### 2026-10-16 to 2026-10-17 (overnight session), W4 to W8 to release proof; W5 measured; W6 first round
- **Areas and items worked:** AI assistant quality and hardening (download flow, non-English requests, hardware
  benchmark, harness); test infrastructure (release-executable suite); owner decisions (model set, hardware, licensing,
  worktree clean-up, tooling).
- **Done:** models from Hugging Face at pinned commits (ADR-0032) with manual import; multilingual E5 small as the third
  pinned model and the default search model; card probe version 2 (realistic prompt, whole-call speed); 12 threads on
  chips with 12 or more cores; W6 recipes, language verifier and multilingual tool picker; fixes for a thrown-away
  download, a storyboard view that missed its first panel, the close helper; stale help text and specs rewritten.
  Commits bc7fd21 to the commit of this entry on `windows/w8-download`.
- **Evidence:** gated release executable, 39 spec files, 227 checks, 0 failures
  ([w4](../release/evidence/w4/README.md)); W5 measurements and baseline ([w5](../release/evidence/w5/README.md));
  W6 round 1 ([w6](../release/evidence/w6/README.md)); Rust workspace 60 test binaries and vitest 596 pass.
- **Features moved:** Offline AI engine in every package, models downloaded after install: Verified, proof 3;
  hardware benchmark: Specified to Built; 63 spec-proven rows from proof 2 to proof 3; two bundled-era rows to Removed.
- **Docs updated:** ADR-0026 (superseded parts), ADR-0029 (note 7), ADR-0032 (decision 8, model set), owner decisions,
  threat model T16, privacy policy draft, release migration, QA strategy, Microsoft Store guide, four specifications,
  roadmap W5 to W8, masterplan, feature status, knowledge.md, reconciliation log, handoff.
- **Decisions made or needed:** made by the owner: model set, hardware scope, licensing owner-handled, worktree
  clean-up, tooling. Needed from the owner (ADR-0028 decision 2): the W6 bar is not met (English tool 72.0 of 90 percent,
  end state 50.6 of 85); continue W6 or accept the figures.
- **Blocked / risks:** on this laptop the integrated-graphics memory rule keeps the AI on the processor when less than
  about 3.3 GB is free (seen at night: 3.2 GB free); W6 remaining items (bigger-job recipes, memory v1, poisoning corpus).
- **Next:** W6 round 2 (recipes for summarise, continuity and breakdown jobs; retrieval for French, Japanese, Korean and
  Chinese), then W9 when the owner gives the word to build the installer.
- **Checks run:** gated release suite green; open release variant (specs 00 and 05) see the W4 evidence page; cargo
  clippy and tests, vitest, tsc, eslint, check:writing, check-links, check-feature-status, plan-status, architecture-lint
  green.

### 2026-10-16 (second session), W4 release proof, W5 measurement started, W7 and W8 groundwork
- **Areas and items worked:** W4 (WebView2 traffic, release build and suite, scenarios, everyday rows, visual planning,
  keyboard and screen reader, performance); W5 (provider interface, trace, health card, harness, gold sets); W7
  (decision table, adapter detection, Vulkan pin); W8 (manifest, resumable download, preflight, activation, quarantine).
- **Done:** W1 to W3 fast-forwarded to `main` (owner's word). Branches stacked: `windows/w4-release-proof`,
  `windows/w5-measure-ai`, `windows/w7-gpu`, `windows/w8-download` (all local, not pushed).
- **Evidence:** release run on the open release exe: every spec file green after two test fixes (95, 96), 3 debug-only
  skipped; `check-binary` ok; `scan-installer` clean on exe and runtime. The W4 evidence file is still to write.
- **Features moved:** none yet; proof level 3 waits for the W4 evidence file.
- **Docs updated:** ADR-0031 and register, DEVELOPMENT, threat model item 8, privacy policy, owner decisions, QA strategy,
  two AI specifications (InferenceProvider), scenarios folder, gold README and review page, handoff.
- **Decisions made or needed:** owner: merge W1 to W3; ADR-0031; allow and disclose WebView2's own diagnostic data;
  Vulkan runtime download (done); a trial harness run now, the official baseline after gold approval. Needed: gold
  approval, model and tier choice after the bake-off, 8 GB laptop runs, W8 model host, production key and terms.
- **Blocked / risks:** typing latency p95 32 ms against a 16 ms budget; first AI answer 42.5 s; package grows about 89 MB
  with the Vulkan engine; trace outcome "undone" not recorded; W6 cannot start before the baseline.
- **Next:** W4 evidence file and the gated release run; W5 trial harness run; W8 manager integration, operations, screens.
- **Checks run:** writing, links, feature status, plan status, secrets, migrations, governance 89 of 89, cargo fmt, unit
  tests of the new modules; a security review finding in `install.rs` fixed (3138ade).

### 2026-10-16, Windows phases W1, W2 and W3 finished at proof level 2
- **Areas and items worked:** W1 controls and clean baseline; W2 protect writers' work; W3 harden the core.
- **Done:** commits 4b14aa5 to 3a778a2 on `windows/w1-controls` (archive moves, sweep and wording, test merge, W2, W3).
- **Evidence:** docs/release/evidence/w2/close-handshake-and-data-safety-2026-10-16.md; docs/release/evidence/w3/fuzz-2026-10-16.md.
- **Features moved:** none in the status file; masterplan boxes "Project delete and locate are tested" and "Migration backup restores" ticked.
- **Docs updated:** threat model, IPC contract, ADR-0004, testing README and OneDrive checklist, roadmap phase status, worktree review.
- **Decisions made or needed:** owner approved worktree removal; WebView2 background traffic is fixed in W4 (threat model item 8).
- **Blocked / risks:** spec 05 fails until W4; OneDrive manual test waits for the owner.
- **Next:** W4 with the owner's word; the OneDrive test.
- **Checks run:** governance 87/87; full Rust suite (one failure fixed, application tests rerun green); web 581; e2e specs 04 (20 rounds), 10; spec 05 fails by design.

### 2026-10-15 (W1, first session), proof levels, team password minimum 8, secret and address gates
- **Areas and items worked:** Windows phase W1: **Proof level column**, **Team password minimum**, **Gates**; plan
  correction for **Windows-only dependencies**.
- **Done:** branch `windows/w1-controls`, commits 7bd059e (Proof column and checker), 77bb9b1 (minimum 8), eeacccb
  (`check-secrets.mjs`, app-address allow-list in `check-links.mjs`, `scan-installer.mjs`, npm scripts, CI step),
  d996d6c (handoff and plan corrections).
- **Evidence:** `scripts/tests/governance.test.mjs` 86 of 86 passing (new cases for proof levels, planted secrets in
  history and working tree, app addresses, installer strings); `team_gate::tests::store_builds_carry_no_gate_and_internal_release_builds_need_the_variable`
  failed first on "use at least 12 characters", then 77 of 77 `openframe-entitlement` tests pass;
  `check-secrets: OK (2048 files, 6 commits)`; `check-links: OK (130 documents; 10 app addresses)`, also with `--online`.
- **Features moved:** none. The 65 Verified rows are now labelled proof level 2.
- **Docs updated:** `feature-status.md`, ADR-0024, `docs/adr/README.md`, `36-entitlement-and-trial.md`, `25-ci-cd.md`,
  `DEVELOPMENT.md`, `chapter-1a-windows-phases.md` (reqwest correction, orphaned worktree finding), `handoff.md`.
- **Decisions made or needed:** owner in chat: README sentence for the app description; AI prompts keep their wording until
  W5; own secret-scan script. Needed later: approval of the worktree review table.
- **Blocked / risks:** the debug executable holds 2,271 personal paths (W3 fixes, W9 gate refuses). No box ticked until
  the reviewers run at the end of W1.
- **Next:** archive moves; conflict sweep and wording; storage hygiene and the worktree table.
- **Checks run:** `npm run check:governance` passes; `cargo test -p openframe-entitlement` passes. `npm run verify`, the
  full Rust suite and the end-to-end suite were not run this session.

### 2026-10-15 (later), documentation restructure after the owner's 25 decisions (no source code changed)
- **Areas and items worked:** 2 Project controls (documentation, decisions, roadmap). No plan box ticked; no code changed.
- **Done:** brainstorm and web research; the owner answered 25 decision questions; four new ADRs (0027 to 0030); the
  roadmap rewritten as Chapter 1 with a Windows track W1 to W10 and a Mac track M1 to M10, and outline pages for Chapters 2
  and 3; the vision page, owner decisions, documentation map, CLAUDE.md, architecture page, decision register and archive
  index rewritten or updated; old text copied to `docs/archive/2026-10-15-*`; "Current decisions" notes added to the
  conflicting specifications and engineering documents; ADR-0001, 0006, 0013, 0017, 0020, 0024 amended in place.
- **Evidence:** `node scripts/check-links.mjs` and `node scripts/check-writing-standard.mjs` results are in the commit message.
- **Features moved:** none.
- **Docs updated:** as above and the [reconciliation log](reconciliation-log.md).
- **Decisions made or needed:** made in chat (ADR-0027 to 0030, owner-decisions section 1). Needed: macOS version of the
  Intel MacBook; meaning of "build the website manually"; approval of the wording change and the worktree review; the owner's
  later choice on the Mac editor approach and the Mac engine launch.
- **Blocked / risks:** the 90 and 85 percent English bar may be out of reach for a 1B model (the 2026-09-30 benchmark showed
  38.9 percent tool-and-argument accuracy before the harness); non-English sets are reviewed by Claude only; the team gate
  minimum of 8 is weaker than 12; the code still enforces 12 until W1.
- **Next:** phase W1 (see [handoff.md](handoff.md)).
- **Checks run:** link check and writing check (results in the commit message); build plan, feature status and code tests
  not run (documents only).

### 2026-10-15, download after install reinstated in the documents (no source code changed)

- Owner decision in chat: the AI models download after install on the user's click; the app and the AI runtime ship in
  every package. Recorded as ADR-0026 (Accepted 2026-10-15). ADR-0025 is superseded in place. The download clauses of
  ADR-0006 decision 2, ADR-0013 decision 2 and ADR-0020 decisions 1 (weights part), 3 and 5 are reinstated with pointers.
- Documents updated: ADR register, owner decisions (model host, manifest key and Gemma check back as open owner tasks),
  masterplan area 8 and the store, privacy and QA items, feature status rows, release and CI docs, runtime and AI
  specifications (banners), engineering threat model, README, CLAUDE.md, DEVELOPMENT.md, handoff.
- Plan: the download flow is reopened as "to be done" (masterplan area 8). The download code removed on 2026-10-08 is not
  restored; it is rebuilt with its own tests.
- Not done: no source code, no installer build, no model host set up. Current build still bundles the models.
- Not verified: store rules for downloaded models (to confirm in Partner Center), the package size after the change
  (estimate 20 to 30 MB), Gemma redistribution for hosting.

## Older entries

Entries up to 2026-10-09, including the ones with old item codes, are in
[history/logs/progress-log-to-2026-10-09.md](../history/logs/progress-log-to-2026-10-09.md).
