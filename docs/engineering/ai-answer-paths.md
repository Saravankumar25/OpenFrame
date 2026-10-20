# AI answer paths: how a question becomes an answer (Windows, October 2026)

> Written 2026-10-20 for the owner's AI briefs (`OpenFrameAI.docx`, 19 October, and the context engineering brief,
> 20 October). It records the request path as built, the decisions taken, what is measured and what is not. Code is the
> source of truth; this page names the files.

## 1. The path of one request

| Step | Where | What happens |
|---|---|---|
| Ask | `apps/desktop/src/ai/Conversation.tsx` | The question, the chosen scope and the conversation id go to `ai.ask`. |
| Scope | `modules/ai/scope.rs` | The project, screenplay, draft and selection the question is about are resolved. |
| Small talk | `commands.rs` `small_talk` | "Hi" or "Thanks" get a friendly reply at once. No tool, no model. |
| Follow-up | `commands.rs` `follow_up`, `just_the_number` | "What about Scene 4?" or "And Ravi?" becomes the previous question about the new subject. "Just the number" returns the number of the previous answer. |
| Direct route | `commands.rs` `direct_call` | A question whose meaning is clear is sent straight to one tool: counts, scene facts, dialogue facts, a character description, navigation, simple proposals. |
| Tool choice | `toolbox.rs` `tools_for_request`, `orchestrator.rs` `run_agent` | Anything else goes to the local model with a short list of tools picked for the question. The product guide is offered only for questions about using OpenFrame. |
| Tools | `tools.rs`, `toolbox/*.rs` | Exact facts are computed from the project database. Text the model must write (a summary, a description) gets only the evidence it needs. |
| Writing | `orchestrator.rs` `run_generation` | The model writes from the evidence: a scene summary from the whole scene, a character description from that character's scenes. |
| Checks | `orchestrator.rs` | No reply may say a change was made unless a Change Set was applied. A non-English question gets an answer in its language, keeping every number. |
| Show | `Conversation.tsx` `ResultView` | Plain text first. Sources, steps and how sure the answer is wait behind "Details". Long lists show eight items and "Show all". Proposed changes keep their review card. |

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
| Changes | Rename Karthik to Ravi, create a task | A proposal card; nothing changes until the person applies it | 0 or more |

## 3. Decisions

1. **Exact facts are database work, not model work.** A 1B model miscounts and invents. Counts, scene facts and dialogue
   facts are computed; the model is used for summaries, descriptions and wording. This also makes them fast on CPU.
2. **Route by meaning words, not one phrasing.** Scene questions are recognised by "Scene N" plus meaning words
   (summarize, who speaks, last line, where), so paraphrases agree. Questions with change words go to proposals.
3. **Whole objects, not top snippets, for named-object questions.** A scene summary reads the scene; a character
   description reads that character's scenes. Retrieval snippets are still used for open discovery questions.
4. **No graph or global summary cache added.** The measured failures were routing and evidence selection, not missing
   relationships. A cached global summary would go stale on every edit.
5. **Plain answers, details on demand.** The badges, "From project data" lines and step counts moved behind "Details".
6. **Short by default, exact when asked.** A request that names a sentence count ("in two sentences", "a
   one-sentence card", "a logline") gets that many sentences. Without one, a summary or a character description keeps
   whole sentences up to about 40 words, other short writing up to about 30, a paragraph up to about 90. Sentences are
   dropped from the end, never cut in the middle. When the model writes an answer from retrieved passages, the passages
   are its sources, not a list under the answer.
7. **Notes are read, never obeyed.** "Summarise my project notes" goes straight to the notes reader, so text inside a
   note cannot become a request to the model.

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
  rest go to the model with a short list of tools. Wording that could mean two things stays with the model: "What is
  the Production Source?" (the term or the draft), "What changed yesterday?" (the activity list is not filtered by
  date) and "Is the shooting draft locked?" (the draft status answer does not yet state the lock first).
- "Does Ravi speak to Maya?", "When do they meet?", continuity checks and schedule-change history still go to the model
  with retrieved snippets; they are not computed.
- File contents: the assistant reads file names and notes, not the bodies of attached PDFs or documents.

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
