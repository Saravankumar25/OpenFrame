# 22 — QA & Acceptance Test Strategy

> **Status legend:** ✅ implemented · 🚧 in development · 📋 planned.
> Authority for *what* to test: FSD acceptance criteria and matrices (§68–70, §171), PRD acceptance criteria
> (§200–210), Security SEC-xxx. This document defines *how*. Contributor rules: CONTRIBUTING.md §4.

## 1. Test pyramid

| Level | Tooling | Location | Runs in | Status |
|---|---|---|---|---|
| Unit (pure logic, SQL helpers, parsers, security primitives) | `cargo test` `#[cfg(test)]` | each crate | CI `rust` job | ✅ domain, persistence, security, project-format, search |
| Application integration (ops through the real pipeline, real SQLite, temp dirs) | `openframe-test-support::TestEnv` | `crates/openframe-application/tests/*.rs` | CI `rust` job | ✅ `core_pipeline.rs`; 🚧 one file per module |
| Frontend unit/component | Vitest + Testing Library + jsdom | `apps/desktop/src/**/*.test.ts(x)` | CI `web` job | ✅ harness (`src/test/setup.ts`); 🚧 module tests |
| Golden files (deterministic exports, import parsing) | Rust tests comparing bytes or normalized text | module test dirs | CI `rust` job | 🚧 with import/export |
| End-to-end desktop | Playwright (WebView2 CDP) or WebdriverIO + `tauri-driver` against a built app | `tests/e2e/` | Release-candidate pipeline (Windows runner) | 📋 |
| Performance | criterion benches + manual traces | 21-performance-budgets.md | RC checklist | 📋 |
| Accessibility | axe-core in component tests + manual keyboard/screen-reader passes (Narrator, NVDA) | component tests + RC checklist | CI + RC | 📋 |

## 2. What every module must prove (application integration tests)

`TestEnv` provides an isolated `AppCore` (`AppConfig::isolated`), `env.ok(op, json)`, `env.err(op, json) -> code`,
`env.undo()`/`redo()`, `env.actor_with_role(Role)`, `env.restart()`, `env.crash_and_restart()` and
`env.write_file()`. For **every** op:

1. **Happy path**, with DTO shape asserted.
2. **Validation**: required fields, limits, unknown fields (`validation.*`), with human messages.
3. **Undo/redo** restores exact prior state, and redo re-applies.
4. **Delete → Recently Deleted → restore** (back to prior parent/position, or "Unassigned") → **purge**.
5. **Permissions**: Viewer and Commenter are denied mutations (`permission.denied`) and nothing changes.
6. **Search**: indexed content is found, deleted content is not, and private content is visible only to its owner.
7. **Persistence** across `restart()`, and **crash** via `crash_and_restart()` (recovery offered, data intact).
8. **Every FSD acceptance criterion** for the module, with the test named after the requirement
   (`fsd_story_004_drag_is_undoable`). Where FSD IDs collide between §68–70 and §171 (FSD Part 2 #1), use the §171 ID
   and mention the §68 alias in a comment.

Foundation coverage today ✅ (`core_pipeline.rs`): project create/open lands on an empty Home; validation creates
nothing; file add → search → undo/redo → trash; undo never overwrites later work; permissions checked before any
mutation; status manual and persistent across restart; crash recovery keeps the latest autosave; single writer;
duplicate creates a new identity; archive hides from recents; non-project folder fails safely; corrupt DB refused
unmodified.

## 3. Test data rules

- Fixtures live only in tests. Shipped code has no demo or fake data.
- Fixtures are synthetic or public domain. No real user projects, and no personal data.
- Tests never touch `%LOCALAPPDATA%\OpenFrame` or `Documents\OpenFrame` (hermetic temp dirs).
- Deterministic: no wall-clock dependence beyond ordering (use relative comparisons), and no network.

## 4. Traceability

- A requirement → test mapping is maintained by test naming (`fsd_<area>_<nnn>_…`, `sec_<nnn>_…`, `prd_<area>_…`).
  A 📋 script will extract test names into a traceability table per release (FSD ID → tests → pass/fail).
- Security controls map to tests in 20-security-threat-model.md §2.
- A requirement with no test is not "done" (Engineering Index §6).

## 5. Release gates (release candidate)

| Gate | Criterion | Automated |
|---|---|---|
| G1 Build hygiene | `cargo fmt --check`, `clippy -D warnings`, `tsc`, `eslint --max-warnings 0` | ✅ CI |
| G2 Tests | 100% of Rust + Vitest tests pass; no ignored tests without a linked issue | ✅ CI |
| G3 Bindings | No drift in `ipc/generated` | ✅ CI |
| G4 Supply chain | `cargo deny check` and `npm audit --omit=dev` clean (or documented exceptions); license inventory current, no copyleft/unknown | ✅ release.yml (blocking) |
| G5 Release prerequisites | `scripts/release-validate.mjs` passes (license selected, versions, signing, updater) | ✅ release.yml |
| G6 P0 acceptance | Every P0 FSD acceptance criterion has a passing automated test, or a signed-off manual E2E result | 📋 traceability script |
| G7 Migration | Projects from every previously shipped version open, migrate (backup created) and round-trip (23-release-migration.md) | 📋 fixture corpus |
| G8 Performance | P1–P6 within budget on minimum hardware (21-performance-budgets.md) | 📋 manual RC checklist |
| G9 Crash/recovery | Manual kill-during-typing test: reopen offers recovery, and no committed edit is lost | 📋 manual + E2E |
| G10 Accessibility | Keyboard-only walkthrough of every workspace; visible focus; labelled icon buttons; status not colour-only; usable in Windows High Contrast (ADR-0012 §7) | 📋 manual |
| G11 Installer | Signed NSIS and MSI install, upgrade over the previous version, and uninstall on clean Windows 11 VMs (per-user; with and without WebView2) | 📋 manual |
| G12 Offline | Full core workflow with the network disabled; only model download and update check show "unavailable offline" | 📋 manual |

## 6. Defect policy

- Data loss, corruption, permission bypass or a security issue is **P0**: it blocks release, and the fix needs a
  regression test.
- A flaky test is quarantined only with an issue and an owner, never silently retried.
