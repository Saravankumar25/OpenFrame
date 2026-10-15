# Handoff: where we are right now

> Rewritten (not appended) at the end of every session. One page. It exists so the next session starts from the facts
> instead of from a long history, which is how context rot is avoided. If this page is older than the newest progress log
> entry, trust the log and fix this page first. Last rewritten: 2026-10-15 (documentation restructure).

## State in six lines

1. **Decisions of 2026-10-15 are recorded** in [ADR-0027](../adr/ADR-0027-rust-core-two-ui-shells.md) (one Rust core, Tauri
   on Windows, SwiftUI on the Mac), [ADR-0028](../adr/ADR-0028-ai-quality-program-and-harness.md) (AI quality program),
   [ADR-0029](../adr/ADR-0029-gpu-acceleration.md) (GPU) and [ADR-0030](../adr/ADR-0030-verification-ladder-and-release-flow.md)
   (proof levels, release flow, team gate minimum 8). They sit with ADR-0026 (models download after install).
2. **The plan is Chapter 1 in two tracks**: Windows phases W1 to W10 ([plan](../roadmap/chapter-1a-windows-phases.md)), then
   Mac phases M1 to M10 ([plan](../roadmap/chapter-1b-mac-phases.md)). Chapters 2 (website) and 3 (stores) get their
   implementation plans only after Chapter 1 and Chapter 2 close.
3. **This session changed documents only.** No source code changed. The code still enforces a team password minimum of 12,
   bundles the models, builds only the processor engine and has no evaluation harness.
4. No installer has ever been built. The owner's word is needed to build one, and now also a release executable.
5. The work is on branch `docs/restructure-three-chapters-2026-10-15`, committed locally, not merged, not pushed. A baseline
   commit holds the earlier 2026-10-15 document changes.
6. The only chapter in progress is Chapter 1, Windows track, phase W1.

## Next items (Windows phase W1, then W2)

1. **W1 controls**: proof-level column in the feature status file and its checker; team password minimum 12 to 8 in
   `crates/openframe-entitlement/src/team_gate/build_hash.rs` with tests; secret scan, link checker with an app-address
   allow-list; remove `reqwest` from the Windows shell build; move `docs/spec-digest/`, `AGENT_BRIEF.md`,
   `SECURITY_REVIEW_2026-09-30.md` and documents 28, 29, 31 into the archive with the script and test-comment updates; a
   line-level sweep of the build plan, launch register and long specifications for old statements.
2. **W1 hygiene**: lower the development profile debug level, merge integration test programs, review the 39 dead worktree
   folders and 31 unmerged agent branches (owner approves the table first).
3. **W2**: write the failing "type, close within 100 ms, reopen" end-to-end step, then the close-request flush with a
   shared pending-save registry; cloud-synced folder warning; documents-folder-unknown refusal; delete and locate tests.

## Blocked on the owner

- The macOS version of the Intel MacBook (Mac phase M1).
- Approval of the wording that replaces "filmmakers"; approval of the worktree review.
- The meaning of "build the website manually" (Chapter 2 plan).
- Later: the model host and production manifest key (W8), the borrowed laptops and virtual machine (W7, W9), the Apple
  silicon Mac (M8, M9), the team for the pilots, and the word to build.

## Do not

- Do not push, build an installer or a release executable, or delete worktrees without the owner's word in the conversation.
- Do not change the AI model or architecture before the baseline from phase W5 exists. No runtime model may judge an AI
  answer; the evaluator is deterministic code (ADR-0028).
- Do not rewrite Rust logic in Swift. The Mac shell is a screen layer over the core (ADR-0027).
- Do not write Chapter 2 or Chapter 3 code, or their phase plans, before the chapter before them closes.
- Do not read the whole progress log or a whole specification to get oriented. Read this page, the phase page, then the one
  work item; use `graphify query` and `grep` for the rest.
- Do not trust a feature row without reading the code it names; every "Verified" row is proof level 2 until W1 relabels it.
- Do not edit `docs/archive/`, `docs/audits/` or `docs/release/evidence/`. The team password is never written to a file.

## Facts a new session cannot see from the code

- The owner is a beginner in Rust, Tauri and Claude Code: explain simply. Their Windows machine is Windows 11 Home (no
  Sandbox or Hyper-V): clean-machine tests use a virtual machine tool and borrowed laptops.
- Hardware: 15.4 GB memory, 18 logical cores, Intel Core Ultra 5 with integrated graphics; an Intel MacBook (macOS version
  unknown); an Apple silicon Mac is coming; borrowed laptops with other graphics cards are coming.
- Benchmark of 2026-09-30 (processor, noisy): Gemma 3 1B Q8_0 gave 38.9 percent correct tool and arguments with about 30
  tools offered, 100 percent valid JSON by grammar. The release bar is 90 and 85 percent after the harness work.
- Target prices: 3,000 rupees, 89 dollars, 89 euros (not final).
- The full audit is [reports/2026-10-09-project-retrospection-and-audit.md](../reports/2026-10-09-project-retrospection-and-audit.md),
  a snapshot: re-check a finding in the code before acting on it.

## Not done in the 2026-10-15 restructure

The documents listed in W1 that scripts cite were not moved; the build plan (658 lines), launch register (1,528 lines) and the
long specifications were not read line by line (they carry "Current decisions" notes instead); the root `README.md` status
table and `development-storage-policy.md` were not reviewed for the new flow (the Mac sentence of `DEVELOPMENT.md`, the Mac
test section and a note in the internal test plan were fixed). The link, writing, feature-status and plan checks passed
after the edits: 130 documents, no broken link; the plan counts are unchanged.
