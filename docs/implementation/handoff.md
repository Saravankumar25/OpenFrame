# Handoff: where we are right now

> Rewritten (not appended) at the end of every session. One page. It exists so the next session starts from the facts
> instead of from a long history, which is how context rot is avoided. If this page is older than the newest progress log
> entry, trust the log and fix this page first. Last rewritten: 2026-10-15.

## State in five lines

1. **Decision changed 2026-10-15 (owner, in chat): the AI models download after install.** [ADR-0026](../adr/ADR-0026-offline-ai-download-after-install.md)
   supersedes ADR-0025. The app and the AI runtime ship in every package; Gemma 3 1B, BGE and the tokenizer download
   on the user's click from a signed manifest. This session changed documents only. No source code changed.
2. **The code still bundles the models.** The download flow was removed on 2026-10-08 and must be rebuilt. Until then the
   docs say "current build bundles the models; target downloads".
3. No installer has ever been built. The owner's word is needed to build one.
4. The working tree has uncommitted documentation changes from 2026-10-15 (about 30 files). The branch is `main`.
   Nothing was committed or pushed, because the owner did not ask for a commit.
5. The only chapter in progress is Chapter 1 ([plan](../roadmap/chapter-1-complete-test-validate.md)).

## Next items (Chapter 1, AI packaging, then the earlier stream)

1. **Owner tasks, now open again** ([owner-decisions.md](owner-decisions.md), section 2 and 3): choose and run the HTTPS
   model host; generate the production Ed25519 manifest key and keep the private key offline; confirm the Gemma
   redistribution terms for hosting the weights.
2. **Rebuild the Download OpenFrame AI flow** (masterplan area 8, "to be done" items). Start with the manifest signature
   check and the `*.partial` download with resume, then the disk-space preflight, the atomic activation and the
   quarantine. Add the tests named in ADR-0026 decision 10 before any UI work.
3. The earlier stream, still open: write the failing end-to-end step "type, close the window within 100 ms, reopen, the
   text is there", then add the close-request flush.

## Blocked on the owner

- The model host and the production manifest key (see item 1 above).
- The word to build the internal `.exe` (and later the `.dmg`).
- Team password for testers; final prices (3,000 or 2,999 rupees); wording replacing "filmmakers"; support address; domain.
- Whether the AI must serve Indian-language writers. The answer decides whether a vocabulary trim is worth trying.
- Review of the 39 dead worktree folders (4.2 GB) under `.claude/worktrees` and the 31 unmerged `worktree-agent-*`
  branches, before anything is removed.
- Whether to keep `claude-mem`, the GSD framework and the stale `AGENTS.md` (see CLAUDE.md, "Installed tooling").

## Do not

- Do not push, build an installer or delete worktrees without the owner's word in the conversation.
- Do not run `npm run build`, `tauri build` or any installer step while the model download flow is being rebuilt.
- Do not change the AI model or the AI architecture without evidence and approval. Quantized or vocabulary-trimmed models
  are candidates only; the 400-case evaluation suite must exist first.
- Do not read the whole progress log or a whole specification to get oriented. Read this page, the chapter page, then the
  one work item; use `graphify query` and `grep` for the rest.
- Do not trust a feature row without reading the code it names. Three rows were stale on 2026-10-09.
- Do not edit `docs/archive/`, `docs/audits/` or `docs/release/evidence/` for the 2026-10-15 change. Those are history.

## Facts a new session cannot see from the code

- The owner has both a Windows laptop and a Mac laptop and is a beginner in Rust, Tauri and Claude Code: explain simply.
- The machine has 15.4 GB of memory and 18 logical cores. Heavy builds go through `scripts/cargo-queued.mjs`.
- Target prices: 3,000 rupees, 89 dollars, 89 euros (not final).
- Measured sizes: release executable 18.2 MB (2026-09-30); AI files 1,149,732,477 bytes (25 files); the Gemma Q8_0 file
  1,069,306,368 bytes, gzip saves about 4.5%. A download-after-install package is an estimate of 20 to 30 MB until built.
- The full audit is [reports/2026-10-09-project-retrospection-and-audit.md](../reports/2026-10-09-project-retrospection-and-audit.md).
  It is a snapshot: re-check a finding in the code before acting on it.

## Audit coverage (continuation note)

Inspected by reading code: the Tauri shell entry file, the registry and dispatch, the mutation pipeline, migrations, the
persistence core, the project format, the AI orchestrator, change sets and toolbox selection, the AI bundle and engine
launch, the frontend IPC layer, query layer, routes and editor sync, packages restore, plus the build configuration, test
and CI layout, scripts, and every document list. Not inspected line by line: most of the 150,000-line application crate
(module bodies for schedule, call sheets, breakdown, story, exports), the import-export parsers, the entitlement and Mac
code, the 41,000 lines of screens, the SQL migration contents. The report lists this in its coverage appendix.
