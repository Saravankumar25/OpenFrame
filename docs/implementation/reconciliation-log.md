# Reconciliation log

> Records (1) every conflict found between the supplied material, the existing documents and the code, with how it was
> resolved; (2) where each supplied document's content now lives, so nothing is lost; (3) stale statements that were
> corrected; (4) how the old coded names map to the descriptive names now in use.
> Renamed on 2026-10-06 from `03-reconciliation-log.md`. Extended on 2026-10-09 (chapter restructure). Checks written "(checked)" were verified in the repository on the
> date given. Checks written "(stated)" come from a supplied document and were not verified independently.

## 2026-10-27: Team password gate removed (owner decision)

The owner removed the team password of the internal builds; the code is removed in the same change. No document moved to
`docs/history/`: ADR-0024 and ADR-0030 keep decisions still in force, so both are marked superseded in part, in place.

| Stale statement | Where | Resolution |
|---|---|---|
| Internal builds ask a team password on every launch | ADR-0024 decisions 3 to 6, ADR-0030 decision 5, ADR-0031 decision 3, ADR-0023 status line | Marked superseded in place; register row and a section 2 row in `adr/README.md` |
| Password rows and the manual task to set the build variable | `owner-decisions.md` | Struck through with a pointer to the new row "Team password removed" |
| Team gate built and reachable | `feature-status.md` PLAT-016, `mac-parity-matrix.md` | Row set to Removed; the parity row edited by hand (a full `npm run mac:parity` run also pulls in unrelated drift and drops Mac evidence cells, so it was not kept) |
| Typing or setting the password | `CLAUDE.md`, `docs/README.md`, vision, chapter 1, 1a, 1b and 3 pages, internal test plan, pilot kit, release gates, store readiness, launch register, engineering 17, 20 (T37 retired), 23, 36 (section 11 marked superseded), 40, `ai-answer-paths.md`, the security and harness specifications, the manual test case list (71 rows) | Rewritten to "opens without a password" or marked superseded |
| `team_gate.*` operations listed | Generated `ai-tool-coverage.md` | Rows and counts edited to match the registry without the two operations; regenerate with the coverage test once the code change builds |
| Password steps in the Word manuals | `manual/OpenFrame_Studio_Manual_Testing_Guide.docx`, `manual/OpenFrame_Studio_User_Manual_Windows_Build.docx` | Not changed yet (binary files with figures of the password screen); still to do |
| Records of the 2026-10-17 and 2026-10-18 builds that had the gate | `release/evidence/`, progress log | Left as written: they record what happened |

## 2026-10-27: Mac documents brought level with Windows `114dff50` (core sync)

Compared: the Windows `docs/` at the last Mac sync (tree `11081707`) with Windows `114dff50`, file by file against the
Mac copy. Rule: shared facts follow Windows; Mac-only documents and Mac sections stay.

| What | How | Files |
|---|---|---|
| Unchanged on the Mac since the last sync | Windows copy taken as is | ADR-0005, 0008, 0009, 0012, 0017, 0028; engineering 11, 13, 21, 23, 32, 33, 34, 39 and `ai-answer-paths.md`; the eleven product specifications; `store/listing.md`; `testing/`; `ux/`; the manual files; `release/windows-internal-release-gate.md` (only a link to the moved endgame plan changed) |
| Changed on both | Three-way merge; Windows wins on shared facts, Mac paths and Mac rows kept | `docs/README.md`, ADR-0013, the ADR register, the architecture page, engineering 20, 27, 35, 37, feature status, the build plan, owner decisions, this log, the vision page, the launch register, the roadmap pages, the ESD, the specifications index |
| New shared documents | Copied | ADR-0035 to ADR-0038, engineering 40, `specs/ai-conversational-harness.md`, `specs/screenplay-formatting-standard-v5.md`, `specs/pdf-design-package-v2/`, `specs/owner-issues-2026-10-24/`, `roadmap/mac-port-list-2026-10-24.md`, `roadmap/mac-ai-harness-port-orders.md`, and the AI evidence of the shared core (`release/evidence/ai-harness-2026-10-24/`, `ai-conversational-*-2026-10-26/`, measured on Windows; the feature status file and the harness spec cite them) |
| Moved to `docs/history/` with the Windows "Superseded by" line | As Windows did | `implementation/endgame-plan.md`, `implementation/final-polish-plan.md`, the five-lane `engineering/ai-answer-paths.md` (now `history/2026-10-26-ai-answer-paths-five-lanes.md`); added because current documents cite them: `history/2026-10-24-ai-harness-rebuild-plan.md`, `history/specs/screenplay-formatting-standard-v4.md`. History index rows added |
| Mac sections added | Written from the code and the Mac-only change list | Engineering 40 section 7 (Mac library place, pointer folder, Finder files, store build without bookmarks); `ai-answer-paths.md` section 4 (FFI path, Metal helper process, timings to come) |

