# AI re-harness: section-scoped assistant

Status: Accepted 2026-11-03 as [ADR-0040](../adr/ADR-0040-section-scoped-assistant.md). Text from the owner's release package (`ai-reharness.md`). Build status is at the end of this page.
Applies to: Windows app first (Rust core). The Mac shell reuses the same core and needs its own picker.
Model: unchanged. Gemma 3 1B, local, offline (ADR-0025, ADR-0026).
Code read: GitHub `main`, tree `10e5d7b`, on 2026-11-02. Read, not run.

## A. In plain words (read this first)

### The problem today

- The assistant has a "Scope" dropdown (Current Screenplay, Beat Board, Production Workspace, Whole Project). It looks like it controls what the AI reads. It mostly does not. It changes a label.
- The AI works out the topic by itself from keywords in your question. Keywords collide ("timeline", "planned", "when"), so it often reads the wrong part of the project and answers badly.
- Answers are tuned for short facts. Asking it to explain a scene or describe a character gives thin or invented text.

### The new idea

- Think of the project as a building with labelled rooms: Story, Screenplay, Breakdown, Production, Call Sheets, Files, and so on. Each room has smaller rooms (Production has Budget, Cast & Crew, Locations...).
- Before you ask, YOU tell the assistant which rooms to look in. Up to 3.
- The assistant looks only in those rooms. It does not guess, and it does not wander into other rooms.
- Result: it knows exactly where to read, so answers are more accurate and it has less room to make things up.

### What you see

1. Open the assistant. Nothing is pre-selected. Send is disabled until you pick at least one section.
2. Pick sections from a tree. Chosen ones show as chips. Some sections ask one more thing (which draft, which scene, which shooting day).
3. Ask your question. The answer says what it used, for example "Using: Screenplay (Draft 2, all scenes) + Cast & Crew".
4. Ask follow-ups. Your selection stays for the conversation. Change it any time.

### Three examples

- Good pick. Screenplay + Cast & Crew. "How many scenes does Ravi play in, and describe his character?" The scene count comes from the project data, the description comes from his scenes and cast record. Both rooms are open, so it answers fully.
- Wrong pick. You picked only Budget and ask "Who is in Scene 4?". The assistant does not answer from memory or peek into Screenplay. It says: "That is in Screenplay. Add it to your selection and ask again." One tap adds it.
- Long answer. Story + Screenplay. "Explain what happens in the second act." You get a proper multi-sentence explanation built from the actual scenes, not a one-line fact.

### What changes and what does not

| Changes | Stays the same |
|---|---|
| You choose the sections, not the AI | Same local model, offline, nothing leaves the machine |
| Explicit "not this section, try X" pointer | Facts like counts, dates, budget are still read straight from the data, not guessed |
| Richer answers for scenes, characters, relationships, story | Every answer is still checked by code before you see it |
| The "Using:" label shows what was really read | Commands (open scene, suggest breakdown) still run through the agent, and only a person can apply changes |

### What it will not do

- It will not decide on its own which section to read.
- It will not answer from a section you did not pick.
- It will not have a "Whole Project" mode. Need two or three areas? Pick them (max 3).

## B. How it works (technical overview)

### Core principle

The user is the router. STAIR's idea (structure as the address of information) is kept, but the "address" is a hand-built leaf registry that the user selects from, so no router model or fine-tuning is needed. The model only writes. Everything before and after the model is deterministic code.

### Pipeline

```
Panel (leaf picks 1..3 + instance picks + conversation id + text)
  -> permission check (UseAi, role)
  -> name resolver, limited to entities homed in picked leaves
       -> name homed elsewhere: pointer (code, no model)
  -> answer-type read (lookup, list, count, explain, relation, compare,
       brainstorm, describe, brief)
  -> fetchers of picked leaves only   (typed queries, authoritative facts)
  -> leaf-scoped hybrid retrieval     (text: FTS + vectors + graph, prefiltered
       by nav path prefix + owner privacy)
  -> dossier assembler                (split token budget across leaves)
  -> writer (Gemma 3 1B)              (the only model call)
  -> deterministic checker, max 3 passes
  -> answer + sources + truthful "Using:" label
  -> code-written fallback if checks fail
```

### What is code and what is model

| Step | Who |
|---|---|
| Which leaves to read | User |
| Permissions, name resolution, answer type, fetching, retrieval, budgeting | Code |
| Out-of-section pointer | Code |
| Turning the dossier into prose | Model |
| Checking names, numbers, scene refs, length, repetition | Code |

### Key components

