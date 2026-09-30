# OpenFrame Studio — AI Specification

**Document type:** Cross-cutting AI product, functional, data-access, command, safety, permission, and integration specification

**Product:** OpenFrame Studio

**Primary platform:** Windows + macOS desktop application

**Operating model:** Local-first, offline-capable, user-owned project files; local AI where available; optional external AI; no mandatory OpenFrame cloud

**Purpose:** Define the complete behavior and boundaries of the OpenFrame Studio AI assistant so it can operate as a system-wide project assistant and command interface without becoming an autonomous or authoritative filmmaker.

**Status:** AI architecture / functional behavior baseline to be propagated into PRD, FSD, UX/UI, and Domain/Data specifications.

---

# 0. Authority and Alignment Contract

## 0.1 Existing product direction

The existing OpenFrame product documents already define AI as an optional project assistant rather than an autonomous filmmaker.

The PRD explicitly establishes an **Amazon Q-style project assistant** model. It states that the assistant should help the user interact with project information, answer project-content questions, locate references, summarize material, suggest breakdown elements, provide scheduling assistance, and prepare other suggested actions.

The existing documents also establish the critical safety rule:

> **AI may suggest. The user confirms anything that becomes meaningful project data.**

The FSD further establishes that project-changing AI actions require a preview and explicit acceptance, and that an accepted AI mutation becomes a normal undoable application action.

This specification does not replace those principles. It expands them into a complete system-wide AI operating model.

## 0.2 Document hierarchy

The specification hierarchy remains:

```text
PRD
 ↓
FSD
 ↓
 ┌──────────────┬────────────────┬─────────────────┐
 │ UX/UI        │ Domain/Data    │ AI Specification│
 └──────────────┴────────────────┴─────────────────┘
                       ↓
                 Implementation
```

The PRD owns product intent and scope. The FSD owns functional behavior. UX/UI owns presentation and interaction. Domain/Data owns logical representation and source-of-truth relationships. The AI Specification is a cross-cutting contract for AI-specific behavior across all four layers.

For AI-specific behavior, this document is authoritative within the AI boundary and must be reflected consistently in the PRD, FSD, UX/UI, and Domain/Data specifications.

The AI specification defines AI-specific functional detail that may be missing from the other specifications, but it must not contradict their existing source-of-truth, permission, offline, versioning, recovery, collaboration, or user-control rules. Any new AI capability that affects user-visible product behavior must be propagated into the PRD, FSD, UX/UI, and Domain/Data specifications during the same alignment pass.

When this specification introduces a capability that is not currently represented elsewhere, that capability must later be propagated into the appropriate parent specification rather than remaining an isolated AI-only behavior.

## 0.3 Logical rather than implementation-specific

This specification defines:

- AI responsibilities;
- AI boundaries;
- data access;
- context handling;
- command interpretation;
- read-only queries;
- deterministic computations;
- mutation proposals;
- permission and confirmation behavior;
- change-set semantics;
- cross-module orchestration;
- local/external processing rules;
- failure behavior;
- audit behavior;
- UX states;
- domain support requirements;
- acceptance criteria.

It does not mandate:

- a programming language;
- a specific database;
- a specific vector database;
- a specific agent framework;
- a specific API framework;
- a specific desktop framework;
- a specific model-serving runtime.

A reference small local model may be selected for implementation, but the product contract must remain model-agnostic.

---

# 1. AI Product Definition

## 1.1 What the OpenFrame AI is

The OpenFrame AI is a **system-wide natural-language assistant and command interface for OpenFrame Studio**.

It is not a separate chatbot product.

It is not a screenplay judge.

It is not an autonomous filmmaker.

It is not an independent database.

It is not the source of truth for project content.

The assistant sits above the existing OpenFrame information model and application actions.

Conceptually:

```text
                    USER
                      │
                      ▼
              Natural-language request
                      │
                      ▼
             OpenFrame AI Assistant
                      │
       ┌──────────────┼───────────────┐
       │              │               │
       ▼              ▼               ▼
  Product         Project         Application
  Knowledge       Knowledge       Tools / Actions
       │              │               │
       └──────────────┼───────────────┘
                      ▼
              Deterministic results
             or proposed Change Set
                      │
             ┌────────┴────────┐
             │                 │
             ▼                 ▼
        Read / Answer      Preview / Confirm
                                  │
                                  ▼
                          Normal app mutation
                                  │
                                  ▼
                           Undo / Activity
```

The language model is primarily the natural-language interpretation, routing, explanation, and response layer.

The application's own data and deterministic services remain authoritative.

## 1.2 Core promise

The assistant should make OpenFrame feel as though the filmmaker has a knowledgeable assistant who understands:

- what OpenFrame is;
- how OpenFrame works;
- what is inside the current project;
- how project objects are related;
- what has happened across drafts and versions;
- what production information exists;
- what the user is currently viewing;
- what the user asked;
- and which application operation can accomplish the requested task.

The user should be able to interact with the software in ordinary language rather than having to manually navigate through every screen for routine questions and repetitive operations.

## 1.3 System-wide rather than screen-limited

The assistant must not be limited to the current visible page.

The user may ask about:

- the current scene;
- the current screenplay;
- a particular draft;
- the entire Story Board;
- selected Idea Vault material;
- characters;
- locations;
- cast and crew;
- production catalog;
- breakdown;
- shot list;
- storyboard;
- schedule;
- shooting days;
- call sheets;
- project files;
- tasks;
- notes;
- activity;
- approvals;
- project status;
- relationships between these objects;
- or the entire project.

The assistant may use a broader context when the user explicitly asks for a project-wide answer.

The assistant must always indicate the scope used when scope could materially affect the answer.

---

# 2. Non-Negotiable AI Rules

These rules are normative.

## 2.1 User control

The user remains the decision-maker.

AI must never silently:

- rewrite screenplay content;
- change a Scene Card;
- alter Story Board hierarchy;
- alter canon;
- rename project objects;
- edit production data;
- alter the shooting schedule;
- modify a call sheet;
- delete project content;
- archive or restore objects;
- lock a script;
- approve an item;
- reject an item;
- publish/release an artifact;
- change permissions;
- expose private information to another user;
- or otherwise mutate project state.

## 2.2 User command is not blanket mutation permission

A natural-language command is sufficient authorization to perform a requested **read-only** operation or non-persistent navigation action.

A natural-language command is **not** sufficient to silently apply a project mutation.

For project-changing actions:

```text
User request
    ↓
Interpret request
    ↓
Prepare exact proposed action
    ↓
Show preview
    ↓
User explicitly accepts
    ↓
Apply normal application action
    ↓
Record undoable change
```

The assistant must not skip the acceptance step merely because the user phrased the original request as an imperative.

Example:

> “Replace Arjun with Arjun Reddy everywhere.”

Correct:

> “I found 184 affected structured references across the current project. 31 screenplay Character elements, 42 Scene Cards, 27 breakdown references, 18 cast references, and 66 other supported references. I will not change arbitrary dialogue/action text automatically. Review changes → Apply.”

Incorrect:

> Changes are applied immediately.

## 2.3 No autonomous project behavior

The AI must not independently decide to:

- run an analysis because it thinks it is useful;
- change project data because it detects an inconsistency;
- execute pending suggestions later;
- continuously rewrite content;
- automatically clean the project;
- automatically remove duplicates;
- automatically update production information after a script change;
- automatically reorder scenes;
- automatically apply breakdown suggestions;
- automatically resolve conflicts;
- automatically approve documents;
- or perform background agent tasks that mutate user-owned content.

The AI may respond to explicit user requests.

The application itself may still perform ordinary non-AI infrastructure operations such as indexing, saving, recovery preparation, and deterministic derived calculations. Such infrastructure operations are not AI decisions.

## 2.4 Source-of-truth integrity

The AI must use the application's canonical objects and relationships.

The model's generated text is never itself authoritative.

The AI must not invent project facts when canonical data is available.

For example, if the project contains 17 characters, the answer must come from the project's character data and/or deterministic screenplay analysis rather than model memory.

## 2.5 Deterministic facts must be computed, not guessed

Exact questions such as:

- How many characters are in the script?
- How many scenes are in Draft 7?
- How many locations are used?
- How many pages are in the screenplay?
- How many shooting days exist?
- How many scenes feature Character A?
- How many scenes take place at Location B?
- Which scenes are unscheduled?
- How many breakdown elements are confirmed?
- How many suggested breakdown elements are pending?

must use deterministic project queries/calculations whenever the required source data exists.

The language model may explain the result, but it must not be the calculator.

## 2.6 Permission inheritance

The assistant can never grant itself more access than the current user has.

It must respect:

- project role;
- object permissions;
- private-note ownership;
- collaboration permissions;
- locked-state restrictions;
- export restrictions;
- project access restrictions.

The AI operates as the current user within the same authorization boundary as the user.

## 2.7 No hidden context

The AI must not use unrelated project material without appropriate scope.

When answering a scoped question, the UI should expose the relevant scope.

Example:

> **Using:** Draft 7 + Scene 24 + Character records

For project-wide queries:

> **Using:** Entire Project

For mixed queries:

> **Using:** Current Screenplay + Production Catalog + Shooting Schedule

The scope should be understandable without requiring users to know internal object IDs.

---

# 3. The Three Kinds of Knowledge

The assistant must distinguish three different knowledge domains.

## 3.1 Product knowledge

Product knowledge describes OpenFrame Studio itself.