| Stale or conflicting statement | Where | Resolution |
|---|---|---|
| ADR-0035 shown as Accepted with no supersession in the register, while the ADR itself says decision 1 was superseded on 2026-10-26 | `adr/README.md` | Register row says so and a section 2 row names ADR-0037. Windows register still to do |
| The harness spec had no row in the specifications index | `specs/README.md` | Row added. Windows index still to do |
| Seven deterministic AI routes listed as current on the Mac; Story page operations named `story.page` and `story.save_page` | `roadmap/mac-windows-structure-parity.md` | Routes marked superseded (the sync took the Windows conversational path); operation names now the Windows ones |
| Links into `docs/archive/` and its Windows `docs/history/` copies (digests, agent brief, first security review, documents 28, 29, 31, the before-restructure pages) | Roadmap pages, vision page, owner decisions, launch register, architecture page, engineering 20, 27, 37, ESD, specifications index, `docs/README.md` | The Mac removed that folder on 2026-10-19, so the Mac wording stays as plain text and is not linked |
| `HTML_Mockups/` removed and `WindowsHTML/` the screen record | `docs/README.md`, UX specification header, engineering 34 | True on Windows only: the Mac tree keeps `HTML_Mockups/` as history and records its screens in `MacHTML/`; both are named |
| Feature status: the merge brought 14 new or renamed rows from Windows | `implementation/feature-status.md` | Shared descriptions taken; no existing row changed level. The new rows carry the Windows levels (the table records the Windows app; the Mac track is measured in `roadmap/mac-parity-matrix.md`) |
| Em dashes in two files touched by the sync | ADR-0009, engineering 13 | Removed; the living-docs writing count fell from 27 to 13 |

**Skipped (Windows only):** the Windows evidence screenshots in `release/evidence/M0/M0.20-visual-baseline/` (the Mac
keeps that folder in `history/evidence/`); the Windows copies of the old `docs/archive/` files; the Windows session
prompts never present in the Mac tree. Checks: `node scripts/check-links.mjs` and `npm run check:writing` pass.

## 2026-10-27: old documentation moved to docs/history (owner request "restructure old documentation into history")

| Moved | From | To | Note |
|---|---|---|---|
| Evidence of the first plan | `docs/release/evidence/M0` to `M14`, `2026-10-06-foundation`, `2026-10-08-session-2` | `docs/history/evidence/` | Current-track evidence (`w2` to `w9`, `mac`, `release`) stays: `check-feature-status.mjs` requires proof under `docs/release/evidence/` |
| Visual baseline images | `docs/release/evidence/M0/M0.20-visual-baseline/` | `tests/e2e/visual-baseline/` | A live input of `90-visual-baseline.test.mjs`, so a test fixture, not history |
| Audits | `docs/audits/` | `docs/history/audits/` | Moved, not edited |
| Report | `docs/reports/` | `docs/history/reports/` | |
| ADR-0025 | `docs/adr/` | `docs/history/adr/` | Superseded as a whole by ADR-0026; the register row stays |
| Old log sections | progress log entries and reconciliation records up to 2026-10-09 | `docs/history/logs/` | Includes the "Old code to new name" tables |

Rule change (owner, 2026-10-27, `CLAUDE.md`): a document or ADR superseded as a whole moves to `docs/history/`; a part
superseded keeps its place, marked SUPERSEDED. References in 23 files were rewritten; `check-links` and
`check-feature-status` pass.

## 2026-10-27: old report and session prompt moved to history (Windows tree)

Owner rule (chat): only the current application's documentation stays in `docs`; older material goes to `docs/history/`.

