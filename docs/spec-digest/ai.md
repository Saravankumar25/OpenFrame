# AI Specification — Implementation Digest

Source: `OpenFrame_Studio_AI_Specification_Updated.md` (3919 lines, 64 sections + Appendices A–E), read in full.
Cross-refs to `OpenFrame_Studio_Security_Privacy_Data_Ownership_Specification.md` are marked **[SEC §n]**.

> **BUILD OVERRIDE — LOCAL AI ONLY.** Approved decisions: managed llama.cpp sidecar + Qwen GGUF; no cloud AI, no BYOK, no Ollama; no accounts; no telemetry by default. Every spec passage about external providers, provider/model disclosure, API keys, or external transmission is flagged below with **⚠ SUPERSEDED (LOCAL AI ONLY)**. Such passages still tell us which *invariants* must hold (e.g. "never imply external transmission"), but the external path itself is not built.
>
> The spec says nothing about API keys or BYOK anywhere. It never names Ollama or llama.cpp. It names "Qwen2.5-0.5B-class local model" as a reference target (§25.2), so the approved Qwen GGUF choice fits the spec.

Legend: "Spec:" = near-verbatim wording. "NOT SPECIFIED" = the spec is silent; do not invent it.

---

## 0. Header facts and authority

- The header's operating model says: "Local-first, offline-capable, user-owned project files; local AI where available; **optional external AI**; no mandatory OpenFrame cloud". **⚠ SUPERSEDED (LOCAL AI ONLY)** for the "optional external AI" clause.
- Core safety rule (§0.1): **"AI may suggest. The user confirms anything that becomes meaningful project data."**
- The FSD requires that project-changing AI actions have a preview plus explicit acceptance. An accepted AI mutation becomes a normal undoable application action.
- Hierarchy (§0.2): PRD → FSD → {UX/UI, Domain/Data, AI Spec} → Implementation. The AI spec is authoritative *within the AI boundary* but "must not contradict" the source-of-truth, permission, offline, versioning, recovery, collaboration, or user-control rules in the other specs.
- Model-agnostic (§0.3): the spec does not mandate a language, DB, vector DB, agent framework, API framework, desktop framework, or model runtime. "A reference small local model may be selected for implementation, but the product contract must remain model-agnostic."
- Product positioning: "**Amazon Q-style project assistant**" (§0.1, §53, §56.1). The goal is "Jarvis for the application" without independent authority (§64).

## 1. What the AI is / is not (§1, §57, §60, §64)

- It IS "a **system-wide natural-language assistant and command interface for OpenFrame Studio**" (§1.1).
- It is NOT a separate chatbot product, a screenplay judge, an autonomous filmmaker, an independent database, or the source of truth for project content.
- The LLM is "primarily the natural-language interpretation, routing, explanation, and response layer". Application data and deterministic services remain authoritative.
- Key principle (§4.3): **"The model decides what operation is needed; the application performs and validates the operation."**
- §57: it is "an intelligent interface over the software's existing capabilities", not "an LLM that happens to know about the software".
- **Must never become** (§60): an autonomous script writer that modifies canon without asking; an autonomous production manager, scheduler, or approval authority; a background cleanup agent; an unrestricted filesystem agent; a hidden cloud synchronization mechanism; a second source of truth; a second project database; a permission bypass; or a private-data leakage mechanism.
- It is system-wide, not screen-limited (§1.3). It can answer about the current scene, screenplay, a particular draft, the whole Story Board, selected Idea Vault material, characters, locations, cast and crew, production catalog, breakdown, shot list, storyboard, schedule, shooting days, call sheets, project files, tasks, notes, activity, approvals, project status, relationships, or the entire project. It "must always indicate the scope used when scope could materially affect the answer."
- §58 "Knows everything" means the AI can *access authoritative sources*. It does not mean facts are embedded in the weights. The AI is complete when it can: (1) understand OpenFrame terminology; (2) locate the product rule; (3) locate the project data; (4) calculate exact facts; (5) navigate; (6) explain relationships; (7) prepare actions; (8) obtain explicit approval; (9) apply the normal operation safely.
- §59 "Does everything" means: "If OpenFrame can do it, AI should be able to route a supported natural-language request to the same underlying operation." Examples: create, edit, rename, duplicate, move, archive, restore, search, inspect, compare, export, break down, schedule, create call sheet, create task, create notes, manage supported production data. "The goal is broad capability coverage, not a giant list of artificial AI commands."

## 2. Non-negotiable rules (§2) — AUTHORITY BOUNDARIES

### 2.1 Never silently (§2.1)
The AI must never silently: rewrite screenplay content; change a Scene Card; alter Story Board hierarchy; alter canon; rename project objects; edit production data; alter the shooting schedule; modify a call sheet; delete project content; archive or restore objects; lock a script; approve an item; reject an item; publish/release an artifact; change permissions; expose private information to another user; or otherwise mutate project state.

### 2.2 A command is not blanket mutation permission (§2.2)
- A natural-language command is sufficient authorization for a **read-only** operation or a **non-persistent navigation** action.
- It is NOT sufficient to silently apply a mutation. The required flow is: User request → Interpret → Prepare exact proposed action → Show preview → User explicitly accepts → Apply normal application action → Record undoable change.
- "The assistant must not skip the acceptance step merely because the user phrased the original request as an imperative."
- Canonical correct response example: "I found 184 affected structured references across the current project. 31 screenplay Character elements, 42 Scene Cards, 27 breakdown references, 18 cast references, and 66 other supported references. I will not change arbitrary dialogue/action text automatically. Review changes → Apply."
- Incorrect: "Changes are applied immediately."

### 2.3 No autonomous behavior (§2.3)
The AI must not independently decide to:
- run an analysis because it thinks it is useful;
- change data because it detects an inconsistency;
- execute pending suggestions later;
- continuously rewrite content;
- automatically clean the project, remove duplicates, update production information after a script change, reorder scenes, apply breakdown suggestions, resolve conflicts, or approve documents;
- perform background agent tasks that mutate user-owned content.

Non-AI infrastructure operations are allowed and are "not AI decisions": indexing, saving, recovery preparation, and deterministic derived calculations.

### 2.4–2.5 Source of truth and determinism
- The AI uses canonical objects. "The model's generated text is never itself authoritative." It must not invent project facts when canonical data exists.
- Exact questions MUST use deterministic queries: character count, scene count in Draft N, locations used, page count, shooting days, scenes featuring Character A, scenes at Location B, unscheduled scenes, confirmed breakdown elements, pending suggested breakdown elements. "The language model may explain the result, but it must not be the calculator."

### 2.6 Permission inheritance
- "The assistant can never grant itself more access than the current user has."
- It respects: project role; object permissions; private-note ownership; collaboration permissions; locked-state restrictions; export restrictions; project access restrictions.
- "The AI operates as the current user within the same authorization boundary as the user."

### 2.7 No hidden context — scope labels
- Do not use unrelated material without appropriate scope. The UI exposes scope in a form users understand, with no internal IDs.
- Exact example labels:
  - **"Using: Draft 7 + Scene 24 + Character records"**
  - **"Using: Entire Project"**
  - **"Using: Current Screenplay + Production Catalog + Shooting Schedule"**
