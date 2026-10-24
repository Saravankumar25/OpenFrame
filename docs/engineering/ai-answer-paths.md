# AI answer paths: how a question becomes an answer (Windows, October 2026)

> Written 2026-10-20 for the owner's AI briefs (`OpenFrameAI.docx`, 19 October, and the context engineering brief,
> 20 October). It records the request path as built, the decisions taken, what is measured and what is not. Code is the
> source of truth; this page names the files. Updated 2026-10-24 for the lanes, briefings, code verifiers, progress and
> Cancel: the decisions are [ADR-0035](../adr/ADR-0035-ai-lanes-briefings-and-code-verifiers.md), the plan and its
> baseline are [ai-harness-rebuild.md](../specs/ai-harness-rebuild.md).

## 1. The path of one request

| Step | Where | What happens |
|---|---|---|
| Ask | `apps/desktop/src/ai/Conversation.tsx` | The question, the chosen scope and the conversation id go to `ai.ask`. |
| Scope | `modules/ai/scope.rs` | The project, screenplay, draft and selection the question is about are resolved. |
| Small talk | `commands.rs` `small_talk` | "Hi" or "Thanks" get a friendly reply at once. No tool, no model. |
| Follow-up | `commands.rs` `follow_up`, `just_the_number` | "What about Scene 4?" or "And Ravi?" becomes the previous question about the new subject. "Just the number" returns the number of the previous answer. |
| Spelling | `typo.rs` `correct`, `orchestrator.rs` `run_agent` | A routing copy of the question is made before any route runs. A word that is a near miss of exactly one route keyword or project name (a character or place) is corrected: 4 to 7 letters within one edit, 8 or more within two, a swap of neighbours counts as one, same first letter, and a fixed list for the common misspellings of "scenes" and "many". A keyword, a common English word, an inflection of a keyword, a number, quoted text or another script is never changed, and a capitalised word near a name is left for the name check. The routes, the tool picker, the lane rules and the intent step read the copy. The model reads, and the record keeps, the person's own words. A route found only through the copy may read; it never builds a proposal. |
| Direct route | `commands.rs` `direct_call` | A question whose meaning is clear is sent straight to one tool: counts, scene facts, dialogue facts, a character description, navigation, simple proposals. |
| Lane router | `lanes.rs`, `understand.rs`, `orchestrator.rs` `run_agent` | Code decides who answers (harness plan 5.2, `docs/specs/ai-harness-rebuild.md`). Direct and Action run the tool loop. A question about named people, scenes or acts, ideas, themes or critique, or about a scene or name the project does not have, goes to the Chat lane. "Explain the whole screenplay" and similar go to the Recipe lane. Lists, finds, counts, production records and anything unclear go to the Agent lane (4 steps by default, 12 at most). The lane, the next best lane and the strength of the rule (0 to 1) are written to the trace. |
| Intent prototypes | `intent.rs`, `intents.json`, `orchestrator.rs` `apply_intent` | Plan 5.2 step 5. Only where the rules were not sure: the Agent lane, or the Chat lane on a weak rule; a plain command, an action verb, a hard rule and a question about a missing name keep their route. About thirty intents, each with short example questions in English and two each in Hindi, Spanish, French, German and Japanese, are embedded once per search model (the "query: " form for E5) and kept in memory. The routing copy is embedded; an intent's score is its best example, and the margin is the lead over the best intent that asks for a different action. A score of 0.88 and a margin of 0.03 are needed (a whole-screenplay recipe 0.03 more). A chat or recipe intent feeds the lane router. A tool intent becomes a direct call when code can fill every argument from what `understand` found (scene number, character names, a quoted or noun-phrase search term, the question for the product guide); otherwise the first agent step is offered only that tool, `clarify` and `final_answer`. The step waits at most 1.5 seconds; with no search model, a failed or slow one, it is skipped and the rules decide as before. The intent, its score, its margin and whether it was used are written to the trace (migration 0006). |
| Chat lane | `chat.rs`, `briefing.rs`, `verify.rs` | Code builds the evidence: the project map, a briefing for each person, scene or act named, and the best retrieval passages, sized by the measured prompt speed. A scene the draft lacks or a name the project does not know is answered without the model. Otherwise the model writes (quick, explain, deep or ideas settings) and the verifiers check the text. |
| Recipe lane | `recipe.rs` | Whole-screenplay and act questions: scenes grouped by act (or about eight), one short note per group, one composed answer, a coverage count (scenes used out of scenes planned), and progress lines between the calls. |
| Tool choice | `toolbox.rs` `tools_for_request`, `orchestrator.rs` `run_agent` | The Agent lane: the local model with a short list of tools picked for the question. The product guide is offered only for questions about using OpenFrame. |
| Tools | `tools.rs`, `toolbox/*.rs` | Exact facts are computed from the project database. Text the model must write (a summary, a description) gets only the evidence it needs. |
| Writing | `orchestrator.rs` `run_generation` | The model writes from the evidence: a scene summary from the whole scene, a character description from that character's scenes. |
| Checks | `orchestrator.rs`, `verify.rs` | No reply may say a change was made unless a Change Set was applied. A non-English question gets an answer in its language, keeping every number. Chat and Recipe answers also pass the verifiers v2: names, scene numbers, quotations, numbers, relationship words, language, length, repetition and empty text. A failed check sends the model the exact offenders; at most three passes inside 60 seconds; an answer that passes on the first pass is shown at once. If every pass fails, the best attempt is shown without its offending sentences and marked "not sure". |
| Progress and Cancel | `orchestrator.rs` `progress`, `cancel.rs`, `Conversation.tsx` `useProgress` | While a request runs the panel shows the latest progress line of its conversation ("Reading Scenes 5 and 6", from `ai.progress` module events) in a polite live region, with a Cancel button that calls `ai.cancel`. No text is streamed. `ai.ask` registers a stop flag for its conversation (the panel names the conversation before the first answer exists); `ai.cancel` sets it and takes no lock the request holds. The request looks at the flag before each step, between verifier passes and before every Recipe call; a model call already running finishes or times out, then the request stops. The result is "Stopped. Nothing was changed." with whatever the steps had found, the request is stored as Canceled, the trace outcome is "cancelled", and no request stays Running. |
| Show | `Conversation.tsx` `ResultView`, `answerText.tsx` | Plain text first, drawn with paragraphs on blank lines, "- " bullets, "1. " numbers and the "Idea:" label in bold, all as React elements (no HTML). A whole-screenplay answer shows "Used 36 of 36 scenes" under the text. Sources, steps, how sure the answer is and the rows that only back the text wait behind the "See sources" button. A list the person asked for stays under the answer (`itemsAreAnswer`, set in Rust). Long lists show eight items and "Show all". Proposed changes keep their review card. |