It includes:

- what each workspace does;
- available actions;
- terminology;
- workflow relationships;
- source-of-truth rules;
- permission rules;
- versioning rules;
- export behavior;
- import behavior;
- offline behavior;
- collaboration behavior;
- supported object types;
- supported operations;
- known UI actions;
- AI behavior and limitations.

The assistant should be able to answer questions such as:

> “What is the difference between Story Board and Screenplay?”

> “Where do I create a shooting day?”

> “What is the Production Source?”

> “Why did the call sheet become stale?”

> “Can AI change a locked shooting script?”

> “What happens when I delete a Scene Card?”

Product knowledge should not depend entirely on the model's parameter memory.

A versioned product knowledge source must exist conceptually so the assistant can retrieve current application rules.

## 3.2 Project knowledge

Project knowledge is information contained in the user's OpenFrame project.

Examples:

- project title;
- project type;
- characters;
- story structure;
- screenplay drafts;
- screenplay scenes;
- screenplay elements;
- notes;
- Idea Vault items;
- production catalog;
- locations;
- cast;
- crew;
- breakdown;
- shot lists;
- storyboards;
- schedule;
- shooting days;
- call sheets;
- tasks;
- comments;
- approvals;
- activity;
- files;
- snapshots;
- version relationships.

This is the primary grounding source for project questions.

## 3.3 Session knowledge

Session knowledge includes temporary interaction context such as:

- current project;
- current workspace;
- current scene;
- selected objects;
- selected text;
- selected draft;
- open panel;
- current user;
- current permissions;
- current AI conversation;
- pending preview;
- pending Change Set.

Session knowledge is contextual, not authoritative.

---

# 4. AI Operating Architecture

## 4.1 Logical components

A conforming implementation should conceptually separate the following responsibilities:

```text
1. Request Intake
2. Intent Interpretation
3. Permission Evaluation
4. Context Resolution
5. Tool Selection
6. Deterministic Query / Calculation
7. Product Knowledge Retrieval
8. Project Data Retrieval
9. Proposed Mutation Construction
10. Mutation Preview
11. User Approval
12. Change Application
13. Undo / Recovery Integration
14. Response Generation
15. Audit / History
```

These may be implemented together internally, but the behavioral separation must remain.

## 4.2 Language model responsibility

The language model may:

- interpret natural-language intent;
- identify objects and references;
- choose appropriate tools;
- convert natural language into structured action parameters;
- summarize returned project data;
- explain results;
- formulate user-facing confirmations;
- explain proposed changes;
- identify ambiguity;
- ask for missing information;
- generate creative text when requested.

The model must not:

- directly modify canonical storage;
- bypass application permissions;
- directly decide authoritative state;
- directly commit a Change Set;
- bypass validation;
- bypass user acceptance;
- fabricate exact project counts.

## 4.3 Deterministic application tools

Application tools are authoritative for operations such as:

- object lookup;
- text search;
- structured search;
- counting;
- aggregation;
- relationship traversal;
- draft comparison;
- scene enumeration;
- character occurrence calculation;
- location usage calculation;
- breakdown state calculation;
- schedule conflict calculation;
- stale-state detection;
- change preview construction;
- mutation validation;
- change application;
- undo;
- export;
- navigation.

The exact implementation is out of scope.

The important rule is:

> **The model decides what operation is needed; the application performs and validates the operation.**

---

# 5. Read, Compute, Navigate, Suggest, Mutate

All assistant operations belong to one of five behavioral classes.

## 5.1 Class A — Read

Examples:

- answer question;
- summarize scene;
- list characters;
- show where a character appears;
- identify locations;
- explain a draft;
- show project status;
- find references.

No project mutation occurs.

No confirmation is required.

## 5.2 Class B — Compute

Examples:

- count characters;
- count scenes;
- count locations;
- calculate page count;
- count unscheduled scenes;
- calculate shooting days;
- determine scene usage;
- compare versions;
- calculate affected objects.

No project mutation occurs.

No confirmation is required.

## 5.3 Class C — Navigate / display

Examples:

- open Scene 42;
- go to Character Room;
- show every scene involving Ravi;
- select Draft 8;
- open the Location entry for Railway Station;
- filter the Production Catalog.

This is application control, not project mutation.

The action may be executed directly because the user explicitly requested it.

## 5.4 Class D — Suggest / prepare

Examples:

- suggest breakdown elements;
- propose a Scene Card;
- draft a synopsis;
- propose schedule grouping;
- prepare a call sheet draft;
- prepare a rename operation;
- produce a revision summary;
- prepare a batch update.

The result is not project-authoritative.

No mutation occurs until acceptance where the suggested result would create or change project data.

## 5.5 Class E — Mutate

Examples:

- create a Scene Card;
- create breakdown elements;
- rename an object;
- modify scene data;
- replace structured references;
- create a task;
- update approved production data;
- alter schedule data;
- archive an object;
- delete an object.

Every project mutation requires:

1. exact interpretation;
2. permission check;
3. preview;
4. explicit user acceptance;
5. validation;
6. application;
7. undoability where supported;
8. change/activity recording.

---

# 6. Universal Command Interface

## 6.1 Purpose

The assistant must function as a universal natural-language command layer across OpenFrame.

The user should not have to know the exact location of the command.

Examples:

> “How many characters are in this screenplay?”

> “Show every scene with Meera.”

> “Open Scene 27.”

> “Find all locations used in Act 2.”

> “Create a breakdown suggestion for Scene 15.”

> “Make a scene card from this note.”

> “Rename the character Ravi to Raghav.”

> “Replace the character name in all structured references.”

> “Show scenes that are scheduled but based on an older draft.”

> “Give me the list of unscheduled scenes.”

> “Create a call sheet draft for Shooting Day 4.”

> “Compare Draft 6 and Draft 7 and tell me what changed.”

## 6.2 Natural-language tolerance

The assistant should understand ordinary language, including:

- abbreviated requests;
- conversational phrasing;
- incomplete but inferable references;
- common filmmaking terminology;
- project-specific names;
- names with spelling variation where ambiguity can be resolved;
- references such as “that scene,” “the current draft,” “this character,” “tomorrow’s shoot,” or “the station.”

The assistant must not guess where two or more interpretations would materially change the result.

## 6.3 Ambiguity handling

When ambiguity is low-impact and can be resolved from current context, the assistant may use the current context.

When ambiguity materially affects a mutation or exact answer, the assistant must ask a focused clarification or show the competing interpretations.

Example:

> “There are two characters named Raju: `Raju` and `Raju Kumar`. Which one should I use?”

For an exact count:

> “Do you mean unique Character records or characters detected in the current screenplay draft?”

The assistant must prefer explicit, deterministic interpretation over silent assumptions.

---

# 7. Context and Scope Resolution

## 7.1 Supported scopes

At minimum:

```text
Current Selection
Current Scene
Current Screenplay
Specific Draft
Story Board
Idea Vault Selection
Production Workspace
Specific Production Object
Specific Shooting Day
Call Sheet
Whole Project
```

The system may support more fine-grained scopes internally.

## 7.2 Scope inheritance

When a request is made from a selected object, the selection may become the initial context.

Example:

User selects Scene 24 and asks:

> “What props are here?”

Default scope:

```text
Scene 24
+
linked Breakdown data
+
relevant Production Catalog references
```

The assistant should not silently scan unrelated drafts or external projects unless the user asks for a broader scope.

## 7.3 Explicit global scope

The user can intentionally broaden a request:

> “Across the entire project, find every place where Arjun is mentioned.”

This activates project-wide scope.

## 7.4 Scope disclosure

For meaningful project queries, show a compact scope indicator.

Example:

> **Using:** Whole Project

or

> **Using:** Draft 8 + Characters + Breakdown

This is especially important for exact counts and mutation previews.

---

# 8. Deterministic Project Query Contract

This section is critical to making a small local model viable.

## 8.1 Principle

Questions about structured project facts should not be solved by asking the language model to reconstruct the project from text alone.

Instead:

```text
Natural-language question
        ↓
Intent + parameters
        ↓
Deterministic query tool
        ↓
Canonical project data
        ↓
Structured result
        ↓
LLM explanation
```

## 8.2 Supported exact-query families

The assistant must be able to query at least:

### Project facts

- project title;
- project type;
- project status;
- language;
- genre;
- creator;
- creation/update dates where exposed;
- current draft;
- production source;
- current shooting day;
- open tasks;
- pending approvals.

### Screenplay facts

- draft count;
- scene count;
- page count;
- element count;
- dialogue count;
- character cue count;
- distinct character references;
- locations;
- interior/exterior usage;
- day/night usage;
- scene durations where represented;
- scenes by act/sequence;
- scenes by character;
- scenes by location;
- scenes by draft;
- scenes changed between drafts.

### Story facts

- number of Acts;
- number of Sequences;
- number of Beats;
- number of Scene Cards;
- parked cards;
- character count;
- relationship count;
- timeline entries;
- outline hierarchy.

### Production facts

- Production Source;
- breakdown status;
- breakdown element count;
- suggested vs confirmed elements;
- catalog item count;
- location count;
- cast count;
- crew count;
- shot count;
- storyboard panel count;
- shooting day count;
- unscheduled scene count;
- scheduled scene count;
- stale production data;
- pending production updates.

### Call-sheet facts

- generated call sheets;
- draft/approved/sent state;
- shooting day relationship;
- stale state;
- affected changes.