- Leaf registry (Rust, about 35 leaves, section 3.2). One entry per screen: path, fetchers, search filter, instance picker, token weight, minimum role. A coverage test fails the build if any screen or indexed entity type maps to zero or several leaves. This is what stops the "five overlapping taxonomies" problem returning.
- Request payload. The panel sends leaf ids and instance picks. The core has no field for these today (`AiScopeArgs` lacks workspace or tab), so R2 adds it.
- Dossier by leaves. Facts from typed fetchers are never cut. Text passages fill the leftover budget. Budget is split across picked leaves by weight with a minimum share per leaf, so one big leaf cannot starve the others (section 3.4).
- Retrieval. Existing hybrid search (BM25 + vector + graph, rank fusion) finally gets wired into the question path, restricted by `search_doc.nav_json` path prefix plus the owner privacy filter. If the index is not ready, it falls back to full-text inside the picked leaves and says so.
- Pointer. Triggers: a resolved name homed in an unpicked leaf, an existence check that finds the thing in an unpicked leaf, or a topic word hinting at one. It names a leaf, never content, and checks permission first, so it cannot leak data.
- Narrative answers. New types (describe, brief, relationship, story explanation) with higher word limits, briefing dossiers where every line carries its scene number, and a writer policy of "state only what the facts show".
- Agent. Tool list is filtered to tools homed in the picked leaves. The agent loop, Change Sets and human-only apply are unchanged.

### Today vs new

| Aspect | Today | New |
|---|---|---|
| Who picks the area | AI, by keyword regex | User, by leaf selection |
| Scope dropdown effect | Label only (`scope.context` unused on Q&A) | Hard boundary on facts, retrieval and tools |
| Taxonomies | 5 overlapping | 1 leaf registry with coverage test |
| Extra data pulled in | Auto-added domains (explaining a scene also pulls breakdown and schedule) | Only picked leaves |
| Text retrieval | Plain FTS, 3 to 6 passages, wide | Hybrid, prefiltered to picked leaves |
| Wrong area | Silent wrong or thin answer | Code-built pointer naming the right leaf |
| Long answers | Capped by fact-style policy | New answer types with narrative dossiers |

### Cost and failure modes

- Latency. A 300 word answer is about 400 tokens, roughly 12 to 18 s of writing plus repeat passes on CPU. Progress lines, no streaming. Needs a real latency run (section 10, item 4).
- Wrong pick leads to a pointer. Tracked as false-pointer rate. One tap fixes it.
- Split budget thins each leaf. Weights tuned on the evaluation set.
- 1B model may embellish long prose. Defences: briefing dossiers, the checker, and showing the owner the numbers if the bar is missed.
- Nothing here is built. Order and gates are in sections 6 and 7 (measure first, R0 to R7).

## 1. Decisions locked

1. The user picks the sections (branches) before asking. The AI never decides where to look.
2. Multi-select. Maximum 3 branches per question.
3. No pre-fill from the current screen. No "Whole Project" option. Send stays disabled until at least one branch is picked.
4. A question that spans sections is handled by picking more than one branch.
5. If the question belongs to a section that is not picked, the assistant says so and names the section to add ("not this section, try that one"). It never answers from outside the pick.
6. Focus is question answering, including long conversational answers: scene briefings, character descriptions, relationships, story explanations. Commands (open scene, suggest breakdown, create task) stay with the agent and are fixed to an extent (section 3.9).
7. Measure first. ADR-0028 decision 1 stays.

## 2. What the code does today

- `scope.rs` has 10 scope kinds. The picker shows 4: Current Screenplay, Beat Board, Production Workspace, Whole Project. `resolve()` builds up to 6,000 characters of context.
- The question path is `orchestrator.rs`: read the message type, resolve names, build the dossier, write, check. It does not receive `scope.context`. Only `scope.draft` and `scope.scene` pass through. `scope.kind` feeds the trace and agent tool scoring (`toolbox.rs`). So picking Beat Board changes the label, not the facts.
- Whole Project resolves to no context items at all.
- `retrieval.rs` and `intelligence/` hold a hybrid search (full-text, vectors, graph, rank fusion). I found no caller of `retrieval::retrieve` or `retriever::gather` in the application source. The only text search on the question path is `dossier/fetch.rs` `passages()`: plain full-text, 3 to 6 passages, 300 characters each, lowest priority.
- `turn/domains.rs` finds the topic by keyword regex over 11 domains. Collisions: "timeline" maps to Schedule while Story has a Timeline tab; "planned" maps to Budget; "when" maps to Schedule.
- `AiScopeArgs` has no field for workspace or tab.
- Five overlapping taxonomies: scope kind, domain, retrieval route, answer type, left navigation.
- Answer policy (`answer/prompt.rs`): plain sentences, no lists, no markdown. Length classes: one sentence 30 words, short 80, paragraph 200, brief 40. Deterministic checker, three passes, then a code-written fallback. Good for facts, thin for explanations.
- Model limits (ADR-0037, S8 measurement, processor engine): writes 22 to 33 tokens a second, reads a prompt at 80 to 160 tokens a second. Prompt about 3,000 tokens. Dossier cap 1,500 tokens (2,500 for a paragraph).

