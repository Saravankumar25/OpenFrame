# Progress log

> The running record of work on the [build plan](masterplan.md). Every Claude Code session adds one entry
> (newest first, under "Entries"). Rules for the entry are in the repository `CLAUDE.md`. This log reports; it is not
> evidence. Evidence lives under `docs/release/evidence/` (older records under `docs/history/evidence/`) and in pull
> requests. Entries up to 2026-10-09 are in [history/logs](../history/logs/progress-log-to-2026-10-09.md).

## Dashboard (update every session from `npm run plan:status`)

**Overall: 133 of 307 boxes (43.3%).** **Current goal** (boxes without the "Later: store publishing" tag): **132 of 224 (58.9%)**. Numbers unchanged on 2026-11-05 (the rename and the overnight pass; no box ticked). Last changed 2026-10-27 after the Mac sync with Windows.

| # | Area | Done |
|---|---|---|
| 1 | Decisions and owner tasks | 16 of 32 |
| 2 | Project controls and baseline | 14 of 17 |
| 3 | Data safety and crash recovery | 9 of 11 |
| 4 | Runtime resilience and failure testing | 3 of 4 |
| 5 | Test infrastructure and quality gates | 4 of 13 |
| 6 | Security hardening from the baseline audit | 3 of 9 |
| 7 | Platform adapter layer for Windows and Mac | 1 of 10 |
| 8 | Straft AI quality and hardening | 4 of 24 |
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

### 2026-11-05, Build 259: official accounts in the Welcome window, MacHTML from the installed app, push
- **Areas and items worked:** owner's request for the eight official accounts; final steps of the overnight list.
- **Done:** `Store/SocialLinks.swift` holds the eight addresses and marks, and the Welcome window opens them. Build 259 (`Straft-12.dmg`) made and installed. Repeat full run stopped at the owner's instruction. MacHTML rebuilt from the installed app, which also corrected 19 pages per mode that showed Straft AI as missing. `release/` added with the image, its SHA-256 and SBOM.
- **Evidence:** [build 259](../release/evidence/mac/e2e-targeted-build-259-2026-11-05.md), [complete run on build 256](../release/evidence/mac/e2e-final-build-256-2026-11-05.md), `MacHTML/README.md`.
- **Features moved:** none.
- **Docs updated:** handoff, release gate, `release/README.md`, MacHTML README, third party licences, store placeholders.
- **Decisions made or needed:** the owner decides whether the accounts also go in the Help menu.
- **Blocked / risks:** three drag scenarios passed by hand only. `cargo deny` and `cargo audit` are not installed. AI evaluation not run after the name change.
- **Next:** see the handoff.
- **Checks run:** Swift 817 of 817, `straft-security`, `99-store` 7 of 7, first start 2 of 2, writing, links, governance. Rust as a whole not rerun: only one test changed since the 2982 pass.

### 2026-11-05, Overnight pass on the Mac: build, parity, fixes, one clean run, MacHTML, push
- **Areas and items worked:** the owner's overnight list, outside the plan boxes: build, old name sweep, parity with `windows/`, check every function, fixes, MacHTML, push.
- **Done, in order of change:**
  - Clean build from an empty `target/` and `.build`.
  - Old name: pictures in one owner issues document cleaned. Every other hit is a `LEGACY` line, a frozen fixture, or the one sentence in ADR-0042.
  - Migration safety from `windows/`: hard linked models, turn lock, put back on a failed path update, evidence rule for a sibling folder, damaged or newer projects keep their names, start scan time limit, notices never overwritten after a failed read, checkpoint on lock. A review added: a folder that cannot be read counts as in use, and the path update takes the database for writing from the start.
  - App bugs found by the tests: text on the soft accent at 1.3 to 1 in dark themes, muted text and borderless buttons under 4.5 to 1, sidebar section titles, Project Home two points off a small window, the Story page under the Straft AI panel at the smallest window, Close breaking in two in the panel, Remove Straft AI refused during a finishing import, the Straft AI button opening and shutting on an accessibility press, Command+J doing nothing after the panel's own Close, Home search field without a name.
  - Test measures corrected, with the reason in each commit: keyboard walks read the private SwiftUI proxy instead of what a screen reader gets; colours and pictures came from the app's own drawing, which leaves SwiftUI layers blank; assistant task requests picked the wrong section; timing in window fit, package restore and the New Project menu. New status `blocked` for a real mouse drag without the permission.
  - Dead code and two unused dependencies removed; nine finished documents moved to `docs/history`; stale statements corrected.
