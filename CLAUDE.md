## What we are building

OpenFrame Studio is a local-first desktop app for writers, screenwriters and production teams. Everyone uses the same
application. Stack: one Rust core over SQLite; Tauri v2 with React and TypeScript on Windows; a native SwiftUI shell on the Mac through
UniFFI (ADR-0027); an offline AI run by llama.cpp behind a provider interface (Gemma 3 as incumbent, models downloaded
after install, ADR-0026 and ADR-0028). Windows 11 and Apple silicon only. The goal is the complete product, tested end to
end, with no known security hole.

The work has three chapters (owner decision 2026-10-09; `docs/product/vision-and-scope.md`, `docs/roadmap/README.md`):

1. **Complete, test and validate**, in two tracks of ten phases each (`docs/roadmap/chapter-1a-windows-phases.md`,
   `docs/roadmap/chapter-1b-mac-phases.md`). Windows first: finish and prove the Windows app, build the internal `.exe`,
   the team tests it by hand. Then the Mac: a SwiftUI shell over the same Rust core, the internal `.dmg`, team testing.
   Builds are unsigned, ask the team password (minimum 8 characters, ADR-0030) and download the AI after install
   (ADR-0026). **This is the only chapter in progress.**
2. **Website.** Purchased domain and company email, faithful recreations of the real screens. Its implementation plan is
   written when Chapter 1 closes.
3. **Commercial distribution.** Microsoft Store and Mac App Store, 7-day trial, paid activation (target 3,000 rupees,
   89 dollars, 89 euros), signing and notarization. Its implementation plan is written when Chapter 2 closes. No
   Chapter 2 or Chapter 3 code is written in Chapter 1.

Decisions behind this: `docs/adr/` (register in `docs/adr/README.md`) and `docs/implementation/owner-decisions.md`.

AI rules (ADR-0028, permanent): measure first. The evaluation harness and its release bar (90 percent correct tool, 85
percent correct end state) come before any AI change. While people use the app, only deterministic code judges AI answers,
never a model, with at most three passes and the answer shown at once when pass 1 passes every check. Claude evaluates at
development time only, and its findings become code.

## Start every session this way (stops context rot)

Context rot is the slow loss of accuracy in long sessions and the quiet build-up of stale instructions. Prevent it:

1. Read, in this order and nothing more: this file, `docs/implementation/handoff.md` (one page, rewritten at the end of
   each session), the phase page you work on (`docs/roadmap/chapter-1a-windows-phases.md` or `chapter-1b-mac-phases.md`), then only the
   one work item you will do.
2. Say which work item, which components it touches, how it will be verified. Do not start unplanned work; if the owner
   asks for something outside the plan, do it and record it in the log.
3. Never read a 2,000-line specification or the whole progress log to "get oriented". Use `graphify query "<question>"`
   (graph in `graphify-out/`), the feature status row, or `grep` on the one file. The specifications are lookups.
4. Work one work item at a time. When a task is done, update the handoff file and stop or start a fresh session. After
   about 60 tool calls on one topic, or when the plan changes, write the handoff and continue in a new session.
5. The source code is the ground truth. A document that disagrees with the code is wrong: fix the document in the same
   change. Status text lags code (three feature rows were stale on 2026-10-09); read the code before trusting a row.
6. Keep this file short (about 150 lines). Anything long belongs in `docs/` with a link here. Anything about the past
   belongs in `docs/archive/` or the progress log, never here.
7. Memory (`~/.claude/projects/C--OpenFrame/memory/`) holds only facts the repository cannot show: owner preferences,
   decisions not yet written down, hardware facts. Delete a memory when the repository records it or it is wrong.

## Project control documents

Map and precedence: `docs/README.md`. Names are descriptive, never item codes.

