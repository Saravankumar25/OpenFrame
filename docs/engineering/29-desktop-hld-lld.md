# OpenFrame Studio: Optimal HLD & LLD for the Desktop Application

> **Repository integration (2026-10-05).** Promoted from the supplied HLD/LLD. It describes the **target architecture** and
> overlaps with `OpenFrame_ESD_Technical_Architecture.md` and docs 13, 14 and 17; where they differ on current behaviour, the ESD and
> the code win, and this file wins only for the target items (platform services, theme subsystem, screenplay layout service,
> lineage rules). The bounded-context view and dependency directions are enforced by CONTRIBUTING §2 and the architecture lint (build plan: architecture lint).
> Platform sections follow ADR-0017 (Accepted 2026-10-06: Windows 11 only, Apple silicon only, three release files from one codebase). The repository additions it lists (`crates/openframe-platform` and so on) belong to the "Platform adapter layer for Windows and Mac" area of the build plan, which is on the release path.

**Baseline reviewed:** OpenFrame `main`, HEAD
`d30b5889ab5edf9f69d60f8e6d600cb11d4921a0`\
**Primary architecture:** Tauri v2 + React/TypeScript + Rust + SQLite\
**Target platforms:** Windows and macOS\
**Design objective:** evolve the current product into a shippable,
maintainable, offline-first desktop studio without breaking existing
data, commands, undo/redo, AI, project files, or user workflows.

------------------------------------------------------------------------

## 1. Executive architecture decision

OpenFrame should remain a **single-process desktop application with a
strongly isolated webview UI and a Rust application core**.

The target architecture is:

``` text
┌─────────────────────────────────────────────────────────────────────┐
│                         OPENFRAME DESKTOP                            │
│                                                                     │
│  React / TypeScript UI                                              │
│  ├─ Shell / navigation / workspaces                                 │
│  ├─ editors / boards / production views                             │
│  ├─ query cache + editing buffers                                   │
│  └─ presentation/theme/accessibility                                │
│                    │                                                │
│                    │ one typed IPC surface                          │
│                    ▼                                                │
│             Tauri adapter / security boundary                       │
│             ├─ of_invoke                                             │
│             ├─ asset access                                          │
│             ├─ dialogs                                                │
│             └─ external URL opening                                   │
│                    │                                                │
│                    ▼                                                │
│              openframe-application                                   │
│              ├─ AppCore                                               │
│              ├─ Registry                                              │
│              ├─ Store / transaction pipeline                          │
│              ├─ domain modules                                       │
│              ├─ search projection                                    │
│              ├─ AI orchestration / tools / Change Sets              │
│              └─ background task services                              │
│                    │                                                │
│       ┌────────────┼─────────────────────┐                           │
│       ▼            ▼                     ▼                           │
│  canonical      derived              optional                       │
│  project data   intelligence         local AI                       │
│  SQLite         SQLite               sidecars                        │
│       │            │                     │                            │
│       └────────────┴─────────────────────┘                            │
│                    │                                                │
│              managed project folder                                  │
│              project.sqlite                                          │
│              manifest.json                                           │
│              assets/                                                 │
│              cache/intelligence.sqlite                               │
└─────────────────────────────────────────────────────────────────────┘
```

### Non-negotiable boundaries

1.  React never owns canonical project truth.
2.  React never directly opens SQLite or arbitrary filesystem paths.
3.  Every canonical mutation goes through `Store::mutate` or the
    existing undo/redo infrastructure.
4.  `Registry` remains the only operation allow-list.
5.  Cross-module SQL is not introduced as a shortcut.
6.  Project SQLite remains the source of truth.
7.  Search and AI intelligence remain derived.
8.  AI never directly writes canonical state.
9.  Human approval remains mandatory for AI Change Sets.
10. Platform differences belong behind Rust/Tauri adapter services, not
    inside feature modules.
11. macOS support must be additive: no Windows behavior is removed
    merely to obtain portability.
12. Every new feature must preserve the lifecycle: **Discover → Use →
    Save → Undo → Recover → Export → Re-import/reopen → Continue**.

