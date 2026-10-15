# OpenFrame Studio launch readiness register

> **What this is.** The register of operations, legal, commerce, security, support and release requirements for taking
> OpenFrame Studio from codebase to a launched, supportable product. Product feature state lives in the
> [feature status document](../implementation/feature-status.md). Build order and the Store gate live in the
> [build plan](../implementation/masterplan.md). Build plan items cite rows of this register by key (for example `COMM-001`).
>
> **Decisions that shape this register (owner, 2026-10-06).** The product is paid, with a 7-day free trial and no credit card to
> start. It runs on Windows 11 only and on Apple silicon Macs only. One codebase, with a Windows adapter and a macOS adapter,
> produces three release files: the Microsoft Store package (`.msix`), the Windows installer (`.exe`) and the Mac App Store
> package (`.pkg`). Commerce rows (`COMM-*`, `EXE-09-*`) and Mac rows (`MAC-*`, `EXE-07-*`) are therefore required for release.
> None of them is conditional any more. Analytics and crash-aggregation rows follow ADR-0021: the app sends nothing
> automatically. The baseline commit is `master cd60f08`, which has the same tree as `main d30b588`. Additions from the
> intake notes are at the end of this document.
>
> The version of this register from before the restructure is kept at
> [launch-register-before-restructure](../archive/2026-10-06-launch-register-before-restructure.md).

**Purpose:** the single execution control document for taking OpenFrame Studio from codebase to a commercially launchable, secure, supportable and maintainable desktop product under the umbrella company.

**How the register is organised:** every requirement appears once, in the requirement register. Reusable quality gates are defined once and referenced by name. Operational runbooks and final audits do not restate every requirement.

**A checkbox is never evidence.** An item is complete only when its required state and objective evidence exist.

**Baseline:** `Saravankumar25/OpenFrame` / `main` / `d30b5889ab5edf9f69d60f8e6d600cb11d4921a0`. The supplied completion plan recommends a dependency-aware strategy that preserves the durable core and reconciles old and new requirements by module.

---

# How requirement status works

| Status | Meaning |
|---|---|
| Specified | Requirement and source known. |
| Designed | Data, IPC, UI, security and ownership contract defined. |
| Built | Real code exists and mechanical checks pass. |
| Reachable | A normal user can discover and use it. |
| Verified | Lifecycle, persistence, failure, permissions, platform and interoperability evidence exists. |
| Release-ready | Release, security, support, commercial and documentation gates are green. |
| Operated | Post-launch monitoring, support, backup and maintenance have been exercised. |
| Continuity-ready | A backup operator can recover, release and support the product. |

Importance has three levels. **Must ship** means a release cannot go out without it. **Should ship** means it blocks launch quality. **Can wait** means it can follow the first release.

Allowed result labels: **OPEN / IN PROGRESS / BLOCKED / EVIDENCE PENDING / N/A / PASS / RISK ACCEPTED**.

N/A requires a reason. Critical N/A decisions require explicit approval. Risk acceptance never substitutes for unresolved customer-data loss, critical security or release-integrity failures.

The completion plan defines the states from Specified to Release-ready and the feature lifecycle Discover → Use → Save → Undo → Recover/Reopen → Export/Re-import/Continue.

# Current-state freeze

- [ ] Freeze the exact repository commit/version used as the release-program baseline.
- [ ] Record the current release declaration separately from implementation claims.
- [ ] Create the canonical feature ledger and evidence index.
- [ ] Create representative sample projects and import/export fixtures.
- [ ] Create AI retrieval/evaluation corpus and hardware matrix.
- [ ] Create Windows 11 and Apple silicon Mac clean-machine matrix.
- [ ] Create commercial/store/account asset inventory.
- [ ] Create release evidence storage with immutable release identifiers.
- [ ] Keep unfinished capabilities out of launch claims until they are Release-ready.

The current evidence says the creative/production release declaration still marks major workspaces as in development/not usable, while Offline AI/assistant are implemented with retrieval-quality and hardware validation still pending.

# Architecture rules that never change

- [ ] React never owns canonical project truth.
- [ ] React never opens SQLite directly.
- [ ] React never performs arbitrary project filesystem operations.
- [ ] Canonical mutations pass through the approved Rust application/store/registry boundary.
- [ ] Registry is an explicit allow-list.
- [ ] SQLite is canonical; search and AI indexes are derived/rebuildable.
- [ ] AI cannot directly mutate canonical state.
- [ ] AI proposals require human Change Set approval.
- [ ] AI cannot recursively grant itself mutation authority.
- [ ] Platform differences are hidden behind adapters/services.
- [ ] Windows/macOS preserve equivalent security properties.
- [ ] Retired LAN collaboration is not silently reintroduced.
- [ ] Generated bindings are reproducible.
- [ ] Forbidden dependency directions are mechanically checked.
- [ ] No known release may violate protected data, security, compatibility or recovery invariants.

The supplied vibe-coding guidance identifies semantic drift as the primary risk and calls for architecture checks, lifecycle tests, migration fixtures, import/export corpora, AI evaluation and release gates.

# Quality gates shared by all requirements

Each gate is defined once here. The Gates column of the requirement register names the gates a requirement must pass. For every gate, the evidence is a link in the evidence index to the automated result, fixture, artifact, review record or production drill.

## Architecture gate
**Acceptance:** Registry allow-list; Store::mutate; UI cannot directly own canonical state; generated bindings and dependency rules.

## Feature lifecycle gate
**Acceptance:** Discover → Use → Save → Undo → Recover/Reopen → Export/Package → Re-import/Reopen → Continue where applicable.

## Persistence and migration gate
**Acceptance:** Fresh, legacy, representative, large and interrupted fixtures; integrity checks; safe failure.

## Cross-platform gate
**Acceptance:** Windows 11 x64 and macOS on Apple silicon; same product and security semantics.

## Round-trip gate
**Acceptance:** Import preview; canonical export; malformed-input safety; semantic round trip; no silent loss.

## Security gate
**Acceptance:** Threat model; IPC; path confinement; parser bounds; secret hygiene; adversarial tests.

## Privacy gate
**Acceptance:** Data inventory; minimization; disclosures; redaction; external-service review.

## AI authority gate
**Acceptance:** Metadata; bounded tools; permission parity; human Change Sets; no direct canonical writes.

## AI evaluation gate
**Acceptance:** Fixed corpus; retrieval metrics; adversarial tests; runtime regression; hardware validation.

## Release artifact gate
**Acceptance:** Immutable version; checksum; signature; SBOM; license inventory; independent verification.

## Installer and update gate
**Acceptance:** Clean install; upgrade; uninstall/reinstall; authenticity; update failure recovery; channel behavior.

## Accessibility and UX gate
**Acceptance:** Keyboard/focus; reader semantics where applicable; DPI; IME; empty/loading/error/success states.

## Performance gate
**Acceptance:** Defined startup, open, search, large-project, export and AI budgets.

## Diagnostics gate
**Acceptance:** Safe logs; crash/error identity; diagnostic bundle; crash-loop recovery.

## Documentation gate
**Acceptance:** Manual; install; recovery; import/export; AI; troubleshooting; policies; release notes.

## Commercial consistency gate
**Acceptance:** Store ↔ website ↔ app ↔ entitlement ↔ email agree on price, trial, limits and refunds.

## Supportability gate
**Acceptance:** Self-service; reproducible support; escalation; non-founder dependency.

## Continuity gate
**Acceptance:** Backups; second admin; recovery drills; asset register; account-loss runbooks.

## Marketing truth gate
**Acceptance:** Every public claim supported by shipped behavior and evidence.

## Launch decision gate
**Acceptance:** Must-ship stop-ship items closed; exceptions formally accepted; evidence index complete; GO/NO-GO signed.

---

# Requirement register

Each requirement appears once. Do not create duplicate copies in feature documents. Use gate names for shared acceptance rules.