- Note: §7.4 uses **"Using: Whole Project"** and §41 uses **"Using: Scene 24 + Characters + Breakdown"**. "Entire Project" and "Whole Project" both appear. The canonical scope-list name is `Whole Project` (§7.1).

### 2.8 Model responsibility split (§4.2)
- **The model may:** interpret intent; identify objects and references; choose tools; convert natural language to structured parameters; summarize returned data; explain results; formulate confirmations; explain proposed changes; identify ambiguity; ask for missing info; generate creative text when requested.
- **The model must not:** directly modify canonical storage; bypass permissions; decide authoritative state; commit a Change Set; bypass validation; bypass user acceptance; fabricate exact counts.
- **The small model must NOT do alone** (§25.3): count objects from raw text; do exact page calculations; decide permissions; edit project files; calculate version diffs; determine schedule conflicts from memory; enforce business rules; validate Change Sets; determine source of truth; retrieve hidden private data.
- **Small-model duties** (§25.4): understand the request; identify the operation; identify entities; generate structured tool parameters; interpret deterministic results; explain them; prepare a clear mutation proposal.
- **§25.5 "Model failure must not become data failure":** when the model misunderstands, ask for clarification, offer recognized interpretations, and leave state unchanged. "A weak model response must never cause an unsafe fallback mutation."

### 2.9 Security boundary (§34)
- "The assistant must never receive unrestricted filesystem or project-package access as a substitute for OpenFrame's object model."
- Required path: AI → Authorized Application Tools → Canonical Domain/Data Layer → Project. Forbidden path: AI → Unrestricted file write.
- "The language model must not modify project files directly."
- **[SEC §16.7]:** "Model text is not permission." Model output such as "The user wants me to change this." does not replace authorization, and model output cannot grant itself storage authority.

## 3. Three knowledge domains (§3)

| Domain | Contents | Authority |
|---|---|---|
| **Product knowledge** | What each workspace does, actions, terminology, workflow relationships, source-of-truth rules, permission rules, versioning, export/import, offline, collaboration, object types, operations, UI actions, AI behavior and limitations | "Must not depend entirely on the model's parameter memory." A **versioned product knowledge source** must exist conceptually |
| **Project knowledge** | Title, type, characters, story structure, drafts, scenes, elements, notes, Idea Vault, catalog, locations, cast, crew, breakdown, shots, storyboards, schedule, shooting days, call sheets, tasks, comments, approvals, activity, files, snapshots, version relationships | Primary grounding source |
| **Session knowledge** | Current project/workspace/scene, selected objects/text/draft, open panel, current user, current permissions, current AI conversation, pending preview, pending Change Set | "Contextual, not authoritative" |

Example product-knowledge questions: "What is the difference between Story Board and Screenplay?", "Where do I create a shooting day?", "What is the Production Source?", "Why did the call sheet become stale?", "Can AI change a locked shooting script?", "What happens when I delete a Scene Card?"

## 4. Operating architecture (§4.1) — 15 logical components

1 Request Intake · 2 Intent Interpretation · 3 Permission Evaluation · 4 Context Resolution · 5 Tool Selection · 6 Deterministic Query/Calculation · 7 Product Knowledge Retrieval · 8 Project Data Retrieval · 9 Proposed Mutation Construction · 10 Mutation Preview · 11 User Approval · 12 Change Application · 13 Undo/Recovery Integration · 14 Response Generation · 15 Audit/History.

These may be implemented together, "but the behavioral separation must remain."

Model-agnostic adapter (§26): User request → Model Adapter → Intent/Tool Request → OpenFrame Tool Layer → Structured Tool Result → Model Adapter → User response. "Changing from one local model to another must not require migrating project objects."

## 5. Operation classes (§5)

| Class | Name | Examples | Mutation? | Confirmation? |
|---|---|---|---|---|
| A | **Read** | answer question, summarize scene, list characters, where a character appears, identify locations, explain a draft, project status, find references | No | No |
| B | **Compute** | count characters/scenes/locations, page count, unscheduled count, shooting days, scene usage, compare versions, calculate affected objects | No | No |
| C | **Navigate / display** | open Scene 42, go to Character Room, show every scene involving Ravi, select Draft 8, open the Location entry for Railway Station, filter the Production Catalog | "application control, not project mutation" | No — "may be executed directly because the user explicitly requested it" |
| D | **Suggest / prepare** | suggest breakdown elements, propose a Scene Card, draft a synopsis, propose schedule grouping, prepare a call sheet draft, prepare a rename, produce a revision summary, prepare a batch update | Not until acceptance | Yes, where the result would create or change data |
| E | **Mutate** | create a Scene Card, create breakdown elements, rename, modify scene data, replace structured references, create a task, update approved production data, alter the schedule, archive, delete | Yes | Yes |

**Every Class E mutation requires 8 steps:** (1) exact interpretation; (2) permission check; (3) preview; (4) explicit user acceptance; (5) validation; (6) application; (7) undoability where supported; (8) change/activity recording.

`operation_class` values for persistence: `Read/Compute/Navigate/Suggest/Mutate` (§50.1).

## 6. Natural-language command interface (§6)

- Universal command layer: "The user should not have to know the exact location of the command."
- Tolerance (§6.2): abbreviated requests, conversational phrasing, inferable references, filmmaking terminology, project-specific names, spelling variation where resolvable, and deictic phrases such as "that scene," "the current draft," "this character," "tomorrow's shoot," "the station."
- "The assistant must not guess where two or more interpretations would materially change the result."
- **Ambiguity (§6.3):**
  - A low-impact ambiguity may be resolved from context.
  - A material ambiguity (for a mutation or exact answer) requires a focused clarification or a display of the competing interpretations.
  - Exact examples: "There are two characters named Raju: `Raju` and `Raju Kumar`. Which one should I use?" and "Do you mean unique Character records or characters detected in the current screenplay draft?"
- §31.5: "Ask **one** focused clarification."
- App. B Ex 2: "Which draft should I count: Draft 7, Draft 8, or the locked shooting draft?"
- §28: "Delete the old draft." → the AI "must not choose the nearest draft arbitrarily". It must identify the target and present a deletion confirmation.

## 7. Context and scope resolution (§7, §18, §36, §37) — RULES

**Supported scopes (minimum, §7.1):** `Current Selection`, `Current Scene`, `Current Screenplay`, `Specific Draft`, `Story Board`, `Idea Vault Selection`, `Production Workspace`, `Specific Production Object`, `Specific Shooting Day`, `Call Sheet`, `Whole Project`. Finer internal scopes are allowed.

Rules:
1. **Inheritance (§7.2):** a request made from a selected object may start from that selection as context. For example, with Scene 24 selected, "What props are here?" uses Scene 24 + linked Breakdown data + relevant Production Catalog references. The AI must not silently scan unrelated drafts or external projects.
2. **Explicit global (§7.3):** the user can broaden scope, e.g. "Across the entire project, find every place where Arjun is mentioned." That activates project-wide scope.
3. **Disclosure (§7.4):** meaningful queries show a compact scope indicator. It is "especially important for exact counts and mutation previews."
4. **Conversation persistence (§18.2):** after "Open Scene 24." then "How many props?", the AI may interpret the second request as Scene 24. If context could have changed or the ambiguity is material, show the context.
5. **Correction (§18.3):** "No, I meant Draft 7." updates the request context. The correction must not alter the project.
6. **Memory ≠ truth (§18.1):** conversation may retain prior requests, resolved references, current scope, pending proposal, and explanation preferences. It "must not override current canonical project data."
7. **Cross-project (§36):** keep Global information (Global Idea Vault) distinct from current-project information.
   - "Find my thriller ideas about buses." may use the Global Idea Vault.
   - "How many characters are in the film?" must use the current project.
   - "Must not merge unrelated projects." Cross-project operations require explicit direction.
   - "Copy this idea from the Global Idea Vault into the current project." is a mutation and requires confirmation.
