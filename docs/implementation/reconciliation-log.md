# Reconciliation log

> Records (1) every conflict found between the supplied material, the existing documents and the code, with how it was
> resolved; (2) where each supplied document's content now lives, so nothing is lost; (3) stale statements that were
> corrected; (4) how the old coded names map to the descriptive names now in use.
> Renamed on 2026-10-06 from `03-reconciliation-log.md`. Extended on 2026-10-09 (chapter restructure). Checks written "(checked)" were verified in the repository on the
> date given. Checks written "(stated)" come from a supplied document and were not verified independently.

## 2026-10-26: five-lane AI documents moved to history

Owner instruction (chat): dead documentation is shifted to history as part of the S9 close-out.

- `docs/engineering/ai-answer-paths.md` (the five lanes) moved to `docs/history/2026-10-26-ai-answer-paths-five-lanes.md`,
  with one added "Superseded by" line. A new `docs/engineering/ai-answer-paths.md` describes the one conversational path
  built in slices S3 to S8.
- `docs/specs/ai-harness-rebuild.md` moved to `docs/history/2026-10-24-ai-harness-rebuild-plan.md`, with one added line.
  The 2026-10-24 owner decision to keep it in `docs/specs/` is replaced by this instruction.
- Links in ADR-0035, ADR-0037, the ADR register, owner decisions, the progress log, this log and the spec now point to the
  history copies; the specs index row is removed; the history index has two new rows. `node scripts/check-links.mjs` OK.

## 2026-10-24: one history folder, harness and layout upgrade recorded

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

## 2026-10-21: HTML_Mockups removed, WindowsHTML added

| Stale or conflicting statement | Where it was | Resolution |
|---|---|---|
| `HTML_Mockups/` (181 hand-drawn screens) was the presentation authority next to the UX specification, and new screens needed a mock-up before build | `CONTRIBUTING.md`, `docs/README.md`, `DEVELOPMENT.md`, UX laws checklist, UX specification header, Chapter 2 plan, writing check | Removed at the owner's request (git history keeps it). `WindowsHTML/`, captured from the running app, records the shipped Windows screens; the UX specification alone wins on labels and layout. The specification's mock-up rule is marked superseded in place |
| The next-session prompt asked to use "the 181 ids in HTML_Mockups/" as a checklist | `docs/history/next-session-windows-html-prompt.md` | The ids were read before the folder was removed; the checklist used was the UI source, feature status and the reachability routes |

## 2026-10-19: Windows final polish (owner change list, AI brief, PDF specs)

| Stale or conflicting statement | Where it was | Resolution |
|---|---|---|
| The owner's prompt asked to make the Windows UI look like the Mac app; later the same evening the owner said the Windows UI stays its own | Owner prompt, then owner message | The later message wins: a Mac-style shell was reverted; only the change list items were made in the Windows design ([final-polish-plan.md](../history/final-polish-plan.md)) |
| Tab cycled only the dialogue block (Action, Character, Parenthetical, Dialogue) | `editor/rules.ts` | Owner change list item 7: Tab now goes round every built-in element in menu order; Shift+Tab goes back. No specification fixed the old cycle |
| The screenplay Review section, four exchange package types and new Episodic or Series projects were Reachable features | feature status SCRIPT-006, packages menu, New Project | Removed at the owner's request (items 9 and 11). The core keeps the data of older projects and refuses to export the removed package types; files made earlier still import |
| "How many scenes are there?" answered "Nothing mentions scenes"; a character question was answered with product help | AI core (`commands.rs`, `tools.rs`, `knowledge.rs`) | Root causes and fixes in the 2026-10-19 progress log entry (OpenFrameAI.docx) |
| OpenFrame Default's Dark mode was the generated Studio Amber dark (same seeds) | `themes.json` | OpenFrame Default has its own Dark palette (owner item 16). The Mac app reads the same file from its own copy and is unchanged until the shared core is synced |
| The Mac core copy has the Metal placement and a location-search fix that Windows lacked | `OpenFrameMac/crates` | The location-search fix is in Windows now (extended to scenes and characters). The Windows AI changes of this session are not yet in the Mac copy (ADR-0034 asks both to match; the owner said not to modify the Mac folder now) |

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
- Not changed: `docs/history/`, `docs/audits/`, `docs/release/evidence/` and the dated report keep their old wording.

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
- Not changed: `docs/history/`, `docs/audits/`, `docs/release/evidence/` and the dated report
  `docs/reports/2026-10-09-project-retrospection-and-audit.md` keep the 2026-10-08 wording as history.

## 2026-10-09: chapter restructure, contradictions found and where each went

Found while reading the repository for the audit report (checked 2026-10-09):

