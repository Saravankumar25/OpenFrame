# Testing strategy

> Written 2026-10-09 from reading the test code, the CI files and the evidence. It explains which kind of test proves what,
> what each costs, what to run when, and what the tests do not prove. The older [QA strategy](../engineering/22-qa-test-strategy.md)
> stays as the detailed reference. Test numbers below are from the project logs unless a line says "counted".

## 1. What a green test run does and does not mean

Thousands of tests pass on the development machine. That proves the code does what the tests say, in a debug build, on one
Windows computer, with a fixed fake AI setting. It does not prove that:

- the installed release program behaves the same (the end-to-end suite drives a debug build);
- the program works on a clean computer, with another folder layout, another antivirus, OneDrive or a slow disk;
- a real writer can finish a real task;
- the Mac build works at all (no test has ever run on a Mac);
- the AI is useful (its quality has no measurement yet).

Passing tests are necessary. They are not "production ready".

## 2. The layers

| Layer | What it proves | Where | Cost | Run |
|---|---|---|---|---|
| Rust unit tests | One function or small module behaves correctly | `#[cfg(test)]` inside `crates/*/src` | Seconds per crate | While coding, for the crate touched |
| Rust integration tests | A whole operation through the registry, database and undo: save, restart, permissions, failure cases | `crates/*/tests/*.rs` (46 files in the application crate) | Minutes; each file links the whole crate | Targeted by file while coding; all before merging |
| Migration fixtures | Every old schema version upgrades and keeps content | `tests/migration_fixtures.rs` | Minutes | When a migration changes |
| Forced-kill and soak | The program survives being killed during save, migration, export, package import | `forced_kill.rs`, `soak.rs`, `soak_sidecar.rs` | Minutes to hours; timing sensitive | Before a release candidate |
| Hostile input corpus | Bad Final Draft, PDF, DOCX, ZIP, theme and package files are refused safely | `hostile_corpus.rs`, `security_parsers.rs` | Minutes | When a parser changes |
| Vitest | React components and editor logic | `apps/desktop/src/**/*.test.ts(x)` (61 files, 471 cases counted) | Under a minute | While coding screens |
| Governance scripts | Architecture rules, migration policy, feature status, links, writing standard | `npm run check:governance` | Seconds, no compile | Before every commit |
| Real runtime and real model | The bundled llama.cpp and Gemma behave | `real_runtime.rs`, `ai_real_model.rs` (ignored by default) | Minutes of CPU | After AI changes |
| End-to-end (E2E) | The real app, driven like a user, on a debug build | `tests/e2e/specs/` (30 files, 157 cases counted, about 20 minutes) | Highest: builds the app, drives real windows | Once at the end of a set of areas |
| Performance | Typing, open and filters inside budgets | `npm run perf` | Minutes, needs a release-like build | Before a release candidate |
| Visual baseline | The 15 main screens look as approved | `90-visual-baseline.test.mjs` | Part of E2E | Part of E2E |
| Installer and clean machine | Install, start, upgrade, uninstall on a clean computer | Manual until a script exists | An hour per machine | Every internal build |
| Tester pilot | Real people finish real work | [internal-test-plan.md](internal-test-plan.md) | Days | Once per internal build candidate |
| Cloud-synced folder | Ten minutes of typing in a OneDrive folder loses nothing and locks nothing | [onedrive-typing-test.md](onedrive-typing-test.md) | 20 minutes by hand | Once per internal build candidate |

## 3. Which tests are valuable, redundant or weak

Counted and read on 2026-10-09:

- **Valuable and expensive to replace:** migration fixtures, forced-kill, the hostile corpus, apply-recovery, package
  round trips, the E2E journey and restart specs.
- **Valuable, partly redundant:** the 46 integration files each rebuild the same large crate into a separate program.
  The coverage is good; the structure costs about 6 GB of programs and 18 GB of debug files. Joining them into a few
  binaries per crate keeps the coverage and removes most of the cost (build plan item already open).
- **Weak:** several rows are "Verified" by tests that run on the development machine only. The visual baseline compares the
  app with its own earlier screenshots, so it detects change, not correctness. Accessibility has almost no automated test.
- **Missing:** a release-build smoke test, a gated-build (team password) test, a window-close test, a test on a folder
  that OneDrive syncs, project delete and locate tests, a retrieval quality measurement, any Mac test.
- **Ignored on purpose:** ten Rust tests are marked ignored (real model, soak, performance, hostile corpus extras). They
  are run by name, not by default.

## 4. Running the right amount

The aim: a small change must not rebuild and retest everything.

1. While coding: run the tests of the crate and file you touched, through the build queue
   (`node scripts/cargo-queued.mjs test -p openframe-application --test <file>`), and Vitest for the screen.
2. Before committing: `npm run check:governance` (seconds) and `cargo fmt --check`.
3. Before merging a branch: format, clippy, all Rust tests, typecheck, lint, Vitest, governance. One run, not several.
4. After a set of areas: one full E2E run on the merged branch, then tick plan boxes with evidence.
5. Before an internal build: performance run, soak run, the clean-machine checklist.

Keep the queue (one heavy build at a time, 4 jobs) and the shared sccache. Do not run E2E per branch. Details and the
reasons are in [development-storage-policy.md](../development-storage-policy.md).

## 5. Test data

- The invented demo project "Night Bus" is created by `01-journey.test.mjs` and reused by the restart, visual and
  reachability specs.
- Four OpenFrame-owned sample scripts live in `templates/samples/` and are compiled into the app.
- The 13-class Final Draft fixture suite is in `tests/fixtures/fdx-suite/`.
- Never use a real person's script in tests or screenshots.

## 6. The Mac

The Mac shell is SwiftUI ([ADR-0027](../adr/ADR-0027-rust-core-two-ui-shells.md)), so the Windows driver (`tauri-driver`) is
not used there. The Mac suite is written in Apple's UI testing framework against the shared scenario list in
`tests/e2e/scenarios/`, plus Rust core tests and Swift package tests, plus a written manual checklist for the team pilot.
Each Mac row records two levels of evidence, "proved on Intel" and "proved on Apple silicon", and stays unverified until
both exist ([Mac phase M9](../roadmap/chapter-1b-mac-phases.md)).

## 7. Evidence rules

A box or a feature row moves up only with evidence: a test name, an end-to-end spec, a report file under
`docs/release/evidence/`. A claim copied from a log without a check is not evidence. "Not run" is written when a thing was
not run.

## 8. Flaky tests

A test that fails once under load is first read, not re-run until green. In the logs: `forced_kill` timing under load, the
sidecar soak that counted processes machine-wide, specs that assumed a fixed window state. Each was fixed in the test or in
the product. No test is skipped or loosened to get a green run.
