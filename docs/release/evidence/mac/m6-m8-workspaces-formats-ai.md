# Mac phases M6 to M8 and StoreKit: evidence

Recorded 2026-10-18 on the development Mac (Apple M1, macOS 26.5.2, Swift 6.3.3 from the Command Line Tools, no full
Xcode). Branch `mac/m1-toolchain`. Commits: 3c38c41 (M6 workspaces, M8 AI screens, real-app test driver), d69efa0 (M7
import, export, packages, PDF and printing), 9ff93b3 (M8 Metal engine, ADR-0033, evaluation figures), d0bc17e (security
review and its fixes), e7d4eeb (StoreKit 2, off in the internal build).

## What "proved" means here

Same meaning as in [m3-m5-shell-editor-workspaces.md](m3-m5-shell-editor-workspaces.md). The Swift tests start the
real Rust core on a temporary folder and drive the view models the window uses, step by step through the shared
scenario files in `tests/e2e/scenarios/`. They are not window-level tests. The only window-level runs are the three
real-app scenarios in the last section, and those ran on one rehearsal build. So the rows below are at proof level 2 for
the logic and the models of the screens. Nothing here is proved on the release `.dmg`.

## Results

`bash apps/macos/scripts/test.sh` on 2026-10-18: **543 tests in 61 suites, all passing.** The count of `@Test`
attributes in `apps/macos/Tests` is 538 in `OpenFrameUITests` plus 5 in `OpenFrameCoreTests` = 543. The sums by file in the tables below are attribute counts.

Rust: `cargo test --workspace --exclude openframe-desktop` passes 2056 of 2057. The one failure was
`core_pipeline::fsd_6_2_continue_remembers_last_location_per_workspace`, which failed once while the load average was
above 80 and passes when run alone. It is a timing test, not a Mac problem, and it is not fixed; it is recorded here so a
repeat is recognised.

Tests for M3 to M5 (253 of the 543 in all, the table in the earlier file) are not repeated. This file covers the other 290
attributes.

## Phase M6: production workspaces (168 tests)

Built in 3c38c41. Sources: `Schedule/`, `CallSheets/`, `Budget/`, `ProjectPages/` (Files, Notes, Recently Deleted,
Activity), and Visual Planning, under `apps/macos/Sources/OpenFrameUI/`.

| Suite file | Tests | Shared scenario implemented at model level |
|---|---|---|
| `ScheduleTests.swift` (suites Schedule scenario, Schedule helpers, Schedule screens) | 9 | `61-schedule.md`, in its order: stripboard, list, calendar, drag a scene into a day, break markers, the call sheet stale review that follows day times |
| `ScheduleEdgeTests.swift` (Schedule edge cases) | 25 | Edge and failure cases of 61: empty project, no scenes, a day with scenes cannot become an off day, undo and redo, finalized schedules, strict validation, wrong input, script changes reviewed and never applied on their own, Day Out of Days range and code style as view state |
| `CallSheetTests.swift` (Call sheets scenario, edge cases, exports, Sides reports and daily view, screens) | 27 | `62-call-sheets-exports.md`: create from a shooting day, snapshot, editor, optional sections, stale and refresh, finalize, new revision, locations "Needs input", sides, six production reports, refused export, project with no schedule |
| `BudgetTests.swift` | 9 | The budget part of 62 ("Planned and Actual with the difference"), money reading and formatting, plans, exports |
| `VisualScenarioTests.swift` | 17 | `86-visual-planning.md`: moodboard canvas, storyboard panels, import, sketch and empty, drag reorder; a file that is not an image, a very large picture, empty names |
| `VisualShotsTests.swift` | 7 | Shot lists and the links between shots and storyboard panels |
| `VisualSupportTests.swift` | 10 | Canvas geometry, text helpers, scene selection, the sketch tool and the PNG it writes, snapshots |
| `ProjectPagesFilesTests.swift` | 19 | Project Files cabinet: managed, external, relink, move by drag payload; the open and save panels are called with the paths the panels would hand over |
| `ProjectPagesNotesTests.swift` | 18 | `80-notes-tasks.md` and its edge cases (nothing typed is lost when a save fails) |
| `ProjectPagesTrashTests.swift` | 14 | Recently Deleted and Activity: the screen part of `25-recently-deleted-purge.md` and the Activity and template parts of `85-project-pages.md` |
| `ProjectPagesSessionTests.swift` | 10 | The rig (a real core, a project open) and the wording helpers |
| `ProjectPagesSnapshotTests.swift` | 3 | The four project-page screens drawn in a real window, light and dark |

