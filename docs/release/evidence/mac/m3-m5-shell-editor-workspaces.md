# Mac phases M3 to M5: evidence

Recorded 2026-10-18 on the development Mac (Apple M1, macOS 26.5.2, Swift 6.3.3 from the Command Line
Tools, no full Xcode). Commits on branch `mac/m1-toolchain`: 6471122, a3a32ee, 5e008e1.

## What "proved" means here

Every Mac row marked "Built; model-level tests pass on Apple silicon" in
[mac-parity-matrix.md](../../../roadmap/mac-parity-matrix.md) has Swift tests that start the real Rust core
on a temporary folder and drive the same view models the window uses, step by step through the shared
scenario files in `tests/e2e/scenarios/`. They are not window-level tests: nothing clicks the real window,
because the UI testing framework needs full Xcode. Those tests are phase M9 (ADR-0030 proof level 3 needs the
release build too). So these rows are at proof level 2 for the logic and the screens' models, not yet for
the clicks.

## Results

`bash apps/macos/scripts/test.sh`: **253 tests in 32 suites, all passing** (after the gap work: Shell lifecycle 18, Editor comfort 6) (suites run one after another;
see the script's note). Main suites:

| Suite | Tests | Covers |
|---|---|---|
| Core journey (OpenFrameCoreTests) | 5 | M2 interface from Swift |
| Shell scenarios | 5 | Home to an empty project and back, bad folder, empty title, team password gate (wrong then right), screens in light and dark |
| Settings | 5 | 16 themes persist, mode, display name, diagnostics, rendering |
| Theme engine | 11 | Every theme and mode resolves, matches an independent reference, AAA contrast for High Contrast |
| Screenplay model | 88 | Diff, sync, rules, keyboard, suggestions, find (ported from the Windows editor's tests) |
| Editor spike | 2 | TextKit 2 and web view typing latency ([numbers](m4-editor-spike.md)) |
| Editor scenarios | 19 | Scenarios 40, 41, 42; typing loss on quit and close; a second new screenplay starts clean |
| Editor panels | 17 | Scenarios 43, 44; drafts, compare, review, comments, scene hub, page setup |
| Idea Vault | 21 | Scenarios 20, 21; global vault, copy to project, voice notes (simulated microphone) |
| Story | 33 | Scenarios 30, 31; drag index rules, keyboard moves, Build Screenplay, 500 cards |
| Breakdown | 23 | Scenarios 50, 51; tagging from a selection, suggestions, catalog, locations, cast and crew |

The development app bundle (`apps/macos/scripts/bundle-dev-app.sh`) was started on a test folder
(`OPENFRAME_DATA_ROOT`): the core started (`"OpenFrame Studio starting"`, shell `swiftui`), created the app
database and the Global Idea Vault, and the app ended cleanly.

## Found and fixed

- After a document loaded, the editor's typing style still belonged to the previous document, so a new
  screenplay's empty last paragraph carried an old element id and the first save failed with
  `conflict.state`. Fixed in `ScreenplayTextView.load`; regression test
  `aSecondNewScreenplayStartsCleanAndSavesTyping`.
- The Story field autosave timer cancelled itself and the save never ran. Fixed by the Story work; test
  `typedTextIsSavedByTheTimerWithoutAnExplicitFlush`.

## Not proved yet

- Window-level clicks, drag and drop in a real window, VoiceOver and input methods driven by a person (M9).
- Spelling underlines in the running app (continuous spelling is on, but a headless test cannot see it).
- Rows marked "Partly built" or "Not started" in the matrix, each with its reason.