### Support facts

- project files;
- tasks;
- notes;
- comments;
- approvals;
- activity;
- snapshots;
- exchange packages.

## 8.3 Exact metric definitions

The assistant must state the source/definition when an answer could have multiple interpretations.

Example:

> “You have **14 Character records** in the project.”

This is different from:

> “The current screenplay contains **12 distinct character cues**.”

Both can be correct.

Likewise:

> “The Production Catalog contains 9 Location records.”

is different from:

> “The current draft uses 7 distinct location headings.”

The assistant must not collapse these metrics.

## 8.4 Example exact questions

### Question

> “How many characters are in the script?”

Preferred response behavior:

1. Resolve the draft.
2. Identify what “characters” means.
3. Query structured character references.
4. Return exact number.
5. State the draft used.

Example:

> “Draft 8 contains 13 distinct character names in screenplay Character elements. The project Character Room contains 15 Character records.”

### Question

> “How many locations?”

Preferred behavior:

Return both metrics when useful:

> “Draft 8 contains 8 distinct screenplay locations. The Production Catalog currently contains 10 Location records.”

### Question

> “How many scenes are in the film?”

Use screenplay scene count from the selected/active draft.

### Question

> “How many scenes are left to schedule?”

Compute:

```text
eligible screenplay scenes
-
scenes represented in shooting schedule
```

subject to the product's canonical scheduling eligibility rules.

### Question

> “Which scenes need breakdown?”

Use actual breakdown state, not LLM inference.

---

# 9. Cross-Module Knowledge Graph Behavior

The assistant must understand relationships among OpenFrame objects.

Conceptually:

```text
Idea
 ↓ explicit promotion/copy
Story Board
 ↓ build / explicit conversion
Screenplay Draft
 ↓ selected production source
Production Source
 ↓
Breakdown
 ↓
Catalog
 ↓
Shot List / Storyboard
 ↓
Shooting Schedule
 ↓
Call Sheet
```

The assistant must also understand lateral relationships:

```text
Character
 ├── Story Board references
 ├── Screenplay references
 ├── Breakdown references
 ├── Cast references
 ├── Scene relationships
 └── Continuity context

Location
 ├── Screenplay scenes
 ├── Breakdown
 ├── Catalog
 ├── Schedule
 └── Call Sheets
```

The assistant uses these relationships to answer cross-module questions.

Example:

> “Where is the police station used?”

The response may return:

- screenplay scenes;
- Story Board references;
- breakdown entries;
- production catalog location;
- scheduled shooting days;
- call sheets that include those scenes.

The assistant must preserve the distinction between authoritative and derived information.

---

# 10. Application Tool Categories

The implementation should expose logical tools rather than allowing the language model arbitrary storage access.

## 10.1 Navigation tools

Examples:

```text
open_project
open_workspace
open_object
open_scene
open_draft
select_objects
apply_filter
show_results
```

## 10.2 Search tools

Examples:

```text
search_project_text
search_screenplay
search_story
search_production
search_files
find_object
find_references
find_cross_module_references
```

## 10.3 Query tools

Examples:

```text
get_project_summary
get_script_statistics
get_character_statistics
get_location_statistics
get_scene_statistics
get_breakdown_statistics
get_schedule_statistics
get_call_sheet_statistics
get_object_relationships
get_version_differences
get_stale_dependencies
```

## 10.4 Analysis tools

Examples:

```text
analyze_continuity
analyze_character_knowledge
analyze_revision_impact
analyze_schedule_conflicts
analyze_breakdown_candidates
analyze_missing_links
```

Analysis results are advisory unless generated by deterministic rules that the product already treats as authoritative.

## 10.5 Preparation tools

Examples:

```text
prepare_scene_card
prepare_breakdown
prepare_synopsis
prepare_schedule_grouping
prepare_call_sheet
prepare_task_list
prepare_rename
prepare_replace
prepare_batch_update
```

## 10.6 Mutation tools

Examples:

```text
create_object
update_object
move_object
duplicate_object
rename_object
update_structured_reference
create_breakdown_elements
update_schedule
update_call_sheet
archive_object
restore_object
delete_object
```

Mutation tools are only invoked after the required approval state exists.

## 10.7 Export tools

Examples:

```text
export_screenplay
export_sides
export_call_sheet
export_breakdown
export_project_package
export_snapshot
```

Export behavior must respect existing export permissions and project boundaries.

---

# 11. Product-Command Coverage

The assistant should provide a natural-language route to the majority of existing product operations.

## 11.1 Project operations

Examples:

> “Open the Black Rain project.”

> “What is this project status?”

> “Show the current shooting draft.”

> “Archive this project.”

For a state-changing action such as archive, preview/confirmation rules apply.

## 11.2 Idea Vault operations

Examples:

> “Find all ideas containing railway station.”

> “Show the three most recent ideas.”

> “Move this idea into the Story Board as a Scene Card.”

> “Create a Beat from this note.”

Creating or moving content requires confirmation when it changes project data.

## 11.3 Story Board operations

Examples:

> “How many Scene Cards are in Act 2?”

> “Show parked cards.”

> “Create a Sequence called The Investigation.”

> “Move this Scene Card after Scene Card X.”

The assistant must preserve Scene Card identity and Story Board semantics.

## 11.4 Screenplay operations

Examples:

> “Open Scene 31.”

> “How many pages are in Draft 7?”

> “Which scenes changed between Draft 6 and Draft 7?”

> “Find every instance of the character cue RAVI.”

> “Prepare a rewrite summary of Scene 22.”

A screenplay mutation requires approval.

## 11.5 Production operations

Examples:

> “Which scenes still need breakdown?”

> “Show all props in Scene 18.”

> “Which locations have no approved status?”

> “Suggest breakdown elements for Scene 27.”

Suggestion does not equal confirmation.

## 11.6 Schedule operations

Examples:

> “Which scenes are unscheduled?”

> “Show the shooting days containing the police station.”

> “Explain why the schedule has a conflict.”

> “Suggest a grouping for the remaining night scenes.”

AI may prepare schedule changes but must not silently reorder the schedule.

## 11.7 Call-sheet operations

Examples:

> “Show tomorrow’s call sheet.”

> “What changed since the call sheet was generated?”

> “Prepare a call sheet draft from Shooting Day 4.”

A generated call-sheet draft remains a draft until the user accepts it.

## 11.8 File and search operations

Examples:

> “Find the location reference PDF.”

> “Show every project file mentioning Mumbai.”

> “Open the production design reference.”

---

# 12. Safe Global Rename and Replace

This is a major required use case.

## 12.1 Principle

“Replace across the software” must be interpreted as a **structured project-wide change operation**, not as blind text replacement.

## 12.2 Supported rename layers

A rename may affect:

1. canonical object names;
2. structured object references;
3. screenplay Character elements;
4. cast references;
5. breakdown references;
6. Story Board references;
7. shot/storyboard references;
8. tasks/notes when explicitly selected;
9. other structured references supported by the operation.

## 12.3 Arbitrary dialogue/action text

Existing FSD behavior must remain respected:

> Renaming a Character record must not blindly modify arbitrary screenplay Dialogue or Action text.

Therefore the AI must separate:

```text
Structured character identity/reference changes
```

from

```text
Arbitrary textual replacement
```

The user may request both, but the UI must present them as distinct change categories.

Example:

> “Rename Ravi to Raghav everywhere.”

Preview:

```text
Canonical Character
Ravi → Raghav
1 object

Structured screenplay Character elements
31 occurrences

Cast references
4 occurrences

Breakdown references
16 occurrences

Scene Card references
8 occurrences

Dialogue/Action raw text
12 occurrences
NOT INCLUDED BY DEFAULT
```

User may explicitly choose whether arbitrary text occurrences should also be changed.

## 12.4 Batch preview

A global rename preview must show:

- requested old value;
- requested new value;
- affected objects;
- affected modules;
- occurrence counts;
- excluded content;
- historical/snapshot impact;
- conflicts;
- locked content;
- permission limitations.

## 12.5 Historical records

Historical snapshots, exported documents, prior issued call sheets, and immutable historical artifacts must not be silently rewritten merely because a current canonical object is renamed.

The preview must explain whether the requested rename affects only active canonical data or also explicitly selected historical materials.

## 12.6 Application

After acceptance:

```text
Approved Change Set
        ↓
Validation
        ↓
Apply
        ↓
Undoable application action
        ↓
Activity entry
```

---

# 13. Breakdown Intelligence

## 13.1 Suggested breakdown

The assistant can analyze a selected scene or production source and propose:

- props;
- vehicles;
- wardrobe;
- makeup;
- special effects;
- stunts;
- weapons;
- animals;
- extras;
- background elements;
- locations;
- set requirements;
- sound requirements;
- production design requirements;
- camera-related requirements;
- transport requirements;
- other supported breakdown categories.

## 13.2 Suggestion state

Suggested items must remain visually distinct from confirmed production items.

Example:

```text
Suggested:
- Red folder
- Sedan
- Blood effect

Confirmed:
- Police radio
- Desk
```

## 13.3 User acceptance

The user can:

- accept all;
- accept selected items;
- edit items;
- reject items;
- cancel.

## 13.4 No silent breakdown mutation

The assistant must never add breakdown elements merely because it detected them.

---

# 14. Schedule Assistance

## 14.1 Allowed behavior

The assistant may:

- identify schedule opportunities;
- explain conflicts;
- calculate grouping options;
- identify location clustering opportunities;
- identify cast availability implications where the data exists;
- identify page/day concentrations;
- identify stale schedule dependencies;
- prepare proposed schedule changes.

## 14.2 Forbidden default behavior

The assistant must not silently:

- reorder scenes;
- change shooting dates;
- change shooting days;
- move strips;
- remove scenes from the schedule;
- change schedule assumptions.

## 14.3 Proposed schedule change

Example:

> “Move the four night scenes at Police Station to one day.”

Assistant response:

```text
Proposed schedule change

Affected scenes:
12, 14, 19, 27

Current:
Day 2: Scene 12
Day 5: Scene 14
Day 7: Scene 19
Day 9: Scene 27

Proposed:
Day 2: 12, 14, 19, 27

Potential impact:
- Day 2 page load increases
- Cast overlap changes
- Existing call sheet for Day 2 becomes stale
- Day 5/7/9 become lower-load days

Apply proposed schedule change?
[Cancel] [Review Changes] [Apply]
```

The AI does not apply the change before acceptance.

---

# 15. Call Sheet Assistance

The assistant may:

- summarize a shooting day;
- list cast needed;
- list locations;
- list scenes;
- identify special requirements;
- generate a draft call sheet;
- identify stale state;
- compare the current schedule to an existing call sheet.

The assistant must not treat the AI-generated call sheet as authoritative merely because it looks complete.

The underlying Shooting Schedule and Call Sheet objects remain subject to the existing source-of-truth rules.

---

# 16. Script and Revision Intelligence

## 16.1 Draft comparison

The assistant should be able to answer:

> “What changed between Draft 6 and Draft 7?”

The underlying comparison must come from the application's version comparison data.

The AI may explain:

- scenes added;
- scenes removed;
- scene text changed;
- character changes;
- location changes;
- production changes;
- schedule implications;
- visual planning implications.

## 16.2 Revision impact

When asked:

> “What did the latest rewrite affect?”

the assistant may traverse:

```text
Screenplay changes
 ↓
Production Source
 ↓
Breakdown
 ↓
Catalog
 ↓
Shots / Storyboard
 ↓
Schedule
 ↓
Call Sheet
```

The assistant reports detected impacts.

It does not apply downstream changes automatically.

## 16.3 Locked script behavior

A locked screenplay remains subject to the existing lock/revision rules.

AI cannot bypass the lock.

When the user requests a change to locked content, the assistant must use the same revision mechanism required of normal application editing.

---

# 17. Continuity Assistance

The assistant may check for:

- character knowledge appearing too early;
- location state inconsistencies;
- injury continuity;
- prop continuity;
- time-of-day sequence concerns;
- character disappearance across long spans;
- repeated events;
- dialogue/canon conflicts;
- possible contradictory information.

Results must be presented as:

```text
Potential continuity issue
```

not:

```text
Confirmed error
```

unless deterministic project rules establish the issue.

The assistant must not automatically correct continuity problems.

---

# 18. Conversation Behavior

## 18.1 Conversational memory

A conversation may retain:

- prior requests;
- resolved references;
- current scope;
- pending proposal;
- explanation preferences.

However, conversation memory must not override current canonical project data.

## 18.2 Context persistence

Example:

User:

> “Open Scene 24.”

Then:

> “How many props?”

The assistant may interpret “props” as referring to Scene 24 because the immediately preceding command established the context.

If the context could have changed or ambiguity is material, show the context.

## 18.3 Correction

If the user says:

> “No, I meant Draft 7.”

The assistant must update the current request context and answer using Draft 7.

The correction should not alter the project.

---

# 19. Permission and Approval Model

## 19.1 Permission layers

The assistant must operate through four conceptual layers:

```text
Layer 1 — User authorization
Layer 2 — Project permission
Layer 3 — Object/state restriction
Layer 4 — Mutation confirmation
```

All must pass before a project change can be applied.

## 19.2 User authorization

A user can ask the assistant to:

- read;
- calculate;
- navigate;
- suggest;
- prepare;
- modify.

The request establishes intent, but modification still requires explicit acceptance.

## 19.3 Project permissions

The assistant must respect the same effective role as the user.

Examples:

- Viewer cannot mutate.
- Commenter cannot mutate screenplay.
- Contributor can mutate only allowed content.
- Owner can perform owner-only operations.
- AI cannot elevate a Viewer into an Editor.

## 19.4 Object/state restrictions

The assistant must respect:

- locked drafts;
- approved artifacts;
- stale documents;
- archived objects;
- deleted/recoverable objects;
- private notes;
- unavailable external files;
- collaboration conflicts.

## 19.5 Confirmation

For every project-changing operation:

```text
Preview
+
Clear consequence
+
User action
```

Minimum controls:

- Apply
- Cancel

For meaningful/batch operations:

- Review Changes
- Select Changes
- Apply Selected
- Cancel

## 19.6 Destructive operations

Delete, permanent deletion, destructive replacement, and similarly high-impact actions must require an explicit destructive confirmation consistent with normal OpenFrame UI behavior.

The assistant must not interpret “clean this up” as authorization to delete.

Example:

> “I found 14 unused Scene Cards. Do you want me to prepare them for deletion?”

Not:

> “I deleted the unused cards.”

---

# 20. Change Set Contract

The existing Domain model already includes a Change Set.

The AI must use that concept for proposed project mutations.

## 20.1 Change Set contents

A Change Set should logically identify:

- change_set_id;
- origin = AI;
- requesting user;
- project;
- target objects;
- base version;
- proposed operations;
- affected modules;
- validation state;
- conflict state;
- review state;
- approval;
- application state.

## 20.2 Operation types

At minimum:

```text
Create
Update
Rename
Move
Duplicate
Delete
Archive
Restore
Link
Unlink
Replace Structured Reference
Update Text
Update Schedule
Update Production Data
```

## 20.3 Base-version protection

The Change Set must be associated with the project/version state on which it was generated.

If the underlying project changes before application:

```text
Pending Change Set
       ↓
Base version no longer current
       ↓
Revalidate
       ↓
Apply / Rebuild preview / Conflict review
```

The assistant must not blindly apply a stale Change Set.

## 20.4 Atomicity

Where the underlying application supports atomic application, a multi-operation AI Change Set should behave as one normal undoable application action.

If some operations cannot safely be applied together, the system must split them explicitly rather than silently partially mutating the project.

## 20.5 Rejection

Rejecting a proposed Change Set must leave project content unchanged.

## 20.6 Expiration

Pending Change Sets should become invalid if:

- target objects were deleted;
- base version changed materially;
- permissions changed;
- required external data disappeared;
- the project was replaced/restored.

The user should be shown why reapplication is required.

---

# 21. Batch Operations

The assistant should support batch work because one of the major purposes of a command assistant is reducing repetitive manual work.

Examples:

> “Create Scene Cards for these 12 selected beats.”

> “Add these five confirmed breakdown elements to every selected scene.”

> “Rename this location everywhere it is structurally referenced.”

> “Show all scenes without a location.”

## 21.1 Batch safety

A batch command must not be interpreted as permission to silently apply all changes.

Required flow:

```text
Request
 ↓
Resolve target set
 ↓
Show count
 ↓
Prepare Change Set
 ↓
Show affected modules
 ↓
User selects/accepts
 ↓
Apply
```

## 21.2 Large batch operations

For a large operation, preview should include:

- total objects;
- grouped affected modules;
- exclusions;
- conflicts;
- locked objects;
- permissions;
- expected result;
- irreversible actions.

Example:

> “This will modify 263 references across 7 modules. 11 references are excluded because they are locked. 4 are ambiguous. Review required.”

---

# 22. AI and Existing Source-of-Truth Rules

The assistant must preserve the product's existing hierarchy.

## 22.1 Idea Vault

AI can read and suggest operations on Idea Vault items.

The assistant must not turn Idea Vault into a live synchronization layer.

## 22.2 Story Board

AI can prepare and modify Story Board data after confirmation.

Story Board changes must not silently rewrite screenplay text.

## 22.3 Screenplay

Screenplay text is authoritative for written script content.

AI-created text becomes screenplay content only after explicit acceptance through the screenplay editing workflow.

## 22.4 Production Source

The assistant may read and analyze the selected Production Source.

It must not silently change the source draft because a production issue is detected.

## 22.5 Breakdown

AI may suggest breakdown elements.

Confirmed production decisions remain user-controlled.

## 22.6 Schedule

AI may analyze and propose changes.

Shooting order remains user-controlled.

## 22.7 Call Sheet

AI may generate or prepare a call sheet.

The resulting document does not automatically mutate the schedule.

## 22.8 Historical snapshots

AI must preserve historical snapshot integrity.

Current-state modifications must not silently rewrite historical exports or issued documents.

---

# 23. Private Notes and Sensitive Project Data

## 23.1 Private-note access

Private notes must follow existing project permissions.

A user's own private notes may be used when the current user asks to use them and the operation is permitted.

Another collaborator's private notes must never be exposed through AI.

## 23.2 Cross-user queries

Example:

> “What is the writer secretly planning?”

If the only source is another user's private notes, the assistant must deny the access and explain that the content is private.

The response must not reveal the existence or contents of the protected text beyond what the permission model allows.

---

# 24. Local AI and External AI

## 24.1 Local AI

Local AI is the preferred path for:

- private project work;
- offline operation;
- low-latency assistance where practical;
- situations where users do not want project content transmitted externally.