## Phase M7: import, export, packages, PDF and printing (35 tests)

Built in d69efa0. Sources: `Interchange/`, `Packages/`.

| Suite file | Tests | Shared scenario implemented at model level |
|---|---|---|
| `InterchangeTests.swift` | 20 | `63-every-export.md` and the import and export parts of `41-screenplay-engine.md`. The FDX round-trip suite of 13 classes (the fixtures of `crates/openframe-import-export/tests/fixtures`, shared with Windows) runs through every format; Indian-script text; selecting no scene cannot export; print preview page range |
| `InterchangeSupportTests.swift` | 0 | Helpers only (fixture finder, rig with a real core, file-panel stub, draft readers) used by both M7 suites |
| `PackagesTests.swift` | 15 | `70-packages.md` in its order: backup, Full Project Package, review package, the reviewer's comment and response, import of the response, restore from Home with the collision choice, a damaged backup, the relaunch. Also stale packages, unmatched notes and the Review Queue, undo, replace after backup, wrong kinds of package, a version from the future, a cancelled task, a folder that cannot be written |

The PDF and print path calls the core's layout engine (`layout::layout`), the same as Windows, so the page count and
breaks of the preview are the core's. A machine without a print subsystem skips the print job; the page range test still
runs (`InterchangeTests.swift`, line 320).

## Phase M8: AI screens and engine (45 tests, plus the evaluation)

Screens built in 3c38c41 (`AI/`, `Settings/AISettingsView.swift`). Engine, lock entry and decision in 9ff93b3:
[ADR-0033](../../../adr/ADR-0033-mac-ai-engine-helper-process-metal.md) (a helper process, Metal, started by the shared
supervisor), `packaging/ai/ai-assets.lock.json`, `packaging/ai/fetch-macos-runtime.sh`.

| Suite file | Tests | Shared scenario implemented at model level |
|---|---|---|
| `AIAssistantTests.swift` | 19 | `73-ai-workspaces.md` (scope line per workspace, chosen scope sticks, the open scene is handed over); `74-ai-change-sets.md` review calls without a model (unknown change set changes nothing, only the newest applied card offers Undo); `72-ai-assistant.md` states without a model (engine missing turns AI off and the rest keeps working, failed request puts the question back, cancel keeps the question, stop with nothing running is harmless); Report an AI output writes a file and sends nothing |
| `AIDownloadTests.swift` | 12 | `78-ai-download.md` and `76-ai-files-integrity.md` without a host: the not-installed card shows size and space and asks nothing; import of a bad or missing file; Remove; Pause with nothing running; settings, graphics choice, Export AI report |
| `AILogicTests.swift` | 14 | Pure view logic: scopes per workspace, panel mode, texts, states |

Evaluation (proof level 2, [m8-ai-evaluation.md](m8-ai-evaluation.md)): 420 English cases on Apple M1 with Metal,
default provider. Correct tool 96.7 percent (bar 90), correct end state 88.6 percent (bar 85), bar met. The poisoned
set is 60 of 60 safe ([m8-eval-poisoned](m8-eval-poisoned/report.md)). Metal against the processor:
[m8-metal-vs-processor.json](m8-metal-vs-processor.json).

## StoreKit 2 for the Mac App Store entitlement (42 tests)

Built in e7d4eeb. It is compiled into the app but off in the internal build: the internal build still shows the team
gate and never names a store screen (checked by `Store internal build`). Sources: `Store/`, `crates/openframe-ffi/src/store.rs`,
`packaging/OpenFrameStudio.storekit`.

