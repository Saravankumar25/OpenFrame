# 18 — Retrieval & Local Intelligence Index

> **Status legend:** ✅ implemented · 🚧 in development · 📋 planned.
> Spec: `openframe_ai_agentic_rag_spec_prompt.md` §8–§22, §31–§34, §42.
> Source: `crates/openframe-search/` (derived index library),
> `crates/openframe-application/src/modules/ai/intelligence/` (documents, graph builders, indexer, retriever,
> assembler, router), `crates/openframe-application/src/modules/ai/retrieval.rs` (contract C2 + ops),
> post-commit hook `Store::set_commit_observer` (`store.rs`) and `AppCore::set_project` (`core.rs`).
> Tests: `crates/openframe-search/src/**` (unit), `crates/openframe-application/tests/ai_retrieval.rs`,
> `crates/openframe-application/tests/ai_retrieval_perf.rs` (§34 measurements).

## 1. What it is ✅

A **derived, rebuildable** retrieval layer for the local AI agent. `project.sqlite` stays the only source of truth;
nothing here is canonical, part of undo/activity, or needed to open a project.

```text
<Project>.openframe/
├─ project.sqlite            canonical (search_doc/search_fts FTS5 stays here, doc 16)
└─ cache/intelligence.sqlite DERIVED — delete any time; rebuilt from project.sqlite
     ├─ index_meta           schema/builder/canonical-schema versions, project id, embedding model id + dim
     ├─ semantic_document    one per domain entity (§13)
     ├─ semantic_chunk       domain-aware chunks (§14) + embedding bookkeeping
     ├─ vec_chunk            sqlite-vec vec0 table (float[dim]) keyed by semantic_chunk.vec_rowid
     ├─ doc_dependency       other canonical rows a document is built from
     ├─ context_node         graph nodes (entity type/id, label, owner)
     └─ context_edge         graph edges (relation, provenance Canonical|Derived|Inferred, origin fragment)
```

No vector server, graph database, Python or network: everything is SQLite inside the process.

## 2. Vector search: sqlite-vec, statically linked ✅

`sqlite-vec` 0.1.9 (MIT OR Apache-2.0) builds its C source with `SQLITE_CORE` and links against the same bundled
SQLite as `rusqlite` 0.37. Verified on Windows MSVC (build + KNN smoke test) before adoption.

- The module is initialised **per connection, only on intelligence connections** (`vector::register`, a direct call
  of `sqlite3_vec_init`). There is no `sqlite3_auto_extension` and no runtime extension loading: project databases
  never gain the module (unit test `module_is_not_global`).
- The rest of the application only sees the `SemanticIndex` trait (`upsert/delete/search/rebuild/get`); swapping the
  implementation (e.g. for an embedded Rust index) touches `openframe-search/src/vector.rs` only.
- Exact brute-force KNN over L2-normalised vectors; similarity = 1 − d²/2 (cosine).

## 3. Opening, versioning and trust ✅

`IntelligenceDb::open` treats the file as **untrusted** (project folders come from anywhere): `PRAGMA quick_check`,
an exact allow-list of schema objects (no triggers/views; vec0 shadow tables only), `trusted_schema=OFF`, then the
identity in `index_meta`: `schema_version` (`openframe_search::SCHEMA_VERSION` = 1), `project_id`,
`canonical_schema_version` (project migration count) and `builder_version`
(`documents::BUILDER_VERSION` = 1). Any mismatch or damage → the file is deleted and recreated, then rebuilt in the
background. A different embedding model id or dimension drops only the vectors (`ensure_embedding_model`) and
re-embeds.

Retrieved text is never taken from this file: the assembler rebuilds every selected item from canonical rows (§8).
A tampered index can therefore only change *which* canonical objects are considered, never their content or
visibility.

## 4. Documents (§13) and chunking (§14) ✅

`intelligence/documents.rs`. One document per logical entity, keyed `kind:id`. Each document starts from the
entity's registered search projection (same liveness, current-draft and private-note rules as Global Search) and is
enriched deterministically:

| Kind | Enrichment | Chunking |
|---|---|---|
| `screenplay_scene` (current draft only) | scene number/draft, characters (cue → linked Story character), location + time parsed from the heading, story day, synopsis, act › sequence of its card | scene first; long scenes split on element boundaries (1,400 chars) |
| `story_act`/`story_sequence`/`story_beat`/`story_scene_card` | act › sequence path, linked characters, source beat/Idea Vault item | card/beat |
| `story_character` | role, relationships, cast, bounded list of its cards | one document |
| `cast_member`, `crew_member`, `location`, `catalog_item` | plays / role+department / status, address, notes / category, status, aliases | one logical entity |
| `scene_breakdown` (per production-source scene) | every non-rejected breakdown element (category, name, suggested flag, notes) | line units |
| `shooting_day` | scheduled scenes (by lineage → current draft), markers | line units |
| `call_sheet` | logical sections: day, scenes, cast, locations, notes | per section |
| `project_note`, `private_note`, `vault_item`, `comment` (with replies), `task` | pinned / owner / type + tags / status + target | paragraphs (1,200) |
| `moodboard(_item)`, `storyboard(_panel)`, `shot`, `side`, `project_file`, `episode`, `project` | scene/board/day links, camera spec, folder, logline… | paragraphs |

Every chunk = `"{Kind}: {title}\n{header lines}\n{piece}"` so an embedding carries its context; per document ≤ 24
chunks. The §14 fields are carried as columns (`chunk_id`, `doc_id`→entity type/id/source table/project/module/
title/owner/source rev/hashes, `embedding_model_id`, `embedded_at`, `metadata_json`).

**Privacy rules in documents.** Private notes carry `owner_user_id` (a private note without an owner is dropped).
Contact details are **never** put into documents (cast/crew/location/catalog contacts, call-sheet emergency
contact, notes keys containing "contact"/"phone"): exact contact lookups belong to deterministic tools. Deleted
objects have no document. Only the current draft's scenes are indexed.

Writes are **hash-guarded**: an unchanged document is a no-op; a changed one re-embeds only chunks whose text changed,
and identical text anywhere re-uses its stored vector.

## 5. Context graph (§16–§17) ✅

Built by the same builders, one **fragment** per entity (`origin_key = doc id`); rebuilding an entity replaces exactly
its fragment. Relations: `appears_in` (character→scene/card), `located_at` (scene→location), `represented_by`
(scene→card), `belongs_to` (scene→draft, card/beat→sequence/act, sequence→act, act→episode), `related_to`
(character→character, type in metadata), `plays` (cast→character), `requires` (scene→catalog item, from confirmed/
manual/suggested breakdown), `scheduled_on` (scene→shooting day), `for_day`, `for_scene`, `part_of`, `depicts`,
`about`, `on`, `derived_from`, `converted_to`, `associated_with`, `replaced_by`.

Provenance: **Canonical** = foreign keys and link rows; **Derived** = deterministic parsing (unlinked cue name ==
character name; heading location == Production location name). **Inferred** is never produced in v1 and is excluded
from traversal. Production/visual objects attach to the *current-draft* scene of the same lineage.

Deleting an entity removes its node and fragment; edges other fragments hold towards it go dormant (traversal joins
on live nodes) and come back on restore. `gc_nodes` drops orphans after a full pass.

Traversal (`graph::expand`): BFS, 1 hop normally, 2 when the question spans modules, ≤ 16 nodes, ≤ 12 edges per node,
relation priorities from the question (schedule words → `scheduled_on`, props → `requires`, …). The permission
callback runs on **every** node: a private node of another user is neither returned nor traversed through.

## 6. Incremental indexing (§15, §32) ✅

```text
Store::mutate / undo / redo / restore / purge / package apply
  └─ commit ─► writer lock released ─► CommitObserver::committed(changes with row images, extra reindex pairs)
                 └─ documents::invalidate (pure: table/id + parent ids from the before/after images) ─► queue
worker thread (per open project):
  debounce 750 ms after the last change (max 5 s under continuous typing), coalesced by document key
  own query_only connection to project.sqlite → build documents + fragments (64 per intelligence transaction)
  own connection to intelligence.sqlite → upsert/delete (hash-guarded)
  embeddings: 16 chunks per call, never while any project connection/lock is held; yields to newly due edits
```

- The canonical writer transaction never waits for indexing; a failing index never fails a save
  (`spec_15_saves_never_wait_for_embedding…`, `spec_33_embedding_failure_never_blocks_saves…`).
