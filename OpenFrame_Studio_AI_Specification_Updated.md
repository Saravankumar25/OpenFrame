# OpenFrame Studio — AI Specification

**Document type:** Cross-cutting AI product, safety, data-access, retrieval, tool, approval and persistence specification

**Product:** OpenFrame Studio

**Primary platform:** Windows 11 desktop application (Tauri v2 + React/TypeScript + Rust + SQLite)

**Operating model:** Local-first, offline-capable, user-owned project files; **one optional local AI** (Offline AI) that
runs on the user's computer; no external AI, no API keys, no accounts, no telemetry.

**Updated:** 30 September 2026 — rewritten for the implemented architecture: one Gemma 3 1B Offline AI, local
embeddings, a derived `intelligence.sqlite`, hybrid retrieval (FTS5 + sqlite-vec + context graph), a deterministic
router, a universal toolbox, a bounded agent loop and Change Sets with human-only approval.

**Status:** Authoritative. It describes what the code on `master` does. Where something is not built yet it is listed
in §20 ("Not implemented / known limits"), never described as if it were.

**Supersedes:** the previous AI specification baseline (external-provider disclosure, provider/model choice,
"model-agnostic" replacement of the model by the user, LAN-session AI scope). Those passages no longer apply (§21).

**Related documents**

| Topic | Document |
|---|---|
| Runtime, download, manifest, sidecars, model choice benchmark | `OpenFrame_Local_AI_Runtime_Model_Management_Specification.md` |
| Derived index, embeddings, graph, retrieval pipeline, measurements | `docs/engineering/18-retrieval-intelligence-index.md` |
| Canonical full-text search (FTS5) | `docs/engineering/16-search.md` |
| Every registered operation, its AI exposure and the tool that covers it | `docs/engineering/ai-tool-coverage.md` (generated) |
| Threats and mitigations | `docs/engineering/20-security-threat-model.md` §2 (T17, T18, T25–T33) |
| Decisions | ADR-0006 (sidecar, signed manifest), ADR-0013 (single model, hybrid RAG, toolbox, agent loop) |
| Source specification of this architecture | `openframe_ai_agentic_rag_spec_prompt.md` |

---

# 1. Product definition

The OpenFrame AI is an **intelligent local control layer over the product**, not a separate chatbot. It answers
questions about the open project, counts and finds things, opens workspaces and objects, writes short summaries and
suggestions, and **prepares** changes that the user reviews. It is:

> A deterministic filmmaking application with an intelligent local agent on top.

The database and application core own truth. Retrieval helps the model find context. The model understands intent and
chooses tools. Tools perform exact reads. Change Sets represent proposed writes. **The human owns the final decision.**

OpenFrame is fully usable if AI is never installed. Nothing outside the AI module depends on it
(`fsd_ai_001_openframe_works_fully_without_offline_ai`).

## 1.1 One model, no choice

There is exactly one production AI profile, internal id `openframe-local-ai-v1`:

| Component | What it is | Role |
|---|---|---|
| Language model | Google **Gemma 3 1B Instruct**, GGUF Q8_0 | Chooses tools, writes short answers, summaries and suggestions |
| Embedding model | **BGE small English v1.5**, GGUF Q8_0, 384 dimensions | Turns project passages and questions into vectors for meaning-based retrieval |
| Runtime | Pinned `llama.cpp` `llama-server` (CPU or Vulkan build) | Two sidecar processes on 127.0.0.1, one per model |

The user never picks a model, a quantization or a runtime. Component names appear only in Settings → Offline AI →
Technical details, third-party notices, the privacy policy, the user manual and engineering documents (runtime spec §1).

## 1.2 Core promise

- AI runs on this computer. Project data is never sent to OpenFrame, Google or any AI service.
- The AI can do only what the current user can do: **AI authority ≤ user authority**.
- The AI never changes the project on its own. Every change is a proposal the user applies with **Apply Changes**.
- There is **no auto mode**: no "always allow", no remembered approval, no background writes by the AI.
- Exact facts come from OpenFrame code, never from the model.

---

# 2. Non-negotiable rules (enforced in code)

