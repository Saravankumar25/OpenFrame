# Handoff: where we are right now

> Rewritten (not appended) at the end of every session. One page. It exists so the next session starts from the facts
> instead of from a long history, which is how context rot is avoided. If this page is older than the newest progress log
> entry, trust the log and fix this page first. Last rewritten: 2026-11-03, end of slice R7 of the section-scoped assistant.

## State

1. **Branch `release/trial-offline-and-store-readiness`** (local, not pushed). Chapter 3 started with the release package
   (`OpenFrame_Release_Package/`, untracked on purpose). The offline Mac trial, the store readiness work and the section-scoped
   assistant (ADR-0041, slices R0 to R7) are committed here.
2. **The assistant has one road.** The person picks up to 3 sections and only those are read. `AskArgs.scope`, the scope
   kinds, the domain table, the fetcher table, the wide passages round, the retrieval router and the project map are deleted.
   Project migration 0020 and app migration 0011 drop their columns. A request with no picks gets a fixed sentence; under
   `AppConfig.ai_eval_all_sections` (tests and evaluation only, false in the FFI) it reads every section through the same path.
3. **Measured with no model:** the pointer bench `crates/openframe-ai-eval/tests/pointer_gold.rs`. **Not measured:** none of this
   has run with the real model since R6. Results of the R6 run are in `release-out/eval/r6` (a run started before R7, on the older
   code: it does not measure this slice).

## Next items

1. **Full evaluation with the real model** on this build against the saved baseline (`docs/release/evidence/ai-reharness/`), then
   measure the false alarms of the relationship, mood and scene reference checks. The older gold sets now read every section, so
   their numbers will move: compare by family, show the owner, never lower a bar quietly.
2. **Windows:** copy the shared Rust and build the picker (`docs/release/windows-port-2026-11-03.md`).
3. **Items from before:** the full regular E2E on the Mac, the snapshot spinner in sides and reports, and the items that need Xcode and the
   developer account (`docs/release/mac-app-store-readiness.md`).

## Owner decisions open

- AI bars: read the real-model numbers when the run above is done.
- Identity in `library.json` is not authentication (doc 40 section 2): keep or change.

## Do not

- Do not push, merge to a remote or build a store package without the owner's word.
- Do not stage `OpenFrame_Release_Package/` or `release-out/`.
- Do not run two E2E runners or two SwiftPM builds at once; build outside the E2E lock.
- Do not leave build caches behind: clear `target/` after each finished build step (owner decision 2026-10-27).