- Parent resolution uses the undo capture's row images, so e.g. undoing an element insertion still re-indexes its
  scene. Draft switches, production-source changes, character-link changes and schedule changes re-synchronise the
  affected kinds; renaming/deleting a Story character or Production location re-derives scene edges.
- Delete → document, chunks, vectors, node and fragment removed after commit. Restore → rebuilt. Undo/redo → same path.
- Migration → `canonical_schema_version` differs → rebuilt. Project open → hash-guarded full reconcile in the
  background (catches changes made while the index was closed, restored checkpoints, copied folders).
- **Lazy activation.** The worker starts at project open when Offline AI is installed, else on the first retrieval,
  `ai.index_status` or `ai.index_rebuild`. Before activation the observer records nothing (activation reconciles).
- Close: `AppCore::set_project` detaches the observer and stops the worker, waiting until it released its files.
- An embedding runtime failure pauses embeddings (retry after 30 s); documents and graph stay current.

States (`retrieval::IndexState`): `Current`, `Updating` (queued/running work or embeddings pending), `Stale` (not
started), `Rebuilding` (fresh/recreated/explicit rebuild), `Failed` (last pass failed; previous data kept),
`Unavailable` (no project / file cannot be opened).

## 7. Retrieval (contract C2, §18–§21, §33) ✅

`modules/ai/retrieval.rs`:

```rust
pub struct RetrieveRequest { query, scope: Option<ResolvedScope>, limit /* ≤ 24 */, route: Option<RetrievalRoute> }
pub enum RetrievalRoute { Structured, Lexical, Semantic, Hybrid, HybridGraph, ProductHelp }
pub struct ContextPacket { items: Vec<ContextItem>, provenance: Vec<Provenance>, index_state, route, degraded,
                           sources: Vec<ContextSource> /* entity ref + nav + via */, notice: Option<String> }
pub fn route_for(query) -> RetrievalRoute;  pub fn retrieve(core, actor, req) -> AppResult<ContextPacket>;
pub fn index_status(core) -> IndexState;
```

`items[i]`, `provenance[i]` and `sources[i]` describe the same item. Ops: `ai.index_status` (query → `AiIndexStatus`
with state, "Preparing project context…" / degraded wording, counts, progress) and `ai.index_rebuild` (command →
`spawn_task("ai.index_rebuild")`, returns the task id; discards and rebuilds only derived data).

Pipeline:

1. `UseAi` + `View` required (AI authority ≤ the user's); query ≤ 2,000 chars; limit clamped to 1..=24.
2. **Router** (`router.rs`, deterministic): product how-to → `ProductHelp` (curated guide, `knowledge.rs` unchanged,
   §22); counts/lists/status/assignee/"open scene" → `Structured` (explicit scope only — exact facts come from SQL
   tools, §43); two or more product areas or relationship wording → `HybridGraph`; meaning/theme questions and
   mutation requests → `Hybrid`; short lookups / quoted text → `Lexical`.
3. **Lexical**: the canonical FTS5 index with an OR query of the question's significant terms (quoted, prefix,
   stop-words removed; Global Search keeps its AND query), owner-filtered, BM25 (title 4×).
4. **Semantic**: query embedding (LRU cache of 64) → vec0 KNN k=48 → chunks → documents; owner and draft filters.
5. **Graph** (HybridGraph): expand from the top 5 fused entities (§5 limits).
6. **Rank** (`openframe_search::rank`): RRF (k=60) over lexical + vector ranks, + scope 0.02, exact-name 0.016,
   graph ≤ 0.012 × score, current draft 0.004. No LLM reranker.