| # | Rule | Where it is enforced |
|---|---|---|
| R1 | Read, search, compute and navigate run immediately, as the requesting user | `toolbox::run_tool` checks the capability of every tool as the real actor |
| R2 | Mutation is never executed while planning; it becomes a `ChangeSetDraft` | Proposal tools return `ToolStep::Proposal`; nothing is dispatched (`proposal_tools_only_build_drafts_and_never_write`) |
| R3 | Only a local user action in the review card accepts, rejects or re-checks a Change Set | `change_set::require_human` refuses every actor whose origin is not `ActorOrigin::Local` (`acceptance_is_refused_for_anything_but_a_local_user_action`) |
| R4 | No tool, tool list or Change Set can contain an `ai.*` operation (so no nested or recursive acceptance) | `toolbox::op_exposed`, `change_set::accept_with` (`a_change_set_can_never_contain_an_assistant_operation`, `no_tool_can_review_apply_or_run_assistant_operations`) |
| R5 | Applying revalidates permission, row revisions and target existence first | `change_set::revalidate` → `Stale` / `Conflict`, never applied (`ai_ac_014_…`, `deleted_target_makes_the_change_set_a_conflict`) |
| R6 | Applied operations run through `AppCore::dispatch` as ordinary commands (permission, validation, undo, activity, search) | `change_set::accept_with` step 4 |
| R7 | The model never sees SQL, file-system paths or a shell, and never names raw registry operations | Tools are domain-named; path-taking and path-returning operations are hidden (`ai-tool-coverage.md`) |
| R8 | Project text, retrieved passages, tool results and conversation history are data, never instructions | Delimited, escaped blocks (§9) |
| R9 | Another user's private notes never reach the model, a tool result, retrieval or provenance | Owner filters in documents, retrieval, tools and provenance links (`ai_ac_013_…`, `spec_20_private_notes_…`, `other_users_private_notes_never_reach_a_tool`) |
| R10 | Permanent deletion ("Delete forever") is never offered to the AI | `trash.purge`, `vault.purge`, `project.delete`, `packages.record_delete` are hidden |
| R11 | The AI never becomes a privilege-escalation path | Tools filtered by role and re-checked on execution; apply-time capability check per operation (`tools_are_filtered_by_role_and_enforced_when_run`, `a_viewer_asking_for_a_change_gets_a_denial_and_no_change_set`) |
| R12 | An assistant request cannot be started on behalf of an applied Change Set | `orchestrator::ask` refuses `ActorOrigin::Ai` |

Prompt wording is defence in depth only. Tool permissions and proposal-only mutation are the security boundary.

---

# 3. Architecture

```text
React (AI panel: conversation, sources, Proposed Changes)        Settings → Offline AI
      │ of_invoke: ai.ask · ai.history · ai.change_set.* · ai.index_status        │ ai.status/setup_info/install/…
      ▼                                                                         ▼
openframe-application  modules/ai
  orchestrator.rs   bounded agent loop (one request → one AI Result)
  toolbox.rs        universal toolbox (contract C3): read tools + proposal tools, strict schemas
  catalog.rs        proposal builders → ChangeSetDraft
  change_set.rs     compose / insert / accept / reject / recheck (human-only)
  retrieval.rs      contract C2: route_for · retrieve · index_status · ai.index_rebuild
  intelligence/     documents, graph builders, indexer worker, retriever, assembler, router
  scope.rs          explicit UI scope → ResolvedScope (+ "Using:" label)
  records.rs        ai_request / ai_result / ai_tool_invocation persistence
  knowledge.rs      product guide (knowledge.md) keyword section retrieval
      │                                     │
      │ AppCore::dispatch (only after        │ derived, rebuildable
      │ human approval)                      ▼
      ▼                               openframe-search → <Project>.openframe/cache/intelligence.sqlite
Registry → Store::mutate → project.sqlite (canonical)
      ▲
openframe-ai  AiManager: manifest · download · store · supervisor (chat + embedding sidecars) · ChatModel · embed()
```

Dependency direction is preserved: React is presentation and input only; the application layer owns the agent,
tools, Change Sets and context assembly; `openframe-search` owns the derived index; `openframe-ai` owns the runtime,
downloads and inference adapters.

## 3.1 Operations

| Operation | Kind | Purpose | Exposed to the model |
|---|---|---|---|
| `ai.status`, `ai.setup_info`, `ai.diagnostics` | query | Offline AI state, exact download plan, technical details | No |
| `ai.install`, `ai.cancel_install`, `ai.uninstall`, `ai.stop_runtime` | command | One-click install (background task), pause/cancel, remove, free memory | No |
| `ai.ask` | command | One request → one bounded agent run → one AI Result | No (it is the entry point) |
| `ai.history` | query | The user's own conversation history | No |
| `ai.change_set.get` / `accept` / `reject` / `recheck` | query / command | Human review of a proposal | No (human-only boundary) |
| `ai.index_status` / `ai.index_rebuild` | query / command | Project-context state; rebuild derived data as a background task | No (maintenance) |

---

# 4. Offline AI installation (summary)

Full contract: runtime spec §1–§9, §14–§15.

- **Not installed:** the AI panel shows "Offline AI is not installed." with **Enable Offline AI** — "Download the AI
  components required for OpenFrame's AI features. AI runs on this computer and project data is not sent to OpenFrame
  or an AI cloud service." — the exact download size, bytes already downloaded, free space needed and available, and
  whether it will run on the graphics card or the processor. One button: **Download Offline AI**.
- **Installing:** Checking device → Downloading → Verifying → Installing → Starting → Ready, one combined progress bar,
  **Pause** (keeps partial files, resumes later) and **Cancel** (discards them). Work continues meanwhile.