| Key | Requirement | Importance | Risk | Status | Gates | Evidence |
|---|---|---|---|---|---|---|
| `EXE-01-001` | React never owns canonical project truth | Must ship | High | Specified | Architecture, Feature lifecycle, Cross-platform, Security, AI authority, Launch decision | ☐ |
| `EXE-01-002` | React never opens SQLite directly | Must ship | High | Specified | Architecture, Feature lifecycle, Cross-platform, Security, AI authority, Launch decision | ☐ |
| `EXE-01-003` | React never writes arbitrary project files | Must ship | High | Specified | Architecture, Feature lifecycle, Cross-platform, Security, AI authority, Launch decision | ☐ |
| `EXE-01-004` | All canonical mutations pass through the approved Rust mutation pipeline | Must ship | High | Specified | Architecture, Feature lifecycle, Cross-platform, Security, AI authority, Launch decision | ☐ |
| `EXE-01-005` | Store::mutate or approved equivalent remains authoritative | Must ship | High | Specified | Architecture, Feature lifecycle, Cross-platform, Security, AI authority, Launch decision | ☐ |
| `EXE-01-006` | Registry is a strict allow-list | Must ship | High | Specified | Architecture, Feature lifecycle, Cross-platform, Security, AI authority, Launch decision | ☐ |
| `EXE-01-007` | Search is derived/rebuildable | Must ship | High | Specified | Architecture, Feature lifecycle, Cross-platform, Security, AI authority, Launch decision | ☐ |
| `EXE-01-008` | AI indexes are derived/rebuildable | Must ship | High | Specified | Architecture, Feature lifecycle, Cross-platform, Security, AI authority, Launch decision | ☐ |
| `EXE-01-009` | AI cannot directly mutate canonical state | Must ship | High | Specified | Architecture, Feature lifecycle, Cross-platform, Security, AI authority, Launch decision | ☐ |
| `EXE-01-010` | AI proposals require human Change Set approval | Must ship | High | Specified | Architecture, Feature lifecycle, Cross-platform, Security, AI authority, Launch decision | ☐ |
| `EXE-01-011` | AI cannot recursively/nestedly grant itself mutation authority | Must ship | High | Specified | Architecture, Feature lifecycle, Cross-platform, Security, AI authority, Launch decision | ☐ |
| `EXE-01-012` | Platform differences are hidden behind adapters | Must ship | High | Specified | Architecture, Feature lifecycle, Cross-platform, Security, AI authority, Launch decision | ☐ |
| `EXE-01-013` | No ADR-0007 LAN collaboration reintroduction | Must ship | High | Specified | Architecture, Feature lifecycle, Cross-platform, Security, AI authority, Launch decision | ☐ |
| `EXE-01-014` | Generated bindings are reproducible | Must ship | High | Specified | Architecture, Feature lifecycle, Cross-platform, Security, AI authority, Launch decision | ☐ |
| `EXE-01-015` | Forbidden dependency directions are mechanically checked | Must ship | High | Specified | Architecture, Feature lifecycle, Cross-platform, Security, AI authority, Launch decision | ☐ |
| `EXE-02-001` | Every schema change has a numbered migration | Must ship | Critical | Specified | Feature lifecycle, Persistence and migration, Cross-platform, Launch decision | ☐ |
| `EXE-02-002` | Migration is transactional | Must ship | Critical | Specified | Feature lifecycle, Persistence and migration, Cross-platform, Launch decision | ☐ |
| `EXE-02-003` | Migration has fresh-database fixture | Must ship | Critical | Specified | Feature lifecycle, Persistence and migration, Cross-platform, Launch decision | ☐ |
| `EXE-02-004` | Migration has representative legacy fixture | Must ship | Critical | Specified | Feature lifecycle, Persistence and migration, Cross-platform, Launch decision | ☐ |
| `EXE-02-005` | Migration has largest realistic fixture | Must ship | Critical | Specified | Feature lifecycle, Persistence and migration, Cross-platform, Launch decision | ☐ |
| `EXE-02-006` | Migration has interrupted-process fixture | Must ship | Critical | Specified | Feature lifecycle, Persistence and migration, Cross-platform, Launch decision | ☐ |
| `EXE-02-007` | Migration has disk-full fixture | Must ship | Critical | Specified | Feature lifecycle, Persistence and migration, Cross-platform, Launch decision | ☐ |
| `EXE-02-008` | Migration has permission-denied fixture | Must ship | Critical | Specified | Feature lifecycle, Persistence and migration, Cross-platform, Launch decision | ☐ |
| `EXE-02-009` | Migration has post-migration integrity_check | Must ship | Critical | Specified | Feature lifecycle, Persistence and migration, Cross-platform, Launch decision | ☐ |
| `EXE-02-010` | Migration has search-index rebuild test | Must ship | Critical | Specified | Feature lifecycle, Persistence and migration, Cross-platform, Launch decision | ☐ |
| `EXE-02-011` | Migration has AI-index rebuild test | Must ship | Critical | Specified | Feature lifecycle, Persistence and migration, Cross-platform, Launch decision | ☐ |
| `EXE-02-012` | Migration preserves undo/redo semantics | Must ship | Critical | Specified | Feature lifecycle, Persistence and migration, Cross-platform, Launch decision | ☐ |
| `EXE-02-013` | Migration preserves exportability | Must ship | Critical | Specified | Feature lifecycle, Persistence and migration, Cross-platform, Launch decision | ☐ |
| `EXE-02-014` | Migration has documented downgrade policy | Must ship | Critical | Specified | Feature lifecycle, Persistence and migration, Cross-platform, Launch decision | ☐ |
| `EXE-03-001` | Project format has explicit version | Must ship | High | Specified | Feature lifecycle, Persistence and migration, Cross-platform, Round trip, Security, Documentation, Launch decision | ☐ |
| `EXE-03-002` | Project has manifest | Must ship | High | Specified | Feature lifecycle, Persistence and migration, Cross-platform, Round trip, Security, Documentation, Launch decision | ☐ |
| `EXE-03-003` | Project records schema version | Must ship | High | Specified | Feature lifecycle, Persistence and migration, Cross-platform, Round trip, Security, Documentation, Launch decision | ☐ |
| `EXE-03-004` | Project records application compatibility | Must ship | High | Specified | Feature lifecycle, Persistence and migration, Cross-platform, Round trip, Security, Documentation, Launch decision | ☐ |
| `EXE-03-005` | Project package validates before import | Must ship | High | Specified | Feature lifecycle, Persistence and migration, Cross-platform, Round trip, Security, Documentation, Launch decision | ☐ |
| `EXE-03-006` | ZIP extraction rejects traversal | Must ship | High | Specified | Feature lifecycle, Persistence and migration, Cross-platform, Round trip, Security, Documentation, Launch decision | ☐ |
| `EXE-03-007` | Symlink escape is rejected | Must ship | High | Specified | Feature lifecycle, Persistence and migration, Cross-platform, Round trip, Security, Documentation, Launch decision | ☐ |
| `EXE-03-008` | Unicode paths are supported | Must ship | High | Specified | Feature lifecycle, Persistence and migration, Cross-platform, Round trip, Security, Documentation, Launch decision | ☐ |
| `EXE-03-009` | Long paths are tested | Must ship | High | Specified | Feature lifecycle, Persistence and migration, Cross-platform, Round trip, Security, Documentation, Launch decision | ☐ |
| `EXE-03-010` | Missing assets have deterministic behavior | Must ship | High | Specified | Feature lifecycle, Persistence and migration, Cross-platform, Round trip, Security, Documentation, Launch decision | ☐ |
| `EXE-03-011` | Windows-to-macOS round trip passes | Must ship | High | Specified | Feature lifecycle, Persistence and migration, Cross-platform, Round trip, Security, Documentation, Launch decision | ☐ |
| `EXE-03-012` | macOS-to-Windows round trip passes | Must ship | High | Specified | Feature lifecycle, Persistence and migration, Cross-platform, Round trip, Security, Documentation, Launch decision | ☐ |
| `EXE-03-013` | Customer can export important creative work | Must ship | High | Specified | Feature lifecycle, Persistence and migration, Cross-platform, Round trip, Security, Documentation, Launch decision | ☐ |
| `EXE-03-014` | Customer can recover after reinstall | Must ship | High | Specified | Feature lifecycle, Persistence and migration, Cross-platform, Round trip, Security, Documentation, Launch decision | ☐ |
| `EXE-03-015` | Customer can recover after machine replacement | Must ship | High | Specified | Feature lifecycle, Persistence and migration, Cross-platform, Round trip, Security, Documentation, Launch decision | ☐ |
| `EXE-04-001` | Element model is structured | Should ship | Medium | Specified | Feature lifecycle, Cross-platform, Round trip, Accessibility and UX, Performance, Documentation, Marketing truth, Launch decision | ☐ |
| `EXE-04-002` | Element progression is deterministic | Should ship | Medium | Specified | Feature lifecycle, Cross-platform, Round trip, Accessibility and UX, Performance, Documentation, Marketing truth, Launch decision | ☐ |
| `EXE-04-003` | Pagination is deterministic | Should ship | Medium | Specified | Feature lifecycle, Cross-platform, Round trip, Accessibility and UX, Performance, Documentation, Marketing truth, Launch decision | ☐ |
| `EXE-04-004` | Dialogue continuation is tested | Should ship | Medium | Specified | Feature lifecycle, Cross-platform, Round trip, Accessibility and UX, Performance, Documentation, Marketing truth, Launch decision | ☐ |
| `EXE-04-005` | Parenthetical behavior is tested | Should ship | Medium | Specified | Feature lifecycle, Cross-platform, Round trip, Accessibility and UX, Performance, Documentation, Marketing truth, Launch decision | ☐ |
| `EXE-04-006` | Scene numbering is tested | Should ship | Medium | Specified | Feature lifecycle, Cross-platform, Round trip, Accessibility and UX, Performance, Documentation, Marketing truth, Launch decision | ☐ |
| `EXE-04-007` | Revision colors are tested | Should ship | Medium | Specified | Feature lifecycle, Cross-platform, Round trip, Accessibility and UX, Performance, Documentation, Marketing truth, Launch decision | ☐ |
| `EXE-04-008` | Locked production draft is tested | Should ship | Medium | Specified | Feature lifecycle, Cross-platform, Round trip, Accessibility and UX, Performance, Documentation, Marketing truth, Launch decision | ☐ |
| `EXE-04-009` | Title page is tested | Should ship | Medium | Specified | Feature lifecycle, Cross-platform, Round trip, Accessibility and UX, Performance, Documentation, Marketing truth, Launch decision | ☐ |
| `EXE-04-010` | Find/replace is tested | Should ship | Medium | Specified | Feature lifecycle, Cross-platform, Round trip, Accessibility and UX, Performance, Documentation, Marketing truth, Launch decision | ☐ |
| `EXE-04-011` | Unicode is tested | Should ship | Medium | Specified | Feature lifecycle, Cross-platform, Round trip, Accessibility and UX, Performance, Documentation, Marketing truth, Launch decision | ☐ |
| `EXE-04-012` | Multilingual content is tested | Should ship | Medium | Specified | Feature lifecycle, Cross-platform, Round trip, Accessibility and UX, Performance, Documentation, Marketing truth, Launch decision | ☐ |
| `EXE-04-013` | FDX import preview is tested | Should ship | Medium | Specified | Feature lifecycle, Cross-platform, Round trip, Accessibility and UX, Performance, Documentation, Marketing truth, Launch decision | ☐ |
| `EXE-04-014` | FDX export is re-importable | Should ship | Medium | Specified | Feature lifecycle, Cross-platform, Round trip, Accessibility and UX, Performance, Documentation, Marketing truth, Launch decision | ☐ |
| `EXE-04-015` | Unknown constructs are preserved/mapped/warned | Should ship | Medium | Specified | Feature lifecycle, Cross-platform, Round trip, Accessibility and UX, Performance, Documentation, Marketing truth, Launch decision | ☐ |
| `EXE-04-016` | Fountain round trip is tested | Should ship | Medium | Specified | Feature lifecycle, Cross-platform, Round trip, Accessibility and UX, Performance, Documentation, Marketing truth, Launch decision | ☐ |
| `EXE-04-017` | DOCX compatibility is documented | Should ship | Medium | Specified | Feature lifecycle, Cross-platform, Round trip, Accessibility and UX, Performance, Documentation, Marketing truth, Launch decision | ☐ |
| `EXE-04-018` | PDF import is explicitly best-effort | Should ship | Medium | Specified | Feature lifecycle, Cross-platform, Round trip, Accessibility and UX, Performance, Documentation, Marketing truth, Launch decision | ☐ |
| `EXE-04-019` | PDF export visual regression is automated | Should ship | Medium | Specified | Feature lifecycle, Cross-platform, Round trip, Accessibility and UX, Performance, Documentation, Marketing truth, Launch decision | ☐ |
| `EXE-04-020` | Screenplay exports remain unbranded | Should ship | Medium | Specified | Feature lifecycle, Cross-platform, Round trip, Accessibility and UX, Performance, Documentation, Marketing truth, Launch decision | ☐ |
| `EXE-05-001` | Malformed ZIP cannot escape destination | Should ship | Medium | Specified | Cross-platform, Round trip, Security, Launch decision | ☐ |
| `EXE-05-002` | Malformed XML cannot crash process unsafely | Should ship | Medium | Specified | Cross-platform, Round trip, Security, Launch decision | ☐ |
| `EXE-05-003` | Malformed PDF is safely rejected or degraded | Should ship | Medium | Specified | Cross-platform, Round trip, Security, Launch decision | ☐ |
| `EXE-05-004` | Malformed DOCX is safely rejected or degraded | Should ship | Medium | Specified | Cross-platform, Round trip, Security, Launch decision | ☐ |
| `EXE-05-005` | Oversized files are bounded | Should ship | Medium | Specified | Cross-platform, Round trip, Security, Launch decision | ☐ |
| `EXE-05-006` | Decompression bombs are bounded | Should ship | Medium | Specified | Cross-platform, Round trip, Security, Launch decision | ☐ |
| `EXE-05-007` | Import is transactional | Should ship | Medium | Specified | Cross-platform, Round trip, Security, Launch decision | ☐ |
| `EXE-05-008` | Import preview exposes warnings | Should ship | Medium | Specified | Cross-platform, Round trip, Security, Launch decision | ☐ |
| `EXE-05-009` | Import preserves original source | Should ship | Medium | Specified | Cross-platform, Round trip, Security, Launch decision | ☐ |
| `EXE-05-010` | Export uses canonical state | Should ship | Medium | Specified | Cross-platform, Round trip, Security, Launch decision | ☐ |
| `EXE-05-011` | Export failure cannot mutate canonical state | Should ship | Medium | Specified | Cross-platform, Round trip, Security, Launch decision | ☐ |
| `EXE-05-012` | Partial output is cleaned up | Should ship | Medium | Specified | Cross-platform, Round trip, Security, Launch decision | ☐ |
| `EXE-05-013` | Existing destination behavior is explicit | Should ship | Medium | Specified | Cross-platform, Round trip, Security, Launch decision | ☐ |
| `EXE-05-014` | Unicode destination paths work | Should ship | Medium | Specified | Cross-platform, Round trip, Security, Launch decision | ☐ |
| `EXE-05-015` | Permission failure is recoverable | Should ship | Medium | Specified | Cross-platform, Round trip, Security, Launch decision | ☐ |
| `EXE-06-001` | Supported Windows versions are documented | Must ship | High | Specified | Cross-platform, Security, Release artifact, Installer and update, Accessibility and UX, Performance, Diagnostics, Documentation, Launch decision | ☐ |
| `EXE-06-002` | Clean install tested | Must ship | High | Specified | Cross-platform, Security, Release artifact, Installer and update, Accessibility and UX, Performance, Diagnostics, Documentation, Launch decision | ☐ |
| `EXE-06-003` | Upgrade tested | Must ship | High | Specified | Cross-platform, Security, Release artifact, Installer and update, Accessibility and UX, Performance, Diagnostics, Documentation, Launch decision | ☐ |
| `EXE-06-004` | Uninstall tested | Must ship | High | Specified | Cross-platform, Security, Release artifact, Installer and update, Accessibility and UX, Performance, Diagnostics, Documentation, Launch decision | ☐ |
| `EXE-06-005` | Reinstall tested | Must ship | High | Specified | Cross-platform, Security, Release artifact, Installer and update, Accessibility and UX, Performance, Diagnostics, Documentation, Launch decision | ☐ |
| `EXE-06-006` | WebView2 behavior verified | Must ship | High | Specified | Cross-platform, Security, Release artifact, Installer and update, Accessibility and UX, Performance, Diagnostics, Documentation, Launch decision | ☐ |
| `EXE-06-007` | AuthentiCode signature verified | Must ship | High | Specified | Cross-platform, Security, Release artifact, Installer and update, Accessibility and UX, Performance, Diagnostics, Documentation, Launch decision | ☐ |
| `EXE-06-008` | Timestamp verified | Must ship | High | Specified | Cross-platform, Security, Release artifact, Installer and update, Accessibility and UX, Performance, Diagnostics, Documentation, Launch decision | ☐ |
| `EXE-06-009` | SmartScreen behavior tested | Must ship | High | Specified | Cross-platform, Security, Release artifact, Installer and update, Accessibility and UX, Performance, Diagnostics, Documentation, Launch decision | ☐ |
| `EXE-06-010` | Windows Defender interaction tested | Must ship | High | Specified | Cross-platform, Security, Release artifact, Installer and update, Accessibility and UX, Performance, Diagnostics, Documentation, Launch decision | ☐ |
| `EXE-06-011` | High DPI tested | Must ship | High | Specified | Cross-platform, Security, Release artifact, Installer and update, Accessibility and UX, Performance, Diagnostics, Documentation, Launch decision | ☐ |
| `EXE-06-012` | Multiple monitors tested | Must ship | High | Specified | Cross-platform, Security, Release artifact, Installer and update, Accessibility and UX, Performance, Diagnostics, Documentation, Launch decision | ☐ |
| `EXE-06-013` | Unicode/IME tested | Must ship | High | Specified | Cross-platform, Security, Release artifact, Installer and update, Accessibility and UX, Performance, Diagnostics, Documentation, Launch decision | ☐ |
| `EXE-06-014` | Long paths tested | Must ship | High | Specified | Cross-platform, Security, Release artifact, Installer and update, Accessibility and UX, Performance, Diagnostics, Documentation, Launch decision | ☐ |
| `EXE-06-015` | External drive removal tested | Must ship | High | Specified | Cross-platform, Security, Release artifact, Installer and update, Accessibility and UX, Performance, Diagnostics, Documentation, Launch decision | ☐ |
| `EXE-06-016` | Read-only folder tested | Must ship | High | Specified | Cross-platform, Security, Release artifact, Installer and update, Accessibility and UX, Performance, Diagnostics, Documentation, Launch decision | ☐ |
| `EXE-06-017` | Network-share behavior explicitly defined | Must ship | High | Specified | Cross-platform, Security, Release artifact, Installer and update, Accessibility and UX, Performance, Diagnostics, Documentation, Launch decision | ☐ |
| `EXE-06-018` | Sleep/wake tested | Must ship | High | Specified | Cross-platform, Security, Release artifact, Installer and update, Accessibility and UX, Performance, Diagnostics, Documentation, Launch decision | ☐ |
| `EXE-06-019` | AI sidecar process cleanup tested | Must ship | High | Specified | Cross-platform, Security, Release artifact, Installer and update, Accessibility and UX, Performance, Diagnostics, Documentation, Launch decision | ☐ |
| `EXE-06-020` | Offline mode tested | Must ship | High | Specified | Cross-platform, Security, Release artifact, Installer and update, Accessibility and UX, Performance, Diagnostics, Documentation, Launch decision | ☐ |
| `EXE-07-001` | Supported macOS versions are documented | Must ship | High | Specified | Cross-platform, Security, Release artifact, Installer and update, Accessibility and UX, Performance, Diagnostics, Documentation, Launch decision | ☐ |
| `EXE-07-002` | Apple Silicon tested | Must ship | High | Specified | Cross-platform, Security, Release artifact, Installer and update, Accessibility and UX, Performance, Diagnostics, Documentation, Launch decision | ☐ |
| `EXE-07-003` | Intel Macs are not supported and the Mac package declares Apple silicon only | Must ship | High | Specified | Cross-platform, Security, Release artifact, Installer and update, Accessibility and UX, Performance, Diagnostics, Documentation, Launch decision | ☐ |
| `EXE-07-004` | Apple Distribution and Mac Installer Distribution signing verified on the package | Must ship | High | Specified | Cross-platform, Security, Release artifact, Installer and update, Accessibility and UX, Performance, Diagnostics, Documentation, Launch decision | ☐ |
| `EXE-07-005` | App Sandbox enabled, and the AI engine helper signed with the inherited sandbox entitlement | Must ship | High | Specified | Cross-platform, Security, Release artifact, Installer and update, Accessibility and UX, Performance, Diagnostics, Documentation, Launch decision | ☐ |
| `EXE-07-006` | Entitlements minimized | Must ship | High | Specified | Cross-platform, Security, Release artifact, Installer and update, Accessibility and UX, Performance, Diagnostics, Documentation, Launch decision | ☐ |
| `EXE-07-007` | Secure timestamp verified | Must ship | High | Specified | Cross-platform, Security, Release artifact, Installer and update, Accessibility and UX, Performance, Diagnostics, Documentation, Launch decision | ☐ |
| `EXE-07-008` | Package passes Apple's validation and the App Store Connect upload checks | Must ship | High | Specified | Cross-platform, Security, Release artifact, Installer and update, Accessibility and UX, Performance, Diagnostics, Documentation, Launch decision | ☐ |
| `EXE-07-009` | The TestFlight build installs and runs | Must ship | High | Specified | Cross-platform, Security, Release artifact, Installer and update, Accessibility and UX, Performance, Diagnostics, Documentation, Launch decision | ☐ |
| `EXE-07-010` | Clean-machine install from TestFlight tested | Must ship | High | Specified | Cross-platform, Security, Release artifact, Installer and update, Accessibility and UX, Performance, Diagnostics, Documentation, Launch decision | ☐ |
| `EXE-07-011` | Project folder access through security-scoped bookmarks survives relaunch | Must ship | High | Specified | Cross-platform, Security, Release artifact, Installer and update, Accessibility and UX, Performance, Diagnostics, Documentation, Launch decision | ☐ |
| `EXE-07-012` | Mac App Store package (.pkg) tested | Must ship | High | Specified | Cross-platform, Security, Release artifact, Installer and update, Accessibility and UX, Performance, Diagnostics, Documentation, Launch decision | ☐ |
| `EXE-07-013` | External drives tested | Must ship | High | Specified | Cross-platform, Security, Release artifact, Installer and update, Accessibility and UX, Performance, Diagnostics, Documentation, Launch decision | ☐ |
| `EXE-07-014` | File permissions tested | Must ship | High | Specified | Cross-platform, Security, Release artifact, Installer and update, Accessibility and UX, Performance, Diagnostics, Documentation, Launch decision | ☐ |
| `EXE-07-015` | Sleep/wake tested | Must ship | High | Specified | Cross-platform, Security, Release artifact, Installer and update, Accessibility and UX, Performance, Diagnostics, Documentation, Launch decision | ☐ |
| `EXE-07-016` | Unicode/case-sensitivity assumptions tested | Must ship | High | Specified | Cross-platform, Security, Release artifact, Installer and update, Accessibility and UX, Performance, Diagnostics, Documentation, Launch decision | ☐ |
| `EXE-07-017` | AI sidecar lifecycle tested | Must ship | High | Specified | Cross-platform, Security, Release artifact, Installer and update, Accessibility and UX, Performance, Diagnostics, Documentation, Launch decision | ☐ |
| `EXE-07-018` | Updater signing tested | Must ship | High | Specified | Cross-platform, Security, Release artifact, Installer and update, Accessibility and UX, Performance, Diagnostics, Documentation, Launch decision | ☐ |
| `EXE-07-019` | Clean-machine release test completed | Must ship | High | Specified | Cross-platform, Security, Release artifact, Installer and update, Accessibility and UX, Performance, Diagnostics, Documentation, Launch decision | ☐ |
| `EXE-07-020` | Developer account recovery documented | Must ship | High | Specified | Cross-platform, Security, Release artifact, Installer and update, Accessibility and UX, Performance, Diagnostics, Documentation, Launch decision | ☐ |
| `EXE-08-001` | Partner Center company ownership verified | Must ship | High | Specified | Release artifact, Installer and update, Documentation, Commercial consistency, Marketing truth, Launch decision | ☐ |
| `EXE-08-002` | Backup administrator exists | Must ship | High | Specified | Release artifact, Installer and update, Documentation, Commercial consistency, Marketing truth, Launch decision | ☐ |
| `EXE-08-003` | Publisher identity verified | Must ship | High | Specified | Release artifact, Installer and update, Documentation, Commercial consistency, Marketing truth, Launch decision | ☐ |
| `EXE-08-004` | Package identity verified | Must ship | High | Specified | Release artifact, Installer and update, Documentation, Commercial consistency, Marketing truth, Launch decision | ☐ |
| `EXE-08-005` | Store listing is complete | Must ship | High | Specified | Release artifact, Installer and update, Documentation, Commercial consistency, Marketing truth, Launch decision | ☐ |
| `EXE-08-006` | Screenshots match build | Must ship | High | Specified | Release artifact, Installer and update, Documentation, Commercial consistency, Marketing truth, Launch decision | ☐ |
| `EXE-08-007` | Description matches build | Must ship | High | Specified | Release artifact, Installer and update, Documentation, Commercial consistency, Marketing truth, Launch decision | ☐ |
| `EXE-08-008` | Privacy URL works | Must ship | High | Specified | Release artifact, Installer and update, Documentation, Commercial consistency, Marketing truth, Launch decision | ☐ |
| `EXE-08-009` | Support URL works | Must ship | High | Specified | Release artifact, Installer and update, Documentation, Commercial consistency, Marketing truth, Launch decision | ☐ |
| `EXE-08-010` | Age rating completed | Must ship | High | Specified | Release artifact, Installer and update, Documentation, Commercial consistency, Marketing truth, Launch decision | ☐ |
| `EXE-08-011` | Category completed | Must ship | High | Specified | Release artifact, Installer and update, Documentation, Commercial consistency, Marketing truth, Launch decision | ☐ |
| `EXE-08-012` | Pricing verified | Must ship | High | Specified | Release artifact, Installer and update, Documentation, Commercial consistency, Marketing truth, Launch decision | ☐ |
| `EXE-08-013` | Regional availability verified | Must ship | High | Specified | Release artifact, Installer and update, Documentation, Commercial consistency, Marketing truth, Launch decision | ☐ |
| `EXE-08-014` | Trial configuration verified | Must ship | High | Specified | Release artifact, Installer and update, Documentation, Commercial consistency, Marketing truth, Launch decision | ☐ |
| `EXE-08-015` | Certification notes prepared | Must ship | High | Specified | Release artifact, Installer and update, Documentation, Commercial consistency, Marketing truth, Launch decision | ☐ |
| `EXE-08-016` | Windows App Certification Kit run where applicable | Must ship | High | Specified | Release artifact, Installer and update, Documentation, Commercial consistency, Marketing truth, Launch decision | ☐ |
| `EXE-08-017` | Clean Store install tested | Must ship | High | Specified | Release artifact, Installer and update, Documentation, Commercial consistency, Marketing truth, Launch decision | ☐ |
| `EXE-08-018` | Store update tested | Must ship | High | Specified | Release artifact, Installer and update, Documentation, Commercial consistency, Marketing truth, Launch decision | ☐ |
| `EXE-08-019` | Store uninstall/reinstall tested | Must ship | High | Specified | Release artifact, Installer and update, Documentation, Commercial consistency, Marketing truth, Launch decision | ☐ |
| `EXE-08-020` | Certification evidence archived | Must ship | High | Specified | Release artifact, Installer and update, Documentation, Commercial consistency, Marketing truth, Launch decision | ☐ |
| `EXE-09-001` | 7-day trial starts correctly | Must ship | High | Specified | Privacy, Documentation, Commercial consistency, Supportability, Marketing truth, Launch decision | ☐ |
| `EXE-09-002` | No credit card is required | Must ship | High | Specified | Privacy, Documentation, Commercial consistency, Supportability, Marketing truth, Launch decision | ☐ |
| `EXE-09-003` | Trial expiry is deterministic | Must ship | High | Specified | Privacy, Documentation, Commercial consistency, Supportability, Marketing truth, Launch decision | ☐ |
| `EXE-09-004` | Offline trial behavior is defined | Must ship | High | Specified | Privacy, Documentation, Commercial consistency, Supportability, Marketing truth, Launch decision | ☐ |
| `EXE-09-005` | Reinstall behavior is defined | Must ship | High | Specified | Privacy, Documentation, Commercial consistency, Supportability, Marketing truth, Launch decision | ☐ |
| `EXE-09-006` | Clock tampering behavior is defined | Must ship | High | Specified | Privacy, Documentation, Commercial consistency, Supportability, Marketing truth, Launch decision | ☐ |
| `EXE-09-007` | Purchase during trial works | Must ship | High | Specified | Privacy, Documentation, Commercial consistency, Supportability, Marketing truth, Launch decision | ☐ |
| `EXE-09-008` | Entitlement refresh works | Must ship | High | Specified | Privacy, Documentation, Commercial consistency, Supportability, Marketing truth, Launch decision | ☐ |
| `EXE-09-009` | Restore purchase works | Must ship | High | Specified | Privacy, Documentation, Commercial consistency, Supportability, Marketing truth, Launch decision | ☐ |
| `EXE-09-010` | Refund path is documented | Must ship | High | Specified | Privacy, Documentation, Commercial consistency, Supportability, Marketing truth, Launch decision | ☐ |
| `EXE-09-011` | Cancellation path is documented | Must ship | High | Specified | Privacy, Documentation, Commercial consistency, Supportability, Marketing truth, Launch decision | ☐ |
| `EXE-09-012` | Store outage behavior is defined | Must ship | High | Specified | Privacy, Documentation, Commercial consistency, Supportability, Marketing truth, Launch decision | ☐ |
| `EXE-09-013` | Pricing is reconciled with finance | Must ship | High | Specified | Privacy, Documentation, Commercial consistency, Supportability, Marketing truth, Launch decision | ☐ |
| `EXE-09-014` | Discount expiry has owner | Must ship | High | Specified | Privacy, Documentation, Commercial consistency, Supportability, Marketing truth, Launch decision | ☐ |
| `EXE-09-015` | Commercial claims match actual flow | Must ship | High | Specified | Privacy, Documentation, Commercial consistency, Supportability, Marketing truth, Launch decision | ☐ |
| `EXE-10-001` | Threat model exists | Must ship | Critical | Specified | Security, Privacy, Release artifact, Installer and update, Diagnostics, Continuity, Launch decision | ☐ |
| `EXE-10-002` | IPC allow-list audited | Must ship | Critical | Specified | Security, Privacy, Release artifact, Installer and update, Diagnostics, Continuity, Launch decision | ☐ |
| `EXE-10-003` | Filesystem scope audited | Must ship | Critical | Specified | Security, Privacy, Release artifact, Installer and update, Diagnostics, Continuity, Launch decision | ☐ |
| `EXE-10-004` | Path traversal rejected | Must ship | Critical | Specified | Security, Privacy, Release artifact, Installer and update, Diagnostics, Continuity, Launch decision | ☐ |
| `EXE-10-005` | Symlink escapes rejected | Must ship | Critical | Specified | Security, Privacy, Release artifact, Installer and update, Diagnostics, Continuity, Launch decision | ☐ |
| `EXE-10-006` | Command injection tested | Must ship | Critical | Specified | Security, Privacy, Release artifact, Installer and update, Diagnostics, Continuity, Launch decision | ☐ |
| `EXE-10-007` | SQL injection tested | Must ship | Critical | Specified | Security, Privacy, Release artifact, Installer and update, Diagnostics, Continuity, Launch decision | ☐ |
| `EXE-10-008` | URL handling tested | Must ship | Critical | Specified | Security, Privacy, Release artifact, Installer and update, Diagnostics, Continuity, Launch decision | ☐ |
| `EXE-10-009` | XML parser tested | Must ship | Critical | Specified | Security, Privacy, Release artifact, Installer and update, Diagnostics, Continuity, Launch decision | ☐ |
| `EXE-10-010` | ZIP extraction tested | Must ship | Critical | Specified | Security, Privacy, Release artifact, Installer and update, Diagnostics, Continuity, Launch decision | ☐ |
| `EXE-10-011` | PDF parser tested | Must ship | Critical | Specified | Security, Privacy, Release artifact, Installer and update, Diagnostics, Continuity, Launch decision | ☐ |
| `EXE-10-012` | DOCX parser tested | Must ship | Critical | Specified | Security, Privacy, Release artifact, Installer and update, Diagnostics, Continuity, Launch decision | ☐ |
| `EXE-10-013` | Fuzzing corpus exists | Must ship | Critical | Specified | Security, Privacy, Release artifact, Installer and update, Diagnostics, Continuity, Launch decision | ☐ |
| `EXE-10-014` | Secret scan is clean | Must ship | Critical | Specified | Security, Privacy, Release artifact, Installer and update, Diagnostics, Continuity, Launch decision | ☐ |
| `EXE-10-015` | Dependency scan reviewed | Must ship | Critical | Specified | Security, Privacy, Release artifact, Installer and update, Diagnostics, Continuity, Launch decision | ☐ |
| `EXE-10-016` | License scan reviewed | Must ship | Critical | Specified | Security, Privacy, Release artifact, Installer and update, Diagnostics, Continuity, Launch decision | ☐ |
| `EXE-10-017` | SBOM generated | Must ship | Critical | Specified | Security, Privacy, Release artifact, Installer and update, Diagnostics, Continuity, Launch decision | ☐ |
| `EXE-10-018` | Signing keys protected | Must ship | Critical | Specified | Security, Privacy, Release artifact, Installer and update, Diagnostics, Continuity, Launch decision | ☐ |
| `EXE-10-019` | Updater signatures verified | Must ship | Critical | Specified | Security, Privacy, Release artifact, Installer and update, Diagnostics, Continuity, Launch decision | ☐ |
| `EXE-10-020` | Security disclosure channel tested | Must ship | Critical | Specified | Security, Privacy, Release artifact, Installer and update, Diagnostics, Continuity, Launch decision | ☐ |
| `EXE-11-001` | Model license recorded | Must ship | High | Specified | Cross-platform, Security, Privacy, AI authority, AI evaluation, Performance, Diagnostics, Documentation, Launch decision | ☐ |
| `EXE-11-002` | Model checksum recorded | Must ship | High | Specified | Cross-platform, Security, Privacy, AI authority, AI evaluation, Performance, Diagnostics, Documentation, Launch decision | ☐ |
| `EXE-11-003` | Embedding license recorded | Must ship | High | Specified | Cross-platform, Security, Privacy, AI authority, AI evaluation, Performance, Diagnostics, Documentation, Launch decision | ☐ |
| `EXE-11-004` | Embedding checksum recorded | Must ship | High | Specified | Cross-platform, Security, Privacy, AI authority, AI evaluation, Performance, Diagnostics, Documentation, Launch decision | ☐ |
| `EXE-11-005` | Model download integrity verified | Must ship | High | Specified | Cross-platform, Security, Privacy, AI authority, AI evaluation, Performance, Diagnostics, Documentation, Launch decision | ☐ |
| `EXE-11-006` | Interrupted download tested | Must ship | High | Specified | Cross-platform, Security, Privacy, AI authority, AI evaluation, Performance, Diagnostics, Documentation, Launch decision | ☐ |
| `EXE-11-007` | Missing model handled | Must ship | High | Specified | Cross-platform, Security, Privacy, AI authority, AI evaluation, Performance, Diagnostics, Documentation, Launch decision | ☐ |
| `EXE-11-008` | Minimum hardware documented | Must ship | High | Specified | Cross-platform, Security, Privacy, AI authority, AI evaluation, Performance, Diagnostics, Documentation, Launch decision | ☐ |
| `EXE-11-009` | CPU fallback tested | Must ship | High | Specified | Cross-platform, Security, Privacy, AI authority, AI evaluation, Performance, Diagnostics, Documentation, Launch decision | ☐ |
| `EXE-11-010` | Vulkan path tested where supported | Must ship | High | Specified | Cross-platform, Security, Privacy, AI authority, AI evaluation, Performance, Diagnostics, Documentation, Launch decision | ☐ |
| `EXE-11-011` | Sidecar crash handled | Must ship | High | Specified | Cross-platform, Security, Privacy, AI authority, AI evaluation, Performance, Diagnostics, Documentation, Launch decision | ☐ |
| `EXE-11-012` | Sidecar hang handled | Must ship | High | Specified | Cross-platform, Security, Privacy, AI authority, AI evaluation, Performance, Diagnostics, Documentation, Launch decision | ☐ |
| `EXE-11-013` | Timeout/cancellation tested | Must ship | High | Specified | Cross-platform, Security, Privacy, AI authority, AI evaluation, Performance, Diagnostics, Documentation, Launch decision | ☐ |
| `EXE-11-014` | Tool allow-list audited | Must ship | High | Specified | Cross-platform, Security, Privacy, AI authority, AI evaluation, Performance, Diagnostics, Documentation, Launch decision | ☐ |
| `EXE-11-015` | Destructive operations classified | Must ship | High | Specified | Cross-platform, Security, Privacy, AI authority, AI evaluation, Performance, Diagnostics, Documentation, Launch decision | ☐ |
| `EXE-11-016` | Human Change Set boundary tested | Must ship | High | Specified | Cross-platform, Security, Privacy, AI authority, AI evaluation, Performance, Diagnostics, Documentation, Launch decision | ☐ |
| `EXE-11-017` | Nested AI mutation rejected | Must ship | High | Specified | Cross-platform, Security, Privacy, AI authority, AI evaluation, Performance, Diagnostics, Documentation, Launch decision | ☐ |
| `EXE-11-018` | Prompt injection corpus exists | Must ship | High | Specified | Cross-platform, Security, Privacy, AI authority, AI evaluation, Performance, Diagnostics, Documentation, Launch decision | ☐ |
| `EXE-11-019` | Retrieval poisoning corpus exists | Must ship | High | Specified | Cross-platform, Security, Privacy, AI authority, AI evaluation, Performance, Diagnostics, Documentation, Launch decision | ☐ |
| `EXE-11-020` | Recall@K measured | Must ship | High | Specified | Cross-platform, Security, Privacy, AI authority, AI evaluation, Performance, Diagnostics, Documentation, Launch decision | ☐ |
| `EXE-11-021` | MRR measured | Must ship | High | Specified | Cross-platform, Security, Privacy, AI authority, AI evaluation, Performance, Diagnostics, Documentation, Launch decision | ☐ |
| `EXE-11-022` | Offline/no-cloud-fallback claim verified | Must ship | High | Specified | Cross-platform, Security, Privacy, AI authority, AI evaluation, Performance, Diagnostics, Documentation, Launch decision | ☐ |
| `EXE-11-023` | AI privacy claim verified | Must ship | High | Specified | Cross-platform, Security, Privacy, AI authority, AI evaluation, Performance, Diagnostics, Documentation, Launch decision | ☐ |
| `EXE-11-024` | Model update regression suite exists | Must ship | High | Specified | Cross-platform, Security, Privacy, AI authority, AI evaluation, Performance, Diagnostics, Documentation, Launch decision | ☐ |
| `EXE-12-001` | Data inventory exists | Must ship | Medium | Specified | Privacy, Documentation, Commercial consistency, Marketing truth, Launch decision | ☐ |
| `EXE-12-002` | Personal-data inventory exists | Must ship | Medium | Specified | Privacy, Documentation, Commercial consistency, Marketing truth, Launch decision | ☐ |
| `EXE-12-003` | Data-flow map exists | Must ship | Medium | Specified | Privacy, Documentation, Commercial consistency, Marketing truth, Launch decision | ☐ |
| `EXE-12-004` | Retention is defined | Must ship | Medium | Specified | Privacy, Documentation, Commercial consistency, Marketing truth, Launch decision | ☐ |
| `EXE-12-005` | Deletion process exists | Must ship | Medium | Specified | Privacy, Documentation, Commercial consistency, Marketing truth, Launch decision | ☐ |
| `EXE-12-006` | Access/request process exists | Must ship | Medium | Specified | Privacy, Documentation, Commercial consistency, Marketing truth, Launch decision | ☐ |
| `EXE-12-007` | Correction process exists | Must ship | Medium | Specified | Privacy, Documentation, Commercial consistency, Marketing truth, Launch decision | ☐ |
| `EXE-12-008` | Portability/export process exists | Must ship | Medium | Specified | Privacy, Documentation, Commercial consistency, Marketing truth, Launch decision | ☐ |
| `EXE-12-009` | Subprocessor inventory exists | Must ship | Medium | Specified | Privacy, Documentation, Commercial consistency, Marketing truth, Launch decision | ☐ |
| `EXE-12-010` | Privacy policy matches implementation | Must ship | Medium | Specified | Privacy, Documentation, Commercial consistency, Marketing truth, Launch decision | ☐ |
| `EXE-12-011` | Terms match implementation | Must ship | Medium | Specified | Privacy, Documentation, Commercial consistency, Marketing truth, Launch decision | ☐ |
| `EXE-12-012` | Refund policy matches commerce | Must ship | Medium | Specified | Privacy, Documentation, Commercial consistency, Marketing truth, Launch decision | ☐ |
| `EXE-12-013` | EULA matches licensing | Must ship | Medium | Specified | Privacy, Documentation, Commercial consistency, Marketing truth, Launch decision | ☐ |
| `EXE-12-014` | Third-party notices are complete | Must ship | Medium | Specified | Privacy, Documentation, Commercial consistency, Marketing truth, Launch decision | ☐ |
| `EXE-12-015` | Open-source licenses are archived | Must ship | Medium | Specified | Privacy, Documentation, Commercial consistency, Marketing truth, Launch decision | ☐ |
| `EXE-12-016` | Model licenses are archived | Must ship | Medium | Specified | Privacy, Documentation, Commercial consistency, Marketing truth, Launch decision | ☐ |
| `EXE-12-017` | Security contact is published | Must ship | Medium | Specified | Privacy, Documentation, Commercial consistency, Marketing truth, Launch decision | ☐ |
| `EXE-12-018` | Qualified counsel review recorded | Must ship | Medium | Specified | Privacy, Documentation, Commercial consistency, Marketing truth, Launch decision | ☐ |
| `EXE-13-001` | Crash IDs are generated | Should ship | Medium | Specified | Privacy, Diagnostics, Documentation, Supportability, Launch decision | ☐ |
| `EXE-13-002` | OS/app/architecture context is recorded | Should ship | Medium | Specified | Privacy, Diagnostics, Documentation, Supportability, Launch decision | ☐ |
| `EXE-13-003` | Sensitive creative content is excluded by default | Should ship | Medium | Specified | Privacy, Diagnostics, Documentation, Supportability, Launch decision | ☐ |
| `EXE-13-004` | Support diagnostic bundle is inspectable | Should ship | Medium | Specified | Privacy, Diagnostics, Documentation, Supportability, Launch decision | ☐ |
| `EXE-13-005` | Secrets are excluded | Should ship | Medium | Specified | Privacy, Diagnostics, Documentation, Supportability, Launch decision | ☐ |
| `EXE-13-006` | Crash-loop protection exists | Should ship | Medium | Specified | Privacy, Diagnostics, Documentation, Supportability, Launch decision | ☐ |
| `EXE-13-007` | Support email is company-owned | Should ship | Medium | Specified | Privacy, Diagnostics, Documentation, Supportability, Launch decision | ☐ |
| `EXE-13-008` | Knowledge base exists | Should ship | Medium | Specified | Privacy, Diagnostics, Documentation, Supportability, Launch decision | ☐ |
| `EXE-13-009` | Installation guide exists | Should ship | Medium | Specified | Privacy, Diagnostics, Documentation, Supportability, Launch decision | ☐ |
| `EXE-13-010` | Recovery guide exists | Should ship | Medium | Specified | Privacy, Diagnostics, Documentation, Supportability, Launch decision | ☐ |
| `EXE-13-011` | Import/export troubleshooting exists | Should ship | Medium | Specified | Privacy, Diagnostics, Documentation, Supportability, Launch decision | ☐ |
| `EXE-13-012` | AI troubleshooting exists | Should ship | Medium | Specified | Privacy, Diagnostics, Documentation, Supportability, Launch decision | ☐ |
| `EXE-13-013` | Windows troubleshooting exists | Should ship | Medium | Specified | Privacy, Diagnostics, Documentation, Supportability, Launch decision | ☐ |
| `EXE-13-014` | macOS troubleshooting exists | Should ship | Medium | Specified | Privacy, Diagnostics, Documentation, Supportability, Launch decision | ☐ |
| `EXE-13-015` | Escalation levels exist | Should ship | Medium | Specified | Privacy, Diagnostics, Documentation, Supportability, Launch decision | ☐ |
| `EXE-13-016` | Second support operator can run the workflow | Should ship | Medium | Specified | Privacy, Diagnostics, Documentation, Supportability, Launch decision | ☐ |
| `EXE-14-001` | Domain is company-owned | Should ship | Medium | Specified | Privacy, Documentation, Commercial consistency, Supportability, Marketing truth, Launch decision | ☐ |
| `EXE-14-002` | Registrar recovery is documented | Should ship | Medium | Specified | Privacy, Documentation, Commercial consistency, Supportability, Marketing truth, Launch decision | ☐ |
| `EXE-14-003` | DNS recovery is documented | Should ship | Medium | Specified | Privacy, Documentation, Commercial consistency, Supportability, Marketing truth, Launch decision | ☐ |
| `EXE-14-004` | HTTPS works | Should ship | Medium | Specified | Privacy, Documentation, Commercial consistency, Supportability, Marketing truth, Launch decision | ☐ |
| `EXE-14-005` | SPF works | Should ship | Medium | Specified | Privacy, Documentation, Commercial consistency, Supportability, Marketing truth, Launch decision | ☐ |
| `EXE-14-006` | DKIM works | Should ship | Medium | Specified | Privacy, Documentation, Commercial consistency, Supportability, Marketing truth, Launch decision | ☐ |
| `EXE-14-007` | DMARC policy is deliberate | Should ship | Medium | Specified | Privacy, Documentation, Commercial consistency, Supportability, Marketing truth, Launch decision | ☐ |
| `EXE-14-008` | Privacy page works | Should ship | Medium | Specified | Privacy, Documentation, Commercial consistency, Supportability, Marketing truth, Launch decision | ☐ |
| `EXE-14-009` | Terms page works | Should ship | Medium | Specified | Privacy, Documentation, Commercial consistency, Supportability, Marketing truth, Launch decision | ☐ |
| `EXE-14-010` | Refund page works | Should ship | Medium | Specified | Privacy, Documentation, Commercial consistency, Supportability, Marketing truth, Launch decision | ☐ |
| `EXE-14-011` | Support page works | Should ship | Medium | Specified | Privacy, Documentation, Commercial consistency, Supportability, Marketing truth, Launch decision | ☐ |
| `EXE-14-012` | Security disclosure works | Should ship | Medium | Specified | Privacy, Documentation, Commercial consistency, Supportability, Marketing truth, Launch decision | ☐ |
| `EXE-14-013` | Downloads are current | Should ship | Medium | Specified | Privacy, Documentation, Commercial consistency, Supportability, Marketing truth, Launch decision | ☐ |
| `EXE-14-014` | Screenshots match build | Should ship | Medium | Specified | Privacy, Documentation, Commercial consistency, Supportability, Marketing truth, Launch decision | ☐ |
| `EXE-14-015` | Marketing claims pass truth review | Should ship | Medium | Specified | Privacy, Documentation, Commercial consistency, Supportability, Marketing truth, Launch decision | ☐ |
| `EXE-14-016` | Transactional and marketing email are separated | Should ship | Medium | Specified | Privacy, Documentation, Commercial consistency, Supportability, Marketing truth, Launch decision | ☐ |
| `EXE-14-017` | Bounce/complaint handling exists | Should ship | Medium | Specified | Privacy, Documentation, Commercial consistency, Supportability, Marketing truth, Launch decision | ☐ |
| `EXE-14-018` | Review request policy is ethical | Should ship | Medium | Specified | Privacy, Documentation, Commercial consistency, Supportability, Marketing truth, Launch decision | ☐ |
| `EXE-14-019` | Social accounts have MFA and backup admin | Should ship | Medium | Specified | Privacy, Documentation, Commercial consistency, Supportability, Marketing truth, Launch decision | ☐ |
| `EXE-15-001` | Local safety backup behavior is documented | Should ship | Medium | Specified | Persistence and migration, Cross-platform, Security, Diagnostics, Continuity, Launch decision | ☐ |
| `EXE-15-002` | Manual backup exists | Should ship | Medium | Specified | Persistence and migration, Cross-platform, Security, Diagnostics, Continuity, Launch decision | ☐ |
| `EXE-15-003` | Restore is tested | Should ship | Medium | Specified | Persistence and migration, Cross-platform, Security, Diagnostics, Continuity, Launch decision | ☐ |
| `EXE-15-004` | Corrupt backup is detected | Should ship | Medium | Specified | Persistence and migration, Cross-platform, Security, Diagnostics, Continuity, Launch decision | ☐ |
| `EXE-15-005` | New-machine restore is tested | Should ship | Medium | Specified | Persistence and migration, Cross-platform, Security, Diagnostics, Continuity, Launch decision | ☐ |
| `EXE-15-006` | Cross-platform restore is tested | Should ship | Medium | Specified | Persistence and migration, Cross-platform, Security, Diagnostics, Continuity, Launch decision | ☐ |
| `EXE-15-007` | RPO is defined where applicable | Should ship | Medium | Specified | Persistence and migration, Cross-platform, Security, Diagnostics, Continuity, Launch decision | ☐ |
| `EXE-15-008` | RTO is defined where applicable | Should ship | Medium | Specified | Persistence and migration, Cross-platform, Security, Diagnostics, Continuity, Launch decision | ☐ |
| `EXE-15-009` | Company source code is backed up | Should ship | Medium | Specified | Persistence and migration, Cross-platform, Security, Diagnostics, Continuity, Launch decision | ☐ |
| `EXE-15-010` | Release artifacts are backed up | Should ship | Medium | Specified | Persistence and migration, Cross-platform, Security, Diagnostics, Continuity, Launch decision | ☐ |
| `EXE-15-011` | Legal records are backed up | Should ship | Medium | Specified | Persistence and migration, Cross-platform, Security, Diagnostics, Continuity, Launch decision | ☐ |
| `EXE-15-012` | Runbooks are backed up | Should ship | Medium | Specified | Persistence and migration, Cross-platform, Security, Diagnostics, Continuity, Launch decision | ☐ |
| `EXE-15-013` | Founder-unavailable procedure exists | Should ship | Medium | Specified | Persistence and migration, Cross-platform, Security, Diagnostics, Continuity, Launch decision | ☐ |
| `EXE-15-014` | GitHub-loss procedure exists | Should ship | Medium | Specified | Persistence and migration, Cross-platform, Security, Diagnostics, Continuity, Launch decision | ☐ |
| `EXE-15-015` | Store-account-loss procedure exists | Should ship | Medium | Specified | Persistence and migration, Cross-platform, Security, Diagnostics, Continuity, Launch decision | ☐ |
| `EXE-15-016` | Apple-account-loss procedure exists | Should ship | Medium | Specified | Persistence and migration, Cross-platform, Security, Diagnostics, Continuity, Launch decision | ☐ |
| `EXE-15-017` | Domain-loss procedure exists | Should ship | Medium | Specified | Persistence and migration, Cross-platform, Security, Diagnostics, Continuity, Launch decision | ☐ |
| `EXE-15-018` | Annual continuity drill is performed | Should ship | Medium | Specified | Persistence and migration, Cross-platform, Security, Diagnostics, Continuity, Launch decision | ☐ |
| `EXE-16-001` | Release candidate is immutable once approved | Must ship | Medium | Specified | Cross-platform, Security, Release artifact, Installer and update, Diagnostics, Documentation, Commercial consistency, Supportability, Continuity, Marketing truth, Launch decision | ☐ |
| `EXE-16-002` | Changelog is generated | Must ship | Medium | Specified | Cross-platform, Security, Release artifact, Installer and update, Diagnostics, Documentation, Commercial consistency, Supportability, Continuity, Marketing truth, Launch decision | ☐ |
| `EXE-16-003` | Release notes are generated | Must ship | Medium | Specified | Cross-platform, Security, Release artifact, Installer and update, Diagnostics, Documentation, Commercial consistency, Supportability, Continuity, Marketing truth, Launch decision | ☐ |
| `EXE-16-004` | Known issues are documented | Must ship | Medium | Specified | Cross-platform, Security, Release artifact, Installer and update, Diagnostics, Documentation, Commercial consistency, Supportability, Continuity, Marketing truth, Launch decision | ☐ |
| `EXE-16-005` | Checksums are generated | Must ship | Medium | Specified | Cross-platform, Security, Release artifact, Installer and update, Diagnostics, Documentation, Commercial consistency, Supportability, Continuity, Marketing truth, Launch decision | ☐ |
| `EXE-16-006` | SBOM is generated | Must ship | Medium | Specified | Cross-platform, Security, Release artifact, Installer and update, Diagnostics, Documentation, Commercial consistency, Supportability, Continuity, Marketing truth, Launch decision | ☐ |
| `EXE-16-007` | Artifacts are signed | Must ship | Medium | Specified | Cross-platform, Security, Release artifact, Installer and update, Diagnostics, Documentation, Commercial consistency, Supportability, Continuity, Marketing truth, Launch decision | ☐ |
| `EXE-16-008` | Signatures are independently verified | Must ship | Medium | Specified | Cross-platform, Security, Release artifact, Installer and update, Diagnostics, Documentation, Commercial consistency, Supportability, Continuity, Marketing truth, Launch decision | ☐ |
| `EXE-16-009` | Windows artifact is tested | Must ship | Medium | Specified | Cross-platform, Security, Release artifact, Installer and update, Diagnostics, Documentation, Commercial consistency, Supportability, Continuity, Marketing truth, Launch decision | ☐ |
| `EXE-16-010` | macOS artifact is tested | Must ship | Medium | Specified | Cross-platform, Security, Release artifact, Installer and update, Diagnostics, Documentation, Commercial consistency, Supportability, Continuity, Marketing truth, Launch decision | ☐ |
| `EXE-16-011` | Store artifact is tested | Must ship | Medium | Specified | Cross-platform, Security, Release artifact, Installer and update, Diagnostics, Documentation, Commercial consistency, Supportability, Continuity, Marketing truth, Launch decision | ☐ |
| `EXE-16-012` | Upgrade is tested | Must ship | Medium | Specified | Cross-platform, Security, Release artifact, Installer and update, Diagnostics, Documentation, Commercial consistency, Supportability, Continuity, Marketing truth, Launch decision | ☐ |
| `EXE-16-013` | Rollback artifact exists | Must ship | Medium | Specified | Cross-platform, Security, Release artifact, Installer and update, Diagnostics, Documentation, Commercial consistency, Supportability, Continuity, Marketing truth, Launch decision | ☐ |
| `EXE-16-014` | Launch war room exists | Must ship | Medium | Specified | Cross-platform, Security, Release artifact, Installer and update, Diagnostics, Documentation, Commercial consistency, Supportability, Continuity, Marketing truth, Launch decision | ☐ |
| `EXE-16-015` | Post-launch health review exists | Must ship | Medium | Specified | Cross-platform, Security, Release artifact, Installer and update, Diagnostics, Documentation, Commercial consistency, Supportability, Continuity, Marketing truth, Launch decision | ☐ |
| `EXE-16-016` | Recurring maintenance calendar exists | Must ship | Medium | Specified | Cross-platform, Security, Release artifact, Installer and update, Diagnostics, Documentation, Commercial consistency, Supportability, Continuity, Marketing truth, Launch decision | ☐ |
| `BUS-001` | Umbrella-company ownership, product/brand ownership and publisher identity are documented. | Should ship | Medium | Specified | Continuity, Launch decision | ☐ |
| `BUS-002` | Legal entity, trading name, product name, billing identity and support identity are reconciled. | Should ship | Medium | Specified | Continuity, Launch decision | ☐ |
| `BUS-003` | Ownership of source code, trademarks, domains, artwork and documentation is documented. | Should ship | Medium | Specified | Continuity, Launch decision | ☐ |
| `BUS-004` | Founder-only dependencies have a named backup or recovery path. | Should ship | Medium | Specified | Continuity, Launch decision | ☐ |
| `BUS-005` | Revenue, refunds, fees, taxes and accounting workflow have an owner and evidence trail. | Should ship | Medium | Specified | Continuity, Launch decision | ☐ |
| `REQ-001` | Every promised capability has one canonical requirement ID in a feature ledger. | Should ship | Medium | Specified | Architecture, Documentation, Launch decision | ☐ |
| `REQ-002` | Each requirement records source, priority, owner, state, evidence and applicable gates. | Should ship | Medium | Specified | Architecture, Documentation, Launch decision | ☐ |
| `REQ-003` | Requirements support Required, Applicable, N/A, Blocked, Evidence Pending, Pass and Risk Accepted states. | Should ship | Medium | Specified | Architecture, Documentation, Launch decision | ☐ |
| `REQ-004` | N/A decisions require a reason; critical N/A decisions require explicit approval. | Should ship | Medium | Specified | Architecture, Documentation, Launch decision | ☐ |
| `REQ-005` | Public product claims are traceable to shipped behavior and evidence. | Should ship | Medium | Specified | Architecture, Documentation, Launch decision | ☐ |
| `REQ-006` | Launch scope and post-launch backlog are explicitly separated. | Should ship | Medium | Specified | Architecture, Documentation, Launch decision | ☐ |
| `ARCH-001` | React remains presentation/client code and never becomes canonical project storage. | Must ship | High | Specified | Architecture, Feature lifecycle, Cross-platform, Security, AI authority, Launch decision | ☐ |
| `ARCH-002` | Canonical mutations remain behind the approved Rust application/store/registry boundary. | Must ship | High | Specified | Architecture, Feature lifecycle, Cross-platform, Security, AI authority, Launch decision | ☐ |
| `ARCH-003` | Search and AI indexes are derived and rebuildable. | Must ship | High | Specified | Architecture, Feature lifecycle, Cross-platform, Security, AI authority, Launch decision | ☐ |
| `ARCH-004` | Platform-specific behavior is isolated behind adapters/services. | Must ship | High | Specified | Architecture, Feature lifecycle, Cross-platform, Security, AI authority, Launch decision | ☐ |
| `ARCH-005` | Forbidden dependency directions are mechanically checked. | Must ship | High | Specified | Architecture, Feature lifecycle, Cross-platform, Security, AI authority, Launch decision | ☐ |
| `ARCH-006` | Generated bindings/contracts are reproducible and drift is a CI failure. | Must ship | High | Specified | Architecture, Feature lifecycle, Cross-platform, Security, AI authority, Launch decision | ☐ |
| `ARCH-007` | Retired LAN collaboration is not reintroduced without a new approved architecture decision. | Must ship | High | Specified | Architecture, Feature lifecycle, Cross-platform, Security, AI authority, Launch decision | ☐ |
| `ARCH-008` | Trust boundaries and data flows are versioned architecture artifacts. | Must ship | High | Specified | Architecture, Feature lifecycle, Cross-platform, Security, AI authority, Launch decision | ☐ |
| `DATA-001` | Every schema change has a forward migration and representative upgrade fixtures. | Must ship | Critical | Specified | Feature lifecycle, Persistence and migration, Cross-platform, Launch decision | ☐ |
| `DATA-002` | Migration tests cover fresh, legacy, representative, large and interrupted projects. | Must ship | Critical | Specified | Feature lifecycle, Persistence and migration, Cross-platform, Launch decision | ☐ |
| `DATA-003` | Migration failure cannot silently discard canonical data. | Must ship | Critical | Specified | Feature lifecycle, Persistence and migration, Cross-platform, Launch decision | ☐ |
| `DATA-004` | Undo/redo semantics survive supported mutations and migrations. | Must ship | Critical | Specified | Feature lifecycle, Persistence and migration, Cross-platform, Launch decision | ☐ |
| `DATA-005` | Autosave/checkpoint behavior is safe under process termination. | Must ship | Critical | Specified | Feature lifecycle, Persistence and migration, Cross-platform, Launch decision | ☐ |
| `DATA-006` | Restart/reopen proves persisted state before a feature is considered complete. | Must ship | Critical | Specified | Feature lifecycle, Persistence and migration, Cross-platform, Launch decision | ☐ |
| `DATA-007` | SQLite integrity and application invariants are checked at recovery boundaries. | Must ship | Critical | Specified | Feature lifecycle, Persistence and migration, Cross-platform, Launch decision | ☐ |
| `PORT-001` | Project format versioning and compatibility policy are explicit. | Must ship | High | Specified | Feature lifecycle, Persistence and migration, Cross-platform, Round trip, Security, Documentation, Launch decision | ☐ |
| `PORT-002` | Project manifest and validation sequence are explicit. | Must ship | High | Specified | Feature lifecycle, Persistence and migration, Cross-platform, Round trip, Security, Documentation, Launch decision | ☐ |
| `PORT-003` | Cross-platform portability is a semantic round trip, not merely a successful open. | Must ship | High | Specified | Feature lifecycle, Persistence and migration, Cross-platform, Round trip, Security, Documentation, Launch decision | ☐ |
| `PORT-004` | Unknown/unsupported constructs are preserved, mapped or warned about explicitly. | Must ship | High | Specified | Feature lifecycle, Persistence and migration, Cross-platform, Round trip, Security, Documentation, Launch decision | ☐ |
| `PORT-005` | Project imports never execute project content as code. | Must ship | High | Specified | Feature lifecycle, Persistence and migration, Cross-platform, Round trip, Security, Documentation, Launch decision | ☐ |
| `PORT-006` | Replacement/import-over-existing behavior is explicit. | Must ship | High | Specified | Feature lifecycle, Persistence and migration, Cross-platform, Round trip, Security, Documentation, Launch decision | ☐ |
| `PORT-007` | Customer recovery on a replacement computer is documented and tested. | Must ship | High | Specified | Feature lifecycle, Persistence and migration, Cross-platform, Round trip, Security, Documentation, Launch decision | ☐ |
| `SCR-001` | Screenplay elements have a stable canonical model independent of rendering. | Should ship | Medium | Specified | Feature lifecycle, Cross-platform, Round trip, Accessibility and UX, Performance, Documentation, Marketing truth, Launch decision | ☐ |
| `SCR-002` | Pagination is deterministic for a pinned renderer/version and covered by fixtures. | Should ship | Medium | Specified | Feature lifecycle, Cross-platform, Round trip, Accessibility and UX, Performance, Documentation, Marketing truth, Launch decision | ☐ |
| `SCR-003` | Scene numbering, revisions, production locking and page metadata have explicit invariants. | Should ship | Medium | Specified | Feature lifecycle, Cross-platform, Round trip, Accessibility and UX, Performance, Documentation, Marketing truth, Launch decision | ☐ |
| `SCR-004` | FDX import is previewable and warns about unknown/lossy constructs. | Should ship | Medium | Specified | Feature lifecycle, Cross-platform, Round trip, Accessibility and UX, Performance, Documentation, Marketing truth, Launch decision | ☐ |
| `SCR-005` | FDX export is round-trip tested. | Should ship | Medium | Specified | Feature lifecycle, Cross-platform, Round trip, Accessibility and UX, Performance, Documentation, Marketing truth, Launch decision | ☐ |
| `SCR-006` | Fountain, DOCX and PDF capabilities have explicit fidelity expectations. | Should ship | Medium | Specified | Feature lifecycle, Cross-platform, Round trip, Accessibility and UX, Performance, Documentation, Marketing truth, Launch decision | ☐ |
| `SCR-007` | Unicode, mixed-language text and IME input are tested. | Should ship | Medium | Specified | Feature lifecycle, Cross-platform, Round trip, Accessibility and UX, Performance, Documentation, Marketing truth, Launch decision | ☐ |
| `SCR-008` | Screenplay PDF output has visual regression coverage. | Should ship | Medium | Specified | Feature lifecycle, Cross-platform, Round trip, Accessibility and UX, Performance, Documentation, Marketing truth, Launch decision | ☐ |
| `PROD-001` | Story, scene, screenplay, breakdown, schedule and call-sheet data maintain explicit lineage. | Should ship | Medium | Specified | Feature lifecycle, Persistence and migration, Cross-platform, Round trip, AI authority, Accessibility and UX, Performance, Documentation, Marketing truth, Launch decision | ☐ |
| `PROD-002` | Story Board customization and long-form content have defined persistence/export behavior. | Should ship | Medium | Specified | Feature lifecycle, Persistence and migration, Cross-platform, Round trip, AI authority, Accessibility and UX, Performance, Documentation, Marketing truth, Launch decision | ☐ |
| `PROD-003` | Breakdown bulk/quick operations preserve canonical data and review state. | Should ship | Medium | Specified | Feature lifecycle, Persistence and migration, Cross-platform, Round trip, AI authority, Accessibility and UX, Performance, Documentation, Marketing truth, Launch decision | ☐ |
| `PROD-004` | Catalog, Locations, Cast and Crew references have stable identity/reuse behavior. | Should ship | Medium | Specified | Feature lifecycle, Persistence and migration, Cross-platform, Round trip, AI authority, Accessibility and UX, Performance, Documentation, Marketing truth, Launch decision | ☐ |
| `PROD-005` | Schedule covers time, duration, breaks, travel/company moves, wrap and conflicts. | Should ship | Medium | Specified | Feature lifecycle, Persistence and migration, Cross-platform, Round trip, AI authority, Accessibility and UX, Performance, Documentation, Marketing truth, Launch decision | ☐ |
| `PROD-006` | DOOD is a deterministic derived view/report. | Should ship | Medium | Specified | Feature lifecycle, Persistence and migration, Cross-platform, Round trip, AI authority, Accessibility and UX, Performance, Documentation, Marketing truth, Launch decision | ☐ |
| `PROD-007` | Call Sheets derive from approved production data with explicit manual overrides. | Should ship | Medium | Specified | Feature lifecycle, Persistence and migration, Cross-platform, Round trip, AI authority, Accessibility and UX, Performance, Documentation, Marketing truth, Launch decision | ☐ |
| `PROD-008` | Budget scope is explicitly lightweight and not full studio accounting. | Should ship | Medium | Specified | Feature lifecycle, Persistence and migration, Cross-platform, Round trip, AI authority, Accessibility and UX, Performance, Documentation, Marketing truth, Launch decision | ☐ |
| `PROD-009` | Production reports are reproducible from canonical state. | Should ship | Medium | Specified | Feature lifecycle, Persistence and migration, Cross-platform, Round trip, AI authority, Accessibility and UX, Performance, Documentation, Marketing truth, Launch decision | ☐ |
| `AI-001` | Every AI-exposed operation has metadata for capability, mutation class, confirmation and side effects. | Must ship | High | Specified | Cross-platform, Security, Privacy, AI authority, AI evaluation, Performance, Diagnostics, Documentation, Launch decision | ☐ |
| `AI-002` | AI cannot bypass canonical mutation APIs or human Change Set approval. | Must ship | High | Specified | Cross-platform, Security, Privacy, AI authority, AI evaluation, Performance, Diagnostics, Documentation, Launch decision | ☐ |
| `AI-003` | AI-visible data respects permissions and privacy classification. | Must ship | High | Specified | Cross-platform, Security, Privacy, AI authority, AI evaluation, Performance, Diagnostics, Documentation, Launch decision | ☐ |
| `AI-004` | AI retrieval quality has a fixed evaluation corpus and thresholds. | Must ship | High | Specified | Cross-platform, Security, Privacy, AI authority, AI evaluation, Performance, Diagnostics, Documentation, Launch decision | ☐ |
| `AI-005` | Model/runtime updates have regression evaluation. | Must ship | High | Specified | Cross-platform, Security, Privacy, AI authority, AI evaluation, Performance, Diagnostics, Documentation, Launch decision | ☐ |
| `AI-006` | Prompt injection, retrieval poisoning, tool misuse and privilege escalation are tested. | Must ship | High | Specified | Cross-platform, Security, Privacy, AI authority, AI evaluation, Performance, Diagnostics, Documentation, Launch decision | ☐ |
| `AI-007` | Model downloads are verified by trusted metadata/hash/signature before use. | Must ship | High | Specified | Cross-platform, Security, Privacy, AI authority, AI evaluation, Performance, Diagnostics, Documentation, Launch decision | ☐ |
| `AI-008` | AI sidecar startup/auth/timeout/cancel/crash/hang/shutdown/orphan cleanup are tested. | Must ship | High | Specified | Cross-platform, Security, Privacy, AI authority, AI evaluation, Performance, Diagnostics, Documentation, Launch decision | ☐ |
| `AI-009` | Minimum hardware and performance are measured on supported targets. | Must ship | High | Specified | Cross-platform, Security, Privacy, AI authority, AI evaluation, Performance, Diagnostics, Documentation, Launch decision | ☐ |
| `AI-010` | Offline/local-AI claims are proven against actual network/process behavior. | Must ship | High | Specified | Cross-platform, Security, Privacy, AI authority, AI evaluation, Performance, Diagnostics, Documentation, Launch decision | ☐ |
| `SEC-001` | Trust boundaries cover WebView, IPC, Rust, filesystem, SQLite, importers, AI, updater and services. | Must ship | Critical | Specified | Security, Privacy, Release artifact, Installer and update, Diagnostics, Continuity, Launch decision | ☐ |
| `SEC-002` | IPC is allow-listed and capability checked. | Must ship | Critical | Specified | Security, Privacy, Release artifact, Installer and update, Diagnostics, Continuity, Launch decision | ☐ |
| `SEC-003` | Path handling rejects traversal, symlink escape and platform-dangerous path forms. | Must ship | Critical | Specified | Security, Privacy, Release artifact, Installer and update, Diagnostics, Continuity, Launch decision | ☐ |
| `SEC-004` | Parsers are bounded for size, depth, decompression and resource use. | Must ship | Critical | Specified | Security, Privacy, Release artifact, Installer and update, Diagnostics, Continuity, Launch decision | ☐ |
| `SEC-005` | Secrets and credentials are absent from source, artifacts and logs. | Must ship | Critical | Specified | Security, Privacy, Release artifact, Installer and update, Diagnostics, Continuity, Launch decision | ☐ |
| `SEC-006` | Dependencies have vulnerability and license review with documented exceptions. | Must ship | Critical | Specified | Security, Privacy, Release artifact, Installer and update, Diagnostics, Continuity, Launch decision | ☐ |
| `SEC-007` | SBOM and release provenance are generated. | Must ship | Critical | Specified | Security, Privacy, Release artifact, Installer and update, Diagnostics, Continuity, Launch decision | ☐ |
| `SEC-008` | Logs exclude screenplay text, private notes, project contents and credentials by default. | Must ship | Critical | Specified | Security, Privacy, Release artifact, Installer and update, Diagnostics, Continuity, Launch decision | ☐ |
| `SEC-009` | Security disclosure and severity/triage workflow exist. | Must ship | Critical | Specified | Security, Privacy, Release artifact, Installer and update, Diagnostics, Continuity, Launch decision | ☐ |
| `SEC-010` | Critical unresolved security findings block release. | Must ship | Critical | Specified | Security, Privacy, Release artifact, Installer and update, Diagnostics, Continuity, Launch decision | ☐ |
| `PRIV-001` | A data inventory covers collected, stored, transmitted and derived data. | Must ship | Critical | Specified | Launch decision | ☐ |
| `PRIV-002` | External services have documented purpose, data flow, retention and contractual review. | Must ship | Critical | Specified | Launch decision | ☐ |
| `PRIV-003` | Privacy policy matches actual implementation. | Must ship | Critical | Specified | Launch decision | ☐ |
| `PRIV-004` | Creative content is not sent externally unless explicitly required and disclosed. | Must ship | Critical | Specified | Launch decision | ☐ |
| `PRIV-005` | Applicable privacy requests have an operational owner and workflow. | Must ship | Critical | Specified | Launch decision | ☐ |
| `PRIV-006` | Crash/telemetry data is minimized and redacted. | Must ship | Critical | Specified | Launch decision | ☐ |
| `LEGAL-001` | Terms/EULA, privacy, refund/cancellation and licensing language are consistent with implementation. | Must ship | High | Specified | Privacy, Release artifact, Documentation, Commercial consistency, Marketing truth, Launch decision | ☐ |
| `LEGAL-002` | Open-source and third-party notices are complete. | Must ship | High | Specified | Privacy, Release artifact, Documentation, Commercial consistency, Marketing truth, Launch decision | ☐ |
| `LEGAL-003` | AI/model/embedding licenses are recorded. | Must ship | High | Specified | Privacy, Release artifact, Documentation, Commercial consistency, Marketing truth, Launch decision | ☐ |
| `LEGAL-004` | Marketing claims receive product/legal truth review. | Must ship | High | Specified | Privacy, Release artifact, Documentation, Commercial consistency, Marketing truth, Launch decision | ☐ |
| `LEGAL-005` | Trademark/domain/product-name rights are reviewed. | Must ship | High | Specified | Privacy, Release artifact, Documentation, Commercial consistency, Marketing truth, Launch decision | ☐ |
| `WIN-001` | Windows clean install, upgrade, uninstall and reinstall are tested on clean machines. | Must ship | High | Specified | Cross-platform, Security, Release artifact, Installer and update, Accessibility and UX, Performance, Diagnostics, Documentation, Launch decision | ☐ |
| `WIN-002` | Windows signatures and timestamps are independently verified. | Must ship | High | Specified | Cross-platform, Security, Release artifact, Installer and update, Accessibility and UX, Performance, Diagnostics, Documentation, Launch decision | ☐ |
| `WIN-003` | WebView2 behavior and prerequisites are documented. | Must ship | High | Specified | Cross-platform, Security, Release artifact, Installer and update, Accessibility and UX, Performance, Diagnostics, Documentation, Launch decision | ☐ |
| `WIN-004` | SmartScreen/Defender behavior is tested from actual release artifacts. | Must ship | High | Specified | Cross-platform, Security, Release artifact, Installer and update, Accessibility and UX, Performance, Diagnostics, Documentation, Launch decision | ☐ |
| `WIN-005` | High DPI, multi-monitor, IME, Unicode, long paths, external drives and sleep/wake are tested. | Must ship | High | Specified | Cross-platform, Security, Release artifact, Installer and update, Accessibility and UX, Performance, Diagnostics, Documentation, Launch decision | ☐ |
| `WIN-006` | Store and direct-distribution channels have explicit distinct update/entitlement behavior. | Must ship | High | Specified | Cross-platform, Security, Release artifact, Installer and update, Accessibility and UX, Performance, Diagnostics, Documentation, Launch decision | ☐ |
| `MAC-001` | Mac clean install, upgrade, uninstall and reinstall from the Mac App Store package are tested, including sandbox behaviour. | Must ship | High | Specified | Cross-platform, Security, Release artifact, Installer and update, Accessibility and UX, Performance, Diagnostics, Documentation, Launch decision | ☐ |
| `MAC-002` | Apple silicon is the only supported Mac architecture and the package enforces it. | Must ship | High | Specified | Cross-platform, Security, Release artifact, Installer and update, Accessibility and UX, Performance, Diagnostics, Documentation, Launch decision | ☐ |
| `MAC-003` | Apple Distribution signing, App Sandbox, minimal entitlements and Apple's package validation are verified. | Must ship | High | Specified | Cross-platform, Security, Release artifact, Installer and update, Accessibility and UX, Performance, Diagnostics, Documentation, Launch decision | ☐ |
| `MAC-004` | Filesystem, case sensitivity, Unicode, permissions, volumes and sleep/wake are tested. | Must ship | High | Specified | Cross-platform, Security, Release artifact, Installer and update, Accessibility and UX, Performance, Diagnostics, Documentation, Launch decision | ☐ |
| `MAC-005` | Apple account/certificate/notarization recovery has a backup operator. | Must ship | High | Specified | Cross-platform, Security, Release artifact, Installer and update, Accessibility and UX, Performance, Diagnostics, Documentation, Launch decision | ☐ |
| `STORE-001` | Partner Center/company ownership has at least two authorized administrators. | Must ship | High | Specified | Release artifact, Installer and update, Documentation, Commercial consistency, Marketing truth, Launch decision | ☐ |
| `STORE-002` | Publisher/package identity matches the legal company and release pipeline. | Must ship | High | Specified | Release artifact, Installer and update, Documentation, Commercial consistency, Marketing truth, Launch decision | ☐ |
| `STORE-003` | Store listing, screenshots, rating, category, regions, pricing, trial and URLs match the release. | Must ship | High | Specified | Release artifact, Installer and update, Documentation, Commercial consistency, Marketing truth, Launch decision | ☐ |
| `STORE-004` | Certification testing is repeatable and evidence is archived. | Must ship | High | Specified | Release artifact, Installer and update, Documentation, Commercial consistency, Marketing truth, Launch decision | ☐ |
| `STORE-005` | Store install, update, uninstall/reinstall and entitlement restoration are tested. | Must ship | High | Specified | Release artifact, Installer and update, Documentation, Commercial consistency, Marketing truth, Launch decision | ☐ |
| `STORE-006` | Store commerce limitations are distinguished from app UX claims. | Must ship | High | Specified | Release artifact, Installer and update, Documentation, Commercial consistency, Marketing truth, Launch decision | ☐ |
| `COMM-001` | The advertised 7-day trial requires no credit card. | Must ship | High | Specified | Privacy, Documentation, Commercial consistency, Supportability, Marketing truth, Launch decision | ☐ |
| `COMM-002` | Trial expiry, reinstall, account/device changes, clock tampering and offline behavior are defined. | Must ship | High | Specified | Privacy, Documentation, Commercial consistency, Supportability, Marketing truth, Launch decision | ☐ |
| `COMM-003` | Purchase, entitlement refresh, restore, cancellation and refund paths are tested. | Must ship | High | Specified | Privacy, Documentation, Commercial consistency, Supportability, Marketing truth, Launch decision | ☐ |
| `COMM-004` | Discounts/dynamic pricing have one authoritative owner per channel. | Must ship | High | Specified | Privacy, Documentation, Commercial consistency, Supportability, Marketing truth, Launch decision | ☐ |
| `COMM-005` | Store, website and app pricing/trial/region/currency claims agree. | Must ship | High | Specified | Privacy, Documentation, Commercial consistency, Supportability, Marketing truth, Launch decision | ☐ |
| `COMM-006` | Commerce outages cannot corrupt project data. | Must ship | High | Specified | Privacy, Documentation, Commercial consistency, Supportability, Marketing truth, Launch decision | ☐ |
| `UPD-001` | Every release has immutable version, channel, artifact hash, signature/provenance and notes. | Must ship | Critical | Specified | Cross-platform, Security, Release artifact, Installer and update, Diagnostics, Documentation, Launch decision | ☐ |
| `UPD-002` | Direct updates require approval and authenticity verification. | Must ship | Critical | Specified | Cross-platform, Security, Release artifact, Installer and update, Diagnostics, Documentation, Launch decision | ☐ |
| `UPD-003` | Store/App Store updates use platform-native mechanisms. | Must ship | Critical | Specified | Cross-platform, Security, Release artifact, Installer and update, Diagnostics, Documentation, Launch decision | ☐ |
| `UPD-004` | Update failure preserves the prior usable application where technically supported. | Must ship | Critical | Specified | Cross-platform, Security, Release artifact, Installer and update, Diagnostics, Documentation, Launch decision | ☐ |
| `UPD-005` | Projects are checkpointed before update and active mutations are drained. | Must ship | Critical | Specified | Cross-platform, Security, Release artifact, Installer and update, Diagnostics, Documentation, Launch decision | ☐ |
| `SUP-001` | Customers can obtain support without founder-only intervention. | Should ship | Medium | Specified | Privacy, Diagnostics, Documentation, Supportability, Launch decision | ☐ |
| `SUP-002` | Support severity, response targets and escalation are defined. | Should ship | Medium | Specified | Privacy, Diagnostics, Documentation, Supportability, Launch decision | ☐ |
| `SUP-003` | Safe diagnostic bundles can be generated without creative content/secrets. | Should ship | Medium | Specified | Privacy, Diagnostics, Documentation, Supportability, Launch decision | ☐ |
| `SUP-004` | KB covers installation, update, recovery, import/export, AI, Windows and macOS. | Should ship | Medium | Specified | Privacy, Diagnostics, Documentation, Supportability, Launch decision | ☐ |
| `SUP-005` | Support can reproduce issues from safe version/build/platform/project-fixture data. | Should ship | Medium | Specified | Privacy, Diagnostics, Documentation, Supportability, Launch decision | ☐ |
| `SUP-006` | Errors provide actionable recovery guidance. | Should ship | Medium | Specified | Privacy, Diagnostics, Documentation, Supportability, Launch decision | ☐ |
| `SUP-007` | Crash loops have a safe recovery path. | Should ship | Medium | Specified | Privacy, Diagnostics, Documentation, Supportability, Launch decision | ☐ |
| `GTM-001` | Website/domain/DNS ownership and recovery are company-controlled. | Should ship | Medium | Specified | Privacy, Documentation, Commercial consistency, Supportability, Marketing truth, Launch decision | ☐ |
| `GTM-002` | Website download links always target current verified releases. | Should ship | Medium | Specified | Privacy, Documentation, Commercial consistency, Supportability, Marketing truth, Launch decision | ☐ |
| `GTM-003` | Website, Store, app and policies make consistent claims. | Should ship | Medium | Specified | Privacy, Documentation, Commercial consistency, Supportability, Marketing truth, Launch decision | ☐ |
| `GTM-004` | Transactional and marketing email are appropriately separated. | Should ship | Medium | Specified | Privacy, Documentation, Commercial consistency, Supportability, Marketing truth, Launch decision | ☐ |
| `GTM-005` | SPF, DKIM and DMARC are configured and monitored. | Should ship | Medium | Specified | Privacy, Documentation, Commercial consistency, Supportability, Marketing truth, Launch decision | ☐ |
| `GTM-006` | Social accounts have MFA, recovery codes and backup administration. | Should ship | Medium | Specified | Privacy, Documentation, Commercial consistency, Supportability, Marketing truth, Launch decision | ☐ |
| `GTM-007` | Review requests are honest and non-deceptive. | Should ship | Medium | Specified | Privacy, Documentation, Commercial consistency, Supportability, Marketing truth, Launch decision | ☐ |
| `GTM-008` | Content calendar covers launch, education, updates and support education. | Should ship | Medium | Specified | Privacy, Documentation, Commercial consistency, Supportability, Marketing truth, Launch decision | ☐ |
| `GTM-009` | Marketing assets have an owner and source of truth. | Should ship | Medium | Specified | Privacy, Documentation, Commercial consistency, Supportability, Marketing truth, Launch decision | ☐ |
| `GTM-010` | Analytics/attribution is privacy-reviewed and non-essential collection can be disabled if applicable. | Should ship | Medium | Specified | Privacy, Documentation, Commercial consistency, Supportability, Marketing truth, Launch decision | ☐ |
| `BCP-001` | Critical company assets have independent backups and tested restores. | Should ship | Medium | Specified | Launch decision | ☐ |
| `BCP-002` | Recovery objectives exist for source, domains, store accounts, signing, website, support and release artifacts. | Should ship | Medium | Specified | Launch decision | ☐ |
| `BCP-003` | Loss of GitHub, Microsoft, Apple, domain or email access has a recovery plan. | Should ship | Medium | Specified | Launch decision | ☐ |
| `BCP-004` | A second operator can perform critical release/support/recovery workflows. | Should ship | Medium | Specified | Launch decision | ☐ |
| `BCP-005` | A disaster-recovery drill is completed before launch and recurring thereafter. | Should ship | Medium | Specified | Launch decision | ☐ |
| `OPS-001` | Launch-day war room has primary/backup owners, communication channel, decision authority and stop-ship triggers. | Must ship | Medium | Specified | Cross-platform, Security, Release artifact, Installer and update, Diagnostics, Documentation, Commercial consistency, Supportability, Continuity, Marketing truth, Launch decision | ☐ |
| `OPS-002` | Release evidence is durably stored by immutable release identifier. | Must ship | Medium | Specified | Cross-platform, Security, Release artifact, Installer and update, Diagnostics, Documentation, Commercial consistency, Supportability, Continuity, Marketing truth, Launch decision | ☐ |
| `OPS-003` | Post-launch health review covers crashes, activation, trial conversion, support, refunds, reviews, performance and security. | Must ship | Medium | Specified | Cross-platform, Security, Release artifact, Installer and update, Diagnostics, Documentation, Commercial consistency, Supportability, Continuity, Marketing truth, Launch decision | ☐ |
| `OPS-004` | Recurring maintenance calendar covers dependencies, certificates, domains, policies, backups and vendors. | Must ship | Medium | Specified | Cross-platform, Security, Release artifact, Installer and update, Diagnostics, Documentation, Commercial consistency, Supportability, Continuity, Marketing truth, Launch decision | ☐ |
| `OPS-005` | End-of-life/product-continuity policy protects customer project access and support expectations. | Must ship | Medium | Specified | Cross-platform, Security, Release artifact, Installer and update, Diagnostics, Documentation, Commercial consistency, Supportability, Continuity, Marketing truth, Launch decision | ☐ |