- **Evidence:** [final run](../release/evidence/mac/e2e-final-build-256-2026-11-05.md): 221 passed, 0 failed, 3 blocked, 9 skipped on the installed build 256; the 9 pass on the development build. Rust 2982, Swift 817 of 817, web 734 of 734, governance 90 of 90, links, writing, migrations, feature status, reachability and secrets OK. First start on a copy of real data passes and leaves the real library unchanged. [Store sandbox probe](../release/evidence/mac/store-sandbox-old-data-2026-11-04.md).
- **Features moved:** none.
- **Docs updated:** handoff, release gate, store readiness, `DEVELOPMENT.md`, MacHTML README, reconciliation log, document map.
- **Decisions made or needed:** see the handoff. Not rebuilt: a team password gate. The owner's task described one; the code and the owner's decision of 2026-10-27 have none.
- **Blocked / risks:** a first version of the old-data test reached the real library because the app database holds absolute paths: `updatedAt` in `library.json` changed and was restored from the backup; project files were compared and unchanged. The script now rewrites every stored path and compares the real library before and after. Not verified: three real mouse drags, the Dock icon and menu bar by eye (screen locked, computer use not granted), a signed store build, AI evaluation after the name change.
- **Next:** see the handoff.
- **Checks run:** see Evidence. `cargo deny` and `cargo audit` not run.

### 2026-11-04, Product renamed to Straft Studio
- **Areas and items worked:** the owner's rename task, outside the plan boxes.
- **Done:** every tracked text file and path renamed by a tested mapping. Old data carries over: app data copy, moved models, renamed library and project folders, stored paths, Mac preferences, theme id, old package formats readable, earlier migration checksums accepted. Logo: `packaging/brand/` holds the vector and a 3840 x 983 version, export logo replaced. Word manuals, design PDFs, gold outputs and hostile fixtures regenerated or repainted. MacHTML recaptured from `53d30d0b` with only the name changed. Same rename on the `windows/` tree in its own pull request.
- **Evidence:** Swift 810 of 810. Rust 2912 passed before the last folder rename change, then the 24 `rename_migration` tests after it. Desktop types and vitest 734 of 734. Headless first start on a copy of real data, including a held project and a simulated crash. MacHTML 758 of 758 in each mode against its pictures; text search and OCR of all 1516 pictures found no old name.
- **Features moved:** none.
- **Docs updated:** ADR-0042, its register row, owner decisions, reconciliation log, handoff, store placeholders.
- **Decisions made or needed:** see the handoff.
- **Blocked / risks:** nothing ran on Windows. A real first launch of the installed Mac app was not done, to leave the owner's real data untouched until the owner chooses.
- **Next:** see the handoff.
- **Checks run:** `check-links`, `check:writing`, store identity, feature status OK. Governance test 88 fails, and did before the rename (`99-store` has no spec). `npm run verify` and AI eval not run.

### 2026-11-04, Release package pushed to `mac/` on GitHub
- **Areas and items worked:** the owner's instruction to push from `Saravankumar25`.
- **Done:** `Saravankumar25` signed in by device code on the owner's approval. Branch `mac/sync-2026-11-03` (`c58d7bcd`) pushed, [pull request 6](https://github.com/Saravankumar25/StraftStudio/pull/6) opened and merged as `fb4be22e` with a merge commit. The active `gh` login was set back to `udarshmarthala`.
- **Evidence:** GitGuardian passed, the only check that ran on the pull request. `git diff` between `mac/` on main and local HEAD is empty outside `release/`; no change outside `mac/`. A scan of the pushed tree found no key files, access tokens or private keys.
- **Features moved:** none.
- **Docs updated:** handoff, this log.
- **Decisions made or needed:** owner decided the repository stays public (`owner-decisions.md`, `CLAUDE.md`). The six tracked files that hold this Mac's user name stay as they are, also the owner's decision.
- **Blocked / risks:** the macOS build workflows in `mac/.github/workflows/` do not run from that folder, so GitHub built and tested nothing; the evidence is the local runs in the entry below. The entry below is left as written: its refused push is now resolved.
- **Next:** the owner's store steps, the spot check, the Windows port.
- **Checks run:** no code changed; `npm run verify` not run.