Local AI processing must remain consistent with local-first product behavior.

## 24.2 External AI

External AI is optional.

When external AI is used, the user must be informed before transmission.

The disclosure must state:

- that an external provider is being used;
- what context is being sent;
- what category of project information is included;
- where practical, the configured provider/model identity.

The user must be able to cancel before transmission.

## 24.3 No misleading disclosure

The product must never imply that an external request is local.

Likewise, if local AI is being used, the UI must not claim that data is being sent to a cloud provider.

## 24.4 External AI and mutation

Even when an external model is used, all actual project mutations remain local OpenFrame application operations subject to local permission and confirmation rules.

The external model never receives direct authority to write the project.

---

# 25. Small Local Model Strategy

## 25.1 Product principle

OpenFrame should not require a large language model merely to operate the product.

A small local model can be the reference target for the assistant because most authoritative operations are performed by deterministic application tools.

Conceptually:

```text
Small LLM
  = language interface / intent router / explanation layer

OpenFrame Tools
  = project knowledge / search / calculations / validation / mutation
```

## 25.2 Reference model target

A model in the approximate small-model class, such as a **0.5B-parameter local model**, is a valid initial implementation target.

Example reference target:

> Qwen2.5-0.5B-class local model.

This is an implementation target, not a permanent product dependency.

The specification does not require a specific model family.

## 25.3 What the small model must NOT do alone

The small model should not be responsible for:

- counting project objects from raw text;
- exact page calculations;
- deciding permissions;
- directly editing project files;
- calculating version diffs;
- determining schedule conflicts from memory;
- enforcing business rules;
- validating Change Sets;
- determining source-of-truth;
- retrieving hidden private data.

## 25.4 What makes the small model sufficient

The implementation should minimize the language model's required world model.

The model mainly needs to:

1. understand the user's request;
2. identify the intended operation;
3. identify relevant entities;
4. generate structured tool parameters;
5. interpret deterministic tool results;
6. explain them;
7. prepare a clear mutation proposal when required.

The application supplies the authoritative data and operations.

## 25.5 Model failure must not become data failure

If the model fails to understand a command:

- ask for clarification;
- offer recognized interpretations;
- leave project state unchanged.

A weak model response must never cause an unsafe fallback mutation.

---

# 26. Model-Agnostic Tool Contract

The AI implementation should be replaceable without changing project data semantics.

Conceptually:

```text
User request
 ↓
Model Adapter
 ↓
Intent / Tool Request
 ↓
OpenFrame Tool Layer
 ↓
Structured Tool Result
 ↓
Model Adapter
 ↓
User response
```

The project should never depend on a particular model's internal state.

Changing from one local model to another must not require migrating project objects.

---

# 27. Structured Tool Invocation

The AI Tool Invocation is a logical supporting record for an application-level operation requested by the assistant. It preserves the request, selected tool, parameters, authorization outcome and execution state without giving the language model direct storage authority.

The model should request application actions through structured commands.

Illustrative logical format:

```text
tool: get_script_statistics

parameters:
  project_id: current_project
  draft_id: draft_8
  metrics:
    - unique_character_cues
    - scene_count
    - page_count
```

For a mutation:

```text
tool: prepare_rename

parameters:
  project_id: current_project
  source_object: character:123
  old_name: Ravi
  new_name: Raghav
  structured_references: true
  arbitrary_text: false
```

The exact serialization format is implementation-specific.

The behavioral requirement is that the model does not get unrestricted write access.

---

# 28. Validation of AI Tool Requests

Before executing a tool:

1. Validate requested operation.
2. Validate parameter types.
3. Resolve referenced objects.
4. Resolve scope.
5. Check user permissions.
6. Check object state.
7. Check lock/approval restrictions.
8. Check whether confirmation is required.
9. Execute read/query or prepare mutation.
10. Never silently downgrade a mutation into a different operation.

Example:

If the user says:

> “Delete the old draft.”

The assistant must not choose the nearest draft arbitrarily.

It must identify the target and present a deletion confirmation.

---

# 29. AI Response Contract

Responses should distinguish:

### Answer

> “Draft 8 contains 13 distinct character cues.”

### Observation

> “The current Production Source is Draft 8.”

### Suggestion

> “I found three likely props in Scene 14.”

### Proposed action

> “I prepared changes for 17 structured references.”

### Applied action

> “Applied the accepted rename to 17 structured references.”

The assistant must not describe a suggestion as if it were applied.

---

# 30. Provenance and Explainability

For exact or consequential answers, the assistant should identify:

- source scope;
- draft/version;
- object category;
- filters;
- calculation basis;
- whether the value is exact or inferred.

Example:

> “There are 8 distinct locations in Draft 7, based on parsed screenplay scene headings. The Production Catalog currently contains 10 Location records.”

This is more trustworthy than simply saying:

> “There are 8 locations.”

---

# 31. Error and Failure Behavior

## 31.1 Model unavailable

Show:

> “AI is currently unavailable. OpenFrame's core workflows continue to work normally.”

Project state remains unchanged.

## 31.2 Tool unavailable

If the assistant cannot access the required project tool:

> “I can explain the workflow, but I cannot retrieve the project count right now.”

Do not invent a result.

## 31.3 External provider failure

Project state remains unchanged.

## 31.4 Permission denial

The assistant explains that the requested action is not permitted for the current user.

It must not suggest bypasses.

## 31.5 Ambiguous request

Ask one focused clarification.

## 31.6 Stale data

If the requested operation was prepared against an older version:

> “The project changed after this suggestion was prepared. Review is required before applying it.”

No automatic application.

## 31.7 Partial failure

If a batch operation cannot apply fully:

- do not claim full completion;
- identify successful/pending/failed groups;
- preserve the original project wherever rollback is required;
- show the user what remains.

---

# 32. Undo, Recovery, and Activity

## 32.1 Undo

Accepted AI mutations must become normal undoable application actions wherever the underlying action is undoable.

Existing FSD behavior states that an accepted mutation becomes a normal application action and is undoable.

The AI specification preserves this.

## 32.2 Recovery

AI failures must never destroy the last safe local state.

## 32.3 Activity history

Meaningful applied AI actions should produce activity/history entries.

Example:

> “AI-applied: renamed Character Ravi → Raghav across 17 structured references.”

The activity entry should identify:

- actor = current user;
- origin = AI-assisted action;
- action;
- affected scope;
- time;
- result.

The activity system remains an application audit/history mechanism, not a transcript of every token or internal reasoning step.

## 32.4 Rejected suggestions

A rejected suggestion must not appear as an applied project mutation.

Optional AI history may retain the fact that a suggestion was generated/rejected if the product chooses to preserve AI conversation history.

---

# 33. AI Conversation Persistence

Conversation history is optional and must remain distinct from project content.

The assistant must not silently convert:

```text
Conversation
```

into:

```text
Project Note
Scene Card
Screenplay
Breakdown
Task
```

without explicit user acceptance.

If the user says:

> “Save that as a Project Note.”

the assistant prepares a Project Note creation and requires confirmation because it creates project data.

---

# 34. Security Boundary

The assistant must never receive unrestricted filesystem or project-package access as a substitute for OpenFrame's object model.

Preferred logical boundary:

```text
AI
 ↓
Authorized Application Tools
 ↓
Canonical Domain/Data Layer
 ↓
Project
```

Not:

```text
AI
 ↓
Unrestricted file write
 ↓
Project corruption risk
```

The language model must not modify project files directly.

---

# 35. Search and Retrieval Strategy

## 35.1 Exact structured search first

For questions about:

- names;
- counts;
- statuses;
- scene IDs;
- object relationships;
- draft versions;
- schedule assignments;

structured project queries should be preferred.

## 35.2 Text search when appropriate

For questions about screenplay wording:

> “Find every scene where someone mentions the railway bridge.”

use screenplay text search.

## 35.3 Semantic retrieval when useful

Semantic retrieval may be used for:

- similar notes;
- related research;
- conceptually similar scenes;
- approximate reference discovery.

However semantic similarity must not replace exact structured queries when the user asks for exact facts.

## 35.4 Retrieval grounding

Retrieved text must remain attributable to its source object.

The assistant should preserve enough metadata to identify:

- project;
- object;
- draft;
- scene;
- source type.

---

# 36. Cross-Project Behavior

The Global Idea Vault exists outside individual projects.

The assistant must distinguish:

```text
Global information
```

from:

```text
Current project information
```

A request such as:

> “Find my thriller ideas about buses.”

may legitimately use the Global Idea Vault.

A request such as:

> “How many characters are in the film?”

must use the current project's screenplay/project context.

The assistant must not merge unrelated projects when answering current-project questions.

Cross-project operations require explicit user direction.

Example:

> “Copy this idea from the Global Idea Vault into the current project.”

That becomes a project mutation and requires confirmation.

---

# 37. Episodic / Series Behavior

The assistant must understand:

```text
Series
 ↓
Season
 ↓
Episode
 ↓
Story
 ↓
Screenplay
 ↓
Production
```

Queries may be scoped to:

- series;
- season;
- episode;
- multiple episodes;
- whole series.

Examples:

> “How many episodes are in Season 2?”

> “Which characters appear in Episodes 1 and 3?”

> “How many locations does Episode 4 use?”

> “Which production elements recur across the season?”

A cross-episode mutation must show the exact affected episodes before application.

---

# 38. Collaboration Behavior

The assistant must operate inside the current collaboration permission context.

