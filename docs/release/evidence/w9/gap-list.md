# Windows endgame gap list (2026-10-17)

Branch `windows/w9-endgame`. This list says, for every open item that blocks the Windows track from its final release
run, what proves it today and what is left.

**Method.** Input A is every "Must ship" row in `docs/implementation/feature-status.md` whose Proof is below 3 or whose
Status is below Verified (84 of 119 Must ship rows; the other 35 already stand at proof 3 with
`docs/release/evidence/w4/README.md`). Input B is every unticked box in areas 2, 3, 4, 5, 6, 7, 8, 16 and 20 of
`docs/implementation/masterplan.md` (65 boxes). Each item was matched against the release-build run of 2026-10-17
(`docs/release/evidence/w4/release-run-2026-10-17-gated.txt`, 39 spec files, 227 checks, 0 failures), the spec file
itself in `tests/e2e/specs/`, the W2, W3, W5, W6, W7 and W9 evidence pages, and the source. I only wrote
`covered-L3` after opening the spec and seeing it do the thing on the release build. Rows backed only by Rust tests
(no spec touches them) are written `needs-test`: the smallest spec step is named, and the owner can decide that a final
release-mode `cargo test --release` run counts as level 3 for backend-only rows (see the last section). Nothing was
built or run for this page except `node scripts/check-writing-standard.mjs`.

Decisions: `covered-L3` (a release spec exercises it), `needs-test` (built, no release spec), `needs-work` (something
real is not built), `clean-machine` (provable only on the clean VM or by the team or owner), `later` (Chapter 2, Chapter
3 or the Mac track), `stale` (done in code, only the document lags).

## Counts

| Decision | A: feature rows | B: masterplan boxes |
|---|---|---|
| covered-L3 | 21 | 2 |
| needs-test | 36 | 10 |
| needs-work | 4 | 9 |
| clean-machine | 6 | 15 |
| later | 16 | 20 |
| stale | 1 | 9 |
| **Total** | **84** | **65** |

Specs run through the real operations the dialogs call when the native Save or Open dialog cannot be driven (63, 41,
62); a `covered-L3` that rests on such a call says so.

## A. Feature rows

