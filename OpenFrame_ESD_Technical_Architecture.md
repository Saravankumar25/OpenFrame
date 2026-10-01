# OpenFrame Studio — Engineering System Design / Technical Architecture

    **Product:** OpenFrame Studio  
    **Document class:** Implementation / engineering specification  
    **Baseline:** Windows 11 production architecture, v1  
    **Generated:** 29 September 2026  
    **Purpose:** Define the production-grade implementation architecture for the local-first OpenFrame desktop application.


## Authority and non-conflict rule

This engineering document implements the approved OpenFrame specifications. It does **not** redefine product behavior.

Implementation precedence is:

1. Product PRD — scope, intent, priority and product boundaries.
2. Functional Specification — observable behavior and acceptance contract.
3. UX/UI Specification — presentation and interaction.
4. Domain & Data Specification — canonical identities, relationships, lifecycle and source-of-truth.
5. AI / Import-Export / Offline-Collaboration / Security specifications — cross-cutting specialized contracts.
6. This engineering document — concrete implementation choices.

If an implementation detail conflicts with an authoritative product rule, the implementation detail must change. Engineering must not “fix” the product by silently changing behavior.


## Locked implementation decisions

These decisions were explicitly approved for the first implementation baseline:

| Decision | Locked choice |
|---|---|
| Initial OS | Windows 11 first; preserve macOS-ready boundaries |
| Desktop shell | Tauri v2 |
| UI | React + TypeScript |
| Core/business logic | Rust owns domain rules; React is presentation/input |
| Local persistence | SQLite + filesystem assets |
| Project UX | One `.openframe` project concept; internal files are implementation detail |
| Offline AI | One-click **Download Offline AI**; OpenFrame installs runtime/model |
| Model selection | One profile, no choice: Gemma 3 1B Instruct + BGE small English embeddings (ADR-0013; the earlier Qwen profiles are superseded) |
| Model distribution | OpenFrame-controlled public model distribution endpoint/CDN |
| AI providers | **Local AI only** for v1; no cloud AI provider integration |
| Accounts | No account required |
| Telemetry | Off by default; optional anonymous diagnostics/crash reports with explicit consent |
| Product model | Free/open-source; exact OSS license identifier must be selected before public release |
| LAN collaboration | Architecture-ready now; ship after stable solo core |
| Concurrent collaboration | Soft locks + optimistic version/conflict handling; no CRDT in v1 |
| Autosave | Continuous autosave + transactional writes + recovery history |
| Encryption | OS security by default; optional encrypted project/package mode is later scope |
| Updates | Notify user; download/install only after explicit approval |
| Migration | Safety backup first, then transactional migration with rollback/recovery path |
| Offline UX | Disable only unavailable online operations; local work continues normally |


## 1. Architectural goals

OpenFrame must feel like one fast native desktop tool even though the UI is web-rendered. The architecture optimizes for:

- local-first operation;
- fast startup and low idle resource use;
- deterministic, recoverable project mutations;
- user-owned files;
- rich desktop interaction;
- offline editing and export;
- optional local AI after a one-click model download;
- future LAN collaboration without introducing a mandatory cloud;
- long-term project compatibility.

## 2. Chosen technology stack