------------------------------------------------------------------------

# 2. Current architecture that must be preserved

The current repository already establishes a strong layering model.

  ----------------------------------------------------------------------------
  Layer                        Current responsibility  Future rule
  ---------------------------- ----------------------- -----------------------
  `openframe-domain`           IDs, errors, roles,     Keep I/O-free
                               capabilities, enums,    
                               format constants        

  `openframe-persistence`      SQLite connection       Remain canonical
                               policy, migrations,     persistence boundary
                               undo, integrity,        
                               backups                 

  `openframe-project-format`   folder layout,          Add platform-neutral
                               manifest, lock, crash   abstractions only
                               marker, atomic          
                               filesystem operations   

  `openframe-security`         path confinement, ZIP   Become shared security
                               safety, hashing,        foundation
                               signatures, redaction   

  `openframe-application`      AppCore, Registry,      Remain platform-neutral
                               Store, modules, AI,     
                               search                  

  `openframe-desktop` / Tauri  IPC, asset protocol,    Own OS-specific
                               dialogs, platform       behavior
                               integration, logging    

  React                        UI and editing buffers  No OS-specific
                                                       filesystem/database
                                                       logic
  ----------------------------------------------------------------------------

This is better than creating separate Windows and macOS application
architectures.

------------------------------------------------------------------------

# 3. HLD: bounded contexts

OpenFrame should be treated as several bounded contexts connected
through explicit contracts.

## 3.1 Project context

Owns:

-   project lifecycle
-   manifest
-   project location
-   lock
-   schema version
-   migrations
-   recovery
-   recent projects
-   project members
-   settings
-   status

Canonical storage:

``` text
<Project>.openframe/
  manifest.json
  project.sqlite
  assets/
  cache/
```

Only the project context may decide whether a project can be opened.

## 3.2 Creative context

Owns:

-   Idea Vault
-   Story Board
-   characters
-   screenplay
-   drafts
-   revisions
-   comments
-   private notes

The screenplay engine must become its own internal subsystem because
pagination, element semantics, revisions, numbering, locking and export
depend on stable document semantics.

## 3.3 Production context

Owns:

-   breakdown
-   production catalog
-   cast
-   crew
-   locations
-   schedule
-   DOOD
-   call sheets
-   sides
-   reports
-   budget
-   visual planning

Production entities reference screenplay/story lineage through IDs,
never through fragile UI labels.

## 3.4 Interchange context

Owns:

-   FDX
-   Fountain
-   DOCX
-   PDF
-   CSV/XLSX where specified
-   project packages
-   Exchange / Review / Response packages
-   backup/restore workflows

Imported data is hostile input.

## 3.5 Search context

Owns only derived indexes.

``` text
canonical row
   ↓
registered SearchDoc
   ↓
FTS5
```

AI intelligence is separate:

``` text
canonical row
   ↓
semantic document
   ├─ lexical data
   ├─ chunks
   ├─ embeddings
   └─ graph fragment
```

No feature is allowed to make derived search/AI data canonical.

## 3.6 AI context

Owns:

-   local model lifecycle
-   prompt construction
-   tool selection
-   retrieval
-   context assembly
-   Change Set construction
-   proposal validation
-   stale detection
-   AI-specific observability

AI must remain an optional dependency of product functionality.

------------------------------------------------------------------------

# 4. LLD: operation architecture

Every operation should have this shape:

``` text
UI / AI proposal
       ↓
typed Args
       ↓
Registry lookup
       ↓
actor + capability check
       ↓
domain validation
       ↓
Store::mutate
       ↓
transaction
       ├─ canonical SQL
       ├─ undo capture
       ├─ activity
       └─ reindex hints
       ↓
COMMIT
       ↓
events
       ↓
React query invalidation
       ↓
async derived indexing
```

### Query

``` text
useOp()
  → of_invoke()
  → Registry::dispatch()
  → actor.require(View)
  → Store::read()
  → DTO
  → React Query
```

