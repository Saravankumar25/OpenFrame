# 21 — Performance & Resource Budgets

> **Status legend:** ✅ implemented/measured · 🚧 in development · 📋 planned.
> The budgets below are release requirements. The application-layer budgets are measured by an automated harness
> (`npm run perf`, §4) on a generated stress project; the WebView-level ones (P1, P3, P5, P6, P9) were measured on a
> release build on 2026-09-30 (§5). Verification on minimum hardware is still 📋 (release-candidate checklist).

## 1. Reference hardware and fixture

| | Minimum (budgets must hold) | Recommended |
|---|---|---|
| OS | Windows 11 23H2+, x64 | Windows 11 24H2 |
| CPU | 4 cores / 8 threads, ~2020 mobile class (e.g. Core i5-1135G7 / Ryzen 5 4500U) | 8+ cores |
| RAM | 8 GB | 16 GB+ |
| Disk | SSD (NVMe or SATA) | NVMe |
| Display | 1366×768 @ 100% up to 1920×1080 @ 150% | — |

**Reference project ("Feature-120"):** a 120-page feature screenplay (~110 scenes, ~4,500 elements), 3 drafts,
~150 scene cards in 3 acts / 12 sequences, 40 characters, a full breakdown (~1,200 elements, ~300 catalog items),
a 25-day schedule, 25 call sheets, 300 comments, 200 vault items and 500 managed media files (≈ 2 GB).

**Stress fixture ✅ (`openframe_test_support::stress`, `StressSpec::feature_120()`).** It is deliberately larger than
Feature-120 and is built **only through the operation registry**, using the same ops the UI calls. Search documents,
activity, undo history, assets on disk and derived production data are therefore real. It contains:

- a 180-scene / 6,139-element Fountain script, imported and copied into 5 drafts (with history points);
- 250 scene cards in 3 acts / 30 sequences;
- 1,500 Idea Vault items (1,200 notes + 300 generated PNGs);
- 200 catalog items + 40 cast entries, and 800 breakdown elements;
- 60 locations, 40 cast and 40 crew;
- 900 shots, and 40 storyboards / 400 panels;
- a 30-day schedule with every strip placed, and 30 call sheets.

The text is synthetic and deterministic (fixed-seed generator). The fixture takes about 21 s to build. The resulting
`project.sqlite` is about 29 MB; the largest parts are elements (4.7 MB), history-point snapshots (2.9 MB) and search
(2.9 MB). A 6,139-element script exports to a 251-page PDF, so the typing and rendering numbers below are for a
script about twice the length of a 120-page feature.

## 2. Budgets

| # | Metric | Budget | Measured how |
|---|---|---|---|
| P1 | **Cold start** (process launch → Home interactive, no project) | **< 3 s** (p95, minimum hardware, after OS boot cache warm-up excluded) | Timestamp at `run()` + `performance.now()` at first interactive paint of Home. Reported in the log at `info`. |
| P2 | **Project open** (Open → Project Home interactive) for Feature-120 | **< 1.5 s** p95. A search index rebuild after migration is excluded (shown as a background task). | `project.open` op duration + first Home render |
| P3 | **Typing latency** in the screenplay editor (keydown → glyph painted) | **< 16 ms** p95 (one frame at 60 Hz), with persistence off the input path | Chrome DevTools/Perfetto trace on WebView2; synthetic 100-wpm typing on scene 55 of Feature-120 |
| P4 | **Search** (`search.query`, 1–3 terms, project + global) | **< 150 ms** p95 end-to-end (IPC round trip included) | Op timing from the `of_invoke` debug log; bench harness |
| P5 | **Idle CPU** (project open, no user input, AI not running) | **≈ 0%**: < 0.5% of one core averaged over 60 s; no periodic work more frequent than every 15 s | Windows Performance Monitor / `typeperf` on `openframe-desktop` + `msedgewebview2` |
| P6 | **RAM without AI** (app process + WebView2 processes), Feature-120 open, after 10 min of use | **< 400 MB** private working set total | Task Manager "Memory (active private working set)", summed |
| P7 | Command latency (ordinary mutation: move card, edit field) | < 50 ms p95 op time; UI reflects the change within 100 ms | `of_invoke` timing (`slow operation` logged at > 250 ms) |
| P8 | Undo/redo step | < 100 ms p95 | op timing |
| P9 | Scroll / drag (Story Board 150 cards, stripboard 110 strips) | 60 fps, no frame > 50 ms | Performance trace |
| P10 | Installer size (NSIS, without AI) | < 25 MB | CI artifact size |
| P11 | Disk overhead | DB ≤ 2× raw text size + indexes; `backups/` bounded by the retention policy (17 §6) | Fixture inspection |
| P12 | Explicit Save (checkpoint + backup) Feature-120 | < 1 s | op timing |