- `docs/reports/2026-10-09-project-retrospection-and-audit.md` moved to `docs/history/2026-10-09-project-retrospection-and-audit.md`
  with one added note. It is a snapshot of one day and its AI figures are stale; this replaces the 2026-10-24 decision to leave it in place. The `docs/reports/` folder is gone.
- `docs/implementation/next-session-mac-html-prompt.md` moved to `docs/history/2026-10-21-next-session-mac-html-prompt.md` with one added note: a one-off session prompt, not current documentation.
- Links changed in `docs/README.md` (map, layout block, history line), the architecture page, the vision page, the build plan and one line of the progress log. Two history index rows added.
- Considered and kept: the specifications that are only partly superseded (AI specification, agentic RAG spec, Offline Collaboration, Security, PRD, FSD, UX, Domain), engineering document 18, the UX audit pages, `docs/specs/owner-issues-2026-10-24/`, `docs/website/prompts/`, the release evidence folders.

## 2026-10-26: five-lane AI documents moved to history (Windows tree)

Owner instruction (chat): dead documentation is shifted to history as part of the S9 close-out.

- `docs/engineering/ai-answer-paths.md` (the five lanes) moved to `docs/history/2026-10-26-ai-answer-paths-five-lanes.md`,
  with one added "Superseded by" line. A new `docs/engineering/ai-answer-paths.md` describes the one conversational path
  built in slices S3 to S8.
- `docs/specs/ai-harness-rebuild.md` moved to `docs/history/2026-10-24-ai-harness-rebuild-plan.md`, with one added line.
  The 2026-10-24 owner decision to keep it in `docs/specs/` is replaced by this instruction.
- Links in ADR-0035, ADR-0037, the ADR register, owner decisions, the progress log, this log and the spec now point to the
  history copies; the specs index row is removed; the history index has two new rows. `node scripts/check-links.mjs` OK.

## 2026-10-24: one history folder, harness and layout upgrade recorded (Windows tree)

Owner instruction: "Update the documentation properly. Move older decisions, deprecated documentation, and material no longer applicable to the current application version into `docs/history/`."

**Moves (git history keeps every file; contents unchanged except a one-line "Superseded by" note at the top of the four files marked *)**

| From | To | Why |
|---|---|---|
| `docs/archive/` (all 21 entries: 18 plan and record files, `2026-09-spec-digest/`, `notes/`, README) | `docs/history/` | One history folder. The README is now the history index. `docs/history/specs/` (the v4 screenplay standard, moved earlier) was already there |
| `docs/implementation/endgame-plan.md` * | `docs/history/endgame-plan.md` | W9 closed on 2026-10-18; the release gate and progress log carry the record |
| `docs/implementation/final-polish-plan.md` * | `docs/history/final-polish-plan.md` | Final polish finished 2026-10-20; the change log carries the record |
| `docs/implementation/next-session-windows-html-prompt.md` * | `docs/history/next-session-windows-html-prompt.md` | Done 2026-10-21; the prompt was already marked done |
| `docs/implementation/worktree-review-2026-10-16.md` * | `docs/history/worktree-review-2026-10-16.md` | The owner decided and the clean-up is done; the storage policy holds the rules |

Links, rules and scripts changed together: `CLAUDE.md` (three rules), `DEVELOPMENT.md` (tree), `docs/README.md` (layout, precedence, map rows), the history index, the writing check skip list (`docs/archive/` entry removed; `docs/history/` was already skipped), the link check (`docs/history/` is a record folder), and every link to `docs/archive/` in ADR-0005, 0009, 0012, 0017, the architecture page, engineering 13, 20, 27, 35 and 37, the roadmap pages, the vision page, owner decisions, the launch register, the Windows release gate, the UX change log and the specifications README. Entries in the progress log and in this log that named `docs/archive/` or the moved plans now name the new path.

**Left where they are, on purpose:** `docs/reports/2026-10-09-project-retrospection-and-audit.md` (a dated snapshot, still the long explanation that the architecture page points to; its AI figures of 8 steps and a 7,000-character catalogue are the state of 2026-10-09), `docs/audits/` and `docs/release/evidence/` (evidence folders), `docs/specs/openframe_ai_agentic_rag_spec_prompt.md` (code and docs cite its section numbers), `docs/implementation/next-session-mac-html-prompt.md` (still the open Mac task in the handoff), `docs/website/prompts/`.

**Stale statements marked superseded in place (no living document was moved)**

