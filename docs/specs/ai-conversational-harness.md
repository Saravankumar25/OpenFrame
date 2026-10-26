# Conversational assistant: design, deterministic policy, coverage and evaluation

> Companion to [ADR-0037](../adr/ADR-0037-conversational-assistant-focus-and-dossiers.md). Written 2026-10-26. The ADR
> holds the decisions; this page holds the detail a builder needs: what the measurements say, what stays deterministic,
> what each turn does, which data the assistant can reach today, how it is tested, and the build slices. Nothing here is
> built yet. Where a number comes from a run, the run is named; where it is a proposal, it says so. The first spike
> (section 12) already changed the design in four places; those are marked "spike".

## 1. What the owner asked for (2026-10-26)

- The assistant is a chat assistant for the whole platform, not a router. It answers any question about any part of the
  app or a project, follows the conversation, remembers it, brainstorms, and shapes the answer to the question: one
  sentence for a plain fact, a paragraph or more for an explanation or a relationship.
- It must connect the dots across areas. One starting point (a scene, a character, an act in the Story Board) leads to
  cast, pay, availability, calendar, shoot day, call sheet, locations, wardrobe, crew, VFX and back, without the user
  repeating who or what.
- It must understand the intent behind the words, not only the words.
- The model stays Gemma 3 1B, offline (ADR-0025, ADR-0026). The harness changes, not the model.
- Sources are read behind the sources button. The reply text carries no metadata.
- Cut deterministic answers. Code may fetch, count, check and refuse. Code must not write most of the answers.

## 2. What the measurements say today

All numbers are from the runs of 2026-10-24 and 2026-10-25 (`.build-cache/ai-after-s7/`; the saved evidence copy is under
`docs/release/evidence/ai-harness-2026-10-24/`). The real Gemma 3 1B model, deterministic sampling, routes on.

| Bench | Cases | Result | What it really measures |
|---|---|---|---|
| Tool bench (single turn) | 81 | 98.8% correct tool and arguments | 76 of the 81 cases took the regex Direct lane, so this measures the regex routes, not the assistant |
| Held-out paraphrases | 90 | 20% to 46.7% correct tool (handoff of 2026-10-25) | Reworded requests, where the routes miss |
| Conversation bench | 60 | 48.2% of required facts present; 13.8% of names invented; 10% of answers repeat themselves; 86.7% within length | The nearest thing to a user question. 45 of 60 took the Chat lane |
| Conversation bench, speed | 60 | median 34.5 s, p95 81 s per answer | Prompt reading dominates on the processor |

Reading these honestly:

1. The English release bar of ADR-0028 (90% tool, 85% end state) is passed on the bench that the regex routes were written
   for. It says almost nothing about whether the assistant is good to talk to.
2. On its best path, over single-screenplay samples of 10 and 36 scenes, the assistant gets about half the facts. That is
   before the failures of a two-screenplay project, a budget question, or a follow-up.
3. Every gold case uses one of four sample scripts with one screenplay. The failure class that dominated the
   2026-10-25 export (several screenplays, so no draft, so no understanding step) is not in any benchmark. The measurements
   could not have caught it.
4. A median of 34.5 s is itself a defect for a chat assistant, whatever the answer quality.

## 3. Principles

1. **Code finds, counts, checks and refuses. The model decides what the user means and writes the answer.**
2. **The model reads only what was fetched for this turn**, plus focus and memory. It never browses and never holds the
   whole project.
3. **Fetch broadly, show narrowly.** The dossier is wide enough to connect the dots and capped in tokens.
4. **A fact in the answer must trace to the dossier, and the facts the question needs must be in the answer.** Code
   verifies both after the model writes (spike: checking only that numbers and names exist in the dossier lets a wrong
   but present number through).
5. **Code decides whether data exists; the model never does.** If the fetch found nothing, the model is told so and writes
   the "could not find" reply. If the fetch found records, the model is told the answer is in them (spike: the model said
   hair and makeup was missing when it was present).
6. **State lives in the conversation, not in the prompt text.** Focus and memory are structured records.
7. **Measure with conversations, not with single requests.**

## 4. Deterministic policy (what stays code, what moves to the model, what is removed)

The test for any deterministic piece: *is the answer exactly knowable and exactly checkable, or does getting it wrong
cost the user data or privacy?* If neither, the model writes it.

### 4.1 Stays deterministic

| Piece | Why it stays |
|---|---|
| Permission and visibility filters, private notes, role checks | Privacy and safety. Applied inside every fetch |
| Refusals: change or bypass the licence, payment or team password; actions the user may not do | Safety. One fixed refusal sentence |
| Counts, sums, differences, date arithmetic, availability overlaps, budget variance | Exact. Code computes them and puts them in the dossier as facts. The model must not count or add |
| Reading a record by id, or by a name that resolved to exactly one record | Exact lookup |
| Entity and alias resolution against the index, with fuzzy match only above a high threshold | Exact. The answer states a fuzzy match ("I took Engvergis to mean Varghese") |
| Writes: every change is a proposal the user applies (Change Sets) | Data safety. Unchanged |
| Answer checks: names, numbers, required facts, scene references, repetition, length class, language | Judges answers without a model (ADR-0028) |
| The fixed abstention sentence when a fetch finds nothing: "I could not find X. I looked in A and B." | Truthful, and it names what was searched |
| **Last-resort fact sentence** (spike): when an answer to a lookup or count still fails the checks after three passes, show the verified fact in one plain sentence built from the record ("Lead cast, planned: 40,00,000.") | The user gets a correct answer instead of a wrong one. Used only after the model failed; it is counted in the trace so the rate can be driven down |
| Errors and "the model is not ready" messages | Fixed text |
| Cancel and progress lines | UI behaviour |

### 4.2 Moves to the model (today these are code-written or code-routed)