Local AI is excluded from P5/P6. Its budgets (model load time, tokens/s, RAM per profile) are part of the model
evaluation gates in the Local AI Runtime spec §16.

## 3. Design choices that serve the budgets ✅

- No work at startup beyond opening `app.sqlite` and registering ops. The Global Idea Vault opens lazily, and the AI
  sidecar starts on demand.
- Workspaces are code-split (`React.lazy` per workspace directory).
- Event-driven invalidation (`staleTime: Infinity`) means no polling for data. **Known exception:** the status bar
  refetches `app.save_state` every 30 s as a safety net. This is cheap: it reads an in-memory snapshot. Save-state
  events re-render only the status bar, save banner and save dialog, never the Shell or the active workspace.
- Reads run on a separate read-only connection (WAL) and never wait for the writer. Every IPC call runs on a blocking
  worker, never on the UI thread.
- Screenplay typing is persisted asynchronously with coalescing (ADR-0010). Rendering never waits for the round trip.
- The Idea Vault grid and list use TanStack Virtual. The other long lists (Story Board with 250 cards, stripboard,
  shots, scene navigator) render every row, but build each context menu lazily, only when it opens (`MenuItems` in
  the design system accepts items or a function). Scene-navigator rows are memoised.
- Thumbnails live in `cache/` and are generated in background tasks.
- SQLite: 16 MB page cache, composite `(parent, position)` indexes, FTS5 prefix indexes (`2 3`).
- Release profile: `lto = "thin"`, `codegen-units = 1`, `opt-level = 3`, stripped debug info. Dev builds optimize
  dependencies (`opt-level = 2`) so debugging stays usable.

## 4. Measurement

1. ✅ **Application layer: `npm run perf`.** This runs `cargo test --profile perf -p openframe-application --test perf
   -- --ignored --nocapture --test-threads=1`. The `perf` cargo profile uses the release opt-level without LTO, so it
   builds in minutes.
   - `crates/openframe-application/tests/perf.rs` builds the stress fixture, then times the following through
     `AppCore::dispatch`: every workspace query, search, typing batches, ordinary mutations, undo/redo, autosave
     throughput, DB growth after 1,000 edits, explicit save, PDF export and backup.
   - Each call takes JSON in and returns JSON out with the result serialised: the IPC path minus the WebView2 hop.
   - The suite fails on any budget breach.
   - While the queries run, it records every SQL statement with `sqlite3_trace_v2`. It prints the 30 most expensive
     statements with their `EXPLAIN QUERY PLAN` and flags full-table scans.
   - Set `OPENFRAME_PERF_EXPORT=<dir>` to keep a copy of the generated project for UI profiling.
   - The non-ignored tests in the same file keep the fixture builder and the performance fixes covered by
     `cargo test`.
2. **WebView level (manual, per release candidate).** Open the exported project in a release build and drive it in
   one of two ways:
   - WebView2 remote debugging (`WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS=--remote-debugging-port=…`) with the CDP
     `Input`, `Profiler`, `Tracing` and `Performance.getMetrics` domains;
   - tauri-driver with the Event Timing API.

   Idle CPU and private working set come from `Get-Process` and `Win32_PerfFormattedData_PerfProc_Process`, summed
   over the app and its WebView2 process tree.
3. A release-candidate checklist (22-qa-test-strategy.md §5) records P1–P6 on minimum hardware. Any regression
   > 10% or budget breach blocks the release.
4. Slow operations (> 250 ms) are already logged at `info` with op name and duration, and must be reviewed before
   release (`%LOCALAPPDATA%\OpenFrame\logs`).

## 5. Measured results (2026-09-30)

Measured on a development machine (Core Ultra 5 125H, 16 GB, NVMe, Windows 11 build 26200). It is faster than the
minimum hardware, so these numbers are not yet the release gate.

### Application layer (`npm run perf`)

Values are p95 over 15 runs unless noted, before → after the fixes in §6.

