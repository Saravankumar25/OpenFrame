# Chapter 1, Windows track: ten phases

> Written 2026-10-15. This is the current plan for finishing the Windows application. It replaces the seven "streams" of the
> earlier Chapter 1 plan (kept in [the archive](../archive/2026-10-15-chapter-1-plan-before-restructure.md)). The Mac track
> ([chapter-1b-mac-phases.md](chapter-1b-mac-phases.md)) starts only when phase 10 here has closed
> ([ADR-0030](../adr/ADR-0030-verification-ladder-and-release-flow.md)). Decisions behind the phases:
> [ADR-0027](../adr/ADR-0027-rust-core-two-ui-shells.md), [ADR-0028](../adr/ADR-0028-ai-quality-program-and-harness.md),
> [ADR-0029](../adr/ADR-0029-gpu-acceleration.md), [ADR-0026](../adr/ADR-0026-offline-ai-download-after-install.md).
> Sizes are planning guesses: Small is under half a day, Medium one to two days, Large three to five days of focused work.
> The detailed checklist with evidence stays in the [build plan](../implementation/masterplan.md); each phase says which
> of its areas it covers.

## 1. The ten phases at a glance

| Phase | Name | Ends with | Needs the owner's word to build? |
|---|---|---|---|
| W1 | Controls and clean baseline | Gates green, history moved, proof levels in the status file | No |
| W2 | Protect writers' work | No known way to lose typed text or a project | No |
| W3 | Harden the core | Fuzzed parsers, property tests, hardened release profile | No |
| W4 | Prove the core on the release build | Release-build end-to-end suite and shared scenario list | A release executable (not an installer) |
| W5 | Measure the AI | Evaluation harness, gold sets, baseline report, model bake-off | No |
| W6 | The AI harness | English bar of 90 and 85 percent met, languages measured | No |
| W7 | GPU acceleration | Decision function, probe, fallback, measured on laptops | No |
| W8 | Download after install | Signed manifest, resume, activation, tiers, tests | No |
| W9 | Installer and clean-machine proof | Internal `.exe` installed on a virtual machine and a borrowed laptop | Yes |
| W10 | Team pilot and freeze | Team findings fixed and re-checked, Windows core frozen | Yes (fix builds) |

Order is strict except where a phase says it can run beside another. W5 may start while W3 and W4 finish, because it needs
only the existing AI code. W7 and W8 may run side by side after W6.

## 2. Phase W1: controls and clean baseline

**Goal.** Start from a repository where the rules agree, the gates exist, and old material is history.

- Accept ADR-0027 to ADR-0030 and the new decision rows ([owner-decisions.md](../implementation/owner-decisions.md)). Done
  in the 2026-10-15 documentation restructure.
- **Proof level column.** Add the five proof levels of ADR-0030 to the feature status file and extend
  `scripts/check-feature-status.mjs` to require a level and its evidence. Relabel today's "Verified" rows as level 2.
- **Team password minimum.** Change the build rule in `crates/openframe-entitlement/src/team_gate/build_hash.rs` from 12 to
  8 characters, update its tests and ADR-0024's text. The password itself is set only in the build computer's environment.
- **Gates.** Add a secret scan of the working tree and history, a link checker for documents and for every address the
  application can open (with an allow-list), and a scan of a built installer for secrets and personal paths (used in W9).
  Wire them into `npm run` scripts and CI.
- **Windows-only dependencies.** ~~Remove `reqwest` from the Windows shell build.~~ Corrected 2026-10-15: the shell
  already compiles `reqwest` for macOS only (`[target.'cfg(target_os = "macos")'.dependencies]` in
  `apps/desktop/src-tauri/Cargo.toml`, used by `platform/mac_inputs.rs`). Windows builds keep it only through
  `openframe-ai`, which needs it. The Mac-only code and its dependency go with the Tauri Mac shell in phase M1
  ([ADR-0027](../adr/ADR-0027-rust-core-two-ui-shells.md)). No W1 code change.
- **History that needs code changes.** Move `docs/spec-digest/`, `docs/engineering/AGENT_BRIEF.md`,
  `SECURITY_REVIEW_2026-09-30.md` and the plan documents 28, 29 and 31 into the archive; update the exempt list in
  `scripts/check-writing-standard.mjs`, the comments in the two Rust test files, `DEVELOPMENT.md` and the ADR citations in
  the same commit; run `node scripts/check-links.mjs`. Done 2026-10-16 (see the archive index).
