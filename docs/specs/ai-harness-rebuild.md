# AI harness rebuild

A plan to make the OpenFrame assistant answer like a strong conversational AI and work like a careful agent, with the same local Gemma 3 1B model.

- **Status:** proposal from a brainstorm session on 2026-10-24, since built in part. The decisions it led to are [ADR-0035](../adr/ADR-0035-ai-lanes-briefings-and-code-verifiers.md); how a request runs now is [ai-answer-paths.md](../engineering/ai-answer-paths.md). Sections 2 and 11 describe the state before the work and are kept as the baseline. Not built: the reranker, AI digest lines, the digest store, token streaming and the engine benchmarks (ADR-0035, "Not adopted"). The owner kept this file in `docs/specs/` (decision 10 of section 15).
- **Scope:** the Windows harness first (`windows/crates/openframe-application/src/modules/ai/` and `windows/crates/openframe-ai/`), then the Mac copy (`mac/...`).
- **Model rule for this plan:** the chat model stays Gemma 3 1B Q8_0 (8,192-token window). The embedding model stays Multilingual E5 small. Everything runs offline.
- **Safety rule for this plan:** the assistant can only propose project changes. A person applies them. This does not change.

## How to read the evidence labels

| Label | Meaning |
|---|---|
| **Code** | Read in the repository source or docs on 2026-10-22 to 2026-10-24 |
| **Measured** | A number written down in the repository's own evidence files |
| **Source** | A claim from a paper, vendor page or blog. Linked in section 16 |
| **Estimate** | My arithmetic from measured numbers. Not run |
| **Unproven** | A design idea with no measurement yet |

---

## 1. Goal

### 1.1 What a person should feel

People will judge the assistant by talking to it, not by watching it act as an agent. The target experience is a frontier assistant attached to the project:

- "Explain what is happening in Scene 2" returns a short, clear paragraph.
- "Who is Karthik?" returns a real character description that uses the story.
- "What is the dynamic between Devi and Kiran in Scenes 5 and 6?" returns an answer that points at what each says and how it shifts.
- "Any ideas for this scene?" returns grounded suggestions, labelled as ideas.
- "Explain the whole screenplay" returns a structured walk through the acts, and says how many scenes it used.
- "Open Scene 3" and "Create a task" stay exact and deterministic.

### 1.2 What "smarter" means here

| Quality | Meaning |
|---|---|
| Knows the project | Answers use real scenes, characters and relationships, not guesses |
| Accurate | Names, scene numbers, counts and quotes are real |
| Complete | The answer covers what was asked, at the length that fits the question |
| Natural | Reads like a helpful colleague, not a database report |
| Honest | Says "I could not find that" and marks ideas as ideas |
| Safe | Never changes the project on its own |

### 1.3 Non-goals

- No change of the chat model or the embedding model in this plan.
- No cloud inference and no outside network use.
- No automatic application of AI changes.
- No model acting as judge while people use the app. Checks are ordinary code (ADR-0028 decision 5).
- No claim of frontier-model reasoning. See section 14.

---

## 2. Where the harness stands today

All items in this section are **Code** unless marked **Measured**.

### 2.1 Request path

| Step | Where | What happens |
|---|---|---|
| Ask | `Conversation.tsx` to `ai.ask` | Text, scope and conversation id go to Rust |
| Permission and scope | `orchestrator.rs` `ask()`, `scope.rs` | `UseAi` required. Scope resolved to a scene, draft, selection or project |
| History | `ask()` | Last 4 turns, 400 characters of each answer |
| Greeting and refusals | `commands.rs` `small_talk`, `not_the_assistants` | Canned replies, no model |
| Follow-ups | `commands.rs` `follow_up`, `just_the_number` | Regex rewrite of "And Ravi?" and "Just the number" |
| Direct routes | `commands.rs` `scoped_call`, `direct_call`, `recipe` | About 7 large regex recipes send clear questions to one tool, no model |
| Retrieval | `retrieval.rs` then `intelligence/*` | Always runs before the model for non-direct requests |
| Agent loop | `orchestrator.rs` `run_agent` | Up to 8 steps (12 hard). One grammar-constrained JSON call per step |
| Writing | `orchestrator.rs` `run_generation` | Summaries and descriptions with word caps |
| Checks | `orchestrator.rs` `verify_language` | Language, kept numbers, false change claims. Max 3 passes, 60 s |
| Result | `persist`, `records.rs` | Answer, sources, steps, trace row (numbers only) |

### 2.2 Limits that shape every answer

| Limit | Value | File |
|---|---|---|
| Request text | 2,000 chars | `orchestrator.rs` |
| Steps | 8 default, 12 hard | `orchestrator.rs` |
| Tool catalogue in prompt | 7,000 chars | `orchestrator.rs` |
| One tool observation | 1,500 chars. All together 6,000 | `orchestrator.rs` |
| Step output | 400 tokens | `orchestrator.rs` |
| Final answer text | 1,200 chars | `orchestrator.rs` |
| History | 4 turns, 400 chars of each answer | `orchestrator.rs` |
| Summary and profile words | 40 | `orchestrator.rs` |
| Short writing words | 30. Paragraph 90 | `orchestrator.rs` |
| Scope context | 6,000 chars. Per item 2,500 | `scope.rs` |
| Retrieved context | 6,000 chars. Per item 1,500. 8 items by default, 24 at most | `assembler.rs`, `retrieval.rs` |
| Graph related items | at most 6, 320 chars each | `assembler.rs` |
| Tool read limits | 80 items, 300 chars per item, 4,000 content chars | `toolbox.rs` |
| Scene read | 3,000 chars of screenplay lines, then "N more lines not shown" | `toolbox/reads.rs` |
| Paging arguments on read tools | none (no offset or cursor found) | `toolbox/*` |
| Temperature | 0.2 default, 0.3 for writing, 0 only in the test mode | `client.rs`, `orchestrator.rs`, `manager.rs` |
| Streaming | off (`"stream": false`) | `client.rs` |
| Markdown in answers | stripped by `plain_text()` | `orchestrator.rs` |

### 2.3 Retrieval

