<!--
OpenFrame pull request. One vertical slice per PR (CONTRIBUTING.md section 3, item 13).
Delete sections that truly do not apply and say why ("n/a: docs only"). Do not leave them blank.
-->

## Build plan and feature status

- Build plan area and box(es): <!-- e.g. Writing comfort: Custom keyboard shortcuts -->
- Feature(s) and status change: <!-- e.g. Custom keyboard shortcuts: Designed to Built -->
- Source requirement: <!-- PRD/FSD/UX section, Revision 1 section, or ADR -->
- Risk level (see docs/engineering/31-ai-assisted-development-controls.md section 2): <!-- cosmetic, local UI, operation wiring, canonical data, cross-module, security, release, data format, AI authority -->

## Feature contract (required for customer-visible changes and for canonical data, cross-module, security, release, data format and AI authority changes)

| Item | This change |
|---|---|
| User outcome | |
| How a user reaches it (screen) | |
| Canonical data | |
| Queries / commands (Registry ops) | |
| Permissions (Viewer and Commenter denied writes?) | |
| Undo and recovery | |
| Search document | |
| AI exposure, or explicit non-exposure | |
| Import / export | |
| Accessibility (keyboard, focus, labels, no colour-only meaning) | |
| Performance budget (docs/engineering/21-performance-budgets.md) | |
| Security and privacy | |
| Windows and Mac impact | |
| Known limitations | |

## Contract before code

- Modules affected:
- Invariants touched (CONTRIBUTING §2):
- Migrations (new file number, never an edited frozen file):
- Platform impacts (Windows adapter, macOS adapter, both packages):

## Semantic diff check: answer each

- [ ] Bypasses `Store::mutate`? **must be no**
- [ ] Adds a Registry operation? If yes: metadata, permission, undo, search document, AI decision, eval case, regenerated `docs/engineering/ai-tool-coverage.md`
- [ ] Changes permissions?
- [ ] Changes file paths or path handling? If yes: confined in Rust, UNC refused, no zip-slip or symlink escape
- [ ] Changes or adds migrations? If yes: next linear number, fixture test from the previous version, docs 11 and 23 updated
- [ ] Changes AI exposure?
- [ ] Changes export or import formats?
- [ ] Changes platform assumptions?

## Evidence (tests are evidence; "works on my machine" is not)

- Lifecycle covered (create, save, restart, undo, redo, delete, restore, export, import):
- Tests added (named after the requirement):
- E2E spec:
- Security suites run (parser, path, package, AI, process, external link, update) and `security-guidance` / `ecc:security-review` result:

## Gates

- [ ] `npm run verify` green (format, clippy `-D warnings`, typecheck, eslint, governance checks, cargo test, vitest)
- [ ] Generated bindings committed, no drift
- [ ] `cargo deny check`, `npm audit --omit=dev`, licence inventory green (new dependency justified in this PR)
- [ ] No project content, paths or keys in logs
- [ ] Writing standard: no em dashes or AI-sounding phrases in new text (`npm run check:writing`)

## Documentation and cleanup kept in step (docs/README.md section 5)

- [ ] Feature status row and build plan box with `(ev: ...)`
- [ ] Dead code and superseded documents removed or marked superseded in this pull request
- [ ] Affected spec or engineering doc
- [ ] ADR and `docs/adr/README.md` row if a decision changed
- [ ] User manual source / in-app help if user-visible
- [ ] Progress log entry