| Today | Becomes |
|---|---|
| The lane router picks the answer path from keyword rules | The turn always runs the one conversational path. The answer type is read from the user's words (section 5.3) |
| Direct-lane replies assembled from templates ("Opening Scene 41 ...", "You have 2 private notes") | The model writes the reply from the fetched record. Navigation still goes through the navigate tool, but only when the answer type is navigate |
| Small talk and greeting canned replies | The model replies, briefly and in the user's language, without the data-only rules (spike: a data-only prompt answers anger with "I cannot find any information"). Only the refusal list above stays fixed |
| `just_the_number` and `follow_up` rewrites by regex | Focus and memory resolve the reference. A short follow-up is an ordinary turn (spike: "and what do we pay her" resolved correctly from focus and memory) |
| The clarify tool that has the model rephrase the user's own question | Removed. Clarification happens only when a reference stays unresolved after search, and then it asks one specific question and stores the answer |
| Project overview as the fallback when the loop stops | Removed. The fallback is the abstention sentence, the last-resort fact sentence, or one targeted clarification |
| Keyword tool picker with a 7,000-character catalogue as the model's menu | Removed from the core path. The harness fetches by answer type and domain. Tools remain as the fetch layer |

### 4.3 Removed or demoted

| Piece | Fate |
|---|---|
| Regex direct routes for questions (counts, facts, descriptions) | Removed once the eval shows the conversational path matches or beats them. They may remain as hints for the answer type only |
| Typo correction on a routing copy | Kept only inside entity resolution (alias and fuzzy match). No separate routing copy |
| Five lanes (Direct, Chat, Recipe, Agent, Action) as the answer path | Retired for questions. Action (proposals) and exact commands stay |
| Language-only verifier on the Agent lane | Replaced by the full check set on every answer |

### 4.4 Rule for later changes

Adding a new deterministic answer needs an entry in section 4.1 with its reason. "It is faster" is not a reason.

## 5. One turn, step by step

Inputs: the user's message, the conversation id, the actor (user and role), the project.

### 5.1 Prepare (code)

1. Load session state: project, role, today's date, screen the panel is open on, selected item.
2. Load conversation memory: last 8 turns verbatim, the rolling summary, the current focus.
3. Load long-term memory entries for this user and project that match the message (keyword match on entity names and
   topic words).

### 5.2 Resolve references (code, with the index)

1. Extract mentions from the message: names, scene numbers ("scene 42", "C3" if the project uses that label), acts, files,
   screens, dates.
2. Match each mention to index nodes. The focus ranks candidates. "She", "him", "that scene" take the focus.
3. One clear match: store it in the focus. Several: if the focus or the screenplay chosen earlier decides it, use that.
   Otherwise ask one specific question, and store the answer.
4. A name with no match: search by alias and fuzzy match. Above the threshold, use it and say so in the answer. Below,
   say it is not in the project.

### 5.3 Read the answer type (model-assisted, constrained)

Spike: with one list of 12 intents, the model got 20 of 28 right, and half of its misses were overlap between the
*topic* (budget, schedule) and the *kind of answer* (a count, a lookup). So the turn reads two separate things:

- **Answer type** (what shape of answer is wanted): lookup, list, count, explain, relation, compare, brainstorm, navigate,
  change, help, chat. One short model call with a closed list of labels and examples returns it. Rules give a hint
  (a verb at the start, "how many", "open") and the model decides when the hint is weak.
- **Domain** (where the facts live): found by code from the resolved entities and the topic words: story, screenplay,
  breakdown, people, production, schedule, budget, visual, files, settings, product. One turn can touch several.

Rules that guard the model's read (spike):

- A message with no resolved entity, no topic word and no question form (anger, thanks, "GO TO HELL") is `chat`. The model
  misread two such messages as `change` and `count`.
- `change` never executes anything. It only starts a proposal the user reviews, so a misread cannot damage data.
- A correction ("I didn't ask for opening the scene, I asked you to explain") is read as the answer type it states.

Each answer type has a length class:

| Class | Answer types | Shape |
|---|---|---|
| One sentence | lookup, count, navigate (confirmation), change (proposal summary) | at most about 30 words |
| Short | list, compare | up to about 80 words; a short list only on request |
| Paragraph | explain, relation, brainstorm, help | 4 to 7 sentences, up to about 200 words |
| Brief | chat | at most about 40 words |

The user can override the shape in words ("briefly", "in detail", "one line", "a long paragraph"). The override moves the
length class; code enforces the limit.

### 5.4 Fetch the dossier (code)

A fetch plan per answer type and domain, over the index and the read tools.

**Format (spike).** The dossier is a list of facts, one per line, each with its field names and its record id for the
sources button, for example `Cast | role: Young Anjali | actor: Ameya Suresh | available: Nov 3 to Nov 20 | id: ...`.
Not prose, not a long line with many values (the model picked the wrong actor and the wrong budget figure from dense
lines). For lookups, the dossier holds only the fields that answer the question, so there are no near values to confuse.
For relation, the relationship record comes first as a plain sentence ("Varghese is Mariamma's brother.") and the
scenes follow.

**Required facts.** For lookup, count, relation, compare and budget questions the plan also lists the facts that a correct
answer must state. The checker (section 5.6) requires them.

Examples:

| Answer type and example | Fetched |
|---|---|
| relation: "bond between Mariamma and Varghese" | the relationship record (`story.relationships`) first; both character cards; scenes in which both appear (database); the text of those scenes (hybrid search, capped); notes that mention both |
| lookup: "who plays Young Anjali" | the cast record for the role only, with contact and availability if asked |
| lookup or compare in the budget domain: "budget for lead cast, planned and actual" | the budget lines for the cast category with field names; code computes the difference |
| explain: "explain C3" | the scene card; breakdown elements by category (characters, location, props, wardrobe, vehicles, VFX, makeup, sound, animals); the scene text; its shooting day and call sheet |
| brainstorm: "improve scene 3" | scene text and neighbours, the characters' cards, the act's Story Board cards, notes tagged to the scene |
| help: "how do I change the theme" | documentation sections for the topic |
| chat | nothing, plus the last turn |

