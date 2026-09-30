# ADR-0004: Single allow-listed IPC dispatcher `of_invoke`, operation registry, asset access resolved in Rust

- **Status:** Accepted (implemented)
- **Date:** 2026-09-30
- **Related:** ESD §6, §13; Security threat model ("strict IPC command allow-list"); invariants 1, 3, 6;
  [14-ipc-contract.md](../engineering/14-ipc-contract.md)

## Context

The webview renders untrusted content: imported scripts, received packages, pasted text, and later AI output. Tauri
can expose many commands and plugins (fs, shell, http), each widening the attack surface. The same operations must
also be callable when applying approved Change Sets (AI, import, package responses), with identical validation and
permission checks.

## Decision

1. The webview gets **exactly four Tauri commands**:
   - `of_invoke(op: string, args?: object)`: the only data path. `op` is looked up in the application `Registry`.
     Unknown names fail with `security.unknown_operation`.
   - `of_open_asset(assetId, global?, reveal?)`: open or reveal a file by **asset id**. The path is resolved from
     the asset row in Rust. The UI never supplies a path.
   - `of_reveal_path(kind, id?, path?)`: reveal known locations (`project`, `globalVault`, `logs`) or a file the
     user just exported (absolute path, must exist).
   - `of_open_url(url)`: hand an `http(s)` link (≤ 4096 chars) to the default browser, on explicit user action only.
2. **Registry = allow-list.** Modules register `query` (read-only) and `command` (mutating) ops named
   `module.action`. Duplicate names panic at startup. Args structs use `#[serde(deny_unknown_fields)]`, so unexpected
   fields are rejected (`validation.invalid_input`).
3. **Same entry for every caller.** `AppCore::dispatch(actor, op, args)` is used by the Tauri adapter (local actor)
   and will be used by AI Change Set application (`ActorOrigin::Ai`). Permission checks use the actor's role
   (ADR-0009). `ActorOrigin::Lan` is an unused remnant (ADR-0007).
4. **Least-privilege capabilities** (`capabilities/default.json`): events listen/unlisten, window title/fullscreen,
   app version, and dialog open/save/message/ask/confirm. No fs, shell or http plugin. ESLint forbids importing
   those plugins.
5. **Asset protocol** is enabled with an **empty static scope**. At runtime Rust allows only the open project folder
   and the Global Idea Vault folder (`sync_asset_scope`, refreshed after every `project.*` op), so `<img>`/`<video>`
   can show project media and nothing else.
6. **CSP:** `default-src 'self'`; `connect-src ipc: http://ipc.localhost`; no remote scripts, frames or objects;
   `freezePrototype: true`.
7. **Events:** Rust pushes `AppEvent`s on one channel, `of://event` (`DataChanged`, `SaveState`, `ProjectOpened`/
   `Closed`, `Task`, `Module`).

## Consequences

- Adding an op never touches Tauri configuration. Security review focuses on the registry and each handler's
  capability.
- Op names are strings at the TS boundary. Argument and result types are generated (ts-rs), and modules may add
  typed wrappers in `src/api/<module>.ts`.
- Every call runs on a blocking worker thread (`spawn_blocking`) and is timed. Slow ops (> 250 ms) are logged at info
  level.
- File pickers return a path to React, which passes it to a Rust op that validates it (`validate_input_file`, size
  limits). React cannot read the file itself.
- The asset scope grants read access to the whole open project folder, including `project.sqlite`. This is
  acceptable because the webview can already read all project data through queries. The scope never extends beyond
  the open project and the vault.

## Alternatives rejected

| Alternative | Reason |
|---|---|
| One `#[tauri::command]` per operation | Hundreds of commands, each needing capability configuration. Harder to audit. Not reusable for Change Set application. |
| `plugin-fs` with scoped paths for media and imports | Gives the webview direct file authority, violating invariants 1 and 6 |
| Local HTTP server for the UI | Opens a network listener that any local process could hit. Needs auth tokens and CORS. |
| Passing file paths from UI to `open` | Path injection risk. Asset ids resolved in Rust close it. |