| Part | Today |
|---|---|
| Index | `cache/intelligence.sqlite`, derived, rebuildable |
| Document kinds | 27 (scenes, story acts, sequences, beats, cards, characters, notes, tasks, comments, catalog, locations, cast, crew, breakdown, shooting days, call sheets, sides, moodboards, storyboards, shots, files, and more) |
| Chunks | 1,000 to 1,400 chars, with a structural header up to 400 chars prepended, at most 24 per document |
| Keyword | FTS5 BM25 over the canonical `search_fts` table, tokenizer `unicode61 remove_diacritics 2`, OR query of English-stopword-filtered terms, prefix-matched, top 40 |
| Vector | sqlite-vec KNN over chunk embeddings, top 48. Model is Multilingual E5 small when pinned |
| Graph | Canonical and Derived edges, 5 seed entities, 1 to 2 hops |
| Fusion | Reciprocal Rank Fusion (k = 60) plus boosts: scope 0.02, exact name 0.016, graph 0.012, current draft 0.004 |
| Rerank | none |
| Context build | Each hit is rebuilt from canonical rows. One best chunk per hit |
| Fallback ladder | hybrid, then FTS plus SQL, then SQL plus scope |
| Tables with no document | about 35 user-data tables (for example `budget_line`, `shooting_schedule`, `schedule_strip`, `season`, `screenplay_revision`, `story_character_relationship`, `vault_folder`, `template`) |

### 2.4 Tools

- 496 registered operations (156 queries, 340 commands). 305 are exposed to the assistant, 191 are not.
- 256 tools: 63 read, compute or navigate, and 193 proposal tools.
- The model sees a request-specific short list, not all of them.
- Tool arguments are validated against closed schemas. Permissions are checked again at run time.

### 2.5 Speed (Measured, `docs/release/evidence/w5`)

Test machine: Intel Core Ultra 5 125H, 15.4 GB, Windows 11, mains power.

| Measure | CPU, 12 threads | Intel Arc GPU, all layers |
|---|---|---|
| Prompt reading | 111 to 138 tokens/s | 494 tokens/s |
| Writing | 26 to 30 tokens/s | 27 tokens/s |
| Request time (3 test prompts) | 27.2 s | 7.1 s |

Other measured facts:

- A typical call carries 2,851 prompt tokens (median) and returns 78 tokens. A request makes 2 model calls (median).
- 86% of model time is spent reading the prompt.
- The second step of a request reuses about 96% of its prompt. Between different questions only about 53% of the prompt text is shared.
- The progress log lists "first AI answer 42.5 s" as a risk.
- Answers are not streamed.
- JSON grammar costs about 3% of request time. Q4_K_M instead of Q8_0 saves about 8%.

### 2.6 Quality measurements

| Item | Result | Note |
|---|---|---|
| Gold set | 749 cases in 12 languages (164 retrieval, 160 tool, 41 safety, 60 writing, 60 poisoned retrieval in English, 24 per other language) | Drafted by Claude, approved by the owner |
| Official baseline (W5, 679 cases, 118.5 min) | English tool and arguments 40.7%, end state 25.3% | Bars are 90% and 85%. Not met |
| Later runs (430 English cases) | Tool and arguments 94.0 to 98.1%, end state 87.3 to 91.2%, median 136 ms | A 136 ms median means most cases never reached the model |
| Retrieval, E5 (252 cases) | recall at 5: 92.1% overall, 99.4% English, MRR 0.93 | French and Japanese 4 of 8 at 5 |
| Retrieval, BGE (same cases) | 80% | E5 chosen |
| Conversation quality | **not measured** | No test scores explanation, character study, relationship or suggestion quality |

### 2.7 Gaps this plan targets

1. No conversation path. Open questions go through a tool-choosing loop whose policy asks for short answers written only from tool results.
2. Evidence is small (about 1.5k tokens) and fragmentary (one chunk per hit).
3. No briefing: nothing computes who speaks to whom, or builds a scene or character summary for the model.
4. No way to continue a clipped read. The scene tool stops at 3,000 characters.
5. About 35 data tables are not reachable by retrieval, and only some are reachable by a specialised tool.
6. No whole-project path. Nothing can answer "explain the screenplay".
7. Creative and factual work share almost the same temperature.
8. No streaming. A 10 to 40 second silence is the first thing a person sees.
9. Quality is measured by tool accuracy, not by conversation. The high scores come largely from regex routes.
10. The trace stores numbers only, so nobody can read real failed conversations.
11. The Mac copy has drifted (`commands.rs` 442 changed lines, `orchestrator.rs` 114, `toolbox.rs` 87, `indexer.rs` 41).

---

## 3. What outside research says, and how I challenged it

Source quality: arXiv and Hugging Face pages did not open in this session. Most items come from search summaries. The Anthropic page and one GitHub issue were opened directly. Blog claims are opinion.

| Finding | Challenge | Decision |
|---|---|---|
| Coding agents mostly dropped embedding search for grep and just-in-time reading | Code has exact identifiers. Stories are conceptual. Sources agree embeddings still fit prose | Keep hybrid search. Borrow the browsing tools and per-task context |
| Skip RAG when the knowledge fits the window (about 200k tokens) | Our window is 8k | Not applicable. Compress the project instead |
| Contextual retrieval: LLM-written chunk context cut top-20 failures from 5.7% to 3.7%, with contextual BM25 to 2.9%, with reranking to 1.9% | We already prepend a deterministic header. Reranking gave the largest extra gain | Enrich the header from known data. Test a small cross-encoder reranker |
| Hierarchical summaries help narrative question answering | LLM merging is not reliably faithful. Small-model plans hallucinate | Build digests in code. Optional model one-liners are validated and labelled |
| Strict JSON output lowers reasoning ("format tax") | We use the grammar only to pick a tool | Keep the grammar for tool steps. Use free text in the chat lane |
| 1 to 3B models do single tool calls reliably, fail at planning | Matches our 40.7% model-path baseline | Fewer model-chosen steps. More code |
| Rule-based co-occurrence graphs have better recall. LLMs label relations better but invent ties | Same-scene presence is not interaction | Interaction from dialogue turns. Labels only from data or quoted lines |
| Extractive compression kept quality at 6 to 25% size and can return nothing when irrelevant (RECOMP) | Their compressors were trained | Sentence squeeze in code. Abstain when nothing is relevant. Test it |
| Long prompts degrade; effective length is often half the window or less | I had proposed 7k tokens of evidence on GPUs | Cap evidence at 3 to 4k tokens. Best passages first and last |
| Persisted KV cache cut re-prefill 27x on Apple Silicon (one preprint) | Gemma-style sliding-window layers have cache reuse problems in llama.cpp | Stable-prefix ordering first. Verify reuse with `cache_n`. Disk persistence later, only if verified |
| Speculative decoding gives tens of percent to 2x. N-gram lookup helps copy-style output and fails on synthesis | Prefill is 86% of our time | Test only for long writing |
| SYCL may beat Vulkan on Intel for prefill (one forum post, discrete card) | Not an integrated GPU, unverified | Run `llama-bench` on the test laptop |
| Read 50 to 100 real conversations and build the failure list by hand | The trace stores no text | Add user-initiated conversation export |
| LLM judges match humans about 80% of the time but have position, length and self biases | Our plan was "Claude judges, one in ten spot-checked" | Human labels first, then a calibrated pairwise judge |
| Story Bible tools feed structured notes to the model; reviews say the model still contradicts them | We have this data already, without manual upkeep | Same pattern. Add code checks |
| Source-grounded tools cite inline but still paraphrase and invent when combining sources | One source is a competitor blog | Per-claim scene citations, checked by code |
| HyDE adds 43 to 60% latency on 1 to 4B models (blog) | Weak evidence both ways | No HyDE. Rewrite queries by code |
| A full-screenplay benchmark (STAGE) showed a structured pipeline raising grounded accuracy from 62.5% to 94.2% on 5 scripts | Tiny pilot, different models | Supports "facts first, then generate". Optional extra test set |