- **Ready:** the header chip reads **AI Ready** (tooltip "Runs on this computer"). No restart is needed.
- The download is 1,125,156,323 bytes on a processor-only computer (runtime + Gemma + BGE). It is fetched only when the
  user clicks the button (or **Update Offline AI**); the signed manifest is fetched at that moment too.
- **Remove Offline AI** (panel menu or Settings, with confirmation) stops both sidecars and deletes every model,
  runtime and partial download. Projects and assistant history are unaffected.
- The chat sidecar starts on the first request; the embedding sidecar starts when the background indexer first needs
  embeddings (for example after a project is opened with Offline AI installed). Both unload after 15 idle minutes.
  **Stop Offline AI (free memory)** stops them at once.

---

# 5. Scope and context

## 5.1 Scopes

The panel's Scope selector offers the scopes that make sense where the user is: Current Selection (or Selected Idea
Vault items) when something is selected, Current Scene when a scene is open, Current Screenplay, Specific Draft when a
draft is open, Story Board, Production, Specific Shooting Day and Call Sheet when one is open, and Whole Project. The
default is inherited from the workspace (for example Current Scene in the screenplay editor when a scene is selected;
Whole Project when nothing more specific applies). The resolved scope is shown
as a "Using: …" line and stored with the request.

## 5.2 Context assembly for one request

1. `scope::resolve` builds labelled context items for the explicit scope (budget 6,000 characters, ≤ 2,500 per item),
   as the requesting user.
2. `retrieval::retrieve` runs once per request (route chosen by the router, the resolved scope, 8 items) and returns a
   `ContextPacket` (§6).
3. `orchestrator::assemble_context` appends the packet's items after the explicit scope under the same 6,000-character
   budget, de-duplicated, each labelled with its source.
4. Everything goes into one `<project_data trust="untrusted">` block (§9).

Retrieval never fails a request: any retrieval error leaves the explicit scope only.

---

# 6. Retrieval: derived index, hybrid ranking, context graph

Full contract and measurements: `docs/engineering/18-retrieval-intelligence-index.md`.

## 6.1 The derived intelligence index

`<Project>.openframe/cache/intelligence.sqlite` (crate `openframe-search`, schema version 1, builder version 1):

- `semantic_document` / `semantic_chunk`: one document per domain entity (screenplay scenes of the current draft,
  story acts/sequences/beats/cards, characters, cast, crew, locations, catalog items, per-scene breakdown, shooting days,
  call sheets, notes, private notes (owner only), Idea Vault items, comments, tasks, moodboards, storyboards, shots,
  sides, project files, episodes, the project), chunked by domain (scene first, card/beat, one logical entity,
  paragraphs, call-sheet sections), ≤ 24 chunks per document.
- `vec_chunk`: a `sqlite-vec` 0.1.9 `vec0` table. sqlite-vec is compiled in and registered **only on intelligence
  connections**; there is no runtime extension loading, and project databases never gain the module.
- `context_node` / `context_edge`: the context graph (§6.3).
- `index_meta`: schema, builder and canonical-schema versions, project id, embedding model id (model id, version and
  SHA-256 prefix) and dimension.

The file is **derived and disposable**: never canonical, never in undo, activity, packages or migrations, and not
needed to open a project. It is opened as untrusted (`quick_check`, exact schema allow-list, `trusted_schema=OFF`); any
damage or identity/version mismatch deletes and rebuilds it. A different embedding model re-embeds only the vectors.
Contact details (phone, e-mail, emergency contact) are never put into documents.

## 6.2 Incremental indexing

After every committed mutation, undo, redo, restore, purge or package apply, the store's commit observer (called after
the writer lock is released) enqueues the affected documents. A background worker per open project debounces (750 ms,
at most 5 s under continuous typing), coalesces by document, reads the project on its own read-only connection and
embeds in batches of 16 without holding any project lock. The canonical save never waits for indexing, and an indexing
or embedding failure never fails a save. Index states: `Current`, `Updating`, `Stale`, `Rebuilding`, `Failed`,
`Unavailable`. The worker starts at project open when Offline AI is installed, otherwise on the first retrieval or
index request.

## 6.3 Context graph

Built deterministically from canonical relations, one fragment per entity. Relations include `appears_in`,
`located_at`, `represented_by`, `belongs_to`, `related_to`, `plays`, `requires`, `scheduled_on` and others (doc 18 §5).
Provenance: **Canonical** (foreign keys, link rows) or **Derived** (deterministic parsing, e.g. a heading location that
equals a Production location name). **Inferred** edges are never produced and are excluded from traversal. The model
never writes graph edges.

Traversal: BFS from the top 5 fused entities, 1 hop normally and 2 when the question spans modules, ≤ 16 nodes, ≤ 12
edges per node, relation priorities from the question; the permission check runs on every node.