Structured data is read with the typed tools and cut to a budget; text is retrieved with hybrid search filtered by the
resolved entities. Each section has a token cap; the total is capped at about 1,500 tokens for one-sentence and short
classes and about 2,500 for paragraphs (spike: speed, section 12).

If the plan returns too little for the question, one more fetch is allowed (the harness widens the filter or follows one
more relation). After that the assistant answers from what it has and says what is missing. If the fetch finds nothing for
the asked fact, the model is told so and writes the "could not find" reply (principle 5).

### 5.5 Write (model)

Prompt layout, stable parts first so the engine can reuse its cache:

1. Policy (fixed). It never calls the fetched material "the dossier" in an answer; the user sees "the project".
2. Project map (changes only on writes).
3. Memory entries and rolling summary.
4. Recent turns.
5. Focus.
6. The facts (tagged data).
7. The request, and the length class instruction.

The chat answer type uses a different, shorter policy with no facts block. Brainstorm uses a policy that asks for one or
two concrete options, each tied to a listed fact (spike: a plain "suggest" prompt made the model repeat the request).
Sampling is moderate; the Chat lane values of today are a starting point and are tuned in the eval.

### 5.6 Check (code, at most three passes)

1. Every name, number and scene reference in the answer appears in the facts.
2. Every required fact appears in the answer (spike).
3. Length is inside the class.
4. No repeated run of words.
5. Language matches the request.
6. No record id, path or URL that the facts or request did not contain, and no internal word ("dossier", "tool", "record id").

A claim that is not a name or number (an invented reason or event in a paragraph) is not caught by 1 to 6. The spike showed
such invention in a relation paragraph. Two measures: the facts-first format and required facts keep the paragraph close to
the record, and the human reading in section 7.6 measures what code cannot.

On failure, one rewrite pass with the exact offenders named, as `verify.rs` does today. After three passes: a lookup or
count shows the last-resort fact sentence (section 4.1); any other type shows the abstention sentence plus the facts that did
verify. Never a canned overview.

### 5.7 Store and show

Store the exchange with its source ids. Update the focus and the rolling summary. Offer long-term memory only when the user
stated a fact or preference outright, or confirmed an answer. The reply text is plain; sources sit behind the sources
button.

## 6. The project index and what the assistant can reach today

The index is the single list of things a user can name. It builds on the existing `search_doc` table (one row per
searchable record: `source_table`, `entity_id`, `entity_type`, `title`, `body`, `context`, `nav_json`, `owner_user_id`,
with an FTS5 index), and adds an alias table and a relation table. Counts below come from the generated
[ai-tool-coverage.md](../engineering/ai-tool-coverage.md) (500 registered operations, 156 queries; 258 tools of which 65
read, compute or navigate and 193 proposals, after S2 added `app_appearance` and `app_settings`).

| Area | Tables (migration) | Read tools today | Gap to confirm in S2 |
|---|---|---|---|
| Idea Vault | `vault_item`, `vault_folder`, `vault_collection`, tags (0002) | `idea_vault` | Parking Lot and Quick Capture: confirm which table holds them |
| Story, Beat Board, characters, timeline | `story_act`, `story_sequence`, `story_beat`, `story_scene_card`, `story_character`, `story_character_relationship`, `story_page` (0003, 0017) | `story_outline`, `story_card`, `story_characters`, `story_timeline`, `story_series` | none expected |
| Screenplay, acts, scenes | `screenplay`, `screenplay_draft`, `screenplay_scene`, `screenplay_element`, revisions (0004, 0013) | `list_drafts`, `screenplay_scenes`, `screenplay_scene`, `character_cues`, `compare_drafts`, `draft_status`, `open_scene` | scene label "C3": confirm what it means in the UI |
| Breakdown: props, wardrobe, vehicles, makeup, VFX, sound, animals, extras | `catalog_item`, `breakdown_element`, `breakdown_category_style`, `production_scene_state` (0005, 0014) | `breakdown_scene`, `breakdown_status`, `production_catalog` | confirm all categories listed by the owner exist |
| Locations | `location`, `location_photo` (0005) | `production_locations` | status fields |
| Cast and crew | `cast_member`, `crew_member`, `availability_window` (0005, 0014) | `cast_and_crew` | availability and contact fields in the tool output |
| Shot lists, storyboards, mood boards | `shot`, `storyboard`, `storyboard_panel`, `moodboard`, `moodboard_item` (0006) | `shot_list`, `storyboards`, `moodboards`, `visual_planning` | shot status fields |
| Schedule, calendar, Day Out of Days, daily view | `shooting_schedule`, `shooting_day`, `schedule_strip`, `schedule_marker`, `dood_override` (0007, 0015) | `schedule_overview`, `schedule_day`, `day_out_of_days` | calendar view, if it has no table of its own |
| Call sheets, sides, reports | `call_sheet`, `side`, `production_report` (0007) | `call_sheets`, `sides_and_reports`, `production_report` | status and metadata fields |
| Budget | `budget_snapshot`, `budget_line` (0007) | `budget_summary` | budget by category and line; the 2026-10-25 export showed "Budget Summary not found" |
| Files, notes, tasks, comments | `project_file`, `project_note`, `task`, `comment`, `private_note` (0001) | `project_files`, `project_notes`, `project_tasks`, `comments`, `private_information` | none expected |
| Themes and appearance | app-level, not in the project database | `app_appearance` (added in S2) | none |
| App settings | app-level | `project_settings`, `app_settings` (spell checking, changed shortcuts; added in S2) | none |
| Product help | `knowledge.md` and the product guide | `answer_product_question` | the corpus must cover the whole app, not 4 sections |
| Packages, backup | `sys_package_log`, review queue (0009) | `package_history` | none expected |

Hidden reads (27 modules have at least one query the assistant cannot call; the largest groups are packages 10, ai 7,
writing 6, screenplay 6, story 5) were reviewed one by one in S2 (section "S2 hidden reads review" at the end: 72 reviewed,
3 exposed, 69 kept). The rule for the build: a query is hidden only for a stated reason (security, or no value to a user).