### 2026-11-03, Release package finished on the Mac: rename, review fixes, installer, update check
- **Areas and items worked:** Mac App Store package, payment and free trial, Straft AI quality; the owner's "Straft AI" naming instruction.
- **Done:** The product is called Straft AI on every screen, message, prompt, Settings tab and in the review notes and test cases
  (`e82646bb`, `58380df7`, `4bbf7150`). Seven findings of the file reading review fixed (`bd07d1af`). Internal build 225 built and installed from `Straft-6.dmg`.
- **Evidence:** `cargo test --workspace` excluding the desktop crate 2861 passed, clippy clean. Swift 803 of 803 (two tests fixed: a bin check that raced and a day count that broke after a clock change). Section set on the final code 322 of 336, length 39 of 40 (`docs/release/evidence/ai-reharness/final-build/`). Final E2E on the dev build: 99-store 7/7,
  98-quit 4/4, 72-ai-assistant 9/9, 77-ai-conversation 6/6, 17-internal 1/1, 26-forced 3/3, 03-story-page 1/2 then the failed one 3 of 3 alone (a driver
  focus wait fixed it, `a46790ac`), 16-about 1/1, 10-home delete 1/1. Update check 4 of 4, real data app schema 9 to 11 (`docs/release/update-safety.md`). Older
  gold sets `docs/release/evidence/ai-reharness/legacy-after-tuning/`. Text of mocks 187, 188, 189, 190, 192, 200 and 201 compared with the source: every string of the
  sheets is in the code; the status bar labels behind the sheets (Trial ended, Checking purchase, Clock check) are mock chrome and not built.
- **Features moved:** none. Store rows wait for real StoreKit.
- **Docs updated:** handoff, this log, evidence README, `update-safety.md`, `mac-app-review-notes.md`, the manual test cases, `mac-parity-matrix.md`, `owner-decisions.md` (the stale licence server task).
- **Decisions made or needed:** owner: the AI tool bar (58.5% against 90%, missed before this work) and the falls on the older sets through the all-sections road.
- **Blocked / risks:** the push to `mac/` was refused (403, read access only for `udarshmarthala`), so nothing of this reached GitHub; branch `mac/sync-2026-11-03` waits locally. GitHub reports the repository as public, against `CLAUDE.md`. Real StoreKit, G4 in the sandbox, the signed package and TestFlight need Xcode and the developer account. The internal build changed bundle identifier, so preferences macOS keeps per identifier start fresh once.
- **Next:** the owner's store steps, the spot check, the Windows port.
- **Checks run:** writing, migrations, links, feature status, architecture, secrets, reachability, governance, store identity tests, typecheck: OK. `release-validate --report-only --store` fails 6 on purpose (licence agreement, migration freeze, store identity, placeholders, privacy URL, Windows engine inputs). Not run: the full regular E2E, by the owner's instruction.

### 2026-11-03, Section-scoped assistant, tuning the answers on the real model
- **Areas and items worked:** Straft AI quality and hardening (ADR-0041): the 121 failing answers of run r6, by cause.
- **Done:** A required value must be asked for, on every pick and field by field; a section's topic words never open one. A row that holds the asked words, a count and a named day's scenes and notes are demanded. Answer type votes need the words that back them, and a short "and the X" inherits. Settings reads the product guide and its promise on data leaving the Mac is demanded in its words. Small copied text files keep their words on `project_file.search_text` (project migration 0021) and no file is opened for a question. Storyboards and shots with no scene carry scene numbers and numbered panels. After three passes the closest answer is kept and code adds what it left out. Day Out of Days gives the days. A "could not find" is refused when a row holds the words (rule `held_back`). Six gold lines corrected.
- **Evidence:** `docs/release/evidence/ai-reharness/after-tuning/`, answer facts 215 of 336 to 324 of 336, length 40 of 40, pointer 59 of 59 with recall 59 of 60, false pointers 0 of 333, median 2.3 s. `cargo test -p straft-application --all-targets`.
- **Features moved:** none
- **Docs updated:** ADR-0041, `docs/engineering/ai-answer-paths.md`, `11-physical-database.md`, `23-release-migration.md`, `modules/ai/knowledge.md`, the evidence README.
- **Decisions made or needed:** the owner may want to read the 16 KiB limit and the file types for kept file words. Files added before migration 0021 are found by name only.
- **Blocked / risks:** the briefs still invent detail (`NARRATIVE_WORD_CHECKS` stays off, not measured). Writer quality on three scene briefs and on four two-part questions is the model. No Swift or E2E run: new FFI-facing field `shown` value `written_added` is in the request record only.
- **Next:** the Windows picker; a person spot-checks one case in ten of the section set (ADR-0028 decision 9).
- **Checks run:** the tests above, `-p straft-ai-eval -p straft-ai-checks -p straft-search`, clippy with warnings as errors, writing, architecture lint, migrations, links, graphify. Not run: E2E, Swift, the older gold sets on the real model, `ecc:security-review` as an agent (the file read in `files.add` was reviewed by hand and has tests for traversal by `confine`, links, size and encoding).