| Conflict or stale statement | Where it was | Resolution |
|---|---|---|
| Audience said "filmmakers" | `README.md`, `CLAUDE.md`, `AGENTS.md`, `tauri.conf.json` descriptions, AI policy text in `modules/ai/orchestrator.rs`, listing drafts | Owner's wording is writers, screenwriters and production teams. README and CLAUDE.md changed; product text waits for the owner's approved sentence (open row in owner-decisions) |
| Internal `.dmg` "Developer ID signed and notarized" | Old `CLAUDE.md` line 8 | Contradicted the same file's current goal and the ADR-0024 amendment (unsigned for now). `CLAUDE.md` rewritten |
| "Not installed on purpose: claude-mem" | Old `CLAUDE.md` | `claude-mem` is enabled in the owner's user settings. Recorded in `CLAUDE.md`; the owner decides |
| GSD framework (33 agents, hooks) | `.claude/` (git-ignored), installed 2026-10-06 | Not mentioned in any project document. Recorded in `CLAUDE.md` |
| `README.md` feature table | Said Idea Vault "in development", themes "not started", paid trial "not built" | Rewritten from `feature-status.md` |
| `AGENTS.md` (untracked) | Old copy of `CLAUDE.md` | Left untouched; flagged to the owner |
| CORE-004 "defect" | `feature-status.md` | Fixed in code (`RecoveryDialogs.tsx`); row corrected |
| CORE-005 "no test asserts the backup" | `feature-status.md` | `migration_fixtures.rs` lines 234 and 354 assert it; row corrected |
| CORE-015 "Missing" | `feature-status.md` | Built (`diagnostics.rs`, `SupportDiagnostics.tsx`); row corrected, state Built |
| Mac "built later on a real Mac" | `CLAUDE.md`, `owner-decisions.md`, memory | The owner has a Mac now; row added to owner-decisions |
| "Three sessions with nine areas each" | Owner's recollection | The repository has 26 numbered areas (build plan), earlier lanes A to G (2026-10-03 plan) and milestones M0 to M16 (2026-10-05); "session 1, 2, 3" were groupings of areas by the agents. No 3 by 9 grid exists |
| Europe price 79 euros | First message of 2026-10-09 | Owner corrected to 89 euros the same day |
| Mock-ups 164 to 168 (live LAN) and 155 (permissions) | `HTML_Mockups/` | Remain as history; barred from website use in `vision-and-scope.md` |
| `.claude/worktrees` "cleaned" | Progress log 2026-10-07 | 39 folders (4.2 GB) still exist; recorded in the storage policy |

Documents added: `docs/product/vision-and-scope.md`, `docs/roadmap/` (4 files), `docs/testing/` (3 files),
`docs/architecture/README.md`, `docs/history/README.md`, `docs/implementation/handoff.md`, `docs/reports/2026-10-09-project-retrospection-and-audit.md`.
Documents rewritten: `docs/README.md`, `CLAUDE.md`. Documents amended: build plan (chapter map, nine boxes), owner decisions
(seven decided rows, eight open rows), feature status (three rows), storage policy (sections 7 and 8), `README.md`
(audience and status table), `DEVELOPMENT.md` (tree line), `docs/spec-digest/README.md` (marked historical).
Nothing was moved or deleted. Files that scripts, tests or ADRs cite by path stay where they are and are listed as
historical in `docs/history/README.md`.

## 2026-10-09: stale feature status statements corrected after the session 2 verification

The joined `session-2` passed every gate and the full end-to-end run (evidence:
`docs/release/evidence/2026-10-08-session-2/e2e-session-2.md`). While ticking boxes and moving rows, these statements in
[feature-status.md](feature-status.md) were found stale and corrected (checked 2026-10-09):

| Row | Stale statement | Now |
|---|---|---|
| THEME-007 | OpenFrame Default Light has 6 pairs below 4.5 to 1, an owner question | The owner decided on 2026-10-07 to raise them; `theme.rs` (unit) `contrast_every_builtin_theme_passes_in_light_and_dark` allows no exception |
| STORY-CARD-005 | Several screenplays per project is still an open owner decision | Decided and built on 2026-10-08 (ADR-0012 amendment); new rows SCRIPT-024 and SCRIPT-025 |
| SCRIPT-017 | The Production column shows data once the ADR-0015 migration lands | Migration 0013 landed on 2026-10-07 and fills the column |
| CORE-007 | The 30-day purge has no E2E run | `25-recently-deleted-purge.test.mjs` passes |
| XFER-012 | Restore is only in the project menu, not on Application Home | Restore Backup or Package is on Application Home |
| CALL-001 to CALL-005, BUDGET-001, TEMP-002, TEMP-003, BRAND-001, SCHED-003 to SCHED-006, DOOD-001 | "Missing", "pending" or field gaps checked on 2026-10-05 | Built by areas 14 and 15 on 2026-10-08 with tests and E2E 61 and 62 |
| Several rows citing E2E specs | "written, not yet run" | The specs ran green on 2026-10-09 |

No plan box was unticked. The box notes added on 2026-10-09 in [masterplan.md](masterplan.md) say why each remaining
box in areas 8 to 18 stays open.

## 2026-10-08: the download flow is removed; the AI is bundled (ADR-0025 implemented)

Branch `area/08-bundled-offline-ai`. The code that downloaded Offline AI is deleted and the AI ships in every package
(evidence: `docs/release/evidence/2026-10-08-session-2/bundled-offline-ai.md`).

**Documents brought in line with the code (checked 2026-10-08):** `docs/engineering/22-qa-test-strategy.md` (offline
gate, AI test layers, failure injection), `23-release-migration.md` §7 (rewritten: bundled AI prerequisites),
`25-ci-cd.md` (`OPENFRAME_AI_ASSETS_DIR`, the overlay config), `27-microsoft-store.md` (§5.4 rewritten, `internetClient`
removed, certification notes), `20-security-threat-model.md` (T16 and T33 rewritten, REL-01 marked superseded),
`SECURITY_REVIEW_2026-09-30.md` (REL-01 marked superseded), `docs/store/privacy-policy.md`, `docs/store/listing.md`,
`docs/release/launch-readiness-register.md` (STORE-012), `docs/implementation/feature-status.md` (AI-001, AI-002,
AI-006, AI-007, AI-011 updated; AI-012 Removed; CORE-013 journey step), `README.md`, `DEVELOPMENT.md`, and the generated
`docs/engineering/ai-tool-coverage.md` (it was also stale for call sheet and template operations merged earlier in
session 2; regenerated from the live registry).