## 6.4 Router and ranking

`intelligence/router.rs` (deterministic; the model does not choose the route):

| Question | Route | Retrieval |
|---|---|---|
| How OpenFrame works | `ProductHelp` | Product guide sections (`knowledge.rs`), no project RAG |
| Counts, lists, status, "open scene …" | `Structured` | Explicit scope only; exact facts come from SQL-backed tools |
| Short lookups, quoted text | `Lexical` | FTS5 |
| Meaning/theme questions, change requests | `Hybrid` | FTS5 + vectors |
| Two or more product areas, relationship wording | `HybridGraph` | FTS5 + vectors + graph expansion |

Ranking (`openframe_search::rank`): Reciprocal Rank Fusion (k = 60) over lexical and vector ranks, plus scope,
exact-name, graph and current-draft boosts. There is no LLM reranker.

## 6.5 Canonical re-validation, privacy and budgets

The assembler **rebuilds every candidate from canonical rows** before it is used: deleted objects, scenes of other
drafts, other users' private notes and out-of-scope items are dropped. A tampered index can therefore change only which
canonical objects are considered, never their content or visibility. Budgets: 6,000 characters total, 1,500 per item,
graph-related entities as ≤ 320-character summaries (≤ 6). Each item carries a source label and provenance.

## 6.6 Fallbacks

| Condition | Behaviour |
|---|---|
| Hybrid available | Hybrid (+ graph where routed) |
| No embedding runtime / vectors not ready / vector error | FTS + SQL tools; `degraded = true`; notice "AI can still answer some questions while project context is being prepared." |
| FTS error | SQL tools + explicit scope |
| Index unavailable | Same; never an error for the user |
| Language model unavailable | The request ends as Unavailable; the rest of OpenFrame is unaffected |

## 6.7 Measured budgets (spec §34)

Measured on a generated feature-length fixture (debug test profile): FTS retrieval p95 17.1 ms (budget 150 ms),
vector search p95 7.7 ms (100 ms), graph expansion p95 1.8 ms (50 ms), hybrid + assembly p95 43.3 ms (250 ms), save
latency while embedding p95 6.2 ms. The query-embedding time of the real model is not included. Details: doc 18 §9.

---

# 7. The universal toolbox

Contract C3, `modules/ai/toolbox.rs`. Inventory: `docs/engineering/ai-tool-coverage.md`, generated by
`tests/ai_tool_coverage.rs`, which fails if any registered operation lacks metadata.

## 7.1 Operation metadata

Every registered operation carries explicit `OperationMetadata`: module, plain description, AI exposure (`Tool` or
`Hidden(reason)`), operation class (Read, Search, Compute, Navigate, Suggest, Mutate), required capability,
destructive, irreversible, extra UI confirmation, file-system effect (none / reads a user-picked file / writes a
user-picked location / project storage) and long-running. The toolbox is built from this metadata plus hand-authored
schemas; the model never has to infer danger from an operation's name.

Current inventory: **423** operations (124 queries, 299 commands); **298** reachable through tools, **125** not
AI-accessible, each with a published reason; **251** tools (58 read/compute/navigate/search, 193 proposal).

## 7.2 Read, compute, search and navigate tools

Domain-named, bounded tools covering Project, Idea Vault, Story, Screenplay, Breakdown, Production (catalog,
locations, cast and crew, production source), visual planning (moodboards, storyboards, shot lists), schedule and
shooting days, call sheets, sides and reports, budget, files, notes, tasks, comments, templates, Recently Deleted,
activity, package history, project settings and search — for example `count_scenes`, `unscheduled_scenes`,
`compare_drafts`, `schedule_day`, `call_sheets`, `open_scene`, `open_object`, `search_project` and
`retrieve_context` (hybrid retrieval, contract C2). Results are clipped (content ≤ 4,000 characters, ≤ 80 items,
details ≤ 24) and read through the caller's connection with the caller's permissions.

Model-written tools are terminal and bounded: `summarize_scope` (≤ 6 sentences from the scope), `suggest_breakdown`
(schema-constrained list with quoted evidence; creates nothing), `answer_product_question` (only from the product
guide) and `clarify` (one focused question). `private_information` handles private-note requests: only the owner's
own notes, and a neutral refusal otherwise.

## 7.3 Proposal tools

193 `propose_*` tools cover every meaningful user mutation that is AI-exposed (for example
`propose_rename_character`, `propose_schedule_scenes`, `propose_call_sheet`, `propose_task`, `propose_update_shot`,
`propose_budget_line`). Each one resolves references to rows (names, scene numbers, "Day 3"), asks one question when
several rows match (`ai.ambiguous`), records base revisions for stale detection and returns a `ChangeSetDraft` with a
title, summary, affected modules, targets, exact operations (`{op, args, label}`), preview rows, exclusions and
warnings. Every built draft is re-validated: only listed, exposed, registered commands, never `ai.*`, at most 200
operations.