## 3. Design

### 3.1 Branch tree

Only leaves can be picked. Maximum 3 leaves per question. A leaf is one screen of the app. Hand-authored registry in Rust, about 35 leaves.

| Branch | Leaves | Instance picker |
|---|---|---|
| Idea Vault | This Project, Global | none |
| Story | Story Page, Beat Board, Outline, Characters, Timeline, Episodes (episodic projects only) | none |
| Screenplay | Scenes | screenplay (required when the project has more than one), draft (default: current), scene (All, or one) |
| Breakdown | Breakdown | scene (All, or one) |
| Production | Overview, Catalog, Locations, Cast & Crew, Moodboards, Storyboards, Shot Lists, Schedule, Day Out of Days, Daily View, Sides, Reports, Budget | scene for Shot Lists, Storyboards, Moodboards, Sides; shooting day for Schedule, Daily View, Day Out of Days |
| Call Sheets | Call Sheets | shooting day or call sheet (All, or one) |
| Files | Files | none |
| Notes & Tasks (added) | Notes, Tasks | none |
| App Help & Settings (added) | Product Guide, Settings | none |

Rules:
- Instance pickers are for things that have many items (scenes, drafts, days, call sheets). Names inside a question (a character, an actor, a location) are resolved by the existing name resolver, restricted to the picked leaves.
- Idea Vault Global and This Project are two leaves, not an instance, so the user sees them in one list.
- Private notes stay visible only to their owner, in every leaf.

### 3.2 Registry

One entry per leaf. Replaces scope kind, domain and retrieval route for questions.

- id and path (for example `production/cast-crew`)
- label and aliases
- fetchers (typed queries, authoritative)
- search filter (path prefix on `search_doc` `nav_json` workspace and sub, plus the owner privacy filter)
- instance picker type, if any
- token weight
- minimum role (for example contact details only for Owner and Editor)
- example questions (used for tests only)

A coverage test fails the build when any navigation screen or any `search_doc` entity type maps to zero leaves or more than one. The existing `FETCHERS` coverage test is the model.

### 3.3 One question, start to end

1. The panel sends: the text, the picked leaves (1 to 3), the instance picks, the conversation id.
2. Permission check (UseAi, role).
3. Name resolution, restricted to entities whose home leaf is picked. Names homed elsewhere go to the pointer (3.6).
4. Answer type read (lookup, list, count, explain, relation, compare, brainstorm, describe, brief). The existing rules and example vote stay.
5. Fetchers of the picked leaves only. Nothing is added automatically. This removes the current rule that explaining a scene also pulls its breakdown and schedule.
6. Branch-scoped text retrieval (3.5).
7. Dossier built with a split budget (3.4).
8. Writer, then deterministic checker, at most three passes.
9. Answer, sources, and a truthful label: "Using: Screenplay (Draft 2, all scenes) + Cast & Crew".

### 3.4 Token budget

- The whole prompt stays about 3,000 tokens until the baseline says otherwise.
- The dossier share is split across the picked leaves by weight. A leaf that needs less gives its spare share to the others.
- Facts that carry a required value are never cut (existing rule).
- Each leaf has a minimum share so one large leaf cannot crowd out the rest.
- Trade-off: reading 3,000 tokens at 80 to 160 a second is slow on a processor. Keep the stable prompt start so the engine reuses its stored work. Do not raise the cap without a latency run.

### 3.5 Retrieval inside the picked leaves

- Structured facts (counts, cast, budget, dates, status) come from typed queries and are authoritative.
- Text (scene text, notes, story page, files) uses the existing hybrid search, pre-filtered to the picked leaves by path prefix and owner. This wires the vector and graph work into the question path.
- If the index is not ready, fall back to full-text inside the picked leaves and show the existing "Preparing project context" notice.
- Scene chunks keep the short contextual header (act, heading, characters, location) from ADR-0037 decision 7.

### 3.6 Out-of-section pointer

Code only. No model call. The model never decides where to look.

Triggers:
- A resolved name whose home leaf is not picked (a character asked about in Screenplay when only Budget is picked, for example).
- The picked leaves return nothing for the asked thing, and a cheap existence check finds it in an unpicked leaf.
- A topic word that points to an unpicked leaf. The old keyword table is demoted from router to hint.

Behaviour:
- Fixed sentence built by code, naming the leaf: "That is in Production > Budget. Add it to your selection and ask again."
- No facts from unpicked leaves appear in the reply.
- Partial answer when part of the question fits the pick: answer that part, point for the rest.
- Permission check before pointing, so the pointer never reveals data the user may not read. It names a leaf, never content.
- Nothing found anywhere: the existing sentence "I could not find X. I looked in A and B." with A and B being the picked leaves.
- The reply can carry an "Add <leaf>" button. At 3 leaves it says to remove one first.