| Area | Choice | Engineering rationale |
|---|---|---|
| Desktop container | Tauri v2 | Native Rust backend, OS webview, smaller distribution than bundling Chromium |
| Frontend | React + TypeScript + Vite | Mature desktop-style interactive UI and fast iteration |
| Styling | Tailwind CSS + OpenFrame design tokens | Consistent design system; no business logic in styling |
| UI primitives | Radix UI or equivalent accessible primitives | Menus/dialogs/focus behavior |
| Local UI state | Zustand | Small view/session state only |
| Backend/query state | TanStack Query | Async IPC cache/invalidation; not canonical data |
| Large collections | TanStack Virtual | Virtualized cards/lists/rows |
| Drag/drop | dnd-kit | Story Board, stripboard, schedule interactions |
| Structured editor | ProseMirror/Tiptap core | Screenplay structure, selection and transaction model |
| Core language | Rust stable | Native performance, strong types and safe concurrency |
| Async runtime | Tokio | File I/O, sidecars, downloads, networking |
| Serialization | Serde | Typed Rust serialization |
| Database | SQLite | Embedded, transactional, local-first persistence |
| DB access | SQLx | Explicit SQL, migrations and typed mapping |
| Full-text search | SQLite FTS5 | Local index without separate search server |
| Files | Native filesystem | Media/assets stay outside SQLite blobs |
| AI runtime | OpenFrame-managed `llama.cpp` sidecar | No Ollama/user setup |
| AI model family | Gemma 3 1B Instruct GGUF (Q8_0) + BGE small English v1.5 GGUF, listed in the signed manifest | One tested profile; the manifest can move it to a new version without product coupling (ADR-0013) |
| Networking later | Rust WebSocket + mDNS discovery | Host-owned LAN sessions |
| PDF/doc output | Deterministic local document renderers | Offline professional exports |
| Logging | `tracing` ecosystem | Structured local diagnostics |
| Distribution | Tauri NSIS `.exe`; optional `.msi` | Windows-native installation |

## 3. Process architecture

```text
┌──────────────────────────────────────────────────────────────────────┐
│ OpenFrame Studio process                                            │
│                                                                      │
│  WebView / React                                                     │
│  ├─ App shell                                                        │
│  ├─ Workspaces                                                       │
│  ├─ Editors                                                          │
│  └─ View state                                                       │
│           │ typed Tauri commands/events                              │
│           ▼                                                          │
│  Rust application                                                    │
│  ├─ Application services                                             │
│  ├─ Domain commands                                                  │
│  ├─ Authorization                                                    │
│  ├─ Persistence / migrations                                         │
│  ├─ Search / indexing                                                │
│  ├─ Import / export                                                  │
│  ├─ Files / media                                                    │
│  ├─ Recovery / backup                                                │
│  └─ Optional LAN host/client                                         │
└───────────────┬──────────────────────────────────────────────────────┘
                │ lifecycle-managed loopback/stdio boundary
                ▼
┌──────────────────────────────────────────────────────────────────────┐
│ Optional OpenFrame local-AI sidecar                                 │
│ `llama.cpp` chat (Gemma 3 1B) + embedding (BGE small) sidecars      │
│ No project filesystem authority                                      │
│ No direct SQLite authority                                           │
└──────────────────────────────────────────────────────────────────────┘
```

The AI process is not required for core application startup. It starts only when requested or when configured prewarming is enabled.

## 4. Repository/workspace structure

```text
openframe-studio/
├─ apps/
│  └─ desktop/
│     ├─ src/                         # React
│     ├─ src-tauri/                   # Tauri entrypoint
│     └─ public/
├─ crates/
│  ├─ openframe-domain/
│  ├─ openframe-application/
│  ├─ openframe-persistence/
│  ├─ openframe-project-format/
│  ├─ openframe-search/
│  ├─ openframe-import-export/
│  ├─ openframe-ai/
│  ├─ openframe-media/
│  ├─ openframe-collaboration/
│  ├─ openframe-security/
│  └─ openframe-test-support/
├─ migrations/
├─ schemas/
│  ├─ ipc/
│  ├─ project-manifest/
│  ├─ packages/
│  └─ ai-tools/
├─ templates/
├─ scripts/
├─ tests/
│  ├─ fixtures/
│  ├─ golden/
│  └─ e2e/
└─ docs/
```

Crates may be consolidated during early development if compile times become painful, but dependency direction must remain clean.

## 5. Dependency direction

```text
React UI
   ↓
Tauri adapter
   ↓
Application services
   ↓
Domain
   ↑
Infrastructure adapters
(SQLite / FS / AI / export / LAN)
```

Rules:
- `openframe-domain` imports no Tauri, React, SQLite or AI runtime types.
- application commands depend on domain interfaces, not UI.
- infrastructure implements ports/interfaces.
- Tauri is an adapter, not the application architecture.
- import/export parsers cannot bypass application validation when applying imported state.

## 6. Command/query separation

### Queries
Read project state or compute deterministic projections. Queries do not mutate canonical state.