---

# Areas covered by the register

Every requirement key belongs to an area. The execution requirements (EXE-01 to EXE-16) and the later keys cover the same ground in two groupings.

**Execution requirements**

| Keys | Area | What it covers |
|---|---|---|
| EXE-01 | Architecture and governance | Rules that prevent semantic drift and preserve the durable core. |
| EXE-02 | Database and migrations | Schema evolution, fixtures, transactions and recovery. |
| EXE-03 | Project format and portability | Portable project packages, compatibility and semantic round trips. |
| EXE-04 | Screenplay and writing | Structured screenplay, pagination, revisions and migration. |
| EXE-05 | Import/export security and reliability | Safe parsers, bounded inputs, transactional import and export. |
| EXE-06 | Windows | Compatibility, signing, installers, Store and direct channels and OS behavior. |
| EXE-07 | macOS | Compatibility, signing, notarization, packaging and OS behavior. |
| EXE-08 | Microsoft Store | Publisher ownership, listing, certification, package and Store lifecycle. |
| EXE-09 | Trial and commerce | 7-day no-card trial, pricing, discounts, entitlement and refunds. |
| EXE-10 | Security | Threat model, parser safety, secrets, dependencies, signing and disclosure. |
| EXE-11 | AI | Local model runtime, retrieval, bounded tools, safety, privacy and evaluation. |
| EXE-12 | Privacy and legal | Data flows, policy truthfulness, licensing and customer rights workflows. |
| EXE-13 | Crash, diagnostics and support | Customer diagnostics, support operations and escalation. |
| EXE-14 | Website, email and marketing | Acquisition, content, email, social, reviews and marketing truth. |
| EXE-15 | Backup and continuity | Customer project recovery and company operational continuity. |
| EXE-16 | Release and operations | CI/CD, release evidence, launch, monitoring and maintenance. |

