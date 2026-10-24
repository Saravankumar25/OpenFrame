# ADR-0028: AI quality program, harness, memory, languages and models

- **Status:** Accepted 2026-10-15 (owner, in chat, answers to the 25-question decision quiz).
- **Date:** 2026-10-15
- **Amends:** [ADR-0013](ADR-0013-single-local-model-hybrid-rag-agent.md) decision 1 (one model for everyone becomes two
  tiers, and the model family is chosen by measurement) and its retrieval and tool-choice design where a decision below
  differs. ADR-0013's human-only Change Sets, ADR-0019 and ADR-0021 are unchanged.
- **Extended 2026-10-24 by [ADR-0035](ADR-0035-ai-lanes-briefings-and-code-verifiers.md):** decisions 1 and 5 keep their rules (measure first; deterministic verifiers, three passes, 60 seconds). The verifier list grows, and the held-out paraphrase and conversation benches join the baseline. Decision 8 carries its amendment of the same day.
- **Related:** [ADR-0026](ADR-0026-offline-ai-download-after-install.md) (download after install),
  [ADR-0027](ADR-0027-rust-core-two-ui-shells.md) (provider interface),
  [ADR-0029](ADR-0029-gpu-acceleration.md), [ADR-0021](ADR-0021-diagnostics-telemetry-analytics-policy.md),
  [Windows phases 5 and 6](../roadmap/chapter-1a-windows-phases.md).

## Context

The AI assistant has a strong safety design (it can only propose, a person applies, an interrupted apply is undone) and
almost no quality measurement. The small model is weak at choosing tools, English-centred, and slow on a processor. The
owner wants the best output a small model can give, strong English first, other languages strong too, a harness that
learns from each user, and a loop that re-checks weak output up to three times.

Research summarised in the 2026-10-15 brainstorm: models under about 10 billion parameters do not reliably improve by
criticising their own answers; gains come from external checks such as tests and verifiers. So the checking in this ADR is
done by code, not by the small model.

## Decision

1. **Measure first.** An offline evaluation harness, a set of gold test cases per layer, and a baseline run come before any
   harness change. Layers: retrieval (recall at 5 and 10, reciprocal rank), whether the right tool was offered, tool choice
   and arguments, schema validity, end state of the project after a proposal is applied, grounding of names and numbers,
   language of the answer, safety and injection cases, writing quality, speed (first token, prompt and generation speed),
   memory and processor use, and context use against the window.
2. **Release bar for English.** At least 90 percent correct tool and arguments, and at least 85 percent correct end state,
   on the gold set, run on the release build. Writing quality is scored by a rubric and a human spot check. If the baseline
   shows the bar is out of reach for the chosen model, the owner is shown the numbers and decides; the bar is not lowered
   silently.
3. **Languages.** English is the first priority and must be the best. Also strong: Hindi, Telugu, Tamil, Kannada,
   Malayalam, Spanish, French, German, Chinese, Japanese and Korean. For these, retrieval and tool choice are held to the
   English bar; writing may be slightly plainer, and its rubric threshold is set from the baseline. The app screens stay
   English only; the AI answers in the language of the request.
4. **Models.** Two tiers: a small default and a larger tier offered on computers with 16 GB of memory or a suitable graphics
   card. The family is not fixed: any open model with a clean redistribution licence may be chosen if the evaluation shows
   it wins, with Gemma 3 as the incumbent. The chat model, the larger tier and the embedding model are chosen by a
   bake-off inside the evaluation harness. One multilingual embedding model is preferred over two. A change of model is a
   manifest version change ([ADR-0026](ADR-0026-offline-ai-download-after-install.md)) and a new row in the owner's
   redistribution check.
5. **The loop.** A task is run, checked by deterministic verifiers, and re-run with machine-written feedback if it fails,
   up to three passes in all and 60 seconds in total on a processor-only computer. Most tasks end on pass 1. Verifiers:
   schema validity, existence of every named object, a dry run of the proposal through the registry, grounding, language
   match, not empty or repeated, format and length. No model judges an answer at runtime, neither the
   one that wrote it nor any other: the checks are ordinary deterministic code. An answer that passes every check on the first
   pass is shown at once and no further pass runs. A later pass runs only after a failed check, and never more than three in all. On a graphics card the spare speed is spent on parallel candidates, not slower retries.
6. **When the loop fails.** After the last pass the person sees the best answer marked "not sure" and one clarifying
   question. The change-set approval still protects project data.
7. **Memory.** Four kinds: working (the current conversation and task), project (facts and style of one project, from its
   own data and "remember this"), episodic (past tasks and whether the person applied, rejected or edited the result, used
   as examples), and procedural (preferences, repeated commands, usage priors for choosing tools). Memory is per project.
   Sharing across projects happens only when the person turns it on. Learning is on by default and shown on a screen where
   the person can view, edit, delete and export everything learned. Memory is written only from the person's own actions and
   explicit "remember", never from text inside project files or imports. Memory is capped and decays. The small model never
   extracts memories.
8. **Observability.** A local table records, per AI task, numbers and ids only (tools offered and chosen, retrieval hits,
   tokens in and out, context fill, time split, passes used, verifier results, memory used, whether the person applied,
   rejected, edited or undid the result). No text is stored by default. The owner can press **Export AI report**, which
   writes a numbers file by hand. Nothing leaves the computer automatically ([ADR-0021](ADR-0021-diagnostics-telemetry-analytics-policy.md)).
   An "AI health" card in Settings shows the figures. The numbers include the prompt tokens the engine served from its stored
   work. Amendment 2026-10-24 (owner decision): a tester may press **Save conversation for review** in the AI panel, which
   writes the requests, answers, sources and trace numbers of that one conversation to a file the person chooses. It runs only
   on that press, saves nothing automatically and sends nothing; the file holds the requesting user's own exchanges only.
9. **Judging.** Open-ended writing in the offline tests is judged at development time by Claude, with a human spot check
   of about one case in ten. This runs only on the developer's computer and never ships. The runtime evaluator is permanent code in one crate shared with the offline harness, so development scoring and runtime checks agree. Claude's findings become code changes, new verifiers, new gold cases and tuned priors made by the developer; they never become a model call inside the application. The Hindi, Telugu and other
   non-English gold sets are reviewed by Claude only, by the owner's choice. Consequence: any claim about those languages
   says "measured by automated review" until a native speaker has checked a sample, and the pilot team is asked to report
   wrong-language or poor answers.
10. **Provider interface.** The AI engine is behind an `InferenceProvider` interface ([ADR-0027](ADR-0027-rust-core-two-ui-shells.md)).
    On the Mac, Apple's on-device model is an opt-in provider; the default stays the same open model as on Windows.
    The harness reads each provider's capabilities (for example the 4,096-token window of Apple's model and its language
    list) and adapts its prompt budget.

## Consequences

- No new AI feature is built until the baseline exists. The existing "do not change the model or the architecture without
  evidence" rule stays; this ADR defines the evidence.
- The tool picker based on English word overlap is replaced by similarity in the embedding space plus usage priors, and
  is judged by the "right tool offered" metric.
- Big jobs run as fixed recipes in code with one small model step at a time.
- Prompt reuse (a stable prompt start so the engine reuses its stored work) is part of the harness work and is checked
  with the speed metrics.
- Fine-tuning a model on the user's computer is not planned. "Self-improving" means the harness adapts to the user.
- The 90 and 85 percent bar is a release gate for Windows (phase 6 exit) and again for the Mac (Mac phase 8 exit).