---

## 4. Design principles

1. **Code researches, the model writes.** Anything a program can compute (counts, who speaks to whom, scene facts, names) is computed and handed over as a briefing.
2. **One job per model call.** Short, focused prompts suit a 1B model.
3. **Separate the lanes.** Exact actions, conversation, lookups and big jobs get different prompts, temperatures and budgets.
4. **Do not clip silently.** Any limit says what was left out and how to get it.
5. **Check every answer with code.** Names, scene numbers, quotes and numbers must exist.
6. **Measure before and after each change.** The first measurement is the current model path with the regex routes off.
7. **Spend tokens by hardware.** Slow machines get shorter evidence. Fast machines get more, up to a tested cap.
8. **Keep the safety model.** Propose-only changes, closed schemas, permission checks, untrusted-data delimiters.
9. **Derived data is rebuildable.** Digests and indexes live in the cache, never in the project file.
10. **One shared behavior on Windows and Mac.**

---

## 5. Target architecture

### 5.1 Overview

```text
ask()
  permission, scope, conversation state            (exists, extended)
    |
  Lane router (code)
    |-- Direct   exact facts and navigation         (exists)
    |-- Chat     conversation, explanation, ideas   (new)
    |-- Agent    lookups across the project         (exists, reshaped)
    |-- Recipe   staged big jobs                    (new)
    |-- Action   propose changes                    (exists)
    |
  Query understanding (code)                        (new)
    |
  Retrieval v2: keyword, vector, graph, direct reads
    -> fusion -> optional rerank -> sentence squeeze
    -> neighbours -> sufficiency check -> widen or abstain
    |
  Briefing builders: scene, character, relationship, act, project   (new)
    |
  Prompt assembly: stable prefix first, volatile last, hardware budget
    |
  Model call(s) via llama-server, streamed
    |
  Verifiers (code, up to 3 passes)
    |
  Answer with scene citations   or   Change Set for review
```

### 5.2 Lane router

Order of decisions, all deterministic:

1. Refusals and greetings (existing `not_the_assistants`, `small_talk`).
2. Direct (existing `scoped_call`, `direct_call`, `recipe`): counts, scene facts, dialogue facts, navigation, simple proposals. No model call.
3. Action: clear change verbs ("rename", "create", "delete", "schedule", "add") go to proposal tools (existing).
4. Recipe: phrases that name a whole range ("whole screenplay", "entire", "each act", "every scene", "overall", "from start to end") or compare drafts.
5. Intent similarity: embed the question with the E5 "query:" prefix and compare with stored prototype vectors, about 15 intents with 5 to 10 example questions each, in English plus the supported languages. Intents: explain a scene, describe a character, relationship, compare, theme, critique, suggest ideas, list or status, find, product help. A margin threshold decides. This also routes non-English questions, which the regexes miss.
6. Default: **Chat** when at least one entity or scene resolves. **Agent** when none resolves and the question asks for a list, status or search across the project.

Rules:

- The router writes its choice, the second-best choice and the margin into the trace.
- A Chat answer that finds nothing and has no resolved entity escalates once to Agent. Total time is capped by the existing 60 s verifier budget.
- Routing accuracy is a measured metric (section 9).

### 5.3 Query understanding (code)

| Step | Behavior |
|---|---|
| Entity resolution | Exact names (existing `names_exactly`), aliases (`catalog_alias`, character names), scene and act references ("Scene 5", "Act 2", "the opening") |
| Fuzzy names | Edit distance 1 for names of 5 or more characters, flagged "did you mean" in the trace |
| Multi-part split | "X and Y in scenes 5 and 6" becomes entity set {X, Y}, scene set {5, 6} |
| Follow-up | Entity set carried from conversation state (section 5.13). Pronouns resolve to the last entity only when exactly one candidate fits, otherwise the assistant asks which one. Gender is never inferred from a name |
| Depth | quick, explain or deep (section 5.8) |
| Language | script-based detection (existing language verifier) |
| Sub-queries | One per entity plus the raw question. No model rewrite and no HyDE |

### 5.4 Retrieval v2

Pipeline:

```text
question -> understanding -> sub-queries
  -> keyword (AI-only FTS with stemming)  ---+
  -> vector over chunks (E5)              ---+--> RRF + boosts
  -> vector over digest text (E5)         ---+
  -> graph (adds speaks_with edges)       ---+
  -> direct reads for resolved entities   ---+
  -> optional cross-encoder rerank (top 30 to 12)
  -> sentence squeeze
  -> neighbours
  -> sufficiency check -> widen once -> abstain
```

Details:

1. **Keyword.** Add a separate derived FTS table in `intelligence.sqlite` with `tokenize = 'porter unicode61 remove_diacritics 2'` for English projects, plus per-language stopword lists. **Do not change `search_fts`.** It also backs Global Search, and changing it would change that feature.
2. **Chunk headers.** Keep deterministic headers. Add act, sequence, characters present, location and a one-line summary of the previous scene (all known to code). Bump `BUILDER_VERSION` so indexes rebuild.
3. **Digest vectors.** Embed each scene digest text as a second collection. This lets "where does the hero feel betrayed" match scenes whose text never uses those words.
4. **Graph.** Add Derived `speaks_with` edges from the interaction table (section 6). Bounded to 1 to 2 hops as today.
5. **Direct reads.** For every resolved entity, read its briefing directly. This removes the dependence on ranking for named things.
6. **Rerank (optional, test first).** A small cross-encoder through `llama-server --reranking`. Candidate: bge-reranker-v2-m3 (about 0.6B parameters in community GGUF builds). Adopt only if the bench shows a clear gain at an acceptable CPU time. It would be a fourth pinned model, so it needs an ADR-0032 row and a licence check.
7. **Sentence squeeze.** Split the top chunks into sentences or screenplay lines. Score = 0.5 * E5 similarity + 0.3 * query-term coverage + 0.2 * entity presence. Keep the best until the evidence budget is full. Always keep the scene heading. Keep original order inside a scene. Unproven: sentence embedding cost on CPU must be measured. Fallback: lexical scoring only.
8. **Neighbours.** Add the lines before and after a kept passage when budget allows.
9. **Sufficiency check (code).** Evidence is sufficient when: every resolved entity appears, every requested scene appears, at least 60% of the significant query terms appear, and the top hit leads the fifth by a margin. If not, widen once (double K, neighbours on, read the entity briefing, include other drafts). If still not sufficient, say what was not found.
10. **Abstain.** Empty evidence gives "I could not find that in the project", never an invented answer.

