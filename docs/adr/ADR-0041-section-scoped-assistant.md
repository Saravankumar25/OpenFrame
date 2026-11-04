# ADR-0041: Section-scoped assistant

- **Status:** Accepted 2026-11-03 (owner go-ahead through the release package, `OpenFrame_Release_Package/ai-reharness.md`).
- **Date:** 2026-11-03
- **Supersedes:** [ADR-0037](ADR-0037-conversational-assistant-focus-and-dossiers.md) decision 5 (domain by topic words) and
  decision 6 (Stage B plan becomes the picked sections; Stage C "one more fetch" goes, because the person widens by adding a
  section). ADR-0035 lane leftovers are confirmed retired.
- **Amends:** ADR-0037 decision 9 for narrative answers.
- **Unchanged:** ADR-0028 decisions 1 and 5, ADR-0013 decisions 6 and 7, ADR-0019, ADR-0025, ADR-0026. Same local model,
  offline; only deterministic code judges answers, at most three passes.

## Context

The Scope dropdown looked like it controlled what the assistant reads, but it changed a label. The topic came from keyword
regex over 11 domains, and the words collide ("timeline", "planned", "when"), so the assistant often read the wrong part of the
project. The hybrid search (full text, vectors, graph) had no caller on the question path. The answer policy was tuned for
short facts, so an explanation was thin or invented.

## Decision

1. The person is the router. They pick sections before asking, up to 3 leaves per question. A leaf is one screen of the app.
   The assistant never decides where to look.
2. No pre-fill from the current screen. No "Whole Project". Send stays disabled, with the hint "Pick a section", until at
   least one leaf is picked.
3. One hand-written leaf registry (`modules/ai/sections.rs`) replaces scope kind, domain and retrieval route for questions.
   A coverage test fails the build when a navigation screen or an indexed entity type maps to zero leaves or to several.
4. Only the picked leaves are read: typed fetchers for facts, leaf-scoped hybrid search for text, with full text as the
   fallback when the index is not ready, saying so. The dossier budget is split across leaves by weight with a minimum share.
5. A question that belongs elsewhere gets a sentence built by code naming the section, with no model call and no content from
   the unpicked section: "That is in Production > Budget. Add it to your selection and ask again." The permission check runs
   first, so a pointer never reveals data the person may not read.
6. New narrative answer types (describe, brief, relationship, story explanation) with higher word limits and code-built
   briefing dossiers, where every line carries its scene number.
7. The agent's tool list is filtered to tools homed in the picked leaves. The agent loop, Change Sets and human-only apply are
   unchanged.
8. The label "Using: ..." is built from the real selection and written to the trace.

## Defaults taken (the owner may change them)

Only leaves can be picked and the cap of 3 counts leaves. Idea Vault Global and This Project are two leaves. The selection is
kept within one conversation. Added branches: Notes & Tasks, App Help & Settings. Starting word limits: describe, brief and
relationship 250; story explanation 350.

Taken while building slices R1 to R3:

- The Story page and the Global Idea Vault are read only when the person picks their section for that question. This is newer than
  the 2026-10-20 owner request that kept the Story page from the assistant, and it narrows it rather than ending it: the page stays
  out of search, out of every tool and out of every other section. The owner can take the `story/page` leaf out with one table row.
- A leaf narrowed to one scene, day or call sheet reads through its fetchers and runs no text search.
- The Home screen belongs to Production Overview; Activity and Recently Deleted belong to Settings, so that every screen has one leaf.
- The trace keeps the picks in one new column, `sys_ai_trace.sections_json`: leaf ids, counts and tokens per leaf.

Taken while building slices R4 to R6:

- Memory goes by what its own words name: an entry naming something no picked section can answer about stays out of the prompt, a
  preference naming nothing stays in. An earlier turn asked under other sections stays out. The focus keeps its ids and writes only
  what the picks can answer about. The project map lists the picked sections, with no title and no scene count.
- A topic word only points when no word points at a pick, the picks' values do not hold it, and the picks returned nothing that
  answers. "Planned", "when" and a bare scene number were taken out of the word table.
- A scene is opened in the Screenplay, so "Open Scene 12" with only Breakdown picked points at Screenplay > Scenes.
- A breakdown suggestion needs Breakdown and Screenplay > Scenes. The general search tools live in every section and read only what
  the picks hold. A story explanation told "briefly" gets the short class, as an explanation does.
- The Story card no longer says who plays a character unless a picked section can answer about people.
- The unsupported relationship word and unstated mood checks start off (`NARRATIVE_WORD_CHECKS`) until a run measures their false alarms.
- A thing code knows is missing gets the fixed sentence with no model: the facts of the picks hold nothing that answers, and a statement from a fetcher speaks of what was asked. In another language the writer still says it, in the person's words.
- The Overview fetcher reads the shoot, call sheet, crew, shot and currency numbers, because that screen shows them and the test set asks them.
- Three turn-set labels moved to the new types (a description, a brief and a story explanation are no longer `explain`), so the read of the older sets is scored against what the new rules mean to do.

Taken while building slice R7 and the security review after it:

- Deleted: the scope kinds and `AskArgs.scope`, the character budget of the context, the domain table, the fetcher table and its automatic domains, the wide passages round, the retrieval router, and the project map with its titles. Migrations drop `ai_request.scope_kind` (project 0020) and `sys_ai_trace.scope_kind` and `turn_domains` (app 0011).
- Kept: `ResolvedScope` with its label, draft, scene and sections, built from the picks, because the tools and proposal builders default to them. `RetrievalRoute` with four kinds, because the tests and the performance bench measure each path alone; the assistant always asks for the widest. `Msg::with_topic`, because a picked section reads its area whatever the question says.
- The topic words are a hint. The pointer uses them to name a section. Two checks inside a picked section also use them: a bare name with nothing else asked reads who plays it, and a money question does not lean on "that". They choose nothing to read.
- The evaluation switch is `ai_eval_all_sections`: a request with no picks reads every section of the project through the same path, over the cap of 3. A person's picks stay capped. It stays false in the FFI.
- With several picks, only what the question asked for is demanded of the answer: a value stays required when the words the person typed open the fact, not the section's topic, and the fact speaks of a word of the question. One pick keeps every required value.
- What one pick does not hold is said only when the question is about it or no other pick holds anything. What a role may not see is always said.
- With several screenplays and none chosen, a section that can choose (Scenes, only under the evaluation switch) asks which one; a section that cannot reads the one with the production data.
- An answer's "scene N" must be given as a scene by the facts, the request or the conversation (rule `scene_ref`).
- A day the person picked replaces any day in the message, and an off day reads only its own call sheets. A superseded call sheet cannot be picked.
- Search tools hide contact details typed into notes from a role that may not see them. An existence pointer no longer asks for the contact role: contact fields are not in the index and a contact question is held back first.
- The reply, the text search and the closing of a failed request use the project the question was asked in, even if the person closed it or opened another meanwhile.

## Measuring

Measure first (ADR-0028 decision 1). The baseline and the new test set come before any runtime change and each slice is
measured before the next. The numbers are in `docs/release/evidence/ai-reharness/`. A missed bar is shown to the owner, never
lowered silently.

## Consequences

- Picking is a step the person must take. Users may forget; the Send hint is the only guard.
- A wrong pick costs one tap on the pointer's Add button. The false-pointer rate is tracked.
- Longer answers take longer on a processor. Progress lines only, no streaming.
