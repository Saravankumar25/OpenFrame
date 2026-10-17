# Developing OpenFrame Studio

This guide covers Windows 11 setup, day-to-day commands, the repository layout and the architecture you will work in.
Read [CONTRIBUTING.md](CONTRIBUTING.md) before opening a pull request.

## 1. Prerequisites (Windows 11)

| Tool | Version | Install (PowerShell) |
|---|---|---|
| Rust via rustup, MSVC host | stable ≥ 1.85 (workspace `rust-version`; edition 2024) | `winget install Rustlang.Rustup` then `rustup default stable-msvc` |
| Visual Studio Build Tools, "Desktop development with C++" (MSVC, Windows 10/11 SDK) | 2022 | `winget install Microsoft.VisualStudio.2022.BuildTools --override "--wait --passive --add Microsoft.VisualStudio.Workload.VCTools --includeRecommended"` |
| Microsoft Edge WebView2 Runtime | evergreen | Preinstalled on Windows 11. Otherwise: `winget install Microsoft.EdgeWebView2Runtime` |
| Node.js | ≥ 20 (LTS) | `winget install OpenJS.NodeJS.LTS` |
| Git | any recent | `winget install Git.Git` |

Open a **new** terminal after installing so `PATH` picks up `%USERPROFILE%\.cargo\bin`. If a terminal still
cannot find `cargo`, run this for the current session:

```powershell
$env:Path = "$env:USERPROFILE\.cargo\bin;$env:Path"
```

Then check everything and install the npm dependencies:

```powershell
npm run setup                          # checks the prerequisites above, prints exact fixes, then runs npm install
node scripts/setup.mjs --check-only    # check only
```

The Rust and TypeScript code also builds and tests on macOS/Linux. Today only the Windows desktop bundle is built. The Mac application is a native SwiftUI shell over the same Rust core, built in the Mac track after the Windows track closes (ADR-0027); the Mac App Store package follows in Chapter 3.

## 2. Commands

All commands run from the repository root.

