# Contributing to OpenFrame Studio

OpenFrame holds filmmakers' only copy of their work. Contributions are judged first on **not losing or corrupting
user data**, second on **matching the approved product behavior exactly**, and only then on everything else.

> **Licensing notice.** No open-source license has been selected yet ([LICENSE-PENDING.md](LICENSE-PENDING.md)).
> The contribution sign-off policy (DCO or CLA) will be set together with the license. Until then, contributions are
> accepted only from the core team.

## 1. Authority: where behavior comes from

Implementation precedence (Engineering Package Index):

1. Product PRD: scope, priority, product boundaries
2. Functional Specification (FSD): observable behavior and acceptance criteria
3. UX/UI Specification and `HTML_Mockups/`: presentation, exact labels
4. Domain & Data Specification: identities, relationships, lifecycle, source of truth
5. AI, Import/Export, Offline/Collaboration and Security specifications: cross-cutting contracts. The LAN
   collaboration sections are superseded by the product-owner decision in [ADR-0007](docs/adr/ADR-0007-lan-collaboration-removed.md).
6. Engineering documents (`docs/engineering/`, `docs/adr/`): concrete implementation choices

When code and a product rule disagree, the code changes. Do not "fix" the product in code. When two specifications
disagree, or a specification is silent, **do not decide silently**. Record the decision in an ADR (see
[ADR-0012](docs/adr/ADR-0012-spec-conflict-resolutions.md) for existing resolutions) and reference it in the code.

## 2. Non-negotiable engineering invariants

From the Engineering Package Index §5. Reviewers reject any change that violates them.

| # | Invariant | How the codebase enforces it |
|---|---|---|
| 1 | React never writes SQLite directly. | The webview has no fs/shell/http plugins (ESLint `no-restricted-imports`, `capabilities/default.json`). Its only data command is `of_invoke`. |
| 2 | AI never writes SQLite or arbitrary project files directly. | Design contract (ADR-0006; AI module in development): AI proposals become Change Sets that are applied by normal commands under `ActorOrigin::Ai`, and the sidecar has no project authority. |
| 3 | Every persistent project mutation is a Rust application command. | `Registry::command` + `Store::mutate` is the only write path. Nothing else opens a write transaction. |
| 4 | Every command that mutates canonical state executes in a transaction. | `Store::mutate` uses `BEGIN IMMEDIATE`, and all side effects are rolled back together. |
| 5 | Every persistent object uses a stable opaque identity; display numbers are not identity. | UUIDv7 `id TEXT PRIMARY KEY`, `position` for order, numbers derived at read time (ADR-0011). |
| 6 | Filesystem assets are referenced through project-managed records and validated paths. | `asset` rows + `openframe_security::confine`; the UI opens files by asset id only. |
| 7 | Project import is validate → preview → Change Set → apply. | Module contract: import ops parse into a preview, and applying uses `Capability::Import`/`ApplyChangeSet` through the pipeline. Archive intake uses `openframe_security::archive`. (The import modules are in development.) |
| 8 | Historical snapshots and issued exports are immutable snapshots. | Module contract: snapshot rows are written once and never updated, and exports are written files that are never re-read as truth. (The snapshot/export modules are in development.) |
| 9 | LAN participants never open the host SQLite file over a network share. | LAN collaboration is removed (ADR-0007), so there are no participants. Projects on network shares are still refused (`project_format.network_location`), and sharing happens by copy or exchange package. Do not add network listeners. |
| 10 | Core operation does not depend on an account, internet, cloud database or cloud AI. | There is no account or cloud code. Network use is limited to explicit, user-approved downloads. |
| 11 | Local AI failure never blocks core editing. | AI runs in an optional sidecar process started on demand. The core never waits on it. |
| 12 | Preserve recoverable state on write failure wherever technically possible. | WAL + `synchronous=FULL`, atomic file writes, safety backups before migration/replace, crash marker + checkpoint. |
| 13 | Model/runtime downloads are untrusted until integrity verification succeeds. | Primitives implemented: `openframe_security::{sha256_file, verify_ed25519}`. The download manager must verify before the atomic `.partial` → final rename (ADR-0006). |
| 14 | Private Notes are a distinct authorization boundary. | Every query filters `owner_user_id = actor.user_id`; search filters owner-only documents. |
| 15 | New work requires tests and traceability to approved behavior. | See §4. Tests are named after the requirement (`fsd_story_003_…`). |

Additional source-of-truth rules (Domain spec, PRD §7.2) that must never be "optimized away": Vault/Story/Screenplay/
Production/Schedule/Call Sheet data is never silently synchronized. Copies get new identities. Production never
rewrites screenplay text. Call sheet edits never rewrite the schedule. Breakdown suggestions never auto-apply.

## 3. Review contract

Every pull request needs at least one reviewer approval and green CI. The author states, and the reviewer verifies:

1. **Traceability.** Which PRD/FSD requirement(s) or ADR this implements, with section or ID.
2. **Data safety.** Mutations go through `Store::mutate` with the right `Capability`. There are no raw write
   connections, no `ON DELETE CASCADE`, and no BLOB columns in canonical tables. Deletes are recoverable
   (`soft_delete` + trash handler) unless the spec says otherwise.
3. **Undo and recovery.** The change is undoable where the FSD requires it (automatic through the pipeline), typing
   uses a coalesce key, and non-undoable actions are marked `.not_undoable()` with a reason.