### Mutation

``` text
useCommand()
  → of_invoke()
  → Registry::dispatch()
  → capability
  → validate Args
  → Store::mutate()
      → transaction
      → undo
      → activity
      → search projection
  → commit
  → DataChanged
  → UI invalidation
```

### AI mutation

``` text
user request
  ↓
agent
  ↓
bounded tool
  ↓
resolve references
  ↓
validate command metadata
  ↓
ChangeSetDraft
  ↓
human review
  ↓
local-only Apply Change Set
  ↓
same Registry command
  ↓
same Store::mutate pipeline
```

This is critical: **AI must not have a second mutation path.**

------------------------------------------------------------------------

# 5. LLD: screenplay engine

The screenplay editor should not store formatting as arbitrary HTML.

Use a semantic model:

``` text
Draft
 └─ Scene
     ├─ Scene Heading
     ├─ Action
     ├─ Character
     ├─ Parenthetical
     ├─ Dialogue
     ├─ Transition
     ├─ Shot / custom element
     └─ ...
```

Each element needs:

-   stable ID
-   type
-   text/content
-   sequence/order
-   scene ID
-   draft ID
-   revision metadata
-   lock state where applicable

The editor presentation may be ProseMirror-based, but ProseMirror is not
the canonical model.

### Automatic progression

Use a deterministic state machine:

``` text
current element
    ↓
element type
    ↓
allowed next types
    ↓
keyboard shortcut / Tab / Enter
    ↓
new semantic element
```

Never encode screenplay semantics only in CSS or editor DOM.

### Pagination

Pagination should be a deterministic rendering service:

``` text
ScreenplayDocument
 → normalize
 → page layout model
 → page breaks
 → continuation rules
 → scene/page numbering
 → revision overlays
 → PDF/DOCX renderer
```

The editor may preview pages, but exported pagination must use the same
layout contract.

------------------------------------------------------------------------

# 6. LLD: Story → Screenplay → Breakdown → Schedule lineage

Use stable lineage IDs.

Example:

``` text
story_scene_card.id
      │
      ├── screenplay_scene.lineage_id
      │
      └── production_source_scene.lineage_id
                │
                ├── breakdown elements
                ├── schedule placements
                ├── call-sheet references
                └── visual planning
```

Never identify a scene only by:

-   scene number
-   heading text
-   card title
-   array index

Those values can change.

------------------------------------------------------------------------

# 7. LLD: database strategy

The existing migration architecture should remain.

Rules:

-   one migration per schema evolution unit
-   migrations are append-only
-   no editing historical migrations after release
-   upgrade is preceded by verified safety backup
-   one atomic migration transaction
-   newer-than-supported databases are refused
-   destructive migrations require explicit design review
-   migration tests cover old fixtures
-   project schema and intelligence schema have independent versions

### New table checklist

Every new canonical table needs:

1.  migration
2.  primary key
3.  foreign keys
4.  indexes for actual query paths
5.  deletion semantics
6.  undo behavior
7.  activity behavior
8.  search projection decision
9.  AI indexing decision
10. permission model
11. export/import decision
12. migration fixture
13. test data
14. recovery behavior

------------------------------------------------------------------------

# 8. LLD: filesystem architecture

Use platform-neutral logical locations:

``` rust
AppPaths {
    app_data,
    logs,
    projects,
    global_vault,
    models,
    temp,
}
```

Do not scatter:

``` rust
"C:\\Users\\..."
```

or:

``` rust
"/Users/..."
```

through feature code.

Only the platform path provider knows how these resolve.

### Project path

A project is still a normal folder:

``` text
Title.openframe/
```

The extension is part of the product contract, not a Windows-only
assumption.

------------------------------------------------------------------------

# 9. Windows + macOS architecture

The application should compile from the same workspace but use platform
adapters.

``` text
openframe-platform
  ├─ paths
  ├─ process
  ├─ file_open
  ├─ reveal
  ├─ permissions
  ├─ power/sleep
  ├─ atomic replacement
  ├─ crash recovery
  └─ updater integration
```