## 2. Question families and how each is answered

| Family | Examples | Answered by | Model calls |
|---|---|---|---|
| Counts | How many scenes, characters, acts, drafts, omitted scenes, night scenes | Database count in the draft in scope | 0 |
| One scene | Who speaks in Scene 3, how many speak, where, day or night, first or last line, first dialogue, does it mention a car | `scene_fact`, reading that scene's own rows | 0 |
| Scene summary | Summarize Scene 3, what happens in Scene 3 | `scene_fact` summary: the whole scene (beginning and end kept for very long scenes, and the answer says so) | 1 |
| Dialogue | How many dialogues does Ravi have, in Scene 3, how many words, which scenes, first speaking scene, most dialogue, who speaks first | `dialogue_fact`, counting dialogue blocks (one cue and its dialogue; parentheticals belong to the block) | 0 |
| Character | Describe Karthik, who is Karthik | `character_profile`: every scene the character speaks in or is named in, sampled evenly across the story when more than the budget holds | 1 |
| Project facts | Which scene comes first, list the scene headings, how long is the screenplay, how many scenes are unscheduled, which draft is the production source, what open tasks do I have, are there unresolved comments | The read tool that holds the fact (scene facts, scene list, pages, schedule, production source, tasks, comments) | 0 |
| Search | Which scenes mention the station | Full-text search over the project; a bare category word ("scenes") answers with its count | 0 or 1 |
| Short writing | Write a logline, write a tagline, suggest a line for Bittu, rewrite Padma's last line, suggest a better title, what is the theme of this story, what is the point of this scene | `summarize_scope`: the content in scope, then a writing prompt that asks for only the requested text. A request that names a scene or act ("a logline for Scene 4") is left to the tools that read it | 1 |
| Product help | How do I export a PDF | The versioned product guide; "not covered" when no section matches | 1 |
| Product terms | What is a Change Set, what does the Production Source mean, what is the Parking Lot, the Beat Board, Sides, Day Out of Days | `answer_product_question`, chosen by wording when the term is one the guide names (`commands.rs` `GUIDE_TERMS`); the guide has a section for each | 1 |
| Notes | What is in my notes, what do my notes say, read my notes and do what they say | `project_notes`, read only: a note is shown, never obeyed | 0 |
| Others' private notes | Show me other people's private notes | `private_information` with `whose: someone_else`, which refuses without saying whether any exist | 0 |
| Changes | Rename Karthik to Ravi, create a task, add a shot to scene 21, make a moodboard for the station, add three desks as set dressing, comment on the draft, pin a note, create characters for this scene | A proposal card; nothing changes until the person applies it. Set dressing is catalogued as Props with the count in the notes (the catalog has no quantity); a comment on "the draft" targets the draft itself; "pin a note" creates the note already pinned | 0 or more |

