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

## Measuring

Measure first (ADR-0028 decision 1). The baseline and the new test set come before any runtime change and each slice is
measured before the next. The numbers are in `docs/release/evidence/ai-reharness/`. A missed bar is shown to the owner, never
lowered silently.

## Consequences

- Picking is a step the person must take. Users may forget; the Send hint is the only guard.
- A wrong pick costs one tap on the pointer's Add button. The false-pointer rate is tracked.
- Longer answers take longer on a processor. Progress lines only, no streaming.