| Budget | Measurement | Before | After | Budget |
|---|---|---:|---:|---:|
| P2 | `project.open` + `project.home` (close and reopen ×7) | 331 ms | 411 ms (p50 185 ms) | < 1.5 s ✅ |
| P4 | `search.query`, 10 terms × (project, project + global) | 9.7 ms | 3.0 ms | < 150 ms ✅ |
| P3 (Rust side) | `screenplay.apply_edits`, one coalesced keystroke batch | 9.1 ms | 5.4 ms | < 16 ms ✅ |
| — | `screenplay.document` refetch triggered by each save (863 KB) | 35.7 ms | 21.0 ms | off the input path |
| P7 | `story.update_card` | 3.1 ms | 2.8 ms | < 50 ms ✅ |
| P7 | `vault.update` | 4.3 ms | 3.2 ms | < 50 ms ✅ |
| P7 | `schedule.move_strip` | 24.1 ms | 35.4 ms | < 50 ms ✅ |
| P8 | `history.undo` | 7.3 ms | 7.6 ms | < 100 ms ✅ |
| P8 | `history.redo` | 10.6 ms | 6.6 ms | < 100 ms ✅ |
| P12 | `project.save` (checkpoint + backup of a 29 MB DB) | 351 ms | 410 ms | < 1 s ✅ |
| — | `vault.list`, all 1,500 items (1.05 MB) | 110.7 ms | 30.2 ms | < 150 ms ✅ |
| — | `vault.list` with a search | 103.6 ms | 12.5 ms | < 150 ms ✅ |
| — | `visual.scenes` | 16.3 ms | 10.2 ms | < 150 ms ✅ |
| — | `storyboard.list` | 7.8 ms | 11.1 ms | < 150 ms ✅ |
| — | Every other workspace query ¹ | ≤ 41 ms | ≤ 35 ms | < 150 ms ✅ |
| — | `screenplay.export` to PDF (251 pages) | 39 ms | 42 ms | — |
| — | `packages.create_backup` (6.5 MB archive) | 1.67 s | 1.58 s | background task |
| — | Autosave throughput (edits that never coalesce) | 264 commits/s | 288 commits/s | — |
| — | DB growth after 1,000 edits | +0.0 MB | +0.0 MB | — |

¹ `story.board`, `screenplay.document`, `screenplay.overview`, `screenplay.characters`, `breakdown.scenes`,
`breakdown.scene`, `catalog.list`, `locations.list`, `cast.list`, `crew.list`, `shot.list`, `storyboard.get`,
`schedule.get`, `callsheets.list`, `callsheets.get`, `history.activity`, `trash.list`.

Notes:

- Differences of a few milliseconds between runs are noise: another build ran on the machine at the same time.
- DB growth stays flat because the undo history is capped at 300 entries and typing activity is merged.
- `history.info` is refetched after every change. It was already fast as a query (0.1 ms), but the same `sys_undo`
  scans cost about 1.1 ms each inside every mutation. They are now index lookups.

### Release build

Measured on `npm run build`, installed per user with the NSIS installer.

| Budget | Measurement | Result | Budget |
|---|---|---:|---:|
| P1 | Launch → Home interactive, warm (5 runs) | 0.58–0.83 s | < 3 s ✅ |
| P1 | First launch right after install (creates the WebView2 profile) | window 1.6 s, renderer 3.2 s | cold; not the gate |
| P2 | Open the stress project (IPC `project.open`), then Project Home rendered | 155 ms + 43 ms | < 1.5 s ✅ |
| P3 | Typing at ~100 wpm in scene 55 of the 180-scene script, keydown → next paint (Event Timing) ² | p50 32 ms, p95 40 ms | < 16 ms ❌ |
| P5 | Idle, stress project open on Project Home ³ | 0.57 % of one core | < 0.5 % ⚠️ |
| P5 | Idle, no project, 60 s measured after a 60 s settle | 0.99 % (WebView2 browser process 0.78 %) | < 0.5 % ⚠️ |
| P6 | Private working set, app + WebView2 tree, stress project open, every workspace visited ⁴ | 313 MB | < 400 MB ✅ |
| P9 | Workspace opened → rendered: Story Board (250 cards) | 0.45–0.52 s | — |
| P9 | Workspace opened → rendered: Idea Vault | 0.43–0.47 s | — |
| P9 | Workspace opened → rendered: Screenplay (6,319 blocks) | 0.53–0.75 s | — |
| P9 | Workspace opened → rendered: stripboard | 0.53–0.69 s | — |
| P10 | NSIS installer | 9.61 MiB | < 25 MB ✅ |
| — | MSI | 13.61 MiB | — |
| — | `openframe-desktop.exe` | 36.2 MB | — |

