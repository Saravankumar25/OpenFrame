# 14 — Tauri IPC Frontend ↔ Backend Contract

> **Status legend:** ✅ implemented · 🚧 in development · 📋 planned.
> Source: `apps/desktop/src-tauri/src/lib.rs`, `apps/desktop/src-tauri/{tauri.conf.json,capabilities/default.json}`,
> `apps/desktop/src/ipc/{client.ts,query.ts,generated/}`. Decision: ADR-0004.

## 1. Surface ✅

| Tauri command | Args | Returns | Notes |
|---|---|---|---|
| `of_invoke` | `op: string`, `args?: object` (default `{}`) | op result as JSON | The only data path. Runs `AppCore::dispatch(local_actor, op, args)` on a blocking worker. Refreshes the asset scope after `project.*` ops. |
| `of_open_asset` | `assetId`, `global?`, `reveal?` | `()` | Opens a file in its default app or reveals it in Explorer. The path is resolved from the asset row in Rust. Missing file → `not_found.file`. |
| `of_reveal_path` | `kind` ∈ `project`, `globalVault`, `logs`, `exported`; `id?`; `path?` | `()` | `project` resolves the path from recents by id. `exported` requires an absolute, existing path (a file the user just saved). |
| `of_open_url` | `url` | `()` | `http(s)` only, ≤ 4096 chars, explicit user action (Idea Vault URL items) |

Plugins: `dialog` (open/save/message/ask/confirm), `opener` (used only from Rust), `single-instance`. There is **no**
fs, shell or http plugin (ESLint `no-restricted-imports` blocks the packages).

Capability file `main-window`: `core:event:allow-listen|unlisten`, `core:window:allow-set-title|set-fullscreen|
is-fullscreen`, `core:app:allow-version`, `dialog:allow-open|save|message|ask|confirm`.

## 2. Request/response ✅

```ts
import { call } from "../ipc/client";
const summary = await call<ProjectSummary>("project.create", { title: "Railway", projectType: "Feature Film" });
```

- Args are camelCase JSON matching the generated `*Args` type. Unknown fields are rejected
  (`validation.invalid_input`, detail names the op and field).
- Results are the generated DTO types (`apps/desktop/src/ipc/generated/*.ts`). `i64` timestamps arrive as
  `number` (epoch ms).
- Errors reject with `OpError { op, code, message, detail?, retryable }`. **UI logic branches on `code`**
  (`err.is("conflict")`), never on `message`. `reportError` shows `message` as a toast and logs code and detail to
  the console.
- Unknown op → `security.unknown_operation` ("That action isn't available.").

## 3. Reads and invalidation ✅

```ts
const cards = useOp<CardDto[]>("story.list_board", { episodeId }, ["story_act", "story_sequence", "story_scene_card"]);
const move  = useCommand<MoveCardArgs, CardDto>("story.move_card", { onSuccess: () => toast.undoable("Moved Scene Card …") });
```

- `useOp(op, args, tables, {store?})`: a TanStack Query with `staleTime: Infinity`. The `tables` list is every table
  the query reads (`"*"` = any change). Freshness comes from events, not polling.
- `installInvalidation()` listens on `of://event`. For `dataChanged {store, tables}` it invalidates every query of
  that store whose `tables` intersect.
- React never keeps project truth beyond editing buffers (ESD §7). There is no `localStorage` or IndexedDB for
  project data.

## 4. Events ✅ (`of://event`, type `AppEvent`)

| `type` | Payload | UI use |
|---|---|---|
| `dataChanged` | `store` (`project`/`global`), `tables[]`, `ids[]`, `origin` (`local`; `ai` for operations of an applied AI Change Set; `exchange` for package imports) | Query invalidation, editor reload (ADR-0010) |
| `saveState` | `status` (`Saved`/`Saving`/`SavedPendingExternal`/`Error`), `lastSavedAt`, `error?`, `pendingOperations` | Status bar indicator (FSD §3.3) |
| `projectOpened` / `projectClosed` | `projectId` | Shell routing |
| `task` | `taskId, kind, label, state, progress?, message?, error?, result?` | Progress UI for background work |
| `module` | `module, name, payload` | Module-specific (e.g. AI runtime state, model download status) |

## 5. Files, media and dialogs ✅

- **Pick → validate in Rust.** `open()`/`save()` from `@tauri-apps/plugin-dialog` return a path. React passes it to
  an op (e.g. `files.add { paths, mode }`), and Rust validates it (`validate_input_file`: exists, regular file, size
  limit) before reading.
- **Display media** with `convertFileSrc(asset.path)`. The asset protocol scope is empty by default. Rust allows only
  the open project folder and the Global Idea Vault folder, recursively (`sync_asset_scope`).
- **Open/reveal** by id through `of_open_asset` / `of_reveal_path`. The UI never passes a path to open.

## 6. Security headers ✅ (`tauri.conf.json`)

```text
default-src 'self'; img-src 'self' asset: http://asset.localhost data: blob:;
media-src 'self' asset: http://asset.localhost blob:; style-src 'self' 'unsafe-inline'; font-src 'self' data:;
script-src 'self'; connect-src ipc: http://ipc.localhost; object-src 'none'; frame-src 'none';
base-uri 'none'; form-action 'none'
```

Plus `freezePrototype: true`. `style-src 'unsafe-inline'` is required by Radix/React inline styles. Scripts are
self-only.

## 7. Type generation ✅

`npm run bindings` (`cargo test --workspace export_bindings`) writes `apps/desktop/src/ipc/generated/*.ts` via ts-rs
(`TS_RS_EXPORT_DIR` in `.cargo/config.toml`). The files are committed, and CI fails on drift.

## 8. Compatibility rules

- The op name and args/DTO shape are a contract between the Rust and TS in the same build (shipped together), so
  there is no version negotiation. Renames must update both sides in one commit.
- AI Change Set application ✅ uses the **same op names and args** through `AppCore::dispatch` with
  `Actor.origin = Ai`, only after the user clicks **Apply Changes**. Ops must therefore not assume a UI caller, and
  must never read UI-only state. The review ops `ai.change_set.accept|reject|recheck` are the reverse: they refuse any
  actor that is not `origin = Local`, so they are reachable only from the UI through `of_invoke`. The model never
  calls ops directly; it chooses tools (AI specification §7). There is no
  network-facing caller: LAN collaboration was removed (ADR-0007), and OpenFrame opens no listener.