Implementation:

``` rust
trait PlatformServices {
    fn app_paths(&self) -> AppPaths;
    fn open_external_file(&self, path: &Path) -> AppResult<()>;
    fn reveal_in_file_manager(&self, path: &Path) -> AppResult<()>;
    fn spawn_managed_process(&self, spec: ProcessSpec) -> AppResult<ProcessHandle>;
    fn platform_capabilities(&self) -> PlatformCapabilities;
}
```

Use:

``` rust
#[cfg(windows)]
mod windows;

#[cfg(target_os = "macos")]
mod macos;
```

but expose the same trait to the application layer.

**Do not put `cfg(windows)` throughout modules.**

------------------------------------------------------------------------

# 10. macOS process and AI sidecar design

The existing AI sidecar is Windows-hardened. macOS must receive
equivalent controls.

Required:

-   absolute executable path resolved by trusted installer/runtime
    directory
-   no shell invocation
-   localhost binding only
-   random per-launch authentication secret
-   random port
-   parent/child lifecycle tracking
-   process termination on app close
-   startup orphan cleanup
-   artifact hash verification
-   signed/notarized distributed sidecars
-   no model execution from arbitrary project paths
-   no project content in process command-line arguments
-   no secrets in logs

macOS should use native process-group/session handling rather than
copying Windows Job Object code.

------------------------------------------------------------------------

# 11. LLD: event architecture

The existing event channel should remain.

Canonical events:

``` text
dataChanged
saveState
projectOpened
projectClosed
task
module
```

New platform events should be additive:

``` text
platform
  { capability, state }
```

Do not create one Tauri command per UI refresh.

React should respond to state-changing events by invalidating the
relevant query.

------------------------------------------------------------------------

# 12. LLD: background tasks

Long operations:

-   imports
-   exports
-   package generation
-   PDF rendering
-   indexing
-   embeddings
-   backup
-   migration preparation
-   AI model downloads

must use `spawn_task`.

Rules:

-   never hold the SQLite writer lock during slow I/O
-   cancellation must be cooperative
-   task state is observable
-   failure maps to `AppError`
-   partial files are temporary
-   final files are committed atomically
-   shutdown cancels tasks
-   tasks never silently mutate UI state

------------------------------------------------------------------------

# 13. LLD: atomic filesystem writes

Use:

``` text
write temporary file
      ↓
flush
      ↓
fsync where appropriate
      ↓
atomic rename/replace
```

For cross-platform correctness:

-   do not assume rename-over-existing behaves identically everywhere
-   provide a platform helper for replacement
-   preserve permissions where required
-   never delete the original until replacement is safely prepared
-   use explicit recovery files for critical exports/manifests

------------------------------------------------------------------------

# 14. HLD: theme system

The revised specification's Theme System should be a presentation-only
subsystem.

``` text
ThemeRegistry
 ├─ canonical OpenFrame Default
 ├─ built-in themes
 ├─ My Themes
 └─ imported themes
```

Theme data:

``` json
{
  "id": "theme-id",
  "name": "Display Name",
  "version": 1,
  "tokens": {}
}
```

Validation:

-   \~10 KB maximum
-   strict schema
-   ID regex `^[a-z0-9-]{3,40}$`
-   bounded numeric values
-   no CSS
-   no JavaScript
-   no URL/import
-   preview before install

Theme tokens must map to semantic CSS variables.

No feature component should need a theme-specific code branch.

------------------------------------------------------------------------

# 15. Security architecture

Security is not a separate feature. It is a cross-cutting constraint.

``` text
Untrusted UI
  ↓
typed IPC
  ↓
allow-listed operation
  ↓
actor/capability
  ↓
validated input
  ↓
safe domain operation
  ↓
canonical transaction
```

For files:

``` text
user-selected path
  ↓
Rust validation
  ↓
type/size/path policy
  ↓
safe parser
  ↓
temporary output
  ↓
validated import
  ↓
canonical mutation
```

For packages:

