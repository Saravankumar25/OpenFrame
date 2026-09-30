# OpenFrame Studio — Module Engineering Brief (read fully before coding)

You are implementing one vertical module of OpenFrame Studio, a local-first Windows 11 desktop app
(Tauri v2 + React 19/TypeScript + Rust + SQLite). The foundation already exists and is tested. Your
job: build your module **end-to-end** (schema → Rust commands/queries → search/trash/undo → TS UI →
tests) to production quality, following the specs. No mocks, no fake data, no buttons that do nothing,
no setTimeout pretending to work. Test fixtures only in tests.

## 0. Authority
Specs in repo root (authoritative): PRD, FSD, UX/UI, Domain/Data, AI, Import/Export, Offline/Collab,
Security. Dense digests: `docs/spec-digest/{fsd-part1,fsd-part2,prd,ux,ai,security,import-export,collaboration}.md`.
Read the digest sections for your module AND grep the full spec for details. Mockups:
`HTML_Mockups/NNN-*.html` (open the ones for your screens; reuse their CSS classes and exact labels).
Hierarchy: PRD > FSD > UX/Domain/AI/Security/etc > engineering. Approved build decisions: LOCAL AI ONLY
(no cloud, no API keys, no Ollama), no accounts, no telemetry, SQLite+files. LAN / local-network
collaboration is REMOVED from scope (product-owner decision 2026-09-30): collaboration is file-based
only via exchange/review/response packages. Ignore every LAN/session/presence/soft-lock requirement in
the specs. DTO `rev`/`position`/count fields that are i64 need `#[ts(type = "number")]`.

## 1. Environment / process
- Windows, PowerShell. Rust at `$env:USERPROFILE\.cargo\bin` — prefix commands with
  `$env:Path = "$env:USERPROFILE\.cargo\bin;$env:Path";`.
- You work in your OWN git worktree/branch. Commit your work to your branch when done (clear message,
  end with `Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>`). Do not push, do not merge.
- A "GateGuard" hook blocks the FIRST Write of any new file until you state: who imports/calls it, API,
  data schema, and the user's instruction ("Build OpenFrame Studio into a production-grade Windows 11
  desktop application."). Just state those facts in your message and retry the same Write. Tip: for a
  big new file, first Write a one-line placeholder (gate), then Write the full content.
- Build Rust only for the crates you need: `cargo test -p openframe-application` (does NOT compile Tauri).
  Frontend: `cd apps/desktop; npx tsc --noEmit -p .; npx eslint src --max-warnings 0; npx vitest run`.
  Run `npm install` at repo root first if `node_modules` is missing in your worktree.
- Regenerate TS types after changing Rust DTOs: `cargo test -p openframe-application export_bindings`
  (writes `apps/desktop/src/ipc/generated/*.ts`; commit them).

## 2. Files you OWN (create/edit freely)
- `crates/openframe-application/src/modules/<your_module>/` (dir with `mod.rs` + submodules) — or the
  specific crate named in your assignment.
- Your migration file(s) under `migrations/` (listed in your assignment). Pre-release: you may edit them.
  HUB TABLE columns (see §5) are contracts: add columns/tables, never rename/remove contract columns.
- `crates/openframe-application/tests/<your_module>*.rs`
- `apps/desktop/src/workspaces/<your-dir>/` (default export from `index.tsx` = workspace root; discovered
  automatically by `app/routes.tsx` via glob — do NOT edit routes.tsx).
- `apps/desktop/src/api/<your_module>.ts` (typed wrappers, optional).

## 3. Files you may touch minimally (expect merge by lead)
- `crates/openframe-application/src/modules/mod.rs`: add exactly `pub mod <m>;` and `<m>::register(r);`.
- `crates/openframe-application/Cargo.toml` / workspace `Cargo.toml`: only if you truly need a new dep
  (prefer existing workspace deps: rusqlite, serde, regex, similar, quick-xml, zip, image, pdf-writer,
  lopdf, csv, rust_xlsxwriter, sha2, reqwest, tokio, sysinfo, time…). Justify any new dependency
  (maintained, permissive license MIT/Apache/BSD/ISC/Zlib, small).
Everything else (store.rs, core.rs, registry.rs, util.rs, shell, design-system, other modules): DO NOT
edit. If you believe a foundation change is required, implement a local workaround and describe the
needed change in your final report.