8. **Episodic (§37):** hierarchy is Series → Season → Episode → Story → Screenplay → Production. Queries may be scoped to series, season, episode, multiple episodes, or whole series. "A cross-episode mutation must show the exact affected episodes before application."
9. **[SEC §9.2, §17.5]:** Security follows the resolved scope. Conversation context must not silently expand permissions. A follow-up reuses context only within the same effective access boundary.
10. **Sessions and permissions:** session knowledge includes current permissions. Pending Change Sets expire when permissions change (§20.6).

## 8. Deterministic query contract (§8) — exact-query families

Pipeline: NL question → Intent + parameters → Deterministic query tool → Canonical data → Structured result → LLM explanation. "Critical to making a small local model viable."

- **Project facts:** title, type, status, language, genre, creator, creation/update dates where exposed, current draft, production source, current shooting day, open tasks, pending approvals.
- **Screenplay facts:** draft count, scene count, page count, element count, dialogue count, character cue count, distinct character references, locations, INT/EXT usage, day/night usage, scene durations where represented, scenes by act/sequence/character/location/draft, scenes changed between drafts.
- **Story facts:** number of Acts, Sequences, Beats, Scene Cards; parked cards; character count; relationship count; timeline entries; outline hierarchy.
- **Production facts:** Production Source, breakdown status, breakdown element count, suggested vs confirmed elements, catalog item count, location count, cast count, crew count, shot count, storyboard panel count, shooting day count, unscheduled/scheduled scene count, stale production data, pending production updates.
- **Call-sheet facts:** generated call sheets, draft/approved/sent state, shooting-day relationship, stale state, affected changes.
- **Support facts:** project files, tasks, notes, comments, approvals, activity, snapshots, exchange packages.

**Metric definitions (§8.3) — never collapse distinct metrics:**
- "You have **14 Character records** in the project." ≠ "The current screenplay contains **12 distinct character cues**."
- "The Production Catalog contains 9 Location records." ≠ "The current draft uses 7 distinct location headings."
- Preferred combined answers: "Draft 8 contains 13 distinct character names in screenplay Character elements. The project Character Room contains 15 Character records." / "Draft 8 contains 8 distinct screenplay locations. The Production Catalog currently contains 10 Location records."
- "How many characters in the script?" (§8.4): 1. resolve the draft; 2. identify what "characters" means; 3. query structured character references; 4. return the exact number; 5. state the draft used.
- "Scenes left to schedule" = eligible screenplay scenes − scenes represented in shooting schedule, subject to canonical scheduling eligibility rules.
- "Which scenes need breakdown?" uses actual breakdown state, "not LLM inference."

**Search strategy (§35):**
- Exact structured search comes first for names, counts, statuses, scene IDs, relationships, draft versions, and schedule assignments.
- Text search is for screenplay wording ("railway bridge").
- Semantic retrieval is allowed for similar notes, related research, conceptually similar scenes, and approximate reference discovery. It "must not replace exact structured queries when the user asks for exact facts."
- **Grounding (§35.4):** retrieved text stays attributable. Keep project, object, draft, scene, and source type metadata.

**Global Search (§48):** Global Search and AI "should not become the same thing." Global Search stays the deterministic UI. The AI may use it internally, e.g. translating "Find all scenes where the character is at the hospital at night." into structured filters plus text search.

## 9. Cross-module knowledge graph (§9)

- Vertical chain: Idea → (explicit promotion/copy) → Story Board → (build/explicit conversion) → Screenplay Draft → (selected production source) → Production Source → Breakdown → Catalog → Shot List/Storyboard → Shooting Schedule → Call Sheet.
- Lateral links, Character: Story Board refs, Screenplay refs, Breakdown refs, Cast refs, Scene relationships, Continuity context.
- Lateral links, Location: Screenplay scenes, Breakdown, Catalog, Schedule, Call Sheets.
- Example "Where is the police station used?" may return screenplay scenes, Story Board refs, breakdown entries, catalog location, scheduled shooting days, and call sheets.
- "Must preserve the distinction between authoritative and derived information."

## 10. AI CAPABILITIES PER MODULE (can / cannot)

| Module | AI CAN | AI CANNOT / constraint |
|---|---|---|
| **Project** (§11.1) | "Open the Black Rain project."; "What is this project status?"; "Show the current shooting draft." | "Archive this project." is state-changing, so preview/confirm is required |
| **Idea Vault** (§11.2, §22.1) | Read, find ("Find all ideas containing railway station."), recent ideas, suggest ops; "Move this idea into the Story Board as a Scene Card." / "Create a Beat from this note." after confirmation | "Must not turn Idea Vault into a live synchronization layer" |
| **Global Idea Vault** (§36) | Cross-project idea search | Copying into a project is a mutation that needs confirmation. Must not merge projects |
| **Story Board** (§11.3, §22.2) | Counts ("How many Scene Cards are in Act 2?"), "Show parked cards."; prepare and modify after confirmation ("Create a Sequence called The Investigation.", "Move this Scene Card after Scene Card X.") | "Must preserve Scene Card identity and Story Board semantics." Story Board changes "must not silently rewrite screenplay text" |
| **Screenplay** (§11.4, §16, §22.3) | Open scene, page count, draft diff ("Which scenes changed between Draft 6 and Draft 7?"), find cues ("RAVI"), "Prepare a rewrite summary of Scene 22."; generate creative text when requested | Any screenplay mutation requires approval. AI text becomes screenplay content "only after explicit acceptance through the screenplay editing workflow." **Locked script (§16.3):** "AI cannot bypass the lock"; it must use the same revision mechanism as normal editing |
| **Draft comparison** (§16.1) | Explain scenes added/removed/changed, character/location/production changes, schedule and visual-planning implications | Comparison data "must come from the application's version comparison data" |
| **Revision impact** (§16.2) | Traverse Screenplay changes → Production Source → Breakdown → Catalog → Shots/Storyboard → Schedule → Call Sheet; report detected impacts | "It does not apply downstream changes automatically." |
| **Production Source** (§22.4) | Read/analyze | "Must not silently change the source draft because a production issue is detected" |
| **Breakdown** (§13, §22.5) | Suggest props, vehicles, wardrobe, makeup, special effects, stunts, weapons, animals, extras, background elements, locations, set requirements, sound, production design, camera-related, transport, and other categories | Suggested items must be "visually distinct from confirmed production items". "Must never add breakdown elements merely because it detected them." "Suggestion does not equal confirmation." Confirmed decisions remain user-controlled |
| **Catalog / Locations** (§11.5) | "Which locations have no approved status?", "Show all props in Scene 18." | Updating approved production data is a mutation |
| **Schedule** (§14, §22.6) | Identify opportunities; explain conflicts; calculate grouping options; location clustering; cast availability implications where data exists; page/day concentrations; stale dependencies; prepare proposed changes | Must NOT silently reorder scenes, change shooting dates or days, move strips, remove scenes from the schedule, or change schedule assumptions. "Shooting order remains user-controlled" |
| **Call Sheet** (§15, §22.7) | Summarize a shooting day; list cast, locations, scenes; identify special requirements; generate a **draft** call sheet; identify stale state; compare schedule vs existing call sheet | "Must not treat the AI-generated call sheet as authoritative merely because it looks complete." A generated call sheet "remains a draft until the user accepts it". The call sheet "does not automatically mutate the schedule" |
| **Continuity** (§17) | Check character knowledge appearing too early, location state, injury, prop, time-of-day sequence, character disappearance, repeated events, dialogue/canon conflicts, contradictions | Present as **"Potential continuity issue"**, not **"Confirmed error"**, unless deterministic rules establish it. "Must not automatically correct" |
| **Files** (§11.8, §40) | "Find the location reference PDF.", "Which project files mention the railway station?", "Summarize this production design PDF." when accessible through OpenFrame's file representation | For inaccessible external files, "must say so rather than hallucinating their contents" |
| **Imports / Exchange Packages** (§39) | Import inspection, package summaries, conflict explanation, mapping explanation, review-package summaries | Must not blindly overwrite the host, silently merge conflicts, or silently discard imported changes. Exchange rules stay authoritative |
| **Tasks / Notes** | Create a Task or Project Note via Change Set | "Save that as a Project Note." requires a prepared creation plus confirmation (§33) |
| **Historical snapshots** (§22.8, §12.5) | Read | Must not silently rewrite historical exports, issued call sheets, or snapshots |
| **Collaboration** (§38) | Act as a normal edit under participant permissions | Cannot bypass object permissions, hide conflicts, or apply stale changes without conflict review. Another participant's private notes stay protected |
| **Series** (§37) | Episode counts, cross-episode character/location/element queries | Cross-episode mutations list exact episodes before application |