| Command | What it does |
|---|---|
| `npm install` | Installs the npm workspace (`apps/desktop`) |
| `npm run dev` | `tauri dev`: starts Vite on `http://127.0.0.1:1420` and the Rust app with hot reload. The first build compiles all crates and takes several minutes. Stages the Offline AI engine when `.ai-assets` is present (without it the app shows its "Offline AI is missing" state). A development build has no model host, so it cannot download the models; use the test suite for that |
| `npm run build` | `tauri build`: optimized release build plus NSIS and MSI installers in `target/release/bundle/`, with the Offline AI engine; the models download from Hugging Face after install at the revisions pinned in `packaging/ai/ai-assets.lock.json` (ADR-0026, ADR-0032). No model host or manifest key is needed. Local builds are unsigned. Fails closed when the pinned engine files are missing or altered. Run it only when the owner says so. |
| `npm run ai:assets` | Verifies `.ai-assets` (or `OPENFRAME_AI_ASSETS_DIR`) against `packaging/ai/ai-assets.lock.json` and stages the engine in `.build-cache/ai-bundle` (`-- --check` verifies only). A git worktree uses the main checkout's `.ai-assets`. |
| `npm run bundle` | Same as `build`, restricted to `--bundles nsis,msi` |
| `npm run installer:internal` | Builds the unsigned per-user NSIS installer for the team into `release-out/` (git-ignored), with a `.sha256`, a `.json` build record and Rust and npm SBOMs, then runs the binary and string scans. Needs a clean tree (or `-- --allow-dirty`) and the team password in `OPENFRAME_TEAM_PASSWORD` or the Windows user variable of that name (8 characters or more). Never uses the test model host. Uninstall keeps projects and the vault; the optional "delete app data" box removes `%LOCALAPPDATA%\OpenFrame` only |
| `npm run test` | `cargo test --workspace`, then Vitest (`apps/desktop`) |
| `npm run test:rust` / `npm run test:web` | One side only |
| `npm run lint` | `cargo clippy --workspace --all-targets -- -D warnings`, then `eslint src --max-warnings 0` |
| `npm run typecheck` | `tsc --noEmit` for the desktop app |
| `npm run format` / `npm run format:check` | `cargo fmt --all` (check mode is what CI runs) |
| `npm run bindings` | Regenerates the TypeScript IPC types from Rust DTOs (see §4) |
| `npm run licenses` | Regenerates `THIRD_PARTY_LICENSES.md` from `cargo metadata` and `package-lock.json` |
| `npm run verify` | Runs what CI's blocking jobs run: format check, lint, typecheck, governance checks and tests. On a 16 GB machine set `CARGO_BUILD_JOBS=4` first and run one cargo build at a time |
| `npm run check:governance` | Architecture lint, migration policy, feature status check (with proof levels), reachability, writing standard, document links and app addresses, secret scan of the working tree and history, plan evidence, and their self-tests (no compile, seconds) |
| `npm run check:links:online` | The link check, plus a fetch of every app address marked `online` in `scripts/app-addresses.json`. Run before a build (ADR-0030); it uses the network |
| `npm run scan:installer -- <file or folder>` | Refuses built files that hold a secret, a personal path, the build computer's user name or the team password in plain text. Run on the unpacked release folder and on the installer (Windows phase W9) |
| `npm run e2e:setup` | Installs the desktop E2E drivers into the gitignored `tests/e2e/.drivers/`: `msedgedriver.exe` matching the installed WebView2 Runtime (downloaded from Microsoft) and `tauri-driver` (`cargo install --root`). No global installs. |
| `npm run test:e2e` | Desktop end-to-end suite: runs `e2e:setup`, verifies and stages the Offline AI engine, builds the real app with it and with the suite's local model host (`127.0.0.1:47811`) and test key compiled in, starts that host, installs the model set in each test data folder as a finished download would (`@e2e-ai-models: none` starts without), (`tauri build --debug --no-bundle`) and drives it with WebdriverIO through `tauri-driver` + Edge WebDriver (`tests/e2e/specs/`). Add `-- --no-build` to reuse the last build, `-- --only=05-` to run some specs, and `-- --release=gated` or `-- --release=open` to drive the hardened release executable (`tauri build --no-bundle`, never an installer) with or without the team password screen; specs marked `@e2e-debug-only` are listed and skipped there. The app runs isolated in `tests/e2e/.run/<time>/` (`OPENFRAME_DATA_ROOT`; the release build honours it for file locations only, ADR-0031); failure screenshots go to `tests/e2e/artifacts/`. Close other OpenFrame windows first, because the suite drives real windows and keyboard input. |
| `npm --workspace apps/desktop run build:web` | Type-checks and builds only the web bundle (`apps/desktop/dist`) |
| `node scripts/release-validate.mjs --report-only` | Shows which release prerequisites are still missing |

Faster inner loops:

```powershell
cargo test -p openframe-application                  # application layer only (does not compile Tauri)
cargo test -p openframe-application --test platform core_pipeline::
cd apps/desktop; npx vitest                          # watch mode
```

`cargo clippy`/`cargo check` on the `openframe-desktop` crate needs `apps/desktop/dist` to exist because
`tauri::generate_context!()` checks `frontendDist` at compile time. Run `build:web` once, or create an empty
`apps/desktop/dist/index.html`. CI uses the second option.

## 3. Runtime locations, logs and diagnostics

