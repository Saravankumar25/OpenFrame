# Reconciliation log

> Records (1) every conflict found between the supplied material, the existing documents and the code, with how it was
> resolved; (2) where each supplied document's content now lives, so nothing is lost; (3) stale statements that were
> corrected; (4) how the old coded names map to the descriptive names now in use.
> Renamed on 2026-10-06 from `03-reconciliation-log.md`. Extended on 2026-10-09 (chapter restructure). Checks written "(checked)" were verified in the repository on the
> date given. Checks written "(stated)" come from a supplied document and were not verified independently.

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
| The owner's prompt asked to make the Windows UI look like the Mac app; later the same evening the owner said the Windows UI stays its own | Owner prompt, then owner message | The later message wins: a Mac-style shell was reverted; only the change list items were made in the Windows design ([final-polish-plan.md](final-polish-plan.md)) |
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

- Moved into `docs/archive/` (one commit, links rewritten, `check-links` clean): `docs/spec-digest/` to
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
  release flow, team gate minimum 8). Rewritten in place after archiving the old text under `docs/archive/2026-10-15-*`:
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