### 3.7 Conversational answers

The current policy is tuned for facts. Narrative questions need more room and better material.

- New answer types: describe (character, location, scene), brief (a scene or sequence), relationship, story explanation. Starting limits, to be tuned on the baseline: describe, brief and relationship up to 250 words; story explanation up to 350 words. The user can still override in words ("briefly", "in detail").
- Lists are allowed only for scene-by-scene briefings. Everything else stays plain prose.
- Dossier for narrative answers is a code-built briefing: scene text excerpts, character cards, relationship records, every line carrying its scene number. ADR-0035 had briefings and S7 deleted them. This brings the idea back as dossier builders per leaf.
- Writer policy: describe only what the facts show. No motives, feelings or backstory the text does not state. A relationship is stated only from a relationship record.
- Checker keeps: names, numbers, scene references, required facts, repetition, language, length. Re-add, where cheap in code, the unsupported relationship word and unstated mood checks that ADR-0035 listed. Measure false positives before turning them on.
- Cost: a 300 word answer is about 400 tokens, so roughly 12 to 18 seconds of writing plus any repeat passes. Progress lines only, no streaming (owner decision of 2026-10-24 holds).
- Risk: the 1B spike showed paragraphs embellish. Briefing dossiers and the checker are the defence. If the bar is missed, show the owner the numbers. Do not lower the bar silently.

### 3.8 Interface

- Hierarchical multi-select with chips. Maximum 3 leaves. No default selection. Send disabled with the hint "Pick a section".
- Instance pickers appear under the chosen leaf.
- The selection is kept for follow-up questions in the same conversation. A new conversation starts empty. (Default taken, see section 8.)
- The label is built from the real selection and written to the trace.
- Mac: same payload from the Rust core. SwiftUI picker added in the Mac track.

### 3.9 Commands and the agent (to an extent)

- The tool list offered to the agent is filtered to tools homed in the picked leaves. This replaces scoring tools by the scope label in `toolbox.rs`.
- "Open Scene 12" and similar resolve the entity inside the picked leaves. If it lives elsewhere, the pointer fires.
- "Suggest breakdown" needs Breakdown (and Screenplay when scene text is needed). If a needed leaf is not picked, the pointer fires.
- Unchanged: the agent loop, step limits, Change Sets, human-only apply (ADR-0013 decision 7, ADR-0019).

## 4. What is removed or changed

| Today | Action |
|---|---|
| 10 `AiScopeKind` values for questions | Replaced by leaves plus instances. Delete when the agent is migrated (R6) |
| `ResolvedScope.context` and the 6,000 character budget | Dead on the question path. Delete |
| Regex domain detection (`turn/domains.rs`) | Demoted to pointer hint |
| Automatic extra domains in `dossier/registry.rs` `plan()` | Removed. Only picked leaves are fetched |
| Retrieval route (`intelligence/router.rs`) | Not used for questions |
| `fetch::passages` wide round across the project | Replaced by leaf-scoped retrieval |
| `project_map` in the prompt | Kept, small, listing the picked leaves |
| "Using:" label | Built from the real selection |

## 5. ADR changes

- New ADR, "Section-scoped assistant". It supersedes ADR-0037 decision 5 (domain by topic words) and decision 6 (Stage B plan becomes the picked leaves; Stage C "one more fetch" goes, because the user widens by adding a leaf).
- Amends ADR-0037 decision 9 for narrative answers (section 3.7).
- ADR-0035 lane leftovers confirmed retired.
- Unchanged: ADR-0028 decisions 1 and 5, ADR-0013 decision 6 and 7, ADR-0019, ADR-0025, ADR-0026.
- Documents to update in the same change: `docs/specs/ai-conversational-harness.md`, `modules/ai/knowledge.md` (how the Section picker works), `docs/engineering/ai-answer-paths.md`.

## 6. Evaluation

Run before any change and again after each slice.

- Baseline of the current build on the existing set (ADR-0037 S0, the 274 case run).
- New set, written at development time (Claude generates, a person spot-checks about one in ten, ADR-0028 decision 9):
  - about 30 questions per leaf
  - the 15 most likely pairs and 5 triples
  - out-of-section negatives (the answer lives in an unpicked leaf)
  - conversational cases per type: scene brief, character description, relationship, story explanation
  - multi-turn follow-ups with the kept selection
  - the example case: Screenplay + Cast & Crew, "How many scenes does X play in, and describe his character?"