Example: `propose_rename_character` changes the Character record, the screenplay Character cues of the current draft
and Cast entries; raw dialogue/action mentions are listed separately and are not changed unless `includeRawText` is
set; locked drafts and earlier drafts are excluded and reported
(`rename_includes_cast_catalog_and_excludes_locked_or_earlier_drafts`).

## 7.4 What the AI cannot reach, and why

Summarised from the generated matrix: accepting/rejecting/re-checking Change Sets (human-only); the assistant's own
entry point and history; Offline AI install/runtime management; project lifecycle on disk (create, open, close,
duplicate, archive, delete, locate); anything that reads or writes a user-picked file-system location (imports,
exports, packages, adding files) or returns local file paths; permanent deletion; package review and apply (a human
workflow); undo/redo/save/cancel task; per-user view state; manual visual arrangement; keystroke-level screenplay
editor primitives; the owner-only unlock override; adding media the user supplies; the Global Idea Vault and other
projects (cross-project boundary); internal maintenance (`search.rebuild`, `ai.index_*`).

## 7.5 Enforcement on every call

`run_tool` re-checks everything; the list offered to the model is not the boundary: unknown tool → `ai.unknown_tool`;
arguments validated against the tool's strict schema (`additionalProperties: false`, bounded strings, lists and
numbers, 8 KB total) → `ai.tool_arguments`; missing capability → `permission.denied`. Proposal tools are offered only
when the actor holds the proposal's capability **and** `ApplyChangeSet`, and every operation they use is exposed.

---

# 8. The bounded agent loop

`modules/ai/orchestrator.rs`. One agent, one loop, no second model acting as critic; Rust validation is the critic.

```text
ai.ask ─► UseAi required ─► request ≤ 2,000 chars ─► scope + retrieval + context (§5)
  ─► tools offered = core tools + request-relevant tools (≤ 7,000-char catalogue)
  loop (default 8 steps; request may set 1..12; hard cap 12):
     ONE grammar-constrained model call → {"action": tool, "arguments": {...}, "done": bool}
        malformed / unknown tool → one corrected retry, then stop
     same tool + same arguments seen before → stop ("the same step was requested twice")
     arguments > 4,096 bytes → blocked, fed back as an error
     toolbox runs the tool as the user:
        read / compute / search → observation fed back (≤ 1,500 chars each, 6,000 total)
        proposal                → collected, never applied (≤ 6 parts)
        navigate / clarify / generation / private → terminal
     "done": true, final_answer, or a terminal tool → end
  end ─► ONE result: answer · navigation · clarification · proposal (one composite Change Set)
         · denied · unavailable · failed
      ─► per-step audit rows; no reasoning text is stored
```

Details that matter:

- **Tool subset.** `tools_for_request` always offers the core tools (`retrieve_context`, `search_project`,
  `project_overview`, `open_object`, `open_workspace`, `open_scene`, `clarify`, `answer_product_question`,
  `private_information`) plus the domain tools whose names and descriptions share the most words with the request.
  Proposal tools are offered only when the request contains change words ("add", "rename", "schedule", "delete", …).
  This keeps the 1B model's prompt short; the benchmark showed one-shot selection among ~30 tools is weak.
- **Schema order.** The step schema names the tool (`action`) before its `arguments`, so grammar-constrained output
  chooses the tool first.
- **Bounds.** Step reply ≤ 400 tokens and ≤ 4,000 characters; final answer ≤ 1,200 characters; recent history = 4
  turns, each side clipped to 400 characters, passed as data.
- **Answers.** Exact results of deterministic tools are shown as **Exact · from project data**; text the model wrote
  (`final_answer`, summaries, product answers) is marked **Inferred · written by Offline AI** and keeps the exact
  facts alongside it. A model-written answer cites up to 6 retrieved sources.
- **Stopping without a model.** If the model fails mid-run, what earlier steps found is returned with "Offline AI
  stopped before finishing. This is what I found so far."; with nothing found the result is Unavailable or Failed and
  "Nothing was changed."
- **Denials.** A tool the user may not use is known to the loop only by name, so the reply can say "I can't do that."
  with the reason and "Nothing was changed."; execution still runs as the real user and is refused.
- **Background tasks.** A tool result that reports a task id ends the loop with "This continues in the background;
  you can keep working." No current tool starts a task: every long-running operation (imports, exports, packages,
  index rebuild) is hidden from the model.

---

# 9. Prompt-injection boundary

- The policy lives only in the system message. Project text, retrieved chunks, tool observations and earlier turns
  travel in delimited blocks: `<project_data trust="untrusted">`, `<observation … trust="untrusted">`,
  `<conversation_history>`, `<product_guide>`. The policy states that only the text after "User request:" is an
  instruction.