**Superseded in place (banner at the top, text kept):** `docs/specs/OpenFrame_Local_AI_Runtime_Model_Management_Specification.md`,
`OpenFrame_Studio_AI_Specification_Updated.md`, `openframe_ai_agentic_rag_spec_prompt.md`,
`OpenFrame_ESD_Technical_Architecture.md`, `OpenFrame_Studio_Engineering_Package_Index.md` and
`OpenFrame_Studio_Security_Privacy_Data_Ownership_Specification.md`: every passage about **Download Offline AI**, the signed
manifest, the model host, install, update, remove and the graphics-card build is history. `docs/spec-digest/prd.md` is a
digest of the supplied material and keeps its note as written.

**Removed with the code:** `crates/openframe-ai/{keys,manifest,tools}/` (development manifest, its key and signing
tool), the `MODEL_DISTRIBUTION_BASE_URL` and `OPENFRAME_MANIFEST_PUBLIC_KEY` build variables and the release check 6b,
the Ed25519 helper in `openframe-security`, the graphics-card (Vulkan) runtime choice: only the processor build is
pinned, so nothing else could be tested together as one set (ADR-0025 decision 4).

## 2026-10-08: current goal, store publishing later, Offline AI bundled in every package

Owner answers in chat on 2026-10-08: (A) the current goal is to finish building, testing, verification, end-to-end tests
and packaging of the internal Windows `.exe` and Mac `.dmg`; publishing to the Microsoft Store and the Mac App Store is a
later step; (B) the `.dmg` is built and tested on a real Apple silicon Mac later; (C) the internal builds stay unsigned
for now and signing moves to the store and public step; (D) the team password gate stays on and the owner sets
`OPENFRAME_TEAM_PASSWORD` on the build PC; (E) every package contains the complete offline AI and nothing is downloaded
(new [ADR-0025](../adr/ADR-0025-offline-ai-bundled-in-every-package.md)). All five are in the decided table of
[owner-decisions.md](owner-decisions.md).

**Superseded in place (text kept, marked, pointer to ADR-0025):**

| Older text | Where | Now |
|---|---|---|
| Delivery by **Download Offline AI** from a model host with a signed manifest | ADR-0006 decision 2, its first two Consequences and the rejected alternative "Bundling the model in the installer" | Bundled in every package, pinned hashes, no host |
| "One download" of the runtime and both models | ADR-0013 decision 2 | Bundled |
| Only model weights download after a click; listings disclose the on-demand download; model host and manifest key set before release | ADR-0020 decisions 1 (weights part), 3 and 5, and one Consequence | Bundled; nothing to disclose; neither host nor key needed |
| Internal `.dmg` Developer ID signed and notarized | ADR-0024 decision 1 and Consequences | Amended: unsigned for now, signing at the store and public step |
| "How the AI engine ships in each channel" (weights download, model host still manual) | owner-decisions.md decided table | Struck through, points to the new row |
| Manual task "Choose the model host ... manifest key" | owner-decisions.md manual work | No longer needed; the Gemma review is now a redistribution check |

**Retagged in the build plan.** Every box in `masterplan.md` was read. 83 boxes carry "(Later: store publishing)" and 20
carry "(Needs Mac hardware)" (some carry both). Per area, store publishing and Mac: area 1, 12 and 0; area 7, 1 and 7;
area 8, 2 and 2; area 17, 1 and 1; area 18, 7 and 2; area 19, 14 and 0; area 20, 2 and 2; area 21, 8 and 6; area 22,
11 and 0; area 23, 12 and 0; area 24, 7 and 0; area 25, 5 and 0; area 26, 1 and 0. Nothing was ticked, unticked or
deleted. Reworded: the intro (four release files, current goal), the security standard "Distribution" line, build order
row 20, area 8 "Offline AI inside each package" and "Gemma redistribution check recorded", the area 7 sandbox box, area 20
"Internal Mac disk image" (now unsigned) and its "Done when", the area 19, 21, 22, 23 and 24 boxes that named the
download. Marked N/A with a reason: the area 1 model host part, area 8 "Download resilience", area 19 "Release build fails
to compile when the AI list public key is not set". New boxes: area 8 "Bundle the weights and tokenizer with pinned
hashes", "Remove the download flow and model host code" and four "Offline AI validation" boxes (`.exe`, `.dmg`, `.msix`,
`.app`); area 20 "Signing of the internal Mac disk image" (split from the old box) and "Internal `.exe` build with the
team gate on, unsigned". `npm run plan:status` now also prints "Current goal" (boxes without the store publishing tag)
and "Later: store publishing".

**Documents that still describe the download.** `docs/engineering/22-qa-test-strategy.md`, `23-release-migration.md`,
`25-ci-cd.md`, `27-microsoft-store.md`, `docs/store/privacy-policy.md`, `docs/implementation/feature-status.md` and the AI
and runtime specifications in `docs/specs/` describe the download code as it is today. They are updated in the change
that bundles the weights and removes that code (area 8 box "Remove the download flow and model host code"). Until then
ADR-0025 wins where they disagree.

**Stale lines fixed in passing.** `docs/adr/README.md` named 0022 as the next free ADR number (now 0026).
`docs/README.md` still listed ADRs 0014 to 0016 and 0020 as awaiting the owner; all four were Accepted on 2026-10-07 and
2026-10-08. The masterplan intro still showed three release files after ADR-0024 made it four.

## 2026-10-08: owner decisions, several screenplays per project, stale script name

