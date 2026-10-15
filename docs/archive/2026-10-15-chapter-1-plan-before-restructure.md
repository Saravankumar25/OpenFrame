# Chapter 1: complete, test and validate OpenFrame

> Written 2026-10-09 after the full retrospection ([report](../reports/2026-10-09-project-retrospection-and-audit.md)).
> This is the current plan for finishing the application. It replaces the habit of working down the 26 areas of the
> [build plan](../implementation/masterplan.md) in order. The build plan stays as the detailed checklist (every box keeps
> its evidence); this page says what to do next, in what order, and how each item is proved. Work items are named, not
> coded. Sizes are planning guesses: Small is under half a day, Medium is one to two days, Large is three to five days of
> focused work.

## 1. Exit criteria

Chapter 1 is finished when all of these are true, each with evidence:

1. Every Must-ship feature of Chapter 1 is Reachable in the real app and Verified by an end-to-end step on the release
   build (not only on the debug build used by the test suite).
2. No open defect that loses data, corrupts a project, fails a migration or recovery, lets the AI change a project on its
   own, or exposes private notes.
3. The internal Windows `.exe` installs, runs, upgrades and uninstalls on a clean Windows 11 computer, and the AI works
   with the network switched off.
4. The internal macOS `.dmg` does the same on a clean Apple silicon Mac.
5. The owner has used both builds on their own Windows and Mac laptops.
6. Real writers and production teams have used the builds as their main writing tool for 2 to 3 days, every problem they
   found has a written bug report, and every important one has a fix that was checked again.
7. The documents say only what is true ([documentation rules](../README.md)).

## 2. Order of work

The streams are in dependency order. A stream starts when the one before it has no open blocker, except where the page
says two streams are independent.

| Order | Stream | Why this position |
|---|---|---|
| 1 | Protect writers' work | A tester must not lose text. These are the only items that can hurt a person. |
| 2 | Finish and prove the core | Close the gap between "built" and "proved" for the Must-ship features. |
| 3 | Make the AI honest and usable | Independent of 2 after the data items; the AI must be measured before testers rely on it. |
| 4 | Build and prove the Windows installer | Needs 1 and the owner's word to build. |
| 5 | Bring up the Mac build | Independent of 2 and 3, needs 1; the owner has a Mac now. |
| 6 | Run the tester pilot | Needs 4 and 5 and the owner's invitation list. |
| 7 | Development hygiene | Runs beside everything; two items (build setting, worktree review) go first because they save hours. |

## 3. Stream 1: protect writers' work

### Save everything typed before the window closes (Small)

- **Objective.** A person who types and immediately closes the window, or whose computer shuts down, keeps every letter.
- **Where.** `apps/desktop/src/workspaces/screenplay/editor/sync.ts` (waits 400 ms before saving),
  `ScreenplayEditor.tsx` (flushes on screen change), `apps/desktop/src-tauri/src/lib.rs` (the window event calls
  `core.shutdown()` when the window is destroyed). There is no handler for "the window wants to close" that flushes the
  editor first.
- **Expected behaviour.** On a close request the window waits until every open editor has sent its pending text to Rust,
  then closes. The same for Idea Vault notes and call sheet fields, which also wait before saving.
- **Verification.** A new end-to-end step: type a sentence, close the window within 100 ms through the window button and
  through Alt+F4, reopen the project, find the sentence. A Rust-level test that shutdown during an in-flight save keeps the
  committed rows. Repeat on the Mac with Command+Q.
- **Accepted when.** The step passes 20 times in a row on Windows and on the Mac.
- **Status.** Unverified risk. The code was read; the loss was not reproduced. The first job is to write the failing step.

### Projects inside cloud-synced folders (Medium)

- **Objective.** A writer whose Documents folder is synced by OneDrive does not get a damaged or locked project.
- **Why.** Projects live in `Documents\OpenFrame\Projects`. Windows 11 often redirects Documents into OneDrive.
  [ADR-0003](../adr/ADR-0003-project-folder-format.md) allows cloud folders and says sync "may still interfere". A
  SQLite database in write-ahead mode (the `-wal` and `-shm` files) is a known bad fit for sync clients.