| What | Where |
|---|---|
| Application data (local profile, recent projects, settings) | `%LOCALAPPDATA%OpenFrame\app.sqlite` |
| Logs | `%LOCALAPPDATA%OpenFrame\logs\openframe.YYYY-MM-DD.log`: JSON lines, rotated daily, 14 files kept |
| New projects (default) | `Documents\OpenFrame\Projects\<Title>.openframe\` |
| Global Idea Vault | `Documents\OpenFrame\Global Idea Vault\` (`vault.sqlite` + `assets/`) |
| Offline AI engine (in the package, read-only) and models (downloaded after install, ADR-0026) | engine: `ai\` next to `openframe-desktop.exe` (installer, MSIX and development builds); models: `%LOCALAPPDATA%OpenFrame\models\<version>\`; writable AI records in `%LOCALAPPDATA%\OpenFrame\ai\` and `logs\` (Store builds: `LocalState\OpenFrame`, see doc 27). `ai\graphics.json` holds the Graphics card setting and what the card checks found; set `"forceIntegratedProbe": true` there by hand to check integrated graphics even when the memory rule says no (ADR-0029, never shown in the app) |

**Log level.** Set the `OPENFRAME_LOG` environment variable. It uses
[`tracing_subscriber::EnvFilter`](https://docs.rs/tracing-subscriber/latest/tracing_subscriber/filter/struct.EnvFilter.html)
syntax and defaults to `info`.

```powershell
$env:OPENFRAME_LOG = "debug"; npm run dev                                     # every operation with its timing
$env:OPENFRAME_LOG = "info,openframe_application=debug"; npm run dev
```

At `info`, the log records startup, failed operations (op name, error code, duration), operations slower than
250 ms, and storage failures. At `debug`, it also logs every operation. Logs must never contain project content
(see [docs/engineering/26-observability.md](docs/engineering/26-observability.md)).

**Dev and installed builds share these folders.** The paths come from the Windows user profile and there is no
override yet. Use test projects, not real work, when developing. Automated tests always use isolated temporary
directories (`AppConfig::isolated`).

## 4. IPC bindings (Rust → TypeScript)

Rust DTOs and argument structs derive `ts_rs::TS` with `#[ts(export)]`. `.cargo/config.toml` sets
`TS_RS_EXPORT_DIR=apps/desktop/src/ipc/generated`, so the generated `export_bindings_*` tests write one `.ts` file
per type:

```powershell
npm run bindings        # = cargo test --workspace export_bindings --quiet
```

Commit the regenerated files with the Rust change. CI fails if `cargo test` leaves a diff in
`apps/desktop/src/ipc/generated`. Never edit generated files by hand. ESLint ignores that folder.

## 5. Repository layout

```text
.
├─ apps/desktop/
│  ├─ src/                    React 19 + TypeScript UI (presentation and input only)
│  │  ├─ app/                 shell, Home, routes (workspace discovery by file convention), stores, toasts
│  │  ├─ design-system/       UI kit (Radix-based primitives + OpenFrame tokens)
│  │  ├─ ipc/                 client.ts (the single typed IPC boundary), query.ts, generated/ (ts-rs output)
│  │  ├─ workspaces/<dir>/    one directory per module workspace (index.tsx default export)
│  │  └─ styles/              openframe.css (mock-derived classes), app.css (Tailwind v4)
│  └─ src-tauri/              Tauri v2 adapter: of_invoke dispatcher, asset scope, logging, capabilities
├─ crates/
│  ├─ openframe-domain/       identities (UUIDv7), error taxonomy, roles/capabilities, canonical enums. No I/O.
│  ├─ openframe-persistence/  SQLite connection policy, migrations, trigger-based undo capture, row helpers
│  ├─ openframe-project-format/ .openframe layout, manifest, single-writer lock, crash marker, atomic writes
│  ├─ openframe-security/     path confinement, safe ZIP extraction, SHA-256, Ed25519 verify, log redaction
│  ├─ openframe-application/  AppCore, op registry, Store (mutation pipeline), modules/*, schema list, AI orchestration
│  ├─ openframe-search/       derived retrieval primitives: FTS5, sqlite-vec vectors, context graph
│  ├─ openframe-import-export/ FDX, Fountain, DOCX, PDF import, PDF layout and generation, import reports
│  ├─ openframe-ai/           Offline AI: engine checked against the pinned lock, model download (pinned Hugging Face files, resume, check, activation, import), llama.cpp engine supervision
│  └─ openframe-test-support/ TestEnv: isolated AppCore, op helpers, crash/restart simulation
├─ migrations/{app,global,project}/  embedded SQL migrations (one numbered file per owning module)
├─ docs/
│  ├─ adr/                    Architecture Decision Records
│  ├─ engineering/            implementation specs and contracts (11 to 39)
│  └─ archive/                history, including the old spec digests (index: docs/archive/README.md)
├─ scripts/                   setup.mjs, license-inventory.mjs, release-validate.mjs
├─ .github/workflows/         ci.yml, release.yml
├─ deny.toml                  cargo-deny policy (licenses, advisories, sources, bans)
├─ HTML_Mockups/              approved interface mock-ups (labels and CSS classes are reused)
└─ docs/specs/              authoritative product specifications (PRD, FSD, UX, Domain, AI, ...); docs/adr/README.md is the decision register
```