Unchanged: privacy filters, canonical rebuild of every item, draft boundaries, private-note ownership.

### 5.5 Digest store and project map (derived, code-built)

**Scene digest**, one row per scene per draft, built by code:

- heading, location, time of day, scene number, act, sequence, beat, linked card
- the writer's synopsis and scene note when present
- characters present and speaking line counts
- first and last dialogue line, longest line of each main speaker, first and last action line
- linked counts (breakdown elements, shots, boards)
- previous and next scene one-liners (heading plus synopsis)

**Optional AI line** (later, behind a switch): one sentence of at most 25 words from the 1B model at temperature 0.

- Validation by code: every capitalised name appears in the scene or the project's names, length is in range, language matches. Failure drops the line.
- Stored with status "AI-written" and shown as such.
- Regenerated when the scene content hash changes.

**Project map**, about 500 tokens: title, draft, counts, acts and sequences with scene ranges, main characters with scene counts, locations, shooting days summary, open task count.

Rules:

- Keyed by content hash and `BUILDER_VERSION`. A changed scene rebuilds only its row. A version bump rebuilds all.
- Lives in `cache/intelligence.sqlite`. Deleted and rebuilt on damage, like the existing index.
- Code-only digests build in seconds. AI lines, if enabled, run only when the app is idle and on mains power, are pausable, and never block saving.

This reverses decision 4 of `ai-answer-paths.md` ("no global summary cache") in a narrow way. Digests are keyed by content, so they cannot go stale.

### 5.6 Briefing builders

A briefing is a labelled text block with stable ids, built by code from the database and graph. The model is told it is data.

| Briefing | Contents | Target size |
|---|---|---|
| Scene | Heading, act, sequence, beat, card, present and speaking characters, previous and next scene, key lines, synopsis, notes, linked location, cast, props | 400 to 700 tokens |
| Character | First and last scene, scenes per act, scene partners, key lines, typed relationships, cast, description from the Story section | 500 to 800 tokens |
| Relationship (A and B) | Shared scenes, exchanges per scene (A speaks then B speaks, and the reverse), who speaks first, first and last interaction, lines around the key exchange, typed relationship if any | 600 to 900 tokens |
| Act or sequence | Scene list with one-liners, main characters, turning scenes from the beats | 500 to 800 tokens |
| Project | The project map | 500 tokens |

Rules:

- A tie type ("sister", "rival") is stated only if it is a typed relationship or appears in quoted text. Otherwise the briefing says "no relationship recorded".
- Interaction is counted from adjacent speaker turns inside a scene. Same-scene presence alone is not interaction.
- Pronoun and gender data is never inferred.
- Every claim line carries a scene number so the model can cite it.

### 5.7 Prompt assembly and context budget

Layout (Chat lane):

```text
[system]  lane policy              stable per lane
          style card                stable
          project map               stable per project revision
[user]    conversation state        small
          briefing                  volatile
          evidence (squeezed)       volatile
          User request: <text>      volatile
```

Stable material comes first so the engine reuses its stored work. Cache hits are checked with `cache_n` in the server timings.

**Budget by hardware.** Use the prompt speed the card probe already measures:

```text
evidence_tokens = clamp(prompt_tokens_per_second * 15 s * 0.6, 1,200, 3,500)
```

Estimate: CPU at 138 tokens/s gives about 1,240 tokens of evidence. Arc at 494 tokens/s gives 3,500 (the cap). The cap stays at 3,500 until a bench shows that more helps.

Split of the evidence budget (starting point, tune on the bench): briefing 45%, squeezed passages 40%, conversation state 10%, spare 5%.

Placement: most relevant passage first, second most relevant last. Weaker material in the middle.

### 5.8 Generation settings by lane

| Lane | Output | Temperature | Other sampling | Grammar | Max tokens |
|---|---|---|---|---|---|
| Direct | tool result, template text | none | none | none | none |
| Chat, factual | free text | 0.3 to 0.5 | top_p 0.95 | off | by depth |
| Chat, explain | free text | 0.5 to 0.7 | top_p 0.95, top_k 64 | off | by depth |
| Chat, ideas | free text | 0.8 to 1.0 | top_p 0.95, top_k 64 | off | 400 |
| Agent step | JSON | 0 to 0.2 | none | on (existing schema) | 400 |
| Recipe notes | short text | 0.2 | none | off | 150 |
| Recipe compose | free text | 0.6 | top_p 0.95 | off | 700 |
| Action | JSON, proposal | 0 | none | on | existing |

Starting values only. The Gemma team's reported family default is temperature 1.0, top_k 64, top_p 0.95; it is not confirmed for the 1B model. The bench sets the final values. Repetition penalty stays off unless repetition shows up.

**Depth** is chosen by code from the wording:

| Depth | Triggers | Length |
|---|---|---|
| quick | "briefly", "in a line", yes or no, "who", "where" | 1 to 3 sentences, up to 120 tokens |
| explain | "explain", "describe", "tell me about", "what happens", default for open questions | 1 to 2 paragraphs, up to 350 tokens |
| deep | "in detail", "thoroughly", "analyse", comparisons | up to 700 tokens, headings allowed |

`plain_text()` stops stripping line breaks and simple lists in explain and deep modes. The UI must render them. This is an owner decision (section 15).

### 5.9 Verifiers (code only)