| Statement | Where | Resolution |
|---|---|---|
| One agent, default 8 steps, 7,000-character catalogue | ADR-0013 decision 6; AI Specification section 8; feature status row for the assistant (its test name `default_step_limit_is_eight_and_the_hard_limit_twelve` is now `default_step_limit_is_four_and_the_hard_limit_twelve`) | Marked superseded in part with a pointer to [ADR-0035](../adr/ADR-0035-ai-lanes-briefings-and-code-verifiers.md). The 7,000 characters remain the tool selection budget, counted on the full schema; only the prompt rendering is compact |
| No summary cache; plain flat text; short by default | `engineering/ai-answer-paths.md` decisions 4 to 6 | Already marked superseded in part on 2026-10-24 by the lane work; its header and its "not done" list now point to ADR-0035 |
| "Nothing in this document is built" | `specs/ai-harness-rebuild.md` status line | Rewritten: built in part, decisions in ADR-0035, what was not built |
| Numbers never include text | ADR-0028 decision 8 | Amendment of 2026-10-24 (Save conversation for review) confirmed present; register row added |
| Screenplay standard v4; 80 pt footer clearance, hairline rule and vector lockup in production PDFs | v4 file (already in history); the report renderer | v5 and ADR-0036 recorded in the register, section 2 |
| `(MORE)` and `CUE (CONT'D)` only for one speech split by a page boundary | `engineering/32-screenplay-engine-contract.md` | Now every page split of one speech (v5) |

**Not found:** no live document states "plain text only" for AI answers except the one marked above, and no live document describes landscape Storyboard, Shot List, Schedule or Card Board outside `specs/pdf-design-package-v2/` (where v1 orientation appears only as "changes from v1").

**Noted, not changed (outside documentation):** two Rust test comments still name `docs/archive/2026-09-30-security-review.md` (`crates/openframe-application/tests/platform/security_review.rs`, `crates/openframe-security/tests/security_paths.rs`). The file is now `docs/history/2026-09-30-security-review.md`. They are comments only; fix them in the next code change. A copy of the v5 screenplay standard sits untracked at the repository root (`OpenFrame_Screenplay_Formatting_Standard_v5_Combined.md`, byte-identical to `docs/specs/screenplay-formatting-standard-v5.md`); the layout rule keeps the root clear, so it can be deleted by whoever owns it.

**Decisions recorded:** owner-decisions rows dated 2026-10-24 (answer shape, progress and Cancel, Save conversation, harness plan location, screenplay v5, PDF design v2, history folder, Mac port list, parallel tracks, end-to-end once). New: [ADR-0035](../adr/ADR-0035-ai-lanes-briefings-and-code-verifiers.md), [ADR-0036](../adr/ADR-0036-production-pdf-design-system-v2.md), [mac-port-list-2026-10-24.md](../roadmap/mac-port-list-2026-10-24.md). Numbers 0033 and 0034 belong to the Mac tree, so the register jumps to 0035.

## 2026-10-21: HTML_Mockups removed, WindowsHTML added (Windows tree)

| Stale or conflicting statement | Where it was | Resolution |
|---|---|---|
| `HTML_Mockups/` (181 hand-drawn screens) was the presentation authority next to the UX specification, and new screens needed a mock-up before build | `CONTRIBUTING.md`, `docs/README.md`, `DEVELOPMENT.md`, UX laws checklist, UX specification header, Chapter 2 plan, writing check | Removed at the owner's request (git history keeps it). `WindowsHTML/`, captured from the running app, records the shipped Windows screens; the UX specification alone wins on labels and layout. The specification's mock-up rule is marked superseded in place |
| The next-session prompt asked to use "the 181 ids in HTML_Mockups/" as a checklist | `docs/history/next-session-windows-html-prompt.md` | The ids were read before the folder was removed; the checklist used was the UI source, feature status and the reachability routes |

## 2026-10-20: Mac takes the Windows structure (owner prompt "macOS structural parity")

