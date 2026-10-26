# AI answer path: how a message becomes an answer (Windows, 2026-10-26)

> The one conversational path of [ADR-0037](../adr/ADR-0037-conversational-assistant-focus-and-dossiers.md), built in
> slices S3 to S8 of [ai-conversational-harness.md](../specs/ai-conversational-harness.md) (the design, the measurements and
> the reasons). The five lanes it replaced are history:
> [2026-10-26-ai-answer-paths-five-lanes.md](../history/2026-10-26-ai-answer-paths-five-lanes.md). Code is the source of
> truth; this page names the files under `crates/openframe-application/src/modules/ai/`.

## 1. One turn

| Stage | Code | What it does |
|---|---|---|
| Ask | `apps/desktop/src/ai/Conversation.tsx`, `orchestrator.rs` | The message, the scope and the conversation id reach `ai.ask`. Progress lines and Cancel run for the whole turn. |
| Memory | `memory/` | The last 8 turns in full within about 1,200 tokens, summary lines for older turns, and up to 5 long-term entries of this user and project that match the message. |
| Resolve | `index/` | An in-memory index of everything a user can name (screenplays, scenes, characters and aliases, relationships, cast, crew, locations, acts, cards, vault items, files, budget lines, shoot days, call sheets), rebuilt when content changes. Titles, scene numbers, names, typos and pronouns are matched against it and the conversation focus. With several screenplays, the title, the focus, a scene number only one screenplay has, or the current draft decides; only when none can does the turn ask one question, and the answer is kept in the focus. |
| Answer type | `turn.rs`, `turn/` | One of lookup, list, count, explain, relation, compare, brainstorm, navigate, change, help, chat, plus its group, the domains and the length class. Code rules first, then a correction takes the previous type, then a nearest-neighbour vote over `turn_examples.jsonl` with the bundled multilingual embedding model, and the 1B model only when the vote is weak. |
| Facts | `dossier/` | One registered fetcher per domain builds facts-first records (one fact per line, field names kept) for the answer group, the values a correct answer must state, and plain statements of what code found missing. Guards that no later stage can undo: other users' private notes never, contact data only for roles that may see it, instructions found in project text marked as data, no file paths. Capped at about 1,500 tokens (2,500 for a paragraph); required facts are never cut. |
| Write | `answer.rs`, `answer/prompt.rs` | One model call. A fixed policy per group (facts, narrative, help; brainstorm; chat with no facts) and the screenplay titles come first and stay byte-stable so the engine reuses the cached prompt start; then memory, recent turns, focus, the facts, the missing statements, the request, the language and the length instruction. Token caps per length class. |
| Check | `answer/check.rs`, `text.rs` | Code only: names and digit numbers must be in the facts or the request, required facts present, length inside the class, no repeated run of words, the request's language, no ids, links, paths or internal words, missing data stated as missing. A failure is rewritten with the offenders named, at most three passes; a pass-1 answer shows at once. |
| Fallback | `answer/fallback.rs` | After three failed passes: a lookup, count or comparison shows the verified fact in one sentence ("Lead cast, planned: 40,00,000."); any other type shows the abstention sentence with the facts that verified. Never a canned overview. |
| Navigate, change | `action.rs`, `catalog/`, `change_set.rs` | Navigation runs only when the answer type is navigate. A change is always a Change Set proposal the person reviews and applies; nothing is changed by the model directly. |
| Store | `trace.rs`, `records.rs` | The exchange with its sources (the records of the facts the answer used, behind the sources button), the focus, long-term memory per the saving rules, and the trace: answer type and how it was read, dossier size, passes, offenders, fallback, cached tokens, stage times. |

Fixed text that stays code (spec section 4.1): refusals (licence, payment, team password, actions the person may not do),
errors, "the model is not ready", progress and Cancel.

## 2. Engine

`crates/openframe-ai/`: llama.cpp b11259, Gemma 3 1B, one slot, `cache_prompt` on every call, a 512 MB prompt store.
Warm-up on project open builds the index and embeds the example messages in the background. Progress lines only, no token
streaming (owner decision 2026-10-24).

## 3. How it is measured

`crates/openframe-ai-eval`: `ai-eval` on the gold sets (`--only lt-` for the Low Tide fixtures), `--turn-only` for the
answer type, `--dossier-only` for fact recall, and a latency section on every run. Results and bars: spec sections 7.5 and
13, and the evidence folders `docs/release/evidence/ai-conversational-*-2026-10-26/`.