- `docs/product/vision-and-scope.md`: audience, chapters, what we never claim.
- `docs/roadmap/`: chapter plans. Chapter 1 has a Windows track (W1 to W10) and a Mac track (M1 to M10) with exit criteria.
- `docs/implementation/masterplan.md`: 26 areas with checklists and evidence (the detailed record).
- `docs/implementation/feature-status.md`: how far each product feature has come (Specified, Designed, Built, Reachable, Verified, Release-ready).
- `docs/implementation/owner-decisions.md`: decisions and manual tasks that only the owner can make or do.
- `docs/implementation/handoff.md`: where we are right now and the next three items.
- `docs/implementation/progress-log.md`, `reconciliation-log.md`, `docs/release/launch-readiness-register.md`.

Progress numbers come from `npm run plan:status` and `node scripts/check-feature-status.mjs`. Never type a count by hand.

### Layout and decisions
- The repository root holds only README, CONTRIBUTING, DEVELOPMENT, CLAUDE, LICENSE-PENDING and THIRD_PARTY_LICENSES
  (plus code and tool folders). Specifications live in `docs/specs/`, decisions in `docs/adr/`, the manual in `docs/manual/`.
- Newer decisions win and older ones are deprecated, not deleted: mark the old text superseded in place, add the new ADR,
  add a row to `docs/adr/README.md`. Sections marked SUPERSEDED are history; never implement them.
- Moving or renaming a document means updating links (run `node scripts/check-links.mjs`), `docs/README.md`, the archive
  index if it is history, and `docs/implementation/reconciliation-log.md`.

### Security standard (permanent, Windows and Mac)
Meet the bar of well-regarded professional desktop applications: least privilege, binary hardening, strict webview and
IPC, safe process control, bounded parsers and confined paths, clean supply chain, signed and verified distribution (at
Chapter 3), no silent network use, App Sandbox on the Mac store build. The full list is the "Windows and Mac security
standard" section of `masterplan.md`. Run `security-guidance` and `ecc:security-review` on parsers, paths, packages, AI,
process control, settings, imports and anything that touches payment or licence checks.

### Writing standard (permanent)
No em dashes and no AI-sounding writing anywhere: app screens, errors, exports, installers, listings, website, manual,
documents, code comments, commit and pull request text. Rules and banned phrases: `docs/engineering/35-writing-standard.md`.
`npm run check:writing` is a ratchet that fails when a count goes up. When you touch a file, fix its hits.

### Clean as you go (permanent)
When a feature or fix lands, in the same pull request: delete the dead code it replaces, mark superseded documents and
decisions as superseded (or archive them), remove stale tests and unused dependencies, and run `graphify update .`. The
repository must never carry an old way and a new way side by side.

### Git flow (permanent, owner decisions 2026-10-06 and 2026-10-08)
Work entirely in the local project folder. GitHub is never a dependency for developing, running, testing, packaging or
building the app. For each finished, verified slice of work:
1. Create or reuse a feature branch from `main` (`git switch -c <area>/<short-name>`).
2. Commit locally with a message that says what changed and why, in normal prose. Local commits are welcome.
3. Never run `git push`, open or merge a pull request, create a repository or change repository settings unless the owner
   explicitly instructs it in that conversation. When asked: push the branch, open a pull request listing the plan items,
   the evidence and the checks, merge only when the automatic checks are green, then switch to `main`, pull and delete the
   branch. Never merge a red check, never skip a hook. Finished work reaches `main` by a local merge only after its unit
   tests and end-to-end specs pass. A work-in-progress commit is a checkpoint, never a reason to merge.
4. Before any push the owner requests: the app builds and runs locally, temporary and debug files are removed,
   `.gitignore` covers build output, dependencies, temporary files, environment files and secrets, and no `.env`, key,
   password, token, certificate or signing key is tracked.
Never run destructive git operations (force push, hard reset, branch deletion of unmerged work) without the owner's say.
The repository stays private. **Never build an installer or a release executable (`npm run build`, `npm run bundle`, `tauri build`, `.msix`,
`.dmg`) until the owner says so in that conversation.**

### While working
- One vertical slice at a time. Follow `CONTRIBUTING.md` (feature contract, risk level, definition of done, the 15 invariants).
- Tick a plan box only with evidence written after it `(ev: ...)`. Move a feature status up only with evidence. Designed,
  built, reachable, verified and release-ready are different states.