- Metrics: required-fact recall in the dossier; answer correctness; checker failures and passes used; pointer precision, recall and false-pointer rate; names and numbers not in the facts; latency median and 95th percentile per answer class.
- Latency bars: the current 15 s median and 30 s 95th percentile for short answers. Set bars for the new long classes from the baseline, written down before the run.
- Trace additions: picked leaves, tokens per leaf, pointer fired and target, retrieval path (hybrid or full-text), degraded flag. Numbers and codes only (ADR-0028 decision 8).
- No model judges an answer at runtime (ADR-0028 decision 5).

## 7. Build order

Each slice is measured before the next starts.

- R0. Baseline and the new test set. No runtime change before this is saved.
- R1. Leaf registry and coverage test. Inventory fetchers per leaf. Likely gaps (no fetcher listed today): Day Out of Days, Daily View, Sides, Reports. Split `visual.rs` into Moodboards, Storyboards, Shot Lists and the Idea Vault.
- R2. Panel payload and interface: picker, instance pickers, Send gating, kept selection, truthful label. Wire the selection into the core.
- R3. Dossier by leaves: name resolution restricted to the pick, split budget, leaf-scoped hybrid retrieval, full-text fallback.
- R4. Out-of-section pointer.
- R5. Conversational answers: new types, briefing dossiers, policy, checker updates.
- R6. Agent: tool filter by leaf, navigate inside the pick, pointer for the rest.
- R7. Delete dead code (section 4), update documents and `graphify`, rerun the full evaluation.

## 8. Defaults taken (owner may change)

- Only leaves are pickable; the cap of 3 counts leaves.
- Idea Vault Global and This Project are leaves.
- The selection is kept within one conversation.
- Added branches: Notes & Tasks, App Help & Settings.
- Instance picker rules in 3.1.
- Starting word limits in 3.7.
- Commands: tool filter plus pointer only.

## 9. Risks

- A wrong pick produces a pointer. A misfiring pointer frustrates people. Measure the false-pointer rate. One tap adds the leaf.
- Production has 13 leaves. A question needing 4 is blocked. Watch the trace. Reconsider the cap only with data.
- A split budget thins each leaf. Tune weights on the evaluation.
- Long answers raise hallucination risk and latency on a 1B model (section 3.7).
- Users may forget to pick. That is the choice taken: no pre-fill. The Send hint is the only guard.
- Index not ready: leaf-scoped retrieval falls back to full-text and says so.
- Pointer must never leak content or existence of data the user cannot read.

## 10. Verify before building

1. `scope.context` is not read on the question path (graphify query, or compare a trace for Beat Board and Current Screenplay on the same question).
2. The hybrid retriever has no caller on the question path.
3. Which leaves lack a read tool today (ADR-0037 open question 4).
4. Real latency of a 250 to 350 word answer on the target PC.

## 11. Out of scope

Model change or fine-tuning, token streaming, automatic section routing, a Whole Project mode, Mac code, anything from Chapter 2 or 3.

## Sources

- STAIR, structure used as the address of information: https://arxiv.org/abs/2609.03874 . We keep the idea. The user is the router, so no training is needed.
- Anthropic context engineering, smallest set of high-signal context: https://agentic-ai.readthedocs.io/en/latest/ContextEngineering/anthropic/
- Metadata pre-filtering and validating router output against a known list: https://pub.towardsai.net/how-to-design-a-production-grade-rag-system-d8135e85ce0b

## 12. Build design (lead's decisions, 2026-11-03)

This section turns the design above into one contract for the slices. Where it narrows the text above, it says so.

### 12.1 One boundary in code

New module `crates/straft-application/src/modules/ai/sections/`. `sections::LEAVES` is the one table, one entry per leaf:
`id` (`production/cast-crew`), `branch`, `label`, `aliases`, `picker` (none, screenplay and draft and scene, scene, shooting day, call sheet), `weight`, `min_capability`, `sources` (the fetch functions it runs), `entity_types` (the `search_doc` entity types homed here), `fact_kinds` (the `Fact.kind` values homed here), `name_kinds` (the `index::Kind` values homed here), `tools` (the agent tools homed here), `example` (a question, for tests only).

Three coverage tests fail the build: every `search_doc` entity type maps to exactly one leaf; every `Fact.kind` a fetcher can produce maps to exactly one leaf; every navigation screen in `apps/desktop/src` (the list is written in the test, and checked against the routes in `model.ts`) maps to exactly one leaf. A fourth: every registered agent tool is homed in at least one leaf or is listed as a loop control tool.

The hard boundary is a single gate at the end of the dossier stage: **a fact whose kind is not homed in a picked leaf is dropped, whatever produced it.** A test feeds every fetcher a project full of data with one leaf picked and proves nothing from another leaf arrives. A fetcher that is too wide is therefore a wasted read, never a leak.

Text retrieval filters by `entity_types` of the picked leaves plus the owner privacy filter. This narrows section 3.5, which says a path prefix on `nav_json`: entity type is the same boundary without parsing JSON per row, and the coverage test holds it. If a test shows one entity type must live under two screens, stop and report.