Owner answers in chat on 2026-10-08: work reaches `main` by a local merge only after tests and end-to-end specs pass, with
no push, pull request or repository change unless the owner asks; ADR-0020 Accepted (the AI engine ships inside every
package); a project may hold several screenplays (ADR-0012 amendment); the Mac trial is confirmed as a free download with
an in-app purchase after 7 days and a full block at expiry (ADR-0022); price, currency and refund rules are decided later.

**Several screenplays per project: data check (checked 2026-10-08 on `session-2`).** The schema already allows it:
`migrations/project/0004_screenplay.sql` has no unique rule that limits a project database to one `screenplay` row, every
draft names its screenplay (`screenplay_draft.screenplay_id`), and scenes, the production source, the schedule and the
revision and lock tables all name a draft. No migration is needed. Drafts are versions of one screenplay, not separate
screenplays, so they do not answer the decision. The one-screenplay limit lives in code:

| Place | What it assumes | Change needed |
|---|---|---|
| `crates/openframe-application/src/modules/screenplay/mod.rs` (module comment) | One screenplay per project or episode | Reword |
| `screenplay/mod.rs` `screenplay_for_scope` | `LIMIT 1` returns the oldest screenplay only | Return a list, or take a `screenplay_id` |
| `screenplay/mod.rs` `ScreenplayOverview` | One `screenplay`; arguments only `episode_id` | Add the list and a `screenplay_id` argument |
| `screenplay/drafts.rs` | Creating a screenplay is refused when one exists | Remove the guard |
| `story/build.rs` | `NewScreenplay` refused when one exists | Remove the guard |
| `interchange/import.rs` | `can_create_screenplay` true only when none exists | Always allow |
| `packages/exchange.rs` `current_draft(None)` | Picks the first screenplay with `LIMIT 1` | Require an id |
| `ai/catalog/story.rs` | New-draft target is the most recently updated screenplay | Take the screenplay as an argument |
| `ai/catalog/screenplay.rs` | Refuses to create when one exists | Remove |
| `apps/desktop/src/workspaces/screenplay/index.tsx` | Route takes only `episodeId`; the import dialog sends a `screenplayId` that is ignored | Read the screenplay id and add a picker |
| `apps/desktop/src/workspaces/story/BuildDialog.tsx` | Shows "a project has one screenplay" | Enable the option |
| `apps/desktop/src/features/interchange/model.ts` | Comment and import modes say one screenplay | Small edit |
| Tests `story.rs`, `screenplay.rs`, `interchange.rs`, `ai_toolbox.rs` | Check that a second screenplay is refused | Turn them into tests that it is allowed |

Already ready for several screenplays: `interchange/export.rs` lists them all, `story/build.rs` returns a list, and the
production source and import dialogs cope with more than one. Stale specification lines, kept as history and overruled by
the ADR-0012 amendment: the Functional Specification near line 3401 and `docs/spec-digest/prd.md` item C-28. The build work
is the area 11 box "Several screenplays per project".

**Stale script name.** `feature-status.md` still told readers to run `scripts/check-ledger.mjs`, which was renamed to
`scripts/check-feature-status.mjs` on 2026-10-06, so the command failed with `MODULE_NOT_FOUND`. The three lines now name
the current script. The archived documents keep the old name as history.

**Two histories named main.** There is no local `main` branch. `origin/main` (`d30b588`) is an ancestor of `session-2`.
The local branch `master` (`cd60f08`, tagged `baseline/pre-masterplan-2026-10`) carries 80 commits of Offline AI work in
a history that shares no commit with `origin/main`. Recorded here; resolved at the local merge to `main`.

## 2026-10-09: correction to commit fbed4bc

The message of commit `fbed4bc` says 64 feature rows moved to Verified. The correct number is 62 (65 rows moved up in all: 62 to Verified, CALL-006 and TEMP-002 to Reachable, AI-007 to Built). `feature-status.md` itself is right; only the commit message is wrong, and commits are not rewritten.

## 0. Restructure on 2026-10-06: descriptive names replace codes

The owner found the milestone, wave, gate, priority and decision labels (M1.01, W2, G7, P0, D5 and similar) impossible to read.
All plan documents now use plain headings. Nothing was deleted: the coded originals are kept unchanged in `docs/history/`.

| New document | Replaces | Archived original |
|---|---|---|
| [masterplan.md](masterplan.md): 26 numbered areas with checklists | `00-masterplan.md` (milestones M0 to M16, waves W1 to W5, gates G0 to G14) | [2026-10-06-masterplan-with-item-codes.md](../history/2026-10-06-masterplan-with-item-codes.md) |
| [feature-status.md](feature-status.md): one row per feature, plain status words | `01-feature-ledger.md` (priorities P0 to P2, states S0 to S5, milestone column) | [2026-10-06-feature-ledger-with-item-codes.md](../history/2026-10-06-feature-ledger-with-item-codes.md) |
| [owner-decisions.md](owner-decisions.md): decisions and manual work by name | `02-owner-decisions-and-manual-work.md` (D1 to D18, X1 to X21) | [2026-10-06-owner-decisions-with-item-codes.md](../history/2026-10-06-owner-decisions-with-item-codes.md) |
| this file | `03-reconciliation-log.md` | [2026-10-06-reconciliation-log-with-item-codes.md](../history/2026-10-06-reconciliation-log-with-item-codes.md) (content also carried over below) |
| [progress-log.md](progress-log.md) | `04-progress-log.md` | none (entries carried over under "Earlier entries"; file created 2026-10-06 in the first build session, the restructure had left only the old file) |

