# 16 — Search & Retrieval

> **Status legend:** ✅ implemented · 🚧 in development · 📋 planned.
> Source: `crates/openframe-application/src/modules/search.rs`, `store.rs` (`reindex_rows`,
> `rebuild_search_index`), `migrations/project/0001_core.sql` (`search_doc`, `search_fts`),
> `apps/desktop/src/app/shell/SearchOverlay.tsx`.

## 1. Model ✅

Search is **exact, local, non-semantic** full-text matching (FSD §41; PRD N-13) over *projections* of canonical rows.
The index is never canonical and can always be rebuilt.

```text
canonical row ──(module indexer: fn(&Connection, id) -> Option<SearchDoc>)──► search_doc ──(triggers)──► search_fts (FTS5)
```

- `search_doc(rowid, source_table, entity_id, entity_type, title, body, context, nav_json, owner_user_id, updated_at)`,
  unique `(source_table, entity_id)`.
- `search_fts` is an FTS5 external-content table over `title`, `body`, tokenizer `unicode61 remove_diacritics 2`,
  `prefix='2 3'`. It is kept in sync by `AFTER INSERT/UPDATE/DELETE` triggers on `search_doc`.
- `SearchDoc.context` is the human area ("Story", "Screenplay", "Production", "Idea Vault", "Files"). `nav` is an
  opaque JSON target the UI understands (`{workspace, sub?, <x>Id}`).

## 2. Indexing ✅

- Each module registers `r.indexer("<table>", fn)`. The indexer returns `None` for deleted, empty or non-searchable
  rows, which removes them from the index.
- **Inside every mutation** (`Store::mutate`, undo/redo): every row touched (from the undo capture) plus explicit
  `tx.reindex(table, id)` requests is re-indexed **in the same transaction**. Search can never show a state that was
  rolled back, and results reflect a change as soon as it commits.
- `tx.reindex` is for derived documents. Example: a scene's document is built from its elements' text, so editing an
  element re-indexes the scene.
- **Full rebuild** (`Store::rebuild_search_index`, op `search.rebuild`): runs on open when the index is empty or after
  a migration, and on demand. It deletes all `search_doc` rows and re-indexes every row of every indexed table.

Indexed today ✅: `project_file` (Files). 🚧 Each module adds its tables (vault items, story cards and beats,
characters, screenplay scenes, catalog items, locations, cast and crew, notes, comments, …).

## 3. Query ✅ (`search.query`)

Args: `text` (≤ 500 chars), `includeGlobal` (also search the Global Idea Vault), `globalOnly`, `entityTypes?`
(restrict for workspace-local search boxes), `limit` (default 50, max 500).

1. **Safe FTS syntax.** `fts_query` splits on whitespace, strips surrounding punctuation, takes at most 12 terms,
   quotes each (`"term"*`, a prefix match) and joins them with `AND`. User input can never inject FTS operators
   (`NEAR`, `OR`, column filters). This is unit-tested.
2. Runs on the reader connection: `search_fts MATCH ?` joined to `search_doc`,
   **`owner_user_id IS NULL OR owner_user_id = actor.user_id`** (Private Notes and other private documents are visible
   only to their owner, Security §10.2), ranked by `bm25(search_fts, 4.0, 1.0)` (title weighted 4× body).
3. `snippet()` returns an excerpt with matches wrapped in `\u0001 … \u0002`, which the UI renders as highlights.
   Project and global results are merged by rank, then truncated to the limit.
4. `actor.require(View)`. An empty or punctuation-only query returns `[]`.

UI ✅: **Ctrl+K** opens the global overlay (keyboard navigation ↑/↓/Enter, grouped by context, navigates via `nav`).
**Ctrl+F** is contextual find inside each workspace (ADR-0012 §6).

## 4. Budgets

Search p95 < **150 ms** for a 120-page screenplay project with typical story and production data (21-performance-
budgets.md). FTS5 with prefix indexes meets this with a wide margin, and the budget is verified by the planned
performance harness.

## 5. Planned 📋

- **In-script find** (screenplay Ctrl+F) runs over the open draft's elements in the editor, not over the global index.
  It supports Enter/Shift+Enter/Esc and replace (undoable, atomic).
- **AI retrieval** reuses FTS as the retrieval stage (Local AI spec §10). Retrieved text is passed to the model as
  *data*. Optional semantic (embedding) retrieval is later scope and would be an additional rebuildable index, never
  canonical.
- Non-Latin tokenization: `unicode61` handles most scripts word-by-word. CJK text has no word boundaries, so CJK
  search quality is a known limitation until a dedicated tokenizer is chosen.