Examples:
- get project home summary;
- list scene cards;
- search screenplay;
- calculate schedule conflicts;
- inspect production-source staleness.

### Commands
Every canonical mutation is represented as a command.

Examples:
- create scene card;
- move scene card;
- update screenplay element;
- create named draft;
- confirm breakdown suggestions;
- assign scene to shooting day;
- finalize call sheet;
- apply approved AI Change Set.

Commands execute through:
1. input validation;
2. project/session authorization;
3. invariant validation;
4. SQLite transaction;
5. activity/audit record;
6. undo/recovery record where applicable;
7. search-index update;
8. domain event publication;
9. frontend invalidation/event.

## 7. Data ownership

Canonical project data lives in the project's SQLite database and project-managed asset tree. React state is a cache/view representation only.

Never use:
- browser `localStorage` as project truth;
- IndexedDB as project truth;
- Redux/Zustand as project truth;
- an AI conversation transcript as project truth;
- an exported PDF/FDX as project truth after export.

## 8. Background work

Longer tasks must not block the UI thread or primary Rust command dispatcher.

Background task categories:
- thumbnail generation;
- search index rebuild;
- package compression;
- PDF/document rendering;
- hash calculation;
- model download;
- model warm-up;
- import parsing;
- backup creation.

Each background operation exposes:
- task id;
- state (`queued`, `running`, `completed`, `failed`, `cancelled`);
- progress where measurable;
- cancellation capability where safe;
- typed error;
- resumability policy.

## 9. File/media strategy

SQLite stores metadata and references. Binary media normally lives in project-managed filesystem storage.

Asset ingestion modes:
- **managed copy** — OpenFrame copies into project assets;
- **external reference** — OpenFrame records a path/reference and availability metadata;
- **generated artifact** — thumbnails/previews/cache, always rebuildable.

No code may assume an external path remains available.

## 10. AI architecture

Local AI is an optional interpretation layer over deterministic OpenFrame capabilities.

```text
User request
  ↓
context resolver
  ↓
local model
  ↓
validated structured intent/tool request
  ↓
OpenFrame tool registry
  ↓
query OR proposed Change Set
  ↓
if mutation: preview → explicit accept
  ↓
normal application command
```

The model never receives arbitrary filesystem access, SQL execution, shell execution, or direct mutation authority.

## 11. Offline architecture

Network absence is a normal runtime condition.

The following do not require network:
- create/open/edit/save project;
- all creative/production modules;
- imports/exports that use local files;
- backup/recovery;
- search;
- LAN on an available local network;
- installed local AI.

Network-only operations in v1:
- initial/updated local model download;
- app update check/download;
- optional opt-in remote crash report if later configured.

## 12. Error taxonomy

All backend errors map to stable codes:

```text
validation.*
permission.*
not_found.*
conflict.*
storage.*
project_format.*
migration.*
import.*
export.*
ai.runtime.*
ai.model.*
network.*
security.*
internal.*
```

Frontend behavior must use error codes, not parse English strings.

## 13. Security architecture

- Tauri capability/permission surface is minimized.
- frontend never receives arbitrary filesystem privilege.
- user-selected paths are scoped and normalized.
- project-relative paths cannot escape project root.
- package extraction protects against zip-slip/path traversal.
- all model/update artifacts are integrity verified.
- no secrets are required for local AI.
- optional crash telemetry is opt-in and scrubbed.
- LAN host binds only to intended interfaces and enforces session authorization.

## 14. Windows-first portability boundary

Windows-specific code is isolated behind adapters:
- shell integration;
- Credential Manager if later needed;
- installer/update;
- filesystem watcher edge behavior;
- native dialogs.

Domain, application, persistence schema, project format and AI tool contracts remain portable to a future macOS build.

## 15. Explicitly rejected architecture

Do not introduce without a new ADR:
- Electron;
- Next.js server runtime;
- FastAPI/Python service for the desktop core;
- PostgreSQL;
- Redis;
- Docker requirement;
- mandatory account/auth server;
- mandatory cloud database;
- Ollama dependency;
- cloud AI provider;
- microservices;
- LAN clients opening host SQLite over SMB/network filesystem;
- agent frameworks controlling project state.