## 3. Decisions

1. **Exact facts are database work, not model work.** A 1B model miscounts and invents. Counts, scene facts and dialogue
   facts are computed; the model is used for summaries, descriptions and wording. This also makes them fast on CPU.
2. **Route by meaning words, not one phrasing.** Scene questions are recognised by "Scene N" plus meaning words
   (summarize, who speaks, last line, where), so paraphrases agree. Questions with change words go to proposals.
3. **Whole objects, not top snippets, for named-object questions.** A scene summary reads the scene; a character
   description reads that character's scenes. Retrieval snippets are still used for open discovery questions.
4. **No graph or global summary cache added.** The measured failures were routing and evidence selection, not missing
   relationships. A cached global summary would go stale on every edit.
   **SUPERSEDED in part (2026-10-24)** by `docs/specs/ai-harness-rebuild.md` section 5.5: code-built digests, briefings
   and a project map are allowed because they are built from the draft on each request and so cannot go stale. As built
   in the lanes slice nothing is cached yet; the Chat and Recipe lanes read them fresh.
5. **Plain answers, details on demand.** The badges, "From project data" lines, step counts and the rows of a count, a
   scene fact or a written answer sit behind one "See sources" button. Whether rows are the answer is decided in Rust,
   not in the screen: `toolbox.rs` `LIST_ANSWERS` names the tools whose items are the list the person asked for (list the
   scenes, open tasks, search results, characters, drafts, notes, comments, suggestions). Every other tool leaves
   `items_are_answer` false, and `compose_reads` sets it only when no model text was written and a list read contributed rows.
   Refusals, failures, questions and private-information denials keep their reasons in view. Text the model wrote goes
   through `plain_text` (no bold, bullets, headings, numbering, rules, code ticks or link syntax; line breaks stay) before
   it is shown or stored.
   **SUPERSEDED in part (2026-10-24)** by `docs/specs/ai-harness-rebuild.md` section 5.8 and the owner decision of
   2026-10-24: explain, deep and ideas answers of the Chat and Recipe lanes keep line breaks, "- " bullets and "1. "
   numbers (`orchestrator.rs` `plain_text_with`, style Rich); a heading hash goes and the heading stays as a plain line.
   Quick answers and the Agent lane keep the flat style described here.