- **Where.** `crates/openframe-project-format/src/lib.rs` (`default_projects_dir`), `crates/openframe-security` (path
  checks), the new-project dialog and Home.
- **Expected behaviour.** OpenFrame detects a synced location (OneDrive, Dropbox, Google Drive, iCloud Drive by folder
  markers and known-folder redirection), tells the person in plain words, and offers a local folder as the default. It
  never moves existing projects by itself.
- **Verification.** A test with a folder under a fake `OneDrive` root; a manual test on a Windows laptop with Documents
  backed up by OneDrive, typing for 10 minutes while the client syncs.
- **Needs the owner.** The default location and the wording (open question).
- **Accepted when.** The manual test finishes without a lock error or a damaged file, and the warning appears for the four
  clients.

### Project delete and "locate a moved project" tests (Small)

- **Objective.** The two Home actions that touch whole projects are tested.
- **Where.** `crates/openframe-application/src/modules/project.rs` (`delete_project`, `locate`).
- **Expected behaviour.** Delete moves the folder to the Recycle Bin or Trash and removes it from recents; locate finds a
  project that was moved and keeps its identity; both refuse a network folder.
- **Verification.** Two integration tests and one end-to-end step on Home. Today no test calls either operation.
- **Accepted when.** Tests fail when the operation is broken (shown by breaking it once on purpose).

### Extend the migration backup test to prove the backup can be restored (Small)

- **What exists.** `tests/migration_fixtures.rs` (lines 234 and 354) already checks that a `pre-migration-v*.sqlite` file
  exists for every old schema version, for projects and for the Global Idea Vault. The feature row CORE-005 still says "no
  test asserts the backup"; that text is stale and is corrected in this restructure.
- **Remaining gap.** The test checks that the file exists, not that opening it gives back the old content.
- **Verification.** Open the backup file with the old schema reader and compare a few rows with the fixture.
- **Accepted when.** The test fails if the backup is empty or damaged.

### Decide what happens when the Documents folder cannot be found (Small, owner decision)

- **Objective.** The app never writes writers' work into a temporary folder by accident.
- **Where.** `openframe_project_format::documents_dir` falls back to the system temp folder (recorded as an open owner
  decision).
- **Recommendation.** Refuse to start with a plain message that names the problem.
- **Accepted when.** The owner decides and a test covers the choice.

### Bring the status text of three rows in line with the code (Small, documents only; done 2026-10-09)

Three feature rows described the code as it was on 2026-10-05: CORE-004 listed the recovery dialog wording defect (fixed in
`RecoveryDialogs.tsx`, `describeRecovery`), CORE-005 said no test asserts the migration backup (it does), and CORE-015
said the diagnostics export is missing (it is built, with `SupportDiagnostics.test.tsx`). The rows were corrected in the
restructure. This is also a lesson: the feature status text lags the code, and an audit must read the code.

## 4. Stream 2: finish and prove the core

The counts below come from `node scripts/check-feature-status.mjs` on 2026-10-09: 65 Verified, 45 Reachable, 30 Built,
11 Designed, 17 Specified. Chapter 1 has 142 product rows; 105 are Must-ship; of those 36 are Verified.

### Prove the Reachable rows that writers use every day (Large)

- **Objective.** Move the rows a writer touches in the first hour from Reachable to Verified.
- **Rows first.** Project create, open, close, duplicate, archive (CORE-001); autosave and recovery (CORE-003, CORE-004);
  Idea Vault views and add (VAULT-001 to VAULT-003); the Story Board characters and episodes (STORY-005, STORY-006);
  Scene hub (SCRIPT-022); Comments and notes (SCRIPT-005); drafts and compare (SCRIPT-006); export formats
  (XFER-002, XFER-004 to XFER-008).
- **Verification.** For each row: saves, undoes, survives restart and a forced kill, exports and re-imports where it
  applies, enforces permissions, and has an end-to-end step. The definition is in [CONTRIBUTING.md](../../CONTRIBUTING.md).
- **Accepted when.** The listed rows read Verified with test names in the evidence column.

### Visual planning gets end-to-end steps (Medium)