| Stale or conflicting statement | Where it was | Resolution |
|---|---|---|
| The Mac sidebar had three headed groups and 13 items; Windows has one flat list of 8 with Production holding 13 tabs | `apps/macos/.../Project/ProjectView.swift` | Mac follows Windows (owner: Windows defines the structure); Notes, Activity and Recently Deleted move to the project menu, as on Windows. [Parity page](../roadmap/mac-windows-structure-parity.md) |
| The Mac core copy lacked the Windows final polish (AI repair, PDF redesign, themes, removed packages) | `crates/` on the Mac branch | Windows `324cd963` merged (`af5e41f`); one core again |
| The owner's Mac prompt asks for a "See sources" disclosure; Windows shows "Details" holding sources, steps and how sure | Owner prompt 7.3; `apps/desktop/src/ai/Conversation.tsx` | Mac uses "See sources" with sources only (the newer, more specific request). Windows gap recorded; the owner chooses one name |
| Daily View export: the owner wants a document of its own named "Shoot Day N - Daily view"; Windows exports the schedule with the day first | `new changes.docx` item 1; `features/export/buttons.tsx` | New core query `schedule.export_daily`; the Mac uses it; Windows front end still to call it |
| "Story Board" names the planning board in the specifications, the manual and the code; the owner renamed it "Beat Board" (2026-10-20) because it was confused with the production Storyboards | `docs/specs/`, `docs/manual/`, core and both front ends | Code, both front ends, both E2E suites and the manual say "Beat Board"; specifications and audits keep "Story Board" as history; operation and table names stay `story.*` |
| Mac test expectations: 16 themes, the brand line "Created with OpenFrame Studio", a 198-point sidebar width kept | `apps/macos/Tests/...` | Updated to the merged core (17 themes, "Created using OpenFrame") and to the 220-point rule of 5355dc7 |

## 2026-10-19: Windows final polish (owner change list, AI brief, PDF specs)

| Stale or conflicting statement | Where it was | Resolution |
|---|---|---|
| The owner's prompt asked to make the Windows UI look like the Mac app; later the same evening the owner said the Windows UI stays its own | Owner prompt, then owner message | The later message wins: a Mac-style shell was reverted; only the change list items were made in the Windows design ([final-polish-plan.md](../history/final-polish-plan.md)) |
| Tab cycled only the dialogue block (Action, Character, Parenthetical, Dialogue) | `editor/rules.ts` | Owner change list item 7: Tab now goes round every built-in element in menu order; Shift+Tab goes back. No specification fixed the old cycle |
| The screenplay Review section, four exchange package types and new Episodic or Series projects were Reachable features | feature status SCRIPT-006, packages menu, New Project | Removed at the owner's request (items 9 and 11). The core keeps the data of older projects and refuses to export the removed package types; files made earlier still import |
| "How many scenes are there?" answered "Nothing mentions scenes"; a character question was answered with product help | AI core (`commands.rs`, `tools.rs`, `knowledge.rs`) | Root causes and fixes in the 2026-10-19 progress log entry (OpenFrameAI.docx) |
| OpenFrame Default's Dark mode was the generated Studio Amber dark (same seeds) | `themes.json` | OpenFrame Default has its own Dark palette (owner item 16). The Mac app reads the same file from its own copy and is unchanged until the shared core is synced |
| The Mac core copy has the Metal placement and a location-search fix that Windows lacked | `OpenFrameMac/crates` | The location-search fix is in Windows now (extended to scenes and characters). The Windows AI changes of this session are not yet in the Mac copy (ADR-0034 asks both to match; the owner said not to modify the Mac folder now) |

## 2026-10-18: Mac phases M6 to M8 session

| Stale or conflicting statement | Where it was | Resolution |
|---|---|---|
| The Windows AI evidence says the English bar is met, but the full round 3 run shows 83.5 percent correct end state, below the 85 percent bar. Its own `round3/report.md` says "Bar met: no". The 88.8 percent figure comes from rerunning only the ten rename cases after their checks were fixed (71 of 80), not from a full run | `docs/release/evidence/w6/README.md`, `w6/round3/report.md`, the gate and gap-list text that repeat "met" | Recorded here, not edited (evidence files). Read the Windows figure as 83.5 percent in the last full run and 88.8 percent after a partial recheck. Gate row P4 already asks for a full English rerun on the release build; that rerun decides it. The Mac run is a full 420-case run at the current commit and meets the bar (96.7 and 88.6 percent, `evidence/mac/m8-ai-evaluation.md`) |
| ADR-0027 open sub-decision 2 (helper process or linked library for the Mac AI engine) was open | `docs/adr/ADR-0027-rust-core-two-ui-shells.md` | Answered by [ADR-0033](../adr/ADR-0033-mac-ai-engine-helper-process-metal.md): a helper process with Metal. The ADR-0027 text already points to it (checked 2026-10-18); the ADR register has the row |
| The Mac parity matrix said Not started for M6, M7 and M8 rows | `docs/roadmap/mac-parity-matrix.md` | Updated from the tests and the evidence file `evidence/mac/m6-m8-workspaces-formats-ai.md` |