| Check | Rule |
|---|---|
| Names | Every capitalised or all-caps name in the answer is in the briefing, the evidence, the project's names, or the question. A small list handles sentence-start words. A name of two words or more also needs every word written with a capital somewhere in the evidence, or the words together ("paddy fields" and "Pedda Revu" do not make "Paddy Revu") |
| Scene numbers | Every "Scene N" exists in the draft and appears in the briefing or evidence |
| Quotes | Text in quotation marks appears in the evidence after normalising spaces and case |
| Numbers | Numbers in the answer appear in the evidence or tool results (extends `numbers_kept`) |
| Scene counts | "N scenes" in the answer is a count the evidence states ("The draft has 36 scenes") or the draft's real count. A scene number ("Scene 16") never justifies "16 scenes" |
| Atmosphere | A short list of mood phrases (bathed in, palpable, soft morning light, dimly lit, eerie, tension hangs, sense of impending, ominous, haunting) is allowed only when every content word of the phrase is in the evidence or the question |
| Relationship words | "sister", "husband", "rival" and similar appear in a typed relationship or in quoted evidence |
| Language | Existing language check |
| Length | Within the depth range and within an asked sentence count |
| Too short | An explanation of one scene under 40 words, with more than 8 evidence lines, is asked once for two short paragraphs covering the scene in order |
| Preamble | A first sentence that only announces the answer ("Okay, let's dive into scene 4.") is dropped in code when it carries no fact; the policy also says to start with the answer itself |
| Repetition | No repeated 4-gram runs |
| Change claims | Existing guard: never say a change was made |
| Empty | Not empty |

On failure the model gets the exact offenders as machine-written feedback and tries again. At most 3 passes and 60 s (existing ADR-0028 rule). After that the answer is shown marked "not sure" with one clarifying question. On a fast GPU, spare time may go to parallel candidates, not slower retries (existing rule).

### 5.10 Recipe engine

A recipe is a fixed procedure in code with model calls as small steps.

**Whole-screenplay summary** (36 scenes as the example):

1. **Plan (code).** List scenes in order. Group by act and sequence. Without them, groups of about 8 scenes.
2. **Group notes (model, cold).** For each group, the prompt holds that group's scene digests (about 80 tokens each) and the question. The model writes a note of at most 100 tokens that names scene numbers.
3. **Act notes (model, cold).** Combine group notes per act.
4. **Compose (model, warm).** Prompt holds the project map, act notes and the question. The model writes the answer.
5. **Coverage check (code).** The union of scene numbers in the notes must cover the plan. Missing scenes are added by a repair call or reported.
6. **Verify and stream.**

Estimate for 36 scenes on CPU with code-built digests: 5 group calls (about 1.3k prompt tokens, 9 s read, 4 s write each) plus the compose call (about 1.5k prompt, 450 tokens out) comes to about 1.5 minutes. On the Arc GPU, about 40 seconds. Not run.

Other recipes, built after the first works:

| Recipe | Units | Output |
|---|---|---|
| Act summary | scenes in the act | structured act walk-through |
| Character arc | scenes of the character, grouped by act | arc description with key lines |
| Continuity check | scene pairs in a range | list of candidate issues for the writer, marked as candidates |
| Production readiness | breakdown, schedule and budget tables through read tools | list of gaps with counts |

Behavior:

- The UI shows a checklist ("Reading Act 2 of 3") and a cancel button.
- State is kept in the per-step audit rows that already exist, so an interrupted recipe can resume.
- Recipes are read-only. A recipe that suggests changes ends in the normal Change Set review.

### 5.11 Agent lane and browse tools

The Agent lane stays for lookups the briefing cannot cover (lists, status, searches across the project). Changes:

- Step cap 3 to 4 (was 8, 12 hard).
- Offer four generic read tools plus a short, request-picked list of specialised tools. This replaces the 7,000-character catalogue.
- Keep grammar-constrained JSON, closed schemas, repeated-call detection and all permission checks.

Generic tools (proposed names):

| Tool | Arguments | Returns |
|---|---|---|
| `project_outline` | optional scope | counts, acts and sequences, drafts, main characters, locations, schedule summary, list of record kinds with counts |
| `list_records` | `kind`, filters, sort, `offset`, `limit` (at most 40) | rows with id, title, 2 to 3 summary fields, plus `total`, `offset`, `next_offset` |
| `read_record` | `kind`, `id` or reference, optional fields, `offset`, `max_chars` (at most 3,000) | requested fields, with a truncation flag and `next_offset` for any long field, plus provenance ids |
| `search_project` | `query`, optional kinds, `offset`, `limit` (at most 12) | hybrid hits with one-line snippets, ids and `next_offset` |

Rules:

- Built from the registry metadata that already drives the tool coverage document.
- Run as the requesting actor. The same owner filters apply to private notes. Contact fields stay behind the existing deterministic tools.
- Results are bounded. Anything cut off says so and gives the next offset.
- New test, in the style of `ai_tool_coverage`: every user-data table is either indexed, reachable through `read_record`, or listed as hidden with a written reason. The test fails when a new table appears without a decision.

### 5.12 Action lane

Unchanged in behavior: `propose_*` tools build a Change Set, a person reviews and applies it, an interrupted apply is undone (ADR-0019).

Additions:

- Batch review: group many related proposals into one reviewable list with a summary.
- The raised composite limit (today 6 parts) is a separate decision, not part of this plan.

### 5.13 Conversation state

A small structure kept per conversation:

```text
entities:       recent [(kind, id, label, turn)], at most 8
scope:          last resolved scope
last_lane:      Direct | Chat | Agent | Recipe | Action
result_refs:    ids of what the last answer used
pending:        open clarifying question, if any
```

- Written by code, never by the model.
- Replaces the "4 turns of 400 characters" text history for resolution. A short text history of the last 2 exchanges stays for tone.
- Old state never overrides current project data.

### 5.14 Runtime and speed

| Item | Plan |
|---|---|
| Servers | Chat and embedding servers as today. Optional rerank server only if adopted |
| Prefix reuse | Stable-prefix prompt order. Log `prompt_n` and `cache_n` per call. Goal: raise cross-question reuse above the measured 53% |
| Tool catalogue | Shrink from 7,000 chars to the generic tools plus a short list. Estimate: saves over a thousand prompt tokens per call. Verify by token counts |
| Backend | Run `llama-bench` for Vulkan and SYCL (and OpenVINO if available) on the test laptop |
| Quantisation | Keep Q8_0. Q4_K_M saved about 8% and is not worth a quality risk |
| Speculative decoding | Test n-gram lookup and a small draft model for recipe writing only |
| KV persistence to disk | Defer. Gemma-style sliding-window reuse has known problems in llama.cpp. Revisit only with `cache_n` evidence |
| Idle work | Digest AI lines and index refresh run when idle and on mains power |
| Memory | No new always-resident model unless a reranker is adopted |

### 5.15 Streaming and the interface

