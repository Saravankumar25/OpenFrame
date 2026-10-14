## What we are building

OpenFrame Studio is a local-first desktop app for filmmakers (Tauri v2, React and TypeScript, Rust, SQLite). It is a **paid app with a 7-day free trial** and it ships as **four files from one codebase** (ADR-0024), through **two platform adapters** (Windows and macOS):

1. a Microsoft Store package (`.msix`),
2. a Mac App Store package (`.pkg`),
3. an internal Windows installer (`.exe`) for the owner's team only, never sold (ADR-0023),
4. an internal Mac disk image (`.dmg`), Developer ID signed and notarized, for the team only.

The two internal builds ask for a team password on every launch; only an Argon2id hash built from an environment
variable is embedded, never the password (ADR-0024). The two store packages are the public market and never show a price.

Windows 11 only. Apple silicon only. The goal is the complete product, tested end to end, with no known security hole. Decisions behind this: `docs/adr/ADR-0017`, `ADR-0018`, `ADR-0022`, `ADR-0023` and `ADR-0024`.

## Project control documents (read first, keep current)

OpenFrame is built against one plan. These files are the source of truth for what to do, what is done and what is decided. They use plain descriptive names, never item codes (no M1.04, W2, G7, P0, S3 or D5):

- `docs/README.md`: documentation map, which document wins, what to update when code changes.
- `docs/implementation/masterplan.md`: 26 numbered areas (for example "Data safety and crash recovery", "AI assistant quality and hardening", "Mac App Store package"), each with a checklist. When every box and the release sign-off table are done, the product is ready to ship.
- `docs/implementation/feature-status.md`: how far each product feature has come (Specified, Designed, Built, Reachable, Verified, Release-ready).
- `docs/implementation/owner-decisions.md`: decisions and manual tasks that only the owner can make or do.
- `docs/implementation/reconciliation-log.md`: known conflicts, where each source document went, and the old-code to new-name tables.
- `docs/implementation/progress-log.md`: dashboard and one entry per session.
- `docs/release/launch-readiness-register.md`: operations, legal, commerce, support and release requirements.

Progress numbers come from `npm run plan:status` (boxes per area). Never type a count by hand.

### Layout and decisions (keep it this way)
- The repository root holds only README, CONTRIBUTING, DEVELOPMENT, Codex, LICENSE-PENDING and THIRD_PARTY_LICENSES. Specifications live in `docs/specs/`, decisions in `docs/adr/` (its README is the decision register), the manual in `docs/manual/`. Never add loose documents to the root.
- Newer decisions win and older ones are deprecated, not deleted: mark the old text superseded in place, add the new ADR, and add a row to `docs/adr/README.md`. Sections marked SUPERSEDED are history; never implement them.
- Moving or renaming a document means updating links (run a link check), `docs/README.md` and `docs/implementation/reconciliation-log.md`.

### Order of work
Follow the "Build order" table in `masterplan.md`. Do not start an area while an earlier area it depends on has unblocked boxes, except where the table marks areas as independent. Boxes that wait on a Proposed ADR or an owner input stay untouched until the owner decides; say so in the log.

### Security standard (permanent, Windows and Mac)
OpenFrame must meet the security bar of well-regarded professional desktop applications on both systems. The standard is the "Windows and Mac security standard" section of `masterplan.md`: least privilege, binary hardening, strict webview and IPC, safe process control, bounded parsers and confined paths, clean supply chain, signed and verified distribution, no silent network use, App Sandbox on the Mac. Apply it to every change and run `security-guidance` and `ecc:security-review` on parsers, paths, packages, AI, process control, settings, imports and anything that touches payment or licence checks.

### Writing standard (permanent)
No em dashes and no AI-sounding writing anywhere: app screens, errors, exports, installers, listings, website, manual, documents, code comments, commit and pull request text. Rules and banned phrases: `docs/engineering/35-writing-standard.md`. `npm run check:writing` is a ratchet that fails when a count goes up. When you touch a file, fix its hits in the same pull request.

### Clean as you go (permanent)
When a feature or fix lands, in the same pull request: delete the dead code it replaces, mark superseded documents and decisions as superseded (or archive them), remove stale tests and unused dependencies, and run `graphify update .`. The repository must never carry an old way and a new way side by side.

### Git flow (permanent, owner decision 2026-10-06, amended the same day)
Work entirely in the local project folder. GitHub is never a dependency for developing, running, testing, packaging or building the app. For each finished, verified slice of work:
1. Create or reuse a feature branch from `main` (`git switch -c <area>/<short-name>`).
2. Commit locally with a message that says what changed and why, in normal prose. Local commits are welcome; they are never published on their own.
3. Never run `git push`, open or merge a pull request, create a repository or change repository settings unless the owner explicitly instructs it in that conversation. When the owner does ask for a push: the branch goes up, a pull request is opened (`gh pr create`) listing the build plan items, the evidence and the checks run, and it is merged only when the automatic checks are green; after the merge switch back to `main`, pull, and delete the branch. Never merge a red check, never skip a hook.
4. Before any push the owner requests: the app builds and runs locally, temporary and debug files are removed, `.gitignore` covers build output, dependencies, temporary files, environment files and secrets, and no `.env`, API key, password, token, certificate or signing key is tracked.
Never run destructive git operations (force push, hard reset, branch deletion of unmerged work) without the owner's say. The repository stays private.

### At the start of every session
1. Read the dashboard and the latest entries in `progress-log.md`, then the area of `masterplan.md` you will touch.
2. Say which areas, plan items (by their bold title) and feature rows (by name) this session will work on. Do not start unplanned work; if the user asks for something outside the plan, do it and record it in the log.
3. Check `owner-decisions.md`: never decide an open item yourself, and never act on a Proposed ADR as if it were accepted. If you are unsure, ask the owner in very simple language.