7. **Assemble** (`assembler.rs`): explicit scope items first; then each candidate is **rebuilt from canonical rows**
   (drops deleted, non-current-draft, other users' private and out-of-scope items), the semantic chunk or the chunk
   sharing most query terms is chosen, duplicates dropped, labels added ("Screenplay · Scene 12 — INT. …").
   Budgets: 6,000 chars total, 1,500 per item, graph-related entities as ≤ 320-char summaries (≤ 6).

Agent integration ✅: `orchestrator::run_agent` calls `retrieve` once per request (route from the router, scope =
the resolved scope, 8 items) and `orchestrator::assemble_context` appends the packet's items after the explicit
scope under the same `CONTEXT_BUDGET`, de-duplicated, inside `<project_data trust="untrusted">`. A model-written
answer cites up to 6 retrieved sources as provenance chips (`Provenance.nav` opens the source). Retrieval errors
never fail the request (the scope alone is used). The model can also call the `retrieve_context` tool during the
loop for a further, differently worded retrieval (toolbox, `run_tool_unlocked`).

Fallbacks (§33): no embedder / vectors not ready / vector error → FTS + SQL, `degraded = true` and `notice` = "AI can
still answer some questions while project context is being prepared."; FTS error → SQL tools + explicit scope; index
unavailable → the same, never an error for the user. Retrieved text is data; the orchestrator keeps it inside the
untrusted-data boundary.

## 8. Embeddings (contract C1) ✅

`openframe_search::Embedder` (`model_id` incl. version + hash prefix, `dim`, `embed`). The production adapter
`AiManagerEmbedder` (`intelligence/mod.rs`) wraps `openframe_ai::AiManager::{embedding_info, embed}`: the installed
Offline AI embedding model (BGE small English v1.5, 384 dimensions, served by the embedding sidecar) with model id
`<model id>@<version>#<first 12 hex of SHA-256>`. `IntelligenceService::embedder` uses an explicitly attached embedder
first (`set_embedder`, tests), else this adapter; when Offline AI is not installed there is none, documents and graph
are still built and retrieval runs FTS + graph + SQL. A different embedding model (for example after an Offline AI
update) has a different model id, so the stored vectors are dropped and re-embedded (§3). Tests use `openframe_test_support::embedder::ConceptEmbedder`
(deterministic feature hashing + small synonym groups) — never shipped.

## 9. Measurements (§34)

Generated fixture: 240 scenes × 12 elements, 40 characters, 60 locations, 300 catalog items, 720 breakdown elements,
30 shooting days, 240 cards, 150 notes/tasks/ideas (`tests/ai_retrieval_perf.rs`, test embedder dim 384).

Measured 2026-09-30 on the development machine (Windows 11, debug test profile: OpenFrame crates at opt-level 0,
dependencies at opt-level 2 — release builds are faster). Index: 1,302 documents, 1,302 chunks, 1,063 graph nodes,
2,839 edges.

| Measurement | p50 | p95 | Budget (§34) |
|---|---|---|---|
| Initial full index build (documents + graph + embeddings, background) | 4.72 s total | — | async, never blocks |
| FTS retrieval (lexical route, end-to-end incl. canonical re-validation + assembly) | 15.0 ms | 17.1 ms | < 150 ms |
| Vector search (k = 48, incl. chunk/document lookup) | 7.4 ms | 7.7 ms | < 100 ms |
| Graph expansion (5 seeds, 2 hops, capped) | 1.6 ms | 1.8 ms | < 50 ms |
| Hybrid retrieval + context assembly | 23.5 ms | 43.3 ms | < 250 ms |
| Hybrid + graph retrieval + context assembly | 26.7 ms | 35.8 ms | < 250 ms |
| Save latency while indexing/embedding runs (embedder slowed to 40 ms/call) | 2.6 ms | 6.2 ms | saves never wait |
| Last keystroke → indexed + embedded (debounce 0 in the test; 750 ms in the app) | 113 ms | — | background |

Query-embedding time of the real embedding model is not included (the test embedder is instantaneous); repeated
queries hit the LRU cache. The test asserts the budgets as written in release builds and 3× in debug builds.

## 10. Known limits 📋

- Scenes of non-current drafts are not indexed; a "Specific Draft" scope gets its outline from the scope resolver and
  other modules' context, and never another draft's scenes.
- The Global Idea Vault (separate store) is not part of the project index; project Idea Vault items are.
- Exchange review records and templates are not indexed.
- The retrieval evaluation harness (Recall@K/MRR, FTS vs vector vs hybrid vs hybrid+graph, §35) does not exist yet.
  The only quality evidence today is `spec_19_hybrid_finds_meaning_that_keyword_search_misses` (test embedder);
  retrieval quality with the real BGE model has not been measured.
- Query-embedding latency of the real embedding model is not part of the §9 measurements.