`openframe-search`, `openframe-import-export` and `openframe-ai` exist (corrected 2026-10-05; the earlier text said they
were not created yet). The ESD's `-media` crate was never created; media handling lives in
`openframe-application/src/modules/*`, as ESD §4 allows ("crates may be consolidated during early development").
The dependency direction rules still apply. `openframe-collaboration` will **not** exist: LAN collaboration was
removed (ADR-0007), and package exchange lives with import/export. A platform crate (`openframe-platform`) is
planned for the "Platform adapter layer for Windows and Mac" area of the build plan (ADR-0017, Accepted 2026-10-06). Check `[workspace.dependencies]` in `Cargo.toml` for
entries that no longer match a crate.

## 6. Architecture overview

```text
React (webview)                                 Rust (single process)
────────────────                                ────────────────────────────────────────────────────────────
useOp(op, args, tables[])  ──invoke──►  of_invoke(op, args)            apps/desktop/src-tauri/src/lib.rs
useCommand(op) / call(op)                 │  spawn_blocking, timing log
                                          ▼
                                 AppCore::dispatch(actor, op, args)    crates/openframe-application/src/core.rs
                                          │  actor = local profile + project role (Owner by default)
                                          ▼
                                 Registry::dispatch                    registry.rs: THE IPC allow-list
                                          │  unknown op → security.unknown_operation
                                          │  serde decode (deny_unknown_fields) → validation.invalid_input
                                          ▼
                                 module handler  fn(&AppCore, &Actor, Args) -> AppResult<R>
                               ┌──────────┴───────────┐
                       Query   ▼                      ▼  Command
          actor.require(View)                         Store::mutate(actor, MutationMeta, |tx| …)   store.rs
          store.read(|conn| …)                          1. actor.require(capability)   → permission.denied
          (reader connection,                           2. BEGIN IMMEDIATE (single writer connection)
           PRAGMA query_only)                           3. module logic: validation + SQL via tx.conn()
                                                        4. drain TEMP-trigger row images (undo capture)
                                                        5. search re-index of touched rows (FTS5)
                                                        6. activity entry (merged when coalescing)
                                                        7. undo entry (coalesced typing within 4 s)
                                                        8. COMMIT (WAL, synchronous=FULL); failure → rollback
                                                           + remove files created in this tx
                                                        9. delete purged files (only after commit)
                                                       10. emit DataChanged{tables, ids} + SaveState
                                          │
◄──── "of://event" ── DataChanged ────────┘  React invalidates exactly the queries whose `tables` intersect
```

Key properties:

- **One mutation path.** Nothing else opens a write transaction on a store. Undo/redo, activity, search indexing and
  UI invalidation therefore work the same for every module, and a module cannot forget them.
- **Queries never mutate.** They run on a separate read-only connection and never block the writer (WAL).
- **Events, not polling, keep the UI consistent.** Each `useOp` declares the tables it reads. `DataChanged` lists the
  tables a commit touched.
- **Long work** (imports, exports, downloads, index rebuilds) runs through `AppCore::spawn_task` with progress,
  cancellation and a typed result event.
- **Two stores share one pipeline.** The open project (`project.sqlite`) and the Global Idea Vault (`vault.sqlite`).
  `app.sqlite` holds only application state (profile, recents, settings), never project content.

Details: [ADR-0004](docs/adr/ADR-0004-single-ipc-dispatcher.md) (IPC),
[ADR-0005](docs/adr/ADR-0005-generic-undo-redo.md) (undo),
[13-command-architecture.md](docs/engineering/13-command-architecture.md),
[14-ipc-contract.md](docs/engineering/14-ipc-contract.md).

### Adding a module

Follow [CONTRIBUTING.md](CONTRIBUTING.md) and [docs/architecture/README.md](docs/architecture/README.md). In summary:

1. Add a migration file under `migrations/project/` (use the number already reserved in `schema.rs`).
2. Create `crates/openframe-application/src/modules/<m>/` with a `register(r)` function covering ops, indexers and
   trash handlers, and add one line to `modules/mod.rs`.