- `escape_untrusted` removes control characters and neutralises every opening/closing delimiter and the
  "User request:" marker with length-preserving ASCII case folding; the scope label is escaped too
  (`ai_01_prompt_delimiters_survive_unicode_case_folding`).
- Observations fed back to the model are re-serialised by OpenFrame (never the raw model text).
- The real boundary is structural: the model can only pick offered tools; tools run with the user's permissions;
  mutations become proposals; acceptance is not a tool. Tests: `prompt_injection_in_project_text_is_data_not_instructions`,
  `injected_text_in_project_content_stays_data_inside_observations`, `read_tools_never_mutate_and_resist_injection_text`,
  `the_model_cannot_accept_even_when_told_to_in_conversation`, `sql_and_oversized_arguments_never_reach_the_database`.

---

# 10. Change Sets and human approval

`modules/ai/change_set.rs`.

```text
prepare ─► Pending ─[Apply Changes]─► revalidate ─┬─ changed ──► Stale    (not applied; "Re-check & Review")
               │                                  ├─ deleted ──► Conflict (not applied)
               │                                  └─ valid ────► Accepted ─► apply every op through the registry
               │                                                             ├─ all ok ─► Applied (one undo step)
               │                                                             └─ any fail ► undo applied ops ─► Failed
               └─[Reject]─► Rejected (project untouched)
```

- **Composite.** A multi-step request ends in one Change Set (≤ 6 parts): operations in order, previews part by part,
  modules/targets/base rows unioned. **Re-check & Review** rebuilds every part from its recorded source against the
  current project and returns it to Pending for a fresh review; it never applies anything.
- **Accept** (`ai.change_set.accept`) requires a local user action and `ApplyChangeSet`; the `Pending → Accepted`
  transition is a compare-and-set, so a double click cannot apply twice (`ai_02_concurrent_accepts_…`). Each operation
  must be a registered, allow-listed command that the user may perform, never `ai.*`.
- **Destructive changes.** When the preview contains a destructive row, accept also needs the product's extra
  confirmation ("Apply changes that delete items?"); without it the request fails with
  `validation.confirmation_required` (`destructive_change_sets_need_the_extra_confirmation`).
- **Apply.** Operations run through `AppCore::dispatch` with `ActorOrigin::Ai` (so events and activity show the change
  as AI-assisted by this user), all-or-nothing; the individual undo steps are grouped into one "AI-applied: …" step.
- **Reject** leaves project content unchanged (AI-AC-008).

---

# 11. Permissions

- `UseAi` is required to ask; Owner, Editor, Commenter and Viewer have it; Export-only does not.
- Read tools require View; proposal tools require their operation's capability and `ApplyChangeSet` (Owner and
  Editor). A Viewer asking for a change gets a clear denial and no Change Set.
- The same checks run again when a Change Set is applied (a role change in between is caught).
- Private notes: the owner filter applies in the intelligence documents, FTS and vector retrieval, graph traversal,
  every read tool, `private_information` and provenance links.

---

# 12. Provenance and exact facts

- Decision rule: exact/project fact → deterministic tool; semantic interpretation → retrieval + model; creative
  suggestion → model, labelled as a suggestion; mutation → proposal + human approval.
- Results carry lightweight provenance (scope, tool sources, retrieved sources, linked objects); chips under an answer
  ("Based on: …") open the source when it can be opened. Vector scores are not shown. Records store references, not
  copies of project content.
- If a fact cannot be found, the assistant says so instead of inventing it (AI-AC-004).

---

# 13. Persistence

Stored inside the project database (`migrations/project/0008_ai.sql`):

| Table | Contents |
|---|---|
| `ai_request` | The user's request text, conversation id, resolved scope (kind, references, label), model reference `openframe-local-ai-v1:<chat model id>`, intent, class, targets, authorization, status, processing (`Local` / `Not Sent`) |
| `ai_result` | Kind, status, the answer text, details, items, navigation, provenance, confidence, Change Set link, error code, task id |
| `ai_tool_invocation` | One row per step: tool, arguments, targets, provenance, authorization, execution, result reference, error code, task id, step index |
| `change_set` | Requesting user and role, title, summary, targets, affected modules, ordered operations, source tool(s) and arguments (for re-check), preview, exclusions, base version (row revisions), review/validation state, stale reason, approver and times |

No chain-of-thought and no copies of the assembled context are stored
(`ai_records_keep_references_and_audit_without_context_copies`). History is personal and survives restart
(`ai_ac_024_history_is_personal_and_survives_restart`). The derived index is outside the project database (§6.1).

---

# 14. Privacy and network

- Inference path: React → Rust → local context and retrieval → local `llama-server` → local Gemma → result. The
  inference client talks only to 127.0.0.1 (no proxy); `openframe-ai` is the only crate with an HTTP client.
- The AI subsystem uses the network only when the user clicks **Download Offline AI** or **Update Offline AI**: the
  signed manifest and the listed component files. Nothing is fetched in the background.