**Other keys**

| Keys | Area | What it covers |
|---|---|---|
| BUS | Business foundation | Ownership, authority, continuity, accounting and commercial prerequisites. |
| REQ | Requirements and traceability | Scope, traceability, evidence and change governance. |
| ARCH | Architecture and governance | Rules that prevent semantic drift and preserve the durable core. |
| DATA | Data integrity and persistence | Migrations, autosave, undo and recovery of canonical project data. |
| PORT | Project format and portability | Portable project packages, compatibility and semantic round trips. |
| SCR | Screenplay and writing | Structured screenplay, pagination, revisions and migration. |
| PROD | Production workflows | Story, board, breakdown, catalog, schedule, DOOD, call sheets and budget. |
| AI | AI | Local model runtime, retrieval, bounded tools, safety, privacy and evaluation. |
| SEC | Security | Threat model, parser safety, secrets, dependencies, signing and disclosure. |
| PRIV | Privacy | Data flows, telemetry and customer-data handling. |
| LEGAL | Legal and licensing | Contracts, notices, licenses, trademarks and claim accuracy. |
| WIN | Windows | Compatibility, signing, installers, Store and direct channels and OS behavior. |
| MAC | macOS | Compatibility, signing, packaging and OS behavior for the Mac App Store package. |
| STORE | Microsoft Store | Publisher ownership, listing, certification, package and Store lifecycle. |
| COMM | Commerce and trial | 7-day no-card trial, pricing, discounts, entitlement and refunds. |
| UPD | Updates and installers | Installer trust, updates, rollback and channel behavior. |
| SUP | Support and customer care | Customer self-service, diagnostics and escalation. |
| GTM | Website, email and marketing | Acquisition, content, email, social, reviews and marketing truth. |
| BCP | Backup and business continuity | Company backups, recovery objectives, second operator and recovery drills. |
| OPS | Release operations | CI/CD, release evidence, launch, monitoring and maintenance. |
| BETA | Beta programme | Real-filmmaker beta on a release candidate and the hardware matrix. |
| REL | Release engineering | Release gates and supply-chain checks. |