3. Add `apps/desktop/src/workspaces/<dir>/index.tsx`. It is discovered automatically.
4. Add tests in `crates/openframe-application/tests/<area>/<m>.rs` and a `mod <m>;` line in that area's `main.rs`, using `openframe_test_support::TestEnv`.
5. Run `npm run bindings`, commit the generated types, and run `npm run verify`.

## 7. Architecture decisions

| ADR | Decision |
|---|---|
| [0001](docs/adr/ADR-0001-tauri-react-rust.md) | Tauri v2 + React/TypeScript + Rust |
| [0002](docs/adr/ADR-0002-rusqlite-over-sqlx.md) | rusqlite (bundled SQLite, FTS5, online backup) instead of SQLx |
| [0003](docs/adr/ADR-0003-project-folder-format.md) | `.openframe` project folder + single-writer lock |
| [0004](docs/adr/ADR-0004-single-ipc-dispatcher.md) | Single allow-listed IPC dispatcher `of_invoke`; asset access resolved in Rust |
| [0005](docs/adr/ADR-0005-generic-undo-redo.md) | Generic trigger-based undo/redo with stale check |
| [0006](docs/adr/ADR-0006-local-ai-sidecar.md) | Local-AI-only managed `llama.cpp` sidecar + signed manifests |
| [0007](docs/adr/ADR-0007-lan-collaboration-removed.md) | LAN collaboration removed (product owner, 2026-09-30); collaboration is file-based through packages |
| [0008](docs/adr/ADR-0008-deterministic-pdf.md) | Deterministic PDF via `pdf-writer` and base-14 fonts |
| [0009](docs/adr/ADR-0009-permission-matrix.md) | Permission matrix decisions for cells the Security spec leaves open |
| [0010](docs/adr/ADR-0010-editor-focused-undo.md) | Editor-focused undo in the screenplay editor vs global undo |
| [0011](docs/adr/ADR-0011-uuidv7-identities.md) | UUIDv7 identities, no stored display numbers |
| [0012](docs/adr/ADR-0012-spec-conflict-resolutions.md) | Resolutions of conflicts between specifications |
| [0013](docs/adr/ADR-0013-single-local-model-hybrid-rag-agent.md) | Single local model, hybrid RAG, bounded agent, human-only Change Sets |
| 0014 to 0021 (0014, 0015, 0016 and 0020 are proposed; 0017, 0018, 0019 and 0021 are accepted) | [Appearance](docs/adr/ADR-0014-appearance-theme-system.md), [production numbering](docs/adr/ADR-0015-screenplay-production-numbering.md), [PDF fonts](docs/adr/ADR-0016-pdf-embedded-fonts-and-shaping.md), [cross-platform](docs/adr/ADR-0017-cross-platform-windows-and-macos.md), [distribution and licensing](docs/adr/ADR-0018-distribution-licensing-commerce.md), [crash-safe apply](docs/adr/ADR-0019-crash-safe-change-set-apply.md), [AI runtime packaging](docs/adr/ADR-0020-ai-runtime-packaging-per-channel.md), [diagnostics policy](docs/adr/ADR-0021-diagnostics-telemetry-analytics-policy.md) |

Documentation map and update rules: [docs/README.md](docs/README.md). Work plan: [docs/implementation/masterplan.md](docs/implementation/masterplan.md).

## 8. Troubleshooting

| Symptom | Fix |
|---|---|
| `link.exe not found` / `LNK1181` / missing `windows.h` | Install the VS Build Tools C++ workload (§1) and open a new terminal |
| `cargo` not recognized | Add `%USERPROFILE%\.cargo\bin` to `PATH` (§1) |
| `frontendDist … doesn't exist` when running clippy/check | Run `npm --workspace apps/desktop run build:web`, or create `apps/desktop/dist/index.html` |
| `Port 1420 is already in use` | Another `npm run dev` is running. Close it (Vite uses `strictPort`) |
| "This project is already open in another OpenFrame window" | Another OpenFrame process holds the project's `.lock`. The OS releases it when that process exits, even after a crash |
| "projects can't be edited directly from a network folder" | Working from UNC paths is refused by design (ADR-0003). Copy the project locally |
| Blank window in `npm run dev` | Check the WebView2 Runtime (§1) and the Vite output in the terminal |
| TypeScript errors about missing `ipc/generated/*` types | Run `npm run bindings` |