- No telemetry exists, so prompts, screenplay text, retrieved chunks, private notes and AI responses are never
  transmitted.
- The sidecars listen on 127.0.0.1 only, on a random port, with a random per-launch key; they run in a kill-on-close
  Job Object.

---

# 15. User-facing states and wording

Ordinary workflow UI never shows: Gemma, GGUF, Q8_0, embedding model, sqlite-vec, RRF, context graph.

| Situation | Wording |
|---|---|
| Header chip | **AI Ready** (installed) · **Off** (not installed) |
| Not installed | "Offline AI is not installed." · **Enable Offline AI** · **[Download Offline AI]** |
| Installing | Checking device · Downloading · Verifying · Installing · Starting · Ready; **Pause** / **Cancel download** |
| Runtime failed | "AI is currently unavailable. OpenFrame's core workflows continue to work normally." + "Ask again to restart it." |
| Index rebuilding | **Preparing project context…** "AI can still answer some questions while project context is being prepared." |
| Working | "Working on it on this computer…" |
| Answer labels | "Exact · from project data" · "Inferred · written by Offline AI" · "Suggested" |
| Proposal | **Proposed Changes** — summary, **Affected areas**, preview, exclusions, "N changes …, applied together as one step you can undo. If any part fails, nothing is changed." **[Reject] [Apply Changes]** |
| Stale | "Proposed Changes — out of date", "The project changed after this suggestion was prepared. Review is required before applying it." **[Reject] [Re-check & Review]** |
| After apply / reject | "Applied N changes. Use Undo to reverse it." · "Rejected. Nothing was changed." |

Technical details (component names, versions, licences incl. the Gemma notice, sizes, SHA-256, backend, hardware,
manifest channel, folders, log files) live in Settings → Offline AI → Technical details.

---

# 16. Failure behaviour

| Failure | Behaviour |
|---|---|
| Offline AI not installed | Setup view; every other workflow unaffected |
| Sidecar crash / timeout | Request ends as Unavailable; "Nothing was changed."; bounded automatic restart, explicit retry |
| Malformed model output | One corrected retry, then stopped; never applied |
| Unknown tool / oversized arguments | Refused before anything runs; fed back or ends the request |
| Repeated identical call / step cap | Loop stops with what was found so far |
| Embedding runtime unavailable | Retrieval falls back to FTS + SQL; saves unaffected |
| Index damaged or stale | Deleted and rebuilt in the background; "Preparing project context…" |
| Stale / deleted targets at apply | Stale / Conflict; nothing applied |
| An operation fails during apply | Already-applied operations are undone; Failed; "Nothing was changed." |

---

# 17. Acceptance criteria and their tests

| ID | Criterion | Test(s) |
|---|---|---|
| AI-AC-001 | Read-only answer changes nothing | `tests/ai.rs` suite (e.g. `ai_ac_002_…`, `ai_ac_005_…`) |
| AI-AC-002 | Exact counts from canonical data, with scope and provenance | `ai_ac_002_exact_counts_come_from_canonical_data_with_scope_and_provenance` |
| AI-AC-003 | Scope disclosed ("Using: …") | `ai_ac_002_…` (scope asserted), UI `Conversation.tsx` |
| AI-AC-004 | Facts that cannot be found are not invented | `ai_ac_004_facts_that_cannot_be_found_are_not_invented` |
| AI-AC-005 | Navigation runs directly and changes nothing | `ai_ac_005_navigation_runs_directly_and_changes_nothing` |
| AI-AC-006 | Breakdown suggestions create nothing | `ai_ac_006_breakdown_suggestions_create_nothing` |
| AI-AC-007–010 | Preview, explicit acceptance, normal application, one undo step | `ai_ac_007_to_010_proposal_requires_explicit_acceptance_and_is_one_undo_step` |
| AI-AC-011 | Permissions inherited | `ai_ac_011_permissions_are_inherited_viewers_cannot_prepare_or_apply`, `tools_are_filtered_by_role_and_enforced_when_run` |
| AI-AC-012 | Locked drafts respected | `rename_includes_cast_catalog_and_excludes_locked_or_earlier_drafts` |
| AI-AC-013 | Other users' private notes never reach the model | `ai_ac_013_…`, `spec_20_private_notes_are_only_ever_returned_to_their_owner`, `other_users_private_notes_never_reach_a_tool` |
| AI-AC-014 | Stale Change Sets revalidated | `ai_ac_014_…`, `recheck_rebuilds_every_part_of_a_stale_composite_change_set` |
| AI-AC-015 | Rename separates structured references from raw text | `rename_includes_cast_catalog_and_excludes_locked_or_earlier_drafts` |
| AI-AC-016 | Batch shows counts and objects first | Proposed Changes preview; `a_multi_step_mutation_ends_in_one_reviewable_change_set` |
| AI-AC-018 | Works without internet once installed | `crates/openframe-ai/tests/lifecycle.rs` (chat and embeddings usable without and after an app restart; inference goes only to the loopback sidecar) |
| AI-AC-019 | Core product usable without AI | `fsd_ai_001_openframe_works_fully_without_offline_ai` |
| AI-AC-020 | AI failure leaves the project unchanged | `ai_ac_020_unavailable_runtime_leaves_the_project_unchanged`, `a_model_failure_mid_run_keeps_what_was_found` |
| AI-AC-021 | Cross-module question | `spec_18_graph_expansion_adds_related_cross_module_context` |
| AI-AC-022 | No autonomous mutation | `no_tool_the_model_can_see_reviews_or_applies_change_sets`, `the_model_cannot_accept_even_when_told_to_in_conversation`, `acceptance_is_refused_for_anything_but_a_local_user_action` |
| AI-AC-023 | Product questions grounded in the product guide | `ai_ac_023_product_questions_are_grounded_in_the_product_guide` |
| AI-AC-024 | Follow-ups use bounded history; history personal and persistent | `ai_ac_024_…`, `history_is_bounded_to_recent_turns` |
| AI-AC-025 | The model adapter never changes data semantics or authorization | `ChatModel` boundary; every agent test runs with a scripted test model |
| AI-AC-026 | Bounded loop: default 8, hard 12, repeated calls stop | `default_step_limit_is_eight_and_the_hard_limit_twelve`, `repeated_identical_calls_stop_the_loop` |
| AI-AC-027 | Saves never wait for indexing; failures fall back | `spec_15_saves_never_wait_for_embedding_and_typing_stays_responsive`, `spec_33_…` |