Moodboards, storyboards and shot lists (VIS-001 to VIS-003) are Reachable with unit tests but no end-to-end step for the
canvas, sketch dialog or drag reorder. Add one spec for the three tabs.

### Keyboard and screen reader pass (Medium)

- **Objective.** Every workspace can be used with the keyboard alone, with visible focus and labelled icon buttons.
- **Where.** Feature row CORE-017 (Specified) and area 17 of the build plan (0 of 14 done).
- **Verification.** A checklist run by a person on Windows (Narrator) and on the Mac (VoiceOver) for the eight main
  workspaces; automated checks for unlabelled buttons already exist in the journey spec and are extended.
- **Accepted when.** The checklist is filled in and its findings are fixed or listed as known.

### Performance on an ordinary laptop (Medium)

- **Objective.** Typing, opening a 120-page script and switching workspaces stay inside the budgets in
  [21-performance-budgets.md](../engineering/21-performance-budgets.md) on an 8 GB laptop.
- **Verification.** `npm run perf` on the joined code (not run since the merge), plus a manual check on a laptop with 8 GB
  of memory with the AI engine running.
- **Accepted when.** The figures are in the evidence folder and within budget.

### Change the product wording from "filmmakers" (Small, owner approval)

The window and installer description, the AI policy text and the README say "filmmakers". The owner's description is
writers, screenwriters and production teams. Text that the app shows is product behaviour, so the owner approves the new
wording first. Files: `apps/desktop/src-tauri/tauri.conf.json`, `crates/openframe-application/src/modules/ai/orchestrator.rs`
(the policy text), `README.md`, `CONTRIBUTING.md`.

## 5. Stream 3: make the AI honest and usable

The assistant is built and its safety rules are strong (it can only propose; a person applies). Its usefulness is
unmeasured. Area 8 of the build plan is 2 of 22 done.

### Measure retrieval and tool choice with a gold set (Large)

- **Objective.** Numbers instead of impressions: how often the right scene, card or character is found, and how often the
  right tool is picked.
- **Where.** New test data and a harness beside `crates/openframe-application/tests/ai_retrieval.rs`; the metrics are
  recall at 1, 5 and 10, mean reciprocal rank, grounding and leakage (feature row AI-003).
- **Verification.** At least 120 questions across Story, Screenplay, Breakdown, Schedule and Vault, in English and in
  Telugu and Hindi, run against the real bundled model. The owner approves the thresholds after the first run.
- **Accepted when.** The report is in the evidence folder and every miss below the threshold has a decision: fix, or tell
  the user.

### Decide what to do about non-English requests (Medium, after the measurement)

The tool picker scores English words in the request against tool names and descriptions
(`modules/ai/toolbox.rs`, `tools_for_request`), and the embedding model is `bge-small-en`. A request in Telugu or Hindi
matches almost nothing. The measurement says how bad this is. The options (keep and say so, add a small multilingual
embedding model, or route by a fixed menu) are an owner decision with numbers. The model is not changed without the
owner's approval.

### Hardware benchmark (Medium)

Run `scripts/ai-benchmark.mjs` on an 8 GB laptop, a recommended laptop and the owner's Mac. Record first answer time,
steps per request, memory and disk. Publish the minimum hardware in the installer notes (feature row AI-004).

### Remaining AI proofs (Medium)

- Index rebuild end to end (`AI-006`), the poisoned-retrieval corpus (`AI-008`), a test case per feature that records the
  expected tool and the permission limit, and an update of `modules/ai/knowledge.rs` for the features added since.
- Change the policy sentence "a filmmaking application" when the owner approves the wording.
- "Report AI output" belongs to Chapter 3 (store policy) unless the owner wants it for testers.

## 6. Stream 4: build and prove the Windows installer

Rule: the installer is not built until the owner says so in that conversation.

### Decide the team password for testers (Small, owner decision)

The gate is built (`crates/openframe-entitlement/src/team_gate/`). It asks the password on every launch and slows wrong
tries. For 2 to 3 days of real use by writers the owner decides: keep it, remove it, or use a short shared password. The
hash is built from `OPENFRAME_TEAM_PASSWORD` on the build computer. The gate has only unit tests; the end-to-end suite
runs builds without it. Whatever is chosen, the gated build gets its own test.