- The client switches to `stream: true` and reads server-sent chunks.
- Rust emits progress and text events to the window. The final `ai.ask` result still carries the full exchange, sources and trace.
- **Streaming and checking conflict.** Verifiers run on the finished text. Design: stream the text into the bubble marked "checking". If a check fails and the answer is regenerated, replace the text once and say so. Expected to be rare; the failure rate is a tracked number. Alternative: no token streaming for the first pass, only progress lines. This is an owner decision.
- Streaming does not shorten the wait for the first word. That wait is prompt reading: about 11 to 16 s on CPU for a 1.5 to 2k-token Chat prompt, and 3 to 4 s on the Arc GPU (estimates).
- Show a status line while waiting ("Reading Scenes 5 and 6").
- Windows: React and Tauri events. Mac: SwiftUI through a UniFFI callback or async stream. The harness logic stays in Rust.

### 5.16 Observability and privacy

- Keep the numbers-only trace. Add: lane, runner-up lane and margin, entities resolved, evidence tokens used, sufficiency result, verifier results by check, passes, streamed-then-replaced flag.
- **New, user-initiated:** a "Save this conversation for review" action that writes the exchange (text, sources, trace numbers) to a file the person chooses. Nothing is saved automatically and nothing leaves the computer. It follows the existing "Export AI report" pattern. It needs an ADR-0028 decision 8 amendment.
- The AI health card shows lane mix, median time to first word, verifier failure rate, and routing escalations.

---

## 6. Data model changes (all derived, in `cache/intelligence.sqlite`)

Sketches only.

| Table | Columns | Purpose |
|---|---|---|
| `scene_digest` | scene_id, draft_id, content_hash, builder_version, heading, location, time_of_day, act, sequence, beat, card_id, synopsis, notes, present_json, speaking_json, key_lines_json, linked_json, prev_line, next_line, ai_line, ai_line_status, built_at | Digest per scene |
| `scene_interaction` | scene_id, speaker_a, speaker_b, exchanges_ab, exchanges_ba, first_pos, last_pos | Who answers whom |
| `project_map` | project_id, revision, text, built_at | Cached project map |
| `ai_fts` | AI-only FTS5 table, tokenizer `porter unicode61 remove_diacritics 2` for English projects | Stemmed keyword search |
| `digest_vec` | vector rows for digest text | Meaning-level search |
| Graph | new Derived edge type `speaks_with` | Relationship traversal |
| `intent_prototype` (app resource, not per project) | intent, language, text, vector | Lane router |

Rules: content-hash invalidation, `BUILDER_VERSION` bump rebuilds, delete and rebuild on damage, never part of undo or activity, never needed to open a project.

No canonical (`project.sqlite`) migration is needed for the first stages. Conversation state fits the existing `ai_request` records or a small new column, decided at build time.

---

## 7. Interface changes

| Area | Change |
|---|---|
| `ai.ask` | Same entry point. Adds optional `depth` hint |
| Events | `ai.delta` (text chunk), `ai.progress` (step, total, label), `ai.cancel` |
| Result | Adds `lane`, `coverage` (for recipes: used X of Y scenes), `citations` (scene numbers and ids) |
| Conversation view | Renders line breaks and simple lists. Shows progress line and cancel. Shows citations as chips that open the scene |
| Settings | Optional switch for AI digest lines. Optional conversation export |
| Mac | Same events through the UniFFI layer |

---

## 8. Mac parity

Facts: the Mac app has its own copy of the AI crates. Its UI is SwiftUI (`apps/macos/Sources/OpenFrameUI/AI/`) over the same Rust through `openframe-ffi`. Drift today: `commands.rs` 442 changed lines, `orchestrator.rs` 114, `toolbox.rs` 87, `indexer.rs` 41, `knowledge.rs` 28, `tools.rs` 22.

Plan:

1. Build and measure on Windows first (Chapter 1 order).
2. Port each stage to the Mac copy as its own change, with the Windows behavior as the reference.
3. Add a drift check that compares the shared AI files and lists intended differences.
4. Run the same benches on both and compare capabilities, retrieved facts, verifier decisions and answer completeness, not exact wording.
5. Decide later whether to extract a shared harness crate. That is a larger change and needs its own decision.

---

## 9. Evaluation plan (measure first)

ADR-0028 says no AI change before a baseline. This plan keeps that rule and widens the baseline.

### 9.1 Model-path baseline

- Add a switch to the eval harness that turns off the regex direct routes.
- Run the English tool, action, safety and writing cases with routes on and off.
- Report both. The gap between them is the true measure of the model path.

### 9.2 Held-out paraphrases

- For every gold intent, write 3 to 5 rewordings, including typos, missing punctuation and different order, that the route authors have not seen.
- Include non-English paraphrases.
- Report routes-on accuracy on this set. It is the honest estimate of real-user accuracy.

### 9.3 Conversation bench

About 150 prompts on the four sample scripts.

| Dimension | Values |
|---|---|
| Question type | explain a scene, describe a character, relationship, compare scenes, theme or meaning, critique, suggest ideas, whole-project, follow-up chains |
| Persona wording | writer, director, producer, assistant director, script supervisor |
| Style | clean, messy (typos, fragments), 15 non-English |
| Gold evidence | scene and line references written by hand from the script |

Scoring:

1. **Code checks:** names exist, scene numbers exist, quotes found, length within range, language matches, nothing invented, correct abstention on unanswerable prompts.
2. **Rubric 1 to 5:** accurate to the script, complete, specific, natural, right length, no invention.
3. **Pairwise comparison** with frontier assistants as a reference, run on sample scripts only (never a real project). Swap positions, control for length. The frontier score shows question difficulty and is not the pass mark.
4. **Human labels** on at least 50 answers. A judge is used only after it agrees with the human labels on at least 80% of the pairs. Gold facts come from the script, not from any model's answer.

### 9.4 Long-form retrieval set

- 8 question types: paraphrase, multi-scene, counting, why and how, multi-hop (character, location, schedule), negation, follow-up, non-English.
- Gold evidence by scene and line.
- Metrics: evidence recall at 5 and 10, evidence precision after the squeeze, sufficiency-check accuracy, correct abstention.

### 9.5 Deterministic correctness tests

- Briefing builders: exact expected output on sample scripts (scene partners, exchange counts, first and last interaction).
- Digest builder: content-hash invalidation, rebuild on version bump.
- Verifiers: positive and negative cases, including the poisoned-retrieval cases.
- Browse tools: paging returns every row exactly once. Table coverage test.
- Router: accuracy and confusion matrix on the paraphrase set.

### 9.6 Tester error analysis

- Testers use the export action on conversations that felt wrong.
- The owner or a reviewer reads the first 50 to 100 and writes the failure list by hand. Fix the most upstream failure first.
- New gold cases come from real failures.

