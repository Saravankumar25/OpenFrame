# ADR-0035: AI lanes, code-built briefings and code verifiers

- **Status:** Accepted 2026-10-24 (owner, in chat, answers to the questions in section 15 of the harness plan; recorded in
  [owner-decisions.md](../implementation/owner-decisions.md)). Built on the branch `upgrade/harness-exports-screenplay`.
  Measured results are not in yet: the held-out and conversation benches exist (`crates/openframe-ai-eval`), the first
  full run on the new lanes is still to be made. Nothing below is claimed as faster or more accurate until that run is saved.
- **Date:** 2026-10-24
- **Amends:** [ADR-0013](ADR-0013-single-local-model-hybrid-rag-agent.md) decision 6 (the agent loop, its step count and
  its place among other paths) and [ADR-0028](ADR-0028-ai-quality-program-and-harness.md) decision 8 (the one place
  conversation text may be saved). Decisions 1 and 5 of ADR-0028 keep their rules and are extended here.
- **Plan and evidence:** [ai-harness-rebuild.md](../specs/ai-harness-rebuild.md) (sections 5 and 11). How a request runs
  today: [ai-answer-paths.md](../engineering/ai-answer-paths.md). Number note: 0033 and 0034 are taken in the Mac tree
  ([register note](README.md)), so this ADR uses 0035.

## Context

The assistant used one path for everything: the local Gemma 3 1B model chose a tool in a loop of up to 8 steps. That was
safe and good for exact facts, and weak for the questions people ask first: "Explain Scene 5", "What is the
relationship between Ravi and Maya", "What is this story about". The model had to pick tools, read their output, and
write, with a prompt that changed from the first token. A misspelt word or a reworded question missed the code routes and
fell to the model, which chose `retrieve_context` for almost everything.

The baseline of 2026-10-24 (tool, held-out paraphrase and conversation runs with the routes on and off) showed the
failures were routing and evidence, not the model's wording. The rule of [CLAUDE.md](../../CLAUDE.md) holds: measure
first, only deterministic code judges answers while people use the app, three passes at most.

## Decision

1. **Code decides the lane. The model never chooses it.** Five lanes (`modules/ai/lanes.rs`):
   - **Direct**: an exact fact or navigation that names its tool. No model call for the answer.
   - **Action**: a clear change verb. The proposal tools prepare a Change Set; a person applies it (ADR-0013 decision 7, unchanged).
   - **Recipe**: a question about the whole screenplay or about each act.
   - **Chat**: a question about named people, scenes or acts, a scene in view, ideas, themes or critique.
   - **Agent**: lists, counts, finds, status, product questions and anything unclear.
   The lane, the next best lane and the strength of the rule (0 to 1) go to the trace (migration `0005_ai_trace_lane.sql`).
2. **Typo-tolerant routing, on a copy.** `typo.rs` makes a routing copy of the request in which a near miss of exactly
   one route keyword or project name is corrected (4 to 7 letters within one edit, 8 or more within two, same first
   letter, a fixed list for common misspellings). A keyword, a common English word, a number, quoted text and another
   script are never changed. The model still reads, and the database still stores, the original text.
3. **Intent prototypes with the search model.** Where the rules are not sure (the Agent lane, or Chat on a weak rule),
   `intent.rs` compares the routing copy with about thirty intents, each with short example questions in English,
   Hindi, Spanish, French, German and Japanese (`intents.json`). The embedder is the app's search model (E5 takes the
   "query: " prefix), so no new model is added. The match is trusted (direct gate) when the best example scores at least 0.90 and leads the
   best intent that asks for a different action by at least 0.03 (0.03 more for a whole-screenplay recipe). From 0.88
   up, or with a smaller lead, the middle band (2026-10-25) offers the first agent step only the tools of the two
   closest intents, `clarify` and `final_answer`, never `retrieve_context`, so the model chooses between two plausible
   tools under the grammar. A request waits
   at most 1.5 s for this step; when the search model is missing, slow or fails, the rule router decides as before. The
   closest intent, its score, its margin and whether it was used go to the trace (migration `0006_ai_trace_intent.sql`); the middle band writes `|top-two` after the
   intent name, so the gate (direct, top-two, none) can be read from the same columns.
   The thresholds are starting values chosen with a stand-in embedder; they are to be tuned on the bench from the trace.