``` text
package
  ↓
size/entry/path checks
  ↓
archive safety
  ↓
schema validation
  ↓
parse
  ↓
review/preview
  ↓
transactional apply
```

------------------------------------------------------------------------

# 16. Observability architecture

Logs remain local.

Structured event:

``` text
timestamp
operation
duration
result_code
task_id
platform
app_version
```

Do not log:

-   screenplay text
-   private notes
-   contacts
-   file contents
-   model prompts
-   AI retrieved context
-   access tokens
-   absolute user paths unless required for a local diagnostic and
    redacted

Add a local diagnostic bundle generator only if it can prove content
redaction.

------------------------------------------------------------------------

# 17. HLD dependency direction

Allowed:

``` text
domain
  ↑
persistence / project-format / security
  ↑
application
  ↑
desktop adapter
  ↑
React
```

Not allowed:

``` text
React → SQLite
React → arbitrary filesystem
module A → module B SQL
AI → database write
search → canonical ownership
platform code → domain semantics
```

------------------------------------------------------------------------

# 18. Implementation without ecosystem disruption

Do not rewrite the application.

Implement in this order:

### Freeze current contracts

Record:

-   operation names
-   Args/DTOs
-   migrations
-   events
-   filesystem layout
-   IPC commands
-   AI tool inventory
-   existing tests

### Introduce platform service interfaces

No behavior change on Windows.

``` text
existing Windows code
      ↓
WindowsPlatformServices
      ↓
same application APIs
```

Then add macOS.

### Add macOS CI

Do not wait for feature completion.

Minimum matrix:

``` text
Windows x64
macOS arm64
```

### Migrate path and process calls gradually

One subsystem at a time:

1.  paths
2.  file opening/reveal
3.  process management
4.  updater
5.  packaging
6.  diagnostics

### Keep command contracts unchanged

A platform change should normally produce:

``` text
same operation
same Args
same DTO
same domain behavior
different adapter implementation
```

### Introduce feature work using the same architecture

Every new feature passes:

``` text
schema
→ domain
→ operation
→ mutation pipeline
→ search
→ AI metadata
→ UI
→ undo
→ recovery
→ export
→ tests
```

------------------------------------------------------------------------

# 19. Architecture quality gates

A feature cannot be considered complete if:

-   it bypasses `Store::mutate`
-   it adds direct SQL from React
-   it introduces arbitrary filesystem access
-   it creates a second IPC surface without a security review
-   it requires platform-specific logic inside a domain module
-   it cannot be undone when the surrounding feature is undoable
-   it is not covered by migration fixtures
-   it silently breaks import/export
-   it is not reachable from the product UI
-   AI exposure is accidental
-   it adds data to logs
-   Windows and macOS behavior diverge without a documented reason

------------------------------------------------------------------------

# 20. Recommended repository additions

These should be additive:

``` text
crates/
  openframe-platform/
    src/
      lib.rs
      paths.rs
      process.rs
      filesystem.rs
      windows.rs
      macos.rs

apps/desktop/
  src/
    platform/
    theme/

docs/engineering/
  28-cross-platform.md
  29-desktop-hld-lld.md
  30-platform-security.md
```

Do not move current modules unless a measured problem requires it.

------------------------------------------------------------------------

# 21. Definition of done

The architecture is ready when:

-   one canonical data path still exists
-   one IPC data path still exists
-   all current operations remain compatible
-   all existing migrations open successfully
-   Windows behavior is unchanged
-   macOS opens/creates/reopens projects
-   autosave/recovery work on both
-   project folders remain portable
-   imports/exports work on both
-   AI sidecars work on both or fail gracefully without harming core
    functionality
-   updater behavior is platform-correct
-   installers are signed
-   macOS distributions are Developer ID signed and notarized
-   Windows direct packages are Authenticode signed
-   Store packages are independently validated
-   CI exercises both OS families
-   E2E tests cover a complete project lifecycle on both platforms

The goal is not merely "builds on Mac". The goal is **one product
architecture whose invariants remain true on both operating systems**.