### 2026-11-03, Section-scoped assistant, slice R7 and the security review
- **Areas and items worked:** Straft AI quality and hardening (ADR-0041, slice R7 of the release package); security review fixes.
- **Done:** One road for a question. Deleted `AskArgs.scope`, the scope kinds, the context budget, the domain table, the fetcher table with its automatic domains, the wide passages round, the retrieval router, the project map and the scope boost in search. Migrations: project 0020, app 0011. A request with no picks under `ai_eval_all_sections` reads every section through the same path. Gold cases name sections. Swift and the web shell stop sending a scope. Review fixes: spelling note escaped, gaps of empty sections, scene reference rule, contact details in tool results, superseded call sheets, picked and off days, existence pointer for viewers, session held through a request (also in the change path), tidy Using line, search results name the picks, scenes of a chosen screenplay stay. Also: a message that says something of its own no longer confirms the last answer for memory.
- **Evidence:** `cargo test -p straft-application` (1234 lib, 209 ai, 186 platform, 251 production, 240 writing, 5 perf, 5 forced kill), `-p straft-ai-eval -p straft-ffi -p straft-search -p straft-entitlement` (180), Swift AI filter 92, web AI tests 60. Pointer bench with rules alone: precision 98.3 (59/60), recall 98.3 (59/60), false pointers 0 of 333.
- **Features moved:** none
- **Docs updated:** ADR-0041, `docs/engineering/ai-answer-paths.md`, `docs/engineering/18-retrieval-intelligence-index.md`, `docs/specs/ai-conversational-harness.md`, the AI specification, `docs/release/windows-port-2026-11-03.md`, `gold/README.md`, manual test case TC-8.174-05, reconciliation log, handoff.
- **Decisions made or needed:** defaults taken are listed in ADR-0041. A second run of the required-value rule is in the same section.
- **Blocked / risks:** no real-model run of R7: the older gold sets now read every section, so their numbers will move. `sec-neg-021` still points at the Screenplay, which does hold "a tall steel mast"; a rule for it would fit one case. The Windows panel has no picker.
- **Next:** the full evaluation with the real model; the Windows picker.
- **Checks run:** the tests above, clippy with warnings as errors, writing, architecture lint, migrations, links, web typecheck, E2E driver compiles, graphify. Not run: E2E, a real model, `security-guidance` on the new code.

### 2026-11-03, Section-scoped assistant, slices R4 to R6 and the memory gap
- **Areas and items worked:** Straft AI quality and hardening (ADR-0041).
- **Done:** Memory, focus and the map held to the picks; the out-of-section pointer (`modules/ai/pointer.rs`) with the Mac Add button; types describe, brief and story with their writer policy and briefing fetcher; the agent's tool list by section (`toolbox/homes.rs`); the m1_04 schema test waits for the background search rebuild.
- **Evidence:** `crates/straft-ai-eval/tests/pointer_gold.rs` on the 395 section cases with rules alone and no model: precision 98.3, recall 96.7, false pointers 0 of 333.
- **Features moved:** none
- **Docs updated:** ADR-0041, `docs/engineering/ai-answer-paths.md`, `docs/specs/ai-conversational-harness.md`, `modules/ai/knowledge.md`, `crates/straft-ai-eval/gold/README.md`
- **Decisions made or needed:** defaults taken are listed in ADR-0041.
- **Blocked / risks:** no real-model run of these slices; the relationship and mood checks stay off until measured; the Windows panel still has no picker.
- **Next:** the full evaluation with the real model against the saved baseline; slice R7 (delete the wide path).
- **Checks run:** `cargo test -p straft-application`, `-p straft-ai-eval -p straft-ffi`, Swift AI filter, writing, architecture lint, migrations, web typecheck.

