# 22: QA & Acceptance Test Strategy

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
| End-to-end desktop | Playwright (WebView2 CDP) or WebdriverIO + `tauri-driver` against a built app | `tests/e2e/` | Release-candidate pipeline (Windows runner) | 🚧 `specs/01-journey`, `02-restart` (AI: "not installed" only) |
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
| Build hygiene gate | `cargo fmt --check`, `clippy -D warnings`, `tsc`, `eslint --max-warnings 0` | ✅ CI |
| Test pass gate | 100% of Rust + Vitest tests pass; no ignored tests without a linked issue | ✅ CI |
| Generated bindings gate | No drift in `ipc/generated` | ✅ CI |
| Supply chain gate | `cargo deny check` and `npm audit --omit=dev` clean (or documented exceptions); license inventory current, no copyleft/unknown | ✅ release.yml (blocking) |
| Release prerequisites gate | `scripts/release-validate.mjs` passes (end-user licence agreement in place and `LICENSE-PENDING.md` removed, versions, signing, updater) | ✅ release.yml |
| Must-ship acceptance gate | Every must-ship FSD acceptance criterion has a passing automated test, or a signed-off manual E2E result | 📋 traceability script |
| Migration gate | Projects from every previously shipped version open, migrate (backup created) and round-trip (23-release-migration.md) | 📋 fixture corpus |
| Performance gate | Cold start, project open, typing latency, search, idle CPU and RAM without AI within budget on minimum hardware (21-performance-budgets.md) | 📋 manual RC checklist |
| Crash and recovery gate | Manual kill-during-typing test: reopen offers recovery, and no committed edit is lost | 📋 manual + E2E |
| Accessibility gate | Keyboard-only walkthrough of every workspace; visible focus; labelled icon buttons; status not colour-only; usable in Windows High Contrast (ADR-0012 §7) | 📋 manual |
| Installer gate | Signed NSIS and MSI install, upgrade over the previous version, and uninstall on clean Windows 11 VMs (per-user; with and without WebView2) | 📋 manual |
| Offline gate | Full core workflow with the network disabled; only the update check shows "unavailable offline"; the bundled Offline AI (ADR-0025) answers, proposes and applies with the network disabled right after install | 📋 manual (the AI part is automated by `72-ai-assistant`: loopback-only sockets of the app and the engines) |
| Offline AI bundle gate | For a new bundle version (a change to `packaging/ai/ai-assets.lock.json`): `scripts/ai-benchmark.mjs` (tool calling, JSON validity, injection case, latency, memory), the ignored `real_runtime` and `ai_real_model` tests and the AI E2E specs pass on minimum and recommended hardware with the processor build (the only bundled build since ADR-0025; runtime spec §16) | 📋 manual |

## 6. AI test layers

Nothing is ever downloaded (ADR-0025): the runtime and both models ship inside every package, pinned in
`packaging/ai/ai-assets.lock.json`. Rust tests in CI use a deterministic scripted `ChatModel` and a deterministic
test embedder (`openframe_test_support`), never shipped. The real bundled components are exercised by the opt-in
Rust tests, the benchmark and the desktop E2E AI specs, which run the test build with the bundled model.