Scripts renamed with them: `scripts/check-ledger.mjs` is now `scripts/check-feature-status.mjs`, and `scripts/lib/ledger.mjs` is now
`scripts/lib/feature-status.mjs`. New: `scripts/plan-status.mjs` counts checked and total boxes per area.

Owner answers that day: paid app with a 7-day free trial; Windows 11 only; Apple silicon only; three release files (Microsoft Store
package, Windows installer, Mac App Store package) from one codebase with a Windows adapter and a macOS adapter; GitHub flow with a
feature branch, a pull request and merge only when checks pass.

### Old code to new name

Use these tables when you meet an old code in an older entry, a code comment, an evidence folder or a commit message.
Evidence folders under `docs/release/evidence/` keep their old names on purpose, because they are records.

**Milestones to areas of the build plan**

| Old code | Area in masterplan.md |
|---|---|
| M0 | 2. Project controls and baseline (owner tasks: 1. Decisions and owner tasks) |
| M1 | 3. Data safety and crash recovery; 4. Runtime resilience and failure testing; 5. Test infrastructure and quality gates; 6. Security hardening |
| M2 | 11. Screenplay engine and file formats |
| M3 | 10. Writing comfort; 12. Appearance and themes |
| M4 | 9. Story Board and Idea Vault |
| M5 | 13. Breakdown, catalog, locations, cast and crew |
| M6 | 14. Shooting schedule and Day Out of Days |
| M7 | 15. Call sheets, budget, templates, samples and exports |
| M8 | 16. Packages and backup |
| M9 | 8. AI assistant quality and hardening |
| M10 | 17. Product-wide quality, accessibility and beta |
| M11 | 19. Microsoft Store package |
| M12 | 22. Legal, privacy, website and support |
| M13 | 23. Final review and submission |
| M14 | 24. Certification, publication and launch |
| M15 | 25. First 90 days after launch |
| M16 | 7. Platform adapter layer for Windows and Mac; 20. Windows installer; 21. Mac App Store package |

**Common plan items**

| Old code | Plain name |
|---|---|
| M0.18 | baseline run |
| M0.19 | feature status check |
| M0.20 | visual baseline |
| M0.21 | supply-chain checks |
| M0.22 | feature status check in CI |
| M0.23 | architecture lint |
| M0.24 | migration policy |
| M0.28 | security baseline audit |
| M1.01 | crash-safe AI changes |
| M1.02 | forced stop at every apply step |
| M1.03 | forced stop tests |
| M1.04 | migration test files |
| M1.05 | migration failure cases |
| M1.06 | property tests |
| M1.07 | stale proposal tests |
| M1.08 | soak tests |
| M1.09 | index shutdown stress |
| M1.11 | failure injection |
| M1.12 | end-to-end tests in CI |
| M1.13 | end-to-end specs per workspace |
| M1.14 | reachability test |
| M1.15 | hostile file collection |
| M1.16 | PDF image comparison |
| M1.17 | screen visual comparison |
| M1.18 | log redaction test |
| M1.19 | export diagnostics |
| M1.20 | performance budgets |
| M1.22 | link and junction safety |
| M1.23 | network drive refusal |
| M3.02 | custom keyboard shortcuts |
| M9.04 | AI tool list regenerated |

Every other item code is explained by the archived original plan, which lists the full text of each item.

**Decisions (old D codes) and manual work (old X codes)**

| Old code | Name |
|---|---|
| D1 | Business model and licence |
| D2 | Scope of the first release |
| D3 | Windows versions |
| D4 | macOS scope and timing |
| D5 | Diagnostics, telemetry and analytics |
| D6 | Theme and dark mode system |
| D7 | Scene production numbers |
| D8 | PDF fonts for Indic scripts |
| D9 | How the AI engine ships in each channel |
| D10 | Crash-safe AI changes |
| D11 | Publisher and product identity |
| D12 | Template list |
| D13 | Sample scripts |
| D14 | Support model |
| D15 | Direct download channel |
| D16 | Launch date and capacity |
| D17 | Beta programme |
| D18 | Localization |
| X1 to X5 | Legal entity, Partner Center account, name reservation, licence file, name clearance (see owner-decisions.md, manual work) |
| X6 to X9 | Domain and email, model host and signing key, Gemma terms review, counsel review |
| X10 to X13 | Test hardware, certification kit run, beta, sample scripts |
| X14 to X19 | Store listing, support setup, continuity, marketing assets, go or no-go, submission |
| X20, X21 | Apple account and signing, Windows signing certificate and updater key |

**Waves, gates, priorities and states**

| Old label | Now |
|---|---|
| W1 to W5, WO | Dropped. The build order table in masterplan.md gives the order, and each area lists what it waits for. |
| G0 to G14 | The final item of each area ("done when") plus the release sign-off table in masterplan.md |
| P0, P1, P2 | Must ship, Should ship, Can wait |
| S0 to S5 | Specified, Designed, Built, Reachable, Verified, Release-ready |
| C1 to C26 (conflicts below) | Numbers kept so older notes still resolve; each row also has a plain title |

## Earlier records (old item codes inside)

The sections below are the original log. They keep the codes they were written with, so read them with the tables above.

### Conflicts and their resolution