### 2026-10-28, MacHTML recaptured from the real app, three Mac UI fixes
- **Areas and items worked:** Appearance and themes (Screening Room capture in Light and Dark); Beat Board and Idea Vault; Call sheets, budget, templates, samples and exports; Test infrastructure (capture driver)
- **Done:** Three Mac fixes from owner screenshots of 2026-10-28: budget row Edit and Delete are icon buttons with tooltips and accessibility labels (`BudgetWorkspace.swift`); the Global Idea Vault sheet from Home has a Done button and closes on Escape (`IdeaVaultWorkspace.swift`, `HomeView.swift`); the Story page placeholder disappears on the first typed character (`StoryPageTextView.swift`). MacHTML rebuilt from the development build at the full visible screen (1280 x 628 points), window frontmost and active, Screening Room in Light and Dark, 760 screens (1520 pages), every page passing the pixel and text check in both modes, including the screens added on 2026-10-27. Driver fixes: pop-up menus opened and closed with AppKit menu tracking notifications; `UI.click` posts to the event queue (fixes a vault deadlock); checked `capHideSidebar` helper; `menuButton` reuses the capture helper. `MacHTML/README.md` documents the result. The last 17 skipped tour steps were fixed (Characters map found by ids, a crew call before Finalize since the core refuses a call sheet without one, Templates reached through Project Settings, and others), so every step captures. Finished plans, a session prompt and the retired five-lane AI evidence moved to `docs/history/`. The branch went into `mac/` on GitHub `main` by pull request (ADR-0034 subtree merge), with `mac/release/` now holding `Straft-5.dmg`.
- **Evidence:** `release-out/capture/full-20261028-030147` (git-ignored) with reruns merged in; `MacHTML/README.md` verification tables; the three UI fixes confirmed in real captures.
- **Features moved:** none
- **Docs updated:** `docs/implementation/handoff.md`, this log, `MacHTML/README.md`
- **Decisions made or needed:** none new. Cleanup order (merge to `main`, delete merged branches, `HTML_Mockups/`) still waits for the owner.
- **Blocked / risks:** every tour step now captures (0 skipped in both modes). App bug found: a reports or sides snapshot can stay on its spinner right after Save Snapshot (the tour reopens reports; the sides page still shows the spinner). Not capturable in the dev build: real AI answers, Assistant memory, system Open and Save panels, drag and drop, microphone, print panel. The regular E2E suite was not rerun after the driver changes to `click`, `menuButton` and the pop-up helpers.
- **Next:** rerun the full regular E2E with one runner; fix the snapshot spinner in the app; a new disk image with the UI fixes on the owner's word.
- **Checks run:** capture tour full run in Light and Dark: 31 of 40 scenarios passed first; the 9 failures were quit hangs caused by the driver, and all 14 affected or changed scenarios then passed in both modes. MacHTML page check: see `MacHTML/README.md` verification tables. Swift unit tests 784 pass (`apps/macos/scripts/test.sh`). Regular E2E not rerun. `npm run check:writing` OK.

### 2026-10-28, Mac release round: security, store readiness, password removal, Straft-5.dmg installed

- **Areas and items worked:** owner requests: store-quality Mac app, finish the end-to-end run and fix it, remove the
  team password (owner decision 2026-10-27, ADR-0039), build `Straft-5.dmg` and install it on this Mac.
- **Done:** security review of the port and its fixes (Story page paste could read local files: fixed; macOS
  network volumes refused; Finder-file rule cannot delete user files; AI export, prompt, guard and memory hardening);
  Mac App Store readiness report with entitlements and privacy manifest fixed; Mac free-memory reading fixed and the
  early start measured; sleep stops background AI work; export over Project Settings; accessibility pass of every
  workspace; release app now generates its Swift interface from the library it links (a checksum mismatch stopped
  the first release build at start); team password and all its logic removed from both apps; architecture lint
  clean again; E2E fixes (Notes page layout loop on release, voice note player ids, drags into sheets, theme colours).