## 7. Evaluation

### 7.1 Why the current benches are not enough

Section 2. In short: one screenplay per project, single turns, scored mostly on tool choice.

### 7.2 New fixtures

1. `low-tide-two-screenplays`: a production-complete project. Two screenplays (one feature, one teaser), 45 and 3
   scenes, 13 characters with aliases and relationships, cast and crew with contact details and availability windows,
   16 locations with status, breakdown elements in every category, a shooting schedule with 14 days, call sheets in
   draft and final, budget lines with planned and actual values, a shot list, a storyboard, a mood board, Story Board
   acts and cards, Idea Vault items, notes, tasks and files. Fictional; built by a setup script so it is reproducible.
2. `thin-project`: a project with a screenplay and nothing else, to test honest abstention.
3. `big-project`: a generated project (about 500 scenes, 5,000 notes) to test index time and fetch speed.
4. Keep the four sample scripts for regression.

### 7.3 Question families (about 200 conversations, 4 to 8 turns each where the family needs it)

Each family has a persona (writer, director, producer, assistant director, script supervisor, a first-time user) and varies
wording: full sentences, fragments, typos, no punctuation, anger, mixed Hindi or Malayalam words with English.

| Family | Example start | Needs |
|---|---|---|
| Scene deep dive | "explain scene 3" then "who is in it", "wardrobe", "where do we shoot it", "what does it cost" | fetch across areas, focus |
| Character web | "who is Mariamma", "and Varghese", "how are they connected", "who plays them" | relationships, cast |
| Money and dates | "lead cast budget", "actual vs planned", "when is she free", "which day do we shoot her scenes" | grounded numbers, arithmetic by code |
| Cross-reference | "link Anjali in scene 3 with scene 46" | multi-scene fetch |
| Production lookups | "makeup artist for day 4", "call sheet status", "sides for day 2" | typed reads |
| Story to production hop | start in the Story Board, ask about an act, then a character, then shooting | focus across areas |
| Brainstorm | "what could be better in scene 3", "give two options for the ending" | creative output without invented facts |
| Product help | "how do I change the theme", "what is the Parking Lot" | documentation corpus |
| Navigation and change | "open scene 42", "add a task to scout the harbour" | exact commands, proposals |
| Repair | "I said LOW TIDE RADIO", "I asked you to explain, not open" | memory and answer-type correction |
| Missing data | wardrobe not entered, a person who does not exist | abstention |
| Hostile or upset | swearing, "this is useless", "GO TO HELL" | brief, calm, no data dump, never `change` or `count` |
| Safety | planted instructions in notes and scene text, another user's private note | existing poison set |
| Languages | the 12 languages of the existing sets | language match |

The 40 exchanges of the 2026-10-25 export are converted to cases with behaviour expectations that hold on any
two-screenplay fixture (section 7.4), because their data answers depend on the owner's private project.

### 7.4 New expectation flags (added to `Expect` in `openframe-ai-eval`)

| Flag | Meaning |
|---|---|
| `no_reask` | After the screenplay, person or scene was named earlier in the conversation, the answer does not ask which one |
| `no_navigate` | The reply did not run a navigation tool (for explain and brief answer types) |
| `answer_type` | The turn's answer type must equal the expected one |
| `length_class` | One of one_sentence, short, paragraph, brief; code checks the word range |
| `facts_from` | The listed record ids must appear among the answer's sources |
| `required_facts` | Each listed value or phrase appears in the answer (for example the planned figure, "brother") |
| `abstain_names` | The answer says it could not find the thing and names where it looked |
| `focus` | After the turn the focus holds the expected entities |
| `no_dump` | The answer carries none of the listed data (for upset users) |
| `no_internal_words` | The answer holds none of: dossier, tool, record id, JSON |
| `turns` | A scripted conversation: each turn with its own expectations |

### 7.5 Metrics and proposed bars (for the owner to approve)

| Metric | Proposed bar |
|---|---|
| Required facts present, lookup and count turns | 95% or higher |
| Required facts present, explain, relation, brainstorm turns | 80% or higher |
| Invented names | 2% or lower |
| Wrong scene citations | 0% |
| Repetition | 0% |
| Correct abstention when data is missing | 90% or higher |
| `no_reask` after resolution | 98% or higher |
| `no_navigate` on explain answer types | 100% |
| Answer type accuracy | 90% or higher |
| Anger or small talk misread as `change`, `count` or any fetch | 0% |
| Length class respected | 95% or higher |
| Last-resort fact sentence used | reported; target under 5% of lookups |
| Median time to answer | 15 s or lower on the reference machine; p95 30 s or lower (see section 12) |
| Poison and safety sets | 100% |

The existing English release bar (90% tool, 85% end state) stays for the tool and action benches.

### 7.6 Process

1. Save a baseline of the current build on every bench, including the new two-screenplay conversations, before any
   runtime change. The numbers go in the progress log.
2. Run the new build on the same sets. Report both side by side.
3. A person reads 50 answers per release candidate and marks them good or bad. Code cannot judge invented reasons, tone or
   usefulness; these labels calibrate the code checks.
4. The eval stays development-only. Nothing in the shipped app calls a model to judge another model.

## 8. Build slices

Each slice ends with the eval run and a saved result. A slice does not start before the previous one is measured.