**Reuse existing operations (§42, §43, §49):**
- An AI-created Scene Card, Breakdown Element, Task, or Call Sheet is a *normal* object of that type. An AI rename or schedule edit behaves like the normal one.
- Quick Actions stay as direct deterministic commands (e.g. "+ New Scene Card", "New Character"). AI "should complement those systems rather than replace". "Both should eventually invoke the same underlying application capability. This keeps AI from creating a second incompatible command system."

## 11. AI TOOL CATALOGUE

§10 says "The implementation should expose logical tools rather than allowing the language model arbitrary storage access". Names are the spec's own examples.

### 11.1 Navigation tools (§10.1) — Class C, direct execution
`open_project`, `open_workspace`, `open_object`, `open_scene`, `open_draft`, `select_objects`, `apply_filter`, `show_results`

### 11.2 Search tools (§10.2) — Class A
`search_project_text`, `search_screenplay`, `search_story`, `search_production`, `search_files`, `find_object`, `find_references`, `find_cross_module_references`

### 11.3 Query tools (§10.3) — Class A/B, deterministic
`get_project_summary`, `get_script_statistics`, `get_character_statistics`, `get_location_statistics`, `get_scene_statistics`, `get_breakdown_statistics`, `get_schedule_statistics`, `get_call_sheet_statistics`, `get_object_relationships`, `get_version_differences`, `get_stale_dependencies`

### 11.4 Analysis tools (§10.4) — advisory
`analyze_continuity`, `analyze_character_knowledge`, `analyze_revision_impact`, `analyze_schedule_conflicts`, `analyze_breakdown_candidates`, `analyze_missing_links`. These are "Advisory unless generated by deterministic rules that the product already treats as authoritative."

### 11.5 Preparation tools (§10.5) — Class D, produce a Change Set proposal
`prepare_scene_card`, `prepare_breakdown`, `prepare_synopsis`, `prepare_schedule_grouping`, `prepare_call_sheet`, `prepare_task_list`, `prepare_rename`, `prepare_replace`, `prepare_batch_update`

### 11.6 Mutation tools (§10.6) — Class E, "only invoked after the required approval state exists"
`create_object`, `update_object`, `move_object`, `duplicate_object`, `rename_object`, `update_structured_reference`, `create_breakdown_elements`, `update_schedule`, `update_call_sheet`, `archive_object`, `restore_object`, `delete_object`

### 11.7 Export tools (§10.7) — "must respect existing export permissions and project boundaries"
`export_screenplay`, `export_sides`, `export_call_sheet`, `export_breakdown`, `export_project_package`, `export_snapshot`

### 11.8 Other intents implied (no tool name given)
The spec implies these intents without naming a tool; any names are ours to define:
- product-knowledge retrieval (§3.1, AC-023);
- lock script, approve artifact, change permission (§45 matrix: "prepare", confirmation, per lock/approval/permission rules);
- permanent delete (§45);
- link/unlink (§20.2 op types);
- update text (§20.2);
- create project note (§33);
- copy from Global Idea Vault (§36);
- import inspection/summary (§39);
- file summarize (§40).

### 11.9 Structured invocation examples (§27)
```
tool: get_script_statistics
parameters: { project_id: current_project, draft_id: draft_8,
  metrics: [unique_character_cues, scene_count, page_count] }

tool: prepare_rename
parameters: { project_id: current_project, source_object: character:123,
  old_name: Ravi, new_name: Raghav, structured_references: true, arbitrary_text: false }
```
Serialization is implementation-specific. Requirement: "the model does not get unrestricted write access."

### 11.10 Tool-request validation (§28), in order
1. Validate the requested operation.
2. Validate parameter types.
3. Resolve referenced objects.
4. Resolve scope.
5. Check user permissions.
6. Check object state.
7. Check lock/approval restrictions.
8. Check whether confirmation is required.
9. Execute the read/query or prepare the mutation.
10. "**Never silently downgrade a mutation into a different operation.**"

Also **[SEC §7.4]**: a denied op must "not silently downgrade into a different write operation". **[SEC §34.5]**: "Model-generated tool parameters cannot elevate authorization."

## 12. AI ACTIONS MATRIX (§45, verbatim)

| Operation | AI may prepare? | Immediate execution? | Confirmation required? | Normal undo? |
|---|---|---|---|---|
| Answer question | Yes | Yes | No | N/A |
| Exact count | Yes | Yes | No | N/A |
| Search | Yes | Yes | No | N/A |
| Open workspace | Yes | Yes | No | N/A |
| Open object | Yes | Yes | No | N/A |
| Summarize | Yes | Yes | No | N/A |
| Continuity warning | Yes | Yes | No | N/A |
| Breakdown suggestion | Yes | No mutation | Yes before add | Yes after apply |
| Scene Card creation | Yes | No | Yes | Yes |
| Synopsis insertion | Yes | No | Yes | Yes |
| Task creation | Yes | No | Yes | Yes |
| Rename | Yes | No | Yes | Yes |
| Global structured replace | Yes | No | Yes | Yes |
| Schedule change | Yes | No | Yes | Yes |
| Call Sheet generation | Yes | No | Yes | Yes |
| Delete | Yes | No | Yes | Yes/recovery |
| Permanent delete | Yes | No | Explicit destructive confirmation | Recovery may be limited |
| Lock script | Yes | No | Yes | According to lock rules |
| Approve artifact | Yes | No | Yes | According to approval rules |
| Change permission | Yes | No | Yes | According to permission rules |

