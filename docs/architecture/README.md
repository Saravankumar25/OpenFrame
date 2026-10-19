# Architecture in one page

> Written 2026-10-09 from reading the code; updated 2026-10-15 for the two-shell decision ([ADR-0027](../adr/ADR-0027-rust-core-two-ui-shells.md)). For the full explanation in simple language, with examples, see Part D of the
> [retrospection and audit report](../reports/2026-10-09-project-retrospection-and-audit.md). The detailed contracts are the
> engineering documents listed at the bottom. This page is the short map to keep current when the structure changes.

## 1. The layers

```text
 The person
    |
    |  keyboard, mouse
    v
+-------------------------------------------------------------+
| Screens: React 19 + TypeScript (apps/desktop/src)            |
|  12 workspaces, a design system, view-only state (zustand),  |
|  a read cache (React Query). No file, no database, no network |
+-------------------------------+-----------------------------+
                                |  one door: of_invoke(op, args)
                                v
+-------------------------------------------------------------+
| Tauri shell (apps/desktop/src-tauri)                          |
|  window, strict content policy, 4 commands, asset protocol,   |
|  logging, single instance, platform hooks                      |
+-------------------------------+-----------------------------+
                                |
                                v
+-------------------------------------------------------------+
| Application core (crates/openframe-application)               |
|  AppCore: registry of 483 operations, entitlement check,       |
|  Store::mutate (the only way to change data), 21 modules       |
+----+---------------+----------------+-----------------+------+
     |               |                |                 |
     v               v                v                 v
 persistence     import-export       search              ai
 SQLite, WAL,    FDX, Fountain,     FTS5 + vectors     llama.cpp
 migrations,     DOCX, PDF, ZIP,    + context graph    processes,
 undo capture    layout, fonts                          bundle check
     |
     v
 project folder on disk: Title.openframe/
   project.sqlite   openframe.json   assets/   cache/   recovery/   backups/
```

The diagram shows the Windows shell. On the Mac the top two boxes are replaced by a SwiftUI application ([ADR-0027](../adr/ADR-0027-rust-core-two-ui-shells.md)); everything below the single door is shared.

Other crates: `openframe-domain` (shared types, errors, roles), `openframe-security` (path confinement, archive limits,
redaction), `openframe-platform` (everything that differs between Windows and macOS), `openframe-entitlement` (trial,
licence and team password), `openframe-project-format` (folder layout and the single-writer lock),
`openframe-test-support` (test helpers). `services/licence-server` is a separate small server for the Mac store channel.

## 2. What happens when a person types a letter

1. The editor (ProseMirror) shows the letter at once.
2. After 400 milliseconds without typing, `SyncController` compares the page with what Rust last saved and sends only the
   difference through `call("screenplay.apply_edits", ...)`.
3. Tauri passes it to `of_invoke`, which runs the work on a background thread.
4. The registry checks the operation exists, the arguments are not absurdly large, and the licence state allows it.
5. The module calls `Store::mutate` with a permission and a label. One database transaction opens, the change is written,
   triggers record an undo step, the search index and the activity list are updated, and the transaction commits.
6. Rust sends a `dataChanged` event; screens that read those tables refresh themselves.

If step 2 is never reached (the window is destroyed first), up to 400 ms of typing is not saved. See phase W2 of the [Windows plan](../roadmap/chapter-1a-windows-phases.md).

## 3. Rules the structure enforces

- React never touches files, SQLite or the network.
- Every change to project data goes through `Store::mutate`.
- The AI can only propose changes; only a local user action applies them.
- Operating-system code lives only in `openframe-platform` (lint rule R6).
- Migrations are numbered, never edited once shipped, and each upgrade is preceded by a verified backup.
- The webview cannot read the database, the intelligence index, recovery data or backups.

`scripts/architecture-lint.mjs` checks these rules in seconds, without compiling.

## 4. Windows and macOS

One Rust core, two shells. `openframe-platform` holds the differences: folders, the atomic file replace, killing child processes with
the app (Job Object on Windows, process group on the Mac), key labels, spell checking, power events, package identity.
On the Mac the shell is a native SwiftUI application that calls the same core through a generated interface (UniFFI). It is built in the Mac track, after the Windows track closes ([ADR-0027](../adr/ADR-0027-rust-core-two-ui-shells.md)); the Tauri Mac shell is retired. The Mac platform adapter has never run. The AI engine is pinned only for Windows today (processor build); the engine provider interface, a Vulkan build and the Mac engine come in phases W5, W7 and M8.

## 5. Where to read more

| Question | Document |
|---|---|
| Why this stack | [ADR-0001](../adr/ADR-0001-tauri-react-rust.md), [ADR-0002](../adr/ADR-0002-rusqlite-over-sqlx.md) |
| The project folder | [ADR-0003](../adr/ADR-0003-project-folder-format.md), [12-project-file-format.md](../engineering/12-project-file-format.md) |
| The single door | [ADR-0004](../adr/ADR-0004-single-ipc-dispatcher.md), [14-ipc-contract.md](../engineering/14-ipc-contract.md) |
| Operations and commands | [13-command-architecture.md](../engineering/13-command-architecture.md) |
| The database | [11-physical-database.md](../engineering/11-physical-database.md) |
| Autosave, undo, recovery | [17-autosave-undo-recovery.md](../engineering/17-autosave-undo-recovery.md) |
| Search and retrieval | [16-search.md](../engineering/16-search.md), [18-retrieval-intelligence-index.md](../engineering/18-retrieval-intelligence-index.md) |
| The AI | [ADR-0013](../adr/ADR-0013-single-local-model-hybrid-rag-agent.md), [ADR-0026](../adr/ADR-0026-offline-ai-download-after-install.md) (superseding ADR-0025), [ADR-0028](../adr/ADR-0028-ai-quality-program-and-harness.md), [ADR-0029](../adr/ADR-0029-gpu-acceleration.md), [ai-tool-coverage.md](../engineering/ai-tool-coverage.md) |
| Threats | [20-security-threat-model.md](../engineering/20-security-threat-model.md) |
| Windows and Mac differences | [37-platform-boundary.md](../engineering/37-platform-boundary.md), 28-cross-platform-compatibility-security.md |
| The two-shell structure and the Mac plan | [ADR-0027](../adr/ADR-0027-rust-core-two-ui-shells.md), [Mac phases](../roadmap/chapter-1b-mac-phases.md). The older 29-desktop-hld-lld.md is history |
