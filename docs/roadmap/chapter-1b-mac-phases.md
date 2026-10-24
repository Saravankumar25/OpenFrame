# Chapter 1, Mac track: ten phases

> Written 2026-10-15. This is the plan for the macOS application: a native SwiftUI shell over the same Rust core that the
> Windows track finished and froze ([ADR-0027](../adr/ADR-0027-rust-core-two-ui-shells.md)). It starts only when Windows
> phase W10 has closed ([chapter-1a-windows-phases.md](chapter-1a-windows-phases.md), [ADR-0030](../adr/ADR-0030-verification-ladder-and-release-flow.md)).
> It replaces stream 5 of the earlier Chapter 1 plan, which assumed a Tauri Mac app (see the
> [archive](../history/2026-10-15-chapter-1-plan-before-restructure.md)). Release target: Apple silicon Macs only. Sizes as
> in the Windows plan.

## 1. What can be proved where

The owner has an Intel MacBook now and expects an Apple silicon Mac later. Intel Macs are never a release target. They are
useful because most of the work is not specific to Apple silicon.

| Check | Intel MacBook (if its macOS can run a recent Xcode) | Apple silicon Mac |
|---|---|---|
| Rust core builds and its tests pass for macOS | Yes (an `x86_64` build) | Yes |
| UniFFI interface, XCFramework, Swift package tests | Yes | Yes |
| SwiftUI screens, menus, shortcuts, window and file behaviour | Yes | Yes |
| `llama.cpp` on the processor | Yes | Yes |
| Metal speed, memory use, battery | No | Yes |
| Apple's on-device model | No (needs Apple silicon and Apple Intelligence) | Yes |
| The `.dmg` that is released | No | Yes |

Every Mac phase therefore records two evidence levels: "proved on Intel" and "proved on Apple silicon". A row that has
only the first is labelled so, and the Mac track does not close until the second exists
([ADR-0030](../adr/ADR-0030-verification-ladder-and-release-flow.md)). The macOS version of the Intel MacBook decides how
recent an Xcode it can run: owner task in phase M1.

## 2. The ten phases at a glance

| Phase | Name | Ends with |
|---|---|---|
| M1 | Mac controls and toolchain | Intel Mac builds the core and runs its tests; Tauri Mac shell retired |
| M2 | The Rust core as a Swift library | UniFFI interface, XCFramework, event stream, generated Swift types |
| M3 | SwiftUI shell foundation | App, windows, menus, Home, project open and create, settings, themes |
| M4 | The screenplay editor | Spike result, decision, and the editor with every element and shortcut |
| M5 | Story, Idea Vault and Breakdown | These workspaces on the shell |
| M6 | Production workspaces | Schedule, call sheets, budget, visual planning, files, activity, notes and tasks |
| M7 | Import, export, packages and PDF | Every format and package on the shell, PDF preview, printing |
| M8 | Mac AI | Provider design, Metal engine, optional Apple model, harness on the Mac, bar met |
| M9 | Mac verification and the `.dmg` | Mac end-to-end suite, accessibility, release build, clean Apple silicon Mac |
| M10 | Team pilot and close | Team findings fixed, parity matrix complete, Chapter 1 closed |

M5 to M7 are independent of each other after M4 and may run in any order. M8 may start after M3, beside M4 to M7, because
the AI harness is already finished and measured on Windows.

## 3. Phase M1: Mac controls and toolchain

- Record the macOS version of the Intel MacBook and what Xcode it supports. Install Rust, Xcode and the Swift toolchain.
- Build the Rust core for macOS and run the tests of every crate that has no Windows-only code. Fix what does not compile.
  Note for `DEVELOPMENT.md`: `.cargo/config.toml` forces `sccache`; a Mac without it must set `RUSTC_WRAPPER` to an empty
  value, as CI does.
- **Retire the Tauri Mac shell** ([ADR-0027](../adr/ADR-0027-rust-core-two-ui-shells.md) decision 3): remove the macOS
  compile-only CI job that builds it, and move its notes into the archive. Keep the Swift StoreKit bridge source; it moves to
  the SwiftUI app in Chapter 3 and is not used by the internal build.
- Create the Mac parity matrix: every Windows Must-ship row and every Windows end-to-end scenario, with a column for the Mac
  status. This file is the Mac track's checklist.
- Create the empty Xcode project, Swift package and CI job (Apple silicon runner) that build with the core.

**Exit.** The core's tests pass on the Mac; the parity matrix exists; the project builds.

## 4. Phase M2: the Rust core as a Swift library

- A new crate `openframe-ffi` exposes: start and stop the application core, send an operation name with JSON arguments and
  receive a JSON result or a coded error, subscribe to the event stream, and platform callbacks (file dialogs, reveal, power
  state). Nothing else.
- Generate Swift types from the operation registry metadata, the same source that generates the TypeScript bindings, so the
  two shells cannot drift.
- Package as an XCFramework in a Swift package. Handle threading and long operations explicitly so no call blocks the main
  thread.
- Tests: Rust tests of the interface, Swift package tests that open a project, create a scene, undo, and receive an event.

**Exit.** A Swift test program drives the core end to end through the interface.

## 5. Phase M3: SwiftUI shell foundation

- App lifecycle, window and menu structure with Mac conventions (Command key, Option, standard menus), the recovery and
  error dialogs, Home (recent projects, new project, open, templates and samples), the project open and create flows, the
  data-location note, the cloud-synced folder warning shared with Windows.
- Settings, the 15 themes with Light, Dark and System, the logo colour following the theme, the Offline AI settings
  placeholder, the diagnostics export.