## 13. CHANGE SET MODEL (§20, §21, §31.6–31.7, §50.3; [SEC §26])

### 13.1 Fields
**§20.1 logical contents:**
- `change_set_id`
- `origin = AI`
- requesting user
- project
- target objects
- base version
- proposed operations
- affected modules
- validation state
- conflict state
- review state
- approval
- application state

**§50.3 AI provenance additions:**
- AI Request reference
- AI Result reference
- user approver
- approval timestamp
- approval scope
- application timestamp

**[SEC §26.2, §54.1] field names:**
- `Change Set.requesting_user_id`
- `Change Set.approver_user_id`
- `Change Set.base_version`
- Required provenance: origin, responsible/requesting user, approver, targets, operations, affected modules, base version, review state, validation state.

**[SEC §26.1] origins:** user edit, review response, package import, approved AI mutation.

### 13.2 Operation types (§20.2, minimum)
`Create`, `Update`, `Rename`, `Move`, `Duplicate`, `Delete`, `Archive`, `Restore`, `Link`, `Unlink`, `Replace Structured Reference`, `Update Text`, `Update Schedule`, `Update Production Data`

### 13.3 Lifecycle states
The spec does not give one enum. It combines the Appendix C request lifecycle, the §41.1 UI states, and the §50.2 Result status.

**Appendix C, "Minimal AI Request Lifecycle States":**
```
Created → Context Resolved → Authorized → Retrieving → Tool Execution → Result Ready
   Result Ready ├─ Informational
                └─ Proposal → Pending Approval ├─ Accepted → Validating → Applying → Applied
                                               └─ Rejected
Failure branches: Unauthorized · Ambiguous · Unavailable · Failed · Stale · Conflict · Canceled
```

**§50.2 AI Result status:** `Informational / Pending Approval / Accepted / Rejected / Applied / Failed`.

**[SEC §26.4]:** the Change Set "becomes Stale/Conflict as appropriate".

**Derived Change Set state set (union of the above; do not add others):** Pending Approval, Accepted, Validating, Applying, Applied, Rejected, Stale, Conflict, Failed, Canceled.

### 13.4 Preview / accept / reject / partial accept
- **Preview (§19.5):** Preview + Clear consequence + User action.
  - Minimum controls: **`Apply`**, **`Cancel`**.
  - Meaningful/batch controls: **`Review Changes`**, **`Select Changes`**, **`Apply Selected`**, **`Cancel`**.
- **Schedule preview example (§14.3):** "Proposed schedule change". It lists affected scenes, a Current/Proposed layout, and Potential impact ("Day 2 page load increases", "Cast overlap changes", "Existing call sheet for Day 2 becomes stale", "Day 5/7/9 become lower-load days"). Prompt: "Apply proposed schedule change?" with buttons **`[Cancel] [Review Changes] [Apply]`**.
- **Panel (§41):** "Proposed Changes / Affected objects / Impact / Conflicts / [Review Changes] [Apply] [Cancel]".
- **Breakdown partial accept (§13.3):** accept all; accept selected items; edit items; reject items; cancel.
- **Reject (§20.5):** "Rejecting a proposed Change Set must leave project content unchanged." (AC-008; [SEC §26.5]: rejected/invalid/stale/conflicted do not mutate.)
- **Rejected suggestions (§32.4):** "must not appear as an applied project mutation". Optional AI history may retain that it was generated/rejected.
- **Atomicity (§20.4):** where supported, a multi-op AI Change Set "should behave as one normal undoable application action". Ops that cannot safely be applied together must be "split … explicitly rather than silently partially mutating."
- **Partial failure (§31.7):** do not claim full completion; identify successful/pending/failed groups; preserve the original project where rollback is required; show what remains.
- **Approval ≠ permission ([SEC §26.3]):** "Approval does not override permission restrictions." Both must be valid.

### 13.5 Staleness / base-version protection (§20.3, §20.6, §31.6, §38)
- Every Change Set is tied to the base project/version state.
- If the project changes: Pending Change Set → Base version no longer current → **Revalidate** → Apply / Rebuild preview / Conflict review. "Must not blindly apply a stale Change Set."
- **Expiration triggers (§20.6):** target objects deleted; base version changed materially; permissions changed; required external data disappeared; project replaced/restored. "The user should be shown why reapplication is required."
- **Stale message (§31.6):** **"The project changed after this suggestion was prepared. Review is required before applying it."** There is no automatic application.
- **Collaboration (§38):** another user changing an affected object forces revalidation. [SEC §20.7]: "Collaboration does not weaken stale-base protection."
- **[SEC §34.6] bad pattern:** "AI prepared a Change Set, project changed, AI applies the old proposal anyway." Correct: "Revalidate stale state and present a new/updated review path."

### 13.6 Batch operations (§21)
- Examples: "Create Scene Cards for these 12 selected beats.", "Add these five confirmed breakdown elements to every selected scene.", "Rename this location everywhere it is structurally referenced."
- Flow: Request → Resolve target set → **Show count** → Prepare Change Set → Show affected modules → User selects/accepts → Apply.
- Large-batch preview must include: total objects; grouped affected modules; exclusions; conflicts; locked objects; permissions; expected result; irreversible actions.
- Example: **"This will modify 263 references across 7 modules. 11 references are excluded because they are locked. 4 are ambiguous. Review required."**

### 13.7 Destructive operations (§19.6)
- Delete, permanent deletion, and destructive replacement need an "explicit destructive confirmation consistent with normal OpenFrame UI behavior."
- "Must not interpret 'clean this up' as authorization to delete."
- Correct: **"I found 14 unused Scene Cards. Do you want me to prepare them for deletion?"**
- Incorrect: "I deleted the unused cards."
- App. B Ex 4: "I found 14 Scene Cards currently parked. I can prepare them for deletion, but none will be deleted automatically. Review the 14 cards?"
- **[SEC §14.6]** deletion path: Request → Permission → State checks → Proposed Change Set → Preview → User acceptance → Normal application mutation.

## 14. SAFE GLOBAL RENAME / REPLACE (§12) — the "rename" flow

- "Replace across the software" is a **structured project-wide change operation**, "not … blind text replacement."
- **Layers a rename may affect:**
  1. canonical object names;
  2. structured object references;
  3. screenplay Character elements;
  4. cast references;
  5. breakdown references;
  6. Story Board references;
  7. shot/storyboard references;
  8. tasks/notes *when explicitly selected*;
  9. other supported structured references.
- **FSD rule:** "Renaming a Character record must not blindly modify arbitrary screenplay Dialogue or Action text." Structured identity/reference changes and arbitrary textual replacement are separate. The UI "must present them as distinct change categories."
- **Preview template (verbatim structure):**
  ```
  Canonical Character  Ravi → Raghav   1 object
  Structured screenplay Character elements   31 occurrences
  Cast references   4 occurrences
  Breakdown references   16 occurrences
  Scene Card references   8 occurrences
  Dialogue/Action raw text   12 occurrences   NOT INCLUDED BY DEFAULT
  ```
  The user may explicitly opt in to arbitrary text.
