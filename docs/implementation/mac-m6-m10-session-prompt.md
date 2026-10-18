# Session prompt: Mac phases M6 to M10, release readiness and the internal .dmg

> Written 2026-10-18 at the end of the Mac M1 to M5 session, at the owner's request ("after M1 to M5 finishes, give
> a handoff prompt to finish M6 to M10, make them release ready and give the .dmg"). Paste everything below the line
> into a new Claude Code session opened in this repository on the Apple silicon Mac.

---

You are continuing the Mac track of OpenFrame Studio on branch `mac/m1-toolchain` (local only; never push or merge
without my word in that session). Phases M1 to M5 are built and tested (253 Swift tests and the Rust core tests pass on Apple silicon; see docs/release/evidence/mac/). Your job: finish phases M6 to M10 of
`docs/roadmap/chapter-1b-mac-phases.md`, make the Mac app release ready by the same bar as the Windows internal build,
and produce the internal `.dmg`. I, the owner, authorise building the internal `.dmg` in phase M9 of this work once
its gate checks pass (this is the owner's word that the plan requires; it covers the internal unsigned disk image
only, not any store package, signing, notarisation or upload).

## Start exactly like this

1. Read, in this order and nothing more: `CLAUDE.md`, `docs/implementation/handoff.md`,
   `docs/roadmap/chapter-1b-mac-phases.md` (sections 8 to 12), `docs/roadmap/mac-parity-matrix.md`,
   `docs/release/evidence/mac/` (all three files), `docs/release/windows-internal-release-gate.md` (how the Windows
   build was proved; the Mac `.dmg` needs its own gate built the same way).
2. Check the machine: `sw_vers`, `uname -m`, `xcodebuild -version` (full Xcode is REQUIRED for M9: the UI testing
   framework, `xcodebuild`, the app bundle build. If it is still missing, stop and ask me to install it; do not work
   around it for M9), `rustc -vV` (1.98.1, host aarch64-apple-darwin), `df -h ~` (keep at least 15 GB free; the Mac has
   about 40 GB free).
3. Build and test what exists before changing anything:
   `RUSTC_WRAPPER= cargo test --workspace --exclude openframe-desktop`, `apps/macos/scripts/build-core.sh`,
   `apps/macos/scripts/test.sh` (all must pass; the Swift suites run serially on purpose).
4. Say which phase and work item you start, which files it touches, and how it will be verified. Keep the task list
   in the app.

## How the Mac app is built (do not redesign it)

- One Rust core, unchanged rules. `crates/openframe-ffi` exposes it through UniFFI: `invoke(op, json)`, events, a few
  platform requests. Generated Swift types and typed operations: `apps/macos/Sources/OpenFrameCore/Generated/`
  (`npm run mac:types` after any DTO or operation change; CI fails on drift).
- SwiftUI shell in `apps/macos` (Swift package; OpenFrameCore, OpenFrameUI, OpenFrameStudio targets). No rules in Swift:
  every read and change is a core operation; screens reload on `dataChanged` events through `app.changes.key([...])`.
- Patterns to copy: `Home/`, `Story/`, `IdeaVault/`, `Breakdown/`, `Screenplay/` (editor on TextKit 2, sync in
  `Screenplay/Model/ScreenplaySync.swift`). Theme colours via `theme["token"]` (16 themes from the shared themes.json).
- Tests: Swift Testing with a real core on a temporary folder (`Fixture` in `Tests/OpenFrameUITests/ShellScenarioTests.swift`);
  each shared scenario in `tests/e2e/scenarios/` is implemented as a model-level test, and in M9 also as a window-level
  UI test.
- Never change Windows-only code (the Tauri adapter); Mac-only fixes in shared crates go behind `cfg(unix)` or into the
  Mac crate, so the Windows build evidence stays valid.
- Use Sonnet subagents for mechanical coding and documentation, with a file-ownership map and at most three at a time
  (they share one SwiftPM build folder); keep design, integration, debugging and decisions yourself. Load the
  `apple-design` skill for UI work and respect the existing design tokens.

## Work, in order

**0. Close the leftovers of M3 to M5** that the parity matrix marks "Partly built" or "Not started" with an M3 to M5
phase, and confirm with me the provisional TextKit 2 editor choice (ADR-0027 open item 1) if I have not yet.