6. **Short by default, exact when asked.** A request that names a sentence count ("in two sentences", "a
   one-sentence card", "a logline") gets that many sentences. Without one, a summary or a character description keeps
   whole sentences up to about 40 words, other short writing up to about 30, a paragraph up to about 90. Sentences are
   dropped from the end, never cut in the middle. When the model writes an answer from retrieved passages, the passages
   are its sources, not a list under the answer.
   **SUPERSEDED in part (2026-10-24)** by `docs/specs/ai-harness-rebuild.md` sections 5.8 and 5.9 for the Chat and
   Recipe lanes: length follows the depth the question asks for (quick: 1 to 3 sentences, 120 tokens; explain: 1 to 2
   paragraphs, 350 tokens; deep: up to 700 tokens; ideas: 400 tokens), a named sentence count still wins, and an answer
   is trimmed to whole sentences before the length check can fail. The 40, 30 and 90 word defaults above still hold for
   the Direct and Agent lanes' generation tools.
7. **Notes are read, never obeyed.** "Summarise my project notes" goes straight to the notes reader, so text inside a
   note cannot become a request to the model.
8. **A create request needs no look-up, and a proposal ends the step.** In the 2026-10-20 real-model run, 9 of 75 action
   cases went on to search after the proposal was ready, and a request such as "Create a character for Kiran's father"
   searched before it proposed. The planner prompt now says a create or add request uses its propose_* tool at once and
   sets done on a single proposal. Not measured yet: this is an AI change and waits for the next evaluation run.

## 4. Measured

- English gold set on the real model (Gemma 3 1B Q8, CPU, 420 cases, before the scene and dialogue tools): correct tool
  and arguments 94.0 percent (bar 90), correct end state 87.3 percent (bar 85). Report: progress log 2026-10-20.
- Deterministic tests: `crates/openframe-application/tests/ai/ai_agent.rs` (scene facts, dialogue facts, follow-ups,
  character description, greetings, false-claim guard) and unit tests in `commands.rs`.

## 5. Not done, stated plainly

- The 100,000-document synthetic benchmark of the context brief was not built. The app's own data sizes (one project,
  hundreds of scenes) are what is measured.
- Of the 100 questions in section 6 of the context brief, 38 now go to a tool directly by their wording and two
  more ("And Ravi?", "Just give me the number") are answered from the conversation (router probe, 2026-10-20). The
  rest go to the model with a short list of tools. Wording that could mean two things stays with the model: "What
  changed yesterday?" (the activity list is not filtered by date) and "Is the shooting draft locked?" (the draft status
  answer does not yet state the lock first). "What is the Production Source?" is now the term and goes to the guide;
  "Which draft is the production source?" is the project fact and goes to its read tool.
- "Does Ravi speak to Maya?", "When do they meet?", continuity checks and schedule-change history still go to the model
  with retrieved snippets; they are not computed.
- File contents: the assistant reads file names and notes, not the bodies of attached PDFs or documents.

- Planned in the harness plan and not built (ADR-0035, "Not adopted"): a reranker, AI digest lines, a stored digest
  table, token streaming, speculative decoding and the engine benchmarks. The first full run of the held-out and
  conversation benches on the new lanes is not saved yet, so no gain is claimed here.
- The intent thresholds (score 0.88, margin 0.03) were chosen with a stand-in embedder and are to be tuned from the trace.

## 6. Owner walkthrough: try these ten questions

Open a project with an imported screenplay and the AI Assistant. Each answer should be one or two plain sentences.

1. "How many scenes are there?" The count of the current draft, with omitted scenes named in the same sentence.
2. "Who speaks in Scene 3?" The names of the speakers of that scene only.
3. "What about Scene 4?" The same question for Scene 4.
4. "Summarize Scene 3." One to three sentences about that scene, nothing from other scenes.
5. "How many dialogues does Ravi have?" Ravi's dialogue blocks in the draft (use a real character name).
6. "And Meera?" The same count for Meera.
7. "Just give me the number." Only the number.
8. "Describe Ravi." A short description from Ravi's own scenes.
9. "How do I export a PDF?" Help from the product guide.
10. "Rename Ravi to Raju." A proposal card; nothing changes until you press Apply.
