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

The Rust and TypeScript code also builds and tests on macOS/Linux. The desktop bundle is Windows-only in v1.

## 2. Commands

All commands run from the repository root.

| Command | What it does |
|---|---|
| `npm install` | Installs the npm workspace (`apps/desktop`) |
| `npm run dev` | `tauri dev`: starts Vite on `http://127.0.0.1:1420` and the Rust app with hot reload. The first build compiles all crates and takes several minutes. |
| `npm run build` | `tauri build`: optimized release build plus NSIS and MSI installers in `target/release/bundle/`. Local builds are unsigned. |
| `npm run bundle` | Same as `build`, restricted to `--bundles nsis,msi` |
| `npm run test` | `cargo test --workspace`, then Vitest (`apps/desktop`) |
| `npm run test:rust` / `npm run test:web` | One side only |
| `npm run lint` | `cargo clippy --workspace --all-targets -- -D warnings`, then `eslint src --max-warnings 0` |
| `npm run typecheck` | `tsc --noEmit` for the desktop app |
| `npm run format` / `npm run format:check` | `cargo fmt --all` (check mode is what CI runs) |
| `npm run bindings` | Regenerates the TypeScript IPC types from Rust DTOs (see §4) |
| `npm run licenses` | Regenerates `THIRD_PARTY_LICENSES.md` from `cargo metadata` and `package-lock.json` |
| `npm run verify` | Runs everything CI's blocking jobs run: format check, lint, typecheck and tests |
| `npm run e2e:setup` | Installs the desktop E2E drivers into the gitignored `tests/e2e/.drivers/`: `msedgedriver.exe` matching the installed WebView2 Runtime (downloaded from Microsoft) and `tauri-driver` (`cargo install --root`). No global installs. |
| `npm run test:e2e` | Desktop end-to-end suite: runs `e2e:setup`, builds the real app (`tauri build --debug --no-bundle`) and drives it with WebdriverIO through `tauri-driver` + Edge WebDriver (`tests/e2e/specs/`). Add `-- --no-build` to reuse the last build. The app runs isolated in `tests/e2e/.run/<time>/` (`OPENFRAME_DATA_ROOT`, honoured by debug builds only); failure screenshots go to `tests/e2e/artifacts/`. Close other OpenFrame windows first — the suite drives real windows and keyboard input. |
| `npm --workspace apps/desktop run build:web` | Type-checks and builds only the web bundle (`apps/desktop/dist`) |
| `node scripts/release-validate.mjs --report-only` | Shows which release prerequisites are still missing |

Faster inner loops:

```powershell
cargo test -p openframe-application                  # application layer only (does not compile Tauri)
cargo test -p openframe-application --test core_pipeline
cd apps/desktop; npx vitest                          # watch mode
```

`cargo clippy`/`cargo check` on the `openframe-desktop` crate needs `apps/desktop/dist` to exist because
`tauri::generate_context!()` checks `frontendDist` at compile time. Run `build:web` once, or create an empty
`apps/desktop/dist/index.html`. CI uses the second option.

## 3. Runtime locations, logs and diagnostics

| What | Where |
|---|---|
| Application data (local profile, recent projects, settings) | `%LOCALAPPDATA%\OpenFrame\app.sqlite` |
| Logs | `%LOCALAPPDATA%\OpenFrame\logs\openframe.YYYY-MM-DD.log`: JSON lines, rotated daily, 14 files kept |
| New projects (default) | `Documents\OpenFrame\Projects\<Title>.openframe\` |
| Global Idea Vault | `Documents\OpenFrame\Global Idea Vault\` (`vault.sqlite` + `assets/`) |
| Local AI models (planned) | `%LOCALAPPDATA%\OpenFrame\models\`, `%LOCALAPPDATA%\OpenFrame\runtimes\` |

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
│  ├─ openframe-application/  AppCore, op registry, Store (mutation pipeline), modules/*, schema list
│  └─ openframe-test-support/ TestEnv: isolated AppCore, op helpers, crash/restart simulation
├─ migrations/{app,global,project}/  embedded SQL migrations (one numbered file per owning module)
├─ docs/
│  ├─ adr/                    Architecture Decision Records
│  ├─ engineering/            implementation specs (11–26) + AGENT_BRIEF.md (module conventions)
│  └─ spec-digest/            dense digests of the authoritative product specs
├─ scripts/                   setup.mjs, license-inventory.mjs, release-validate.mjs
├─ .github/workflows/         ci.yml, release.yml
├─ deny.toml                  cargo-deny policy (licenses, advisories, sources, bans)
├─ HTML_Mockups/              approved interface mock-ups (labels and CSS classes are reused)
└─ OpenFrame_*.md             authoritative product specifications
```

The ESD lists further crates (`openframe-search`, `-media`, `-import-export`, `-ai`). They are declared in
`[workspace.dependencies]` but not created yet. That functionality currently lives in
`openframe-application/src/modules/*`, as ESD §4 allows ("crates may be consolidated during early development").
The dependency direction rules still apply. `openframe-collaboration` will **not** exist: LAN collaboration was
removed (ADR-0007), and package exchange lives with import/export. The stale `[workspace.dependencies]` entry
should be dropped.

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

Follow [docs/engineering/AGENT_BRIEF.md](docs/engineering/AGENT_BRIEF.md). In summary:

1. Add a migration file under `migrations/project/` (use the number already reserved in `schema.rs`).
2. Create `crates/openframe-application/src/modules/<m>/` with a `register(r)` function covering ops, indexers and
   trash handlers, and add one line to `modules/mod.rs`.
3. Add `apps/desktop/src/workspaces/<dir>/index.tsx`. It is discovered automatically.
4. Add tests in `crates/openframe-application/tests/<m>.rs` using `openframe_test_support::TestEnv`.
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