### 12.2 The payload

`AskArgs` becomes `{ text, sections: Vec<SectionPick>, conversationId, maxSteps }`, with `SectionPick { leaf, screenplayId?, draftId?, sceneId?, shootingDayId?, callSheetId? }`. One to three picks; the core refuses anything else with plain words (`invalid_input`), and an unknown leaf too. The panel's scope select and `scopeArgs` go away. `ResolvedScope` stays internal for the agent until R6 and is built from the picks.

`ai.sections` is a new query returning the tree for the open project: only leaves the person may read, Episodes only on an episodic project, and for pickers the screenplays, drafts, scenes, days and call sheets to choose from.

### 12.3 The label

`AiExchange.scopeLabel` is built by code from the picks that were really read, for example "Using: Screenplay (Draft 2, all scenes) + Cast & Crew". The trace gets the leaf ids, tokens per leaf, whether a pointer fired and its target, hybrid or full text, and whether retrieval was degraded. Numbers and codes only (ADR-0028 decision 8).

### 12.4 The pointer

Code only. `AiResultDto.pointer: { leaf, label } | null`. Sentence: `That is in Production > Budget. Add it to your selection and ask again.` At three leaves: `That is in Production > Budget. Remove a section and add this one, then ask again.` Triggers in order: a resolved name whose home leaf is not picked; the picked leaves returned nothing and a cheap existence check finds the thing in an unpicked leaf; a topic word from `turn/domains.rs` (now a hint only) pointing at an unpicked leaf when the picked leaves returned nothing. Permission is checked before pointing. Nothing found anywhere: the existing "I could not find X. I looked in A and B." with the picked leaves. A question that part fits: answer that part, then the pointer sentence after it.

### 12.5 Answers

`AnswerType` gains `Describe`, `Brief`, `StoryExplanation`; `Relation` stays. `LengthClass` gains `Long250` and `Long350`, with `max_tokens` scaled (about 1.3 tokens per word plus a margin). The dossier cap stays 1,500 and 2,500 tokens until a latency run says otherwise; narrative answers use 2,500. Lists are allowed only for a scene-by-scene briefing. Unsupported-relationship and unstated-mood checks stay off until their false positives are measured on the set.

### 12.6 Evaluation in practice

Gold set: `crates/straft-ai-eval/gold/sections.lowtide.en.jsonl`. Ids carry the sample mark: `sec-s-<leaf with hyphens>-NNN` for the fixed model sample and `sec-x-<leaf with hyphens>-NNN` for the rest; the other families are `pair`, `triple`, `neg`, `conv` and `turn` (`sec-s-pair-NNN`, `sec-x-neg-NNN` and so on). A case carries `sections`, and `expect.pointer` when the answer lives in a leaf that was not picked. The new answer types and the long classes sit in `new_answer_type` and `new_length_class` so the first harness still loads the file. Details: `gold/README.md`. Target about 30 per leaf where the fixture has the data; fewer where it holds less, with the count stated per leaf and never padded with empty questions. The fixtures may gain data for leaves that have none.

The model runs at about 18 seconds a case in wall time. A full model run of 1,000 cases would take five hours, so: `--dossier-only` (no model writing) on the whole set after each slice; the model on a fixed sample of about 300 cases (`--only sec-s-`), once before any runtime change on the frozen R0 binary and once at R7. The R0 run on the old binary ignores `sections`, which is the point: it measures today's behaviour on the new questions.

Latency bars, written before any run: short answers stay at median 15 s and 95th percentile 30 s; the long classes (`Long250`, `Long350`) at median 30 s and 95th percentile 60 s, the ADR-0028 limit on a processor. The answer quality bar for the new set: required facts in the dossier at least 90 percent; pointer precision and recall at least 90 percent; false pointers at most 5 percent; invented names no worse than the R0 baseline.

## 13. Build status

Updated as each round lands. A round is built when its tests are green; it is measured by the evaluation described in section 12.6.

