# ADR-0040: Section-scoped assistant: the person picks where the assistant reads

- **Status:** Accepted 2026-11-03. The owner included the design in the release package, which file 00 says counts as the
  go-ahead.
- **Date:** 2026-11-03
- **Design:** [ai-section-scoped-assistant.md](../specs/ai-section-scoped-assistant.md) (the package's `ai-reharness.md`).
- **Supersedes:** [ADR-0037](ADR-0037-conversational-assistant-focus-and-dossiers.md) decision 5 (domain by topic words) and
  decision 6 (the Stage B plan becomes the picked sections; Stage C "one more fetch" goes, because the person widens by
  adding a section).
- **Amends:** ADR-0037 decision 9 for narrative answers.
- **Unchanged:** ADR-0028 decisions 1 and 5 (measure first; only deterministic code judges answers while people use the
  app), ADR-0013 decisions 6 and 7, ADR-0019, ADR-0025, ADR-0026. The model stays Gemma 3 1B.

## Decision

1. The person picks one to three leaves of a fixed section tree before asking. The assistant never decides where to look.
2. There is no pre-fill from the current screen and no "Whole Project". Send stays disabled until a leaf is picked.
3. Only the picked leaves are fetched and searched. Facts from typed queries are never cut. Text passages fill the rest,
   from the existing hybrid search filtered to the picked leaves, with full text inside the same leaves as the fallback.
4. When the question belongs to a section that is not picked, code (never the model) names the section to add. The
   pointer checks permission first and names a section, never content.
5. Narrative answers get their own types and limits (describe, brief, relationship, story explanation) and briefing
   dossiers in which every line carries its scene number. The writer states only what the facts show.
6. The agent's tool list is filtered to tools homed in the picked leaves. The agent loop, Change Sets and human-only apply
   do not change.
7. The hand-built leaf registry in Rust replaces scope kind, domain and retrieval route for questions. A coverage test
   fails the build when a screen or an indexed entity type maps to no leaf or to more than one.
8. Measure first: a saved baseline before any runtime change, then a run after each slice. The bars are not lowered
   silently; if one is missed, the numbers go to the owner.

## Consequences

- A question that needs four leaves is blocked by the cap of three. The trace records this so the cap can be reconsidered
  with data, and only with data.
- The Mac shell needs its own picker over the same Rust payload. That is Mac track work.
- Long answers cost more time on a processor; latency bars for the long classes are written down before their run.