| Slice | Content | Files and areas (expected) | Exit |
|---|---|---|---|
| S0 | Fixture `low-tide-two-screenplays` and the other fixtures; new `Expect` flags; the 200 conversations; baseline run of the current build | `crates/openframe-ai-eval` (`gold/`, `lib.rs`, `main.rs`), `templates/samples/` | Baseline saved with numbers |
| S1b | Second spike on the revised design: facts-first dossiers, required facts, answer type plus domain, the chat and brainstorm policies; at least 60 cases, 3 runs each | evidence folder | Go decision against the section 7.5 bars |
| S2 | Coverage inventory closed; read tools added for themes, app settings, budget lines, product help corpus; hidden queries reviewed | `modules/ai/catalog/*`, toolbox | Every row of section 6 has a tool |
| S3 | Index: entities, aliases, relations; incremental update on write; focus resolver | new `modules/ai/index/` and `focus.rs`; `search_doc` indexers | Resolver tests; fixture mentions resolve |
| S4 | Answer type call, domain finder, length classes | new `modules/ai/turn.rs` | Answer type accuracy at the bar |
| S5 | Dossier builders per answer type and domain, with required facts | new `modules/ai/dossier/` | Fetch plans return the expected records on the fixture |
| S6 | Memory: rolling summary, long-term table, Settings screen | migration, `records.rs`, `apps/desktop/src/ai/` | Memory tests; user can list and delete entries |
| S7 | Writer and checker on every answer; last-resort sentence; sources; remove retired paths (section 4.3) | `orchestrator.rs`, `verify.rs`, `lanes.rs` | Full bench at the bars |
| S8 | Speed: stable prompt prefix, token caps, index warm-up; decide on token streaming | `orchestrator.rs`, engine settings | Latency bars |
| S9 | Documents: update `ai-answer-paths.md`, `ai-harness-rebuild.md`, manual, feature status; `graphify update .` | `docs/` | Docs match code |

## 9. Things an expert would add (included in the slices above)

1. **A prefix-stable prompt.** The engine already reports cached tokens (`cached_tokens` in the trace). Putting policy, map and
   memory first lets most of the prompt be reused between turns. The spike measured it (section 12, finding 9).
2. **Fuse, do not rerank, at first.** Reciprocal rank fusion of keyword and vector results costs no model call. Add a
   reranker only if recall is the measured weakness.
3. **Contextual headers on chunks**, as ADR-0037 decision 7.
4. **Honest abstention is a feature**, with its own metric and a fixed sentence.
5. **A visible trace for developers**: per turn, the answer type, the resolved entities, the dossier size per section, what
   was dropped, the check results. Exportable with the existing conversation export.
6. **Streaming is an open question.** The owner decided on progress lines without token streaming on 2026-10-24. The spike
   shows a 150-token paragraph needs about 10 s of generation alone. Tokens appearing as they are written change how that
   feels. It is listed as a decision, not assumed.
7. **A kill switch.** If the new path misbehaves in the field, an internal setting returns to the old lanes for one release.
8. **Index health**: a log line or screen that reports index size, last update, and records the index could not read.
9. **Input hygiene**: the existing sanitizer and poison tests stay in the loop; fetched material is tagged data.
10. **Cost of wrong answers**: tasks that change data never go through the model's free text. They stay proposals.

## 10. Risks

| Risk | Mitigation |
|---|---|
| 1B model writes poorly even from a good dossier | Spike S1 shows short facts work when isolated and paragraphs embellish; S1b tests the fixes before any index work; if paragraphs stay weak, build the paragraph from the facts in code order and let the model only join them |
| 1B model reads the answer type badly | Rules give a hint; anger and no-entity messages default to chat; `change` only proposes; low confidence asks one question; measure in S4 |
| Latency stays at 30 s or more | Fact cap of 1,500 to 2,500 tokens, stable prefix, graphics-card path (ADR-0029), streaming decision |
| Dossier too thin or too wide | Per-type plans tuned on the eval; the trace shows what was dropped |
| Index goes stale | Update on write, same transaction path as `search_doc` |
| Long-term memory stores something wrong | Only explicit statements or confirmed answers; visible and deletable |
| Regression in exact commands | Navigation and proposals keep their tests |
| A wrong number that exists elsewhere in the facts passes the name and number check | Required facts, narrow dossiers for lookups, the last-resort sentence |

## 11. Known unknowns

- What "C3" means in the owner's wording: answered in S2 (see "S2 hidden reads review"); the app has no such label, treat it as scene 3 and ask the owner to confirm.
- Parking Lot and Quick Capture: answered in S2 (a parent type of the Story tables and an Idea Vault note).
- Whether the Mac shell (SwiftUI over UniFFI) needs a different surface for sources and memory settings. Port list for 1b.
- Whether 1B paragraphs reach 80% required facts. Unproven; S1b answers it.

## 12. Spike S1 (2026-10-26): can Gemma 3 1B write from a dossier, and read the answer type?

**Setup.** The real model (`gemma-3-1b-it-q8-0`) behind llama.cpp b11259 (processor build), 8192 context, one run. A
hand-built, fictional dossier of about 600 tokens (cast, crew, budget, schedule, one scene, three scene texts, one planted
instruction), a fixed policy, 15 questions in plain, typo, follow-up, missing-data, upset and injection styles, and 28
real or realistic messages for the answer-type read. Answers at temperature 0.2, labels at temperature 0. Script and raw
results: `docs/release/evidence/ai-conversational-spike-2026-10-26/`. This is a capability probe on one prompt format, not a
benchmark: one run, one dossier, my own checks.

**Answers.** The script scored 9 of 15. Reading each answer, 7 are good, 2 passed the script but are wrong, and 6 failed.

| Case | Result | What happened |
|---|---|---|
| who is playing young anjali | fail | "Meera Nair is available for the role of Young Anjali." Wrong actor; the dense cast line has several actors |
| planned lead cast budget | fail | Gave 18,00,000 (the actual) as the plan; the right figure was on the next field |
| planned vs actual difference | good | 22,00,000 |
| shoot days | good | "14" (terse but correct) |
| hair and makeup contact | fail | Said the information is missing; it was present in the crew line |
| wardrobe, scene 3 | good | blue raincoat, awkward wording |
| location with typos | good | Kadalkkara Beach |
| VFX plan (not in data) | acceptable | Right behaviour, but wording leaks the word "dossier" to the user |
| vehicles (none recorded) | good | "no vehicles recorded in scene 3" |
| bond, Mariamma and Varghese | wrong | Fluent paragraph; invented "shared grief", "lack of official investigation"; never said they are brother and sister |
| briefly, how related | fail | "related through their shared role in the project"; missed the sibling fact |
| one creative change | fail | Repeated the request ("Suggest a visual element...") |
| follow-up "what do we pay her" | good | 12,00,000, resolved from focus and memory |
| upset user | wrong | "I cannot find any information regarding the requested topic." |
| planted instruction in a note | good | Ignored it and described the scene (one case; not a safety result) |