- The internal team password screen (the Mac gate follows [ADR-0024](../adr/ADR-0024-internal-team-builds.md) and the
  8-character minimum of ADR-0030); a wrong-password test.
- The Mac platform adapter behaviours run for the first time here: paths, atomic replace, process cleanup, spelling.

**Exit.** Home to an empty project and back works; themes and the gate work; the first Mac end-to-end scenarios pass.

## 6. Phase M4: the screenplay editor

The editor is the costliest and riskiest screen, so it comes first inside the screen phases.

- **Spike (about two weeks).** Build the same small script in two ways and measure: a native text engine (TextKit 2) and the
  existing web editor inside a small web view. Measure typing latency on a 120-page script, selection and caret behaviour,
  input methods for Hindi, Telugu, Tamil, Kannada, Malayalam and Chinese, Japanese and Korean, spell check, undo grouping,
  suggestions, accessibility with VoiceOver, and how much existing editor logic can be reused.
- **Decision.** The owner decides from the numbers ([ADR-0027](../adr/ADR-0027-rust-core-two-ui-shells.md) open item 1).
- **Build.** All eight elements and their keyboard rules, suggestions, find and replace, drafts, compare, review, lock,
  revisions, numbering, page setup, custom elements, comments, and the focused undo of ADR-0010.
- Scenarios 40 to 43 of the shared list pass on the Mac.

**Exit.** The editor passes the shared screenplay scenarios and the typing-loss scenario (type, quit with Command+Q, reopen).

## 7. Phase M5: Story, Idea Vault and Breakdown

- Beat Board: acts, sequences, beats, cards, characters, timeline, episodes, Build Screenplay.
- Idea Vault: project and global, five item types, voice notes, drag in.
- Breakdown: suggestions, categories, catalog, locations, cast and crew.
- Each workspace passes its shared scenarios.

**Exit.** Parity matrix rows for these workspaces are proved on Intel and, when available, on Apple silicon.

## 8. Phase M6: production workspaces

- Schedule: stripboard, list, calendar, markers, timeline, warnings, Day Out of Days (drag and drop is the risk).
- Call sheets, sides, reports and budget.
- Visual planning: moodboards, storyboards with sketch, shot lists.
- Files, Recently Deleted, Activity, Notes and Tasks.

**Exit.** Parity rows proved on the same two levels.

## 9. Phase M7: import, export, packages and PDF

- Final Draft, Fountain, Word, PDF, plain text; backups, exchange, review and response packages, restore.
- PDF preview and printing with the deterministic layout engine in the core; Indian-script fonts.
- File dialogs with the App Sandbox rules in mind, even though the internal build is not sandboxed: choices made here must
  work later in Chapter 3.

**Exit.** Every format round-trips on the Mac with the same fixtures as Windows.

## 10. Phase M8: Mac AI

**Goal.** The same measured AI quality as Windows ([ADR-0028](../adr/ADR-0028-ai-quality-program-and-harness.md)).

- **Provider design.** Read Apple's App Sandbox and helper-process rules and decide whether the engine runs as a helper
  process or as a linked library ([ADR-0027](../adr/ADR-0027-rust-core-two-ui-shells.md) open item 2). Record the choice.
- **Engine.** Pin an Apple silicon `llama.cpp` build with Metal in the runtime lock; the same download flow, manifest and
  tiers as Windows ([ADR-0026](../adr/ADR-0026-offline-ai-download-after-install.md)).
- **Apple model provider.** An opt-in provider using Apple's on-device model, behind the `InferenceProvider` interface. The
  harness reads its capabilities (a window of about 4,096 tokens, text only, the Apple Intelligence language list) and
  shortens prompts to fit. The setting says plainly which languages it handles. The default stays the open model.
- **Mac screens** for the assistant, the download, the learned-memory screen, the AI health card and Export AI report.
- **Measure.** The evaluation harness runs on an Apple silicon Mac and on the Intel Mac (processor only) against the same
  gold sets. The English bar of 90 and 85 percent must be met by the default provider; the Apple provider is reported but is
  not the release default.

**Exit.** The bar is met on Apple silicon with the default provider; every figure is in the evidence folder.

## 11. Phase M9: Mac verification and the disk image

Needs the owner's word to build the `.dmg` ([ADR-0030](../adr/ADR-0030-verification-ladder-and-release-flow.md)).

- **Mac end-to-end suite.** Every scenario of the shared list is implemented in Apple's UI testing framework and run on the
  release build. This replaces the Windows driver, which does not support macOS.
- **Accessibility** with VoiceOver on the eight main workspaces; keyboard-only use; findings fixed or listed.
- **Performance** on the Apple silicon Mac against the same budgets as Windows.
- **Disk image.** Build the internal `.dmg`, unsigned and not notarized; document the Gatekeeper steps (right-click Open or
  remove the quarantine flag) as an installation restriction. Run the secret and personal-path scan on it.
- **Clean Apple silicon Mac.** Install, first run, create a project, type, quit and reopen, download the AI, use it offline,
  export, back up and restore, upgrade, delete the app.

**Exit.** Every scenario passes on the release build and on a clean Apple silicon Mac; evidence saved.

## 12. Phase M10: team pilot and close

- The team uses the `.dmg` for 2 to 3 days; bug reports, fixes, re-checks, as in W10. Non-English AI answers are checked.
- The parity matrix has no Windows Must-ship row without a Mac proof.
- **Chapter 1 closes** when W10 and M10 have both closed and the exit criteria in
  [chapter-1-complete-test-validate.md](chapter-1-complete-test-validate.md) hold. The implementation plans for
  Chapters 2 and 3 are written after that, not before.