| Layer | What it proves | Location | Runs in |
|---|---|---|---|
| Runtime unit + integration | Bundle lock validation (unsafe paths, hashes, roles, embedded tokenizer, processor build only), first launch with no download or setup, hash check before the first engine start with the check record in app data, altered / missing / linked / replaced files, a new app version re-checks everything, engine files re-hashed before every launch, supervisor lifecycle, retired download clean-up, with the test executable standing in for `llama-server` | `crates/openframe-ai/src/**`, `tests/{bundled,supervisor,soak_sidecar,sidecar_log_redaction}.rs` | CI `rust` |
| Derived index | Index open/trust/rebuild on identity change or damage, vector module not global, graph traversal limits, RRF ranking | `crates/openframe-search/src/**` (unit) | CI `rust` |
| Retrieval | Documents/chunks/vectors/graph for every domain, private notes and contact details, hybrid vs keyword, graph expansion, incremental indexing incl. undo/redo/delete/restore, saves never wait, embedding failure fallback, router, restart/rebuild, index ops, project close | `crates/openframe-application/tests/ai_retrieval.rs` | CI `rust` |
| Retrieval budgets (§34) | FTS, vector, graph, hybrid + assembly and save latency on a generated feature-length fixture | `tests/ai_retrieval_perf.rs` | CI `rust` (3× budget in debug) |
| Toolbox | No tool reviews/applies Change Sets or runs `ai.*`; unknown tool / bad args; proposals never write; reads never mutate; role filtering; private notes; ambiguity; every proposal builds registry-valid operations; schema coverage; request-relevant tool subset | `tests/ai_toolbox.rs` | CI `rust` |
| Coverage matrix | Every registered operation has metadata; tools match metadata; strict bounded schemas; `ai-tool-coverage.md` regenerated from the Registry (fails on drift) | `tests/ai_tool_coverage.rs` | CI `rust` |
| Agent loop | Multi-step reads, inferred vs exact answers, terminal navigation, 8/12 step limits, repeated-call stop, model failure mid-run, bounded history, composite Change Set, stale recheck, human-only acceptance, destructive confirmation, injection in observations, SQL/oversized args, Viewer denial | `tests/ai_agent.rs` | CI `rust` |
| Assistant acceptance (AI-AC-xxx) | Works without AI, exact counts, no invented facts, navigation, suggestions create nothing, preview → accept → one undo step, permissions, stale/conflict, rename exclusions, all-or-nothing apply, allow-list, ambiguity, private notes, injection, malformed output, audit without context copies, personal history | `tests/ai.rs`, `tests/security_review.rs` (`ai_01_*`, `ai_02_*`) | CI `rust` |
| Frontend | Panel states (ready, off with the repair message, no download anywhere), proposal card actions, scope options, index notice | `apps/desktop/src/ai/*.test.ts(x)` | CI `web` |
| E2E (real bundled model, deterministic sampling) | Ready on first launch; exact and retrieved answers with sources; navigation; scope in every workspace; propose, apply, undo, redo, reject, stale; kill on apply (ADR-0019 all or nothing) and while the engine answers; missing and altered bundle files; loopback-only network use | `tests/e2e/specs/7{2..6}-ai-*.test.mjs`, journey step in `01-journey.test.mjs` | RC pipeline |
| Real components (manual / opt-in) | Full check of the staged bundle → chat (deterministic) → grammar-constrained JSON → embeddings → recorded check on the next start; the assistant through the operation registry (exact count, retrieval, propose, apply, undo, reject); model benchmark | `OPENFRAME_AI_BUNDLE_DIR=.build-cache/ai-bundle cargo test -p openframe-ai --test real_runtime -- --ignored`; the same variable with `-p openframe-application --test ai_real_model -- --ignored --test-threads=1`; `node scripts/ai-benchmark.mjs` | Offline AI bundle gate |

Not covered yet: a retrieval **quality** evaluation set (Recall@K / MRR, FTS vs vector vs hybrid vs hybrid+graph,
agentic spec §35); an E2E index rebuild with the real model; the web view's own network use in the no-egress check
(threat model T33; the AI E2E checks the app and the engines).

### Additional release gates (added 2026-10-05; see the build plan)

| Gate | Criterion | Automated |
|---|---|---|
| Desktop end-to-end gate | The desktop E2E suite (one spec per workspace plus the full lifecycle journey) passes on a Windows runner and is **required** for release (build plan: end-to-end tests in CI) | 📋 CI job |
| Retrieval quality gate | Recall@K and MRR meet owner-approved thresholds for FTS, vector, hybrid and hybrid+graph on the gold set (build plan: retrieval gold set, AI assistant area) | 📋 |
| Forced-kill gate | Killing the process at every Change Set operation boundary and during save, migration, export and package import leaves a consistent project (build plan: crash-safe AI changes, forced stop at every apply step, forced stop tests) | 📋 |
| Round-trip gate | The 13 FDX fixture classes and the Revision 1 §50 migration definition of done pass (build plan: Screenplay engine and file formats area) | 📋 |
| Rendering gate | PDF image-compare regression for screenplay and production documents passes (build plan: PDF image comparison, and golden or image regression for production PDFs) | 📋 |
| Packaged app gate | WACK passes and the installed MSIX passes the lifecycle journey and answers with the bundled Offline AI on a clean machine with the network off (build plan: Microsoft Store package area; ADR-0025 validation list) | 📋 manual + E2E |
| Reachability and feature status gate | Every feature status row has a UI route, tests and a status its evidence supports; reachability test green (build plan: feature status check in CI, reachability test) | 📋 |