- If code behaviour, a decision, a schema, an operation or a release setting changes, update the documents listed in
  `docs/README.md` section 8 in the same change.
- Do not edit `docs/audits/` or the files in `docs/archive/` (evidence and history). Record corrections in the
  reconciliation log.
- Check edge cases, crash cases and failure cases for every feature, including the AI path: the user must never lose data
  and the app must never hang.
- Never decide an open item in `owner-decisions.md`, and never act on a Proposed ADR as if it were accepted. If unsure,
  ask the owner in very simple language.

### Storage and workspace hygiene (permanent)
Full policy: `docs/development-storage-policy.md` (sections 7 and 8 hold the root causes and the final rules).
- `C:\OpenFrame` is the canonical checkout. Create a worktree only when parallel isolated work genuinely needs one, and
  remove it as soon as its work is merged and verified. Before removal prove its commits and uncommitted work are
  preserved. Never merge or remove a worktree merely because its branch looks merged.
- Rust `target/`, `node_modules`, packages, caches and logs are reproducible. One build folder per checkout under
  `C:\OpenFrame\.build-cache`, built through `node scripts/cargo-queued.mjs` (one heavy build at a time, 4 jobs).
  Never a `target/` inside a worktree.
- **Junction danger.** A worktree's `node_modules` is a junction. `rm -rf` in Git Bash follows junctions and once deleted
  1,088 tracked files of the main checkout. Unlink first with `cmd //c rmdir <worktree>\node_modules`, then
  `git worktree remove`, then `git worktree prune`. Never `rm -rf` a worktree folder.
- Budget: investigate at about 10 GB of temporary data beyond dependencies; never hold 50 GB without the owner's approval.
  Measure sizes with `du -sh`, not recursive PowerShell listing.
- Agents never run in parallel unless the owner asks. Parallel agents each need a file-ownership map first.

### At the end of every session (mandatory)
1. Rewrite `docs/implementation/handoff.md` (current state, next three items, open blockers, what not to do).
2. Add an entry at the top of `docs/implementation/progress-log.md` using its template and update the dashboard from
   `npm run plan:status`. Tick the plan boxes you completed with evidence.
3. Run `graphify update .` after code changes.
4. Commit locally as described in the git flow. Do not push.
5. Finish the reply with a short status: progress by area, what is blocked on the owner, the next two or three items, and
   plainly what was not run or not verified.

## Installed tooling

| Purpose | Tool | Rule here |
|---|---|---|
| Plan, build, review | `superpowers`, `agent-skills`, `ecc` | Plans map to work items; they never replace `CONTRIBUTING.md` or the feature status file. Do not run overlapping workflows at once |
| Security while coding | `security-guidance`, `ecc:security-review` | Parsers, paths, AI, process, payment, imports |
| Fewer output tokens | `caveman`, `graphify` | Code, commit messages and documents stay in normal prose; compress chat replies only |
| UI and copy quality | `impeccable`, `frontend-design`, `docs/engineering/34-ux-laws-checklist.md` | The UX specification and real screens win over taste |
| Review agents | `ecc:rust-reviewer`, `ecc:typescript-reviewer`, `ecc:silent-failure-hunter`, `ecc:pr-test-analyzer` | Before ticking a plan box |

Owner decision 2026-10-16: `claude-mem` stays (user settings); the GSD framework and the stale `AGENTS.md` were removed
(moved to the git-ignored `.build-cache/removed-2026-10-16/`, its hooks dropped from `.claude/settings.json`). OmniRoute
is not installed on purpose (it routes prompts through third parties).
Anything named in `docs/website/prompts/` is unverified: check source, licence and permissions first.

## graphify

This project has a knowledge graph at `graphify-out/` (local, git-ignored). For codebase questions run
`graphify query "<question>"` first; `graphify path "<A>" "<B>"` for relationships; `graphify explain "<concept>"` for a
concept. Read `graphify-out/GRAPH_REPORT.md` only for a broad architecture review. After code changes run
`graphify update .` (AST only, no cost).