| Feature (short) | Line | Proof now | Decision | Evidence or next step |
|---|---|---|---|---|
| Project create, open, close, duplicate, archive, delete | 56 | 1 | covered-L3 | `10-home`: rename, duplicate, archive and restore, Delete Project with the typed name, relaunch; `00` and `01` create, close, reopen. The "no test for delete and locate" text is stale (`project_lifecycle.rs`, 4 tests). Locate has no spec |
| Single-writer lock; network paths refused | 57 | 1 | needs-test | No spec. Add to `10-home`: open a `\\localhost\share` path and a second open of a held project, expect the plain refusal. Mapped-drive text is stale (`openframe-platform/src/drives.rs`, `DRIVE_REMOTE`) |
| Autosave, Save checkpoint, save indicator | 58 | 1 | needs-test | Autosave and "Saved" are in `01`, `40`, `26`. Ctrl+S checkpoint and the save-error state have no spec. Add both to `40` |
| Crash detection and recovery offer | 59 | 1 | covered-L3 | `26-forced-kill-keeps-work`: kill, relaunch, click "Open Recovery", every piece of work is there, new work saves. Wording-defect text is stale (fixed in `RecoveryDialogs.tsx`) |
| Migrations: backup first, refuse newer | 60 | 1 | needs-test | Rust only (`migration_fixtures.rs`, `migration_failures.rs`). Add to `02-restart`: open a project folder made at an older schema (fixture) on the release build and see it upgrade with its backup |
| Undo and redo, refuses to overwrite later work | 61 | 2 | covered-L3 | Undo and redo in `20`, `30`, `31`, `40`, `50`, `61`, `80`, `85`. Refusal case is Rust only (`undo_never_overwrites_later_work_by_someone_else`). Property-test text is stale (`property_invariants.rs`) |
| Project Files cabinet | 63 | 1 | needs-test | `95` and `96` only open the page and walk Tab stops. Add a Files spec: add a file, search it, relink after moving it (same box as masterplan line 240) |
| Global search; private notes owner-only | 64 | 1 | covered-L3 | `01`: "Search (Ctrl+K) finds the scene and navigates to it". Private-note hiding is Rust only (`comments.rs`) and has no second user in the app to test |
| App shell, Home, navigation, toasts with Undo | 65 | 1 | covered-L3 | `01`, `10`, `95` (every canonical route opens), `96`; undo toast in `20` |
| Roles and capability matrix | 67 | 1 | needs-test | Rust only (`permission_matrix.rs`). Add to `70-packages`: as the reviewer, try an edit and expect the refusal |
| Offline operation | 68 | 1 | covered-L3 | `72` loopback-only sockets while AI works, `05` web view off the network (two runs), `78` restart with the host down |
| Local logs, redacted, 14 days | 69 | 1 | needs-test | Rust unit tests only. Add to the gated run: after `01` and `72`, read the app log and the engine log, assert no typed text, prompt, project path or key |
| Export diagnostics bundle | 70 | 1 | covered-L3 | `63`: the bundle is written, is a zip, holds no project text (through the operation, not the button). Manual read of the bundle stays a team task |
| High Contrast, no colour-only meaning | 71 | 1 | clean-machine | `91` proves the in-app high-contrast theme in both modes on the release build. Windows contrast themes switched on is a manual pass (W9) |
| Keyboard-only walkthrough, focus, labels | 72 | none | covered-L3 | `96-keyboard-and-screen-reader`: Home, Vault, Story, Screenplay, Breakdown, Production, Call Sheets, Files, every Tab stop named and visibly focused. Row says pending, so the row is stale. A Narrator pass stays manual |
| Idea Vault views, empty state | 80 | 1 | needs-test | No spec clicks Grid, Card, List, Folder. Add to `20` |
| Add note, image, URL, file, voice, video | 81 | 1 | needs-test | `20`, `21` cover note and voice note. Add image, URL, file, video through the operations in `21` |
| Drag-in files, partial success | 82 | 1 | needs-test | `21` states WebDriver cannot drag from Explorer. Add an operation-level add with one missing file to `21`; the real drag is a team step |
| Global Idea Vault and copy to project | 84 | 1 | covered-L3 | `21`: "Global Idea Vault: capture there, copy into the project", relaunch keeps the copies |
| Search the vault, Quick Capture | 85 | 1 | needs-test | Quick Capture is in `20`. Vault search has no spec. Add one search step to `20` |
| Send to Story Board | 86 | 1 | covered-L3 | `21`: "Send to Story Board as a Scene Card; the Vault original stays" |
| Characters, relationships, timeline, sequence hub | 99 | 1 | needs-test | `30` covers characters (create, edit, delete, restore). Relationships, Timeline and the sequence hub are only route-opened in `95`. Add them to `30` |
| Series, seasons, episodes, season board | 100 | 1 | needs-test | `31` "episodes scope the Story Board in an episodic project". Season board has no spec. Add to `31` |
| Story Board / Outline PDF export | 101 | 1 | covered-L3 | `63`: "the Story Board exports as PDF" (written, starts with `%PDF-`); Outline view in `30`. Golden is Rust |
| Structured model, stable element ids | 115 | 1 | covered-L3 | `40`, `41`, `42` write typed elements and read them back by type after relaunch (`editorElements`) |
| Element rule profiles | 116 | 1 | needs-test | Margins, spacing and mapping are Rust only. `41` shows the A4 page and the custom element's next element. Add a margin or line-count assert to `41` |
| Focus mode, Writing Room, notes, comments | 118 | 1 | needs-test | Comments covered in `44`. Focus mode and Writing Room are not opened by any spec. Add to `42` |
| Drafts, history, compare, review rounds | 119 | 1 | needs-test | `40` (second draft, rename, delete, restore), `44` (Compare). Automatic-history restore and review rounds have no spec. Add history restore to `40` |
| Production scene numbers | 121 | 1 | needs-test | `41` locks the draft and the FDX holds `Number="1"`. Add: insert a scene after the lock, expect `1A` |
| Deterministic pagination, preview, print, PDF | 122 | 1 | needs-test | `41` page view shows "Page 2" and the PDF has 2 pages. Add an assert that the print-preview operation and the PDF report the same page count |
| Keep-together rules | 123 | 1 | needs-test | Rust only. Same added step as 124 in `41` |
| Dialogue continuation (MORE, CONT'D) | 124 | 1 | needs-test | Rust only. Add a long speech to `41` and check the page view shows MORE and CONT'D |
| Print preview identical to PDF | 135 | 1 | needs-test | See 122; no spec opens Print Preview |
| Scene Hub, moving a scene in production | 136 | 1 | needs-test | `44` opens the Scene Hub from the context menu. The moving-a-scene-in-production warning has no spec. Add to `44` |
| FDX export valid, re-importable | 148 | 1 | covered-L3 | `41` and `63`: export FDX, check `TELEPHONE` and `StartsNewPage`, import it back as a draft (operation-level). Reopening in Final Draft is an owner task |
| FDX round-trip fixture suite (13 classes) | 149 | 1 | needs-test | Rust only (`fdx_suite.rs`). One class runs in `41` and `63`. Counts as level 3 only if the owner accepts the final release-mode Rust run |
| Fountain import and export | 150 | 1 | covered-L3 | `63` export and re-import (3 scenes, dialogue kept); `01` pasted Fountain |
| DOCX import and export | 151 | 1 | covered-L3 | `63` export and re-import as a draft |
| PDF import with preview and safe failure | 152 | 1 | needs-test | `63` re-imports FDX, Fountain, DOCX, TXT but not the PDF it just wrote. Add `pdf` to that loop, plus a scanned-style PDF refusal |
| TXT and pasted text import | 153 | 1 | covered-L3 | `63` TXT round trip; `01` "import pasted Fountain from the quick actions menu" |
| Import report exact; export failure leaves source | 154 | 1 | needs-test | `63` asserts scene counts. Add a damaged file to `63` and assert the project is unchanged (as `70` does for a damaged backup) |
| Screenplay outputs carry no branding | 155 | 1 | needs-test | `41` asserts the PDF has no "OpenFrame". Add the same assert for FDX, DOCX, Fountain and TXT in `63` |
| PDF visual regression | 156 | 1 | needs-test | Rust goldens (`pdf_golden.rs`, `pdf_regression.rs`). Release proof is the owner's choice (last section) |
| Script change reconciliation, Needs Review | 168 | 1 | needs-test | No spec edits the source script after a breakdown. Add to `50`: change a heading, see Needs Review, nothing deleted |
| Large-project performance | 172 | 1 | needs-test | `97` proves start and project open on the 36-scene example. Filters: run `npm run perf` once on the final commit and file the output (no new code) |
| Catalog and archive or replace | 173 | 1 | needs-test | `51` "catalog item to all its scenes". Add archive and replace-in-selected-scenes to `51` |
| Replace a rejected location | 176 | 1 | needs-test | `51` covers location fields only. Add Replace to `51` |
| Moodboard canvas and export | 184 | 1 | covered-L3 | `86` drags a note, restart keeps its place; `62` exports `moodboard.pdf` with the brand line |
| Storyboard panels, sketch tool | 185 | 1 | covered-L3 | `86`: sketch dialog draws, Undo takes a stroke back, saves a panel; two empty panels, drag reorder, restart keeps order; `62` storyboard sheet. Import panel not covered |
| Shot list and details | 186 | 1 | needs-test | `62` exports the shot list from a template. Creating, ordering and editing shots has no spec. Add to `86` |
| Call sheet branded printable PDF | 216 | 1 | covered-L3 | `62`: `callsheets.export_pdf` after finalize, brand line checked in `exportAndCheck` |
| Project notes, tasks, Activity | 219 | 1 | covered-L3 | `80` (note autosave, delete, restore, task, undo, relaunch), `85` (Activity lists and filters) |
| Retrieval gold set and metrics | 248 | none | needs-work | Small. Harness and gold sets exist (`crates/openframe-ai-eval`, `gold/retrieval.en.jsonl`; Recall@1, 5, 10, MRR and grounding in `w6/report.md`). nDCG, leakage and stale-result metrics are not in `ai-eval`. Row text "pending" is stale for the rest |
| Hardware benchmark | 249 | 1 | clean-machine | Done for the owner's laptop (`w5/README.md`, `w7`). Needs an 8 GB computer and the second laptop (W9 step 5) |
| Real-model E2E | 251 | 1 | needs-test | `72` to `75` run on the release build with the real models (ask, retrieve, propose, apply, reject, stale, restart, crash). Index rebuild is missing. Add one rebuild step to `72` |
| No-egress test | 252 | 1 | covered-L3 | `72` "no network: loopback only while AI works"; `05` covers both web view runs, so the T33 gap text is stale |
| Prompt injection corpus | 253 | 1 | needs-test | Rust tests plus a safety gold set (85 cases, 98.8 percent safety in `w6/report.md`). Add one planted-injection question to `72` on the real model |
| New operation gets metadata, tool, eval case | 254 | 1 | needs-test | Process rule enforced by `ai_tool_coverage.rs`. Final proof is the Rust run on the release commit |
| Report AI output | 255 | none | later | Needs the support address (Chapter 2 domain and mail). `ai.export_report` is a different action (Settings, AI health file) |
| Supported Windows versions enforced | 267 | none | clean-machine | Confirm in the W9 Windows 11 matrix; listing text is Chapter 3 |
| CI with desktop E2E as release gate | 268 | 1 | clean-machine | A green GitHub runner needs an owner-approved push. Local gated run is `w4/` |
| Supply chain green | 269 | 1 | stale | `w9/supply-chain.md` 2026-10-17: `cargo audit` 0 vulnerabilities, `cargo deny` ok, `npm audit` 0. Update the row. GitHub run waits for a push |
| Architecture lint | 270 | 1 | needs-test | Run `scripts/tests/governance.test.mjs` on the final commit and file the output; GitHub run optional |
| Migration fixture corpus per schema | 271 | 1 | needs-work | Small. Capture permanent binary fixtures (project and app database) from the build that goes to the team |
| WebView2 missing or broken handling | 272 | none | clean-machine | Clean VM without the runtime |
| Platform service boundary | 273 | 1 | later | Seven allow-listed files remain (`docs/engineering/37-platform-boundary.md` section 6). Windows works with the list; the moves serve the Mac track |
| macOS adapter and CI | 274 | none | later | Mac track, needs Mac hardware |
| End-user licence terms | 275 | none | later | Owner-handled licensing |
| Partner Center identity | 276 | none | later | Chapter 3 |
| Store MSIX release candidate | 277 | none | later | Chapter 3 |
| WACK on the MSIX | 278 | none | later | Chapter 3 |
| Clean-machine install, upgrade, uninstall | 279 | none | clean-machine | W9 steps 1 and 2 on the VM and a borrowed laptop |
| Privacy policy hosted on HTTPS | 280 | none | later | Chapter 2 website |
| Store listing, IARC, certification notes | 281 | none | later | Chapter 3 |
| SBOM, checksums, evidence package | 282 | none | needs-work | Small. SBOM step exists in `release.yml` only; add a local script that writes checksum, SBOM and licence inventory for the internal `.exe` |
| Release notes, known issues, rollback artifact | 283 | none | needs-work | Small. Author the team notes and the known-issues page; the previous build is the rollback file |
| Signed installer with approval updater | 284 | none | later | Signing and the updater key are Chapter 3 (`owner-decisions.md` line 145) |
| Payment and 7-day trial in both stores | 285 | 1 | later | Store channels. `17-internal-build-release` already proves the internal build never locks. `15` and `16` skip on release by design |
| Mac App Store package | 286 | none | later | Mac track and Chapter 3 |
| Windows 11 manifest and listing | 287 | none | later | Store manifest and listing, Chapter 3 |
| Shared entitlement interface and trial rules | 288 | 1 | later | Edit gating per operation matters only to store builds |
| Team password gate of internal builds | 290 | 1 | needs-test | The gated release run uses the right password only. Add a wrong-password and lockout step to the gated run (W9 step 3). Build-fails-without-variable needs the owner's word to build |
| Store entitlement adapters | 291 | 1 | later | Needs Partner Center identity and StoreKit |
| First-run safety screens after purchase | 292 | 1 | later | Purchase flow is store-only; Rust test `trial_first_run.rs` and debug spec `15` exist |

## B. Masterplan boxes

| Area and box (short) | Decision | Evidence or next step |
|---|---|---|
| 2: Align local history with GitHub (204) | stale | Local `main` exists and work branches are cut from it. GitHub alignment waits for an owner push |
| 2: Dead code and old documents audit (205) | needs-work | Small. Run the unused-code and unused-dependency tools, link check for orphan documents, record the rest |
| 2: Owner review of the documentation set (206) | clean-machine | Owner task |
| 3: Typed text survives closing the window (221) | covered-L3 | `04-close-keeps-typing`: close button and Alt+F4, 20 rounds, on the release build. Mac half is Mac track |
| 3: Cloud-synced project folders (222) | clean-machine | Detection and tests done (`w2/`). Manual OneDrive typing test (`docs/testing/onedrive-typing-test.md`) is the owner's |
| 4: Long-running soak tests (230) | needs-test | Run the ignored `soak::m1_08_soak_long` once (`OPENFRAME_SOAK_SECS=1800`) and file the result; no code |
| 5: Desktop E2E in CI as release gate (239) | clean-machine | Needs a push to see the GitHub runner |
| 5: One spec per workspace and one journey (240) | needs-test | Only a Files spec is missing (see row 63); export then re-import is now in `63` |
| 5: Hostile file collection (242) | stale | Corpus `crates/openframe-import-export/tests/fixtures/hostile/` (including `packages/`), `hostile_corpus.rs`, `packages.rs`; fuzz of 6 targets in `w3/fuzz-2026-10-16.md`. libFuzzer CI run waits for a push |
| 5: PDF image comparison tests (243) | stale | `pdf_golden.rs` (title page, continuation, revision marks, long word, A4, multilingual) and `pdf_regression.rs` (call sheet). Schedule, sides and reports have no golden |
| 5: Log redaction test (245) | needs-test | `sidecar_log_redaction.rs` and `panic_log_lines_are_redacted_and_bounded` exist. Add the release-log scan from row 69 |
| 5: Performance budgets extended (247) | needs-test | `97` covers typing, idle, memory, start, open. `perf.rs` has call sheet and PDF budgets. Run `npm run perf` on the final commit |
| 5: Writing standard hits to zero (249) | needs-work | `check:writing` now reports app 16, crates 202, scripts and CI 26, living docs 29. App 16 is small and user-visible; the rest is larger |
| 5: Dead code check in CI (250) | needs-work | Small. Add the tool as a warning job |
| 5: One test binary per crate (252) | stale | `openframe-application` is now 6 binaries (`tests/{ai,platform,production,writing}/main.rs`, `forced_kill.rs`, `perf.rs`). `openframe-import-export` (16) and `openframe-ai` (5) remain; optional |
| 6: Asset protocol scope (260) | stale | `asset_scope.rs`: `replacing_the_roots_revokes_the_previous_project_at_once` |
| 6: App database migration safety (264) | needs-work | Small. `core.rs` calls `migrate::apply` on `app.sqlite` with no integrity check and no backup first |
| 6: Release hardening flags (265) | needs-work | Small. Control-flow guard, path remap and `check-binary.mjs` exist; CET evaluation is not recorded |
| 6: Supply-chain pinning (266) | stale | Actions pinned to commits, tools pinned (`ci.yml`, `release.yml`), SBOM step at `release.yml` line 250, audits clean (`w9/supply-chain.md`) |
| 6: Tauri command permissions (267) | stale | Documented in `docs/engineering/14-ipc-contract.md` (one data command plus reviewed helpers) and fixed by lint rule C3 |
| 6: Maintained font shaping crates (269) | later | Dated exception to 2027-01-06; renew or replace in Chapter 2 |
| 7: Move every OS call behind the boundary (278) | later | Doc 37 section 6; serves the Mac track |
| 7: Windows adapter finished (279) | later | Adapter crate has tests; open items are the section 6 moves |
| 7: macOS adapter (280) | later | Mac hardware |
| 7: Mac CI (281) | later | Mac hardware |
| 7: AI engine on Mac (282) | later | Mac hardware |
| 7: Mac sandbox compatibility (283) | later | Mac hardware |
| 7: Web view behaviour on Mac (284) | stale | Marked superseded in the box (ADR-0027) |
| 7: Windows and Mac project round trip (285) | later | Mac hardware |
| 7: Full journey on Mac (286) | later | Mac hardware |
| 8: Retrieval test set (292) | needs-work | Small. Same as row 248: add nDCG, leakage, stale-result metrics |
| 8: Owner approves quality thresholds (293) | clean-machine | Owner decision. English bar met in W6 round 3 (96.7 percent tool, 88.8 percent end state) |
| 8: AI test cases for every feature (294) | needs-work | Larger. W6 lists "a case per feature" as not done; use the `ai_tool_coverage.rs` matrix to find operations with no gold case |
| 8: Product knowledge file (296) | needs-work | Small. `knowledge.md` has 17 sections; Files, Notes and Tasks, visual planning, appearance, page setup, revisions and packages are missing |
| 8: No-network test (297) | covered-L3 | `05` and `72` on the release build. Install and first launch from the installer is part of W9 |
| 8: Prompt injection and poisoned retrieval (298) | needs-work | Larger. W6 says the poisoned-retrieval corpus is not done; privilege-escalation and nested-AI tests have no named test |
| 8: AI runtime in each package, download after install (299) | clean-machine | Box text is stale (download is built, spec `78`). Needs the installer and a real Hugging Face download |
| 8: Remove download flow (301) | stale | Marked N/A in the box |
| 8: Download validation, internal `.exe` (302) | clean-machine | W9 |
| 8: Download validation, `.dmg` (303) | later | Mac hardware |
| 8: Download validation, `.msix` (304) | later | Store publishing |
| 8: Download validation, `.app` (305) | later | Store publishing, Mac hardware |
| 8: Download resilience (306) | needs-test | `78` covers resume, pause, damaged file set aside, host down. Disk-space check and retry status codes are Rust only. Add a low-disk and a 503 step to `78` using the local test host |
| 8: Model update regression suite (307) | later | Needed when a model changes after release |
| 8: AI engine lifecycle (308) | needs-test | `75` covers kill during answer and no stray engine. Hang, timeout, refusal when the job object fails and the one-process rule have no named test (`crates/openframe-ai/tests/supervisor.rs`) |
| 8: Report AI output (309) | later | Support address, Chapter 2 |
| 8: Hardware benchmarks (310) | clean-machine | Owner's laptop done; 8 GB machine and published minimum need other computers |
| 8: Real-model E2E (311) | needs-test | Only index rebuild is missing (row 251) |
| 8: Gemma redistribution check (312) | clean-machine | Owner confirmation (`owner-decisions.md` line 137) |
| 8: Visual C++ runtime app-local (314) | clean-machine | Clean Windows 11 machine without the runtime |
| 8: macOS engine pinned (316) | later | Mac hardware |
| 16: Packages carry new data (450) | needs-test | Budget and day times are documented as travelling in Backup and Full Project Packages (`coverage.rs`). No test proves it. Add budget and day times to the restore in `70` |
| 16: Packages E2E and restore drill (452) | clean-machine | `70` passes on release. Restore on a second Windows account is a team step |
| 20: Installer build (515) | clean-machine | Needs the owner's word to build; EULA page waits for the licence text |
| 20: Signing (516) | later | Chapter 3 |
| 20: Updater with approval (517) | later | Not in `tauri.conf.json`; updater key is Chapter 3 |
| 20: Uninstall keeps projects (518) | clean-machine | W9 |
| 20: Clean-machine tests (519) | clean-machine | W9 |
| 20: Team password gate (520) | needs-test | See row 290 |
| 20: Internal Mac disk image (521) | later | Mac track |
| 20: Signing of the Mac disk image (522) | later | Mac track, Chapter 3 |
| 20: Team distribution with checksum (523) | clean-machine | After the W9 build; checksum from the script in row 282 |
| 20: Binary hardening gate on this file (524) | needs-test | Run `scripts/check-binary.mjs` and `scripts/scan-installer.mjs` on the built installer; no new code |
| 20: Internal `.exe` build with gate (525) | clean-machine | W9 |
| 20: Disk image target (527) | later | Mac track |

## Smallest set of work to close the gap before the final build

Ordered by value. Items 1 to 8 are added steps in existing specs, so one re-run of the release suite covers them.

| # | Work | Rows closed | Files |
|---|---|---|---|
| 1 | Screenplay layout checks: long speech shows MORE and CONT'D, print preview and PDF report the same page count, scene `1A` after a lock, margin or line-count assert | 116, 121, 122, 123, 124, 135 | `tests/e2e/specs/41-screenplay-engine.test.mjs` |
| 2 | Import and export edges: re-import the PDF, a damaged file leaves the project unchanged, no branding in FDX, DOCX, Fountain, TXT | 152, 154, 155 | `63-every-export.test.mjs` |
| 3 | Idea Vault: four views, image, URL, file and video items, partial add, vault search | 80, 81, 82, 85 | `20-idea-vault.test.mjs`, `21-idea-vault-flow.test.mjs` |
| 4 | Story: relationships, Timeline, sequence hub, season board | 99, 100 | `30-story.test.mjs`, `31-story-board-flow.test.mjs` |
| 5 | Writing and drafts: Ctrl+S checkpoint, Focus and Writing Room, history restore, moving a scene in production, Needs Review after a script change | 58, 118, 119, 136, 168 | `40`, `42`, `44`, `50` |
| 6 | Production: catalog archive and replace, location replace, shot create and order | 173, 176, 186 | `51-breakdown-depth.test.mjs`, `86-visual-planning.test.mjs` |
| 7 | Project safety: network path and second-writer refusal, old-schema project upgrades, reviewer edit refused, backup carries budget and day times, new Files spec | 57, 60, 63, 67, 450, 240 | `10`, `02`, `70`, new `tests/e2e/specs/35-files.test.mjs` |
| 8 | AI: index rebuild step, planted injection question, low-disk and 503 download steps | 251, 311, 253, 306 | `72`, `78` |
| 9 | Gated-run checks: wrong password and lockout, log scan for typed text and paths | 290, 520, 69, 245 | `tests/e2e/run.mjs` gated variant, new log-scan step |
| 10 | Runs with no new code, filed as evidence: `npm run perf`, the 30-minute soak, the governance suite, `check-binary` and `scan-installer` on the built installer | 172, 247, 230, 270, 524 | `docs/release/evidence/w9/` |
| 11 | Small code: integrity check and backup before migrating `app.sqlite`; nDCG, leakage and stale-result metrics in `ai-eval`; missing `knowledge.md` sections; CET note; checksum and SBOM script; the 16 app writing-standard hits | 264, 265, 248, 292, 296, 282, 249 (B) | `crates/openframe-application/src/core.rs`, `crates/openframe-ai-eval/src/lib.rs`, `crates/openframe-application/src/modules/ai/knowledge.md`, `scripts/` |
| 12 | Documents: move the 21 covered rows to proof 3 after the final run; mark the 9 stale boxes and row 269 done with the evidence named above; author release notes and known issues | 283 and the stale items | `docs/implementation/feature-status.md`, `masterplan.md`, `reconciliation-log.md` |

Settled, not an owner question: the decided row "Tests from a release-ready point of view" (owner-decisions.md,
2026-10-16) counts the Rust suite run once with `--release` as release-build proof. Backend-only rows (116, 123, 149,
156, 254 and similar) reach proof level 3 through that run plus the export step on the release executable; the layout
rows in item 1 also get a spec step because a person sees them.