- **Rename preview must show (§12.4):** old value; new value; affected objects; affected modules; occurrence counts; excluded content; historical/snapshot impact; conflicts; locked content; permission limitations.
- **Historical (§12.5):** snapshots, exported documents, prior issued call sheets, and immutable historical artifacts are not silently rewritten. The preview explains whether the change affects only active canonical data or also explicitly selected historical materials.
- **Application (§12.6):** Approved Change Set → Validation → Apply → Undoable application action → Activity entry.
- **Conversational example (App. B Ex 3):** "I can prepare that. I found one Character record named Ravi and 17 structured references. I also found 12 raw text mentions in dialogue/action, which are separate from structured references. Review scope?"
- **Applied-message examples:** "Applied the accepted rename to 17 structured references." (§29). Activity: **"AI-applied: renamed Character Ravi → Raghav across 17 structured references."** (§32.3)

## 15. PERMISSION AND APPROVAL MODEL (§19)

- **4 layers, all must pass:** Layer 1 User authorization → Layer 2 Project permission → Layer 3 Object/state restriction → Layer 4 Mutation confirmation.
- **User authorization (§19.2):** the user can ask to read, calculate, navigate, suggest, prepare, or modify. "The request establishes intent, but modification still requires explicit acceptance."
- **Role examples (§19.3):**
  - "Viewer cannot mutate."
  - "Commenter cannot mutate screenplay."
  - "Contributor can mutate only allowed content."
  - "Owner can perform owner-only operations."
  - "AI cannot elevate a Viewer into an Editor."
  - ⚠ **Terminology conflict:** "Contributor" is not a canonical security role. [SEC §6.1] fixes the role set to exactly **Owner, Editor, Commenter, Viewer, Export-only**, and its Terminology Lock says "Editor — do not replace with Contributor". Implement **Editor**. [SEC §16.4] restates this as "Editor can mutate only permitted content".
- **Object/state restrictions (§19.4):** locked drafts; approved artifacts; stale documents; archived objects; deleted/recoverable objects; private notes; unavailable external files; collaboration conflicts.
- **Permission denial (§31.4):** "The assistant explains that the requested action is not permitted for the current user. It must not suggest bypasses."
- **[SEC §34.4] bad example:** "I know you are a Viewer, but the owner probably intended you to change this."
- **[SEC §7.4]:** denial must not reveal protected private information "merely to explain the denial."
- **[SEC §65 Scenario 1]:** Viewer asks AI to rename → intent resolved → permission check fails → "No Change Set can become applicable" → project unchanged.

## 16. PRIVATE-NOTE BOUNDARIES (§23; [SEC §8, §10.2, §34.1])

- Private notes follow existing project permissions.
- **Own notes:** usable "when the current user asks to use them and the operation is permitted." [SEC §8.4] adds that the user must be the owner/authorized private context, the request must be permitted, and the context must be intentionally included.
- **Others' notes:** "must never be exposed through AI."
- **Cross-user query (§23.2):** "What is the writer secretly planning?" If the only source is another user's private notes, deny and explain the content is private. "Must not reveal the existence or contents of the protected text beyond what the permission model allows."
- **[SEC §8.5]:** the refusal must not reveal the note body, protected detail, hidden private-note content, or "a transformed version of the private content".
- **[SEC §34.1] leak example (prohibited):** "I cannot show the note, but the writer is planning to kill the producer." Correct copy: **"The requested information is private and cannot be accessed in this context."**
- **[SEC §10.2]:** private notes must not be discoverable via AI semantic retrieval, project-wide relationship lookup, global search by another collaborator, or collaboration session retrieval. **Implementation consequence:** the retrieval index or permission filter must exclude non-owned Private Notes *before* anything reaches the model context.
- **[SEC §65 Scenario 7]:** Story-only LAN share, participant asks AI for private screenplay info → no screenplay/private-note retrieval.
- AI-AC-013 / SEC-019 / SEC-AC-007.

## 17. PROMPT-INJECTION / UNTRUSTED-CONTENT RULES

The AI spec has **no section titled "prompt injection"**. The applicable rules are:
- **[SEC §25.11] Imported content remains data:** "Imported text does not become an OpenFrame command simply because it contains words resembling commands or instructions. The import layer's job is to interpret supported document structure, not to execute arbitrary imported content as application operations."
- **[SEC §16.7] No authority delegation to model output:** "Model text is not permission." Model output cannot grant itself storage authority.
- **[SEC §34.4–34.5]:** no permission bypass through natural language; "Model-generated tool parameters cannot elevate authorization."
- **§2.2 / §2.3:** only a *user request* may start an action; nothing runs autonomously. **Consequence:** instructions inside screenplay text, notes, files, or imported packages must never trigger tool calls or mutations. At most they surface as proposals that the user must accept.
- **§28 validation** runs on every tool request regardless of what the model says, including the permission, object-state, and lock checks.
- **§34:** no unrestricted filesystem access, which limits the blast radius.
- **§39:** exchange packages cannot overwrite, merge, or discard silently.
- **§40 / §2.4:** do not hallucinate inaccessible file content.
- Invariant #2 (§51): "AI requests are user-originated."
- Any concrete injection defenses (delimiting retrieved content, instruction hierarchy, etc.) are **NOT SPECIFIED** and are implementation choices that must satisfy the above.

## 18. DISCLOSURE / PROVENANCE RULES

### 18.1 Response categories (§29), each distinct in wording
- **Answer:** "Draft 8 contains 13 distinct character cues."
- **Observation:** "The current Production Source is Draft 8."
- **Suggestion:** "I found three likely props in Scene 14."
- **Proposed action:** "I prepared changes for 17 structured references."
- **Applied action:** "Applied the accepted rename to 17 structured references."
- "The assistant must not describe a suggestion as if it were applied."

### 18.2 Provenance (§30)
- For exact or consequential answers, identify: source scope; draft/version; object category; filters; calculation basis; exact vs inferred.
- Model answer: "There are 8 distinct locations in Draft 7, based on parsed screenplay scene headings. The Production Catalog currently contains 10 Location records." (preferred over "There are 8 locations.").
- **Confidence (§50.2):** `confidence_state` = `Exact/Inferred/Unavailable`. "'Confidence' must not become a numeric pseudo-certainty score for exact project facts. Exact deterministic results should be marked as exact."
- **[SEC §17.3]:** provenance must be sufficient "without exposing unauthorized data". **[SEC §42]:** derived summaries must not leak data from a broader hidden scope.

### 18.3 "What happened" states (§41.2), which the UI must differentiate
"AI answered." / "AI suggested." / "AI prepared changes." / "User accepted." / "Application applied." — "These are not the same state."

