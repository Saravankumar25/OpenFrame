# ADR-0037: Conversational assistant with focus, memory and per-turn dossiers

- **Status:** Accepted 2026-10-26 (owner, in chat: finish the open stages, redocument, then build in a new session;
  recorded in [owner-decisions.md](../implementation/owner-decisions.md)). Open questions 2 and 3 take the defaults
  written below until the owner changes them. Nothing below is built yet. Detail, deterministic policy, evaluation and
  slices: [ai-conversational-harness.md](../specs/ai-conversational-harness.md). First spike:
  `docs/release/evidence/ai-conversational-spike-2026-10-26/`.
- **Date:** 2026-10-26
- **Would amend, on acceptance:** [ADR-0035](ADR-0035-ai-lanes-briefings-and-code-verifiers.md) decision 1 (code picks
  one of five answer lanes for every question), [ADR-0013](ADR-0013-single-local-model-hybrid-rag-agent.md) decision 6
  (the agent loop as the general answer path) and [ADR-0028](ADR-0028-ai-quality-program-and-harness.md) decision 8
  (conversation storage, extended to long-term memory). ADR-0035 decisions 2 and 3 (typo-tolerant routing copy, intent
  prototypes) stay for exact commands only.
- **Model:** unchanged. Gemma 3 1B, local, offline (ADR-0025, ADR-0026). This ADR changes what the model is given, not
  which model runs.

## Context

The owner's requirement, stated 2026-10-26: the assistant must behave as a conversational chat assistant on the whole
platform. It must answer any question about any object in a project (screenplay, scenes, acts, characters, cast, crew,
locations, props, wardrobe, vehicles, VFX, makeup, budget, schedule, call sheets, shot lists, boards, files, settings,
product help), follow a conversation, remember what was said, brainstorm, and answer at the length the question needs.
Sources are shown behind a sources button, not inside the reply.

A 40-exchange export from 2026-10-25 (one project, two screenplays) shows the current build cannot do this. Causes in
the code, as read on 2026-10-25:

1. **Scope never narrows to a screenplay.** Draft resolution returns no draft when a project has several screenplays
   ([queries.rs](../../crates/openframe-application/src/modules/ai/queries.rs), `current_draft`). The understanding step
   runs only when a draft exists ([orchestrator.rs](../../crates/openframe-application/src/modules/ai/orchestrator.rs),
   `run_agent`). So name matching, relationship and entity context are all skipped.
2. **No conversation state.** The model sees the last four exchanges as text, each answer cut to 400 characters. A
   chosen screenplay or a resolved person is not stored, so the user repeats it.
3. **Lane routing decides the answer path with keyword rules.** A question about a relationship can fall to a scene
   opener. A question about a budget can fall to a tool that finds nothing.
4. **The only general path is a tool loop on a 1B model.** It chooses among 14 to 28 tools from a keyword-filtered
   catalogue. The model does not see the project as a whole.
5. **Only Chat and Recipe answers pass the deterministic checks** (`verify.rs`). Agent-lane answers pass only a language
   check, so a looping paragraph and wrong scene numbers were shown.

These are harness faults. A larger model on the same harness would still fail the first, second and third causes.

## Decision

1. **One conversational path for questions.** Every question is answered by the assistant. The five lanes of ADR-0035
   stop choosing the answer path. Two things stay deterministic and outside the assistant: exact commands (navigate to a
   scene, create a task, mark a task done) and proposals (Change Sets, ADR-0013 decision 7). Those keep their current
   code routes. What else stays deterministic, and why, is the table in section 4 of the spec; any new deterministic
   answer needs an entry there.

2. **A project index covers every object type.** The index is built from the database, not from prose. Each node has an
   id, a type, a label, aliases (for example "Young Anjali", "ANJALI", "Anjali"), a parent, status fields where the
   object has one, and links to related nodes. Node types, at minimum:
   - Story: idea vault items, beat board cards and acts, story board, characters, timeline, parking lot, quick capture.
   - Screenplay: screenplays, drafts, acts, sequences, scenes, scene elements.
   - Scene breakdown: characters, locations, props, cast, crew, vehicles, wardrobe, extras, background, hair and makeup,
     special effects, VFX, sound, animals.
   - People: cast and crew with contact details and availability.
   - Production: locations and their status, schedule, calendar, day out of days, daily view, sides, reports, budget
     lines, call sheets and their status.
   - Visual: mood boards, storyboards, shot lists and their status.
   - Files, settings, themes, product help (a separate documentation corpus, same pipeline).

   A changed object is re-indexed when it is written. Nothing is cached as a summary, so nothing goes stale.