- **R0** evaluation set and baseline: gold set `crates/straft-ai-eval/gold/sections.lowtide.en.jsonl` (706 cases, 29 leaves), frozen baseline binary.
- **R1** leaf registry (built, tested). `modules/ai/sections/` holds the table (`registry.rs`, 29 leaves in the gold order), the tool homes (`tools.rs`), the picks and the gate (`mod.rs`), the tree for the panel (`tree.rs`, the `ai.sections` query) and the coverage tests (`tests.rs`). The fetchers are split per leaf (`dossier/fetch/*`); Day Out of Days, Daily View, Sides, Reports, Overview, Catalog, Timeline, Episodes and the Global Idea Vault have their own reads. Deviations from section 12.1:
  - A kind several leaves write (a count, a passage, an assumption, a missing statement) cannot have one home. Those four are `SHARED_KINDS`: `run_leaf` stamps each fact with the leaf that read it (`Fact::home`), and the gate checks the stamp.
  - The Outline shows the same acts and cards as the Beat Board. Its facts carry their own kinds (`Outline act`, `Outline card`), so each kind still has one home.
  - A name kind can be served by several leaves (a character name is read by Characters, Scenes, Cast & Crew, the Schedule and others). `name_kinds` lists the kinds a leaf serves; the first leaf in the table that lists a kind is its home, the leaf the pointer names.
  - Agent tools: the read tools are listed on their leaf; the 193 proposal tools are listed by what they change in `tools.rs`. Four tools span every leaf (`CROSS_TOOLS`), three read pages that are no section (`UNSECTIONED_TOOLS`, with `propose_restore_deleted`), and `final_answer` ends the loop.
  - Story Page keeps its leaf with a one-line note and no fetcher: the page is the writer's own free writing, kept apart on purpose (owner request 2026-10-20, `registry::hidden::FREE_WRITING`).
