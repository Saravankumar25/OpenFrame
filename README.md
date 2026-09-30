# OpenFrame Studio

OpenFrame Studio is a **local-first desktop studio for filmmakers**. It takes a film from first idea to shoot day in one
project: Idea Vault, Story Board (acts, sequences, beats and scene cards), screenplay, breakdown and production
catalog, visual planning, shooting schedule and call sheets. It is built for Windows 11 with Tauri v2, React and
TypeScript for the interface, and Rust with SQLite for everything that owns data.

> **Status: pre-release, under active development.** The durable project core is implemented and tested. Most
> creative and production workspaces are being built as separate modules (see [Feature status](#feature-status)).
> No installer has been published, and **no open-source license has been selected yet** (see
> [LICENSE-PENDING.md](LICENSE-PENDING.md)).

## Privacy and data ownership: what is true today

These statements describe the code in this repository. They are not aspirations. Planned behavior is labeled as planned.

| Promise | How it holds in this implementation |
|---|---|
| **No account, no sign-in** | There is no login, server or identity provider. On first launch OpenFrame creates a local profile in `%LOCALAPPDATA%\OpenFrame\app.sqlite`: a random UUIDv7 user id plus a display name, defaulting to your Windows user name and changeable in the app. |
| **Your projects are ordinary folders on your computer** | Each project is a `<Title>.openframe` folder, by default under `Documents\OpenFrame\Projects`. It holds a SQLite database, a JSON manifest and your media under `assets/`. The Global Idea Vault lives in `Documents\OpenFrame\Global Idea Vault`. You can copy, back up or delete these folders yourself. |
| **No telemetry, analytics or crash upload** | The application contains no telemetry, analytics or crash-reporting code, and makes no background network requests. |
| **No network listener** | OpenFrame never opens a listening socket. Collaboration is file-based: you export a package and send it however you like. |
| **The UI cannot reach the network or your disk** | The webview's Content Security Policy allows connections only to the Tauri IPC channel. The webview has no filesystem, shell or HTTP plugin (lint-enforced). Every read and write goes through one allow-listed Rust entry point (`of_invoke`), and files are opened by id with the path resolved in Rust. |
| **Network use** | The running app itself makes no network requests. There are two exceptions. (1) When you open a web link saved in the Idea Vault, the link is handed to your default browser. (2) If the WebView2 runtime is missing, the installer downloads Microsoft's WebView2 bootstrapper. Windows 11 ships with WebView2, so this normally does not happen. *Planned:* the one-click **Download Offline AI** model download and the update check. Both happen only after you explicitly approve them. |
| **Logs stay local** | Diagnostic logs are written only to `%LOCALAPPDATA%\OpenFrame\logs` and kept for 14 days. They record operation names, error codes and timings, not project content. Panic messages are redacted (user paths, e-mail addresses, tokens). |
| **Not encrypted by OpenFrame** | Project data is protected by your Windows account's file permissions, like any document. OpenFrame does not encrypt projects at rest. Optional encrypted projects are later scope. |
| **AI is local-only, when it ships** | The approved v1 design uses a local `llama.cpp` sidecar with no cloud providers, API keys or Ollama. *The AI assistant is not in this build yet.* |

## Feature status

| Area | Status |
|---|---|
| Project lifecycle: create, open, close, duplicate, archive, delete to Recycle Bin, recent projects, locate moved project | Implemented |
| Single-writer project lock; refuses network-share (UNC) project locations | Implemented |
| Continuous autosave (every change is a committed SQLite transaction), explicit Save checkpoint, save-state indicator | Implemented |
| Crash detection and recovery offer (keep latest autosave or return to last confirmed save) | Implemented |
| Schema migrations with a verified safety backup first, one atomic migration transaction, and refusal of projects made by newer versions | Implemented |
| Generic undo/redo for every canonical change, refusing to overwrite later work | Implemented |
| Recently Deleted (restore / permanent delete), Activity history | Implemented |
| Project Files cabinet (managed copies, external references, relink, folders) | Implemented |
| Global search (SQLite FTS5) over everything modules index, with Private Notes visible only to their owner | Implemented (engine); coverage grows with each module |
| App shell, Home, workspace navigation, command/search overlay, toasts with Undo | Implemented |
| Idea Vault (project and global) | In development |
| Story Board: acts, sequences, beats, scene cards, characters, timeline | In development |
| Screenplay editor, drafts, compare, review, comments, lock and revisions | In development |
| Breakdown, production catalog, locations, cast and crew | In development |
| Moodboards, storyboards, shot lists | In development |
| Shooting schedule, call sheets, sides, reports | In development |
| Import/export (PDF, FDX, Fountain, DOCX, CSV/XLSX), backup packages | In development |
| Local AI assistant (managed `llama.cpp` sidecar, signed model manifests, Change Sets) | In development |
| Collaboration through Exchange / Review / Response packages (file-based; roles govern package and export scope) | In development |
| Real-time LAN collaboration | **Removed from scope** by product-owner decision (2026-09-30, ADR-0007). OpenFrame opens no network listener. |
| Signed installer (NSIS/MSI) and in-app updater | Release pipeline written; blocked on certificate, updater key and license selection |

"In development" means the module is being built against the foundation contracts described in
`docs/engineering/`. It is not usable in this build. A workspace that is not present yet shows
"This workspace isn't available in this build." instead of fake content.

## Quick start (developers)

Requirements: Windows 11, Node.js ≥ 20, Rust (stable, MSVC) ≥ 1.85, Visual Studio Build Tools with the
C++ workload, and the WebView2 Runtime. [DEVELOPMENT.md](DEVELOPMENT.md) has exact install commands.

```powershell
npm run setup      # checks prerequisites, prints fixes, then runs npm install
npm run dev        # launches the desktop app with hot reload (the first Rust build takes several minutes)
npm run test       # Rust + frontend tests
```

## Documentation

| Document | Purpose |
|---|---|
| [DEVELOPMENT.md](DEVELOPMENT.md) | Setup, commands, repository layout, architecture overview, logs |
| [CONTRIBUTING.md](CONTRIBUTING.md) | Review contract, engineering invariants, testing and dependency policy |
| [docs/adr/](docs/adr) | Architecture Decision Records (ADR-0001 … ADR-0012) |
| [docs/engineering/](docs/engineering) | Engineering specifications that reflect the implemented code (database, file format, commands, IPC, search, recovery, security, performance, QA, release, packaging, CI, observability) |
| `OpenFrame_*.md` (repository root) | Authoritative product specifications: PRD, FSD, UX/UI, Domain/Data, AI, Import/Export, Offline/Collaboration, Security. Implementation precedence: PRD > FSD > UX > Domain > cross-cutting specs > engineering docs. The LAN sections are superseded by ADR-0007. |
| [THIRD_PARTY_LICENSES.md](THIRD_PARTY_LICENSES.md) | Generated inventory of shipped third-party dependencies and their licenses |

## License

**Not yet selected.** No license is granted until one is chosen. See [LICENSE-PENDING.md](LICENSE-PENDING.md).