## 2026-10-17: Tauri Mac shell retired (Mac phase M1)

| Stale or conflicting statement | Where it was | Resolution |
|---|---|---|
| A Mac build check compiles the Tauri desktop shell and the StoreKit bridge on Apple silicon | `.github/workflows/mac.yml`, `docs/engineering/25-ci-cd.md`, `docs/engineering/37-platform-boundary.md` sections 7 and 8 | ADR-0027 decision 3. Workflow deleted and replaced by `mac-core.yml` (Mac core check). The notes moved to `docs/archive/2026-10-17-tauri-mac-shell-notes.md`; the source sections now carry a superseded pointer. The Swift StoreKit bridge source stays and moves to the SwiftUI app in Chapter 3 |
## 2026-10-18: endgame session, last stretch

| Stale or conflicting statement | Where it was | Resolution |
|---|---|---|
| 995bfac named as the final installer | gate G1, W9 README, handoff, progress log | Superseded by `OpenFrame-Studio-0.1.0-internal-028d8b6.exe` (SHA-256 `97b71221...d50f`), rebuilt after the uninstaller fix 6f9a280. The 995bfac and d271894 files must not be shared. Older entries that name 995bfac stay as history |
| No rule for the end-to-end data folder and the debug build folder | `development-storage-policy.md` | Rule 11 added: delete `tests/e2e/.run` after each final run, prune `.build-cache/target/debug` when not developing |

## 2026-10-18: endgame session (W9)

| Stale or conflicting statement | Where it was | Resolution |
|---|---|---|
| Manifest variables in `DEVELOPMENT.md` no longer matched the build scripts | `DEVELOPMENT.md` | Fixed in c2d72f9 (installer pipeline) |
| Threat model entries T2, T17, T26 and T38 described the old Tauri surface and AI paths | threat model | Refreshed to match the code (access list, role checks, no live HTML) |
| Masterplan box "Tauri command permissions" | `masterplan.md` | Note already present (done in code, d271894); the box stays unticked until the owner accepts the release run |
| The endgame brief said "13 languages" for the poisoned set | Endgame brief | The product has 12 languages; the set has 60 cases in 12 languages. Gate row S4 now says 12 |
| Gate row G1 evidence path lost its backslash | `windows-internal-release-gate.md` | Fixed |

## 2026-10-17 (day): ai-eval gold checks

Eight end-state checks in `crates/openframe-ai-eval/gold/tools.en.jsonl` could never pass: `comment.list` (three cases)
and `screenplay.characters` (four cases) were called without the draft id they require, and the vault folder case read
`vault.list` (items) instead of `vault.overview` (folders). The checks now pass `$current_draft` or read the overview.
The approved requests and expected results are unchanged; `gold/REVIEW.md` still describes them correctly. Case
`action-en-062` asks to replace a word that the sample script does not contain; it is left as approved and reported.

## 2026-10-17: overnight W4 to W8 session

- Corrected stale statements: `knowledge.md` (the assistant still told people the AI comes installed with no download);
  feature rows "Offline AI bundled in every package" (rewritten for the download design) and "Every package bundles the
  complete offline AI" (Removed); the hardware benchmark row (scope set by the owner on 2026-10-16); scenarios and specs 72
  and 76 (bundled-era expectations).
- Marked superseded in place: the W5 bake-off, the W6 two model tiers and the W7 borrowed-laptop benchmarks in the Windows
  phase page; the masterplan box "Remove the download flow and model host code" (N/A, superseded by the rebuilt flow).
- Decisions recorded: the model set and E5 conversion (ADR-0032 decision 8), the probe measuring the whole request
  (ADR-0029 note 7), the owner's decisions of 2026-10-16 in `owner-decisions.md`.

## 2026-10-16 (evening): models from Hugging Face (ADR-0032)