- **Storage hygiene.** Lower the debug level of the development profile, merge integration test programs into fewer
  binaries, and review the 39 dead worktree folders and 31 unmerged agent branches. Nothing is deleted before the owner
  approves the review table. Found 2026-10-15: the 39 folders under `.claude/worktrees` (4.2 GB) point at
  `C:/dev/OpenFrame/.git`, which no longer exists, so their branches are gone and git does not list them; the review
  compares each folder's files with `main`.
- **Wording.** Change "filmmakers" to "writers, screenwriters and production teams" in the window description, the
  installer text, the AI policy text and the README, once the owner approves the sentence.
- **Owner tasks.** The macOS version of the Intel MacBook; approval of the wording; approval of the worktree review.

**Exit.** All gates run and pass. The status file carries proof levels. The repository contains no old rule next to a new one
(checked by the conflict sweep recorded in the [reconciliation log](../implementation/reconciliation-log.md)).

## 3. Phase W2: protect writers' work

**Goal.** A tester cannot lose text or a project. Build plan areas 3 and 4.

- **Save everything typed before the window closes (Small to Medium).** Add a close-request handler that waits for every open
  editor to send its pending text, then closes. Replace the per-editor unmount flushes with one shared pending-save
  registry used by window close, workspace switch and project switch. Where: `apps/desktop/src/workspaces/screenplay/editor/sync.ts`,
  `ScreenplayEditor.tsx`, `apps/desktop/src-tauri/src/lib.rs`. Verify: a new end-to-end step that types, closes within 100
  ms by the window button and by Alt+F4, reopens, and finds the text; it passes 20 times in a row. The Rust test that a
  shutdown during a save keeps the committed rows.
- **Cloud-synced folders (Medium).** One project-location service detects OneDrive, Dropbox, Google Drive and iCloud Drive
  folders (by markers and known-folder redirection), warns in plain words, and offers a local default. It never moves
  existing projects. Verify with a test under a fake `OneDrive` root and a manual 10-minute typing test on a laptop with
  Documents backed up by OneDrive.
- **Documents folder unknown.** The application refuses to start with a plain message instead of writing into the
  temporary folder (`openframe_project_format::documents_dir`). A test covers it. The owner confirms this choice at review.
- **Project delete and locate tests (Small).** Two integration tests and one end-to-end step on Home. Break the operation
  once on purpose to prove the tests fail.
- **Migration backup restore (Small).** Extend the migration test so the pre-migration backup is opened with the old reader
  and compared with the fixture.
- **Restore rollback reporting (Small).** Replace the silent `let _ =` cleanups in `packages/backup.rs` with an error that
  names the hidden `.replaced-` folder.

**Exit.** Every item has its test. The manual OneDrive test finishes with no lock error and no damaged file.

## 4. Phase W3: harden the core

**Goal.** Make "no known security hole" a sentence backed by evidence. Build plan areas 4, 5 and 6.

- **Property tests** (`proptest`) for these invariants: undo after any sequence of changes returns to the earlier state;
  the search index equals the database; a mutation fully applies or does not apply.
- **Fuzzing** (`cargo-fuzz`) for the parsers that take outside files: FDX, Fountain, DOCX, XML, project and package
  archives. Review the roughly 60 production `unwrap` and `expect` calls, starting with the parser files. Each parser runs
  a fixed fuzz time with no crash; the figure goes in the evidence folder.
- **Per-query viewer test** generated from the registry: every read is refused for a user without View permission.
- **Release profile hardening.** Control-flow guard, stripped symbols, no personal paths in panic text. Verified on the
  release build in W4.
- **Supply chain.** `cargo deny` and `cargo audit` are required gates; review the largest dependencies; keep the dated
  exception for the two font crates and plan their replacement (due 2027-01-06).
- **Threat model.** Update `docs/engineering/20-security-threat-model.md` for the changes of this chapter, trace the web view
  processes for network use (threat T33), and record the known limit of the team gate (ADR-0030).

**Exit.** New gates green; fuzz and property results recorded; the threat model has no open item without a decision.

## 5. Phase W4: prove the core on the release build

**Goal.** Move features from "proved on development" to "proved on the artifact". Build plan areas 5, 9 to 17.

- **Owner's word needed** to build the release executable that the suite drives. It is not an installer.
- **Release-build suite.** The end-to-end suite runs against the release executable, with and without the team gate. A short
  release smoke spec runs first: start, create, type, close, reopen, AI answers, and no network sockets except the engine on
  `127.0.0.1`.