**M6, production workspaces** (phase page section 8): Schedule (stripboard, list, calendar, markers, timeline,
warnings, Day Out of Days; drag and drop is the risk: use native drag with keyboard alternatives), call sheets, sides,
reports, budget, visual planning (moodboards, storyboards with a sketch tool on PencilKit-free AppKit drawing or Canvas,
shot lists), Files, Recently Deleted, Activity, Notes and Tasks. Shared scenarios 25, 61, 62, 80, 85, 86. Remove the
"not on the Mac yet" placeholders as each lands.

**M7, import, export, packages and PDF** (section 9): Final Draft, Fountain, Word, PDF, plain text; backups, exchange,
review and response packages, restore; PDF preview (PDFKit showing the core's deterministic PDF) and printing
(NSPrintOperation of that PDF); Indian-script fonts; file dialogs written so they still work under the App Sandbox
later (security-scoped bookmarks where a file is kept). Scenarios 63 and 70; every format round-trips with the Windows
fixtures.

**M8, Mac AI** (section 10, ADR-0026, 0028, 0032): decide helper process versus linked library after reading Apple's
App Sandbox rules (ADR-0027 open item 2: present the trade-off and ask me before building); pin the Apple silicon
`llama.cpp` build with Metal in `packaging/ai/ai-assets.lock.json` (official ggml-org release, SHA-256 of the archive
and every file; then update the AI diagnostics test that counts the engine); same download flow and tiers; the Mac
assistant, download, learned memory, AI health card and AI report screens; the optional Apple on-device model
provider behind `InferenceProvider` (off by default). Run the evaluation harness on this Mac; the English bar (90
percent correct tool, 85 percent correct end state) must be met by the default provider. Scenarios 72 to 78. The AI
rules of ADR-0028 are permanent: measure first, only deterministic code judges answers while people use the app.

**M9, verification and the disk image** (section 11): every shared scenario as a UI test (XCUITest) against the
release build; VoiceOver and keyboard-only passes on the eight main workspaces; performance against the Windows
budgets; security review (`security-guidance`, `ecc:security-review`) of the FFI boundary, file paths, packages,
process control and the AI engine; hardened runtime settings and an entitlements file ready for the sandboxed store
build; `PrivacyInfo.xcprivacy`. Then a Mac internal release gate document modelled on
`docs/release/windows-internal-release-gate.md`, and the **internal `.dmg`**: a release build with the team password
(`OPENFRAME_TEAM_PASSWORD` set on this Mac by me; never write it in a file or a log; at least 8 characters, ADR-0030),
unsigned and not notarised, created with `hdiutil` from the app bundle, named
`OpenFrame-Studio-<version>-internal-<commit>.dmg` in `release-out/`, with its SHA-256, size, the secret and
personal-path scan, an SBOM, and the Gatekeeper steps for the team (Control-click Open, or remove the quarantine flag)
written into the pilot kit. Prove it on a clean Apple silicon Mac user account (install, first run, team password,
create a project, type, quit and reopen, download the AI, use it offline, export, back up and restore, upgrade, delete
the app). Record every result as evidence under `docs/release/evidence/mac/`.

**M10, team pilot and close** (section 12): prepare the Mac pilot kit next to the Windows one
(`docs/testing/pilot-kit/`), and the triage steps. The pilot itself needs the team; when it has run, fix and re-check
findings, then mark the parity matrix complete (no Windows Must-ship row without a Mac proof). Chapter 1 closes only
when W10 and M10 have both closed.

## Rules that do not change

- Commit locally per finished, verified slice, with a plain-prose message and the Co-Authored-By line; never push,
  open a pull request or merge without my word in that session.
- No em dashes and no AI-sounding writing anywhere (`npm run check:writing`, which also covers Swift now).
- Clean as you go: remove placeholders and dead code in the same change; update the parity matrix, evidence and docs in
  the same change.
- Never decide an open item in `owner-decisions.md` alone; ask me in very simple language. The two that come up here:
  the AI engine form (M8) and anything about signing or the stores (Chapter 3, out of scope).
- Show progress as a table of phase, title, bar out of 100 and percent, at each milestone.
- At the end: rewrite `docs/implementation/handoff.md`, add a progress-log entry, run `graphify update .`, commit, and
  report plainly what was proved, what was not run, and what waits on me.