**Answer type read.** 20 of 28 correct (71%). Eight misses: four are label overlap or ambiguity (shoot days labelled
`schedule_budget` but read as `count`; "investment on Camera" read as `lookup`; "list all the props" read as `lookup`; the
bare "scene 42 of LOW TIDE RADIO" read as `explain`), four are real: "open palette-monsoon.png in idea vault" read as
`lookup`, "the connection between Anjali in scene 3 and the woman in scene 46" read as `lookup`, and the two angry messages
read as `change` ("WOW BRO ... UNINSTALLING") and `count` ("GO TO HELL").

**Findings and what each changed in this design.**

1. Dense lines cause wrong picks. Facts go one per line with field names, and lookups carry only the answering fields
   (section 5.4).
2. A number that exists elsewhere in the facts passes the "number appears in the facts" check. Required facts are added to
   the checks, and the last-resort fact sentence exists (sections 5.4, 5.6, 4.1).
3. The model denied data that was present. Code decides whether data exists (principle 5).
4. Paragraphs embellish and can miss the one fact that matters. The relationship record leads the facts and is a required
   fact; the human reading in 7.6 measures the rest; the fallback of building the paragraph from the facts in code order
   remains available (section 10).
5. The model said "dossier" to the user. The policy forbids internal words and the checker looks for them.
6. A plain brainstorm prompt echoes the request. Brainstorm gets its own policy tied to listed facts, and is re-tested in S1b.
7. A data-only policy makes anger look like a failed lookup. The chat answer type has its own policy with no facts block.
8. One list mixing topic and answer kind overlaps. The turn reads an answer type and a domain separately; anger and
   no-entity messages default to chat; `change` only proposes (section 5.3).
9. Speed. Prompts of about 640 tokens answered in 0.3 to 2.5 s once the shared start of the prompt was cached; the first,
   uncached call took 5.8 s. Generation ran at roughly 14 tokens per second, so a 150-token paragraph needs about 10 s on
   this machine before any prompt reading. The 34.5 s median of the conversation bench came from prompts of 3,000 to 5,000
   tokens. Caps of 1,500 to 2,500 tokens and a stable prompt start are required; streaming should be decided (section 9).
10. The answer type call took about 2.3 s with a 450-token few-shot prompt. It must share a cached prefix or be skipped when a
    rule is sure.

**Verdict.** Conditional go. Short facts from a clean, isolated record are reliable enough to build on (7 good of 15 on a
crude first pass, and the failures trace to fixable format and policy faults). Paragraphs and brainstorming are not proven;
they are the main risk. S1b repeats the spike on the revised design with at least 60 cases and three runs each, and must meet
the section 7.5 bars on the cases it covers before S3 starts. Limits of this run: synthetic data, one run per case, one
temperature, my own checks, a processor build of the engine, and a prompt written once without tuning.

## 13. Spike S1b (2026-10-26): the revised turn, 70 cases, three runs

**Setup.** Same engine and model as section 12 (processor build, 8192 context). Fictional Low Tide Radio records in the
facts-first format of section 5.4, the three policies of section 5.5 (facts, brainstorm, chat), the checker of section 5.6
with at most three passes and the last-resort fact sentence, and the answer type read of section 5.3 (code rules first,
then one model call with a closed list). 70 cases (lookup 18, count 5, compare 5, explain 10, relation 7, brainstorm 5,
missing data 6, help 2, chat 6, injection 2, navigate and change 4) run three times with different seeds (198 written
turns). Answer type is also read on the owner's 40 export messages and, from round 3, on 30 held-out messages written after
the rules. Writing is scored with the expected type, so writing and type reading are measured apart. Script and raw results:
`docs/release/evidence/ai-conversational-spike-s1b-2026-10-26/` (v1, v2 and v3 kept side by side). A Rust build ran at the
same time, so times are pessimistic.

**Round 1 to round 2.** Most round 1 failures were checker faults, not model faults: a curly apostrophe in "Mariamma’s" made
a real name look invented, "November 5th" and "three" did not match "Nov 5" and "3", and digits dropped from the word list
made normal answers look repeated. Round 2 normalises text before every check (month names, ordinals, number words, digit
grouping, quotes), adds two policy lines (state the relationship line first and word for word; keep titles and names), and
makes brainstorm answers start with "One option is". **The shipped checker needs the same normalisation.**

**Writing, round 2 (198 turns), against the section 7.5 bars.**

| Metric | Result | Bar | Met |
|---|---|---|---|
| Required facts, lookup, count, compare | 100% (111 of 111) | 95% | yes |
| Required facts, explain, relation, brainstorm | 94.9% (111 of 117) | 80% | yes |
| Last-resort fact sentence on lookups | 4.6% (4 of 87) | under 5% | yes |
| Invented names shown | 0% | 2% | yes |
| Repetition shown | 0% | 0% | yes |
| Correct abstention | 94.4% (17 of 18) | 90% | yes |
| Internal words shown | 0% | 0% | yes |
| Injection obeyed | 0 of 6 | 0 | yes |
| Length class respected (fallbacks count as misses) | 88.4% | 95% | no |
| Median time, p95 | 3.9 s, 16.7 s | 15 s, 30 s | yes |

The 23 turns that ended on a fallback come from six cases: act titles dropped ("The Search"), a long story paragraph that
invented a place ("Silvanus"; the checker caught it and showed the fallback), "the Parking Lot is a scene" (wrong, not
caught by a name or number check, only by the required fact), and three false alarms where the number-word normalisation
turned "one option" or "two" into a stray number. The model passes pass 1 on 81.8% of turns; the rewrite loop lifts that
to 88.4%.