## 7. Soak and failure injection (build plan: soak tests, failure injection)

### Soak tests

| Suite | What it proves | How |
|---|---|---|
| `crates/openframe-application/tests/soak.rs` | Concurrent IPC (4 writer and 2 reader threads on one `AppCore`), rapid explicit saves (every 50–150 ms), thousands of mutations, undo and redo during an index rebuild, close and reopen during indexing with writes racing the close, repeated AI cancel (`app.cancel_task` on `ai.index_rebuild`, assistant requests whose model call is cancelled or unavailable) | Real ops through `AppCore::dispatch`; the test embedder from `openframe-test-support`; a local `ChatModel` that always ends abnormally. Each writer keeps a model of what it wrote; phase 2 checks the content against the model after every undo and redo. End checks: `integrity_check`, `foreign_key_check`, undo history (states, 300 cap, parseable steps, undo/redo round trip), counts equal the models, index current and complete, no save error, every task ended, working set growth under 256 MB, clean close and restart without a recovery offer. Every phase runs under a watchdog and every op under a 30 s bound, so a deadlock fails instead of hanging. |
| `crates/openframe-ai/tests/soak_sidecar.rs` | Sidecar crash and restart loop with requests cancelled in flight; explicit stop and start; crash storm ends in Failed with a plain-language reason and an explicit start recovers | The test binary re-launched as a fake `llama-server` (as in `supervisor.rs`). Checks a new loopback port and key per launch, never Busy after a cancel, old ports closed, no fake server process left, working set growth under 64 MB. |

Profiles: `OPENFRAME_SOAK_SECS` sets the total time. The short profile runs in `cargo test` (60 s for
`soak.rs`, 20 s for the sidecar loop). The ignored long profiles default to 30 minutes:

```text
OPENFRAME_SOAK_SECS=1800 cargo test -p openframe-application --test soak -- --ignored --nocapture m1_08_soak_long
OPENFRAME_SOAK_SECS=1800 cargo test -p openframe-ai --test soak_sidecar -- --ignored --nocapture m1_08_sidecar_soak_long
```

Evidence: `docs/release/evidence/M1/M1.08-soak.txt` (evidence folders and file names keep their old item codes on purpose; see the evidence README).

### Failure injection

`crates/openframe-application/tests/failure_injection.rs`. Every
case expects a human error code and message, no partial file where the user would find it, no data loss, and a
project that still opens with every acknowledged change.

| Condition | Mechanism |
|---|---|
| Disk full at commit; device write failure | Debug-only `store::fault::fail_commits` (`SQLITE_FULL`, `SQLITE_IOERR_WRITE` through the normal error mapping); release builds compile it to nothing |
| Disk full during an export | Debug-only `modules::exports::fault::disk_full` (`ERROR_DISK_FULL`, 112) |
| Permission denied | `icacls /deny *S-1-1-0:(W)` on the export folder; read-only attribute on `project.sqlite` |
| Path disappears | Export folder deleted after it was chosen; project folder moved while closed; `recovery/` deleted while open |
| External volume removed | Project on a `subst` drive letter that is removed while open, then mapped again |
| Read-only destination | Read-only attribute on an existing export or backup file |
| Sleep and resume | Indexing: the embedder stalls or fails for the "sleep" (nothing is downloaded since ADR-0025, so the former download case is gone) |
| Antivirus-locked file | A handle opened with `FILE_SHARE_READ` only (or no sharing) on the export destination, the checkpoint, `project.sqlite` at open and an import source |
| Very long paths | Project, export and backup more than 260 characters deep |
| Non-ASCII project paths | `Résumé/Café/日本` folders, a non-ASCII title and a crash recovered in that folder |

Evidence: `docs/release/evidence/M1/M1.11-failure-injection.txt` (same naming note as above).

## 8. Defect policy

- Data loss, corruption, permission bypass or a security issue is a **release blocker**: it blocks release, and the fix needs a
  regression test.
- A flaky test is quarantined only with an issue and an owner, never silently retried.
