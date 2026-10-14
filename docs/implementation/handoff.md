# Handoff: where we are right now

> Rewritten (not appended) at the end of every session. One page. It exists so the next session starts from the facts
> instead of from a long history, which is how context rot is avoided. If this page is older than the newest progress log
> entry, trust the log and fix this page first. Last rewritten: 2026-10-09.

## State in five lines

1. Branch `docs/chapter-reset` holds the documentation restructure and the audit report (local commits only, not pushed).
   `main` is at `0acbd5e` with all of session 2 merged.
2. No installer has ever been built. The owner's word is needed to build one.
3. Features are largely built and proved on the development machine (65 Verified, 45 Reachable, 31 Built of 173 rows).
   Nothing is Release-ready. The Mac build has never run.
4. The only chapter in progress is Chapter 1 ([plan](../roadmap/chapter-1-complete-test-validate.md)).
5. Source code was not changed by the restructure.

## Next three items (Chapter 1, stream 1: protect writers' work)

1. Write the failing end-to-end step "type, close the window within 100 ms, reopen, the text is there", then add the
   close-request flush.
2. Write the two missing tests for `project.delete` and `project.locate`.
3. Add the cloud-synced folder warning (needs the owner's wording and default folder).

Before item 1, ask the owner for these decisions (see [owner-decisions.md](owner-decisions.md), "Still open"): team
password for testers, default project folder, product wording, macOS AI engine.

## Blocked on the owner

- The word to build the internal `.exe` (and later the `.dmg`).
- Team password for testers; final prices (3,000 or 2,999 rupees); wording replacing "filmmakers"; support address; domain.
- Review of the 39 dead worktree folders (4.2 GB) under `.claude/worktrees` and the 31 unmerged `worktree-agent-*`
  branches, before anything is removed.
- Whether to keep `claude-mem`, the GSD framework and the stale `AGENTS.md` (see CLAUDE.md, "Installed tooling").

## Do not

- Do not push, build an installer, or delete worktrees without the owner's word in the conversation.
- Do not start Chapter 2 or 3 work. Do not change the AI model or the AI architecture without evidence and approval.
- Do not read the whole progress log or a whole specification to get oriented. Read this page, the chapter page, then the
  one work item; use `graphify query` and `grep` for the rest.
- Do not trust a feature row without reading the code it names. Three rows were stale on 2026-10-09.
- Do not run E2E per branch. One full run after a set of areas is merged.

## Facts a new session cannot see from the code

- The owner has both a Windows laptop and a Mac laptop and is a beginner in Rust, Tauri and Claude Code: explain simply.
- The machine has 15.4 GB of memory and 18 logical cores. Heavy builds go through `scripts/cargo-queued.mjs`.
- Target prices: 3,000 rupees, 89 dollars, 89 euros (not final).
- The full audit is [reports/2026-10-09-project-retrospection-and-audit.md](../reports/2026-10-09-project-retrospection-and-audit.md).
  It is a snapshot: re-check a finding in the code before acting on it.

## Audit coverage (continuation note)

Inspected by reading code: the Tauri shell entry file, the registry and dispatch, the mutation pipeline, migrations, the
persistence core, the project format, the AI orchestrator, change sets and toolbox selection, the AI bundle and engine
launch, the frontend IPC layer, query layer, routes and editor sync, packages restore, plus the build configuration, test
and CI layout, scripts, and every document list. Not inspected line by line: most of the 150,000-line application crate
(module bodies for schedule, call sheets, breakdown, story, exports), the import-export parsers, the entitlement and Mac
code, the 41,000 lines of screens, the SQL migration contents. The report lists this in its coverage appendix.