| # | Conflict | Evidence | Resolution | Owner |
|---|---|---|---|---|
| C1 | Business model: free/open-source (repo) vs commercial with 7-day no-card trial (launch register). | README, `LICENSE-PENDING.md`, Engineering Index vs register COMM-001..006 | ADR-0018 (Proposed). Commerce rows are conditional in the masterplan. | Owner D1 |
| C2 | Windows support: Windows 11 vs Windows 10 1809+. | README, DEVELOPMENT vs `MinVersion 10.0.17763.0` in `27-microsoft-store.md` and `listing.md` [V] | ADR-0017 recommends Windows 11. Manifest and listing change with the decision. | Owner D3 |
| C3 | macOS: "Windows 11 first, macOS-ready" vs "equal first-class". | Engineering Index vs doc 28 | ADR-0017: architecture rules now, macOS release as M16, outside the Store gate. | Owner D4 |
| C4 | Light theme only vs Light/Dark/System plus 15 themes. | ADR-0012 §7 [V] vs spec §47A | ADR-0014 (Proposed). Docs 30 holds the engineering spec. | Owner D6 |
| C5 | No stored scene numbers vs imported, alphanumeric, frozen production numbers. | ADR-0011 item 3 [V] vs spec §12 | ADR-0015 (Proposed). Closes the ADR-0012 open question. | Owner D7 |
| C6 | PDF Latin-1 only vs P0 Indic-script round trip including PDF. | ADR-0008 [V] vs spec §19 | ADR-0016 (Proposed): embedded fonts and shaping. | Owner D8 |
| C7 | Telemetry: none (README) vs opt-in diagnostics (Engineering Index) vs analytics and crash aggregation (register). | [V] three documents | ADR-0021 (Proposed): nothing automatic; user-initiated diagnostics. | Owner D5 |
| C8 | Model runtime downloaded after install vs bundled for the Store. | `27-microsoft-store.md` §5.4; audit §12, §19 | ADR-0020 (Proposed). | Owner D9 |
| C9 | AI Change Set apply is not crash-atomic. | `change_set.rs` has no journal or recovery [V]; audit §10 | ADR-0019 (Proposed); milestone M1. | Owner D10 |
| C10 | Migration policy: "pre-release migration files may still be edited" (doc 23) vs "never edit a migration on main" (completion plan). | doc 23 §4 vs plan §10 | New rule: files 0001 to 0010 are **frozen at the first external build** (internal RC or beta). Until then edits are allowed but discouraged. Every new schema change is a new numbered file. Recorded in doc 23. | Engineering |
| C11 | Schedule model: plan proposes a new unified `ScheduleItem` type; the repo already has `schedule_marker` with Meal, Travel, Company Move, Custom plus time and duration. | `0007_schedule.sql` [V] | **Extend, do not replace.** Add day timeline columns and strip start time. A unified item table would rewrite working code and violates the plan's own "no rewrite" law. | Engineering |
| C12 | Budget: spec wants Planned and Actual; repo has one `amount`. | `budget_line` [V] | New migration; existing `amount` becomes Planned; add Actual. | Engineering |
| C13 | Templates: repo has 7 built-ins; spec recommends 8 with different members. | `notes/templates.rs` [V] | Owner decision D12. | Owner D12 |
| C14 | Scene card has only `short_description`; spec wants a long description. | `0003_story.sql` [V] | Additive column. | Engineering |
| C15 | Screenplay element types are a fixed CHECK list of 8; custom elements and page breaks need more. | `0004_screenplay.sql` [V] | SQLite cannot alter a CHECK in place: table-rebuild migration with the id/rev/timestamps preserved (doc 23 §4 rule 3). | Engineering |
| C16 | Audit says LAN residue remains in code (`Capability::HostSession`, `ActorOrigin::Lan`, "local-network collaboration" message). | No match in `crates/` for those symbols; `ActorOrigin` has Local, Exchange, Ai; the network-folder message already points to copy or packages [V] | **Already resolved in code.** Remaining LAN text is in specification and manual files (321 mentions in 35 Markdown files). Banners now, body rewrite tracked as DOC-003. ADR-0007 consequences note updated. | Docs |
| C17 | Audit says `DEVELOPMENT.md` claims later crates do not exist. | `DEVELOPMENT.md` §5 text; crates `openframe-search`, `-import-export`, `-ai` exist [V] | Corrected in `DEVELOPMENT.md`. | Docs |
| C18 | Doc 23 says project schema version 9. | 10 files in `migrations/project/` [V] | Corrected to 10. | Docs |
| C19 | `LICENSE-PENDING.md` and older notes mention a Qwen GGUF. | `LICENSE-PENDING.md` L26 [V]; ADR-0013 and Local AI spec say Gemma 3 1B and BGE small | Corrected in `LICENSE-PENDING.md`. | Docs |
| C20 | Engineering Index lists documents `10_` to `30_` by names that do not exist; real files are `docs/engineering/NN-*.md` and the specifications in `docs/specs/`. | [V] | Index rewritten with real paths. | Docs |
| C21 | Engineering Index Phase 6 still lists LAN host, discovery, soft locks. | [V] | Replaced with file-package collaboration. | Docs |
| C22 | Completion plan says "no screenplay pagination; build once". The repo already has a PDF layout engine (`layout.rs`, 868 lines) with MORE/CONT'D. | [V] | Plan treats it as the starting point; editor parity is verified in M2. | Engineering |
| C23 | Store listing and privacy policy describe features the README marks "In development". | `docs/store/listing.md` vs README [V] | Listing carries a gating banner: no claim may be submitted until its ledger row is S5. | Release |
| C24 | The supplied checklist and the completion plan use different phase numbers (0 to 13 vs 0 to 11), the OS plan uses A to F, the Store audit uses P0/P1/P2. | [V] | Single milestone list M0 to M16 in the masterplan with a crosswalk (§3). | Docs |
| C25 | AI "agentic spec" file `openframe_ai_agentic_rag_spec_prompt.md` sits in the root as a prompt. | [V] | Kept in place because code and docs cite its section numbers; classified as the implemented AI behaviour spec in `docs/README.md`. | Docs |
| C26 | The user manual (`.docx`, `.pdf`) still contains the LAN figures 164 to 168. | **Verified 2026-10-05 [V]:** text extracted from the `.docx` has 181 figures and shows "Starting a local collaboration session (host)" (164) and "Joining a session (participant)" (165) | Regeneration planned (DOC-007). Source of truth for the manual to be decided in M10. | Docs |

