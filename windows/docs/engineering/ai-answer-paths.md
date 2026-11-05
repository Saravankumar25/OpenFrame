# AI answer path: how a message becomes an answer (Windows, updated 2026-11-03)

> The one conversational path of [ADR-0037](../adr/ADR-0037-conversational-assistant-focus-and-dossiers.md), made
> section-scoped by [ADR-0040](../adr/ADR-0040-section-scoped-assistant.md) (design:
> [ai-section-scoped-assistant.md](../specs/ai-section-scoped-assistant.md)). Built in slices S3 to S8 of
> [ai-conversational-harness.md](../specs/ai-conversational-harness.md) (the design, the measurements and the reasons),
> then rounds R1 to R7 of the section-scoped design. The five lanes it replaced are history:
> [2026-10-26-ai-answer-paths-five-lanes.md](../history/2026-10-26-ai-answer-paths-five-lanes.md). Code is the source of
> truth; this page names the files under `crates/straft-application/src/modules/ai/`.

## 1. One turn

The person picks one to three sections (leaves of `sections::LEAVES`) before asking. Code reads those and nothing else;
no model decides where to look.

| Stage | Code | What it does |
|---|---|---|
| Ask | `apps/desktop/src/ai/Conversation.tsx`, `SectionPicker.tsx`, `orchestrator.rs` | The message, the picked sections (with the screenplay, draft, scene, day or call sheet chosen under each) and the conversation id reach `ai.ask`. The core refuses none, more than three, an unknown section, one twice and one the person may not read. Progress lines and Cancel run for the whole turn. |
| Memory | `memory/` | The last 8 turns in full within about 1,200 tokens, summary lines for older turns, and up to 5 long-term entries of this user and project that match the message. |
| Resolve | `index/` | An in-memory index of everything a user can name (screenplays, scenes, characters and aliases, relationships, cast, crew, locations, acts, cards, vault items, files, budget lines, shoot days, call sheets), rebuilt when content changes. Titles, scene numbers, names, typos and pronouns are matched against it and the conversation focus. With several screenplays, the title, the focus, a scene number only one screenplay has, or the current draft decides; only when none can does the turn ask one question, and the answer is kept in the focus. |
| Answer type | `turn.rs`, `turn/` | One of lookup, list, count, explain, describe, brief, story explanation, relation, compare, brainstorm, navigate, change, help, chat, plus its group and its length class (the narrative types get the 250 and 350 word classes). Code rules first, then a correction takes the previous type, then a nearest-neighbour vote over `turn_examples.jsonl` with the bundled multilingual embedding model, and the 1B model only when the vote is weak. The topic words of `turn/domains.rs` are a hint for the pointer only. |
| Facts | `dossier/`, `sections/` | Each picked section runs its own sources (`sections::run_leaf`) and builds facts-first records (one fact per line, field names kept), the values a correct answer must state, and plain statements of what code found missing. One gate (`sections::gate`) drops any fact whose kind is not homed in a picked section. Guards that no later stage can undo: other users' private notes never, contact data only for roles that may see it, instructions found in project text marked as data, no file paths. The budget (about 1,500 tokens, 2,500 for a narrative answer) is split across the picked sections by weight; required facts are never cut. Passages come from the hybrid retriever, filtered to the picked sections. |
| Pointer | `sections/pointer.rs` | When the question lives in a section that was not picked, one fixed sentence names it ("That is in Production > Budget. Add it to your selection and ask again.") and the panel offers an Add button. Triggers in order: a name homed elsewhere, an empty result with a hit in an unpicked section, a topic word. A section the person may not read is never named. Part of the question that fits the pick is answered and the sentence follows it, added by code after the checks. |
| Write | `answer.rs`, `answer/prompt.rs` | One model call. A fixed policy per group (facts and help; narrative; brainstorm; chat with no facts) and the names of the picked sections come first and stay byte-stable so the engine reuses the cached prompt start; then memory, recent turns, focus, the facts, the missing statements, the request, the language and the length instruction. The narrative policy states only what the facts show and a relationship only from a relationship record. Token caps per length class. |
| Check | `answer/check.rs`, `text.rs` | Code only: names and digit numbers must be in the facts or the request, required facts present, length inside the class, no repeated run of words, the request's language, no ids, links, paths or internal words, missing data stated as missing, plain paragraphs for a narrative answer (a scene-by-scene briefing may be a list). The unsupported-relationship-word and unstated-mood checks exist and are off until their false positives are under 5 percent. A failure is rewritten with the offenders named, at most three passes; a pass-1 answer shows at once. |
| Fallback | `answer/fallback.rs` | After three failed passes: a lookup, count or comparison shows the verified fact in one sentence ("Lead cast, planned: 40,00,000."); any other type shows the abstention sentence with the facts that verified. Never a canned overview. |
| Navigate, change | `action.rs`, `catalog/`, `change_set.rs`, `sections/tools.rs` | Navigation runs only when the answer type is navigate and the scene or thing is inside the picked sections; otherwise the pointer. A change is always a Change Set proposal the person reviews and applies; nothing is changed by the model directly. The agent is offered only the tools homed in the picked sections, a tool outside them never runs even if the model names it, and "Suggest breakdown" needs Breakdown and Screenplay. |
| Store | `trace.rs`, `records.rs` | The exchange with its sources (the records of the facts the answer used, behind the sources button), the pointer, the focus, long-term memory per the saving rules, and the trace: answer type and how it was read, the picked sections, tokens per section, whether a pointer fired and its target, hybrid or full-text retrieval, dossier size, passes, offenders, fallback, cached tokens, stage times. |

Fixed text that stays code (spec section 4.1): refusals (licence, payment, actions the person may not do), the pointer sentences,
errors, "the model is not ready", progress and Cancel.

## 2. Engine

`crates/straft-ai/`: llama.cpp b11259, Gemma 3 1B, one slot, `cache_prompt` on every call, a 512 MB prompt store.
Warm-up on project open builds the index and embeds the example messages in the background. Progress lines only, no token
streaming (owner decision 2026-10-24).

## 3. How it is measured

`crates/straft-ai-eval`: `ai-eval` on the gold sets (`--only lt-` for the Low Tide fixtures), `--turn-only` for the
answer type, `--dossier-only` for fact recall, and a latency section on every run. Results and bars: spec sections 7.5 and
13, and the evidence folders `docs/release/evidence/ai-conversational-*-2026-10-26/`.
