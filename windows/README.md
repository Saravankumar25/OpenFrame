# OpenFrame Studio

OpenFrame Studio is a **local-first desktop studio for writers, screenwriters and production teams**. It takes a film from first idea to shoot day in one
project: Idea Vault, Story Board (acts, sequences, beats and scene cards), screenplay, breakdown and production
catalog, visual planning, shooting schedule and call sheets. It is built with Tauri v2, React and TypeScript for the
interface, and Rust with SQLite for everything that owns data. It targets Windows 11 and Apple silicon Macs from one
codebase and will be sold with a 7-day free trial.

> **Status (2026-10-09): pre-release, no installer built yet.** The application is largely built and has been
> tested end to end on the development machine in a debug build. Nothing is release-ready: no installer has been built or
> installed on a clean computer, the Mac build has never run, and the offline AI has not been measured for quality. The
> work is in three chapters (finish and test, website, store distribution); see [the roadmap](docs/roadmap/README.md). The
> app is a paid, closed-source product, so no open-source license applies; the end-user license agreement is still to be
> written (see [LICENSE-PENDING.md](LICENSE-PENDING.md)).

## Privacy and data ownership: what is true today

These statements describe the code in this repository. They are not aspirations. Planned behavior is labeled as planned.

| Promise | How it holds in this implementation |
|---|---|
| **No account, no sign-in** | There is no login, server or identity provider. On first launch OpenFrame creates a local profile in `%LOCALAPPDATA%\OpenFrame\app.sqlite`: a random UUIDv7 user id plus a display name, defaulting to your Windows user name and changeable in the app. |
| **Your projects are ordinary folders on your computer** | Each project is a `<Title>.openframe` folder, by default under `Documents\OpenFrame\Projects`. It holds a SQLite database, a JSON manifest and your media under `assets/`. The Global Idea Vault lives in `Documents\OpenFrame\Global Idea Vault`. You can copy, back up or delete these folders yourself. |
| **No telemetry, analytics or crash upload** | The application contains no telemetry, analytics or crash-reporting code, and makes no background network requests. |
| **No network listener** | OpenFrame opens no listening socket that other computers can reach. Collaboration is file-based: you export a package and send it however you like. The only listeners are the Offline AI engines (while in use), bound to `127.0.0.1` on a random port and protected by a random per-launch key. |
| **The UI cannot reach the network or your disk** | The webview's Content Security Policy allows connections only to the Tauri IPC channel. The webview has no filesystem, shell or HTTP plugin (lint-enforced). Every read and write goes through one allow-listed Rust entry point (`of_invoke`), and files are opened by id with the path resolved in Rust. |
| **Network use** | The running app itself makes no network requests. There are two exceptions. (1) When you open a web link saved in the Idea Vault, the link is handed to your default browser. (2) If the WebView2 runtime is missing, the installer downloads Microsoft's WebView2 bootstrapper. Windows 11 ships with WebView2, so this normally does not happen. Offline AI is installed with the app and never uses the network (ADR-0026). No project content is ever sent. *Planned:* the direct-download edition's update check, only after you approve it. |
| **Logs stay local** | Diagnostic logs are written only to `%LOCALAPPDATA%\OpenFrame\logs` and kept for 14 days. They record operation names, error codes and timings, not project content. Panic messages are redacted (user paths, e-mail addresses, tokens). |
| **Not encrypted by OpenFrame** | Project data is protected by your Windows account's file permissions, like any document. OpenFrame does not encrypt projects at rest. Optional encrypted projects are later scope. |
| **AI runs on this computer, right after install** | The optional AI assistant uses **Offline AI**: one local model (Google Gemma 3 1B Instruct) and a small local embedding model (BGE small English v1.5), run by OpenFrame-managed `llama.cpp` processes. There are no cloud AI providers, API keys, accounts or Ollama, and no model choice. The AI runtime ships inside the app. The models (about 1.1 GB) download once, after install, when you click **Download OpenFrame AI** (ADR-0026); the app checks every downloaded file against a signed manifest and SHA-256 hashes before it uses it. After that AI works offline, with no setup. The per-project search index for the assistant (`cache/intelligence.sqlite`) is derived data that can be deleted and is rebuilt. **The AI never changes your project on its own**: every change is shown as *Proposed Changes* and applied only when you click **Apply Changes**, and it can never do more than you are allowed to do. |