# Feature contract template

Create one feature-contract record in the feature ledger for every customer-visible feature and every high-risk internal capability. Do not paste this template repeatedly into the master checklist.

| Field | Question |
|---|---|
| **User outcome** | What customer outcome exists? |
| **Entry point** | Where can a normal user discover it? |
| **Canonical data** | What persistent data is authoritative? |
| **Queries/commands** | Which read and mutation operations exist? |
| **Registry/permissions** | Are operations registered and permission-scoped? |
| **UX states** | Empty/loading/success/partial-success/error/cancel/retry? |
| **Undo/recovery** | What reverses it and what happens after crash/restart? |
| **Search** | What is indexed and when? |
| **AI** | Explicit exposure and explicit non-exposure? |
| **Import/export** | What external-format impact exists? |
| **Backup/migration** | What package/backup/schema impact exists? |
| **Cross-platform** | What must be identical across OSes? |
| **Accessibility** | Keyboard/focus/reader/DPI/IME? |
| **Performance** | Scale and measurable budget? |
| **Security/privacy** | Trust boundary and data classification? |
| **Diagnostics/support** | What safe evidence helps support? |
| **Documentation** | What manual/help/release content changes? |
| **Tests/evidence** | What proves the feature is Verified and Release-ready? |

---

# Screenplay and Final Draft acceptance