3. **Focus.** The assistant keeps a focus per conversation: the screenplay, scene, act and characters the user is working
   on. Each turn, code resolves references ("C3", "that act", "the same character", "him") against the index using the
   focus first. A reference that cannot be resolved produces one clarifying question. A resolved reference is stored and
   is not asked again.

4. **Memory, two kinds.**
   - *Short-term:* the last 8 turns verbatim, plus a rolling summary of older turns. Per conversation.
   - *Long-term:* user-confirmed facts and preferences for one user in one project. Example: "Engvergis is Varghese",
     "answer in one sentence unless I ask for detail", "lead cast budget is the figure in Budget > Cast". Written only
     from an explicit user statement or an answer the user confirms. Listed, editable and deletable in Settings. Only
     the owner of the memory reads it. ADR-0028 decision 8 governs what may be stored.

5. **Answer type and domain, per turn.** Each turn gets two separate readings (the first spike showed that one mixed list
   of intents overlaps and misreads):
   - an *answer type*: lookup, list, count, explain, relation, compare, brainstorm, navigate, change (proposal only),
     help, chat (thanks, anger, small talk, a correction with no new question);
   - a *domain*, found by code from the resolved entities and topic words: story, screenplay, breakdown, people,
     production, schedule, budget, visual, files, settings, product. A turn may touch several.

   The answer type is read by one short, constrained model call, with a rule hint where a rule is sure. A message with no
   entity, no topic word and no question form is `chat`. The answer type fixes a length class: one sentence for lookup and
   count, short for list and compare, a paragraph for explain, relation, brainstorm and help, brief for chat. The user can
   override in words ("briefly", "in detail"). Code enforces the class; the model does not choose it. The owner dropped the
   Laya model on 2026-10-26, so no new classifier model is introduced here.

6. **Progressive context, per turn.** The model is never given the whole project.
   - *Stage A (always):* policy, the request, the focus card, the long-term memory entries that match the request, the
     short-term turns, and a compact project map (counts and names per area).
   - *Stage B (by answer type and domain):* a fixed fetch plan, returned as one fact per line with field names and the
     facts a correct answer must state. A relation question fetches both character cards, the scenes
     in which both appear (database query), and the text of those scenes. A budget question fetches the budget lines
     for the focus. A lookup fetches the single record.
   - *Stage C (bounded):* if the Stage B dossier leaves a gap, one more fetch is allowed, chosen by the harness from the
     index. The model may name the gap; the harness performs the fetch. No unbounded loop.
   - Each section has a token cap. The whole prompt is capped at about 3,000 tokens. Overflow drops the least relevant
     items, and the drop is recorded in the trace.

7. **Retrieval.**
   - Structured facts (counts, cast, budget, contacts, dates, status) come from typed database queries and are
     authoritative. The model may not override them.
   - Text (scene text, notes, story cards, documents) is searched with hybrid retrieval: keyword (BM25) and vector
     similarity, filtered first by the indexed entities the turn resolved to.
   - Scene chunks carry a short contextual header (act, scene heading, characters, location) before indexing, so a
     chunk makes sense alone. This follows Anthropic's contextual retrieval approach.
   - Permissions are applied inside each fetch, not after the answer is written.

8. **Context isolation.** The prompt has separate tagged blocks: policy, memory, focus and map, database results, retrieved
   text, user request. Scene text, notes, tool results and memory are data. The existing rule that only the text after
   "User request:" is an instruction stays.

9. **Answer checks and fallback.** Deterministic checks run on every answer, whatever the lane:
   - every name, number and scene reference in the answer must appear in the dossier;
   - every required fact of the fetch plan must appear in the answer (a wrong figure that exists elsewhere in the dossier
     passes the first check, as the spike showed);
   - the answer length must match the answer type's length class;
   - a repeated run of words is a failure;
   - no internal word reaches the user ("dossier", "tool", "record id").
   Up to three passes, as ADR-0028 decision 5 allows. Code, not the model, decides whether data exists: if the fetch found
   nothing, the assistant says it is not in the project and names what it searched; if it found records, the model is told
   the answer is in them. It does not invent a figure, a name or a scene. If a lookup or count still fails after three
   passes, the user sees the verified fact as one plain sentence built from the record (counted in the trace); any other
   type shows the abstention sentence plus the facts that verified. No canned overview.