## Feature status

Written from [feature-status.md](docs/implementation/feature-status.md) on 2026-10-09; that file is the source and has the
evidence for every row. "Verified" here means proved by the end-to-end test suite and the unit tests on the development
machine, in a debug build. It does not mean installed and tried on a clean computer.

| Area | State |
|---|---|
| Projects: create, open, close, duplicate, archive, recents; autosave; crash recovery; migrations with a safety backup first; undo and redo; Recently Deleted; Activity; Files; search | Built and Reachable; undo, Recently Deleted and the 30-day purge Verified. Delete and "locate a moved project" have no tests yet |
| Idea Vault (project and global) | Reachable, with the research flags and the purge Verified |
| Story Board: acts, sequences, beats, scene cards, characters, timeline, episodes, Build Screenplay | Verified for boards, cards, drawers and Build Screenplay; characters and episodes Reachable |
| Screenplay editor: elements, drafts, compare, review, lock and revisions, production numbers, pages, page setup, custom elements, suggestions, shortcuts, spell check | Mostly Verified |
| Import and export: Final Draft, Fountain, Word, PDF, text; backup and project packages; exchange and review packages | Final Draft import, packages and backup Verified; other formats Reachable. The manual restore drill and a Final Draft reopen check are owner tasks |
| Breakdown, production catalog, locations, cast and crew | Verified |
| Moodboards, storyboards, shot lists | Reachable. No end-to-end step yet |
| Shooting schedule and Day Out of Days | Verified |
| Call sheets, sides, reports, budget, templates, sample scripts | Verified; the call sheet PDF is Reachable |
| Appearance: 15 themes, Light, Dark, System, theme import | Verified |
| Offline AI: Gemma 3 1B and BGE small, bounded assistant, human-only Apply Changes, crash-safe apply | Working on Windows x64 in the test build, where the models are still bundled; the download after install is to be rebuilt (ADR-0026). Answer quality, speed on small laptops and non-English requests are not measured. No Mac engine yet |
| Internal Windows installer (`.exe`) | Not built. Needs the owner's word |
| Internal Mac disk image (`.dmg`) | Not built. The Mac code compiles in CI and has never run |
| Real-writer test (2 to 3 days) | Planned: [internal test plan](docs/testing/internal-test-plan.md) |
| Product website | Not started (Chapter 2) |
| Microsoft Store and Mac App Store, trial and payment | Not started in practice (Chapter 3); the trial and lock code exists and has only been tested with fakes |
| Real-time LAN collaboration | **Removed from scope** by product-owner decision (2026-09-30, ADR-0007). OpenFrame opens no network listener |

A workspace or action that is not available shows "This workspace isn't available in this build." instead of fake content.

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
| [docs/README.md](docs/README.md) | **Start here.** Documentation map, which document wins, and what to update when code changes |
| [docs/implementation/masterplan.md](docs/implementation/masterplan.md) | Build plan: 26 areas with checklists, from today to submission in both stores and the installer |
| [docs/implementation/feature-status.md](docs/implementation/feature-status.md) | How far every product feature has come (Specified to Release-ready) |
| [docs/implementation/owner-decisions.md](docs/implementation/owner-decisions.md) | Decisions and non-code work that need the owner |
| [docs/release/launch-readiness-register.md](docs/release/launch-readiness-register.md) | Operations, legal, commerce, security, support and release requirements |
| [docs/adr/](docs/adr) | Architecture Decision Records (see the register for accepted and proposed decisions) |
| [docs/engineering/](docs/engineering) | Engineering specifications that reflect the implemented code (database, file format, commands, IPC, search, recovery, security, performance, QA, release, packaging, CI, observability) |
| [`docs/specs/`](docs/specs) | Authoritative product specifications: PRD, FSD, UX/UI, Domain/Data, AI, Import/Export, Offline/Collaboration, Security, and the Screenplay and Production Depth Specification (Revision 1). Precedence is in [docs/README.md](docs/README.md) §1. The LAN sections are superseded by ADR-0007. |
| [THIRD_PARTY_LICENSES.md](THIRD_PARTY_LICENSES.md) | Generated inventory of shipped third-party dependencies and their licenses |

## License

OpenFrame Studio is proprietary software. No license is granted until the end-user license agreement is published. See [LICENSE-PENDING.md](LICENSE-PENDING.md).