### 18.4 Local vs external disclosure (§24) — ⚠ SUPERSEDED (LOCAL AI ONLY)
- §24.1 Local AI is "preferred" for private work, offline use, low latency, and users who do not want transmission. **→ It is now the only path.**
- §24.2 "External AI is optional… user must be informed before transmission… state: external provider being used; what context; category of info; provider/model identity where practical… able to cancel before transmission." **⚠ SUPERSEDED.** No external path is built, so there is no disclosure dialog.
- §24.3 "The product must never imply that an external request is local. Likewise, if local AI is being used, the UI must not claim that data is being sent to a cloud provider." **→ The second half STILL APPLIES:** local-only UI must truthfully say processing is local (SEC-024, SEC-AC-021).
- §24.4 External AI never gets write authority. **⚠ SUPERSEDED** (moot). Mutations are local-only regardless.
- §31.3 "External provider failure — Project state remains unchanged." **⚠ SUPERSEDED.** The equivalent is sidecar/model failure → §31.1.
- §41.1 UI state "External Disclosure". **⚠ SUPERSEDED.** Drop it or leave it unused.
- §50.1 `external_processing_state` (Local/External/Not Sent) and `model_reference` (provider/model descriptor). **⚠ Partially superseded.** The field can be kept as the constant `Local` (or omitted). `model_reference` is still useful to record the local GGUF model identity.
- §51 invariant 14: "External provider usage is represented when applicable." Never applicable.
- §54 UX "local/external AI indication" → show a **local** indicator only.
- §55 "optional model/provider reference" → model reference only.
- AI-AC-017 "External disclosure" **⚠ SUPERSEDED** (N/A). AI-AC-018 "Offline local AI" **applies fully**.
- §62 DoD "External transparency — External AI processing is explicitly disclosed" **⚠ SUPERSEDED**.
- §63 UX checklist "External AI disclosure retained" **⚠ SUPERSEDED**.
- FSD-AI-011 "External disclosure" (§52) **⚠ SUPERSEDED**.
- §61 future "optional larger models; optional specialist models". These remain possible *locally*.
- §60 "must not become … a hidden cloud synchronization mechanism". Consistent with local-only.
- **API keys / BYOK:** never mentioned in the AI spec. There is nothing to supersede, and none should be added.

## 19. AI PERSISTENCE (§32, §33, §50)

### 19.1 Conversation (§33, §18.1)
- "Conversation history is **optional** and must remain distinct from project content."
- The AI must not silently convert Conversation → Project Note / Scene Card / Screenplay / Breakdown / Task without explicit acceptance.
- "Save that as a Project Note." → prepare a creation → confirm.
- Retained items: prior requests, resolved references, current scope, pending proposal, explanation preferences.
- **[SEC §44.3]:** "AI conversation history may be retained according to product settings." There is no universal retention period. "The implementation must not invent a fixed retention promise in product documentation unless separately specified."
- **[SEC §4.5]:** AI interaction data (Class E) "may itself contain project-sensitive content" and follows the user/project privacy boundary.
- **[SEC §15.5]:** AI history ≠ Activity. There is no per-token audit.

### 19.2 AI Request fields (§50.1)
| Field | Meaning |
|---|---|
| request_id | stable request identity |
| user_id | requesting user |
| project_id | project scope |
| session_id | conversation/session context |
| scope | resolved context |
| request_text | original request |
| intent | interpreted operation |
| operation_class | Read/Compute/Navigate/Suggest/Mutate |
| target_objects | resolved targets |
| authorization_state | permission result |
| external_processing_state | Local/External/Not Sent — ⚠ always Local |
| model_reference | provider/model descriptor if applicable (use: local model id) |
| created_at / completed_at | timestamps |
| status | lifecycle state (Appendix C) |

### 19.3 AI Result fields (§50.2)
| Field | Meaning |
|---|---|
| result_id | stable identity |
| request_id | source request |
| content | user-facing answer |
| structured_data | returned deterministic values |
| provenance | source scope/version/objects |
| suggested_changes | optional Change Set |
| confidence_state | Exact/Inferred/Unavailable |
| status | Informational/Pending Approval/Accepted/Rejected/Applied/Failed |
| error | optional failure info |
| created_at | timestamp |

[SEC §17.2]: the Result "must not contain another user's restricted private information merely because the model could infer or retrieve it."

### 19.4 AI Tool Invocation (§27, §50.4)
- Fields: `invocation_id`, `request_id`, `tool_name`, `parameters`, `target_objects`, `authorization_state` (allowed/denied), `execution_state` (pending/running/succeeded/failed), `result_reference`.
- "Internal supporting construct unless later promoted into a user-visible audit feature."

### 19.5 AI Context Reference (§50.5)
Current project, draft, scene, selected objects, project scope, permission context. Supports explainability and safe previews.

### 19.6 Activity (§32.3)
- Meaningful applied AI actions create an activity entry with: **actor = current user; origin = AI-assisted action; action; affected scope; time; result.**
- "Not a transcript of every token or internal reasoning step."
- [SEC §15.4] base Activity fields: `activity_id`, `project_id`, `actor_user_id`, `action_type`, target type/ID, timestamp, human-readable summary.

### 19.7 Undo / Recovery (§32.1–32.2)
- Accepted AI mutations are normal undoable actions wherever the underlying op is undoable.
- "AI failures must never destroy the last safe local state."

## 20. UI STATES AND MESSAGES (§31, §41, §54)

### 20.1 Panel structure (§41, right-side AI panel)
```
AI Assistant
Scope [ Current Scene ▾ ]
Context  Using: Scene 24 + Characters + Breakdown
Conversation
Answer / Result
For mutations: Proposed Changes · Affected objects · Impact · Conflicts · [Review Changes] [Apply] [Cancel]
```

### 20.2 Required states (§41.1, minimum)
`AI Off`, `Ready`, `Interpreting`, `Retrieving`, `Generating`, `Answer`, `Proposal`, `Awaiting Approval`, `Applying`, `Applied`, `Rejected`, `Stale`, `Conflict`, `Failed`, `External Disclosure` (⚠ SUPERSEDED — omit), `Unavailable`.

### 20.3 State → exact copy / behavior map
| Requested state | Spec source | Exact message / behavior |
|---|---|---|
| **Unavailable** (model down) | §31.1 | **"AI is currently unavailable. OpenFrame's core workflows continue to work normally."** Project state unchanged |
| Tool unavailable | §31.2 | **"I can explain the workflow, but I cannot retrieve the project count right now."** "Do not invent a result." |
| **Denied** (permission) | §31.4 | Explain the action "is not permitted for the current user"; "must not suggest bypasses". Exact copy NOT SPECIFIED |
| Denied (private note) | [SEC §34.1] | **"The requested information is private and cannot be accessed in this context."** |
| **Stale** | §31.6 | **"The project changed after this suggestion was prepared. Review is required before applying it."** No auto-apply |
| Conflict | §20.3, §38 | Conflict review path; exact copy NOT SPECIFIED |
| Ambiguous | §31.5, §6.3 | "Ask one focused clarification." e.g. "There are two characters named Raju: `Raju` and `Raju Kumar`. Which one should I use?" |
| **Answer** | §29, §30 | Answer + provenance + scope ("Using: …"), Exact/Inferred marker |
| **Navigate** | §5.3 | Executes directly; no confirmation; no mutation. Exact copy NOT SPECIFIED |
| **Suggest** | §29, §13.2 | "I found three likely props in Scene 14." Suggested items visually distinct from Confirmed |
| **Rename** | §12.3, App. B Ex 3 | Categorized preview with raw text "NOT INCLUDED BY DEFAULT"; "Review scope?" |
| Proposal / Awaiting Approval | §19.5 | Apply / Cancel; batch: Review Changes / Select Changes / Apply Selected / Cancel |
| Applied | §29, §32.3 | "Applied the accepted rename to 17 structured references." |
| Partial failure | §31.7 | Successful/pending/failed groups; no full-completion claim |
| AI Off | §41.1, AC-019 | Core product fully usable |