## 4. Backend conventions (crates/openframe-application)
- Every op = `fn(&AppCore, &Actor, Args) -> AppResult<R>`; register with `r.query("mod.action", f)` or
  `r.command(...)`. Args structs: `#[derive(Deserialize, TS)] #[ts(export)] #[serde(rename_all="camelCase",
  deny_unknown_fields)]`. DTOs: `#[derive(Serialize, TS)] #[ts(export)] #[serde(rename_all="camelCase")]`;
  use `#[ts(type = "number")]` for i64 timestamps.
- Reads: `core.project()?.store.read(|c| ...)` (after `actor.require(Capability::View, "...")`).
- Mutations: ONLY via `store.mutate(actor, MutationMeta::new("mod.action", human_summary, Capability::Edit)
  .target("type", &id) [.coalesce(key) for typing]`, |tx| { ... })`. Inside, use `tx.conn()` SQL.
  The pipeline gives you transaction + permission + generic undo/redo + activity + search reindex + events.
  Undo is automatic (row images captured by triggers) — never write inverse logic.
- Identity: `openframe_domain::new_id()` (UUIDv7). Timestamps `now_ms()`. Canonical tables: `id TEXT PK,
  created_at, updated_at, rev INTEGER DEFAULT 1, deleted_at` (+`position` for user order). NO BLOB
  columns, NO `ON DELETE CASCADE`. Infra tables prefixed `sys_`/`search_` are not undo-tracked.
- Updates: `openframe_persistence::rows::update_fields(conn, table, id, &[(col, val)], ALLOWED_COLS,
  expected_rev, "what")` (bumps rev/updated_at; optional optimistic `expected_rev` → `conflict.stale`).
  Ordering: `rows::{sibling_ids, next_position, place_at, renumber}`.
- Delete: `store::soft_delete(tx, DeleteSpec{..})` (recoverable, appears in Recently Deleted) + register
  `r.trash_handler(TrashHandler{ object_type, table, label, restore: Option<fn>, purge: fn })`. Restore
  must return to prior parent/position or to "unassigned" and say so (FSD §52.5). Purge deletes rows it
  exclusively owns; use `modules::files::purge_asset_if_unreferenced(tx, asset_id)` for files.
- Search: `r.indexer("table", fn(&Connection, &str id) -> AppResult<Option<SearchDoc>>)`; return None for
  deleted rows. `SearchDoc{entity_type (table name), title, body, context ("Story", "Screenplay",
  "Production", "Idea Vault"…), nav: json!({"workspace": "...", "sub"?: "...", "<x>Id": id}), owner_user_id}`.
  Use `tx.reindex(table,id)` when a doc depends on other rows (e.g. scene doc from element text).
- Files/media: `util::{ingest_file (managed copy), ingest_bytes, reference_external, load_asset,
  asset_file_path, copy_asset_between}`; asset rows are shared across modules; reference them via
  `*_asset_id` columns. Frontend shows files via `convertFileSrc(asset.path)` from `@tauri-apps/api/core`.
- Errors: `AppError::{required, invalid_input, validation, not_found, conflict, stale, permission_denied,
  locked_draft, private_content, import(code,msg), export(code,msg), ai(code,msg)}` — messages are
  filmmaker-friendly, no SQL/OS jargon; technical detail via `.with_detail()` (never project content).
- Long work (import parse, export render, downloads): `core.spawn_task(kind, label, |core, h| {...})`
  returns a task id; report `h.progress(f, msg)`, honour `h.check()?` cancellation; result delivered as
  `AppEvent::Task`. Keep short work synchronous.
- Permissions: choose the right Capability (View/Edit/SoftDelete/PermanentDelete/Comment/ResolveComments/
  LockOrFinalize/ManageProject/Export/CreatePackage/Import/ApplyChangeSet). Private notes: always filter
  `owner_user_id = actor.user_id`.
- Source-of-truth rules are absolute: no hidden sync between Vault/Story/Screenplay/Production/Schedule/
  Call Sheet. Copies create new identities. Production never rewrites screenplay text. Call sheet edits
  never rewrite the schedule. AI never mutates directly.
- Tests: `crates/openframe-application/tests/<module>.rs` using `openframe_test_support::TestEnv`
  (`TestEnv::with_project(title, "Feature Film")`, `env.ok(op, json!)`, `env.err(op, json!) -> code`,
  `env.undo()`, `env.redo()`, `env.actor_with_role(Role::Viewer)`, `env.restart()`,
  `env.crash_and_restart()`, `env.write_file(name, bytes)`). Cover: happy paths, validation, undo/redo,
  delete/restore/purge, permissions (Viewer/Commenter denied), search hits, persistence across restart,
  and every FSD acceptance criterion for your module (name tests after the requirement, e.g.
  `fsd_story_003_reorder_keeps_identity`).

## 5. Hub tables (contracts; see migrations for full DDL)
- core (0001): project, project_member, season, episode, deleted_item, asset, project_file(_folder),
  snapshot, project_note, task, comment (target_type/target_id/scene_id/anchor_json, parent_id replies,
  status Open/In Discussion/Resolved, review_round_id), private_note, template, sys_activity, sys_undo,
  sys_view_state, search_doc/search_fts.
- story (0003): story_act, story_sequence, story_beat, story_scene_card(parent_type act|sequence|parking|
  unassigned, screenplay_scene_id), story_character.
- screenplay (0004): screenplay(current_draft_id), screenplay_draft(status Draft|Review|Locked|Revision,
  created_from_draft_id), screenplay_scene(draft_id, lineage_id = stable identity across drafts,
  position → display number, heading, synopsis, story_day), screenplay_element(scene_id, position,
  element_type, text).
- production (0005): production_source(draft_id, active), catalog_item, breakdown_element(source_id,
  scene_id, scene_lineage_id, category, catalog_item_id, confirmation_state), location, cast_member
  (character_id), crew_member.
- Other modules own their tables (visual 0006, schedule 0007, ai 0008, collaboration 0009,
  idea vault 0002 project + global 0002).

## 6. Frontend conventions (apps/desktop/src)
- Data: `useOp<T>(op, args, tables[], {store?})` — `tables` = every table the query reads (drives
  automatic invalidation from Rust `dataChanged` events). Mutations: `useCommand<A,R>(op, {onSuccess})`
  (errors auto-toasted) or `call(op, args)` + `reportError`. Never keep project truth in React state
  beyond editing buffers. Types from `../../ipc/generated/*`.
- UI kit: `design-system/index.tsx` (Button, IconButton, Field, TextInput, TextArea, Select, Segmented,
  Checkbox, Chip, Banner, Dot, EmptyState, Skeleton, PageHeader, Dialog, ConfirmDialog, Drawer, Menu,
  ContextMenu) + mock CSS classes from `styles/openframe.css` (.card, .h4, .row, .grid, .tbl, .banner,
  .chip, .toolbar, .subnav, .vcard, .board, .actband, .seq, .sc, .beat, .strip, .daycol, .paper…).
  Reuse exact labels/texts from the UX spec + mockups. Icons: lucide-react.
- Navigation: `useNav()` → `route {workspace, sub?, params?}`; workspaces read `route.sub`/`route.params`
  (search/Continue pass ids like `sceneCardId`). Quick-action intents: `useIntent().consume("story.new_act")`.
- Toast with undo: `toast.undoable("Moved Scene Card …")`. Global Ctrl+Z/Ctrl+Y/Ctrl+S/Ctrl+K are
  handled by the shell (not inside text inputs). Set `useUi().setUndoScope("global")` when showing the
  Global Idea Vault.
- Drag & drop: @dnd-kit (core/sortable) — installed. Large lists: @tanstack/react-virtual. Rich text/
  structured editing: prosemirror-* (installed).
- Accessibility: keyboard reachable, visible focus, labels on icon buttons, never colour-only status.
- File pickers: `import { open, save } from "@tauri-apps/plugin-dialog"` → pass chosen path to a Rust op
  which validates it. No fs/shell/http plugins in the webview (lint-enforced).
- Frontend tests (vitest + @testing-library/react) for non-trivial pure logic/components.

## 7. Definition of done for your module
All FSD behaviours for your module implemented through all layers; `cargo test -p openframe-application`
green (all tests, not just yours); `npx tsc --noEmit -p .` and `npx eslint src --max-warnings 0` clean;
bindings regenerated; committed on your branch. Final report (concise): what's implemented (by FSD
section/ID), ops added, tables added, tests added, anything NOT done and why, any foundation changes needed.
