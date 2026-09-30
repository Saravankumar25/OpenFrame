# 21 — Performance & Resource Budgets

> **Status legend:** ✅ implemented/measured · 🚧 in development · 📋 planned.
> The budgets below are **defined** here and are release requirements. Automated measurement is 📋 planned
> (§4). No budget has been verified by an automated harness yet.

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
a 25-day schedule, 25 call sheets, 300 comments, 200 vault items and 500 managed media files (≈ 2 GB). The fixture
generator is 📋 planned (`openframe-test-support`), built only from public-domain or synthetic text.

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
  refetches `app.save_state` every 15 s as a safety net (P5 allows this; revisit if idle traces show a cost).
- Reads run on a separate read-only connection (WAL) and never wait for the writer. Every IPC call runs on a blocking
  worker, never on the UI thread.
- Screenplay typing is persisted asynchronously with coalescing (ADR-0010). Rendering never waits for the round trip.
- Large lists use TanStack Virtual. Thumbnails live in `cache/` and are generated in background tasks.
- SQLite: 16 MB page cache, composite `(parent, position)` indexes, FTS5 prefix indexes (`2 3`).
- Release profile: `lto = "thin"`, `codegen-units = 1`, `opt-level = 3`, stripped debug info. Dev builds optimize
  dependencies (`opt-level = 2`) so debugging stays usable.

## 4. Measurement plan 📋

1. `openframe-test-support`: a Feature-120 fixture generator plus a `benches/` (criterion) suite for `project.open`,
   `search.query`, typical commands and undo on the application layer (headless, deterministic).
2. A WebView-level trace script for P3/P9 using Playwright against `tauri dev` with WebView2 remote debugging, run
   manually before each release candidate.
3. A release-candidate checklist (22-qa-test-strategy.md §5) records P1–P6 on minimum hardware. Any regression
   > 10% or budget breach blocks the release.
4. Slow operations (> 250 ms) are already logged at `info` with op name and duration, and must be reviewed before
   release (`%LOCALAPPDATA%\OpenFrame\logs`).