### Where each supplied document lives now

| Supplied document | Now | Treatment |
|---|---|---|
| `OpenFrame_New_Spec_Revised.md` | `OpenFrame_Studio_Screenplay_and_Production_Depth_Specification.md` (repo root) | Promoted to a normative specification (Revision 1), header added, content unchanged. §47A kept with its numbering. |
| `OpenFrame_End_to_End_Completion_Implementation_Plan_2026-10-03.md` | `docs/history/2026-10-03-end-to-end-completion-plan-source.md` | Archived with a banner. Its strategy, laws, completion states, AI integration pipeline, PR checklist and agent protocol are absorbed into `00-masterplan.md`, `CONTRIBUTING.md` and doc 31. |
| `OpenFrame_Manual_vs_GitHub_Audit_Updated_2026-10-03.md` | `docs/audits/2026-10-03-manual-vs-repository-audit.md` | Evidence snapshot. 181-figure table stays here; the ledger references figure ranges. |
| `OpenFrame_Production_Microsoft_Store_Readiness_Audit_2026-10-02.md` | `docs/audits/2026-10-02-production-and-store-readiness-audit.md` | Evidence snapshot. Every P0/P1/P2 item became a masterplan or ledger row (see §4). |
| `OpenFrame_Master_Product_Launch_Operations_Readiness_Checklist_REVISED.md` | `docs/release/launch-readiness-register.md` | Promoted as the canonical operations register; additions from the intake notes appended (Appendix F). |
| `OpenFrame_Optimal_HLD_LLD_Desktop_App_2026-10-03.md` | `docs/engineering/29-desktop-hld-lld.md` | Promoted as target architecture; header explains overlap with the ESD. |
| `OpenFrame_Strong_OS_Compatibility_and_Security_Windows_macOS_2026-10-03_REVISED.md` | `docs/engineering/28-cross-platform-compatibility-security.md` | Promoted; scoped by ADR-0017. |
| `OpenFrame_Vibe_Coding_Gaps_and_Coverage_2026-10-03.md` | `docs/engineering/31-ai-assisted-development-controls.md` | Promoted; CONTRIBUTING points to it. |
| `claude_premium_website_design_motion_prompt.md` | `docs/website/prompts/premium-website-design-motion-prompt.md` | Kept as a prompt. Used by `docs/website/README.md`. Tool names in it are unverified. |
| `claude_production_readiness_anti_ai_slop_prompt.md` | `docs/website/prompts/production-readiness-anti-ai-slop-prompt.md` | Same. |
| `info.txt` (Store certification tips, brand checklist, 40 extra items) | `docs/history/notes/store-certification-and-launch-gap-notes.md` | Content absorbed into `27-microsoft-store.md` §12 and register Appendix F. |
| `All required docs.txt` (documentation stack guidance) | `docs/history/notes/documentation-stack-guidance.md` | Absorbed into `docs/README.md` documentation map. |

### Phase crosswalk

| Masterplan | Completion plan | Launch register Appendix B | OS plan | Store audit |
|---|---|---|---|---|
| M0 | Phase 0 | Phase 0 | | P0 items 2, 3, 4 |
| M1 | Phase 0 and 11 (hardening) | Phase 0, 8 | | P0 12; P1 15 to 17 |
| M2 | Phase 1, 8 | Phase 1 | | |
| M3 | Phase 2 | Phase 2 | | |
| M4 | Phase 3 | Phase 3 | | |
| M5 | Phase 4 | Phase 4 | | |
| M6 | Phase 5 | Phase 5 | | |
| M7 | Phase 6, 7 | Phase 6 | | |
| M8 | Phase 9 | Phase 7 | | |
| M9 | Phase 10 | Phase 8 | | P0 6, 7; P1 13, 14 |
| M10 | Phase 11 | Phase 8, 12 | | P1 21, 24 |
| M11 | Phase 11 | Phase 9, 10 | | P0 8 to 11; P1 20, 22 |
| M12 | | Phase 11 | | P0 5 |
| M13 | | Phase 12 | | |
| M14 | | Phase 12, 13 | | |
| M15 | | Phase 13 | | |
| M16 | | Phase 9, 10 (macOS, direct) | Phases A to F | P2 25, 26 |

### Store audit findings mapped

| Audit item | Where it is tracked |
|---|---|
| Choose first Store scope | D2, M0 |
| `cargo deny` failing | PLAT-003, M0 |
| Licence selection | REL-001, M0 |
| Partner Center identity | REL-002, M0 |
| Privacy policy final | REL-006, M12 |
| AI output reporting | AI-010, M9 |
| Production AI distribution trust | AI-012, M9 |
| Supported Windows versions | PLAT-001, M0 and M11 |
| RC MSIX from real pipeline | REL-003, M11 |
| WACK | REL-004, M11 |
| Installed-package E2E on clean machines | REL-005, M11 |
| Change Set interrupted-apply recovery | AI-005, M1 |
| Hardware benchmarks | AI-004, M9 |
| Retrieval evaluation | AI-003, M9 |
| Desktop E2E required | PLAT-002, M1 |
| Forced-kill and soak tests | CORE-003, CORE-004, M1 |
| Bundle runtime in MSIX | AI-011, M9 |
| Diagnostics export | CORE-015, M1 |
| Accessibility and high-DPI pass | CORE-016, CORE-017, M10 |
| Listing accuracy | REL-007, M13 |