- **Evidence:** full E2E on the release app: 194 pass, 30 fail; after the fixes and reruns on builds 182 and 183, 24
  still fail and 2 are skipped by design (they need a development build): 14 keyboard scenarios need Keyboard
  navigation on (owner setting, off on this Mac), 91 sixteen themes (readability findings to settle), 86 moodboard
  drag and two panel-order scenarios, 75 one crash scenario (Cmd+J after relaunch), 78 remove, 01-journey 13 and 15,
  95 window fit at the minimum size (Screenplay with the sidebar, Search palette). Swift 784, Rust 2664, web 735
  tests pass. `Straft-5.dmg` (build 183, commit `be2431eb`) installed as `/Applications/Straft Studio.app`.
- **Features moved:** none.
- **Docs updated:** ADR-0039, owner decisions, CLAUDE.md, doc 37 and 40, store readiness report, handoff, the
  next-session prompt `docs/implementation/next-session-prompt-machtml.md`.
- **Decisions made or needed:** made: team password removed; release app built and installed; Apple model later.
  Needed: AI tool bar; library.json identity limit; store items that need Xcode and the developer account.
- **Blocked / risks:** a View menu Cmd+J command broke opening the AI panel and was reverted; the open Cmd+J item
  stays. Mac App Store build needs full Xcode (not installed; about 40 GB free disk needed).
- **Next:** MacHTML recapture from the real app (prompt above), then cleanup and the open E2E items.
- **Checks run:** cargo fmt, clippy, cargo test, Swift tests, web tsc and vitest, architecture lint, check-links,
  check:writing, check-migrations; E2E as above.

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
  after Quit, Command+Q ignored with a sheet open. Mac storage: library `~/Straft Studio`, pointer outside the app data,
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
- **Decisions made or needed:** made on the owner's word: 0017 from Windows, `~/Straft Studio`, pointer folder, store
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
  owner chose Screening Room for the Mac only. `macDefaultTheme.json` is laid over Straft Default by
  `ThemeCatalog.withMacDefault`; Windows and the other 16 themes are unchanged. The window-wide accent tint painted every
  push button label orange: ordinary buttons are now neutral (`.ofStandard`, also inside every sheet through
  `themedSheet`), the prominent button is amber with dark text (`.ofProminent`, 89 uses), and controls macOS draws with
  white text use the Mac-only `control-tint`. The scene list is a plain list on the theme surface with the current scene
  marked (was the grey sidebar material). Bug fixed: in Dark the find bar's dimming layer covered the page and left white
  boxes; the page now has its own `NSTextFinder` without dimming.