10. **Sources.** Each answer carries the ids of the records it used. The UI shows them behind the sources button. The reply
    text carries no record ids, paths or metadata.

11. **Brainstorm and suggestions.** Intent "brainstorm" builds a dossier of the relevant story material and asks the model
    for options. Options are not saved unless the user asks. A change that the user wants made goes through a proposal.

12. **Model limits, stated plainly.** A 1B model is weak at choosing many tools and at long reasoning. This design keeps
    tool choice in code and gives the model a short, complete dossier to read and write from. The model does not choose
    among the 250 tools in the core path. Model-driven tool use can be added later as the Stage C expansion, only if the
    evaluation shows it helps.

## Consequences

- The Agent lane's general loop is no longer the default answer path. Its tools remain as the source of Stage B and
  Stage C fetches.
- Chat and Recipe logic is reused where it fits (story, scene and character briefings) but is no longer a separate lane.
- `scope.rs` and `queries.rs` change so that a focus, not a single current draft, drives resolution in a project with
  several screenplays.
- The conversation store gains focus and long-term memory tables. Migration number to be chosen at build time.
- The evaluation set must include multi-turn conversations. A single-turn benchmark cannot show this design working.
- Documents that describe the current lanes (`docs/engineering/ai-answer-paths.md`, `docs/specs/ai-harness-rebuild.md`)
  are updated in the same change that lands the build, not before.

## Migration (each step measured before the next)

- **S0. Baseline and eval set.** Build the multi-turn evaluation set: about 200 conversations, including the 40 exchanges
  from the 2026-10-25 export, two-screenplay projects, and questions across every domain above. Save a baseline run of the
  current build. No runtime change before this step is saved.
- **S1. Spike. Done 2026-10-26: conditional go.** On a hand-built dossier, 7 of 15 answers were good, 2 passed the
  script but were wrong, 6 failed; the answer-type read was 20 of 28. Short facts from an isolated record work; paragraphs
  embellish and brainstorming echoes the request. The findings changed decisions 5, 6 and 9 above. Detail in section 12
  of the spec.
- **S1b. Second spike** on the revised design (facts-first dossiers, required facts, answer type plus domain, chat and
  brainstorm policies), at least 60 cases and three runs each, against the spec's bars. S3 does not start before it.
- **S2. Index and focus.** Project index for every node type, and focus resolution.
- **S3. Intent and length.** Rules first, then intent prototypes.
- **S4. Dossier builders.** One per intent family: story and character, scene, people and cast, production (budget,
  calendar, call sheets), product help.
- **S5. Memory.** Short-term summary, then long-term memory and its Settings screen.
- **S6. Retire the Q&A lanes.** Keep exact commands and proposals. Update the engineering and spec documents in the same
  change.
- **S7. Sources button.** Confirm the reply carries no metadata and the sources list is complete.

## Open questions for the owner

1. The change to ADR-0035 decision 1 (one conversational path instead of five lanes): accepted by the owner in chat on
   2026-10-26.
2. Long-term memory: per user per project, visible and deletable in Settings. Default taken; owner may change.
3. Stage C may cost a second retrieval round on slow hardware. Default taken: up to one extra round per turn.
4. Coverage: the index needs a source for every node type in the list above. Some types (vehicles, wardrobe, budget
   lines, shot-list status) may not have a database table or a read tool today. S2 starts with that inventory; the
   first pass is in section 6 of the spec (themes and app settings have no read tool; budget has a summary tool only).
5. Token streaming. The owner decided on progress lines without streaming on 2026-10-24. The spike measured about 14
   tokens per second, so a 150-token paragraph needs about 10 s. Keep progress lines only, or stream the answer as it is
   written? Decide before S8.

## Sources consulted

- Anthropic, "Effective context engineering for AI agents": just-in-time retrieval over pre-loaded context, and "context
  rot" as the limit of attention. https://www.anthropic.com/engineering/effective-context-engineering-for-ai-agents
  (read through search summaries; check the original before citing exact wording).
- Contextual retrieval, prepend a short situating context to each chunk before embedding and BM25 indexing. Reported
  figures are from secondary write-ups, not verified against Anthropic's original post:
  https://gradientflow.com/contextual-retrieval-rag/