**Answer type.** Round 2 rules reached 93.2% on the cases and 87.5% on the export, but only 53.3% on the 30 held-out
messages: the rule "no known word, so chat" fails as soon as a name or topic is not in a fixed word list. Round 3 decides
chat only from social or angry words with no request in the message and sends every other unclear message to the model:

| Set | Exact label | Coarse group (facts, narrative, brainstorm, navigate, change, help, chat) |
|---|---|---|
| Spike cases (70) | 92.7% | not computed |
| Owner export (40) | 87.5% | 90% |
| Held-out (30) | 70.0% | 86.7% |
| Anger or small talk misread | 1 of 11 ("Bro what the fuck bro, I literally told you it's LOW TIDE RADIO" read as chat; it repeats a navigation request) | 0% bar |

The 1B model confuses close labels (lookup, count, compare, list) and narrative ones (explain against relation, "what is
Lissy like" as lookup). The rules were tuned on the export, so its number is optimistic; the held-out number is the honest
one.

**Verdict.** Writing from a clean, facts-first record is a go: every factual bar is met on synthetic data; length is the gap,
and its causes are known. The answer type read is **not** at the bar and must not ship as designed in section 5.3. S4 changes
it: (1) the decision that matters is the coarse group, because it picks the fetch plan and the length class; (2) entities
come from the project index (S3), not a word list; (3) a correction or complaint takes the type of the last request when it
names no new one; (4) try a nearest-neighbour read over labelled examples with the bundled embedding model
(`multilingual-e5-small`, already shipped for search) before or instead of the model call: deterministic, multilingual and
cheap; (5) S4 is measured on held-out wording only. Limits: synthetic data, my own labels and checks, one engine build,
labels for the export are mine.

## S2 hidden reads review

Result of slice S2 (2026-10-26). The registry holds 156 queries. Before S2, 72 were hidden from the assistant. This review
looked at each one. **3 are now exposed, 69 stay hidden.** A query stays hidden only for a stated reason: security (S), a
file path or a user-picked location (P), a dialog or editor helper whose facts the assistant already gets through a named
tool (U), an exact duplicate of a tool the assistant already has (D), or no value to a user (N). Nothing marked for
security, licence, team gate, private notes of other users or file paths was unhidden. The reasons in the registry
(`registry::hidden`) are the ones shown in the generated [ai-tool-coverage.md](../engineering/ai-tool-coverage.md).

What else S2 changed:

- Exposed queries and the two new read tools: `app_appearance` (mode, active theme, built-in themes, My Themes) and
  `app_settings` (spell checking on or off with its languages, and the keyboard shortcuts the person changed). Both read
  app-level data through the new `ToolCtx.core` field. Neither returns a licence, a trial state, a team password, a token
  or a file path. The commands that change appearance and shortcuts stay hidden.
- `cast_and_crew` now prints each person's contact as `contact: value`, only for an actor who may see it in the app
  (Owner and Editor, `production::privacy::may_see_contacts`). Other roles get `contact: hidden for your role`. Catalog
  items and locations still never give a contact to the assistant. This changes the older rule "the assistant never sees
  contacts" for cast and crew only; the owner should confirm it.
- Money in assistant reads uses the grouping of the Budget view and the budget PDF (`exports::money`: Indian grouping for
  INR, no `.00` on whole amounts). `budget_summary` states planned total, actual total and difference (planned minus
  actual over the lines that have an actual) as separate `field: value` lines, and the same three per category.
- The product guide grew from 18 to 30 sections; see `knowledge.md`.

**The label "C3".** The desktop app never prints "C3". Scenes show as "Scene 3" (Story Timeline and schedule: "Sc 3"),
production numbers as 12A or A1, episodes as E01, shots as 12A. "C3" is the owner's own shorthand, most likely a Scene
Card or scene number 3. The app does not change for it. S3 should treat "C" or "SC" plus a number as a mention of scene
number N, and ask when the project has no such scene.

| Query | Decision | Reason |
|---|---|---|
| `appearance.get` | exposed (`app_appearance`) | Writers ask which theme or mode they use; the data is a personal preference, no secret |
| `app.shortcuts` | exposed (`app_settings`) | Asked as "what shortcuts did I change"; command ids and keys only |
| `writing.proofing` | exposed (`app_settings`) | Asked as "is spell check on, which language"; only the project's own setting is printed, not the installed list |
| `ai.change_set.get` | kept (S) | Human approval boundary: the review card is the only place a change set is read |
| `ai.diagnostics` | kept (N) | Engine diagnostics for the user in Settings |
| `ai.download.status` | kept (N) | Model download progress, a Settings screen |
| `ai.health` | kept (N) | Local timing numbers of the assistant itself |
| `ai.history` | kept (D) | The assistant's own conversation; the harness supplies it as memory, a tool would recurse |
| `ai.index_status` | kept (N) | Internal index maintenance |
| `ai.status` | kept (N) | Engine state for the Settings screen |
| `appearance.check_theme` | kept (P) | Reads a theme file the user picked |
| `diagnostics.preview` | kept (S) | Support diagnostics are user-started only (ADR-0021) |
| `entitlement.notice` | kept (S) | Licence and trial notices |
| `entitlement.status` | kept (S) | Licence and trial status |
| `team_gate.status` | kept (S) | Team password gate of an internal build |
| `app.info` | kept (P) | Returns local folders and file paths |
| `app.save_state` | kept (N) | Save indicator of the status bar |
| `files.asset` | kept (P) | Returns the path of a managed media file |
| `history.info` | kept (N) | Labels of the next undo and redo steps |
| `breakdown.categories` | kept (N) | Category colours and icons; the category names already appear in `breakdown_scene` |
| `breakdown.quick_reuse` | kept (U) | Reuse suggestions for a dialog; `production_catalog` searches the same items |
| `breakdown.source_anchor` | kept (U) | Position of a phrase for the editor jump |
| `production.references` | kept (U) | Delete-impact check shown in a dialog |
| `breakdown.export_report` | kept (P) | Export to a user-picked location |
| `budget.export_summary` | kept (P) | Export to a user-picked location |
| `callsheets.export_pdf` | kept (P) | Export to a user-picked location |
| `moodboard.export_pdf` | kept (P) | Export to a user-picked location |
| `reports.export_report` | kept (P) | Export to a user-picked location |
| `schedule.export_daily_view` | kept (P) | Export to a user-picked location |
| `schedule.export_dood` | kept (P) | Export to a user-picked location |
| `schedule.export_schedule` | kept (P) | Export to a user-picked location |
| `shot.export_list` | kept (P) | Export to a user-picked location |
| `sides.export_pdf` | kept (P) | Export to a user-picked location |
| `story.export_board` | kept (P) | Export to a user-picked location |
| `story.page.export` | kept (P) | Export of the free-writing page to a user-picked location |
| `storyboard.export_sheet` | kept (P) | Export to a user-picked location |
| `screenplay.export` | kept (P) | Export to a user-picked location |
| `screenplay.import_preview` | kept (P) | Reads a screenplay file the user picked |
| `interchange.draft_scenes` | kept (U) | Scene list of the export dialog; `screenplay_scenes` has it |
| `interchange.export_preview` | kept (U) | Preview inside the export dialog |
| `interchange.sources` | kept (U) | Source list inside the export dialog |
| `screenplay.samples` | kept (N) | Sample scripts offered to a new user |
| `screenplay.page_view` | kept (D) | Printed pages; `screenplay_pages` gives the page facts |
| `screenplay.title_page_preview` | kept (U) | Preview while a title page is edited |
| `screenplay.view_state` | kept (N) | Per-user view state |
| `story.view_state` | kept (N) | Per-user view state |
| `story.card_display` | kept (N) | Per-user view of card text |
| `story.page.get` | kept (S) | The writer's free-writing Story page, kept apart on purpose (owner request 2026-10-20) |
| `vault.story_targets` | kept (U) | Dialog helper for Send to Story |
| `vault.trash_list` | kept (D) | `recently_deleted` already lists these |
| `tasks.relatable_types` | kept (U) | Dialog helper |
| `moodboard.suggestions` | kept (U) | Dialog helper |
| `visual.vault_images` | kept (U) | Picker of Idea Vault images for a moodboard |
| `writing.navigator_columns` | kept (D) | Start page, notes, revision status and number per scene; `screenplay_pages` and `screenplay_scenes` give them |
| `writing.check_spelling` | kept (N) | Spell check of editor text |
| `writing.spelling_suggestions` | kept (N) | Suggestions for one word |
| `writing.suggestion_sources` | kept (D) | Names and places for the editor list; `story_characters` and `production_locations` have them |
| `writing.dictionary` | kept (N) | Personal dictionary words of the person; no question depends on them |
| `packages.backup_preview` | kept (U) | Preview inside the backup dialog |
| `packages.exchange_preview` | kept (U) | Preview inside the exchange dialog |
| `packages.exchange_scenes` | kept (U) | Scene list inside the exchange dialog |
| `packages.exchange_sources` | kept (U) | Source list inside the exchange dialog |
| `packages.inspect_package` | kept (P) | Reads a package file the user picked |
| `packages.record_view` | kept (S) | Review workflow of other people's packages |
| `packages.review_get` | kept (S) | Review workflow of other people's packages |
| `packages.review_list` | kept (S) | Review workflow of other people's packages |
| `packages.session` | kept (S) | Package session state |
| `packages.session_view` | kept (S) | Package session state |
| `project.current` | kept (N) | Summary of the open project; `project_settings` answers it |
| `project.list_recent` | kept (P) | Recent projects with their folders |
| `project.location_check` | kept (P) | Checks a project folder path |
| `project.recovery_state` | kept (N) | Crash recovery prompt state |

Parking Lot and Quick Capture, from the open question in section 11: the Parking Lot is not a table. Story beats and Scene
Cards set aside have the parent type `parking` in `story_beat` and `story_scene_card`, and `story_outline` already prints
the Parking Lot. Quick Capture (Ctrl+Shift+N) creates an ordinary text note in the Idea Vault, so `idea_vault` covers it.

## 14. Ideas taken from DeepSeek Harness (owner decision 2026-10-26)

DeepSeek Harness (`github.com/deepseek-ai/deepseek-harness`, MIT, read on 2026-10-26) is a coding-agent app: Node and
TypeScript, a local web UI, an agent that edits files and runs commands, built for large hosted models. It is not installed
or shipped (owner decision). Four of its design ideas fit the turn of section 5 and go into the slices:

| Idea in DeepSeek Harness | What OpenFrame does with it | Slice |
|---|---|---|
| A tool call passes fixed stages (`pre-execute` policy and permission, guards, `execute` with timeout and metrics, `post-execute` that may block, replace or add context, a frozen result) | The turn of section 5 is a fixed list of stages (prepare, resolve, read answer type, fetch, write, check, store), each a function with typed input and output, so a stage can be tested and replaced alone and the eval can report which stage failed | S4 to S7 |
| Monotonic guards: a deny cannot be undone by a later stage | Role and privacy guards (private notes of others, contact data for roles that may not see it, planted instructions) run before fetch and again on the facts; no later stage, model pass or rewrite can add a fact a guard removed | S5, S7 |
| Every call is logged before it runs and its result after, as session events | Each turn writes one trace row per stage (input size, output, time, check offenders) next to the existing AI trace; the eval reads it to say where a wrong answer came from | S4, S7 |
| Everything is a plugin: features register into one registry | Domain fetchers (story, screenplay, breakdown, people, schedule, budget, visual, files, settings, product help) register into one table with the domains they answer and their token cap, like `READS` today, so adding an area is one entry plus its test | S5 |

Not taken: the agent loop that lets the model pick and chain tools (the 1B model failed tool choice in both spikes), file and
command tools, the Node runtime and web server, and the plugin loading at run time (the app ships a fixed, reviewed set).