4. **The Chat lane: code researches, the model writes.** `understand.rs` finds the people, places, scenes and acts a
   question names and how deep the answer should go. `briefing.rs` builds labelled plain-text briefings from the draft on
   each request: a project map, and a briefing for each scene, character, relationship or act. Every line that states a
   fact carries its scene number. A tie such as "sister" appears only from a Character Relationship record; pronouns and
   gender are never inferred. `chat.rs` sizes the evidence from the engine's measured prompt speed (about 15 seconds of
   reading at 60 percent, a fixed 1,240 tokens when nothing has been measured), checks the evidence is enough, and
   answers without a model call when the scene or name does not exist. Settings by depth: quick 120 tokens, explain
   350, deep 700, ideas 400; the temperature is set per depth (lower for facts, higher for ideas).
5. **The Recipe lane for whole-script questions.** `recipe.rs` plans the scenes by act (or groups of about eight), asks
   the model for one short note per group, composes one answer, and counts coverage (scene numbers cited against scenes
   planned). A time cap of 150 s applies, with 40 s kept for the composing call; on cancel or on the cap the answer is
   written from the notes already made and says how much was covered.
6. **Deterministic verifiers, up to three passes in 60 seconds.** `verify.rs` checks the finished text in code: names,
   scene numbers, quotations, numbers, scene counts, relationship words, mood the script does not state, language,
   length, repetition, empty text and false change claims. An answer that passes on pass 1 is shown at once. Otherwise the
   model gets one message that names the exact offenders and writes again; never more than three passes, all inside the
   60 seconds the request already has. If every pass fails, the attempt with the fewest offenders is shown with the
   offending sentences removed and marked not sure. This extends ADR-0028 decision 5 and keeps its rule: no model judges.
7. **The Agent lane is shorter and cheaper.** The default is 4 steps (was 8); the hard cap stays 12. The prompt shows a
   compact signature for each tool instead of its full JSON schema (at most 60 percent of the old size, tested); the
   selection budget of 7,000 characters is still counted on the full schema, so the same tools are offered as before.
   The prompt starts with text that does not change between requests (policy and project map), so the engine can reuse
   its stored work; the prompt tokens served from that cache are recorded in the trace (migration `0004_ai_trace_cache.sql`).
8. **Progress and Cancel, no streaming.** While a request runs the panel shows the latest progress line
   ("Reading Scenes 5 and 6", from `ai.progress` events) and a Cancel button that calls `ai.cancel`. The request checks the
   stop flag before each step, between verifier passes and before every Recipe call. No answer text appears before the
   verifiers have passed it, so nothing is shown and then replaced. A cancelled request is a column of the trace
   (migration `0006`).
9. **Saving a conversation, only by the person's own action.** `ai.export_conversation` writes the requesting user's own
   exchanges of one conversation to a file chosen in the Save dialog. It runs only on that press (ADR-0028 decision 8,
   amended).
10. **Answer shape.** Explain, deep and ideas answers of the Chat and Recipe lanes keep line breaks, "- " lists and "1. "
    lists; a heading hash is removed. Quick answers and the Agent lane stay flat text (owner decision 2026-10-24).

## Consequences

- Exact facts stay database work; questions about meaning get evidence built in code, so the 1B model writes from a short,
  checked briefing and does not plan.
- The request path has more branches, so the trace carries the lane, the runner-up, the intent figures and the cached
  token count. They are numbers and short codes only (ADR-0028 decision 8, ADR-0021).
- Briefings are computed on demand from the draft, so they cannot go stale. The test
  `a_200_scene_draft_builds_every_briefing_quickly` (`briefing/tests.rs`) fails when any briefing of a 200-scene draft
  takes 500 ms or more. No timing figure from a run is recorded in this document.
- The speed of the first token is unchanged on a processor. The gain claimed is that fewer wrong routes reach the model.
  It is measured by the baseline benches, not stated here.
- Mac: the same Rust core carries all of this; the SwiftUI side needs the progress line, Cancel, the Save conversation
  button and the answer text style ([mac port list](../roadmap/mac-port-list-2026-10-24.md)).

## Not adopted, and why

| Idea of the plan | Status | Why |
|---|---|---|
| A reranker model | Not adopted | It needs a fourth pinned model (ADR-0032) and an owner decision. ADR-0013 stays: no reranker |
| AI digest lines (a model writes a one-line digest of each scene when idle) | Not built | Opt-in and needs the owner's answer on idle work; briefings use code-built digests only |
| Token streaming ("stream, then replace if a check fails") | Not adopted | The owner chose progress lines only |
| A persistent digest store (a table of content-hashed digests) | Not built | Briefings are computed on demand and the test above bounds their time, so there is nothing to keep fresh |
| Speculative decoding and SYCL or other engine benchmarks | Not run | No measurement exists; no claim is made |
| Draft models, a tuned tool-calling adapter | Not built | Kept as later options in the plan |