| Suite file | Tests | Covers |
|---|---|---|
| `StoreBridgeTests.swift` | 16 | The Swift side of the bridge over a fake Apple: the five rules of `crates/openframe-entitlement/src/mac/bridge.rs` (verified results only, and the rest), and the licensing server client |
| `StoreLicenceTests.swift` | 22 | The licence flow through the real core as the store channel (debug test switch) with a fake Apple and a fake licensing server that signs with a real Ed25519 key: trial start, days left, expiry, purchase, restore, revocation, unverified results dropped, environments never mixed |
| `StoreScreenTests.swift` | 4 | The licence screens drawn in a real window, light and dark; the store build names its first screen, the internal build never does |

## Security review

[m9-security-review.md](m9-security-review.md) is a read-only review of what the Mac adds (12 findings, none critical).
Commit d0bc17e changed these files: the Story attachment open path, the
Mac list of launchable file types in `crates/openframe-security/src/lib.rs`, the hardened-runtime entitlements of the
build script, the installer scan, and `E2E/run.sh`. The review document itself was written before those fixes and still
lists them as findings; read its table with the commit.

## Real-app end-to-end (the only window-level evidence)

Because there is no full Xcode, there is no XCUITest. An injected driver (`apps/macos/Sources/OpenFrameE2EDriver`,
runner `apps/macos/E2E/run.sh`) drives the real app through the accessibility tree. On the release rehearsal build with
the real team gate, three scenarios passed (3 of 3, 2026-10-18):

| Scenario | Result |
|---|---|
| `00-release-smoke/create-type-close-reopen` | pass |
| `17-internal-build-release/team-gate-wrong-then-right` | pass |
| `26-forced-kill-keeps-work/saved-text-survives-sigkill` | pass |

The rehearsal build is not the final `.dmg`, so this is not yet "proved on the Mac release build".

| Run | Result |
|---|---|
| E2E scenario run on the final build | **pending** |

Fill this table from `release-out/e2e/<timestamp>/summary.md` when the final run finishes.

## Known gaps

- **Real StoreKit purchases have not run.** The sheet needs Xcode's StoreKit testing with
  `packaging/OpenFrameStudio.storekit`. Tests use a fake Apple and a fake licensing server.
- **AI Change Set apply with a real model is not proved.** `AIAssistantTests` reaches the review calls without a model.
  Scenarios 72, 74 (Apply, one Undo, Reject, out of date) and 75 with an answer, and 78 with a download, wait for the
  real-app end-to-end work.
- **Crash-safe apply (scenario 75) has no Mac test at model level.** The core's own Rust tests cover the apply; the Mac
  needs the forced-kill run with the AI.
- **The learned-memory screen is empty by design.** The core keeps no learned memory and has no operation to list or
  change it (`Settings/AISettingsView.swift`), so the screen says so. It cannot be tested further until the core has
  the operation.
- **No cancel operation for AI asks in the core.** The Mac Cancel button stops waiting, puts the question back and
  stops the engine (`aiStopRuntime`); the core request itself is not cancelled. A late answer is read from history.
- **Script-change review is covered only by `scriptChangesAreReviewedNeverAppliedOnTheirOwn`** at model level; no test
  edits the script while the Schedule is open in a real window.
- **Drags from Finder and the system file panels** (`NSOpenPanel`, `NSSavePanel`) cannot be driven by the test driver;
  the operations behind them are tested with the paths the panels would hand over. Drag of a scene card into a day is
  tested through the model, not with a mouse.
- **Snapshots are pictures for a person to look at**, not comparisons. There is no visual baseline yet
  (`90-visual-baseline.md`, M9).
- **Opening an exported FDX in Final Draft is not checked here;** the tests check structure and re-import.
- **Windows-only scenario:** `05-webview-network.md` (the web view and its listener check) does not apply; the Mac
  equivalent is the network check in M9.
- **Performance with the AI running** (`97-performance.md`) and **VoiceOver and keyboard passes** (`96`) are M9.
- **One Rust test is timing sensitive** under heavy load (see Results).