² Baseline: a plain `<input>` in the same harness gives p50 16 ms.
³ 60 s measured after a 30 s settle. By process: app 0.03 %, renderer 0.05 %, WebView2 browser process 0.31 %,
GPU process 0.03 %.
⁴ By process: GPU process 155 MB, renderer 70 MB, app 42 MB.

**AI runtime.** No `llama*` process and no AI service exist unless an `ai.*` operation runs.
`modules::ai::service` is created on first use, and the sidecar starts only on `ai.ask` or warm-up.

**P5.** App code is idle: the app process and the renderer together use under 0.1 % of one core. The remainder is
WebView2's own browser process, which was still settling at the 60 s mark.

**P3 is not met for a 6,000-element script.** A CDP trace of 71 keystrokes shows JavaScript at 3–5 ms per keystroke
(ProseMirror transaction, plugins and React). The rest is Blink work that scales with the number of blocks in the
single contenteditable root:

- the native typing command: about 12 ms, including a forced layout;
- layout: about 4.5 ms;
- paint and commit: about 8 ms;
- hit-testing after layout: about 3 ms.

Three things were tried and not kept:

- `content-visibility: auto` on each block made typing slower (layout rose to 8.6 ms).
- `contain: layout paint style` saved only about 1 ms.
- Applying typed text as a ProseMirror transaction skips the native typing command, but the caret scroll then forces
  the same layout, so the net effect was neutral.

Meeting P3 at this size needs a structural change: render only the scenes near the viewport, or use one editor per
scene, so that the editable root holds a few hundred blocks. 📋 This is tracked as a release blocker for long scripts.

## 6. Performance fixes (2026-09-30)

These were found by the harness (SQL profile and `EXPLAIN QUERY PLAN`) and by a review of the WebView code.
Behaviour is unchanged, and every fix has a regression test (`tests/perf.rs`, `design-system/menu.test.tsx`,
`editor.test.ts`).

- **Search**
  - The query now uses `ORDER BY rank` with the same `bm25(4, 1)` weights. FTS5 then sorts internally, so
    `snippet()` runs only for the returned rows. `snippet()` reads each document's text back from `search_doc`; over
    the run, these content fetches dropped from 16,832 to 3,288.
  - The entity-type filter now runs in SQL before the `LIMIT`. Before, a type-restricted search took `limit × 3` hits
    of all types and could return none of the requested type.
- **Idea Vault list**
  - Assets are loaded with one bulk query instead of one query per item.
  - File availability is checked with a single `GetFileAttributesW` call (`util::file_exists`). `std::fs::metadata`,
    used before, opens a file handle, which wakes on-access antivirus scanning and costs about 0.1 ms per file.
  - A vault search loads only the matching items.
- **Visual planning lists.** `visual.scenes` and `storyboard.list` compute scene fingerprints in bulk: two queries
  per draft instead of two per scene.
- **Undo bookkeeping**
  - New index `idx_undo_actor_state` on `sys_undo(actor_id, state, seq)` (project migration 10, global migration 3).
    Five `sys_undo` statements run on every mutation, every undo and, through `history.info`, after every change.
    Each was a full scan that also read every row's large `changes_json` overflow pages.
  - Undo and redo no longer rewrite the `id` column when restoring a row. Writing it made SQLite check every foreign
    key that references the table, which scans child tables that have no index on the referencing column.
- **Frontend**
  - The Shell no longer subscribes to save-state events. Before, each save re-rendered the whole active workspace
    twice.
  - The screenplay view updates cursor and outline state only when they actually change.
  - Scene-navigator rows are memoised.
  - Context menus of scene rows, story cards and sequences, strips and shots are built only when they open.
  - The block-id pass and the comment-decoration pass no longer walk all 6,000 blocks on every keystroke.
  - The current scene is found by walking back from the cursor instead of scanning the whole script.
  - Catalog search is debounced (150 ms) and keeps the previous rows while loading.
  - Shots are grouped with a map instead of an O(shots × scenes) search.

### Remaining observations (not changed)

- `schedule.move_strip` re-derives the whole board to refresh call-sheet fingerprints: 18–35 ms, within P7.
- `cast.list` returns 461 KB for 40 members because each member carries its scene list.
- Automatic screenplay history points store a full snapshot, about 600 KB for this script, every 5 minutes of
  editing, and 60 are kept per draft. That is up to about 35 MB for one long, actively edited draft.
- `sys_activity` is never trimmed. Each non-typing command adds about 190 bytes.
- The `screenplay.document` refetch after each save (21 ms in Rust, 863 KB over IPC) could be skipped when the change
  came from the editor itself.