4. **Permissions.** Viewer and Commenter are denied mutations. Private content is owner-filtered.
5. **Errors.** Errors use a stable `AppError` code (ESD §12 taxonomy) and a filmmaker-friendly message with no
   SQL or OS jargon. Technical detail goes in `.with_detail()` and never contains project content.
6. **UI.** Labels match the UX spec and mock-ups exactly. Everything is keyboard reachable with visible focus and
   labelled icon buttons, and status is never shown by colour alone. There is no fake data and no buttons that do
   nothing.
7. **Schema.** Shipped migrations are never edited, only appended to (pre-release module migrations may still
   change; see 23-release-migration.md). Hub-table contract columns are never renamed or removed.
8. **Bindings.** Regenerated TypeScript types are committed.
9. **Docs.** An ADR or engineering document is added or updated when the change makes or alters a decision.

Reviewers block a PR for any unaddressed item. "Works on my machine" is not evidence. Tests are.

## 4. Testing requirements

| Layer | Tooling | Required for |
|---|---|---|
| Domain/persistence/security unit tests | `cargo test -p <crate>` (in-module `#[cfg(test)]`) | Pure logic, SQL helpers, parsers, path/zip safety |
| Application integration tests | `crates/openframe-application/tests/*.rs` with `openframe_test_support::TestEnv` | **Every op**: happy path, validation, undo/redo, delete/restore/purge, permissions (Viewer/Commenter denied), search hits, persistence across `env.restart()`, crash via `env.crash_and_restart()`, and every FSD acceptance criterion of the module |
| Frontend unit/component tests | Vitest + Testing Library (`apps/desktop/src/**/*.test.ts(x)`) | Non-trivial pure logic, and components with behavior (keyboard, focus, error states) |
| Golden/fixture tests | Rust tests with fixtures under the test crate | Import parsers, PDF/FDX/Fountain output determinism |

Rules:

- Name tests after the requirement they prove (`fsd_script_005_comment_reply_resolve`).
- Tests must be hermetic. Use `TestEnv`/`AppConfig::isolated`, never the real `%LOCALAPPDATA%` or `Documents`.
- Fixtures live only in tests, never in shipped code paths.
- A bug fix starts with a failing regression test.
- CI must be green: `cargo fmt --check`, `clippy -D warnings`, `cargo test --workspace`, `tsc`, `eslint --max-warnings 0`,
  `vitest`, the web build, and no generated-bindings drift.

Full strategy and release gates: [docs/engineering/22-qa-test-strategy.md](docs/engineering/22-qa-test-strategy.md).

## 5. Dependency policy

Adding a dependency is a design decision. Prefer the existing workspace dependencies (`Cargo.toml`
`[workspace.dependencies]`, `apps/desktop/package.json`).

A new dependency must be:

1. **Necessary.** It does real work that would be unreasonable to write and maintain ourselves.
2. **Permissively licensed.** MIT, Apache-2.0, BSD-2/3-Clause, ISC, Zlib, Unicode-3.0, 0BSD, CC0, Unlicense or BSL-1.0.
   Copyleft (GPL/AGPL/LGPL/SSPL/EUPL) and unknown licenses are rejected. Weak copyleft (MPL-2.0) needs an explicit
   per-crate exception in `deny.toml` with a reason.
3. **Maintained and reputable.** Recent releases, no unaddressed RustSec/npm advisories, reasonable transitive footprint.
4. **From crates.io / npmjs only.** No git dependencies without review (`deny.toml` `[sources]`).
5. **Offline-safe.** It does no network I/O, telemetry or update checks of its own.
6. **Webview-safe** (npm). It never gives the webview filesystem, shell or network authority.

In the PR, justify the dependency, run `npm run licenses`, and commit the updated `THIRD_PARTY_LICENSES.md`. CI runs
`cargo deny check`, `npm audit --omit=dev` and the license inventory. Releases treat them as hard gates.

TLS is rustls-only (`openssl-sys` and `native-tls` are banned in `deny.toml`).

## 6. Commit and branch conventions

- Branch from `master`, one topic per branch: `feat/<module>-<topic>`, `fix/<topic>`, `docs/<topic>`.
- Use [Conventional Commits](https://www.conventionalcommits.org/): `feat(story): reorder scene cards across sequences`,
  `fix(persistence): keep FK checks on after failed migration`, `docs(adr): add ADR-0013`. Types: `feat`, `fix`,
  `perf`, `refactor`, `test`, `docs`, `build`, `ci`, `chore`.
- The subject is imperative and ≤ 72 characters. The body explains *why* and references the FSD/PRD section or ADR.
- Commit generated bindings and `THIRD_PARTY_LICENSES.md` in the same commit as the change that caused them.
- Never commit secrets or signing material (`*.pfx`, `*.p12`, `*.key`, `.env*` are gitignored). Never commit real user
  projects or personal data as fixtures.
- Do not rewrite shared history. Rebase your own branch before review if needed.
- AI-assisted commits carry a `Co-Authored-By:` trailer.

## 7. Security reports

Do not open public issues for vulnerabilities (path traversal, package extraction, IPC bypass, permission bypass,
signature verification). Report them privately to the maintainers. The threat model is in
[docs/engineering/20-security-threat-model.md](docs/engineering/20-security-threat-model.md).