### 9.7 Speed and resource measures

Time to first word, total time, prompt tokens, cache hit rate, tokens per second, peak memory, by lane and by hardware (CPU and the Arc GPU).

### 9.8 Release bars

| Measure | Bar |
|---|---|
| English tool and arguments, routes on | 90% (existing) |
| English end state, routes on | 85% (existing) |
| Paraphrase set, routes on | set from the baseline, not lower than the existing bar by more than 10 points |
| Model path with routes off | report only until the baseline exists, then set by the owner |
| Invented names in conversation bench answers | at most 2% (proposal) |
| Wrong scene citations | at most 2% (proposal) |
| Answers within the asked length | at least 90% (proposal) |
| Correct abstention on unanswerable prompts | at least 90% (proposal) |
| Conversation bench, "acceptable or better" against reference | set from the baseline |

The owner decides the final numbers after seeing the baseline, as ADR-0028 decision 2 requires.

### 9.9 Optional external check

STAGE is an open full-screenplay benchmark (scene summarisation, question answering, knowledge graph). Check its licence and the copyright of its screenplays before any use. Development only.

---

## 10. Rollout: work items in order

Each item is one slice with its own verification. Do them one at a time and update the handoff after each.

1. **Baseline the model path and conversation quality.**
   - Touches: `openframe-ai-eval`, gold folder.
   - Work: route-off switch, held-out paraphrase set, conversation bench v0 (60 prompts), run on the current build.
   - Verify: report with numbers. No product change.
   - Exit: owner reads the report and approves bars.
2. **Conversation export and first error analysis.**
   - Touches: AI panel, ADR-0028 decision 8.
   - Verify: export writes only on user action. Read 50 conversations. Failure list written.
3. **Prompt diet and prefix order.**
   - Touches: `orchestrator.rs` prompt building, `toolbox.rs` catalogue.
   - Work: log `prompt_n` and `cache_n`, count tokens by section, reorder, shrink the catalogue.
   - Verify: speed and cache numbers before and after. Gold set unchanged or better.
4. **Chat lane v0: depth, temperature, policy.**
   - Touches: lane router (minimal), new Chat prompt, `plain_text`, depth detection.
   - Verify: conversation bench against the baseline. Safety cases still pass.
5. **Streaming.**
   - Touches: `client.rs`, `manager.rs`, events, React conversation view.
   - Verify: first-chunk time, replaced-text rate, cancel works, no hung state.
6. **Scene and character briefings plus interaction table.**
   - Touches: new briefing module, `scene_interaction`, graph edge.
   - Verify: deterministic tests on sample scripts. Conversation bench for scene and character prompts.
7. **Relationship and act briefings.**
   - Verify: relationship prompts on the bench. No relationship label without data.
8. **Verifiers v2.**
   - Touches: new verifier set and feedback messages.
   - Verify: positive and negative unit tests, poisoned cases, false-positive rate on bench answers.
9. **Query understanding and keyword upgrade.**
   - Touches: entity resolver, follow-up state, AI-only FTS with stemming, sufficiency check, neighbours.
   - Verify: long-form retrieval set. `search_fts` and Global Search untouched.
10. **Sentence squeeze and reranker bake-off.**
    - Verify: evidence precision and recall, CPU time per query. Adopt the reranker only if it clears the threshold. If adopted: ADR-0032 row, licence check, download size.
11. **Browse tools and table coverage test.**
    - Verify: paging correctness, permissions, private notes, table coverage test passes.
12. **Digest store, project map, whole-screenplay recipe.**
    - Verify: coverage counts, timing, content-hash invalidation, bench whole-project prompts.
13. **More recipes and optional AI digest lines.**
    - Verify: validation rejects invented names. Opt-in switch. Idle and mains-only behavior.
14. **Conversation state upgrade.**
    - Verify: follow-up chains on the bench, ambiguous pronoun asks which one.
15. **Speed experiments.**
    - Work: Vulkan versus SYCL benchmark, n-gram and draft-model speculation for recipes.
    - Verify: `llama-bench` and recipe timings. Keep only measured wins.
16. **Mac port and parity.**
    - Verify: same benches on both, drift check.
17. **Release gate.**
    - Verify: all benches against the bars. Owner reads 20 sample conversations in full.

Reasoning for the order: measure first (rule), then the cheap changes that likely help most (depth, temperature, prompt, streaming), then the biggest quality changes (briefings, verifiers, retrieval), then the larger pieces (browse tools, digests, recipes).

---

## 11. Rules and decisions to change

| Where | Current | Proposed | Why |
|---|---|---|---|
| ADR-0028 decision 1 | Baseline before AI changes | Keep. Add route-off baseline, held-out paraphrases and the conversation bench to the baseline | Current scores do not measure conversation |
| ADR-0028 decision 5 | Code verifiers, 3 passes, 60 s | Keep. Extend the verifier list (section 5.9) | Catch invented details |
| ADR-0028 decision 8 | Numbers only, no text | Add a user-initiated conversation export | Error analysis needs real text |
| ADR-0013 decision 6 | One agent, 8 steps (12 hard), 7,000-char catalogue | Agent lane 3 to 4 steps, generic tools plus short list. Other lanes defined | Small models fail at long plans |
| ADR-0013 reranking | No reranker | Allow a cross-encoder if it wins the bake-off | Reranking gave the largest extra gain in published tests |
| ADR-0032 | Three pinned models | Add a fourth only if adopted | Reranker |
| `ai-answer-paths.md` decision 4 | No summary cache | Allow code-built, content-hashed digests | Whole-project questions |
| `ai-answer-paths.md` decisions 5 and 6 | Plain text, short by default | By lane and depth | Thin answers |
| `POLICY` in `orchestrator.rs` | One policy: "short answer written only from those results" | One policy per lane. Chat may use general craft knowledge, labelled | No conversation lane today |
| Temperature constants | 0.2 and 0.3 | Per lane (section 5.8) | Creative work needs room |
| `CLAUDE.md` AI rules | Measure first. Deterministic code judges. 3 passes | Unchanged | They fit this plan |

New or amended ADRs: one for the lane architecture, one for the conversation export, one for the reranker only if adopted, one for the shared harness crate only if chosen.

Docs to regenerate or update when each item lands: `ai-answer-paths.md`, `ai-tool-coverage.md` (generated), `feature-status.md`, `handoff.md`, `progress-log.md`, `docs/README.md` map.

---

## 12. Risks