### 20.4 UX additions (§54)
- command entry from the application shell;
- contextual AI actions in workspaces;
- clear answer-vs-mutation distinction;
- batch preview;
- affected-object counts;
- structured vs raw-text rename choices;
- stale Change Set state;
- permission denial state;
- local/external indication (→ local only);
- product-data provenance;
- "no autonomous-action indicators".
- "The UX should not create a separate 'AI application' disconnected from the OpenFrame shell."

## 21. DOMAIN INVARIANTS (§51)

1. AI-generated content is not authoritative until accepted.
2. AI requests are user-originated.
3. AI cannot own canonical story/production truth.
4. Applied AI changes become normal project mutations.
5. Proposed AI changes have explicit base-version context.
6. AI Change Sets cannot apply across invalid/stale bases without revalidation.
7. User permissions always constrain AI actions.
8. Private data access follows normal authorization.
9. Historical snapshots are not silently rewritten.
10. AI cannot bypass object lifecycle rules.
11. Navigation and read operations do not mutate project content.
12. Suggested changes do not become project data automatically.
13. Rejected changes have no project mutation effect.
14. External provider usage is represented when applicable. ⚠ N/A under LOCAL AI ONLY.
15. Exact project facts are derived from authoritative data.

## 22. FSD traceability (§52, §63)

- Existing: FSD-AI-001 Optional · 002 Context selector · 003 Question answering · 004 Scene card suggestion · 005 Breakdown suggestion · 006 Synopsis · 007 Schedule advice · 008 Mutation preview · 009 Accept/reject · 010 No silent rewrite · 011 External disclosure (⚠ SUPERSEDED) · 012 AI failure safety.
- Expansion IDs FSD-AI-013..027 (§63). The Security spec cites: FSD-AI-019 Permission inheritance; 020 Locked/private/approval enforcement; 021 Change Set base-version validation; 022 Application tool boundary and validation; 024 Scope/provenance disclosure; 025 AI lifecycle/activity integration; 027 Cross-project/episodic scope control.
- New areas to cover: system-wide commands, deterministic statistics, exact queries, cross-module traversal, navigation commands, structured replace, batch ops, base-version validation, permission inheritance, locked-state enforcement, private-note protection, product knowledge, tool invocation validation, audit/activity, stale proposals, lifecycle, cross-project boundary, episodic scope, collaboration/conflict, model-agnostic operation.

## 23. ACCEPTANCE CRITERIA (§46, all 25)

| ID | Name | Criterion |
|---|---|---|
| AI-AC-001 | Read-only answer | Answers a supported question without changing content |
| AI-AC-002 | Exact deterministic count | Counts come from the canonical data/query layer |
| AI-AC-003 | Scope disclosure | Context-dependent query shows or states the scope |
| AI-AC-004 | No hallucinated exact facts | If a fact can't be obtained, don't invent it |
| AI-AC-005 | Tool-based navigation | Open supported objects/workspaces by natural language |
| AI-AC-006 | Suggestion without mutation | Breakdown suggestion creates nothing until accepted |
| AI-AC-007 | Mutation preview | Every project-changing action previews first |
| AI-AC-008 | Explicit acceptance | Rejecting leaves content unchanged |
| AI-AC-009 | Normal application mutation | Accept produces the same canonical object/action as the UI |
| AI-AC-010 | Undo | Undoable wherever the underlying op is |
| AI-AC-011 | Permission inheritance | Cannot mutate what the user cannot |
| AI-AC-012 | Locked-state enforcement | Cannot bypass lock/revision |
| AI-AC-013 | Private-note protection | Cannot expose another user's private notes |
| AI-AC-014 | Stale Change Set detection | Outdated-base Change Set revalidated before apply |
| AI-AC-015 | Global rename safety | Structured vs arbitrary text distinguished; explicit choice for arbitrary |
| AI-AC-016 | Batch safety | Batch shows counts and objects before apply |
| AI-AC-017 | External disclosure | ⚠ SUPERSEDED (N/A — no external path) |
| AI-AC-018 | Offline local AI | Local AI works without internet within installed model/tools |
| AI-AC-019 | AI optionality | Core product usable with AI disabled |
| AI-AC-020 | AI failure safety | Failure leaves local data unchanged |
| AI-AC-021 | Cross-module query | Traverses relationships across workspaces |
| AI-AC-022 | No autonomous mutation | No change without explicit acceptance |
| AI-AC-023 | Product knowledge | Answers OpenFrame workflow questions from the current product knowledge source |
| AI-AC-024 | Conversation context | Resolves simple follow-ups without corrupting state |
| AI-AC-025 | Model replacement | Swapping the model doesn't change data semantics or authorization |

**Normative query examples (§47) for test fixtures:**
- 47.1 "How many characters are in this script?" → distinct Character elements (+ optional Character Room compare).
- 47.2 Location stats (screenplay vs Catalog).
- 47.3 "Which scenes contain both Ravi and Arjun?"
- 47.4 Draft comparison.
- 47.5 "Break down Scene 24." → suggestions, acceptance required.
- 47.6 "Rename Ravi to Raghav across the project."
- 47.7 "What scenes are unscheduled?"
- 47.8 "Create a call sheet for Day 4." → preview, explicit acceptance before creating.
- 47.9 "Find every mention of the railway station across the project." → group by type/module; structured vs raw text.
- 47.10 "Why is my call sheet marked stale?" → product rule + actual version relationship.

**App. A command corpus** (Read/Compute 15, Navigation 7, Suggest/Prepare 7, Mutate-with-approval 7). Use it as an intent-classification test set:
- Mutate set: "Create the Scene Card.", "Apply the accepted breakdown suggestions.", "Rename Ravi to Raghav.", "Apply the selected schedule changes.", "Create the call sheet.", "Add this as a Project Note.", "Delete these selected Scene Cards."
- App. B Ex 5: "Fix the schedule." → do not guess. "I found two scheduling conflicts: cast overlap on Day 3 and a location conflict on Day 5. I can prepare proposed changes for review."

**Definition of Done (§62):**
- product understanding; project understanding; exactness; command capability; mutation safety; cross-module; batch; permission; version safety; privacy;
- offline/local operation ("Local AI can operate without cloud dependence when configured");
- ~~external transparency~~ ⚠;
- recovery; normal integration.

**Governing formula (§61, App. E):** User request + Permission + Preview where mutation occurs + Explicit acceptance + Normal application action. "Any AI behavior that bypasses scope → permission → proposal → confirmation → validation → normal application mutation is inconsistent with the OpenFrame AI contract."

## 24. Gaps the spec leaves open (do not invent; decide explicitly)

- Exact copy for permission-denied, conflict, navigate-success, and suggest states. Only the messages quoted above are given.
- Concrete prompt-injection defenses (only the "data not commands" rule exists).
- Conversation retention period and settings UI (only "according to product settings").
- The product-knowledge source format ("versioned … must exist conceptually").
- Scheduling eligibility rules (deferred to "canonical scheduling eligibility rules").
- Tool names are "Examples". The final tool surface is ours, but it must cover §10 categories and §45 operations.