This section is specific to the screenplay product. It does not repeat the generic lifecycle gate; it states what “deep enough” means for the screenplay product.

## Screenplay engine
- [ ] Structured screenplay element model remains canonical and independent of UI rendering.
- [ ] Automatic element progression is deterministic.
- [ ] Action, Scene Heading, Character, Parenthetical, Dialogue, Transition and supported custom elements have explicit rules.
- [ ] Dialogue continuation across pages is deterministic.
- [ ] Keep-together/orphan/widow behavior is tested.
- [ ] Explicit page-break directives are represented safely.
- [ ] Scene numbering is stable across edits and production-numbered drafts.
- [ ] Title page data is structured and exportable.
- [ ] Header/footer/page-setup behavior is deterministic.
- [ ] Production/page locking is explicit and reversible according to the product contract.
- [ ] Revision colors/metadata are preserved and exported where supported.
- [ ] Custom screenplay elements do not silently flatten during migration.
- [ ] Find/replace is safe across long documents and does not corrupt element boundaries.
- [ ] Unicode and multilingual screenplay text round-trips without corruption.
- [ ] Spell-check/proofing language behavior is explicit and does not alter canonical text unexpectedly.
- [ ] Navigator/scene navigation reflects canonical structure.
- [ ] Print preview and PDF output use the same deterministic layout contract.

## Final Draft / FDX migration
- [ ] FDX is treated as migration-critical.
- [ ] Import produces a preview/count summary and warnings.
- [ ] Scene headings are preserved.
- [ ] Characters/dialogue/parentheticals are preserved.
- [ ] Transitions/shots/custom elements are preserved, mapped or explicitly warned.
- [ ] Scene numbers are preserved where supported.
- [ ] Title-page information is preserved where supported.
- [ ] Revisions are preserved where supported.
- [ ] Unknown constructs never silently become unrelated content.
- [ ] Exported FDX can be re-imported into OpenFrame for round-trip verification.
- [ ] Representative fixtures cover dialogue-heavy, action-heavy, multilingual, revised, numbered, custom-element and long-dialogue scripts.

## Fountain / DOCX / PDF
- [ ] Fountain import/export has explicit fidelity expectations.
- [ ] DOCX is documented as a compatibility path rather than a claim of perfect screenplay semantics.
- [ ] PDF import is explicitly best-effort.
- [ ] PDF import provides preview/detection/page estimate/warnings and safe failure.
- [ ] Original PDF source remains untouched during import.

## Screenplay output branding
- [ ] Screenplay PDF has no unwanted OpenFrame branding.
- [ ] FDX has no unwanted OpenFrame branding.
- [ ] Fountain has no unwanted OpenFrame branding.
- [ ] Screenplay DOCX has no unwanted OpenFrame branding.

---

# Story Board, Breakdown and production acceptance

## Story Board
- [ ] Long scene descriptions have defined capacity and rendering behavior.
- [ ] Notes/attachments capacity is defined.
- [ ] Card display fields are user-selectable where specified.
- [ ] Presets and custom card modes persist correctly.
- [ ] Expanded/minimized card readability is tested.
- [ ] Story Board links remain correct when screenplay/scene identities change.
- [ ] Search/index projections include intended Story Board content.
- [ ] AI reads/proposes only through bounded contracts.

## Breakdown
- [ ] Breakdown categories have stable identity.
- [ ] Category colors are paired with text/icons so color is not the only signal.
- [ ] Quick operations and bulk operations are transactional and undoable where applicable.
- [ ] Filters and review state are persistent where intended.
- [ ] Source-phrase jump remains linked to screenplay structure.
- [ ] Catalog reuse does not create accidental duplicate canonical entities.
- [ ] Breakdown reports are reproducible from canonical state.
- [ ] Large-project performance is measured.

## Catalog / Locations / Cast / Crew
- [ ] Entity identity is stable and reusable.
- [ ] References survive rename and edit operations.
- [ ] Deletion/reference behavior is explicit.
- [ ] Search covers intended fields.
- [ ] Permissions and privacy classification are explicit for personal/contact-like data.
- [ ] Export/report behavior is defined.

---

# Schedule, Day Out of Days, call sheet and budget acceptance

## Schedule
- [ ] Schedule has canonical scene/date/day identity.
- [ ] Time fields and scene duration are validated.
- [ ] Break items are explicit.
- [ ] Travel/company-move items are explicit.
- [ ] Day totals are deterministic.
- [ ] Target wrap is visible and validated.
- [ ] Cast availability is represented or derived according to the contract.
- [ ] Location availability is represented or derived according to the contract.
- [ ] Conflict warnings are deterministic and actionable.
- [ ] Schedule filters do not mutate canonical state.
- [ ] Stale/review state is visible where derived data may be outdated.
- [ ] Schedule PDF/spreadsheet export is reproducible.

