# ADR-0020: Offline AI runtime packaging per distribution channel

- **Status:** Accepted 2026-10-08 (owner, in chat: "AI engine inside each package"). Amends ADR-0006 for every
  release file. **Partly superseded 2026-10-08 by [ADR-0025](ADR-0025-offline-ai-bundled-in-every-package.md):** the
  weights, tokenizer and every runtime asset are bundled too, and nothing downloads. Decisions 1 (runtime part) and 4
  stay in force.
- **Date:** 2026-10-05, accepted 2026-10-08
- **Related:** ADR-0006, ADR-0013; `docs/engineering/27-microsoft-store.md` §§5.4, 10;
  `docs/engineering/23-release-migration.md` §7; `crates/openframe-ai/`

## Context

Offline AI downloads three things after install: model weights (Gemma 3 1B), embedding weights (BGE small) and the
`llama.cpp` runtime **executable**. Store policy allows user-consented add-ons that enhance disclosed
functionality and forbids undisclosed dynamic code. The supplied Store audit rates downloading and launching an
executable as the reviewer-sensitive part, unlike downloading weights.

## Decision

1. **Every package:** bundle the tested `llama.cpp` runtime (and the OpenMP runtime on Windows) **inside each of the
   four release files** of ADR-0024: the Microsoft Store `.msix`, the Mac App Store `.pkg`, the internal `.exe` and the
   internal `.dmg`. No release file downloads or launches an executable. ~~Download only model and embedding weights
   after an explicit user click. The signed manifest, SHA-256 and size checks stay.~~ **SUPERSEDED 2026-10-08 by
   ADR-0025:** the weights and tokenizer ship inside every package with pinned hashes; nothing is downloaded.
2. **Superseded 2026-10-08:** the proposed text let a direct channel keep downloading the runtime. There is no direct
   sales channel any more (ADR-0023), and the owner chose one rule for every package (decision 1).
3. ~~The listing and Notes for Certification disclose live generative AI, the optional on-demand weight download, that
   AI is optional, and the report-inappropriate-output path.~~ **SUPERSEDED 2026-10-08 by ADR-0025:** there is no
   download to disclose. Listings still disclose live generative AI, that AI is optional and the report path.
4. The in-app **Report AI output** action is required before Store submission: it opens the support channel with
   app version and optional user-approved context, and **never sends prompts or project content automatically**.
5. ~~The production manifest key and model host are set before release builds; the dev key is never trusted
   (`release-validate` already enforces this).~~ **SUPERSEDED 2026-10-08 by ADR-0025:** no model host and no download
   list; the manifest key items and their release check are removed with the download code.

## Consequences

- Installer size increases by the runtime. Measure and record it.
- Runtime updates ship through Store updates, so the runtime version is part of the app version.
- On the Mac the runtime sits inside the signed app bundle, which App Sandbox and notarization require anyway.
- ~~The model host and the production manifest key are still owner tasks (`owner-decisions.md`, manual work).~~
  Superseded 2026-10-08 by ADR-0025: neither is needed.
- Requires an AI-benchmark and `real_runtime` pass on the bundled build (build plan area: AI assistant quality and hardening).