During local-network collaboration:

- AI cannot bypass object permissions;
- AI cannot hide conflicts;
- AI cannot apply stale changes without conflict review;
- AI mutations must be treated like other edits;
- another participant's private notes remain protected.

If another user changes an affected object after an AI Change Set was generated, revalidation is required before applying the stale Change Set.

---

# 39. AI and Imports/Exchange Packages

The assistant may help with:

- import inspection;
- package summaries;
- conflict explanation;
- mapping explanation;
- review-package summaries.

It must not:

- blindly overwrite the host project;
- silently merge conflicts;
- silently discard imported changes.

The underlying exchange package rules remain authoritative.

---

# 40. AI and Files

The assistant may answer questions about Project Files when the file is accessible through OpenFrame's supported file representation.

Examples:

> “Find the cinematography reference.”

> “Which project files mention the railway station?”

> “Summarize this production design PDF.”

For external files that cannot be accessed, the assistant must say so rather than hallucinating their contents.

---

# 41. AI Scope and Context UI Requirements

The existing UX already defines a right-side AI panel.

The system-wide model requires the panel to support at least:

```text
AI Assistant

Scope
[ Current Scene ▾ ]

Context
Using: Scene 24 + Characters + Breakdown

Conversation

Answer / Result

For mutations:
Proposed Changes
Affected objects
Impact
Conflicts
[Review Changes]
[Apply]
[Cancel]
```

## 41.1 Required states

At minimum:

- AI Off;
- Ready;
- Interpreting;
- Retrieving;
- Generating;
- Answer;
- Proposal;
- Awaiting Approval;
- Applying;
- Applied;
- Rejected;
- Stale;
- Conflict;
- Failed;
- External Disclosure;
- Unavailable.

## 41.2 User should know what happened

The UI must differentiate:

```text
AI answered.
AI suggested.
AI prepared changes.
User accepted.
Application applied.
```

These are not the same state.

---

# 42. Universal Command / Quick-Action Integration

The PRD already defines project-level Quick Actions and the existing UX/FSD support command/search access.

The AI should complement those systems rather than replace simple deterministic quick actions.

Examples:

```text
Quick Action:
+ New Scene Card

AI:
“Create a Scene Card describing the train station encounter.”
```

The first is direct UI action.

The second is natural-language orchestration.

Both should eventually invoke the same underlying application capability.

This keeps AI from creating a second incompatible command system.

---

# 43. AI Must Reuse Existing Application Operations

The assistant must not create parallel versions of product functionality.

Examples:

- AI-created Scene Card must become a normal Scene Card.
- AI-created Breakdown Element must become a normal Breakdown Element.
- AI-created Task must become a normal Task.
- AI-created Call Sheet must become a normal Call Sheet.
- AI-applied rename must behave like a normal rename.
- AI-applied schedule edit must behave like a normal schedule edit.

This means users can continue editing AI-created data manually after acceptance.

---

# 44. AI Operation Lifecycle

Every AI request should follow a predictable lifecycle.

```text
1. User request
       ↓
2. Determine scope
       ↓
3. Determine intent
       ↓
4. Check access
       ↓
5. Retrieve product/project data
       ↓
6. Determine operation class
       ↓
7A. Read/compute/navigate
       ↓
   Execute + answer

or

7B. Suggest/prepare
       ↓
   Build proposal
       ↓
   Preview
       ↓
   User approval
       ↓
   Validate
       ↓
   Apply normal application action
       ↓
   Record undo/activity
```

The assistant must not shortcut this lifecycle for convenience.

---

# 45. AI Actions Matrix

| Operation | AI may prepare? | Immediate execution? | Confirmation required? | Normal undo? |
|---|---:|---:|---:|---:|
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

---

# 46. AI Acceptance Criteria

The AI implementation is considered functionally correct only if all of the following hold.

## AI-AC-001 — Read-only answer

Given a project containing known structured data, the assistant can answer a supported question without changing project content.

## AI-AC-002 — Exact deterministic count

Given a screenplay with known characters, scenes, and locations, the assistant returns the deterministic count from the canonical data/query layer.

## AI-AC-003 — Scope disclosure

When a query is context-dependent, the assistant shows or states the scope used.

## AI-AC-004 — No hallucinated exact facts

When the application cannot obtain a required project fact, the assistant does not invent it.

## AI-AC-005 — Tool-based navigation

A user can ask to open supported project objects/workspaces in natural language.

## AI-AC-006 — Suggestion without mutation

Asking for a breakdown suggestion does not create breakdown elements until accepted.

## AI-AC-007 — Mutation preview

Every project-changing AI action presents a preview before mutation.

## AI-AC-008 — Explicit acceptance

Rejecting a proposal leaves project content unchanged.

## AI-AC-009 — Normal application mutation

Accepting a proposal produces the same canonical project object/action that a normal UI operation would produce.

## AI-AC-010 — Undo

Accepted AI mutations are undoable wherever the underlying operation is undoable.

## AI-AC-011 — Permission inheritance

AI cannot mutate content the current user cannot mutate.

## AI-AC-012 — Locked-state enforcement

AI cannot bypass screenplay lock/revision rules.

## AI-AC-013 — Private-note protection

AI cannot expose another user's private notes.

## AI-AC-014 — Stale Change Set detection

A proposed Change Set generated against an outdated base state is revalidated before application.

## AI-AC-015 — Global rename safety

A character rename distinguishes structured references from arbitrary screenplay text and requires explicit choice for arbitrary text replacement.

## AI-AC-016 — Batch safety

A batch mutation shows affected counts and objects before application.

## AI-AC-017 — External disclosure

External AI requests show the relevant data-disclosure UI before transmission.

## AI-AC-018 — Offline local AI

A locally configured AI assistant can operate without internet to the extent supported by the installed local model/tools.

## AI-AC-019 — AI optionality

The core product remains usable when AI is disabled.

## AI-AC-020 — AI failure safety

AI failure leaves local project data unchanged.

## AI-AC-021 — Cross-module query

The assistant can answer a question that requires traversing supported relationships across multiple workspaces.

## AI-AC-022 — No autonomous mutation

No project-changing action occurs without explicit user acceptance.

## AI-AC-023 — Product knowledge

The assistant can answer supported questions about OpenFrame's own workflow using the current product knowledge source.

## AI-AC-024 — Conversation context

The assistant can resolve simple follow-up references using current conversation/session context without corrupting project state.

## AI-AC-025 — Model replacement

Replacing the underlying language model does not change project data semantics or authorization rules.

---

# 47. Required AI Query Examples

The following examples are normative capability examples, not UI copy requirements.

## 47.1 Script statistics

> “How many characters are in this script?”

Expected behavior:

- resolve current/selected draft;
- query screenplay;
- calculate distinct Character elements;
- optionally compare against Character Room records;
- return exact counts.

## 47.2 Location statistics

> “How many locations are in the script?”

Expected behavior:

- query screenplay scene headings;
- count distinct locations;
- distinguish screenplay locations from Production Catalog locations.

## 47.3 Scene usage

> “Which scenes contain both Ravi and Arjun?”

Expected behavior:

- query screenplay scenes;
- identify character references;
- return scene identities/display numbers;
- do not mutate anything.

## 47.4 Draft comparison

> “What changed between Draft 7 and Draft 8?”

Expected behavior:

- query canonical draft/version comparison;
- summarize differences;
- identify downstream impact where available;
- no mutation.

## 47.5 Breakdown

> “Break down Scene 24.”

Expected behavior:

- identify target scene;
- retrieve relevant screenplay content;
- prepare candidate production elements;
- present suggested list;
- require acceptance before adding.

## 47.6 Character rename

> “Rename Ravi to Raghav across the project.”

Expected behavior:

- resolve Character record;
- find structured references;
- identify raw text matches separately;
- prepare exact Change Set;
- show counts and exclusions;
- require confirmation;
- apply only accepted operations.

## 47.7 Schedule

> “What scenes are unscheduled?”

Expected behavior:

- compare eligible screenplay scenes against schedule entries;
- return exact list.

## 47.8 Call sheet

> “Create a call sheet for Day 4.”

Expected behavior:

- retrieve Shooting Day 4;
- prepare call-sheet content using existing schedule data;
- present preview;
- require explicit acceptance before creating the Call Sheet.

## 47.9 Project-wide search

> “Find every mention of the railway station across the project.”

Expected behavior:

- search supported project content;
- group results by object type/module;
- distinguish structured references from raw text results.

## 47.10 Software question

> “Why is my call sheet marked stale?”

Expected behavior:

- use current product knowledge;
- inspect actual call-sheet/schedule version relationship;
- explain both the product rule and the actual project state.

---

# 48. AI and Global Search

Global Search and AI should not become the same thing.

Global Search remains the deterministic search interface.

AI can use Global Search internally when interpreting a natural-language search request.

Example:

> “Find all scenes where the character is at the hospital at night.”

The assistant may translate this into structured search filters and text search.

But the user should still be able to use the direct Global Search UI independently.

---

# 49. AI and Existing Quick Actions

The existing Quick Actions remain direct, predictable commands.

AI should extend them with natural language.

Example:

```text
Direct action:
New Character

AI:
“Create a Character named Meera, role label Detective, with this description…”
```

The AI prepares the same canonical Character object.

The Quick Action system remains useful when the user prefers deterministic UI.

---

# 50. Data Model Extensions Required by AI

The existing Domain/Data specification already contains:

- AI Request;
- AI Result;
- Change Set;
- Snapshot.

These should remain.

The following logical additions or field extensions are recommended.

## 50.1 AI Request — additional logical fields

Potential fields:

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
| external_processing_state | Local/External/Not Sent |
| model_reference | provider/model descriptor if applicable |
| created_at | request timestamp |
| completed_at | completion timestamp |
| status | lifecycle state |

## 50.2 AI Result — additional logical fields

Potential fields:

| Field | Meaning |
|---|---|
| result_id | stable result identity |
| request_id | source request |
| content | user-facing answer |
| structured_data | returned deterministic values |
| provenance | source scope/version/objects |
| suggested_changes | optional Change Set |
| confidence_state | Exact/Inferred/Unavailable where useful |
| status | Informational/Pending Approval/Accepted/Rejected/Applied/Failed |
| error | optional failure information |
| created_at | timestamp |

“Confidence” must not become a numeric pseudo-certainty score for exact project facts. Exact deterministic results should be marked as exact.

## 50.3 Change Set — AI-specific provenance

Existing Change Set fields remain.

Additional logical provenance may include:

- AI Request reference;
- AI Result reference;
- user approver;
- approval timestamp;
- approval scope;
- application timestamp.

## 50.4 Tool Invocation

A logical supporting construct may represent an AI-selected application operation.

Possible fields:

| Field | Meaning |
|---|---|
| invocation_id | stable identity |
| request_id | parent AI Request |
| tool_name | logical application tool |
| parameters | structured parameters |
| target_objects | resolved target set |
| authorization_state | allowed/denied |
| execution_state | pending/running/succeeded/failed |
| result_reference | result link |

This is an internal supporting construct unless later promoted into a user-visible audit feature.

## 50.5 AI Context Reference

A logical supporting construct may preserve what the assistant used:

- current project;
- draft;
- scene;
- selected objects;
- project scope;
- permission context.

This supports explainability and safe mutation previews.

---

# 51. Domain Invariants for AI

The Domain/Data specification should adopt these invariants.

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
14. External provider usage is represented when applicable.
15. Exact project facts are derived from authoritative data.

---

# 52. FSD Changes Required After This Specification

The current FSD AI contract should be expanded from the existing twelve AI requirements into a fuller system-wide AI contract.

The existing IDs remain valid:

```text
FSD-AI-001  Optional
FSD-AI-002  Context selector
FSD-AI-003  Question answering
FSD-AI-004  Scene card suggestion
FSD-AI-005  Breakdown suggestion
FSD-AI-006  Synopsis
FSD-AI-007  Schedule advice
FSD-AI-008  Mutation preview
FSD-AI-009  Accept/reject
FSD-AI-010  No silent rewrite
FSD-AI-011  External disclosure
FSD-AI-012  AI failure safety
```

These should remain.

Additional functional requirements should be added in the later FSD revision for:

- system-wide command execution;
- deterministic project statistics;
- exact project queries;
- cross-module traversal;
- navigation commands;
- structured replace;
- batch operations;
- Change Set base-version validation;
- permission inheritance;
- locked-state enforcement;
- private-note protection;
- AI product knowledge;
- tool invocation validation;
- AI audit/activity integration;
- stale proposal handling;
- AI operation lifecycle;
- cross-project boundary;
- episodic scope;
- collaboration/conflict behavior;
- model-agnostic operation.

These should receive priorities according to the updated PRD.

---

# 53. PRD Changes Required After This Specification

The PRD already contains the correct high-level idea:

> Amazon Q-style project assistant.

The later PRD revision should expand the current AI section so it explicitly defines:

1. AI as a system-wide assistant, not only a contextual chatbot.
2. AI as a natural-language control plane over OpenFrame.
3. Exact project questions and deterministic statistics.
4. Broad read access within user permissions.
5. Command-based navigation and search.
6. Project-changing actions through confirmation-gated Change Sets.
7. Safe batch operations.
8. Structured project-wide rename/replace.
9. Cross-module orchestration.
10. Local small-model operation as a supported target.
11. AI not being required for core workflows.
12. No autonomous/background project mutation.
13. AI product knowledge and project knowledge as separate concepts.

The PRD should keep AI optional rather than making the product depend on it.

---

# 54. UX/UI Changes Required After This Specification

The current UX AI panel already provides:

- scope selection;
- conversation;
- answer area;
- mutation preview;
- accept/reject;
- external disclosure.

The later UX revision should expand that panel into a system-wide assistant surface.

It should visually support:

```text
Natural-language command
 ↓
Scope
 ↓
What AI is using
 ↓
Answer / Result
 ↓
For mutations:
Exact preview
Affected objects
Impact
Conflicts
Approval
 ↓
Applied state
```

Additional UX requirements:

- command entry available from the application shell;
- contextual AI actions available inside relevant workspaces;
- clear distinction between answer and mutation;
- batch preview;
- affected-object counts;
- structured vs raw-text rename choices;
- stale Change Set state;
- permission denial state;
- local/external AI indication;
- product-data provenance where useful;
- no autonomous-action indicators.

The UX should not create a separate “AI application” disconnected from the OpenFrame shell.

---

# 55. Domain/Data Changes Required After This Specification

The current Domain/Data model already has the essential foundation:

```text
AI Request
AI Result
Change Set
Snapshot
```

These should be extended rather than replaced.

Required logical extensions:

- AI Request intent;
- operation class;
- resolved scope;
- resolved targets;
- authorization state;
- AI result provenance;
- tool invocation support;
- user approval reference;
- applied/rejected status;
- base-version protection;
- stale/conflict state;
- optional product-knowledge reference;
- optional model/provider reference.

The canonical project entities remain unchanged.

The AI is a control layer over them, not a replacement for them.

---

# 56. Conflict Assessment with Existing Documents

## 56.1 PRD

### Existing compatibility

The concept is explicitly compatible because the PRD already describes AI as an Amazon Q-style project assistant.

The existing user-control principle also matches the requested behavior.

### Gap

The current scope is narrower than the desired system-wide command assistant.

Therefore the PRD must later expand AI from:

```text
Assistant with selected capabilities
```

to:

```text
System-wide assistant and command interface over existing product capabilities
```

No fundamental product-philosophy conflict exists.

## 56.2 FSD

### Existing compatibility

The FSD already supports:

- scoped AI;
- project questions;
- suggestions;
- mutation preview;
- explicit acceptance;
- no silent rewrite;
- external disclosure;
- AI failure safety.

### Gap

The FSD does not yet fully specify:

- universal command execution;
- deterministic query tools;
- broad project statistics;
- cross-module orchestration;
- structured project-wide rename;
- batch operations;
- richer permission gating;
- Change Set base-version semantics;
- product knowledge;
- tool invocation;
- model abstraction.

The FSD must therefore be expanded.

## 56.3 UX/UI

### Existing compatibility

The UX already has an AI side panel and explicit preview/accept/reject behavior.

### Gap

The UX does not yet fully expose the AI as a universal application command surface.

The later UX revision must integrate AI with:

- shell command access;
- contextual actions;
- result provenance;
- exact statistics;
- batch previews;
- structured replacement;
- permission states;
- stale proposals.

## 56.4 Domain/Data

### Existing compatibility

The Domain/Data specification already defines:

- AI Request;
- AI Result;
- Change Set;
- Snapshot;
- AI traceability.

### Gap

Those entities need enough structure to represent:

- intent;
- scope;
- tool execution;
- authorization;
- approval;
- provenance;
- base versions;
- stale/conflict state.

This is an extension, not a model replacement.

---

# 57. Important Architectural Principle

The OpenFrame AI should be thought of as:

> **an intelligent interface over the software's existing capabilities**

not:

> **an LLM that happens to know about the software**

This distinction is what allows a small model to remain useful.

The project data remains in OpenFrame.

The product rules remain in OpenFrame.

The calculations remain in OpenFrame.

The permissions remain in OpenFrame.

The model translates human language into safe application operations.

---

# 58. What “Knows Everything About the Software” Means

For specification purposes, “knows everything” must not mean that every product fact is permanently embedded in the language model.

It means the assistant can access the authoritative sources required to answer supported questions about:

```text
Product behavior
+
Current project data
+
Current user/session context
+
Supported object relationships
+
Application capabilities
```

The AI should therefore be considered complete when it can reliably:

1. understand supported OpenFrame terminology;
2. locate the relevant product rule;
3. locate the relevant project data;
4. calculate exact supported facts;
5. navigate to the relevant objects;
6. explain relationships;
7. prepare supported application actions;
8. obtain explicit approval;
9. apply the normal application operation safely.

---

# 59. What “Does Everything” Means

The assistant does not need an independent implementation of every application function.

Instead:

```text
If OpenFrame can do it,
AI should be able to route a supported natural-language request
to the same underlying operation.
```

This should be the target for supported product operations.

Examples:

- create;
- edit;
- rename;
- duplicate;
- move;
- archive;
- restore;
- search;
- inspect;
- compare;
- export;
- break down;
- schedule;
- create call sheet;
- create task;
- create notes;
- manage supported production data.

Whether the assistant exposes every low-level field should depend on whether that field is a meaningful user-facing operation.

The goal is broad capability coverage, not a giant list of artificial AI commands.

---

# 60. Things the AI Must Never Become

OpenFrame AI must not become:

- an autonomous script writer that modifies canon without asking;
- an autonomous production manager;
- an autonomous scheduler;
- an autonomous approval authority;
- a background cleanup agent;
- an unrestricted filesystem agent;
- a hidden cloud synchronization mechanism;
- a second source of truth;
- a second project database;
- a permission bypass;
- a private-data leakage mechanism.

---

# 61. Future Extensibility

The architecture should leave room for:

- richer natural-language workflows;
- additional deterministic analytics;
- richer continuity analysis;
- broader document understanding;
- voice commands;
- more advanced multi-step actions;
- optional larger models;
- optional specialist models.

However, future AI capabilities must continue to obey:

```text
User request
+
Permission
+
Preview where mutation occurs
+
Explicit acceptance
+
Normal application action
```

---

# 62. Definition of Done for the AI Layer

The AI layer is ready for an implementation milestone when:

### Product understanding

The assistant can answer supported questions about OpenFrame using current product knowledge.

### Project understanding

The assistant can answer supported questions about the current project from canonical data.

### Exactness

Exact counts and statistics are calculated by deterministic application operations.

### Command capability

A user can request supported navigation and project operations using natural language.

### Mutation safety

All project-changing AI actions require visible approval.

### Cross-module behavior

The assistant can traverse supported project relationships.

### Batch behavior

The assistant can prepare multi-object changes safely.

### Permission behavior

The assistant cannot exceed the current user's permissions.

### Version safety

AI proposals cannot silently apply against an incompatible project version.

### Privacy

Private information remains protected.

### Offline/local operation

Local AI can operate without cloud dependence when configured.

### External transparency

External AI processing is explicitly disclosed.

### Recovery

AI failure does not corrupt local project content.

### Normal integration

AI-created/applied objects behave exactly like normal OpenFrame objects.

---

# 63. Cross-Document Alignment Checklist

After this AI specification is accepted, the four other documents must be revised and then re-audited.

## PRD

- [ ] AI scope expanded to system-wide assistant.
- [ ] Amazon Q-style positioning retained.
- [ ] Exact data questions added.
- [ ] Natural-language command behavior added.
- [ ] Mutation approval rule retained and expanded.
- [ ] Global rename/replace capability described.
- [ ] Batch automation described.
- [ ] Product knowledge/project knowledge concept described.
- [ ] Small local model target described as an implementation option, not a hard product dependency.
- [ ] No-autonomy rule explicit.

## FSD

- [ ] Existing FSD-AI-001..012 retained; system-wide AI behavior is expanded through FSD-AI-013..027.
- [ ] New AI functional requirements added.
- [ ] Deterministic query contract added.
- [ ] Tool invocation contract added.
- [ ] Mutation classification added.
- [ ] Change Set base-version protection added.
- [ ] Batch operations added.
- [ ] Global rename/replace behavior added.
- [ ] Permission inheritance added.
- [ ] Private-note rules added.
- [ ] Locked-state rules added.
- [ ] AI product knowledge rules added.
- [ ] AI activity/history rules added.
- [ ] Cross-module AI operations added.
- [ ] Acceptance matrix expanded.

## UX/UI

- [ ] AI panel remains the core contextual surface.
- [ ] Global command access added.
- [ ] Context/scope indicator retained.
- [ ] Exact result/provenance presentation added.
- [ ] Mutation preview expanded.
- [ ] Batch preview added.
- [ ] Structured-vs-raw-text replacement UI added.
- [ ] Permission denied state added.
- [ ] Stale Change Set state added.
- [ ] Conflict state added.
- [ ] External AI disclosure retained.
- [ ] No autonomous-action behavior retained.

## Domain/Data

- [ ] AI Request extended.
- [ ] AI Result extended.
- [ ] Change Set extended.
- [ ] Tool Invocation supporting construct added if needed.
- [ ] AI Context supporting construct added if needed.
- [ ] Approval provenance added.
- [ ] Base-version protection added.
- [ ] AI-specific invariants added.
- [ ] Existing canonical object identities preserved.
- [ ] Existing source-of-truth rules preserved.

---

# 64. Final System Definition

OpenFrame Studio AI should be defined as:

> **A system-wide, user-controlled natural-language assistant that understands OpenFrame's product rules and the user's accessible project data, can answer exact questions through deterministic application queries, can navigate and search the application by command, can prepare suggestions and multi-step operations, and can execute project-changing actions only through explicit user-approved mutations against the existing canonical OpenFrame data model.**

The assistant is not the authority.

The user is the authority over their project.

The application's canonical domain/data model is the authority over project state.

The application's deterministic functions are the authority over exact calculations and validation.

The AI is the interface that connects ordinary human language to those capabilities.

The intended operating model is:

```text
USER
  │
  │ natural language
  ▼
OPENFRAME AI
  │
  ├── product knowledge
  ├── session context
  ├── project scope
  ├── deterministic queries
  ├── application tools
  └── permission checks
  │
  ├── read / compute / navigate
  │       └── execute directly when explicitly requested
  │
  └── suggest / mutate
          └── preview
              └── user approval
                  └── normal OpenFrame action
                      └── undo / activity / recovery
```

This is the required foundation for making OpenFrame AI feel like a “Jarvis for the application” without giving the AI independent authority over the project.

---

# Appendix A — Canonical User Command Examples

## Read / Compute

- “How many characters are in Draft 8?”
- “How many locations are used in the screenplay?”
- “How many scenes are unscheduled?”
- “Which scenes contain Ravi?”
- “Which characters appear in Scene 42?”
- “How many pages are in the shooting draft?”
- “How many props are confirmed?”
- “How many locations are approved?”
- “Which call sheets are stale?”
- “What changed between Draft 7 and Draft 8?”
- “How many shots are in Scene 14?”
- “How many storyboard panels are linked to Scene 14?”
- “Which scenes were added in Draft 8?”
- “Which shooting days contain the railway station?”
- “How many episodes are in Season 2?”

## Navigation

- “Open Scene 24.”
- “Show Draft 7.”
- “Open the railway station location.”
- “Take me to the unscheduled scenes.”
- “Show all scenes involving Arjun.”
- “Open the latest call sheet.”
- “Show the production catalog.”

## Suggest / Prepare

- “Suggest a breakdown for Scene 24.”
- “Turn this note into a Scene Card.”
- “Prepare a synopsis.”
- “Suggest schedule groupings for the night scenes.”
- “Prepare a call sheet for Day 4.”
- “Prepare a rename from Ravi to Raghav.”
- “Find all affected references before renaming Ravi.”

## Mutate with approval

- “Create the Scene Card.”
- “Apply the accepted breakdown suggestions.”
- “Rename Ravi to Raghav.”
- “Apply the selected schedule changes.”
- “Create the call sheet.”
- “Add this as a Project Note.”
- “Delete these selected Scene Cards.”

---

# Appendix B — Safe Interpretation Examples

## Example 1 — Exact count

User:

> “How many characters are there?”

Assistant should not immediately assume one metric if multiple valid counts exist.

Preferred:

> “In the current screenplay draft there are 13 distinct character cues. The Character Room contains 15 Character records.”

## Example 2 — Ambiguous draft

User:

> “How many scenes?”

If multiple drafts exist and there is no current draft context:

> “Which draft should I count: Draft 7, Draft 8, or the locked shooting draft?”

## Example 3 — Rename

User:

> “Rename Ravi everywhere.”

Assistant:

> “I can prepare that. I found one Character record named Ravi and 17 structured references. I also found 12 raw text mentions in dialogue/action, which are separate from structured references. Review scope?”

## Example 4 — Deletion

User:

> “Clean up unused cards.”

Assistant:

> “I found 14 Scene Cards currently parked. I can prepare them for deletion, but none will be deleted automatically. Review the 14 cards?”

## Example 5 — Schedule

User:

> “Fix the schedule.”

Assistant must not guess what “fix” means.

Preferred:

> “I found two scheduling conflicts: cast overlap on Day 3 and a location conflict on Day 5. I can prepare proposed changes for review.”

---

# Appendix C — Minimal AI Request Lifecycle States

```text
Created
  ↓
Context Resolved
  ↓
Authorized
  ↓
Retrieving
  ↓
Tool Execution
  ↓
Result Ready
  ├── Informational
  └── Proposal
         ↓
     Pending Approval
       ├── Accepted
       │     ↓
       │   Validating
       │     ↓
       │   Applying
       │     ↓
       │   Applied
       └── Rejected
```

Failure branches:

```text
Unauthorized
Ambiguous
Unavailable
Failed
Stale
Conflict
Canceled
```

---

# Appendix D — Core AI Design Principle

The implementation should optimize for this equation:

```text
Smaller language model
+
strong application tools
+
canonical project data
+
deterministic calculations
+
strict permission control
+
explicit mutation approval
+
good product knowledge
=
powerful OpenFrame assistant
```

Not:

```text
Very large language model
+
raw project text
+
unrestricted write access
=
OpenFrame AI
```

The first model preserves user control, determinism, portability, local-first behavior, and maintainability.

---

# Appendix E — Required Propagation Rule

This AI specification is the source for the next alignment pass.

Any later document that says:

- “AI can modify…”
- “AI can automate…”
- “AI can access…”
- “AI can rename…”
- “AI can analyze…”
- “AI can create…”
- “AI can answer…”
- “AI can update…”

must be checked against this document.

Any AI behavior that bypasses:

```text
scope
→ permission
→ proposal
→ confirmation
→ validation
→ normal application mutation
```

is inconsistent with the OpenFrame AI contract.