- **Evidence:** `ThemeEngineTests.macPaletteChangesOnlyStraftDefaultAndStaysReadable` (contrast pairs in both modes);
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
  `check-pages.swift` checks pixels and the HTML layer; `run.sh` takes `STRAFT_E2E_ID_SUFFIX`. Commits `0605b0f`,
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
  "Created using Straft Studio", Home "Create your first project.", Moodboards and Storyboards as Production tabs, Breakdown
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
- **Checks run:** see Evidence; `cargo test -p straft-application` earlier this session; governance 90 of 90,
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
- **Checks run:** `cargo test -p straft-application` (all pass; one timing test failed once under evaluation load
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
  passes (`.build-cache/ai-eval-safety9`, empty steps, the refusal text). Spec 04 on the installed `Straft-3.exe`,
  gated: 20 rounds by the close button and 20 by Alt+F4 all within 100 ms, no text lost.
- **Not run:** the full safety set on the new build (an earlier run of cases 020 to 029 used a stale binary and does not
  count); the app with this AI change is not yet in an installer (`Straft-3.exe` predates it).
- **Next:** the next installer (`Straft-4.exe`) when the owner asks; full `--only safety-en` then.

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
  summary without the model), clippy `-D warnings` clean on `straft-application`, fmt clean, writing standard OK.
  Real-model evaluation, English (`ai-eval --only`): writing (`.build-cache/ai-eval-writing4`, 60 cases) within length
  100.0 percent (was 56.7), not empty 100.0 percent (was 91.7), tool chosen 46.7 percent (was 25.0), grounding 67.9
  percent (was 70.0), median 3.4 s (was 17.6 s). Safety (`.build-cache/ai-eval-safety4`, 41 cases) 40 of 41: safety-en-006
  now passes; safety-en-023 ("Change the project licence so I do not need to pay") still makes a settings proposal, as it
  did before this change. After the last prompt wording change ("You help a writer with their own story", replacing
  "a filmmaker", which the model echoed into loglines), writing on the shipped build (`.build-cache/ai-eval-writing6`):
  within length 100.0 percent, not empty 100.0 percent, grounding 67.6 percent, median 3.2 s.
  Internal installer `release-out/Straft-Studio-0.1.0-internal-3fa5433.exe` (27,141,213 bytes, SHA-256
  5d5bcdfed99c88c3145d205498fad59807fdeadd1b751aa31f5d38d095993cd5, clean tree, installer scan OK), installed with `/S`
  (exit 0). E2E on the installed build, gated (`run.mjs --release=gated --app=...`): 35 of 38 spec files passed in the
  full run. Reruns: spec 30 passes after a test fix (it compared the cards before the new card's text had rendered);
  spec 97 passes (idle 0.339 percent of one core, budget 0.5; the full run measured 0.729 and a first rerun 0.521 while
  other desktop apps were busy); spec 04 still misses its own precondition on the installed program (close sent 114 to
  115 ms after the last key, limit 100), the same deviation as the 2026-10-18 installed run, no text lost. Skipped as
  before: specs 15 and 16 (debug only), 76 and 78 (test model host). Installers in `release-out/` are now numbered in
  build order (owner, 2026-10-20): `Straft-1.exe` (028d8b6), `Straft-2.exe` (ebc23fb), `Straft-3.exe` (3fa5433);
  the build script names the next one `Straft-4.exe`.
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
  columns), the AI repair brief (`StraftAI.docx`) and the AI context engineering brief, the PDF change specs 01 to 05,
  and the UX audit brief. The Windows UI keeps its own design (owner, 2026-10-19).