### Build and install on a clean Windows 11 computer (Medium)

- **Steps.** Set `OPENFRAME_TEAM_PASSWORD` (if kept), run `npm run bundle`, copy the installer to a clean computer or
  virtual machine, install, start, create a project, type, close, reopen, use the AI with the network switched off,
  export a PDF, make a backup and restore it on a second Windows account, upgrade over the previous build, uninstall,
  reinstall.
- **Check.** Installer size (the AI adds about 1.1 GB; the NSIS limit is 2 GB), installation time, start time, memory use,
  the Visual C++ runtime next to the engine, WebView2 present.
- **Document.** The SmartScreen warning ("Windows protected your PC", More info, Run anyway) as an installation
  restriction, not a defect.
- **Accepted when.** Every step passes on two machines and the evidence is saved.

### Smoke test the release build (Medium)

The end-to-end suite drives a debug build. Add a short spec that runs against the release executable with a gated or
ungated build: start, create, type, close, reopen, AI answers, no sockets except the engine on 127.0.0.1.

## 7. Stream 5: bring up the Mac build

The owner has a Mac now. Everything Mac-specific is unverified until it runs there.

1. **Build on the Mac.** Install Rust, Node and Xcode command line tools; run `cargo check` for the desktop shell and
   `npm run dev`. Fix what does not compile. Record the exact steps in `DEVELOPMENT.md`. Note: `.cargo/config.toml` makes every
   build go through `sccache`; on a Mac without it, install `sccache` or set `RUSTC_WRAPPER` to an empty value (CI does
   this), otherwise the first build fails.
2. **Pin a macOS AI engine.** `packaging/ai/ai-assets.lock.json` holds only a Windows x64 llama.cpp. Add the Apple silicon
   build of llama.cpp (processor or Metal; a decision with a benchmark), with sizes and hashes, and load it from
   `Contents/Resources/ai`. This needs the owner's approval of the engine choice because it changes what ships.
3. **Add the disk image.** `tauri.conf.json` lists only `nsis` and `msi`. Add the `dmg` target for the Mac build,
   keep it unsigned, and document the Gatekeeper steps (right-click Open, or remove the quarantine flag) as an
   installation restriction.
4. **Check the Mac behaviours.** Spell check (the Mac adapter is written but never run), file dialogs, shortcuts with the
   Command key, window close and quit, data in `~/Library/Application Support/OpenFrame`, documents in
   `~/Documents/OpenFrame`.
5. **Mac test automation.** `tauri-driver`, which the end-to-end suite uses, does not support macOS (the Tauri
   documentation says so). Options: the WebdriverIO Tauri service named in the same documentation, or a written manual
   checklist run by a person. Decide after the first manual pass.
6. **The internal build does not need StoreKit.** The Swift StoreKit bridge compiles on every Mac build. If it gets in the
   way, compile it only for the store channel.

Accepted when the `.dmg` installs on a clean Apple silicon Mac, the AI answers with the network off, and the Mac checklist
is complete.

## 8. Stream 6: the tester pilot

Described in [../testing/internal-test-plan.md](../testing/internal-test-plan.md): who, what they do each day, how
problems are reported ([template](../testing/bug-report-template.md)), how fixes are checked again, and when the pilot
stops.

## 9. Stream 7: development hygiene

- Set the debug-information level of the development profile to zero, which removes about 18 of the 34 GB in the build
  folder, and merge the integration tests of each crate into fewer binaries (item already in the build plan).
- Review and then remove the 39 dead worktree folders under `.claude/worktrees` and the 31 unmerged agent branches.
  Nothing is deleted before the review table is approved.
- Adopt the rules in [development-storage-policy.md](../development-storage-policy.md) section 8.

## 10. What moves to Chapter 3

Store packaging, Partner Center, App Store Connect, StoreKit and Microsoft Store purchase calls, the licence server,
price and refund rules, store listings, certification, signing and notarization, the end-user licence agreement, the
privacy policy hosting, "Report AI output", and the first 90 days after launch. They are listed with the 83 build plan
boxes tagged "Later: store publishing" in [chapter-3](chapter-3-commercial-distribution.md).