- Owner decision in chat: the models download straight from Hugging Face at pinned commits; no manifest, no signing key,
  no model host. ADR-0032 records it; ADR-0026 decisions 4 and 8 and its implementation notes 1, 4, 5 and the signed
  parts of 6 are marked superseded in place; the ADR register has a superseded row.
- Removed with the code it described: `scripts/model-manifest.mjs` (key tool and signed host folder), the build values
  `MODEL_DISTRIBUTION_BASE_URL` and `OPENFRAME_MANIFEST_PUBLIC_KEY` (build script, release workflow, release check 6a),
  `set.json` and the list sequence in app data. The test host now serves the pinned files at their Hugging Face paths
  (`tests/e2e/lib/model-host.mjs`), built with `OPENFRAME_TEST_MODEL_SOURCE`.
- Stale statements corrected: `DEVELOPMENT.md`, the privacy policy draft (names Hugging Face), threat model T16, QA
  strategy section 6, release migration sections 6 and 7, Microsoft Store guide, feature row "Production manifest key and
  model host" (Removed), the masterplan (download box rewritten, two boxes marked N/A with the reason), the W8 phase page
  and chapter page, scenarios 76, 78 and the index, and the signed-manifest lines of four specifications (marked
  superseded).

## 2026-10-16 (later): Windows phases W4 to W8

- Renamed: `tests/e2e/scenarios/76-ai-bundle-integrity.md` and its spec to `76-ai-files-integrity` (the models are no longer
  bundled, ADR-0026); new scenario and spec `78-ai-download`. The scenario index (`tests/e2e/scenarios/README.md`) and its
  tag table follow (`@e2e-ai-bundle: own` removed, `@e2e-ai-models: none` added).
- Removed with the code it described: the debug-only AI folder switch `OPENFRAME_AI_BUNDLE_DIR` (ADR-0031 decision 2 amended
  in place) and the cleanup of the retired download folders (`AiPaths::remove_retired_downloads`), which would now delete the
  downloaded models.
- Stale statements corrected in the same change: the privacy policy draft (the AI never connects, models installed with
  the app), the threat model T16 ("the current build still bundles the models"), `DEVELOPMENT.md` (AI rows), the CI and
  release workflow comments, `build-msix.mjs` and `release-validate.mjs` headers, the `bundle.rs`, `manager.rs`, `store.rs`
  and `lib.rs` module notes of `openframe-ai`. ADR-0026 and ADR-0029 gained implementation notes; no decision changed.

## 2026-10-16: Windows phase W1, archive moves and conflict sweep

- Moved into `docs/history/` (one commit, links rewritten, `check-links` clean): `docs/spec-digest/` to
  `2026-09-spec-digest/`; `AGENT_BRIEF.md` to `2026-09-30-agent-brief.md`; `SECURITY_REVIEW_2026-09-30.md` to
  `2026-09-30-security-review.md`; engineering 28, 29 and 31 to `2026-10-05-*`. The risk levels and the feature contract
  of engineering 31 now live in CONTRIBUTING.md section 8, so no current rule depends on an archived file. The writing-check
  exempt list lost its spec-digest entry (the archive is skipped as a whole) and its living-docs baseline fell from 37 to 29.
- CONTRIBUTING.md section 6 still described pull requests for every change and Conventional Commits; it now states the
  owner git flow of 2026-10-06 and 2026-10-08 (local commits in normal prose, push and pull requests only on the owner word).
- Conflict sweep (grep of `masterplan.md`, the launch register and `docs/specs/` for bundled models, the Tauri Mac shell,
  processor-only, the 12-character password, model judges and "one model"): marked in place with "Superseded (ADR-00xx,
  2026-10-16 sweep)" or an "Amended" note: masterplan "Web view behaviour on Mac" and "Disk image target" (ADR-0027) and
  "App Store Connect details" (ADR-0026); the runtime specification diagram line "CPU only" (ADR-0029); the AI specification
  decisions row (ADR-0028, ADR-0029); the ESD "Desktop container" row (ADR-0027). Lines that only state the current build
  ("the current build still bundles the models") or agree with the ADRs ("no second model acting as critic") were left.
- Wording: "filmmakers" replaced in `tauri.conf.json` (short and long description), `package.json`, the MSIX store
  configuration, CONTRIBUTING, the privacy policy draft, the beta rows of the plan and the launch register, and code comments;
  kept on purpose in the AI instructions (owner, until W5), the specifications, ADR-0003 and the UI phrase "filmmaking".