- **Shared scenario list.** Every end-to-end scenario is written as a numbered, plain list of steps in
  `tests/e2e/scenarios/`, the file the Mac track implements again ([ADR-0027](../adr/ADR-0027-rust-core-two-ui-shells.md)).
- **Prove the rows writers use every day** (Large): project create, open, close, duplicate, archive; autosave and recovery;
  Idea Vault; Story Board characters and episodes; Scene hub; comments; drafts and compare; every export format. Each row
  saves, undoes, survives restart and a forced kill, exports and imports where it applies, enforces permissions, and has an
  end-to-end step.
- **Visual planning** gets end-to-end steps for the canvas, the sketch dialog and drag reorder (Medium).
- **Keyboard and screen reader pass** with Narrator on the eight main workspaces (Medium); findings fixed or listed.
- **Performance** on an 8 GB laptop with the AI running, inside the budgets of
  [21-performance-budgets.md](../engineering/21-performance-budgets.md) (Medium).
- **CI.** The suite runs in CI against a test model manifest and a local test host, so the 1.1 GB model files are not needed.

**Exit.** The release-build suite is green; proof level 3 recorded for the Must-ship rows; the scenario list is complete.

## 6. Phase W5: measure the AI

**Goal.** Numbers instead of impressions ([ADR-0028](../adr/ADR-0028-ai-quality-program-and-harness.md) decisions 1, 4, 8, 9). Build plan area 8.

- **Provider interface.** Put the current `llama.cpp` supervisor behind an `InferenceProvider` interface with capabilities
  (context size, languages, grammar support). No behaviour change.
- **Local trace table and AI health card.** One row per AI task with numbers and ids only; the Settings card shows medians,
  apply rate, pass count and the weakest tool; a manual **Export AI report** button writes the numbers file.
- **Evaluation harness.** A runner that executes gold cases through the real pipeline and scores them: retrieval,
  right-tool-offered, tool choice and arguments, schema validity, end state after apply, grounding, language, safety and
  injection, writing (rubric by Claude at development time with a human spot check), speed, memory and processor, context use.
- **Gold sets.** At least 400 cases: 150 retrieval, 150 tool and action, 60 writing, 40 safety. English plus Hindi, Telugu,
  Tamil, Kannada, Malayalam, Spanish, French, German, Chinese, Japanese and Korean. Drafted by Claude from the four sample
  scripts and read by the owner. The non-English sets carry the label "automated review only" (ADR-0028 decision 9).
- **Baseline.** Run the harness on today's system on the release build and on an 8 GB laptop; save the report.
- **Bake-off.** Candidates inside the same harness: Gemma 3 1B (incumbent), larger Gemma models, other open models with a
  clean redistribution licence, and embedding models (the current English one against multilingual ones). Each candidate's
  licence is read first.
- **Owner tasks.** Approve the gold sets; approve the bake-off result and the two model tiers.

**Exit.** The baseline report and the bake-off report are in the evidence folder and the owner has chosen the models.

## 7. Phase W6: the AI harness

**Goal.** Reach the English bar and carry the other languages ([ADR-0028](../adr/ADR-0028-ai-quality-program-and-harness.md)).

Each step ends with a rerun of the W5 harness; a step stays only if a number improves.

- **Stable prompt start.** Put the policy and the core tools first and the changing parts last so the engine reuses its
  stored work; check first-token time. Reduce the tool catalogue against measured accuracy.
- **Tool picker.** Replace English word overlap in `modules/ai/toolbox.rs` with similarity in the embedding space plus usage
  priors; the metric is "right tool offered".
- **Recipes.** Fixed code steps for the ten most used big jobs (summarise an act, continuity check, breakdown suggestions,
  and similar) with one small model step at a time. The existing direct commands are the first recipes.
- **Verifier loop.** The checks and the three-pass, 60-second loop of ADR-0028 decisions 5 and 6, with machine-written
  feedback; the trace records each pass. The checks are deterministic code in one crate shared with the W5 harness; no model judges at runtime; an answer that passes every check on pass 1 is shown at once.
- **Memory v1.** Working, project, episodic and procedural memory, per project with opt-in sharing, the "What OpenFrame AI
  has learned" screen, caps and decay, writes only from the person's own actions.