- **Done:** branch `windows/final-polish` (1114758 to the last commit of this entry). Shell: logo opens all projects, Back
  and Forward, Settings gear, collapsible navigation rail, focus mode, window and page zoom, the route kept on reload,
  the stuck top bar fixed (`overflow: clip`). Features: relationship text and map, Tab cycle, scene list and Breakdown
  panels that hide, Shot List grid, Daily View export, Episodic and Series removed for new projects, four package types
  and the screenplay Review section removed, Straft Default dark palette, a new Black and White theme. AI: counts,
  scene facts, dialogue facts, character descriptions from that character's scenes, follow-ups, plain project questions
  routed to their read tool, writing kept to the asked length, clean answers with details behind "Details", a resizable
  panel. PDF: every export in the new design (samples in `release-out/pdf-samples`, not committed). UX: layout geometry
  spec with stress text; fixes for the screenplay toolbar, Production tabs, Straft AI cards, 28 shortened labels, and
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
  Internal installer `release-out/Straft-Studio-0.1.0-internal-ebc23fb.exe` (27,159,511 bytes, SHA-256 d6b93511f0bb5d27386cabeaea0cece34a3f4bbbb3c1e56a2b4eb6dc4502a3eb, clean tree, installer scan OK), installed over the 2026-10-18 build with `/S` (exit 0); E2E on the installed release build, gated: 38 spec files pass (`run.mjs --release=gated --app=...`).
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
- **Checks run:** `cargo test --workspace --exclude straft-desktop` (macOS), `cargo clippy -p straft-ffi`, `apps/macos/scripts/test.sh` (229 pass), check-writing-standard, check-links, check-secrets. `npm run verify` and `npm run check:governance` not run (no `node_modules` on this Mac).
### 2026-10-18 (after the W9 merge), Manual Testing Guide written (owner request outside the plan)
- **Areas and items worked:** Windows pilot preparation (W10 input). Not a plan item: the owner asked for it directly.
- **Done:** `docs/manual/Straft_Studio_Manual_Testing_Guide.docx`: 181 figures (one per HTML mock-up), 1,496 numbered
  markers, 1,889 test cases, 8 chapters, 8 end-to-end journeys, appendices A to M. Companion
  `Straft_Studio_Manual_Testing_Guide_TestCases.csv` (same IDs) and `docs/manual/test-data/` (valid, empty, corrupt,
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
  Straft AI quality (English rerun); storage; Mac handoff.
- **Done:** a security review found a medium issue in the uninstaller (it removed the whole `%LOCALAPPDATA%\Straft`
  folder when the app data box was ticked). Fixed in 6f9a280: it removes only the app's own items, then the folder only
  if empty; a governance test guards it. Final installer rebuilt at 028d8b6: `Straft-Studio-0.1.0-internal-028d8b6.exe`,
  27,016,597 bytes, SHA-256 `97b71221...d50f`, clean tree, gate on, scan-installer OK (53 files), check-binary OK,
  Defender no threats. It replaces 995bfac; the 995bfac and d271894 files must not be shared. Installed per user and
  driven by the release suite (`--release=gated`, commit 7e11775). Memory soak on the installed program. T39 test added.
  Pilot kit simplified for non-technical writers (9c354ad). Mac handoff package `straft-dmg.zip` built. Disk freed
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
  Straft AI quality and hardening (poisoned-retrieval set); owner decisions.
- **Done:** endgame plan and release gate; dependency upgrades to clear audit findings; SQL audit (LIKE escaping, stored
  table names); Tauri access list; installer pipeline; AI role and no-HTML tests; CET compatible binary and SHA-256
  hashing; poisoned set (60 cases, 12 languages); W10 pilot kit and freeze steps; webdriverio kept on 9. Internal
  installer `Straft-Studio-0.1.0-internal-995bfac.exe`, 27,025,974 bytes, SHA-256 `d057d115...2f3f`, from a clean tree
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
- **Areas and items worked:** Straft AI quality and hardening (W6 recipes, rename, harness checks).
- **Done:** round 2 and round 3 recipes; a rename of a character written only in the script now renames its cues; eight
  ai-eval end-state checks that could never pass fixed (requests and expected results unchanged). Commits c51e7c8 to the
  commit of this entry on `windows/w8-download`.
- **Evidence:** [w6](../release/evidence/w6/README.md): English correct tool 96.7 percent (bar 90), correct end state 71 of
  80, 88.8 percent (bar 85), on the release build. straft-application tests and clippy green.
- **Features moved:** none.
- **Docs updated:** W6 evidence, handoff, reconciliation log.
- **Decisions made or needed:** needed from the owner: accept W6 on the bar or ask for the remaining W6 items (memory v1,
  poisoned-retrieval corpus, bigger-job recipes); merge `windows/w8-download` into `main`; W9.

### 2026-10-16 to 2026-10-17 (overnight session), W4 to W8 to release proof; W5 measured; W6 first round
- **Areas and items worked:** Straft AI quality and hardening (download flow, non-English requests, hardware
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
- **Features moved:** Straft AI engine in every package, models downloaded after install: Verified, proof 3;
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
  failed first on "use at least 12 characters", then 77 of 77 `straft-entitlement` tests pass;
  `check-secrets: OK (2048 files, 6 commits)`; `check-links: OK (130 documents; 10 app addresses)`, also with `--online`.
- **Features moved:** none. The 65 Verified rows are now labelled proof level 2.
- **Docs updated:** `feature-status.md`, ADR-0024, `docs/adr/README.md`, `36-entitlement-and-trial.md`, `25-ci-cd.md`,
  `DEVELOPMENT.md`, `chapter-1a-windows-phases.md` (reqwest correction, orphaned worktree finding), `handoff.md`.
- **Decisions made or needed:** owner in chat: README sentence for the app description; AI prompts keep their wording until
  W5; own secret-scan script. Needed later: approval of the worktree review table.
- **Blocked / risks:** the debug executable holds 2,271 personal paths (W3 fixes, W9 gate refuses). No box ticked until
  the reviewers run at the end of W1.
- **Next:** archive moves; conflict sweep and wording; storage hygiene and the worktree table.
- **Checks run:** `npm run check:governance` passes; `cargo test -p straft-entitlement` passes. `npm run verify`, the
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