AI-AC-017 (external disclosure) is retired: there is no external AI path (§21).

---

# 18. Test layers

Summary; full plan in `docs/engineering/22-qa-test-strategy.md` §6.

- `openframe-ai`: manifest trust, downloads, supervisor, whole-install lifecycle with a stand-in `llama-server`, AI
  security (`tests/security_ai.rs`); `real_runtime` (ignored by default) against the real components.
- `openframe-search`: index open/trust/rebuild, vector module scope, graph, ranking (unit tests).
- `openframe-application`: `tests/ai.rs`, `ai_agent.rs`, `ai_toolbox.rs`, `ai_tool_coverage.rs`, `ai_retrieval.rs`,
  `ai_retrieval_perf.rs` — all with a deterministic scripted test model and a deterministic test embedder
  (`openframe_test_support`), never shipped.
- Frontend: `AiPanel.test.tsx`, `OfflineAiSetup.test.tsx`, `model.test.ts`. E2E: the "not installed" journey.

---

# 19. Performance rules

No embedding work on the UI thread or inside a write transaction; indexing is debounced and background; one model call
per loop step (no critic calls); query embeddings cached (LRU 64); bounded context keeps Gemma latency low; the model is
not invoked for deterministic work that a tool answers (a single deterministic tool call with `done: true` ends the
request without a second model call).

---

# 20. Not implemented / known limits

- **Retrieval evaluation harness** (spec §35: Recall@K, MRR, FTS vs vector vs hybrid vs hybrid+graph on a labelled set)
  does not exist yet. The spec's rule "do not ship the vector/graph complexity unless it outperforms" is therefore not
  yet evidenced beyond `spec_19_hybrid_finds_meaning_that_keyword_search_misses` (test embedder).
- **E2E** covers only the "not installed" state; ask/retrieve/propose/apply/reject/stale/restart/rebuild are covered by
  Rust integration tests with the scripted model, not by a desktop E2E run with the real model.
- **Hardware coverage:** the Vulkan build on a discrete graphics card and minimum-spec (8 GB) hardware have not been
  measured (runtime spec §16).
- **Tool selection accuracy** of the 1B model is modest (≈39% tool + key arguments in one shot in the benchmark);
  mitigated by the request-relevant tool subset, deterministic tools and schema-constrained output.
- **Retrieval scope:** only the current draft's scenes are indexed; the Global Idea Vault, exchange review records and
  templates are not in the project index.
- **Background tasks from tools:** supported by the loop, unused (§8).
- **Semantic retrieval is English-oriented** (BGE small English); FTS still works for other languages as in Global
  Search.

---

# 21. Superseded content of the previous AI specification

Retired (not implemented, not tested, not shown):

- External AI providers, the external-disclosure dialog, "Send to External AI", "AI Provider Connected", provider
  retention statements and AI-AC-017 (ADR-0012 §5).
- User model selection, model tiers (Lightweight / Recommended / High Quality) and "replace the language model" as a
  user feature (ADR-0013). AI-AC-025 now means the internal adapter boundary only.
- AI inside LAN collaboration sessions (ADR-0007).
- A separate "AI states" vocabulary such as Interpreting / Retrieving / Generating / External disclosure. The states
  users see are those in §15; persisted states are those of `ai_result` and `change_set` (§13).