### Items from the intake notes (`info.txt`) not in the supplied register

Added to the register as Appendix F: Store certification pre-flight rows (WACK, clean sandbox, privacy URL,
IARC rating for mature screenplay content, Notes for Certification, manifest capabilities), real-filmmaker beta,
launch trailer, founder story, creator and community outreach, 30-day content bank, status and outage
communication, customer feedback system, localization architecture, public documentation, versioning strategy,
support SLA, funnel analytics (reconciled with ADR-0021), post-launch incident response.


### Documentation restructure 2026-10-05 (nothing deleted)

Reason: the repository root held 13 loose specification files plus the manual. They moved with `git mv` (history kept) and names are unchanged. Links were rewritten and checked (0 broken). No code or script referenced them by name.

| Old path (repository root) | New path |
|---|---|
| `OpenFrame_Studio_Mega_PRD_Aligned_Updated.md` | `docs/specs/OpenFrame_Studio_Mega_PRD_Aligned_Updated.md` |
| `OpenFrame_Studio_Functional_Specification_Document_Aligned_Updated.md` | `docs/specs/OpenFrame_Studio_Functional_Specification_Document_Aligned_Updated.md` |
| `OpenFrame_Studio_UX_UI_Specification_Updated.md` | `docs/specs/OpenFrame_Studio_UX_UI_Specification_Updated.md` |
| `OpenFrame_Studio_Domain_Data_Specification_Updated.md` | `docs/specs/OpenFrame_Studio_Domain_Data_Specification_Updated.md` |
| `OpenFrame_Studio_Screenplay_and_Production_Depth_Specification.md` | `docs/specs/OpenFrame_Studio_Screenplay_and_Production_Depth_Specification.md` |
| `OpenFrame_Studio_Offline_Collaboration_Specification.md` | `docs/specs/OpenFrame_Studio_Offline_Collaboration_Specification.md` |
| `OpenFrame_Studio_Import_Export_Specification.md` | `docs/specs/OpenFrame_Studio_Import_Export_Specification.md` |
| `OpenFrame_Studio_Security_Privacy_Data_Ownership_Specification.md` | `docs/specs/OpenFrame_Studio_Security_Privacy_Data_Ownership_Specification.md` |
| `OpenFrame_Studio_AI_Specification_Updated.md` | `docs/specs/OpenFrame_Studio_AI_Specification_Updated.md` |
| `OpenFrame_Local_AI_Runtime_Model_Management_Specification.md` | `docs/specs/OpenFrame_Local_AI_Runtime_Model_Management_Specification.md` |
| `openframe_ai_agentic_rag_spec_prompt.md` | `docs/specs/openframe_ai_agentic_rag_spec_prompt.md` |
| `OpenFrame_ESD_Technical_Architecture.md` | `docs/specs/OpenFrame_ESD_Technical_Architecture.md` |
| `OpenFrame_Studio_Engineering_Package_Index.md` | `docs/specs/OpenFrame_Studio_Engineering_Package_Index.md` |
| `OpenFrame_Studio_User_Manual.docx` | `docs/manual/OpenFrame_Studio_User_Manual.docx` |
| `OpenFrame_Studio_User_Manual.pdf` | `docs/manual/OpenFrame_Studio_User_Manual.pdf` |

Also created: `docs/adr/README.md` (decision register: active, superseded, proposed), `docs/specs/README.md` (status of each specification), `docs/spec-digest/README.md` (digests are derived and partly stale).

Older decisions: 57 section headings carrying live-LAN content were marked **SUPERSEDED by ADR-0007** in place (PRD 2, FSD 13, UX 3, Domain 1, Offline/Collaboration 16, Security 22). Text unchanged. Nothing was removed. Other superseded or amended decisions are listed in `docs/adr/README.md` section 2.

Remaining at the root by design: README, CONTRIBUTING, DEVELOPMENT, CLAUDE, LICENSE-PENDING, THIRD_PARTY_LICENSES (the last is written by `scripts/license-inventory.mjs`).

Masterplan recut: every box now has a wave tag (W1 to W5 and the owner track WO); W1 is 56 of 224 boxes (25.0%); box M0.28 (Windows security baseline audit) and section 1a (security standard) were added.

2026-10-05 (Wave 1 session 1): the M0.28 security baseline added 10 boxes (M1.21 to M1.28 in W1, M11.15 and M11.16 in W4) and amended M1.18, M9.09, M9.16 and M10.07. Totals are now 234 boxes, W1 64 (27.4%). ADR-0019 and ADR-0021 were accepted by the owner, who delegated the choice to the recorded recommendation. The ledger gained `UI route` and `Tests` columns and per-section `Source:` lines; all [R]/[A] tags were replaced with evidence (M0.19), which corrected 65 states. Earlier ledger statements that were wrong: XFER-001 (unknown FDX constructs are flattened into Action), XFER-008 (the import report lives in `modules/interchange/import.rs`), XFER-010 (no PDF golden bytes exist), SCRIPT-010 (keep-together rules exist), CORE-013 (the quoted "unavailable offline" string does not exist), CORE-014 (only panic and startup lines are redacted), VAULT-004 (research flags are not implemented), BRK-004 and BRK-005 (partially implemented, not missing). Doc 23 also listed the global schema version as 2; it is 3.