## DOOD
- [ ] DOOD is derived from cast + shooting schedule.
- [ ] DOOD recalculates when canonical inputs change.
- [ ] Manual editing of derived DOOD data is either forbidden or explicitly modeled.
- [ ] DOOD export is deterministic.
- [ ] AI can read/propose schedule changes only through approved tools and Change Sets.

## Call Sheets
The supplied production-reference material indicates that a useful indie call sheet should cover substantially more than a minimal document. Treat the following as a domain acceptance list while keeping optional sections hidden by default.

- [ ] Production/project identity.
- [ ] Production contacts.
- [ ] General call.
- [ ] Day/date.
- [ ] Weather where available and appropriate.
- [ ] Sunrise/sunset where available and appropriate.
- [ ] Crew call.
- [ ] Shooting call.
- [ ] Meal call.
- [ ] Locations.
- [ ] Parking/access information.
- [ ] Nearest hospital/emergency information where appropriate.
- [ ] Scene schedule.
- [ ] Talent/cast information.
- [ ] Atmosphere/background/extras information where applicable.
- [ ] Department notes.
- [ ] Advance schedule.
- [ ] Crew by department.
- [ ] Walkie/channel information where applicable.
- [ ] Manual overrides are clearly distinguished from derived fields.
- [ ] PDF output is stable and printable.

## Budget
- [ ] Budget scope remains lightweight and indie-production focused.
- [ ] Planned amounts are canonical.
- [ ] Actual amounts are canonical.
- [ ] Variance is derived deterministically.
- [ ] Totals reconcile.
- [ ] Export/report output is reproducible.
- [ ] The product does not imply full studio accounting capabilities it does not provide.

## Production-output branding
- [ ] Call-sheet PDF may use subtle OpenFrame branding according to the approved export policy.
- [ ] Schedule PDF/spreadsheet may use subtle branding according to the approved export policy.
- [ ] Breakdown/production/DOOD/budget reports follow the same approved branding policy.
- [ ] Branding never modifies customer-owned source files merely to add product identity.

---

# Appearance and theme acceptance

The revised specification treats the theme system as an **application appearance layer**, not a content-system rewrite. The current OpenFrame appearance remains the factory/default appearance.

## Theme contract
- [ ] Theme changes appearance only.
- [ ] Theme does not change navigation.
- [ ] Theme does not change labels or feature availability.
- [ ] Theme does not change project data.
- [ ] Theme does not change screenplay semantics or production logic.
- [ ] Theme does not change exports.
- [ ] Theme does not change shortcuts or permissions.
- [ ] Theme does not change file paths or project identity.
- [ ] A user who never visits Appearance Settings sees the canonical OpenFrame default.

## Mode / theme separation
- [ ] Mode supports Light.
- [ ] Mode supports Dark.
- [ ] Mode supports System.
- [ ] Theme and Mode are separate settings.
- [ ] System mode follows OS appearance changes without restart where supported.
- [ ] Invalid stored theme data falls back safely to OpenFrame Default.

## Semantic tokens
- [ ] Background.
- [ ] Surface.
- [ ] Elevated surface.
- [ ] Primary/secondary/muted text.
- [ ] Border/strong border.
- [ ] Accent/accent hover/accent soft.
- [ ] Focus.
- [ ] Selection.
- [ ] Sidebar/sidebar text.
- [ ] Success/warning/error/info.
- [ ] Shadow.
- [ ] Tokenization search covers hard-coded hex/rgb/rgba/named colors and SVG fills/strokes.
- [ ] Each hard-coded color is classified as theme token, intentional fixed value or document/output value.
- [ ] New themes do not require screen-by-screen component edits.

## Built-in theme registry
- [ ] OpenFrame Default.
- [ ] Graphite.
- [ ] Deep Navy.
- [ ] Teal & Ember.
- [ ] Parchment Room.
- [ ] Sage Cut.
- [ ] Burgundy Gold.
- [ ] Sunset Coral.
- [ ] Daylight Blue.
- [ ] Violet Night.
- [ ] Olive Field.
- [ ] Desert Sand.
- [ ] Colorist Neutral.
- [ ] Silver Screen.
- [ ] High Contrast.

## Custom themes
- [ ] Theme import is app-local and does not mutate project canonical state.
- [ ] Theme export/import has explicit schema/version.
- [ ] Malicious/oversized/malformed theme data is rejected safely.
- [ ] Theme IDs are validated.
- [ ] Seed/token values are validated.
- [ ] Arbitrary CSS/script execution is impossible through theme import.
- [ ] Invalid theme falls back to a safe default.
- [ ] “My Themes” behavior is defined if enabled.
- [ ] Theme selection persists correctly.
- [ ] Theme changes do not alter screenplay document rendering unless the product explicitly chooses to theme only the application chrome.
- [ ] Representative screens have visual regression coverage across Light/Dark/System and built-in themes.
- [ ] High Contrast preserves usability and does not rely on color alone for meaning.

---

# AI-assisted development control

- [ ] Before coding: list affected modules, invariants, files, operations, migrations, tests and platform impact.
- [ ] Implement the smallest vertical slice.
- [ ] Run format/lint/typecheck/tests/generated-binding checks.
- [ ] Inspect semantic diff for Store bypass, direct DB writes, path changes, permission changes, migration changes, AI exposure and export changes.
- [ ] Run lifecycle tests for create/save/restart/undo/redo/delete/restore/export/import where applicable.
- [ ] Run targeted security tests for parser/path/package/AI/process/external-link/update changes.
- [ ] Changes that cross modules or touch security, release, data format or AI authority never merge solely because the feature works.
- [ ] Record how AI sees and does not see the new operation.

The supplied vibe-coding document explicitly recommends this workflow and PR gates.

# End-to-end and failure testing

## Mandatory customer journey

`launch → create project → story → scene → screenplay → breakdown → schedule → call sheet → save → close → relaunch → reopen → edit → undo → redo → export → re-import → continue`

Run it on every supported OS target. This journey is explicitly required by the OS/security plan.

### Storage
- [ ] Inject **disk full** → verify safe failure, user guidance and recovery.
- [ ] Inject **permission denied** → verify safe failure, user guidance and recovery.
- [ ] Inject **path disappears** → verify safe failure, user guidance and recovery.
- [ ] Inject **external volume removed** → verify safe failure, user guidance and recovery.
- [ ] Inject **read-only destination** → verify safe failure, user guidance and recovery.

### Process
- [ ] Inject **AI sidecar crash** → verify safe failure, user guidance and recovery.
- [ ] Inject **AI sidecar hang** → verify safe failure, user guidance and recovery.
- [ ] Inject **kill during save** → verify safe failure, user guidance and recovery.
- [ ] Inject **kill during migration** → verify safe failure, user guidance and recovery.
- [ ] Inject **kill during export** → verify safe failure, user guidance and recovery.

### Network
- [ ] Inject **offline** → verify safe failure, user guidance and recovery.
- [ ] Inject **manifest unavailable** → verify safe failure, user guidance and recovery.
- [ ] Inject **interrupted model download** → verify safe failure, user guidance and recovery.
- [ ] Inject **interrupted update download** → verify safe failure, user guidance and recovery.

### Files
- [ ] Inject **malformed ZIP** → verify safe failure, user guidance and recovery.
- [ ] Inject **malformed XML** → verify safe failure, user guidance and recovery.
- [ ] Inject **malformed PDF** → verify safe failure, user guidance and recovery.
- [ ] Inject **malformed DOCX** → verify safe failure, user guidance and recovery.
- [ ] Inject **oversized file** → verify safe failure, user guidance and recovery.
- [ ] Inject **decompression bomb** → verify safe failure, user guidance and recovery.
- [ ] Inject **symlink** → verify safe failure, user guidance and recovery.
- [ ] Inject **path traversal** → verify safe failure, user guidance and recovery.
- [ ] Inject **Unicode path** → verify safe failure, user guidance and recovery.
- [ ] Inject **case collision** → verify safe failure, user guidance and recovery.

### Commerce
- [ ] Inject **Store outage** → verify safe failure, user guidance and recovery.
- [ ] Inject **entitlement delay** → verify safe failure, user guidance and recovery.
- [ ] Inject **trial expiry** → verify safe failure, user guidance and recovery.
- [ ] Inject **refund/cancellation state** → verify safe failure, user guidance and recovery.
- [ ] Inject **restore purchase** → verify safe failure, user guidance and recovery.

### Update
- [ ] Inject **bad artifact** → verify safe failure, user guidance and recovery.
- [ ] Inject **signature mismatch** → verify safe failure, user guidance and recovery.
- [ ] Inject **interrupted download** → verify safe failure, user guidance and recovery.
- [ ] Inject **low disk space** → verify safe failure, user guidance and recovery.
- [ ] Inject **restart failure** → verify safe failure, user guidance and recovery.

The OS/security plan explicitly requires storage, process, network and malformed-file failure injection.

# Windows and Mac release matrix

Windows 11 only and Apple silicon only were decided on 2026-10-06. The Mac column applies to the Mac App Store package.

| Capability | Windows 11 (x64) | macOS (Apple silicon) | Evidence |
|---|---:|---:|---|
| Clean install | ☐ | ☐ | |
| Upgrade | ☐ | ☐ | |
| Uninstall/reinstall | ☐ | ☐ | |
| Project create/open/close | ☐ | ☐ | |
| Migration | ☐ | ☐ | |
| Autosave/recovery | ☐ | ☐ | |
| Undo/redo | ☐ | ☐ | |
| Trash/restore | ☐ | ☐ | |
| Files | ☐ | ☐ | |
| Search | ☐ | ☐ | |
| AI runtime | ☐ | ☐ | |
| Import/export | ☐ | ☐ | |
| High DPI/display scaling | ☐ | ☐ | |
| IME/Unicode | ☐ | ☐ | |
| External drives | ☐ | ☐ | |
| Permissions | ☐ | ☐ | |
| Sleep/wake | ☐ | ☐ | |
| Crash recovery | ☐ | ☐ | |
| Offline behavior | ☐ | ☐ | |
| Update | ☐ | ☐ | |
| Artifact verification | ☐ | ☐ | |

The OS/security plan requires equivalent lifecycle tests on Windows 11 x64 and on Apple silicon Macs. Intel Macs are not supported.

# Microsoft Store launch runbook

## Ownership
- [ ] Legal company owns Partner Center account
- [ ] Backup administrator exists and recovery is tested
- [ ] Publisher/package identity is reconciled
- [ ] Billing/tax/payment ownership is documented

## Listing
- [ ] Name/icon/publisher are final
- [ ] Description matches shipped capabilities
- [ ] Screenshots match build
- [ ] Privacy URL works
- [ ] Support URL works
- [ ] Age rating/category/regions are accurate
- [ ] Pricing/trial match commercial ledger
- [ ] AI/offline/accessibility claims are truthful

## Certification
- [ ] Applicable validation tooling run
- [ ] Clean Store install tested
- [ ] First launch/project creation tested
- [ ] Upgrade tested
- [ ] Uninstall/reinstall tested
- [ ] Entitlement restoration tested
- [ ] Certification evidence archived
- [ ] Live listing smoke-tested after publication

At launch, verify current Microsoft Store/Partner Center policy rather than relying on static policy details in this document.

# Commerce and 7-day no-card trial

- [ ] Trial lasts exactly the advertised 7 days.
- [ ] No credit card is required.
- [ ] Trial start/expiry are deterministic.
- [ ] Reinstall/device/account/clock/offline behavior is explicitly defined.
- [ ] Purchase during trial works.
- [ ] Entitlement refresh works after delayed Store responses.
- [ ] Restore purchase works.
- [ ] Cancellation/refund behavior is documented.
- [ ] Discounts and dynamic pricing have one authoritative system per channel.
- [ ] Website, Store and app pricing/trial/region/currency claims agree.
- [ ] Commerce outages never corrupt or delete project data.

# Website, customer email and social

- [ ] Company-owned domain and registrar recovery.
- [ ] DNS recovery documented.
- [ ] HTTPS works.
- [ ] Product/features/pricing/trial/download pages exist.
- [ ] Privacy/terms/refund/support/security pages work.
- [ ] Download links point to current verified artifacts.
- [ ] SPF configured.
- [ ] DKIM configured.
- [ ] DMARC policy deliberate and monitored.
- [ ] Transactional and marketing email separated appropriately.
- [ ] Bounce/complaint handling exists.
- [ ] Social accounts use MFA and backup administration.
- [ ] Launch/update content is prepared.
- [ ] Review requests are honest and non-deceptive.
- [ ] Public claims are linked to product evidence.

# Support and diagnostics

- [ ] Company-owned support channel.
- [ ] Severity definitions and response targets.
- [ ] Case IDs and escalation workflow.
- [ ] Safe diagnostic bundle.
- [ ] No screenplay/private-note/credential leakage.
- [ ] Knowledge base for install/update/recovery/import-export/AI/Windows/macOS.
- [ ] Reproduction workflow using version/build/platform/fixture information.
- [ ] Actionable customer error messages.
- [ ] Crash-loop recovery.
- [ ] Security issues have confidential escalation.
- [ ] Second operator can run support without founder-only knowledge.

# Privacy, legal, security and supply chain

- [ ] Data inventory.
- [ ] Data-flow map.
- [ ] External-service/subprocessor inventory.
- [ ] Retention/deletion/access/correction/export process where applicable.
- [ ] Privacy policy matches implementation.
- [ ] Terms/EULA match implementation.
- [ ] Refund/cancellation terms match commerce.
- [ ] Open-source notices complete.
- [ ] AI/model/embedding licenses recorded.
- [ ] Trademark/domain/product-name rights reviewed.
- [ ] Threat model current.
- [ ] Secret scan clean.
- [ ] Dependency vulnerability scan reviewed.
- [ ] License scan reviewed.
- [ ] SBOM generated.
- [ ] Signing keys protected.
- [ ] Security disclosure channel tested.
- [ ] Critical unresolved findings block release.

The OS/security plan requires signed model/update handling, bounded import security, protected secrets, privacy-safe logging and cross-platform security invariants.

# Backup, disaster recovery and founder independence

- [ ] Customer manual backup exists.
- [ ] Restore is tested.
- [ ] Replacement-machine restore is tested.
- [ ] Cross-platform restore is tested where promised.
- [ ] Source code backup exists.
- [ ] Release artifact backup exists.
- [ ] Legal/policy record backup exists.
- [ ] Runbooks are backed up.
- [ ] GitHub-loss recovery tested/documented.
- [ ] Microsoft Store account-loss recovery tested/documented.
- [ ] Apple account-loss recovery tested/documented.
- [ ] Domain-loss recovery tested/documented.
- [ ] Email/support-loss recovery tested/documented.
- [ ] Second operator can execute critical recovery workflows.
- [ ] Pre-launch DR drill completed.
- [ ] Recurring DR drill scheduled.

# Release engineering and CI/CD

- [ ] Format/lint/typecheck/clippy.
- [ ] Unit/integration/frontend tests.
- [ ] Architecture checks.
- [ ] Migration fixture tests.
- [ ] Lifecycle E2E.
- [ ] Import/export round-trip corpus.
- [ ] AI evaluation where affected.
- [ ] Security tests where affected.
- [ ] Windows CI.
- [ ] macOS CI.
- [ ] Immutable release candidate.
- [ ] Checksums.
- [ ] SBOM.
- [ ] License inventory.
- [ ] Artifact signatures.
- [ ] Independent signature verification.
- [ ] Clean-machine installer tests.
- [ ] Upgrade/update tests.
- [ ] Rollback/fallback plan.
- [ ] Release notes/known issues.
- [ ] Evidence archive.

The supplied vibe-coding plan defines compile → behavior → architecture → data → security → platform → release gates.

# Launch timeline

| When | What happens |
|---|---|
| 180 to 121 days before launch | Ownership, accounts, scope, architecture freeze, feature ledger, evidence system. |
| 120 to 91 days before launch | Core feature integration, migration fixtures, import/export corpus, OS CI, security model, support foundation. |
| 90 to 61 days before launch | Screenplay/production hardening, AI evaluation, themes/visual regression, installers, website, policies. |
| 60 to 31 days before launch | Release candidates, clean-machine tests, Store preparation, support/restore drills, marketing assets. |
| 30 to 15 days before launch | Certification/submission, commercial validation, email deliverability, final security and docs. |
| 14 to 7 days before launch | Full E2E, failure injection, platform parity, public-surface consistency audit, war-room rehearsal. |
| 6 to 1 days before launch | Final evidence, signatures/notarization/Store package, backups, access verification, GO/NO-GO. |
| Launch day | Publish, smoke-test install/trial/purchase/update, monitor incidents and support. |
| Days 1 to 7 after launch | Daily health review; urgent reliability/security fixes only under release controls. |
| Days 8 to 30 after launch | First product health review, top bugs, support/knowledge improvements, security review. |
| Days 31 to 90 after launch | Quarterly operating cadence, continuity drill, roadmap reprioritization. |

# Launch-day war room and stop-ship

- [ ] Incident commander named.
- [ ] Engineering lead named.
- [ ] Product decision maker named.
- [ ] Support lead named.
- [ ] Security/privacy escalation owner named.
- [ ] Backup owners named.
- [ ] Live release dashboard ready.
- [ ] Release artifact identifiers known.
- [ ] Store/site/support links verified.
- [ ] Rollback/fallback procedure ready.
- [ ] Decision log active.