- **R2** payload and panel (built, tested). `ai.ask` takes `sections` (one to three picks) in place of `scope`; the core refuses none, four, an unknown leaf, a leaf twice, and a leaf the person may not read. The panel has the `SectionPicker`, chips, instance pickers, the Send hint and a selection kept for the conversation. The "Using:" line is built by code from the picks (`sections/label.rs`). The trace has the new columns (migration `0010_ai_trace_sections.sql`). `ai-eval` reads `sections` from a case, maps a legacy case to the leaves the old router would have read (`legacy.rs`), and resolves titles, ordinals and day numbers to ids (`picks.rs`).
- **R3** dossier by leaves (built, tested). `dossier::build` takes the picks; the plan, the extra domains and the wider round are gone. Names are restricted to the picked leaves (the rest are kept aside for the pointer). Budget split by weight with a minimum share (`dossier/allot.rs`). Passages come from the hybrid retriever, filtered by the entity types of the picked leaves and the owner (`dossier/passages.rs`, `retriever::gather_in`), and fall back to full text with the degraded flag when the index is not ready.
- **R4** out-of-section pointer (built, tested). `sections/pointer.rs` is code only; no model decides where to look. Triggers in the order of 12.4: a name homed in a leaf that was not picked (`Dossier::elsewhere`, which now carries the kind, so a topic word such as "plays" picks Cast & Crew rather than the Story for a character); the picked leaves returned nothing for the words of the question and an existence check over `search_doc` (all of up to two topic words, the person's own private notes only) finds the thing in a leaf that was not picked; a topic word of `turn/domains.rs` (hint only, and only when it names one leaf alone) after the same empty result. "Returned nothing" means no fact the picked leaves gave holds a word of the question. The fixed sentences are the ones in 12.4. A leaf the person may not read is never named (`Decision::NotFound`, the existing "I could not find X. I looked in A and B."). Story Page is never a target (`Leaf::note`). When the message also names something the picked leaves hold (a name taken from the message, not from a picker), the answer is written from the picked facts and the sentence is appended after the checks, so the writer never sees it. `AiResultDto.pointer` (`AiPointerDto { leaf, label }`), the trace column `pointer`, and the panel button `Add <label>` (adds the leaf, focuses the question box; no button at three leaves, where the sentence says to remove one). The harness scores `expect.pointer` (precision, recall, false-pointer rate) in the report of a full run and, because the pointer is code, in `--dossier-only` too.
- **R5** conversational answers (built, tested). `AnswerType::{Describe, Brief, StoryExplanation}` and `LengthClass::{Long250, Long350}` (40 to 250 and 60 to 350 words; about 1.3 tokens a word plus a margin for `max_tokens`). Rules first (`turn/rules.rs`), then 63 new lines in `turn_examples.jsonl` (five English explain examples were relabelled), the short model fallback unchanged. The person's words win: "briefly" shortens, "in detail" lifts 250 to 350, "one line" gives one sentence; "Brief me" is the verb and does not count as "brief". A new narrative policy (`prompt::NARRATIVE`): state only what the facts show, no motive, feeling or past the facts do not state, a relationship only from a relationship record. Plain paragraphs, except a scene-by-scene briefing (an act, a day, a sequence, a range: `turn::scene_by_scene`), which is asked for one line per scene and which the checker lets through (`Rule::List` otherwise). The narrative dossier is the existing narrative fetch (scene summary and scene text with the scene number, character cards, relationship records), now capped at 2,500 tokens for the long classes. The unsupported relationship word and unstated mood checks exist behind `check::STRICT_NARRATIVE_CHECKS = false`; `--dossier-only` runs them over the expected answer of every `sec-` case and reports the false-positive rate (turn them on only under 5 percent). The harness reads `new_answer_type`, `new_length_class`, `min_words` and `max_words`, and the latency report shows each length class against its bar (12.6).
- **R6** agent (built, tested). The tool list offered to the agent is filtered to the tools homed in the picked leaves (`sections::tools::gate_tool`); the scoring by scope label in `toolbox.rs` is gone. Under a scope with no picks (internal calls, tests) every tool stays open. A tool outside the picks is not offered and never runs, even when the model names it: the step becomes the pointer to the leaf the tool lives in. "Suggest breakdown" needs Breakdown and Screenplay (`tools::NEEDS`). "Open Scene 12" and the other navigation commands resolve inside the picked leaves, otherwise the pointer fires (`pointer::decide_open`). The pages that are no section (Activity, Recently Deleted, package history) are not offered. The agent loop, the step limits, Change Sets and the human-only apply did not change.
- **R7** clean-up. Deleted: the ten `AiScopeKind` values and `AiScopeArgs` with their generated TypeScript, `ResolvedScope.kind` and `ResolvedScope.context`, the 6,000 character budget and `scope::resolve` (a scope is now built by `scope::from_picks` or `scope::whole`), the scope step of the retrieval assembler that read `context`. Kept on purpose: `intelligence/router.rs`, because the `retrieve_context` agent tool still routes by it (questions no longer do); `turn/domains.rs`, as the pointer hint; `ContextItem`, a plain source and text pair that retrieval and the breakdown suggestion use. The `scope_kind` column keeps its old values in old rows and nothing reads it back; new rows write `Sections`. No migration was added or edited. Documents updated: `ai-conversational-harness.md` (marked), `knowledge.md`, `ai-answer-paths.md`, `ai-tool-coverage.md`, the feature status row.
- **R4 pointer rules, 2026-11-04 (measured).** `pointer.rs` also checks the Global Idea Vault (`Input.vault`) and scores a search document by how many of up to five question words it holds (`need`). The first rule set made 6.2 percent false pointers, because rules fired when the pick already answered. Fixed by class, with the home of each thing as the rule: a Story Day is the Timeline's (the words "story day"; asked under the Timeline nothing points); a day is a shooting day only when the message says shoot, shooting, shot or schedule, and a call time on one is the Call Sheets'; a scene numbered in the message and asked for props is the Breakdown's even when the pick did not resolve it; what a character says is in Screenplay > Scenes; Timeline, Reports, Catalog, Breakdown and Tasks list scenes, days and their items themselves (`READS_ADDRESSES`), so an address in the message is read there; the existence check is skipped under the Breakdown or the Catalog, never names the Call Sheets, never fires on a question that points back ("those", "the second one"), and for four or more words needs `need` of them; "called" and "named" are ways of asking; the bare words "scenes" and "to do" are no section name; "how do I" with a Settings word names Settings. The topic-word rule (trigger 3, `turn::Domain`) was removed: it never named the right leaf when it was the only trigger. Measured on 597 cases (`ai-eval --dossier-only --only sec-`): precision 92.6 percent (75 of 81), recall 68.8 percent (75 of 109), false pointers 1.0 percent (6 of 597). Before this round: precision 45.8 percent (54 of 118), recall 49.5 percent (54 of 109), false pointers 6.2 percent (37 of 597). Precision and the false-pointer bar are met; recall (90 percent) is not. Evidence: `docs/release/evidence/ai-section-scoped-2026-11-03/r4-pointer/report.md`.
- **First measures of R4 and R5** (`ai-eval --dossier-only --only sec-`, no writing model, 597 section-scoped cases, 2026-11-04). Pointer: precision 56.8 percent (46 of 81 pointers), recall 42.2 percent (46 of 109 expected), false pointers 3.4 percent (20 of 597). The false-pointer bar (at most 5 percent) is met; precision and recall (90 percent) are not. Where the recall goes: questions whose answer is in the Global Idea Vault (it lives in another database than the project search index, so the existence check cannot see it); content questions about notes, files and call sheets whose words are not all in one search document; "Which props does scene 3 need?" is pointed to the Catalog instead of the Breakdown (same words, two homes); scene or day questions asked under a section that does not read scenes. Nothing was tuned beyond the order except four rules found by the first run: a name that the picked leaves' own facts say is held by them (a card that mentions Babu), a scene or day the message names counts as an address the picked leaves read by, verbs of asking ("brief", "describe") are no topic word, and the names a person uses for a section (its label and aliases) come before the topic words. Narrative checks that are off: the unsupported relationship word and the unstated mood flag 0 of 596 expected answers (0.0 percent, under the 5 percent bar). That proxy is weak, because an expected answer is a handful of fact strings with almost no relationship or mood words, so both checks stay off until the model sample (`--only sec-s-`) measures them on written answers.