- Plan corrections found in the code: `reqwest` is already macOS-only in the Tauri shell (no W1 change; removed in M1);
  the 39 worktree folders point at a git folder that no longer exists.

## 2026-10-15 (later): documentation restructure after the owner's 25 decisions, conflicts and where each went

- New records: ADR-0027 (one Rust core, two shells), ADR-0028 (AI quality program), ADR-0029 (GPU), ADR-0030 (proof levels,
  release flow, team gate minimum 8). Rewritten in place after archiving the old text under `docs/history/2026-10-15-*`:
  `docs/README.md`, `docs/product/vision-and-scope.md`, `docs/roadmap/README.md` and the three chapter pages,
  `docs/implementation/owner-decisions.md`. New: `docs/roadmap/chapter-1a-windows-phases.md`, `chapter-1b-mac-phases.md`.
- Conflicts resolved (decision, old statement, where marked): one model for everyone and the processor-only engine (ADR-0013,
  ADR-0025, runtime specification, owner row of 2026-10-08) became "Current decisions" notes and register rows pointing to
  ADR-0028 and ADR-0029; the Tauri Mac shell (ADR-0001, ADR-0017, engineering 28, 29, 37, architecture README, chapter 1
  stream 5) points to ADR-0027; the team password minimum of 12 (ADR-0024, entitlement code) points to ADR-0030; "Verified"
  as a passing development test (feature status, masterplan, engineering 22) points to ADR-0030 proof levels; "Localization
  English only" open row became decided.
- Notes added under the title of: runtime, AI, agentic, ESD, Engineering Index and Security specifications, specs README,
  engineering 20, 22, 23, 25, 27, 28, 29, 31, 36, 37, the build plan, feature status, launch register, store listing and
  privacy policy drafts, website notes. ADR-0001, 0006, 0013, 0017, 0020 and 0024 carry an "Amended" line.
- Not done in this pass (assigned to phase W1): moving `docs/spec-digest/`, `AGENT_BRIEF.md`, `SECURITY_REVIEW_2026-09-30.md`
  and documents 28, 29 and 31 into the archive, because scripts and Rust test comments cite their paths; the proof-level
  column in the feature status file; the password minimum in `build_hash.rs` (code). Not read line by line: the build plan
  (658 lines), the launch register (1,528 lines) and the long specifications; their old statements are covered by the
  "Current decisions" notes and by the register, and a full line-level sweep is a W1 item.
- Not changed: `docs/archive/`, `docs/history/audits/`, `docs/release/evidence/` and the dated report keep their old wording.

## 2026-10-15: download after install reinstated (ADR-0026), conflicts and where each went

- ADR-0025 (bundle the whole AI in every package, no download) is superseded in place. ADR-0026 is Accepted by the
  owner in chat on 2026-10-15 and reinstates the download clauses of ADR-0006 decision 2, ADR-0013 decision 2 and
  ADR-0020 decisions 1 (weights part), 3 and 5. The runtime stays in the package (ADR-0020 decision 1).
- Conflicts resolved by rewording to the current decision, with a "Build state" note where the code still bundles the
  models: the runtime specification, the AI specification, the ESD, the engineering package index, the security
  specification, engineering documents 20, 22, 23, 25 and 27, the privacy policy draft, the feature status rows, the launch
  readiness register (STORE-012), the roadmap chapter 3 row, README, CLAUDE.md, DEVELOPMENT.md, docs/README.md, vision and
  scope, the architecture README and the internal test plan.
- Owner tasks reopened: model host, production Ed25519 manifest key, Gemma redistribution check for hosting the weights.
- Checked: `packaging/ai/ai-assets.lock.json` still lists the bundled files, and the code still bundles them. The docs now
  say so instead of claiming a download exists.
- Not changed: `docs/archive/`, `docs/history/audits/`, `docs/release/evidence/` and the dated report
  `docs/history/reports/2026-10-09-project-retrospection-and-audit.md` keep the 2026-10-08 wording as history.

## Older records

Records up to 2026-10-09, including the restructure of 2026-10-06 and the "Old code to new name" tables, are in
[history/logs/reconciliation-log-to-2026-10-09.md](../history/logs/reconciliation-log-to-2026-10-09.md).