- **Languages.** Instructions in English, answer in the language of the request, language check as a verifier, a
  multilingual embedding model if the bake-off chose one, a language-neutral stop-word handling in retrieval.
- **Two model tiers** wired through the manifest and the hardware check; update `modules/ai/knowledge.rs` and the policy text.
- **Remaining proofs.** Index rebuild end to end, the poisoned-retrieval corpus, a case per feature with its expected tool and
  permission limit.

**Exit.** On the release build the English gold set reaches 90 percent correct tool and arguments and 85 percent correct end
state. The other languages are measured and reported against the same bar. If a number cannot be reached, the owner is
shown the figures and decides (ADR-0028 decision 2).

## 8. Phase W7: GPU acceleration

**Goal.** Use a graphics card when it is safe ([ADR-0029](../adr/ADR-0029-gpu-acceleration.md)). Build plan area 8.

- The decision function and its table of at least 30 imagined computers.
- Adapter detection on Windows and the Vulkan loader check.
- The Vulkan `llama.cpp` runtime pinned in `packaging/ai/ai-assets.lock.json` beside the processor build.
- The launch plan in the supervisor; the first-run probe, the remembered result per adapter, driver and runtime; the
  restart on the processor if the engine on the card stops.
- The Settings choice (Automatic or Off) and the AI health card line.
- The hidden developer flag, and a run of the whole path on the owner's integrated graphics.
- Benchmarks on each borrowed laptop (processor against card), stored with the card name and driver version.

**Exit.** Unit tests green; the probe and fallback shown on the integrated graphics; each measured card has an evidence file.
A card without a measurement is documented as "designed, unit tested".

## 9. Phase W8: download after install

**Goal.** Rebuild the download flow removed on 2026-10-08 ([ADR-0026](../adr/ADR-0026-offline-ai-download-after-install.md)).

- Order: the manifest signature check; the `*.partial` download with resume; the disk-space preflight; atomic activation;
  quarantine for a bad file; the tests named in ADR-0026 decision 10; then the screens (Download OpenFrame AI, progress,
  pause, resume, remove).
- The manifest carries both tiers and the runtime entries; the hardware check proposes a tier.
- CI tests the flow against a test manifest and a local host.
- **Owner tasks (they block the first real download).** Choose and run the HTTPS model host; generate the production Ed25519
  manifest key and keep the private key offline; confirm the redistribution terms for each chosen model.

**Exit.** Every ADR-0026 test passes; a real download from the chosen host works on a clean computer in W9.

## 10. Phase W9: installer and clean-machine proof

**Goal.** Prove the installer ([ADR-0030](../adr/ADR-0030-verification-ladder-and-release-flow.md) levels 3 and 4).
Needs the owner's word to build. Build plan area 20.

1. Set the team password in the build computer's environment (never in a file), build the internal `.exe`, check its size
   (about 20 to 30 MB is the estimate) and the secret and personal-path scan of the installer.
2. Install on a virtual machine and on a borrowed laptop, both with no developer tools. Start, create a project, type, close,
   reopen, download the AI, use it with the network off, export a PDF, back up and restore on a second Windows account,
   upgrade over the previous build, uninstall, reinstall.
3. Run the gated build through the release end-to-end suite and a wrong-password test (the gate has had unit tests only).
4. Check antivirus behaviour on two setups, WebView2 present, the Visual C++ runtime next to the engine, memory and start
   time. Write down the SmartScreen steps as an installation restriction.
5. Repeat the AI benchmark on each laptop.

**Exit.** Every step passes on two computers and the evidence is saved.

## 11. Phase W10: team pilot and freeze

**Goal.** The team uses the Windows build by hand. Build plan area 17 (the 2 to 3 day pilot). Plan:
[internal-test-plan.md](../testing/internal-test-plan.md).

- The team uses the build as the main writing tool for 2 to 3 days. Every problem is a written bug report
  ([template](../testing/bug-report-template.md)).
- Fixes go into new internal builds (the owner's word each time); each important fix is checked again by the person who found it.
- The pilot also checks wrong-language or poor AI answers in the non-English languages, as ADR-0028 requires.
- On exit the Windows core is tagged and frozen: the Rust core, the operation list and the scenario list. The Mac track
  starts from this tag.

**Exit.** No open defect that loses data, corrupts a project, fails a migration or recovery, lets the AI change a project on
its own, or exposes private notes. The team has signed off. Proof level 5 is recorded for the Must-ship rows.