### While working
- One vertical slice at a time. Follow `CONTRIBUTING.md` (feature contract, risk level, definition of done) and the architecture invariants in its section 2.
- Tick a plan box only with evidence written after it `(ev: ...)`. Move a feature status up only with evidence. Never mark something done that was designed, written or compiled but not tested. "Designed, built, reachable, verified, release-ready" are different states.
- If code behaviour, a decision, a schema, an operation or a release setting changes, update in the same change the documents listed in `docs/README.md` (feature status, plan box, affected spec or engineering doc, ADR, AI coverage doc, manual source). Documentation must never lag the repository.
- Do not edit `docs/audits/` or `docs/archive/` (evidence and history). Record corrections in `reconciliation-log.md`.
- Check edge cases, crash cases and failure cases for every feature, including the AI path: the user must never lose data and the app must never hang.

### Storage and workspace hygiene (permanent)
Full policy: `docs/development-storage-policy.md`. A past run reached 273 GB because agent worktrees each kept their own Rust `target/` and `node_modules`. Never again.
- `C:\OpenFrame` is the canonical checkout. Develop there. Create a worktree only when parallel isolated work needs one, and reuse an existing one first. Every agent worktree is temporary; never leave orphan source copies.
- Rust `target/`, `node_modules`, generated packages, caches and logs are reproducible, never project data. Every checkout builds into the shared `CARGO_TARGET_DIR` (`C:\OpenFrame\.build-cache\target`, user-level variable, git-ignored); never give a worktree its own `target/`. Avoid full installs or builds in every worktree. Never delete source, commits, branches, databases, user data, assets or config.
- Budget: before launching several agents, consider disk impact. At about 10 GB of temporary worktree or build data beyond expected dependencies, inspect and clean before spawning more. Never hold 50 GB or more without the owner's explicit approval.
- Remove a worktree only after proving its commits and uncommitted work are preserved, using `git worktree remove` and then `git worktree prune`, never raw deletion. Keep any worktree with unique work.
- When a task is integrated: confirm work is preserved, remove its worktrees, prune, delete its `target/`, `node_modules` and caches, confirm the app still builds and runs, and report what was removed and roughly how much space came back.
- Measure sizes with Bash `du -sh`, not recursive PowerShell `Get-ChildItem` (it ran for over an hour on `target/`).

### At the end of every session (mandatory)
1. Append an entry to `docs/implementation/progress-log.md` (newest first) using its template: areas and items worked, done, evidence, features moved, docs updated, decisions needed, blockers, next items, checks run.
2. Update the dashboard in that file from `npm run plan:status` and tick the plan boxes you completed.
3. Run `graphify update .` after code changes.
4. Commit, push and open or update the pull request as described in the git flow.
5. Finish the reply with a short status report: progress by area (for example "Data safety and crash recovery: 5 of 7"), what is blocked on the owner, and the next two or three items. State plainly what was not run or not verified.

## Installed tooling and when to use it

Plugins installed 2026-10-05 (user scope), on top of `ecc`, `impeccable`, `frontend-design`, `figma`. Use the smallest set that fits; do not run overlapping workflows at once (for example Superpowers brainstorming and `ecc:plan` for the same task).

| Purpose | Tool | Rule here |
|---|---|---|
| Plan, build, review with TDD | `superpowers`, `agent-skills` (`/spec`, `/plan`, `/build`, `/test`, `/review`, `/code-simplify`, `/ship`) | Plans must map to build plan items; they never replace `CONTRIBUTING.md` or the feature status file |
| Project scan | `Codex-setup` | Read-only recommendations; apply none without the user's approval |
| Security while coding | `security-guidance`, `ecc:security-review` | Run on any parser, path, package, AI, process, payment or theme-import change |
| Fewer output tokens | `caveman`, plus `ecc:context-budget`, `ecc:strategic-compact`, `ecc:token-budget-advisor`, `graphify` | Code, commit messages, docs and ADRs stay in normal prose; compress only chat replies |
| UI and copy quality | `impeccable`, `frontend-design`, `anthropic-skills:humanizer`, `docs/engineering/34-ux-laws-checklist.md` | The UX spec and `HTML_Mockups/` win over taste; no fake claims, no em dashes in user-facing text |
| Review agents | `ecc:rust-reviewer`, `ecc:typescript-reviewer`, `ecc:react-reviewer`, `ecc:silent-failure-hunter`, `ecc:pr-test-analyzer`, `ecc:refactor-cleaner` | Before ticking a plan box |

Not installed on purpose: **OmniRoute** (routes prompts and source code through third-party providers, many look-alike forks, rotates free-tier accounts to avoid usage limits; conflicts with the project's privacy rules and probably provider terms) and **Codex-mem** (records every tool call into a local database with a background worker, can call external AI providers and sync to a cloud service, costs extra tokens; duplicates `progress-log.md` and memory). Install either only after the owner decides, local-only for Codex-mem. Anything else named in `docs/website/prompts/` is unverified: check source, licence and permissions first.

## graphify

This project has a knowledge graph at graphify-out/ with god nodes, community structure, and cross-file relationships.

Rules:
- For codebase questions, first run `graphify query "<question>"` when graphify-out/graph.json exists. Use `graphify path "<A>" "<B>"` for relationships and `graphify explain "<concept>"` for focused concepts. These return a scoped subgraph, usually much smaller than GRAPH_REPORT.md or raw grep output.
- If graphify-out/wiki/index.md exists, use it for broad navigation instead of raw source browsing.
- Read graphify-out/GRAPH_REPORT.md only for broad architecture review or when query/path/explain do not surface enough context.
- After modifying code, run `graphify update .` to keep the graph current (AST-only, no API cost).