| Risk | Effect | Mitigation |
|---|---|---|
| Lane misrouting | Worse answer for that question | Margin threshold, one escalation, measured accuracy, paraphrase set |
| Verifier false positives | Good answers rejected or retried | Allow-lists, measured rate, retry budget, "not sure" fallback |
| Verifier misses | Invented detail shown | Poisoned and negative cases, tester failure list |
| Warm sampling adds inventions | More wrong names | Checks stay on, temperature tuned on the bench, factual mode kept cooler |
| Stale or wrong digests | Wrong summary | Content hash keys, version bump, AI lines validated and labelled, can be disabled |
| Added time | Slower on CPU | Hardware budget, stable prefix, fewer steps, honest progress and cancel |
| Streaming then replace | Text flickers | Measure rate, show "checking", or fall back to progress-only |
| Changing the FTS tokenizer | Breaks Global Search | Separate AI-only table, never touch `search_fts` |
| Regex and prototype upkeep | Maintenance load | Prototype list in a data file with tests |
| Mac drift grows | Two behaviors | Drift check, port every stage, parity benches |
| Privacy | Text saved by mistake | Export only on user action, local file, no automatic capture |
| Judge bias | Misleading scores | Human labels first, position swap, length control, gold facts from scripts |
| Reranker memory and size | Heavy on 8 GB laptops | Adopt only if the gain is clear. Q8 builds are about 0.6 GB |
| 1B ceiling | Plain prose, weak subtext | Set expectations (section 14). Larger tier remains an owner option under ADR-0028 |
| Scope creep | Long program | One work item at a time. Stop points after items 4, 8 and 12 |

---

## 13. Expected time budgets (Estimate, not run)

| Task | CPU (138 tokens/s read, 28 write) | Arc GPU (494 read, 27 write) |
|---|---|---|
| Direct answer | under 1 s | under 1 s |
| Chat, explain (1.5 to 2k prompt, 350 out) | first word 11 to 16 s, done in 25 to 30 s | first word 3 to 4 s, done in 16 s |
| Agent, 3 steps | 35 to 60 s | 12 to 20 s |
| Whole screenplay, 36 scenes | about 1.5 min | about 40 s |
| 1,500-word piece in 5 sections | 2 to 2.5 min | about 1 min |
| Code-built digests for 36 scenes | seconds | seconds |
| AI digest lines for 36 scenes (opt-in) | about 6 min background | about 2 min background |

These follow from the W5 measured speeds. They need to be replaced with measurements in item 3 and item 12.

---

## 14. What will and will not improve

Likely to improve (still to be measured):

- Accuracy and completeness of scene, character and relationship answers.
- Natural tone and the right length.
- Honest "not found" and labelled ideas.
- Whole-project questions, with coverage shown.
- Fewer dead ends when a read is clipped.
- Perceived responsiveness while text streams.

Will not become frontier-level:

- Deep subtext, symbolism and critical judgement.
- Original creative ideas. Expect grounded but sometimes generic suggestions.
- Long polished prose across many pages.
- Speed on a slow CPU. The first word still waits for prompt reading.

Not proven:

- That the whole design is "many times better". The size of the gain is unknown until item 1 gives the model-path baseline.

Optional levers outside this plan, each needing an owner decision:

- A larger model tier on 16 GB machines, which ADR-0028 already allows.
- A tuned tool-calling adapter. Research shows large gains for small tool-callers, but it changes the model.

---

## 15. Decisions needed from the owner

1. May explain and deep answers show line breaks and simple lists in the conversation view?
2. Is "stream, then replace if the check fails" acceptable, or should the first pass show progress only?
3. May testers save conversations to a file for review, only when they choose to?
4. May idle background work build digests, with optional AI lines, only on mains power?
5. Is a first word after 11 to 16 s on CPU acceptable for explain questions, given streaming and a status line?
6. Should ideas be labelled "idea" in the answer text?
7. May a small reranker model be tested, knowing it adds a fourth pinned model if adopted?
8. Do you want a tuned tool-calling adapter kept as a later option?
9. Mac: port stage by stage, or extract a shared harness crate first?
10. Where should this document live when you decide to keep it (`docs/specs/`, `docs/engineering/`, or an ADR)?

---

## 16. Sources and what was not verified

Opened directly:

- Anthropic, Contextual Retrieval: https://www.anthropic.com/news/contextual-retrieval
- llama.cpp issue 21468 (prompt cache reuse on Gemma-family models): https://github.com/ggml-org/llama.cpp/issues/21468

From search summaries (not opened):

- Tam et al., Let Me Speak Freely: https://arxiv.org/pdf/2408.02442
- RAPTOR: https://arxiv.org/abs/2401.18059v1
- Context-aware hierarchical merging: https://arxiv.org/pdf/2502.00977
- Plan-guided summarisation for small models: https://arxiv.org/pdf/2504.09071
- RECOMP: https://arxiv.org/pdf/2310.04408
- RULER: https://arxiv.org/abs/2404.06654v2
- Literary character networks: https://arxiv.org/pdf/2412.11560
- CREFT: https://arxiv.org/pdf/2505.24553
- STAGE: https://arxiv.org/pdf/2601.08510
- Agent memory below the prompt (persistent KV): https://arxiv.org/pdf/2603.04428
- Harness engineering, eleven coding agents: https://arxiv.org/pdf/2609.00006
- Small language models for agentic systems: https://arxiv.org/pdf/2510.03847
- TinyAgent: https://bair.berkeley.edu/blog/2024/05/29/tiny-agent/
- Hamel Husain, evals FAQ: https://hamel.dev/blog/posts/evals-faq/
- LLM as a judge overview: https://cameronrwolfe.substack.com/p/llm-as-a-judge
- Gemma 3 run guide (sampling values): https://docs.unsloth.ai/basics/tutorial-how-to-run-gemma-3-effectively
- llama.cpp speculative decoding docs: https://www.mintlify.com/ggml-org/llama.cpp/inference/speculative-decoding
- llama.cpp rerank endpoint guide: https://www.simplified.guide/llama-cpp/server-call-rerank-api
- NovelCrafter Codex: https://www.novelcrafter.com/features/codex
- Apple Foundation Models and tool calling: https://www.natashatherobot.com/p/ai-agents-apples-foundation-models-tool-calling

Not verified:

- No part of this plan has been run. All code facts come from reading; all timings come from the repository's W5 evidence or from my arithmetic.
- The "RAG Deserves an Index" paper and the Hugging Face cache blog could not be opened.
- Warm sampling values for the 1B model, the reranker benefit and CPU cost, sentence-embedding cost, the SYCL versus Vulkan result on the test laptop, and the quality of AI digest lines are all unproven.
- The mapping from tests that pass to real-user quality is not known. Items 1 and 2 exist to find out.
