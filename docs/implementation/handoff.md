# Handoff: where we are right now

> Rewritten (not appended) at the end of every session. One page. It exists so the next session starts from the facts
> instead of from a long history, which is how context rot is avoided. If this page is older than the newest progress log
> entry, trust the log and fix this page first. Last rewritten: 2026-11-03, after tuning the section answers on the real model.

## State

1. **Branch `release/trial-offline-and-store-readiness`** (local, not pushed). Chapter 3 started with the release package
   (`OpenFrame_Release_Package/`, untracked on purpose). The offline Mac trial, the store readiness work and the section-scoped
   assistant (ADR-0041, slices R0 to R7) are committed here.
2. **The assistant has one road.** The person picks up to 3 sections and only those are read. `AskArgs.scope`, the scope
   kinds, the domain table, the fetcher table, the wide passages round, the retrieval router and the project map are deleted.
   Project migration 0020 and app migration 0011 drop their columns. A request with no picks gets a fixed sentence; under
   `AppConfig.ai_eval_all_sections` (tests and evaluation only, false in the FFI) it reads every section through the same path.
3. **Measured with the real model on this build:** the section set scores 324 of 336 answer facts, length 40 of 40, pointer
   precision 59 of 59 and recall 59 of 60, false pointers 0 of 333, median 2.3 s. `docs/release/evidence/ai-reharness/README.md` holds
   the runs and what still fails. **Not measured:** the older gold sets on this build, and the relationship and mood checks, which
   stay off.

## Next items

1. **The older gold sets on the real model** against the saved baseline, then the false alarms of the relationship and mood checks
   (the scene reference check showed none). The older sets now read every section, so their numbers will move: compare by family,
   show the owner, never lower a bar quietly. A person spot-checks one section case in ten.
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