### Stop-ship examples
- [ ] Data corruption/project loss.
- [ ] Migration failure.
- [ ] Private creative-content leak.
- [ ] AI privilege bypass.
- [ ] Unverifiable release artifact.
- [ ] Broken installer/update path.
- [ ] Failed notarization/invalid Store package.
- [ ] Critical security vulnerability.
- [ ] Cross-platform canonical-data divergence.
- [ ] Import/export semantic loss against published promise.
- [ ] Crash recovery that can lose customer work.

# Post-launch operations

| Cadence | Required review |
|---|---|
| Daily first week | Crashes, activation/trial, Store health, support, incidents, reviews. |
| Weekly | Top bugs, customer friction, support backlog, conversion, refunds, release pipeline. |
| Monthly | Dependencies, security advisories, certificates, domains, vendors, costs, email health. |
| Quarterly | Backups/restore, account access, continuity, privacy/legal consistency, roadmap. |
| Per release | Full release gates and evidence archive. |
| Annually | Security/legal review, DR drill, vendor review, product continuity review. |

# Automation register

| Automation | Frequency | Human approval | Failure alert |
|---|---|---|---|
| CI quality gates | Every PR | No | Yes |
| Dependency/security scans | Daily/PR | Review | Yes |
| Backup verification | Scheduled | No | Yes |
| Certificate/domain expiry | Daily | No | Yes |
| Website/download smoke test | Daily/release | No | Yes |
| Email deliverability | Scheduled | No | Yes |
| Crash aggregation | Continuous | No | Yes |
| Support SLA alerts | Continuous | No | Yes |
| AI retrieval evaluation | Per model/index release | Yes | Yes |
| Release signature verification | Per release | Yes | Yes |
| SBOM/license generation | Per release | No | Yes |

# Evidence index template

| Evidence ID | Requirement keys | Artifact/test | Version/commit | Date | Owner | Result | Location |
|---|---|---|---|---|---|---|---|
| EV- | | | | | | | |
| EV- | | | | | | | |
| EV- | | | | | | | |
| EV- | | | | | | | |
| EV- | | | | | | | |
| EV- | | | | | | | |
| EV- | | | | | | | |
| EV- | | | | | | | |
| EV- | | | | | | | |
| EV- | | | | | | | |
| EV- | | | | | | | |
| EV- | | | | | | | |

# Risk register template

| Risk ID | Risk | Probability | Impact | Mitigation | Owner | Trigger | Status |
|---|---|---|---|---|---|---|---|
| R- | | | | | | | |
| R- | | | | | | | |
| R- | | | | | | | |
| R- | | | | | | | |
| R- | | | | | | | |
| R- | | | | | | | |
| R- | | | | | | | |
| R- | | | | | | | |
| R- | | | | | | | |
| R- | | | | | | | |
| R- | | | | | | | |
| R- | | | | | | | |
| R- | | | | | | | |
| R- | | | | | | | |
| R- | | | | | | | |

# Vendor and dependency register

| Vendor/dependency | Purpose | Data | Criticality | Terms/contract | Renewal | Backup option | Owner |
|---|---|---|---|---|---|---|---|
| | | | | | | | |
| | | | | | | | |
| | | | | | | | |
| | | | | | | | |
| | | | | | | | |
| | | | | | | | |
| | | | | | | | |
| | | | | | | | |
| | | | | | | | |
| | | | | | | | |
| | | | | | | | |
| | | | | | | | |
| | | | | | | | |
| | | | | | | | |
| | | | | | | | |
| | | | | | | | |
| | | | | | | | |
| | | | | | | | |
| | | | | | | | |
| | | | | | | | |

# Responsibility and continuity matrix

| Capability | Primary | Backup | External dependency | Runbook | Tested |
|---|---|---|---|---|---|
| Source code | | | | | ☐ |
| CI/CD | | | | | ☐ |
| Windows signing | | | | | ☐ |
| Microsoft Store | | | | | ☐ |
| macOS signing | | | | | ☐ |
| Apple notarization | | | | | ☐ |
| Updater signing | | | | | ☐ |
| Domain/DNS | | | | | ☐ |
| Website | | | | | ☐ |
| Email | | | | | ☐ |
| Support | | | | | ☐ |
| Security disclosure | | | | | ☐ |
| Privacy requests | | | | | ☐ |
| Legal policies | | | | | ☐ |
| Customer project recovery | | | | | ☐ |
| AI model distribution | | | | | ☐ |
| Release management | | | | | ☐ |
| Incident response | | | | | ☐ |
| Marketing/social | | | | | ☐ |
| Billing/finance | | | | | ☐ |

# Product health scorecard

| Area | Metric | Target | Current | Trend | Action threshold |
|---|---|---|---|---|---|
| Reliability | Crash-free rate | | | | |
| Reliability | Project-open failure | | | | |
| Reliability | Recovery success | | | | |
| Product | Activation | | | | |
| Product | Trial completion | | | | |
| Product | Trial-to-paid conversion | | | | |
| Product | Feature adoption | | | | |
| Customer | Support volume | | | | |
| Customer | First response | | | | |
| Customer | Resolution time | | | | |
| Customer | Repeat contact | | | | |
| Commerce | Refund rate | | | | |
| Commerce | Entitlement failures | | | | |
| Commerce | Certification failures | | | | |
| Quality | Must-ship and should-ship defects | | | | |
| Quality | Regression escape | | | | |
| Quality | Rollback count | | | | |
| Security | Open critical findings | | | | |
| Security | Dependency exposure | | | | |
| Security | Security reports | | | | |
| Operations | Backup verification | | | | |
| Operations | Expiry risk | | | | |
| Operations | Single-person dependencies | | | | |

# Final adversarial audit

- [ ] Can a new customer install from the intended public channel without internal help?
- [ ] Can the customer start the advertised 7-day trial without a credit card?
- [ ] Can the customer create/save/close/reopen/continue real work?
- [ ] Can the customer recover from a crash/interrupted save?
- [ ] Can the project move to another supported computer?
- [ ] Can promised formats round-trip without silent loss?
- [ ] Are Windows/macOS semantics and security equivalent?
- [ ] Can AI understand the product without uncontrolled authority?
- [ ] Can AI never bypass human Change Set approval?
- [ ] Can malicious projects/packages/models/updates fail safely?
- [ ] Can support diagnose without requesting creative content?
- [ ] Can another operator support customers without founder-only knowledge?
- [ ] Can the company recover from loss of GitHub/Store/Apple/domain/email access?
- [ ] Do Store/site/app/email/marketing make consistent claims?
- [ ] Are privacy/legal documents truthful?
- [ ] Are release artifacts independently verifiable?
- [ ] Are all Must ship requirements PASS or explicitly blocked from release?
- [ ] Does every PASS have evidence?
- [ ] Would operations survive six months without the original developer?

# Master release gate

- [ ] All applicable Must ship requirements are Release-ready or later.
- [ ] All required gates are PASS.
- [ ] No critical data/security/recovery stop-ship issue remains.
- [ ] All supported OS artifacts pass.
- [ ] Store submission/certification is ready for the intended channel.
- [ ] Trial/pricing/entitlement behavior is proven.
- [ ] Website/support/email/legal/privacy surfaces are live and consistent.
- [ ] Release artifacts are signed, verifiable and archived.
- [ ] Customer recovery path is tested.
- [ ] Support can operate without founder-only knowledge.
- [ ] Launch war room and fallback plan are ready.
- [ ] Evidence index is complete enough for independent review.
- [ ] Product owner signs GO.

| Decision | Value |
|---|---|
| Release version | |
| Release commit | |
| Decision owner | |
| Date/time | |
| Evidence index | |
| GO / NO-GO / GO WITH FORMAL RISK ACCEPTANCE | |

# Maintenance calendar

| Frequency | Maintenance |
|---|---|
| Per PR | Quality, architecture, security and binding gates. |
| Per release | Full release evidence, installers, Store/site consistency. |
| Weekly | Support, crash, review and incident review. |
| Monthly | Dependencies, certificates, domains, vendors, costs and email health. |
| Quarterly | Backup restore, account access, continuity and policy consistency. |
| Annually | Security, legal, DR, vendor and product-continuity review. |

# Source-of-truth hierarchy

1. **Protected architecture/security invariants**
2. **Current approved product specification and decisions**
3. **Canonical feature ledger/requirement register**
4. **Current implementation evidence/tests**
5. **Current customer-facing release declaration**
6. **Supporting manuals/historical plans**
7. **Ideas/prototypes/obsolete documents**

The completion plan explicitly rejects “finish all old items, then redesign” and recommends dependency-aware reconciliation by module.

# Anti-overengineering rule

- [ ] Prefer automation over recurring manual memory.
- [ ] Prefer evidence over confidence.
- [ ] Prefer one canonical requirement over multiple copies.
- [ ] Prefer safe failure over silent partial success.
- [ ] Prefer reversible operations over destructive shortcuts.
- [ ] Prefer platform adapters over duplicated business logic.
- [ ] Prefer explicit launch scope over feature creep.
- [ ] Do not add enterprise-scale infrastructure unless a real product requirement justifies it.

# Final operating philosophy

```text
clear product promise
+ canonical requirements
+ durable architecture
+ safe AI-assisted development
+ automatic verification
+ cross-platform evidence
+ secure release artifacts
+ self-service support
+ business continuity
+ measured post-launch operations
= a product that does not depend on constant founder firefighting
```

The goal is not zero bugs forever. The goal is that known data, security, compatibility, recovery and commercial invariants are protected by architecture, automation, evidence and operating procedures.

# Earlier version of this register

The version of this register from before the restructure, including the old traceability appendix that repeated the execution requirements, is kept at [launch-register-before-restructure](../archive/2026-10-06-launch-register-before-restructure.md).

# Dependency-aware implementation order

Names are the areas of the [build plan](../implementation/masterplan.md). Rows are in build order.

| Area of the build plan | Work |
|---|---|
| Project controls and baseline | Baseline freeze, feature ledger, architecture lint, fixtures and evidence system. |
| Screenplay engine and file formats | Screenplay trust: structured model, pagination, revisions, FDX, Fountain, PDF and DOCX boundaries and round trips. |
| Writing comfort; Appearance and themes | Writer comfort, appearance and theme, and visual regression. |
| Story Board and Idea Vault | Story Board depth and lineage. |
| Breakdown, catalog, locations, cast and crew | Breakdown, catalog, locations, cast and crew depth. |
| Shooting schedule and Day Out of Days | Schedule, DOOD, availability and conflicts. |
| Call sheets, budget, templates, samples and exports | Call sheets, production documents and lightweight budget. |
| Packages and backup | Import and export hardening and package portability. |
| AI assistant quality and hardening; Security hardening | AI evaluation, security hardening, minimum-spec performance and tool coverage. |
| Platform adapter layer for Windows and Mac; Windows installer; Mac App Store package | Windows and Mac production hardening, installers, signing and clean-machine tests. |
| Microsoft Store package | Microsoft Store certification and submission, and direct distribution readiness. |
| Legal, privacy, website and support | Website, support, email, privacy and legal, marketing and social launch system. |
| Final review and submission | Full release candidate, failure injection, launch war room and final audit. |
| Certification, publication and launch; First 90 days after launch | Launch and first 90 days of measured operations. |

The supplied execution plan uses this dependency-aware philosophy and explicitly places lifecycle, import/export, AI evaluation, rendering regression, OS CI and release gates before the final feature rush.

# Release evidence package

- [ ] Release manifest
- [ ] Source commit/tag
- [ ] Build logs
- [ ] Checksums
- [ ] Signature verification
- [ ] SBOM
- [ ] License inventory
- [ ] Migration results
- [ ] Lifecycle E2E results
- [ ] Import/export results
- [ ] Security results
- [ ] AI evaluation results
- [ ] Windows clean-machine result
- [ ] macOS clean-machine result
- [ ] Store/certification evidence
- [ ] Installer/update evidence
- [ ] Visual regression report
- [ ] Accessibility report
- [ ] Performance report
- [ ] Known-issues list
- [ ] Support readiness
- [ ] Website/store consistency check
- [ ] Backup/restore drill
- [ ] Go/no-go record

# Final sign-off

| Gate | Owner | Backup | Evidence | Status | Date |
|---|---|---|---|---|---|
| Product scope | | | | ☐ | |
| Requirements/traceability | | | | ☐ | |
| Architecture | | | | ☐ | |
| Data integrity | | | | ☐ | |
| Screenplay | | | | ☐ | |
| Production workflows | | | | ☐ | |
| Import/export | | | | ☐ | |
| AI | | | | ☐ | |
| Security | | | | ☐ | |
| Privacy | | | | ☐ | |
| Legal/licensing | | | | ☐ | |
| Windows | | | | ☐ | |
| macOS | | | | ☐ | |
| Microsoft Store | | | | ☐ | |
| Commerce/trial | | | | ☐ | |
| Installers/updates | | | | ☐ | |
| Website | | | | ☐ | |
| Email | | | | ☐ | |
| Support | | | | ☐ | |
| Marketing/social | | | | ☐ | |
| Backup/DR | | | | ☐ | |
| Release engineering | | | | ☐ | |
| Founder-dependency elimination | | | | ☐ | |
| Final adversarial audit | | | | ☐ | |

# Keeping this document current

- [ ] When a requirement changes, update its canonical row first.
- [ ] When a cross-cutting rule changes, update the gate catalog once.
- [ ] Do not duplicate a requirement because multiple teams touch it.
- [ ] Add a new key for a materially new promise; supersede old keys with traceable decisions rather than deleting history.
- [ ] At every release, regenerate evidence index and run the adversarial audit.

# Additions from the intake notes (2026-10-05)

Requirements present in the owner's notes (`docs/archive/notes/store-certification-and-launch-gap-notes.md`) or in the Store audit and not in the register above. Keys are new and stable. Status starts at Specified. Must ship blocks Store submission. Should ship blocks launch quality.

| Key | Requirement | Importance | Where it is built |
|---|---|---|---|
| `STORE-007` | In-app way to report inappropriate AI output (Store policy for live generative AI); never sends project content automatically | Must ship | AI assistant quality and hardening |
| `STORE-008` | Store listing discloses live generative AI and the optional on-demand download | Must ship | Final review and submission |
| `STORE-009` | Age rating (IARC) answered accurately for screenplays with mature themes or violence | Must ship | Final review and submission |
| `STORE-010` | Notes for Certification: no login needed, how to test optional AI, network use, capability justification | Must ship | Final review and submission |
| `STORE-011` | Clean first launch on a bare machine or Windows Sandbox; no locally installed runtime dependency | Must ship | Microsoft Store package |
| `STORE-012` | Manifest capabilities limited to those used (runFullTrust, plus internetClient for the user-clicked AI model download and the update check, ADR-0026; internetClient was removed 2026-10-08 under ADR-0025) with written justification | Must ship | Microsoft Store package |
| `STORE-013` | At least one clear, uncropped screenshot of the real UI from the release candidate | Must ship | Final review and submission |
| `STORE-014` | Windows App Certification Kit report (zero failures) archived for the exact package | Must ship | Microsoft Store package |
| `STORE-015` | Supported Windows versions in manifest, listing and docs agree with what was tested | Must ship | Project controls and baseline |
| `STORE-016` | Store policy version re-checked on the submission date | Should ship | Final review and submission |
| `BETA-001` | Real-filmmaker beta on a release candidate, with feedback channel and a closed list of findings | Must ship | Product-wide quality, accessibility and beta |
| `BETA-002` | Windows hardware matrix (minimum, recommended, discrete GPU, high DPI, multi-monitor) | Must ship | AI assistant quality and hardening; Product-wide quality, accessibility and beta |
| `GTM-011` | Launch trailer and Store media | Should ship | Certification, publication and launch |
| `GTM-012` | Founder story and public narrative | Should ship | Certification, publication and launch |
| `GTM-013` | 30-day content bank | Should ship | Certification, publication and launch |
| `GTM-014` | Creator outreach list; community strategy (Reddit, Discord partnerships) that follows each community rules | Should ship | Certification, publication and launch |
| `GTM-015` | Website SEO basics and social account reservation | Should ship | Legal, privacy, website and support; Certification, publication and launch |
| `SUP-008` | Customer feedback system (in addition to support) | Should ship | Certification, publication and launch |
| `SUP-009` | Status and outage communication channel | Should ship | Certification, publication and launch |
| `OPS-006` | Post-launch incident response runbook | Must ship | Certification, publication and launch |
| `OPS-007` | Versioning strategy (SemVer app version, four-part MSIX version, schema versions) and changelog | Must ship | Microsoft Store package; Final review and submission |
| `OPS-008` | Localization-ready architecture confirmed (English-only first release) | Should ship | Product-wide quality, accessibility and beta |
| `OPS-009` | Public documentation and release notes | Should ship | Product-wide quality, accessibility and beta; Final review and submission |
| `AI-011` | Store build bundles the runtime; weights downloaded on user click (ADR-0020) | Must ship | AI assistant quality and hardening |
| `DATA-008` | Multi-operation AI Change Set apply is crash-safe (ADR-0019) | Must ship | Data safety and crash recovery |
| `REL-011` | Desktop E2E is a required release gate | Must ship | Test infrastructure and quality gates |
| `REL-012` | Supply-chain checks (cargo deny, npm audit, licence inventory) pass without bypass | Must ship | Project controls and baseline |